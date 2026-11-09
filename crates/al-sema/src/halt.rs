//! Stopping a compile from outside (ADR-066 amendment 10, decision 6, extended to the compile): a hard memory limit
//! stops the run before the resident memory passes it, also while the apps compile.
//!
//! The caller arms the threads of a compile with a flag ([`arm`]): the thread that calls the compile, the threads of a
//! rayon pool it runs the compile in (its start handler arms them) and the threads the compile starts itself
//! ([`current`] is handed to them). Once the flag is set, [`check`] unwinds the armed thread with [`Halted`], without
//! the panic hook (`resume_unwind`), at the next object or file; the caller catches it and reports the run as stopped.
//! A thread that is not armed never stops, so a compile of another run in the same process goes on.

use std::cell::RefCell;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

thread_local! {
    static TOKEN: RefCell<Option<Arc<AtomicBool>>> = const { RefCell::new(None) };
}

/// The payload of the unwind [`check`] starts.
#[derive(Debug)]
pub struct Halted;

/// The flag this thread is armed with.
pub fn current() -> Option<Arc<AtomicBool>> {
    TOKEN.with(|t| t.borrow().clone())
}

/// Arms this thread with `token` until the guard drops (then the thread's earlier flag is back).
pub fn arm(token: Option<Arc<AtomicBool>>) -> Armed {
    Armed {
        before: TOKEN.with(|t| t.replace(token)),
    }
}

/// Arms this thread for good (a rayon pool's start handler: the thread lives only as long as its pool).
pub fn arm_thread(token: Option<Arc<AtomicBool>>) {
    TOKEN.with(|t| *t.borrow_mut() = token);
}

/// Restores the thread's earlier flag when it drops.
pub struct Armed {
    before: Option<Arc<AtomicBool>>,
}

impl Drop for Armed {
    fn drop(&mut self) {
        let before = self.before.take();
        TOKEN.with(|t| *t.borrow_mut() = before);
    }
}

/// Whether the flag this thread is armed with is set.
pub fn requested() -> bool {
    TOKEN.with(|t| t.borrow().as_ref().is_some_and(|f| f.load(Ordering::Relaxed)))
}

/// Unwinds this thread with [`Halted`] when its flag is set.
pub fn check() {
    if requested() {
        std::panic::resume_unwind(Box::new(Halted));
    }
}

/// Whether a panic payload is the unwind of [`check`].
pub fn is_halt(payload: &(dyn std::any::Any + Send)) -> bool {
    payload.is::<Halted>()
}

/// Goes on unwinding when `payload` is the unwind of [`check`]: a `catch_unwind` that turns a panic of one routine into
/// a refusal must not swallow a stop.
pub fn rethrow_halt(payload: Box<dyn std::any::Any + Send>) -> Box<dyn std::any::Any + Send> {
    if is_halt(&*payload) {
        std::panic::resume_unwind(payload);
    }
    payload
}

/// Joins a scoped thread of the compile: its result, its stop unwound further on this thread, or a panic that says
/// `what` for any other panic.
pub fn join<T>(handle: std::thread::ScopedJoinHandle<'_, T>, what: &str) -> T {
    match handle.join() {
        Ok(v) => v,
        Err(p) => {
            let _ = rethrow_halt(p);
            panic!("{what}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_armed_thread_stops_once_its_flag_is_set_and_others_go_on() {
        let flag = Arc::new(AtomicBool::new(false));
        let _armed = arm(Some(flag.clone()));
        check();
        flag.store(true, Ordering::SeqCst);
        let stopped = std::panic::catch_unwind(check).expect_err("stops");
        assert!(is_halt(&*stopped));
        // Another thread, not armed, goes on.
        std::thread::spawn(check).join().expect("not armed");
        // A thread armed with the flag handed to it stops too.
        let token = current();
        let other = std::thread::spawn(move || {
            let _armed = arm(token);
            check();
        });
        assert!(is_halt(&*other.join().expect_err("armed")));
    }

    #[test]
    fn the_guard_gives_back_the_earlier_flag() {
        let flag = Arc::new(AtomicBool::new(true));
        {
            let _armed = arm(Some(flag));
            assert!(requested());
        }
        assert!(!requested());
        check();
    }
}
