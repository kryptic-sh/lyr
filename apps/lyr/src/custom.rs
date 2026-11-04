//! Running custom modules' scripts (`docs/plan/07-modules.md#custom`): one
//! thread and one script per module per bar, handing each update to the main
//! loop over a channel and waking it, as the sources do.
//!
//! A script runs with `WAYBAR_OUTPUT_NAME` and `LYR_OUTPUT_NAME` set to its
//! bar's output. How often it runs is its [`Runs`]: each interval, never two
//! runs at once; once, then again on a signal or a click; or on and on, each
//! line it prints an update. A script's process tree is stopped when its bar
//! goes, on a reload ([`Runner::restart`]), and when the bar quits (the
//! runner is dropped).

use std::io::{self, BufRead as _, BufReader, Read as _};
use std::process::ExitStatus;
use std::sync::{Arc, Mutex, PoisonError, mpsc};
use std::time::Duration;

use crcbl_shell::ShellWaker;
use lyr_config::{Config, Module, ReturnType, Runs};
use lyr_core::custom::{Output, parse};

use crate::exec::{self, Tree};

/// One script's new output for the bar on `output`: `None` leaves the
/// module out.
#[derive(Debug)]
pub struct Shown {
    /// The module, by its index in [`Config::custom`].
    pub module: usize,
    pub output: Option<String>,
    pub shown: Option<Output>,
}

/// A [`Shown`] from the scripts of one config.
#[derive(Debug)]
struct Message {
    generation: u64,
    shown: Shown,
}

/// Every running script, and the channel their updates come in on.
pub struct Runner {
    waker: Option<ShellWaker>,
    sender: mpsc::Sender<Message>,
    receiver: mpsc::Receiver<Message>,
    /// Counts the reloads, so an update from a script of an older config,
    /// whose module index may now name another module, is dropped.
    generation: u64,
    scripts: Vec<Script>,
}

/// One running script, as the main loop holds it.
struct Script {
    module: usize,
    output: Option<String>,
    signal: Option<u32>,
    /// A message asks for a run now; dropped, it tells the thread to stop.
    wake: mpsc::Sender<()>,
    slot: Arc<Mutex<Slot>>,
}

/// What a script's thread and the main loop share.
#[derive(Debug, Default)]
struct Slot {
    /// Set once the script is to stop: no process starts after it.
    stopped: bool,
    /// The tree of the last process started.
    tree: Option<Tree>,
}

impl Script {
    /// Stops its process tree and its thread.
    fn stop(self) {
        let mut slot = self.slot.lock().unwrap_or_else(PoisonError::into_inner);
        slot.stopped = true;
        if let Some(tree) = slot.tree.take() {
            tree.kill();
        }
        // Dropping `wake` wakes a thread that waits for its next run.
    }
}

/// What a script's thread runs, copied out of the config.
struct Job {
    /// `custom/<name>`, for the log.
    name: String,
    exec: String,
    exec_if: Option<String>,
    return_type: ReturnType,
    runs: Runs,
    on_update: Option<String>,
    /// The bar's output, for the environment.
    output: Option<String>,
}

/// What one run of a script came to.
enum Step {
    /// The script was stopped meanwhile.
    Stopped,
    Show(Option<Output>),
    /// Its output was not readable: what the bar shows is kept.
    Keep,
}

