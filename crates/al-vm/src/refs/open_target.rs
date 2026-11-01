//! The platform tables `RecordRef.Open` refuses because of the target of the app that opens them.

use super::{al_error, unsupported};
use crate::Session;
use al_builtins::native::Signal;

impl Session {
    /// Refuses `RecordRef.Open(no)` when the opening app's target may not open table `no`.
    pub(super) fn recordref_open_target_check(&self, no: i64) -> Result<(), Signal> {
        // corpus recordref/TestRecordRefOpenCompilationTarget.al:27-35: an internal system table is
        // refused for a Cloud-target app (this app is Cloud), whether or not a dependency app
        // declares an object with that number (the platform's `System.app` does, as
        // `Object Metadata`; a complete compile of the dependencies must not change the answer).
        // Agent Creation Control (2000000297) is refused the same way from BC 28 on, where the System app declares it
        // (platform/lt-aw-fresh-tenant/AwTests.al:590-616, BC28PLUS arm :612); on BC 27 no table has the number and
        // the open fails as for a missing object (:614).
        if no == 2_000_000_297 && self.table_object(no).is_some() {
            match self.caller_target() {
                Some(t) if t.eq_ignore_ascii_case("Cloud") => {
                    return Err(al_error(format!(
                        "You cannot open record {no} from a RecordRef data type when you are using target Cloud."
                    )));
                }
                // An OnPrem app opens it and reads its one row (onprem platform/lt-aw-fresh-tenant/AwOnPremTests.al:234-253).
                Some(t) if t.eq_ignore_ascii_case("OnPrem") => {}
                other => {
                    return unsupported(format!(
                        "RecordRef.Open(2000000297) from an app whose target is {} (only Cloud and OnPrem are pinned)",
                        other.unwrap_or("not known")
                    ));
                }
            }
        }
        // Printer is refused to a Cloud-target app the same way (platform/lt-y21-env-columns/Y21EnvColumns.al:43-50
        // Y21_Printer_RecordRefRefused, which pins the text up to `target`; the rest is the template above). Another target is
        // not pinned.
        if no == 2_000_000_039 && self.table_object(no).is_some() {
            match self.caller_target() {
                Some(t) if t.eq_ignore_ascii_case("Cloud") => {
                    return Err(al_error(format!(
                        "You cannot open record {no} from a RecordRef data type when you are using target Cloud."
                    )));
                }
                other => {
                    return unsupported(format!(
                        "RecordRef.Open(2000000039) from an app whose target is {} (only Cloud is pinned)",
                        other.unwrap_or("not known")
                    ));
                }
            }
        }
        // The OnPrem tables the runtime serves (al_builtins::platform REGISTRY, PtOnPremTables.al:57-213) opened from a
        // Cloud-target app: Document Service Scenario fails from BC 28 on and opens on BC 27 (onprem platform/PtOnPremTables.al:
        // 290-312, CloudRunner_RefusesOnPremTables). The other four, and a caller whose target is not known, are not pinned.
        if matches!(
            no,
            2_000_000_115 | 2_000_000_130 | 2_000_000_170 | 2_000_000_177 | 2_000_000_228
        ) && self.table_object(no).is_some()
        {
            match self.caller_target() {
                Some(t) if t.eq_ignore_ascii_case("OnPrem") => {}
                Some(t) if t.eq_ignore_ascii_case("Cloud") && no == 2_000_000_115 => {
                    if self.engine.target.as_ref().map_or(29, |t| t.bc.major) >= 28 {
                        return Err(Signal::Error(Box::new(al_builtins::native::AlError {
                            message: format!(
                                "You cannot open record {no} from a RecordRef data type when you are using target Cloud."
                            ),
                            code: "NotAllowedForCompilationTarget".into(),
                            call_stack: String::new(),
                            ..Default::default()
                        })));
                    }
                }
                other => {
                    return unsupported(format!(
                        "RecordRef.Open({no}), an OnPrem platform table, from an app whose target is {} (only OnPrem, and \
                         Cloud for 2000000115, are pinned)",
                        other.unwrap_or("not known")
                    ));
                }
            }
        }
        if no == 2_000_000_071 {
            return Err(al_error(format!(
                "You cannot open record {no} from a RecordRef data type when you are using target Cloud."
            )));
        }
        Ok(())
    }
}
