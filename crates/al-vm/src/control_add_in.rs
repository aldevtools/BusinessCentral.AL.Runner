//! Control add-ins (ADR-027 section 3 row 18). A test session has no client, so BC never creates the add-in of a
//! `usercontrol`: every call of one of its procedures raises an error, and its event triggers never run (the runtime has no
//! path that raises them). The page itself works as usual.

use crate::Session;
use al_builtins::native::{AlError, Signal};

/// The error code of a call of a control add-in procedure.
pub(crate) const NOT_CREATED: &str = "DotNetControlAddInNotCreated";

impl Session {
    /// A call of a procedure of the add-in of usercontrol `control` on page `page` (`ir::Expr::ControlAddInCall`).
    ///
    /// From BC 28 it raises 'The control add-in on control <control> on page <page> has not been instantiated.' with code
    /// DotNetControlAddInNotCreated, wherever the call is made: in OnOpenPage, OnAfterGetRecord, OnClosePage, an action and a
    /// TryFunction (corpus lt-ac/ACControlAddIn.al TZ_* tests, corpus master 01d7177f, BC28PLUS), on a usercontrol with
    /// Visible = false and on one a page extension adds (same file), from a field's OnValidate
    /// (lt-ac/ACControlAddIn.al:99 AC_ControlAddIn_ProcedureCallFromFieldTrigger), and on a page variable never opened
    /// (pages/lt-f3-pages/F3ValidationAddinFilter.al:449 Addin_PageVariableNeverOpened_Calls). How each caller wraps the code
    /// is the caller's (actions, validation, closing the page).
    ///
    /// BC 27 is not pinned: Windows BC 27.5 gave a client callback error through the test runner hub
    /// (pages/lt-ao-testpage/AoTests.al:795), so the call refuses there.
    pub(crate) fn control_add_in_call(&mut self, control: &str, page: &str) -> Signal {
        if !self.has(al_version::Gate::ControlAddInNotInstantiatedErrorFrom28) {
            return Signal::Unsupported(format!(
                "a call of a procedure of control add-in `{control}` on page `{page}`: what BC does with it before the major the gate names is not pinned"
            ));
        }
        Signal::Error(Box::new(AlError {
            message: format!("The control add-in on control {control} on page {page} has not been instantiated."),
            code: NOT_CREATED.into(),
            ..Default::default()
        }))
    }
}
