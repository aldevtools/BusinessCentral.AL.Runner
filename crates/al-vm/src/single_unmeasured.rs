//! SingleInstance codeunits whose state on BC this run does not know (al-runner issue 8239): every read of one of their
//! globals by name is refused, from any code (a test, session-open code, the runner's steps around a test, a handler, a
//! subscriber a RecordRef write reaches); a write reads nothing and runs. The caller that found the state unknown marks
//! the instance with the reason ([`Session::mark_single_unmeasured`], `Instance::unmeasured`); the read checks it
//! (`Session::global_value`, interp.rs). The mark is on the instance: it moves with it to a later session
//! (`take_single_instances`, `adopt_single_instances`) and goes when the session drops it (`reset_single_instances`).

use crate::Session;
use al_builtins::native::Signal;
use std::rc::Rc;

impl Session {
    /// Marks the single instance of codeunit `object`, if the session has one: what it holds is not BC's value, for `why`.
    pub fn mark_single_unmeasured(&mut self, object: usize, why: Rc<str>) {
        if let Some(inst) = self.singletons.get(&object) {
            inst.borrow_mut().unmeasured = Some(why);
        }
    }
}

/// The refusal of a read of a global of a marked single instance (`Instance::unmeasured`).
pub(crate) fn read_refused(why: &str) -> Signal {
    Signal::unsupported(why.to_string())
}
