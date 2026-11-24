//! Names the test a crashed process was running. A fault the process cannot survive (SIGSEGV, SIGBUS, SIGILL,
//! SIGFPE) or an abort (SIGABRT: a corrupt heap found by the C library, a stack overflow) ends the whole run before
//! any report is written; without this, the run just stops and the cause looks like fewer or missing outcomes. Each worker thread records which test it runs (`running`); a signal handler,
//! installed once by the command line (`install`), writes that test to stderr and then hands the signal on to the
//! handler that was there before (Rust's stack-overflow handler, or the default action, which dumps core).
//!
//! Names the test a crashed process was running: each worker records its codeunit and test
//! (`running`, from `events::starting` and `run_unit`); a handler for SIGSEGV, SIGBUS, SIGILL, SIGFPE
//! and SIGABRT, installed by both command lines (`install`), writes it to stderr and hands the signal
//! on (Rust's stack-overflow handler or the default action). Entry points: `install`, `running`,
//! `idle`.

use std::cell::Cell;

/// Room for "codeunit N test NAME"; a longer name is cut.
const NAME_MAX: usize = 240;

thread_local! {
    /// The codeunit and test this thread runs: (codeunit id + 1, or 0 for none; name length; name bytes). Plain
    /// data with a const initializer and no destructor, so the signal handler can read it.
    static RUNNING: Cell<(u32, usize, [u8; NAME_MAX])> = const { Cell::new((0, 0, [0; NAME_MAX])) };
}

/// Records that this thread now runs `test` of codeunit `codeunit` (an empty `test`: the codeunit's own setup).
pub(crate) fn running(codeunit: u32, test: &str) {
    let mut buf = [0u8; NAME_MAX];
    let mut n = test.len().min(NAME_MAX);
    while !test.is_char_boundary(n) {
        n -= 1;
    }
    buf[..n].copy_from_slice(&test.as_bytes()[..n]);
    RUNNING.with(|c| c.set((codeunit.saturating_add(1), n, buf)));
}

/// Records that this thread runs no test.
pub(crate) fn idle() {
    RUNNING.with(|c| c.set((0, 0, [0; NAME_MAX])));
}

/// Installs the handler, once per process. Call it from `main` after the Rust runtime is set up, so its
/// stack-overflow handler is the one handed on to.
pub fn install() {
    #[cfg(unix)]
    unix::install();
}

#[cfg(unix)]
mod unix {
    use super::{NAME_MAX, RUNNING};
    use std::cell::UnsafeCell;
    use std::sync::Once;
    use std::sync::atomic::{AtomicBool, Ordering};

    const SIGNALS: [libc::c_int; 5] = [libc::SIGSEGV, libc::SIGBUS, libc::SIGILL, libc::SIGFPE, libc::SIGABRT];

    /// The handlers that were installed before ours, by the position of the signal in `SIGNALS`. Written once in
    /// `install`, before our handler can run; only read after that.
    struct Previous(UnsafeCell<[libc::sigaction; 5]>);
    // SAFETY: written only inside `INSTALL.call_once`, before the handler that reads it is installed.
    unsafe impl Sync for Previous {}
    static PREVIOUS: Previous = Previous(UnsafeCell::new(unsafe { std::mem::zeroed() }));
    static INSTALL: Once = Once::new();

    pub(super) fn install() {
        INSTALL.call_once(|| {
            for (i, &sig) in SIGNALS.iter().enumerate() {
                // SAFETY: plain libc calls on a zeroed `sigaction`; `PREVIOUS` is not read before this returns.
                unsafe {
                    let mut new: libc::sigaction = std::mem::zeroed();
                    new.sa_sigaction = handler as *const () as usize;
                    new.sa_flags = libc::SA_SIGINFO | libc::SA_ONSTACK;
                    libc::sigemptyset(&mut new.sa_mask);
                    let prev = &mut (*PREVIOUS.0.get())[i];
                    if libc::sigaction(sig, &new, prev) != 0 {
                        *prev = std::mem::zeroed();
                    }
                }
            }
        });
    }

    /// Writes all of `bytes` to stderr (async-signal-safe).
    fn put(bytes: &[u8]) {
        let mut rest = bytes;
        while !rest.is_empty() {
            // SAFETY: `write` on fd 2 with a valid buffer.
            let n = unsafe { libc::write(2, rest.as_ptr().cast(), rest.len()) };
            if n <= 0 {
                return;
            }
            rest = &rest[n as usize..];
        }
    }

    /// `n` in decimal, into `buf`; the digits written.
    fn digits(mut n: u64, buf: &mut [u8; 20]) -> &[u8] {
        let mut i = buf.len();
        loop {
            i -= 1;
            buf[i] = b'0' + (n % 10) as u8;
            n /= 10;
            if n == 0 {
                return &buf[i..];
            }
        }
    }

    /// Whether the crash was told already: Rust's stack-overflow handler aborts, and the abort is not told again.
    static TOLD: AtomicBool = AtomicBool::new(false);

