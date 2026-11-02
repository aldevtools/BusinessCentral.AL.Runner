//! TestPage SetValue on a control over a TABLE field that declares ValuesAllowed (al-runner issue 6672).
//!
//! Measured by corpus writer Y65 on BC 29 (pages/lt-y65-values-allowed-table-field/Y65ValuesAllowedTableField.al:236-574 on corpus
//! branch al-runtime-rs-staging-y65, not pinned yet; assumption `table-field-values-allowed-setvalue`), each case on a freshly
//! opened NavigatePage over a temporary table, Card or List with a row:
//! - an Enum or Option field: an Enum value or an Integer in the list is taken; another (also an Integer that is no value) is a
//!   TestValidation error `Your entry of '<n>' ...` where `<n>` is the ordinal the control HOLDS, and Value then shows that
//!   ordinal (:236-306); a member name as a Text in a Variant (`'None'`, `'published only'`, `'D'`) is `The value "<t>" can't be
//!   evaluated into type Option.` (Evaluate:Format) and the control keeps its value (:251-286);
//! - an Integer field: `Value '3' is not a valid value. The following values can be used: 1;5.` (:280-286);
//! - a Decimal field: `Value '2.00' ...: 1.5;2.` although 2 is listed, and `Value '1.70' ...` (:292-306);
//! - an Enum field whose control has a ValuesAllowed of its own: the control's list replaces the field's (:292-306).
//!
//! A NavigatePage adds no Refresh hint, a Card does, a List does not (`validation_suffix`). What the file does not pin stays
//! refused.

use super::*;
use al_builtins::assumptions::{Assumption, assume};

