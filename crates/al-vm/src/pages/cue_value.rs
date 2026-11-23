//! The Value a TestPage reads from a field control in a cuegroup (al-runner issue 8845): an Integer or a Decimal field shows its
//! value as anywhere else, an Option or an Enum field its ordinal, and a field of any other type `0`, by its name and through
//! `TestPage.GetField` (a Text pinned by testpage/ABTests.al:747, a List page's cuegroups with and without ShowAsTree; corpus pull request 1538, testpage/GetFieldByControlId.al:742 for Text, a Text holding a number, Code, a
//! Code holding a number, Integer, Decimal, Boolean, Date, BigInteger, Option, Time, DateTime, Guid, Enum and Duration; :653;
//! assumption `cuegroup-field-value`). In a cuegroup with `CuegroupLayout = Wide` the same for Text (pages/properties/
//! ABLayouts.al:68, :70, also written `wide`), Decimal (pages/controls/AaControls.al:422), and Code and Integer (:742). Any
//! other type refuses there.

use super::*;
use al_builtins::assumptions::{Assumption, assume};

impl Session {
    /// The Value control `idx` shows when it sits inside a cuegroup and its value `v` reads otherwise than the control's own
    /// text; `None` when it reads as anywhere else (not in a cuegroup, or an Integer or a Decimal).
    pub(super) fn cue_value_shown(&self, st: &St, idx: usize, v: &Value) -> Result<Option<Value>, Signal> {
        if !self.in_cue_group(st, idx) {
            return Ok(None);
        }
        let wide = self.in_wide_cue_group(st, idx);
        let measured = match v {
            Value::Integer(_) | Value::Decimal(_) => return Ok(None),
            Value::Text(_) | Value::Code(_) => true,
            Value::Boolean(_)
            | Value::Date(..)
            | Value::BigInteger(_)
            | Value::Option(..)
            | Value::Time(_)
            | Value::DateTime(_)
            | Value::Guid(_)
            | Value::Enum(..)
            | Value::Duration(_) => !wide,
            _ => false,
        };
        if !measured {
            return unsupported(format!(
                "Value of a field control in a cuegroup{} holding {} (not measured)",
                if wide { " with CuegroupLayout = Wide" } else { "" },
                v.describe()
            ));
        }
        // The Text is pinned (testpage/ABTests.al:747 in two cuegroups of a List page, ABLayouts.al:68 in a wide one); the rest
        // comes from the open probe.
        if !matches!(v, Value::Text(_)) {
            assume(Assumption::CuegroupFieldValue);
        }
        Ok(Some(Value::Text(
            match v {
                Value::Option(ordinal, _) | Value::Enum(_, ordinal) => ordinal.to_string(),
                _ => "0".to_string(),
            }
            .into(),
        )))
    }
}