impl Runner {
    pub fn new(waker: Option<ShellWaker>) -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            waker,
            sender,
            receiver,
            generation: 0,
            scripts: Vec::new(),
        }
    }

    /// Runs the script of each custom module `config`'s zones show, on each
    /// bar's output in `outputs`, and stops those no longer wanted.
    pub fn sync(&mut self, config: &Config, outputs: &[Option<&str>]) {
        let mut wanted: Vec<(usize, Option<&str>)> = Vec::new();
        for module in config.shown() {
            let Module::Custom(index) = module else {
                continue;
            };
            let Some(custom) = config.custom.get(index) else {
                continue;
            };
            if custom.exec.is_none() {
                continue;
            }
            for &output in outputs {
                if !wanted.contains(&(index, output)) {
                    wanted.push((index, output));
                }
            }
        }
        let (kept, gone): (Vec<Script>, Vec<Script>) = std::mem::take(&mut self.scripts)
            .into_iter()
            .partition(|script| wanted.contains(&(script.module, script.output.as_deref())));
        self.scripts = kept;
        gone.into_iter().for_each(Script::stop);
        for (index, output) in wanted {
            if self.running(index, output).is_some() {
                continue;
            }
            let custom = &config.custom[index];
            let Some(exec) = custom.exec.clone() else {
                continue;
            };
            let Some(runs) = custom.runs() else {
                continue;
            };
            let job = Job {
                name: format!("{}{}", lyr_config::custom::PREFIX, custom.name),
                exec,
                exec_if: custom.exec_if.clone(),
                return_type: custom.return_type,
                runs,
                on_update: custom.on_update.clone(),
                output: output.map(str::to_owned),
            };
            match self.start(index, custom.signal, job) {
                Ok(script) => self.scripts.push(script),
                Err(error) => {
                    tracing::warn!("custom/{}: its thread did not start: {error}", custom.name)
                }
            }
        }
    }

    /// Starts `job`'s thread for the module `index`, run again by `signal`.
    fn start(&self, index: usize, signal: Option<u32>, job: Job) -> io::Result<Script> {
        let (wake, woken) = mpsc::channel();
        let slot = Arc::new(Mutex::new(Slot::default()));
        let output = job.output.clone();
        let sender = self.sender.clone();
        let waker = self.waker.clone();
        let generation = self.generation;
        let thread_slot = Arc::clone(&slot);
        std::thread::Builder::new()
            .name(format!("lyr {}", job.name))
            .spawn(move || {
                let deliver = |shown: Option<Output>| {
                    let message = Message {
                        generation,
                        shown: Shown {
                            module: index,
                            output: job.output.clone(),
                            shown,
                        },
                    };
                    let delivered = sender.send(message).is_ok();
                    if let Some(waker) = &waker {
                        waker.wake();
                    }
                    if let Some(command) = &job.on_update
                        && let Err(error) = exec::spawn(command)
                    {
                        tracing::warn!(
                            "{}: on_update `{command}` did not start: {error}",
                            job.name
                        );
                    }
                    delivered
                };
                serve(&job, &woken, &thread_slot, deliver);
            })?;
        Ok(Script {
            module: index,
            output,
            signal,
            wake,
            slot,
        })
    }

    /// The running script of module `index` on `output`.
    fn running(&self, index: usize, output: Option<&str>) -> Option<&Script> {
        self.scripts
            .iter()
            .find(|script| script.module == index && script.output.as_deref() == output)
    }

    /// Stops every script, for a new config whose module indices may differ;
    /// [`Self::sync`] starts the new config's.
    pub fn restart(&mut self) {
        self.scripts.drain(..).for_each(Script::stop);
        self.generation += 1;
    }

    /// Runs the script of module `index` on the bar on `output` now, as a
    /// click does; a continuous one runs on regardless.
    pub fn run(&self, index: usize, output: Option<&str>) {
        if let Some(script) = self.running(index, output) {
            // A thread that has stopped already logged why.
            let _stopped = script.wake.send(());
        }
    }

    /// Runs every script whose `signal` is `number` now. Returns how many
    /// there are.
    pub fn signal(&self, number: u32) -> usize {
        self.scripts
            .iter()
            .filter(|script| script.signal == Some(number))
            .inspect(|script| {
                // As in `run`.
                let _stopped = script.wake.send(());
            })
            .count()
    }

    /// The next update from a script of the current config, if one came.
    pub fn recv(&self) -> Option<Shown> {
        loop {
            let message = self.receiver.try_recv().ok()?;
            if message.generation == self.generation {
                return Some(message.shown);
            }
        }
    }
}

impl Drop for Runner {
    fn drop(&mut self) {
        self.scripts.drain(..).for_each(Script::stop);
    }
}