    /// Writes which test this thread runs. Kept out of `handler`, so a second, nested signal (the abort after a
    /// stack overflow) needs little of the small signal stack.
    #[inline(never)]
    fn tell(sig: libc::c_int) {
        let name: &[u8] = match sig {
            libc::SIGSEGV => b"SIGSEGV",
            libc::SIGBUS => b"SIGBUS",
            libc::SIGILL => b"SIGILL",
            libc::SIGABRT => b"SIGABRT",
            _ => b"SIGFPE",
        };
        // SAFETY: the cell belongs to this thread; read in place, without copying it onto the signal stack.
        let (codeunit, len, test) = unsafe { &*RUNNING.with(|c| c.as_ptr()) };
        let mut num = [0u8; 20];
        put(b"\nerror: the process crashed (");
        put(name);
        put(b"). ");
        if *codeunit == 0 {
            put(b"No test was running on the thread that crashed (compile or setup).");
        } else {
            put(b"Running on the thread that crashed: codeunit ");
            put(digits(u64::from(codeunit - 1), &mut num));
            if *len == 0 {
                put(b" (before its first test)");
            } else {
                put(b" test ");
                put(&test[..(*len).min(NAME_MAX)]);
            }
            put(b".");
        }
        put(b" No outcome of this run was reported. A rerun of the same input that is clean points to the machine (an unstable CPU core or RAM), not to the test.\n");
    }

    extern "C" fn handler(sig: libc::c_int, info: *mut libc::siginfo_t, ctx: *mut libc::c_void) {
        if !TOLD.swap(true, Ordering::SeqCst) {
            tell(sig);
        }
        let i = SIGNALS.iter().position(|&s| s == sig).unwrap_or(0);
        // SAFETY: `PREVIOUS` was filled before this handler was installed. Restoring a handler and raising a signal
        // are async-signal-safe.
        unsafe {
            let prev = &(*PREVIOUS.0.get())[i];
            let user = !info.is_null() && (*info).si_code <= 0;
            if user {
                // Sent by kill or raise: returning would not raise it again, so take the default action now.
                libc::signal(sig, libc::SIG_DFL);
                libc::raise(sig);
                return;
            }
            let action = prev.sa_sigaction;
            if action == libc::SIG_DFL || action == libc::SIG_IGN {
                libc::signal(sig, libc::SIG_DFL);
                // Returning re-runs the faulting instruction, which now takes the default action (core dump).
                return;
            }
            if prev.sa_flags & libc::SA_SIGINFO != 0 {
                let f: extern "C" fn(libc::c_int, *mut libc::siginfo_t, *mut libc::c_void) =
                    std::mem::transmute(action);
                f(sig, info, ctx);
            } else {
                let f: extern "C" fn(libc::c_int) = std::mem::transmute(action);
                f(sig);
            }
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use std::os::unix::process::ExitStatusExt;

    /// Run in a child process by `a_crash_names_the_running_test`: records a running test, then crashes.
    #[test]
    fn crash_child() {
        let Some(mode) = std::env::var_os("AL_CRASH_CHILD") else {
            return;
        };
        // No core dump of this planned crash (the machine's crash reporter would show it).
        #[cfg(target_os = "linux")]
        unsafe {
            libc::prctl(libc::PR_SET_DUMPABLE, 0);
        }
        super::install();
        crate::events::starting(134, "PostInvoiceTwice");
        if mode == "overflow" {
            #[allow(unconditional_recursion)]
            fn deep(n: u64) -> u64 {
                let pad = std::hint::black_box([n; 64]);
                deep(n + 1) + pad[7]
            }
            std::hint::black_box(deep(0));
        } else if mode == "abort" {
            // As the C library does when `free` finds a corrupt heap.
            std::process::abort();
        } else if mode == "fault" {
            // A real fault on a null pointer, as the crashes seen in the runner were.
            unsafe { std::ptr::read_volatile(std::ptr::null::<u8>().wrapping_add(13)) };
        } else {
            unsafe { libc::raise(libc::SIGSEGV) };
        }
        unreachable!("the child survived its crash");
    }

    fn crash(mode: &str) -> (std::process::ExitStatus, String) {
        let out = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "crash::tests::crash_child",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("AL_CRASH_CHILD", mode)
            .output()
            .unwrap();
        (out.status, String::from_utf8_lossy(&out.stderr).into_owned())
    }

    #[test]
    fn a_crash_names_the_running_test() {
        for (mode, sig, name) in [
            ("fault", libc::SIGSEGV, "SIGSEGV"),
            ("raise", libc::SIGSEGV, "SIGSEGV"),
            ("abort", libc::SIGABRT, "SIGABRT"),
        ] {
            let (status, stderr) = crash(mode);
            assert_eq!(status.signal(), Some(sig), "{mode}: {status}\n{stderr}");
            assert!(
                stderr.contains(&format!("crashed ({name})")) && stderr.contains("codeunit 134 test PostInvoiceTwice"),
                "{mode}: {stderr}"
            );
        }
    }

    /// A stack overflow still reaches Rust's own handler, which names it and aborts.
    #[test]
    fn a_stack_overflow_is_still_named() {
        let (status, stderr) = crash("overflow");
        assert_eq!(status.signal(), Some(libc::SIGABRT), "{status}\n{stderr}");
        assert!(stderr.contains("has overflowed its stack"), "{stderr}");
        assert!(stderr.contains("codeunit 134 test PostInvoiceTwice"), "{stderr}");
    }
}