impl Session {
    /// The hint a TestValidation error of control `idx` ends with: a table-field control with a row on a page that is not a
    /// list adds ` (Select Refresh to discard errors)` (handlers/TestPageErrTeardown_Tests.al, TestPageBooleanSpelling_Tests.al:210;
    /// a list adds none, misc/lt-probes-needed-3/PnC07TestPage.al). A NavigatePage adds none (Y65ValuesAllowedTableField.al:243,
    /// :262, :282, :296) for a control over a table field with ValuesAllowed.
    pub(super) fn validation_suffix(&self, st: &St, idx: usize, rec_bound: bool) -> &'static str {
        let object = st.borrow().object as usize;
        // Only measured for a control over a table field with ValuesAllowed: other NavigatePage controls keep the hint.
        let navigate = self.engine.object_facts[object].page_type == Some(ir::PageType::NavigatePage)
            && self
                .control_field(st, idx)
                .is_some_and(|f| prop_of(&f, "ValuesAllowed").is_some());
        if navigate && rec_bound {
            assume(Assumption::TableFieldValuesAllowedSetValue);
            return "";
        }
        if rec_bound && !self.shows_no_row(st, idx) && !self.is_list(st) {
            " (Select Refresh to discard errors)"
        } else {
            ""
        }
    }

    /// The field's ValuesAllowed of an Enum or Option control `idx` that states none of its own, when the page is one Y65
    /// measured (with a row). `Ok(None)` when it does not apply.
    fn table_list(&self, st: &St, idx: usize, ty: &RtType) -> Result<Option<String>, Signal> {
        if !matches!(ty, RtType::Enum(_) | RtType::Option(_)) {
            return Ok(None);
        }
        let engine = self.engine();
        let ctl = &page_of(&engine, st).controls[idx];
        let Some(list) = self.control_field(st, idx).and_then(|f| prop_of(&f, "ValuesAllowed")) else {
            return Ok(None);
        };
        if ctl.prop("valuesallowed").is_some() {
            return Ok(None);
        }
        self.measured_page(st, idx, matches!(ctl.binding, ir::Binding::Field(_)))?;
        Ok(Some(list))
    }

    /// Refuses a page Y65 did not measure: a table-field control with a row on a Card, NavigatePage or list.
    fn measured_page(&self, st: &St, idx: usize, rec_bound: bool) -> Result<(), Signal> {
        let object = st.borrow().object as usize;
        let page_type = &self.engine.object_facts[object].page_type;
        let measured_type =
            self.is_list(st) || matches!(page_type, Some(ir::PageType::Card | ir::PageType::NavigatePage));
        if !rec_bound || !self.has_source(st) || self.shows_no_row(st, idx) || !measured_type {
            return unsupported(
                "SetValue on a control over a table field with ValuesAllowed, not a table field of a Card, NavigatePage or list with a row (not pinned)",
            );
        }
        assume(Assumption::TableFieldValuesAllowedSetValue);
        Ok(())
    }

    /// SetValue of `arg` on an Enum or Option control over a table field with ValuesAllowed and none of its own (see the module).
    /// `Ok(())` lets the write go on: a value of the list.
    pub(super) fn table_values_allowed_entry(
        &mut self,
        st: &St,
        idx: usize,
        ty: &RtType,
        arg: &mut Value,
        current: &Value,
        from_variant: bool,
    ) -> Result<(), Signal> {
        // A one-character literal in a Variant is a Char here; on an Integer or Decimal field it is the text typed (`'1'` is taken
        // as 1, Y65ValuesAllowedTableField.al:276, :385, :531 `int text 1`).
        if let (Value::Char(c), RtType::Integer | RtType::Decimal) = (&*arg, ty) {
            if from_variant
                && self
                    .control_field(st, idx)
                    .is_some_and(|f| prop_of(&f, "ValuesAllowed").is_some())
            {
                let Some(t) = char::from_u32(*c as u32) else {
                    return unsupported("SetValue of a Char that is no character (not pinned)");
                };
                *arg = Value::Text(t.to_string().into());
            }
        }
        let Some(list) = self.table_list(st, idx, ty)? else {
            return Ok(());
        };
        let arg = &*arg;
        let engine = self.engine();
        let ordinal = match (arg, ty) {
            (Value::Integer(n), _) => *n,
            (Value::Enum(a, o), RtType::Enum(id)) if a == id => *o,
            (Value::Option(o, _), RtType::Option(_)) => *o,
            // A member name in a Variant (Y65ValuesAllowedTableField.al:251-262 `'None'`, `'published only'`, :271 `'D'`, `'A'`).
            (Value::Text(_) | Value::Code(_) | Value::Char(_), _) if from_variant && member_name(arg).is_some() => {
                let t = member_name(arg).unwrap_or_default();
                return Err(if self.has(al_version::Gate::ConversionErrorTextsFrom28) {
                    error_code(
                        format!("The value \"{t}\" can't be evaluated into type Option."),
                        "Evaluate:Format",
                    )
                } else {
                    error_code("The type NavOption is unknown.".to_string(), "NotSupportedType")
                });
            }
            _ => {
                return unsupported(format!(
                    "SetValue of {} on a control over a table field with ValuesAllowed (not pinned)",
                    arg.describe()
                ));
            }
        };
        let values: Vec<(i32, String)> = match ty {
            RtType::Enum(id) if self.enum_option_list(st, idx, *id)?.is_none() => {
                engine.program.objects[*id as usize].enum_values.clone()
            }
            RtType::Option(members)
                if self
                    .control_field(st, idx)
                    .is_some_and(|f| prop_of(&f, "OptionCaption").is_none()) =>
            {
                members.iter().enumerate().map(|(i, m)| (i as i32, m.clone())).collect()
            }
            _ => {
                return unsupported(
                    "SetValue on an Enum or Option control with captions over a table field with ValuesAllowed (not pinned)",
                );
            }
        };
        let Some(allowed) = super::controls::allowed_ordinals(&list, &values) else {
            return unsupported(
                "SetValue on a control over a table field whose ValuesAllowed names no value of its type (not pinned)",
            );
        };
        if allowed.contains(&ordinal) {
            return Ok(());
        }
        let held = match current {
            Value::Enum(_, h) | Value::Option(h, _) => *h,
            _ => {
                return unsupported(
                    "SetValue on a control over a table field with ValuesAllowed that holds no Enum or Option",
                );
            }
        };
        let caption = self.control_caption(st, idx)?;
        let name = page_of(&engine, st).controls[idx].name.clone();
        let stored = format!(
            "Your entry of '{held}' is not an acceptable value for '{caption}'.{}",
            self.validation_suffix(st, idx, true)
        );
        super::controls::note_refusal(st, idx, stored.clone())?;
        if self.is_list(st) {
            let key = self.row_key(&self.page_rec(st))?;
            st.borrow_mut().values_allowed_refused_row = Some((idx, key));
        }
        Err(error_code(
            format!("Validation error for Field: {name},  Message = '{stored}'"),
            "TestValidation",
        ))
    }

    /// The check of a typed value against the table field's ValuesAllowed after the entry was read (`parsed` from `typed`), for
    /// the field types other than Enum and Option. `refuse_bound` makes the TestValidation error.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn table_values_allowed_value(
        &self,
        st: &St,
        idx: usize,
        ty: &RtType,
        typed: &str,
        parsed: &Value,
        rec_bound: bool,
        refuse_bound: &dyn Fn(String) -> Signal,
    ) -> Result<(), Signal> {
        let Some(raw) = self.control_field(st, idx).and_then(|f| prop_of(&f, "ValuesAllowed")) else {
            return Ok(());
        };
        let no_row = self.shows_no_row(st, idx);
        match ty {
            // Taken above (`table_values_allowed_entry`), or by the control's own list.
            RtType::Enum(_) | RtType::Option(_) => Ok(()),
            RtType::Integer | RtType::Decimal => {
                self.measured_page(st, idx, rec_bound)?;
                let entries: Vec<&str> = raw.split(',').map(str::trim).collect();
                let listed = entries.join(";");
                let invalid = |shown: String| {
                    refuse_bound(format!(
                        "Value '{shown}' is not a valid value. The following values can be used: {listed}."
                    ))
                };
                match parsed {
                    Value::Integer(n) => {
                        let Some(values) = entries
                            .iter()
                            .map(|e| e.parse::<i32>().ok())
                            .collect::<Option<Vec<_>>>()
                        else {
                            return unsupported(
                                "SetValue on an Integer field whose ValuesAllowed is not a list of numbers (not pinned)",
                            );
                        };
                        if values.contains(n) {
                            Ok(())
                        } else {
                            Err(invalid(n.to_string()))
                        }
                    }
                    Value::Decimal(d) if d.normalize().scale() <= 2 => {
                        let Some(values) = entries
                            .iter()
                            .map(|e| e.parse::<rust_decimal::Decimal>().ok().map(|v| (v, e.contains('.'))))
                            .collect::<Option<Vec<_>>>()
                        else {
                            return unsupported(
                                "SetValue on a Decimal field whose ValuesAllowed is not a list of numbers (not pinned)",
                            );
                        };
                        // A listed whole number is refused too (`2` of `1.5, 2`); a listed value with decimals is not measured.
                        if values.iter().any(|(v, decimals)| v == d && *decimals) {
                            return unsupported(
                                "SetValue of a Decimal that a ValuesAllowed with decimals lists (whether it is taken is not pinned)",
                            );
                        }
                        Err(invalid(format!("{d:.2}")))
                    }
                    _ => unsupported("SetValue on a number field with ValuesAllowed of another value (not pinned)"),
                }
            }
            // A Text or Code field of quoted values on a card with a row: an entry outside the list, and also the empty text
            // although '' is listed, is a TestValidation error that lists the values with `;`; `A` is taken
            // (pages/lt-cd-testpage-leads/CdTestPageLeads.al:396-423, Item56_ValuesAllowedEmpty_SetValueEmptyAfterA,
            // Item56_ValuesAllowed_SetValueNotAllowed). Validate in code does not check it (:426-444). An entry that matches only
            // when the case is ignored, or that a Code field upper-cases, and the empty text on a list without '' are not pinned.
            _ => {
                let card_row = rec_bound && !no_row && !self.is_list(st);
                let Some(values) = super::controls::quoted_list(&raw)
                    .filter(|_| card_row && matches!(ty, RtType::Text(_) | RtType::Code(_)))
                else {
                    return unsupported(
                        "SetValue on a field with ValuesAllowed that is not a Text or Code field of quoted values on a card with a row (not pinned)",
                    );
                };
                if matches!(ty, RtType::Code(_)) && typed != typed.to_uppercase() {
                    return unsupported(
                        "SetValue of a lower-case entry on a Code field with ValuesAllowed (not pinned)",
                    );
                }
                let exact = values.iter().any(|v| v == typed);
                if exact && !typed.is_empty() {
                    return Ok(());
                }
                if !exact && values.iter().any(|v| v.eq_ignore_ascii_case(typed)) {
                    return unsupported(
                        "SetValue on a field with ValuesAllowed of an entry that differs in case only (not pinned)",
                    );
                }
                if typed.is_empty() && !exact {
                    return unsupported(
                        "SetValue of the empty text on a field whose ValuesAllowed does not list '' (not pinned)",
                    );
                }
                Err(refuse_bound(format!(
                    "Value '{typed}' is not a valid value. The following values can be used: {}.",
                    values.join(";")
                )))
            }
        }
    }

    /// What Value shows for an Enum or Option control `idx` whose value `v` is outside its ValuesAllowed: the ordinal. Pinned for
    /// the control's own list on a list (pages/lt-y37-values-allowed/Y37ValuesAllowedSetValue.al:302, :322 `4` for Finished) and
    /// for the table field's list on a NavigatePage (Y65ValuesAllowedTableField.al:271-276 `now 0` for Opt A of `B, D`); elsewhere
    /// the table field's list is not measured and the caption shows as before. `Ok(None)` when the caption shows.
    pub(super) fn outside_values_allowed_shown(
        &mut self,
        st: &St,
        idx: usize,
        v: &Value,
    ) -> Result<Option<Value>, Signal> {
        let (Value::Enum(_, ordinal) | Value::Option(ordinal, _)) = v else {
            return Ok(None);
        };
        let Some(ty) = self
            .control_type(st, idx, v)
            .filter(|t| matches!(t, RtType::Enum(_) | RtType::Option(_)))
        else {
            return Ok(None);
        };
        let own = page_of(&self.engine(), st).controls[idx]
            .prop("valuesallowed")
            .is_some();
        let allowed = if own {
            self.values_allowed(st, idx, &ty)?.unwrap_or_default()
        } else {
            let Some(list) = self.control_field(st, idx).and_then(|f| prop_of(&f, "ValuesAllowed")) else {
                return Ok(None);
            };
            let object = st.borrow().object as usize;
            let navigate = self.engine.object_facts[object].page_type == Some(ir::PageType::NavigatePage);
            if !navigate {
                return Ok(None);
            }
            let values: Vec<(i32, String)> = match &ty {
                RtType::Enum(id) => self.engine().program.objects[*id as usize].enum_values.clone(),
                RtType::Option(members) => members.iter().enumerate().map(|(i, m)| (i as i32, m.clone())).collect(),
                _ => return Ok(None),
            };
            let Some(allowed) = super::controls::allowed_ordinals(&list, &values) else {
                return Ok(None);
            };
            if !allowed.contains(ordinal) {
                assume(Assumption::TableFieldValuesAllowedSetValue);
            }
            allowed
        };
        if allowed.contains(ordinal) {
            return Ok(None);
        }
        if own && !self.is_list(st) {
            return unsupported(
                "Value of a control whose value is outside its ValuesAllowed, not on a list (not pinned)",
            );
        }
        Ok(Some(Value::Text(ordinal.to_string().into())))
    }
}

/// The text of a Text, Code or Char `v` (a one-letter literal in a Variant is a Char) made of letters and spaces only, with a
/// letter.
fn member_name(v: &Value) -> Option<String> {
    let t = match v {
        Value::Text(t) | Value::Code(t) => t.to_string(),
        Value::Char(c) => char::from_u32(*c as u32)?.to_string(),
        _ => return None,
    };
    (t.chars().any(char::is_alphabetic) && t.chars().all(|c| c.is_alphabetic() || c == ' ')).then_some(t)
}

/// The value of property `name` of table field `f`.
fn prop_of(f: &ir::Field, name: &str) -> Option<String> {
    f.properties
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.clone())
}
