//! A thread running a Core Foundation run loop, for the macOS sources whose
//! change notifications arrive as run loop sources.

use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread::JoinHandle;

use objc2_core_foundation::{CFRetained, CFRunLoop};

/// The thread's run loop, as the one handle another thread may hold.
struct RunLoop(CFRetained<CFRunLoop>);

// SAFETY: the handle is used from another thread only through `stop`, and
// Core Foundation's run loop functions are thread-safe (Apple's Threading
// Programming Guide, "Run Loop Objects"); `CFRunLoopStop` and
// `CFRunLoopWakeUp` exist to be called from other threads.
unsafe impl Send for RunLoop {}

impl RunLoop {
    /// Stops the loop if it is running, and wakes it so it notices.
    fn stop(&self) {
        self.0.stop();
        self.0.wake_up();
    }
}

/// A thread whose run loop runs until this is dropped.
pub struct RunLoopThread {
    run_loop: RunLoop,
    /// Set before stopping, so a loop stopped before it began running does
    /// not run again.
    stopping: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl std::fmt::Debug for RunLoopThread {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunLoopThread").finish_non_exhaustive()
    }
}

impl RunLoopThread {
    /// Starts a thread named `name`. `setup` runs on it first: it adds its
    /// sources to the thread's run loop and returns what must live while the
    /// loop runs, which is dropped on the thread once the loop has stopped.
    ///
    /// # Errors
    ///
    /// When the thread cannot start, has no run loop, or `setup` fails.
    pub fn spawn<T: 'static>(
        name: &str,
        setup: impl FnOnce(&CFRunLoop) -> io::Result<T> + Send + 'static,
    ) -> io::Result<Self> {
        let (started, start) = mpsc::channel();
        let stopping = Arc::new(AtomicBool::new(false));
        let stop = stopping.clone();
        let thread = std::thread::Builder::new()
            .name(name.to_owned())
            .spawn(move || {
                let Some(run_loop) = CFRunLoop::current() else {
                    let _gone = started.send(Err(io::Error::other("no run loop")));
                    return;
                };
                let kept = match setup(&run_loop) {
                    Ok(kept) => kept,
                    Err(error) => {
                        let _gone = started.send(Err(error));
                        return;
                    }
                };
                let _gone = started.send(Ok(RunLoop(run_loop)));
                while !stop.load(Ordering::Acquire) {
                    CFRunLoop::run();
                }
                drop(kept);
            })?;
        let run_loop = start
            .recv()
            .map_err(|_| io::Error::other(format!("the {name} thread stopped")))??;
        Ok(Self {
            run_loop,
            stopping,
            thread: Some(thread),
        })
    }
}

impl Drop for RunLoopThread {
    fn drop(&mut self) {
        self.stopping.store(true, Ordering::Release);
        let Some(thread) = self.thread.take() else {
            return;
        };
        // A stop that lands before the loop starts running is lost, so it is
        // sent until the thread has seen the flag and ended.
        while !thread.is_finished() {
            self.run_loop.stop();
            std::thread::yield_now();
        }
        let _joined = thread.join();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    use super::RunLoopThread;

    #[test]
    fn what_setup_keeps_lives_until_the_thread_is_dropped() {
        struct Kept(Arc<AtomicBool>);
        impl Drop for Kept {
            fn drop(&mut self) {
                self.0.store(true, Ordering::Release);
            }
        }
        let dropped = Arc::new(AtomicBool::new(false));
        let kept = dropped.clone();
        let thread = RunLoopThread::spawn("lyr test", move |_| Ok(Kept(kept))).expect("started");
        assert!(!dropped.load(Ordering::Acquire), "kept while it runs");
        drop(thread);
        assert!(dropped.load(Ordering::Acquire), "dropped once stopped");

        let refused = RunLoopThread::spawn("lyr test", |_| -> std::io::Result<()> {
            Err(std::io::Error::other("no source"))
        });
        assert!(refused.is_err());
    }
}
