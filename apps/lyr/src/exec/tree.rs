//! A script's process tree: the shell lyr starts and everything it starts in
//! turn, so stopping a custom module's script stops what the script started
//! too, not only the shell.
//!
//! - Unix: the shell leads a process group of its own, and stopping the tree
//!   sends `SIGTERM` to the group through `kill(1)`, since lyr links no libc
//!   binding of its own. A process that leaves the group (`setsid`) escapes.
//! - Windows: the shell starts suspended, is put in a Job Object that kills
//!   its processes when its last handle closes, and only then runs, so
//!   nothing it starts escapes the job. Its thread is found in a snapshot of
//!   the system's threads, since std's handle to it is not stable yet
//!   (`windows_process_extensions_main_thread_handle`); the snapshot lists
//!   every thread, so a script run costs it once. Dropping a [`Tree`] closes that
//!   handle, so on Windows it also kills what is left of the tree; on Unix
//!   only [`Tree::kill`] does.

use std::io;
use std::process::{Child, Command};

/// The process tree of one started script.
#[derive(Debug)]
pub struct Tree {
    /// The process group, which is the shell's process id.
    #[cfg(unix)]
    group: u32,
    /// The job every process of the tree is in.
    #[cfg(windows)]
    job: std::os::windows::io::OwnedHandle,
}

/// Sets `command` up to start a tree of its own ([`adopt`] finishes it).
pub(super) fn prepare(command: &mut Command) {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt as _;
        command.process_group(0);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        use windows_sys::Win32::System::Threading::{CREATE_NO_WINDOW, CREATE_SUSPENDED};
        // Replaces the flags `child` set, so its own is kept here.
        command.creation_flags(CREATE_NO_WINDOW | CREATE_SUSPENDED);
    }
}

/// The tree of `child`, started from a command [`prepare`] set up. On
/// Windows this puts it in its job and lets it run.
///
/// # Errors
///
/// When Windows refuses the job, or the child's thread cannot be resumed;
/// the child is killed then.
pub(super) fn adopt(child: &mut Child) -> io::Result<Tree> {
    #[cfg(unix)]
    {
        Ok(Tree { group: child.id() })
    }
    #[cfg(windows)]
    {
        let adopted = windows::adopt(child);
        if adopted.is_err() {
            // It never ran, being suspended: nothing else to stop.
            if let Err(error) = child.kill() {
                tracing::warn!("a script lyr could not put in a job did not stop: {error}");
            }
        }
        adopted
    }
}

impl Tree {
    /// Stops every process in the tree. One that already ended is not an
    /// error; a failure is logged.
    pub fn kill(&self) {
        #[cfg(unix)]
        {
            // A negative id names the process group.
            let group = format!("-{}", self.group);
            let killed = super::child("kill")
                .args(["-s", "TERM", "--", group.as_str()])
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
            // `kill` fails when the group has ended already, which is
            // what was wanted.
            if let Err(error) = killed {
                tracing::warn!("`kill` did not start to stop a script: {error}");
            }
        }
        #[cfg(windows)]
        windows::terminate(&self.job);
    }
}

#[cfg(windows)]
mod windows {
    use std::io;
    use std::os::windows::io::{AsRawHandle as _, FromRawHandle as _, OwnedHandle};
    use std::process::Child;

    use windows_sys::Win32::Foundation::{HANDLE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First, Thread32Next,
    };
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
        SetInformationJobObject, TerminateJobObject,
    };
    use windows_sys::Win32::System::Threading::{OpenThread, ResumeThread, THREAD_SUSPEND_RESUME};

    use super::Tree;

    /// What a process killed with its job exits with.
    const KILLED: u32 = 1;

    /// `handle` owned, or the last OS error when it is null or invalid.
    fn owned(handle: HANDLE) -> io::Result<OwnedHandle> {
        if handle.is_null() || handle == INVALID_HANDLE_VALUE {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: a handle the call just returned, valid and owned by no one
        // else.
        Ok(unsafe { OwnedHandle::from_raw_handle(handle) })
    }

    /// Puts the suspended `child` in a new kill-on-close job, then resumes
    /// its threads.
    pub(super) fn adopt(child: &Child) -> io::Result<Tree> {
        // SAFETY: no attributes and no name are valid arguments.
        let job = owned(unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) })?;
        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        let size = u32::try_from(size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>())
            .map_err(io::Error::other)?;
        // SAFETY: `limits` is the structure this class takes, `size` its
        // size, and `job` a live job handle.
        let set = unsafe {
            SetInformationJobObject(
                job.as_raw_handle(),
                JobObjectExtendedLimitInformation,
                (&raw const limits).cast(),
                size,
            )
        };
        if set == 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: both handles are live: the job's here, the child's while
        // `child` is borrowed.
        if unsafe { AssignProcessToJobObject(job.as_raw_handle(), child.as_raw_handle()) } == 0 {
            return Err(io::Error::last_os_error());
        }
        resume(child.id())?;
        Ok(Tree { job })
    }

    /// Resumes every thread of the process `id`: a process started
    /// suspended has its main thread alone. A thread that ends before it is
    /// opened is passed over; only finding none to resume is an error.
    fn resume(id: u32) -> io::Result<()> {
        // SAFETY: plain flags; the process id is ignored for a thread
        // snapshot, which lists every thread on the system.
        let snapshot = owned(unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) })?;
        let mut entry = THREADENTRY32 {
            dwSize: u32::try_from(size_of::<THREADENTRY32>()).map_err(io::Error::other)?,
            ..THREADENTRY32::default()
        };
        let mut resumed = 0;
        // SAFETY: a live snapshot, and `entry` with its size set, as both
        // calls require.
        let mut more = unsafe { Thread32First(snapshot.as_raw_handle(), &raw mut entry) } != 0;
        while more {
            if entry.th32OwnerProcessID == id {
                // SAFETY: plain arguments; the result is checked.
                let opened =
                    owned(unsafe { OpenThread(THREAD_SUSPEND_RESUME, 0, entry.th32ThreadID) });
                if let Ok(thread) = opened {
                    // SAFETY: a live thread handle opened with the right to
                    // resume it.
                    if unsafe { ResumeThread(thread.as_raw_handle()) } == u32::MAX {
                        return Err(io::Error::last_os_error());
                    }
                    resumed += 1;
                }
            }
            // SAFETY: as `Thread32First`.
            more = unsafe { Thread32Next(snapshot.as_raw_handle(), &raw mut entry) } != 0;
        }
        if resumed == 0 {
            return Err(io::Error::other(format!(
                "process {id} has no thread to resume"
            )));
        }
        Ok(())
    }

    /// Kills every process in `job`.
    pub(super) fn terminate(job: &OwnedHandle) {
        // SAFETY: a live job handle.
        if unsafe { TerminateJobObject(job.as_raw_handle(), KILLED) } == 0 {
            tracing::warn!(
                "a script's job did not stop: {}",
                io::Error::last_os_error()
            );
        }
    }
}