/// A script's thread: runs `job` as its [`Runs`] says until it is stopped
/// or the main loop has gone, handing each update to `deliver`.
fn serve(
    job: &Job,
    woken: &mpsc::Receiver<()>,
    slot: &Mutex<Slot>,
    deliver: impl Fn(Option<Output>) -> bool,
) {
    if let Runs::Continuous { restart } = job.runs {
        continuous(job, restart, woken, slot, deliver);
        return;
    }
    loop {
        match update(job, slot) {
            Step::Stopped => return,
            Step::Show(shown) => {
                if !deliver(shown) {
                    return;
                }
            }
            Step::Keep => {}
        }
        let next = match job.runs {
            Runs::Every(every) => woken.recv_timeout(every).or_else(|error| match error {
                mpsc::RecvTimeoutError::Timeout => Ok(()),
                mpsc::RecvTimeoutError::Disconnected => Err(()),
            }),
            Runs::Once | Runs::Continuous { .. } => woken.recv().map_err(drop),
        };
        if next.is_err() {
            return;
        }
        // Asks that came in during the run are answered by the one coming.
        while woken.try_recv().is_ok() {}
    }
}

/// One run of an interval or one-off script, `exec_if` first.
fn update(job: &Job, slot: &Mutex<Slot>) -> Step {
    if let Some(condition) = &job.exec_if {
        match run_to_end(condition, job, slot) {
            Ok(None) => return Step::Stopped,
            Ok(Some((_, status))) if !status.success() => return Step::Show(None),
            Ok(Some(_)) => {}
            Err(error) => {
                tracing::warn!("{}: exec_if `{condition}` did not run: {error}", job.name);
                return Step::Show(None);
            }
        }
    }
    match run_to_end(&job.exec, job, slot) {
        Ok(None) => Step::Stopped,
        // A failed run leaves the module out, as waybar does.
        Ok(Some((_, status))) if !status.success() => Step::Show(None),
        Ok(Some((out, _))) => read(job, &out),
        Err(error) => {
            tracing::warn!("{}: `{}` did not run: {error}", job.name, job.exec);
            Step::Show(None)
        }
    }
}

/// `out` read as `job`'s return type; an unreadable one is logged and
/// kept from the bar.
fn read(job: &Job, out: &str) -> Step {
    match parse(out, job.return_type) {
        Ok(shown) => Step::Show(shown),
        Err(error) => {
            tracing::warn!("{}: {error}", job.name);
            Step::Keep
        }
    }
}

/// Starts `command` for `job` in `slot`, unless the script was stopped.
fn start(command: &str, job: &Job, slot: &Mutex<Slot>) -> io::Result<Option<std::process::Child>> {
    let mut slot = slot.lock().unwrap_or_else(PoisonError::into_inner);
    if slot.stopped {
        return Ok(None);
    }
    let env: Vec<(&str, &str)> = job
        .output
        .as_deref()
        .map(|output| [("WAYBAR_OUTPUT_NAME", output), ("LYR_OUTPUT_NAME", output)])
        .into_iter()
        .flatten()
        .collect();
    let (child, tree) = exec::script(command, &env)?;
    slot.tree = Some(tree);
    Ok(Some(child))
}

/// Whether the script was stopped.
fn stopped(slot: &Mutex<Slot>) -> bool {
    slot.lock().unwrap_or_else(PoisonError::into_inner).stopped
}

/// Runs `command` for `job` to its end: its output, read as UTF-8 with
/// anything else replaced, and how it exited. `None` when the script was
/// stopped.
fn run_to_end(
    command: &str,
    job: &Job,
    slot: &Mutex<Slot>,
) -> io::Result<Option<(String, ExitStatus)>> {
    let Some(mut child) = start(command, job, slot)? else {
        return Ok(None);
    };
    let mut out = Vec::new();
    if let Some(mut stdout) = child.stdout.take() {
        stdout.read_to_end(&mut out)?;
    }
    let status = child.wait()?;
    reaped(slot);
    if stopped(slot) {
        return Ok(None);
    }
    Ok(Some((String::from_utf8_lossy(&out).into_owned(), status)))
}

