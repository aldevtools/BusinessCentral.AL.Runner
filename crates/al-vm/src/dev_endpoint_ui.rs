//! `al-runner test --dev-endpoint-ui` (al-runner issue 3697, opt-in): a UI call with no handler bound gets the answer
//! Business Central's dev-endpoint test runner gives it, instead of the "Unhandled UI" error of the `TestRunner`
//! codeunit mode that every corpus run uses (handlers/TestUnhandledUiAndDialogDefaults.al:68-107).
//!
//! Only the shapes measured on real BC in the dev-endpoint mode are answered (`answer`); every other shape stays
//! refused under the flag too. Each answer is kept for the runner, which prints it as a warning naming the test, and
//! marks the assumption `dev-endpoint-ui`, so a test that passes through one is a flagged pass, never a clean one.

use crate::Session;
use crate::ui::UiKind;
use al_builtins::assumptions::{self, Assumption};
use al_builtins::value::Value;

/// What the dev-endpoint mode needs in the session.
#[derive(Debug, Default)]
pub(crate) struct DevEndpointUi {
    /// The flag is on.
    pub(crate) on: bool,
    /// The calls answered since the last `take_dev_endpoint_answers`, as `<call> <shown>: <answer>`.
    pub(crate) answered: Vec<String>,
}

/// The dev-endpoint answer to a UI call of `kind` with handler arguments `args`, as the final values of the handler
/// parameters a bound handler would leave; `None` for a shape not measured.
fn answer(kind: UiKind, args: &[Value]) -> Option<(Vec<Value>, String)> {
    let _ = (kind, args);
    None
}

impl Session {
    /// `--dev-endpoint-ui`: answer the UI calls no handler takes as the dev-endpoint test runner does.
    pub fn set_dev_endpoint_ui(&mut self, on: bool) {
        self.ui_state.dev_endpoint.on = on;
    }

    /// The UI calls answered under `--dev-endpoint-ui` since the last call, in order.
    pub fn take_dev_endpoint_answers(&mut self) -> Vec<String> {
        std::mem::take(&mut self.ui_state.dev_endpoint.answered)
    }

    /// A UI call of `kind` that no bound handler takes: the dev-endpoint answer when the flag is on and the shape is
    /// measured, as the final handler parameter values; `None` to raise "Unhandled UI".
    pub(crate) fn dev_endpoint_answer(&mut self, kind: UiKind, shown: &str, args: &[Value]) -> Option<Vec<Value>> {
        if !self.ui_state.dev_endpoint.on {
            return None;
        }
        let (finals, said) = answer(kind, args)?;
        assumptions::assume(Assumption::DevEndpointUi);
        let call = format!("{} {shown}", kind.label());
        self.ui_state.dev_endpoint.answered.push(format!("{}: {said}", call.trim_end()));
        Some(finals)
    }
}
