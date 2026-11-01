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
/// parameters a bound handler would leave and the answer as the warning names it; `None` for a shape not measured.
///
/// Measured through the dev endpoint (TestRunnerHub) on a BC 28.5 container (corpus branch al-runtime-rs-staging-y53,
/// handlers/lt-ui-handler-modes/UiHandlerModes.al:132-137, DevEndpointTestRunner_MeasuredValues): `Confirm` answers
/// its default, No when it is omitted; `StrMenu` answers its default, 1 when omitted; `Message` returns. The arguments
/// already hold the omitted defaults (`native.rs` `confirm`, `str_menu`).
fn answer(kind: UiKind, args: &[Value]) -> Option<(Vec<Value>, String)> {
    match (kind, args) {
        (UiKind::Confirm, [_, Value::Boolean(b)]) => Some((args.to_vec(), b.to_string())),
        // Only a default that names an option was measured (2, and 1 when omitted); 0 or past the last option was not.
        (UiKind::StrMenu, [Value::Text(options), Value::Integer(n), _])
            if *n >= 1 && (*n as usize) <= options.to_string().split(',').count() =>
        {
            Some((args.to_vec(), n.to_string()))
        }
        (UiKind::Message, [_]) => Some((args.to_vec(), "no answer".into())),
        _ => None,
    }
}

impl Session {
    /// `Hyperlink(url)` with no `[HyperlinkHandler]` bound: true when `--dev-endpoint-ui` lets the call return, as it does
    /// through the dev endpoint (UiHandlerModes.al:129-144, BC 28.5).
    pub(crate) fn dev_endpoint_hyperlink(&mut self, url: &str) -> bool {
        if !self.dev_endpoint_unhandled(UiKind::Hyperlink) {
            return false;
        }
        self.note_dev_endpoint_answer(UiKind::Hyperlink, url, "no answer");
        true
    }

    /// Whether `--dev-endpoint-ui` may answer a UI call of `kind` here: the flag is on, a test method runs, no handler
    /// function runs, and the test bound no handler of `kind` (for any page).
    pub(crate) fn dev_endpoint_unhandled(&self, kind: UiKind) -> bool {
        self.ui_state.dev_endpoint.on
            && self.ui_state.scope.as_ref().is_some_and(|s| !s.in_handler)
            && !self.has_ui_handler(kind)
    }

    /// Marks the assumption and keeps the answer for the runner's warning.
    pub(crate) fn note_dev_endpoint_answer(&mut self, kind: UiKind, shown: &str, said: &str) {
        assumptions::assume(Assumption::DevEndpointUi);
        let call = format!("{} {shown}", kind.label());
        self.ui_state
            .dev_endpoint
            .answered
            .push(format!("{}: {said}", call.trim_end()));
    }

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
        self.note_dev_endpoint_answer(kind, shown, &said);
        Some(finals)
    }
}
