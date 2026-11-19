//! The session's application areas: their one owner (`ApplicationArea()`, `ApplicationArea(Text)`, the copy a page
//! takes when it opens, the runner's reset before each test method), and whether their value is known.
//!
//! Microsoft's Test Runner app empties the areas before every test method (ALTestRunnerResetEnvironment.Codeunit.al:27);
//! a test reads the empty text at its start (corpus permissions/testpermissions/BaTpEnv.al:53) and not the areas an
//! earlier test set (session-system/TestSessionApplicationAreaTestBoundary.al:53-59). Under `--dev-endpoint-ui` the
//! runner models BC's dev-endpoint test runner, which skips that reset; what the areas hold there is not measured
//! (al-runner issue 8333), so they are known only after the test method set them. `ApplicationArea()` refuses an
//! unknown value here; a page keeps whether its copy was known, and its readers refuse only where the value decides
//! the answer (`pages/areas.rs`).

use crate::Session;
use al_builtins::describe::Describe;
use al_builtins::native::{Signal, unsupported};
use al_builtins::value::Value;

/// The refusal of a read whose answer depends on areas that are not known.
pub(crate) const AREAS_UNKNOWN: &str = "a read of the application areas under --dev-endpoint-ui before the test set them \
     (ApplicationArea(), or a page control whose area list has no All): the dev-endpoint test runner does not empty them \
     before a test, and what they hold there is not measured (al-runner issue 8333)";

/// The session's areas: `#Basic,#Suite`, or the empty text, which enables every area.
#[derive(Debug, Default)]
pub(crate) struct SessionAreas {
    text: String,
    /// `ApplicationArea(Text)` ran since the runner's reset before the test method.
    set_in_test: bool,
}

impl SessionAreas {
    /// No areas set (the state the install code must leave, `install_left.rs`).
    pub(crate) fn is_empty(&self) -> bool {
        self.text.is_empty()
    }
}

impl Session {
    /// The runner's reset before a test method: no areas, and none set by the test yet.
    pub(crate) fn reset_application_areas(&mut self) {
        self.application_areas = SessionAreas::default();
    }

    /// Whether the areas AL code would read now are measured: always without `--dev-endpoint-ui`; under it, only in a
    /// test method that set them.
    fn application_areas_known(&self) -> bool {
        !self.ui_state.dev_endpoint.on || (self.ui_state.scope.is_some() && self.application_areas.set_in_test)
    }

    /// The areas a page copies when it opens, and whether they are known (see [`Self::application_areas_known`]).
    pub(crate) fn page_application_areas(&self) -> (String, bool) {
        (self.application_areas.text.clone(), self.application_areas_known())
    }

    /// `ApplicationArea()` reads the session's areas, `ApplicationArea(Text)` sets them
    /// (corpus pages/application-area/TestPageApplicationAreaControlRemoval.al).
    pub(crate) fn application_area(&mut self, set: Option<Value>) -> Result<Value, Signal> {
        match set {
            None => {
                if !self.application_areas_known() {
                    return unsupported(AREAS_UNKNOWN);
                }
                Ok(Value::Text(self.application_areas.text.clone().into()))
            }
            Some(Value::Text(t) | Value::Code(t)) => {
                // Tags are `#` and letters or digits, separated by commas (metadata/platform-tables/PlTSession.al:
                // ApplicationArea_SetTwiceAndInvalid); whether a well-formed tag exists is not pinned.
                let well_formed = t.split(',').all(|tag| {
                    let tag = tag.trim();
                    tag.strip_prefix('#')
                        .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_alphanumeric()))
                });
                if !t.is_empty() && !well_formed {
                    return Err(Signal::Error(Box::new(al_builtins::native::AlError {
                        message: format!(
                            "'{t}' is not a valid value for the APPLICATIONAREA function parameter.\\\\    The value must be a text string that contains one or more application area tags.\\    A tag must start with # and can be followed by any combination of letters (Aa-Zz) and numbers (0-9).\\    Use a comma to separate multiple tags. Example: #Basic,#Suite"
                        ),
                        code: "Argument".into(),
                        call_stack: String::new(),
                        ..Default::default()
                    })));
                }
                self.application_areas = SessionAreas {
                    text: t.to_string(),
                    set_in_test: true,
                };
                Ok(Value::Null)
            }
            Some(other) => Err(Signal::unsupported(format!("ApplicationArea({})", other.describe()))),
        }
    }
}
