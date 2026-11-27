//! How `al-runner serve` stops without losing the workspace layer entry it is still writing (issue 9232). Serve writes
//! the entry of a request's compile on a thread after the response (ADR-022 amendment 2026-11-15, decision 6). The
//! `shutdown` reply and a stop signal (SIGTERM and SIGINT; Ctrl-C and Ctrl-Break on Windows) wait for the pending
//! writes first, for at most [`WRITE_BOUND`]. Entry points: `install`, `finish_writes`.

use std::time::Duration;

/// How long the `shutdown` reply and a stop signal wait for the pending workspace layer writes.
///
/// Measured for issue 9232 (release build, Linux, an entry of 2.1 MB for 4,000 objects): the encode and write take
/// 0.75 s after the response on an idle machine and 1.4 to 1.7 s under load. The editor clients stop the server soon
/// after `shutdown`: one sends SIGTERM as soon as the reply comes or after 500 ms without one; the other sends SIGTERM
/// at once and SIGKILL 2 s later (`docs/integrations/alchemist-lethal.md`, A14 and S8). 1.8 s covers the slowest
/// measured write, which started at the response before `shutdown`, so it had already run when the wait starts, and
/// leaves 0.2 s for the exit before the SIGKILL, which would end the process without a word.
pub(crate) const WRITE_BOUND: Duration = Duration::from_millis(1800);

/// Waits for the pending workspace layer writes for at most [`WRITE_BOUND`]. When the bound passes first, says so on
/// stderr: `then` is what the server does next.
pub(crate) fn finish_writes(why: &str, then: &str) {
    if !al_testrun::workspace_cache::wait_for_writes_within(WRITE_BOUND) {
        eprintln!(
            "[server] {why}: a workspace cache entry was still being written after {} ms; {then}",
            WRITE_BOUND.as_millis()
        );
    }
}

/// Makes a stop signal wait for the pending writes ([`finish_writes`]) before the process ends as the signal ends it.
/// A second signal ends it at once. Once per process.
pub(crate) fn install() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(imp::install);
}

#[cfg(unix)]
mod imp {
    use std::io::Read;
    use std::os::fd::IntoRawFd;
    use std::os::unix::net::UnixStream;
    use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

    const SIGNALS: [libc::c_int; 2] = [libc::SIGTERM, libc::SIGINT];

    /// The end of the socket pair the handler writes the signal number to; the stop thread reads the other end.
    static WAKE: AtomicI32 = AtomicI32::new(-1);
    /// A stop signal came already.
    static STOPPING: AtomicBool = AtomicBool::new(false);

    /// The handler: wakes the stop thread (`write` is async-signal-safe; waiting for a thread is not), or, for a
    /// second signal, ends the process at once as without a handler.
    extern "C" fn on_signal(sig: libc::c_int) {
        if STOPPING.swap(true, Ordering::SeqCst) {
            end_by(sig);
            return;
        }
        let b = sig as u8;
        // SAFETY: `write` of one byte from a valid buffer; a closed or unset descriptor only makes it fail.
        unsafe { libc::write(WAKE.load(Ordering::SeqCst), (&b as *const u8).cast(), 1) };
    }

    /// Ends the process by `sig` with its default action, so the exit status is the one the signal gives without a
    /// handler (`signal` and `raise` are async-signal-safe).
    fn end_by(sig: libc::c_int) {
        // SAFETY: plain libc calls; the default action of SIGTERM and SIGINT ends the process.
        unsafe {
            libc::signal(sig, libc::SIG_DFL);
            libc::raise(sig);
        }
    }

    pub(super) fn install() {
        let Ok((mut stop, wake)) = UnixStream::pair() else {
            return;
        };
        let spawned = std::thread::Builder::new().name("serve-stop".into()).spawn(move || {
            let mut b = [0u8; 1];
            if stop.read_exact(&mut b).is_ok() {
                let sig = libc::c_int::from(b[0]);
                super::finish_writes("stop signal", "ending without it");
                end_by(sig);
            }
        });
        if spawned.is_err() {
            return;
        }
        WAKE.store(wake.into_raw_fd(), Ordering::SeqCst);
        for sig in SIGNALS {
            // SAFETY: a zeroed `sigaction` with our handler and an empty mask; the handler is async-signal-safe.
            unsafe {
                let mut new: libc::sigaction = std::mem::zeroed();
                new.sa_sigaction = on_signal as *const () as usize;
                new.sa_flags = libc::SA_RESTART;
                libc::sigemptyset(&mut new.sa_mask);
                libc::sigaction(sig, &new, std::ptr::null_mut());
            }
        }
    }
}

#[cfg(windows)]
mod imp {
    use std::sync::atomic::{AtomicBool, Ordering};

    const CTRL_C_EVENT: u32 = 0;
    const CTRL_BREAK_EVENT: u32 = 1;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn SetConsoleCtrlHandler(handler: Option<unsafe extern "system" fn(u32) -> i32>, add: i32) -> i32;
    }

    /// A stop event came already.
    static STOPPING: AtomicBool = AtomicBool::new(false);

    /// Windows calls this on a thread of its own. FALSE hands the event on to the default handler, which ends the
    /// process as Ctrl-C ends it without a handler. The first Ctrl-C or Ctrl-Break waits for the pending writes; a
    /// second one ends the process at once, as on Unix.
    unsafe extern "system" fn on_ctrl(event: u32) -> i32 {
        if (event == CTRL_C_EVENT || event == CTRL_BREAK_EVENT) && !STOPPING.swap(true, Ordering::SeqCst) {
            super::finish_writes("stop signal", "ending without it");
        }
        0
    }

    pub(super) fn install() {
        // SAFETY: registers a handler that lives as long as the process.
        unsafe { SetConsoleCtrlHandler(Some(on_ctrl), 1) };
    }
}

#[cfg(not(any(unix, windows)))]
mod imp {
    pub(super) fn install() {}
}