/// Forgets the tree of the process just waited for, on Unix: once its
/// leader is reaped, the group's id may be given to another process group,
/// which stopping the script would then signal. A Windows job handle names
/// its job alone, so it is kept: the next run's replaces it, or stopping the
/// script ends what the run left behind.
fn reaped(slot: &Mutex<Slot>) {
    if cfg!(unix) {
        slot.lock().unwrap_or_else(PoisonError::into_inner).tree = None;
    }
}

/// A continuous script: each line it prints is an update. Once it exits it
/// is run again after `restart`, when set; else it stays as it last was.
fn continuous(
    job: &Job,
    restart: Option<Duration>,
    woken: &mpsc::Receiver<()>,
    slot: &Mutex<Slot>,
    deliver: impl Fn(Option<Output>) -> bool,
) {
    loop {
        let gated = match &job.exec_if {
            None => Ok(Some(true)),
            Some(condition) => {
                run_to_end(condition, job, slot).map(|ran| ran.map(|(_, status)| status.success()))
            }
        };
        match gated {
            Ok(None) => return,
            Ok(Some(true)) => match lines(job, slot, &deliver) {
                // A script that failed leaves the module out, as waybar does.
                Ok(Some(status)) => {
                    if !status.success() && !deliver(None) {
                        return;
                    }
                }
                Ok(None) => return,
                Err(error) => {
                    tracing::warn!("{}: `{}` did not run: {error}", job.name, job.exec);
                }
            },
            Ok(Some(false)) => {
                if !deliver(None) {
                    return;
                }
            }
            Err(error) => {
                tracing::warn!("{}: exec_if did not run: {error}", job.name);
            }
        }
        if stopped(slot) {
            return;
        }
        let Some(restart) = restart else {
            tracing::info!(
                "{}: the script ended, and has no restart_interval",
                job.name
            );
            // Until it is stopped: what it asks for is a run, which a
            // continuous script does not take.
            while woken.recv().is_ok() {}
            return;
        };
        // A click while the script ran is no reason to cut the wait short.
        while woken.try_recv().is_ok() {}
        if let Err(mpsc::RecvTimeoutError::Disconnected) = woken.recv_timeout(restart) {
            return;
        }
    }
}

/// Runs `job`'s continuous script, handing each line to `deliver`, until it
/// ends: how it exited, or `None` once the script was stopped or the main
/// loop has gone.
fn lines(
    job: &Job,
    slot: &Mutex<Slot>,
    deliver: impl Fn(Option<Output>) -> bool,
) -> io::Result<Option<ExitStatus>> {
    let Some(mut child) = start(&job.exec, job, slot)? else {
        return Ok(None);
    };
    let read = child
        .stdout
        .take()
        .map_or(Ok(true), |stdout| each_line(job, stdout, &deliver));
    if !matches!(read, Ok(true)) {
        // The main loop has gone, or the pipe broke: the script would run
        // on unread.
        let slot = slot.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(tree) = &slot.tree {
            tree.kill();
        }
    }
    // Waited for even when reading failed, so it does not linger.
    let status = child.wait()?;
    reaped(slot);
    Ok((read? && !stopped(slot)).then_some(status))
}

/// Hands each line of `stdout` to `deliver` until it ends. `false` once the
/// main loop has gone.
fn each_line(
    job: &Job,
    stdout: std::process::ChildStdout,
    deliver: impl Fn(Option<Output>) -> bool,
) -> io::Result<bool> {
    let mut stdout = BufReader::new(stdout);
    let mut line = Vec::new();
    while stdout.read_until(b'\n', &mut line)? > 0 {
        let text = String::from_utf8_lossy(&line);
        // A Windows script's lines end in CRLF.
        let text = text.trim_end_matches('\n').trim_end_matches('\r');
        if let Step::Show(shown) = read(job, text)
            && !deliver(shown)
        {
            return Ok(false);
        }
        line.clear();
    }
    Ok(true)
}

#[cfg(test)]
mod tests;
