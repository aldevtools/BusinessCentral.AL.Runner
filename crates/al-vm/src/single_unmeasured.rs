//! SingleInstance codeunits whose state the run found somewhere no pin covers (al-runner issue 8239). The caller that
//! found it marks the instance (`Instance::unmeasured`): [`Session::mark_single_unmeasured`] when what it holds is not
//! BC's value, then every read of one of its globals by name is refused, from any code (a test, session-open code, the
//! runner's steps around a test, a handler, a subscriber a RecordRef write reaches), and a write reads nothing and runs;
//! [`Session::mark_single_assumed`] when an assumption entry gives BC's value, then every such read marks the entry. The
//! read checks it ([`read`], from `Session::global_value`, interp.rs). The mark is on the instance: it moves with it to a
//! later session (`take_single_instances`, `adopt_single_instances`) and goes when the session drops it
//! (`reset_single_instances`).
//!
//! SingleInstance codeunits whose state the run found somewhere no pin covers (al-runner issue 8239):
//! `Instance::unmeasured` holds `SingleMarks` (one for every global, and one each for some globals:
//! `mark_single_global_unmeasured`, al-runner issue 8945); `mark_single_unmeasured` (Refused) makes every read
//! of one of their globals by name (`interp.rs` `global_value`, through `read`) refused, a write
//! runs; `mark_single_assumed` (Assumed, only on an unmarked instance) makes every such read mark the
//! assumption entry. The mark moves with the instance to a later session. Set by al-testrun
//! `partner_login::raise`.

use crate::Session;
use al_builtins::assumptions::Assumption;
use al_builtins::native::Signal;
use al_builtins::value::SingleMark;
use std::rc::Rc;

impl Session {
    /// Marks the single instance of codeunit `object`, if the session has one: what it holds is not BC's value, for `why`.
    pub fn mark_single_unmeasured(&mut self, object: usize, why: Rc<str>) {
        if let Some(inst) = self.singletons.get(&object) {
            inst.borrow_mut().unmeasured.whole = Some(SingleMark::Refused(why));
        }
    }

    /// Marks global `global` of the single instance of codeunit `object`, if the session has one: what it holds is not
    /// BC's value, for `why`; its other globals keep their marks.
    pub fn mark_single_global_unmeasured(&mut self, object: usize, global: usize, why: Rc<str>) {
        if let Some(inst) = self.singletons.get(&object) {
            inst.borrow_mut()
                .unmeasured
                .globals
                .push((global, SingleMark::Refused(why)));
        }
    }

    /// Marks the single instance of codeunit `object`, if the session has one and it carries no mark yet: what it holds is
    /// BC's value by assumption entry `a`.
    pub fn mark_single_assumed(&mut self, object: usize, a: Assumption) {
        if let Some(inst) = self.singletons.get(&object) {
            let mut inst = inst.borrow_mut();
            if inst.unmeasured.whole.is_none() {
                inst.unmeasured.whole = Some(SingleMark::Assumed(a));
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

#[cfg(test)]
mod tests {
    use al_builtins::describe::Describe;

    // A mark on one global refuses the reads of that global only; the other globals of the instance read as before
    // (al-runner issue 8945: the part of a memo a runner-dependent setup ask changed).
    #[test]
    fn a_mark_on_one_global_refuses_only_its_reads() {
        const SRC: &str = "codeunit 50100 Keeper { SingleInstance = true; var A: Integer; B: Integer;
              procedure GetA(): Integer begin exit(A); end;
              procedure GetB(): Integer begin exit(B); end; }";
        let (ws, l) = al_sema::lower::compile(
            vec![al_sema::workspace::AppSource {
                id: "a".into(),
                name: "a".into(),
                dependencies: vec![],
                files: vec![("f.al".into(), SRC.into())],
            }],
            &Default::default(),
        );
        assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
        let engine = crate::Engine::new(l.program, Vec::new());
        let at = engine.program.objects.iter().position(|o| o.name == "Keeper").unwrap();
        let routine = |name: &str| {
            engine.program.objects[at]
                .routines
                .iter()
                .position(|r| r.name == name)
                .unwrap()
        };
        let mut session = crate::Session::new(engine.clone());
        let get = |s: &mut crate::Session, name: &str| {
            let inst = s.codeunit_instance(at);
            s.call(at, routine(name), inst, Vec::new())
        };
        assert!(get(&mut session, "GetA").is_ok());
        let b = engine.program.objects[at]
            .globals
            .iter()
            .position(|g| g.name == "B")
            .unwrap();
        session.mark_single_global_unmeasured(at, b, "not measured".into());
        assert!(get(&mut session, "GetA").is_ok());
        let refused = get(&mut session, "GetB").unwrap_err();
        assert!(refused.describe().contains("not measured"), "{}", refused.describe());
    }
}
