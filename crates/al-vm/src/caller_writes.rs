use crate::{Session, Signal};
use al_builtins::value::Instance;
use std::cell::RefCell;
use std::rc::Rc;

impl Session {
    /// A procedure the caller runs on a report variable before its run: the globals it can write win over the saved request page
    /// values (`restore_page_values`). A write that cannot be followed is refused rather than guessed.
    pub(crate) fn note_caller_writes(
        &mut self,
        inst: &Rc<RefCell<Instance>>,
        object: usize,
        routine: usize,
    ) -> Result<(), Signal> {
        let engine = self.engine.clone();
        let o = &engine.program.objects[object];
        if o.report.is_none() || o.routines[routine].is_trigger || self.report.report_trigger.is_some() {
            return Ok(());
        }
        if !self
            .report
            .prepared
            .iter()
            .any(|w| w.upgrade().is_some_and(|i| Rc::ptr_eq(&i, inst)))
        {
            return Ok(());
        }
        let Some(slots) = crate::pages::written_by_routine(&o.routines, object, routine) else {
            return Err(Signal::Unsupported(
                "a report procedure that writes a variable through an interface call, run before a SaveValues request page (not pinned)".into(),
            ));
        };
        match self
            .report
            .caller_writes
            .iter_mut()
            .find(|(w, _)| w.upgrade().is_some_and(|i| Rc::ptr_eq(&i, inst)))
        {
            Some((_, have)) => have.extend(slots),
            None => self.report.caller_writes.push((Rc::downgrade(inst), slots)),
        }
        Ok(())
    }
}
