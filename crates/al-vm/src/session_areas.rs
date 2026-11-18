//! The session's application areas: the one place AL code reads and writes them (`ApplicationArea()`,
//! `ApplicationArea(Text)`, a page open), and the runner's reset before each test method.
//!
//! Microsoft's Test Runner app empties the areas before every test method (ALTestRunnerResetEnvironment.Codeunit.al:27);
//! a test reads the empty text at its start (corpus permissions/testpermissions/BaTpEnv.al:53) and not the areas an
//! earlier test set (session-system/TestSessionApplicationAreaTestBoundary.al:53-59). Under `--dev-endpoint-ui` the
//! runner models BC's dev-endpoint test runner, which skips that reset; what the areas hold there is not measured
//! (al-runner issue 8333), so a read in a test that has not set them is refused.

use crate::Session;
use al_builtins::describe::Describe;
use al_builtins::native::{Signal, unsupported};
use al_builtins::value::Value;

impl Session {
    /// The runner's reset before a test method: no areas, and none set by the test yet.
    pub(crate) fn reset_application_areas(&mut self) {
        self.application_areas.clear();
        self.application_areas_set_in_test = false;
    }

    /// Refuses a read of the areas by AL code (`ApplicationArea()`, a page whose controls can be read) that is not
    /// measured: under `--dev-endpoint-ui`, outside a test method or before the test method set them. A page open
    /// checks after its handler lookup: a part is covered by its host, and an open that found no handler (Unhandled
    /// UI, an unattended open) shows no control.
    pub(crate) fn check_application_areas_read(&self) -> Result<(), Signal> {
        let measured =
            !self.ui_state.dev_endpoint.on || (self.ui_state.scope.is_some() && self.application_areas_set_in_test);
        if !measured {
            return unsupported(
                "a read of the application areas (ApplicationArea(), a page open) under --dev-endpoint-ui before the test \
                 set them: the dev-endpoint test runner does not empty them before a test, and what they hold there is \
                 not measured (al-runner issue 8333)",
            );
        }
        Ok(())
    }

    /// `ApplicationArea()` reads the session's areas, `ApplicationArea(Text)` sets them
    /// (corpus pages/application-area/TestPageApplicationAreaControlRemoval.al).
    pub(crate) fn application_area(&mut self, set: Option<Value>) -> Result<Value, Signal> {
        match set {
            None => {
                self.check_application_areas_read()?;
                Ok(Value::Text(self.application_areas.clone().into()))
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
                self.application_areas = t.to_string();
                self.application_areas_set_in_test = true;
                Ok(Value::Null)
            }
            Some(other) => Err(Signal::unsupported(format!("ApplicationArea({})", other.describe()))),
        }
    }
}
