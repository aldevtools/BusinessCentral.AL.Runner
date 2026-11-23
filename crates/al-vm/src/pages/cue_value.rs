//! The Value a TestPage reads from a field control in a cuegroup (al-runner issue 8845), by its name and through
//! `TestPage.GetField`: an Integer or a Decimal field shows its value as anywhere else, an Option or an Enum field its ordinal,
//! and a field of any other measured type `0`. Corpus pull request 1538, testpage/GetFieldByControlId.al (assumption
//! `cuegroup-field-value`): on a Card :742 (Text, a Text holding a number, Code, a Code holding a number, Integer, Decimal,
//! Boolean, Date, BigInteger, Option, Time, DateTime, Guid, Enum and Duration; in a cuegroup with `CuegroupLayout = Wide`
//! Code and Integer) and :653 (Text), and the same on a CardPart :856, a ListPart :923, a List :990 and a Document :2116, each
//! opened as a TestPage of its own, and on a CardPart read through the Card that shows it as a part :2166; on a Card also a
//! DateFormula, and a Boolean in a wide cuegroup :2216. Pinned without the entry: a Text on a List (testpage/ABTests.al:747, cuegroups with and without
//! ShowAsTree) and a Text in a wide cuegroup on a Card (pages/properties/ABLayouts.al:68, :70). On a RoleCenter BC finds no
//! field there, by its name or its id (:1076): refused for every type. A Decimal, and an Integer outside a wide cuegroup, read
//! as anywhere else on the other page kinds and in a part too (pinned); an Integer in a wide cuegroup only on the measured
//! pages (:742 `WideIntValue`, under the entry), and every other type there refuses, as main refused the wide Integer.
//!
//! The Value a TestPage reads from a field control in a cuegroup, by its name or through GetField
//! (al-runner issue 8845, assumption `cuegroup-field-value`): an Integer or a Decimal its value, an
//! Option or an Enum its ordinal, the other measured types `0`, on a Card, a CardPart, a ListPart,
//! a List or a Document opened on its own, or a CardPart a Card shows (a DateFormula and a wide
//! Boolean on a Card only); a RoleCenter refuses. Entry points: `cue_value_shown` (by
//! `controls.rs`).

use super::*;
use al_builtins::assumptions::{Assumption, assume};

/// The page kinds the cue cells open, as `object_facts` names them.
fn measured_kind(kind: Option<&al_ir::PageType>) -> Option<&'static str> {
    match kind {
        Some(al_ir::PageType::Card) => Some("card"),
        Some(al_ir::PageType::List) => Some("list"),
        Some(al_ir::PageType::Document) => Some("document"),
        Some(al_ir::PageType::Other(o)) if o == "cardpart" => Some("cardpart"),
        Some(al_ir::PageType::Other(o)) if o == "listpart" => Some("listpart"),
        _ => None,
    }
}

impl Session {
    /// The Value control `idx` shows when it sits inside a cuegroup and its value `v` reads otherwise than the control's own
    /// text; `None` when it reads as anywhere else (not in a cuegroup, a Decimal or an Integer outside a RoleCenter, the wide
    /// Integer on a measured page only).
    pub(super) fn cue_value_shown(&self, st: &St, idx: usize, v: &Value) -> Result<Option<Value>, Signal> {
        if !self.in_cue_group(st, idx) {
            return Ok(None);
        }
        let wide = self.in_wide_cue_group(st, idx);
        let host = st.borrow().host.clone();
        let (kind, role_center, host_card) = {
            let engine = self.engine();
            let page_type = engine.object_facts[st.borrow().object as usize].page_type.clone();
            let role_center = matches!(&page_type, Some(al_ir::PageType::Other(o)) if o == "rolecenter");
            let host_card = host.as_ref().and_then(|(h, _)| h.upgrade()).is_some_and(|h| {
                h.borrow().host.is_none()
                    && matches!(
                        engine.object_facts[h.borrow().object as usize].page_type,
                        Some(al_ir::PageType::Card)
                    )
            });
            (measured_kind(page_type.as_ref()), role_center, host_card)
        };
        if role_center {
            return unsupported(
                "Value of a field control in a cuegroup on a RoleCenter page (BC finds no such field, GetFieldByControlId.al:1076)",
            );
        }
        // A Decimal, and an Integer outside a wide cuegroup, read as anywhere else, also in a part (pages/properties/
        // P119Cues.al:68 `level-value` `9`, testpage/TestPageCueGroupFixedGridContracts.al:355 `11`; a wide Decimal
        // pages/controls/AaControls.al:422).
        if matches!(v, Value::Decimal(_)) || (!wide && matches!(v, Value::Integer(_))) {
            return Ok(None);
        }
        // A page of its own, or a CardPart a Card shows as a part (:2166).
        let Some(kind) = kind.filter(|k| host.is_none() || *k == "cardpart" && host_card) else {
            return unsupported(
                "Value of a field control in a cuegroup outside a Card, a CardPart, a ListPart, a List or a Document opened as a \
                 TestPage of its own, or a CardPart a Card shows (not measured)",
            );
        };
        let measured = match v {
            // A Text in a wide cuegroup is pinned on a Card only (ABLayouts.al:68).
            Value::Text(_) => !wide || kind == "card",
            Value::Code(_) | Value::Integer(_) => true,
            // A wide Boolean and a DateFormula are measured on a Card only (:2216).
            Value::Boolean(_) => !wide || kind == "card",
            Value::DateFormula(_) => !wide && kind == "card",
            Value::Date(..)
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
        let pinned = matches!(v, Value::Text(_)) && (kind == "list" && !wide || kind == "card" && wide);
        if !pinned {
            assume(Assumption::CuegroupFieldValue);
        }
        if matches!(v, Value::Integer(_)) {
            return Ok(None);
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
