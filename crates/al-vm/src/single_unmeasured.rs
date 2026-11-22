//! SingleInstance codeunits whose state the run found somewhere no pin covers (al-runner issue 8239). The caller that
//! found it marks the instance (`Instance::unmeasured`): [`Session::mark_single_unmeasured`] when what it holds is not
//! BC's value, then every read of one of its globals by name is refused, from any code (a test, session-open code, the
//! runner's steps around a test, a handler, a subscriber a RecordRef write reaches), and a write reads nothing and runs;
//! [`Session::mark_single_assumed`] when an assumption entry gives BC's value, then every such read marks the entry. The
//! read checks it ([`read`], from `Session::global_value`, interp.rs). The mark is on the instance: it moves with it to a
//! later session (`take_single_instances`, `adopt_single_instances`) and goes when the session drops it
//! (`reset_single_instances`).

use crate::Session;
use al_builtins::assumptions::Assumption;
use al_builtins::native::Signal;
use al_builtins::value::SingleMark;
use std::rc::Rc;

impl Session {
    /// Marks the single instance of codeunit `object`, if the session has one: what it holds is not BC's value, for `why`.
    pub fn mark_single_unmeasured(&mut self, object: usize, why: Rc<str>) {
        if let Some(inst) = self.singletons.get(&object) {
            inst.borrow_mut().unmeasured = Some(SingleMark::Refused(why));
        }
    }

    /// Marks the single instance of codeunit `object`, if the session has one and it carries no mark yet: what it holds is
    /// BC's value by assumption entry `a`.
    pub fn mark_single_assumed(&mut self, object: usize, a: Assumption) {
        if let Some(inst) = self.singletons.get(&object) {
            let mut inst = inst.borrow_mut();
            if inst.unmeasured.is_none() {
                inst.unmeasured = Some(SingleMark::Assumed(a));
            }
        }
    }
}

/// A read of a global of a marked single instance: refused, or the assumption entry marked.
pub(crate) fn read(mark: &SingleMark) -> Result<(), Signal> {
    match mark {
        SingleMark::Refused(why) => Err(Signal::unsupported(why.to_string())),
        SingleMark::Assumed(a) => {
            al_builtins::assumptions::assume(*a);
            Ok(())
        }
    }
}
