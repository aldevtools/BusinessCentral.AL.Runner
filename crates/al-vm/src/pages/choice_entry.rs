//! What a TestPage SetValue makes of the entry on an Enum or Option control, before the generic conversion: a Text handed in a
//! Variant, and the entries a request page control bound to an Option or Enum field of a global record takes (moved out of
//! controls.rs; al-runner issue 7376).

use super::*;

impl Session {
    /// The Option or Enum field a request page control is bound to, when its source is a field of a global record of the report
    /// (`MySetup."Logo Position"`) or of an element of a global record array (`SetupArr[2]."Logo Position"`): such a control takes
    /// the field's type (reports/request-page/RequestPageOptionFieldOfGlobalRecord.al:186-499, BC 27.5, 28.5 and 29.0). Other field
    /// types keep the type of the value the control shows (`control_type`).
    pub(super) fn request_record_choice_type(&self, st: &St, idx: usize) -> Option<RtType> {
        if !self.is_request_page(st) {
            return None;
        }
        let engine = self.engine();
        let ctl = &page_of(&engine, st).controls[idx];
        if !matches!(ctl.binding, ir::Binding::Expr(_)) || ctl.trigger("__sourceset").is_none() {
            return None;
        }
        let (t, p) = ctl.var_field.or(ctl.array_element_field)?;
        let ty = engine.program.objects[t].table.as_ref()?.fields[p].ty.clone();
        matches!(ty, RtType::Option(_) | RtType::Enum(_)).then_some(ty)
    }

    /// The entries such a request page control takes, as RequestPageOptionFieldOfGlobalRecord.al pins them: an Integer (in a
    /// Variant or not, :186-198, :262-322, :388-440), `true` in a Variant on an Option field (:288-291 `boolean value Left`), the Enum
    /// value itself in a Variant on an Enum field (:347-354), and a Text in a Variant (`variant_choice_text`). Anything else is
    /// refused. `true` becomes the ordinal 1.
    pub(super) fn request_record_choice_entry(
        &self,
        ty: &RtType,
        arg: &mut Value,
        from_variant: bool,
    ) -> Result<(), Signal> {
        if matches!((&*arg, ty), (Value::Boolean(true), RtType::Option(_))) && from_variant {
            *arg = Value::Integer(1);
            return Ok(());
        }
        let pinned = match (&*arg, ty) {
            (Value::Integer(_), _) => true,
            (Value::Enum(id, _), RtType::Enum(t)) => from_variant && id == t,
            (Value::Text(_) | Value::Code(_), _) => from_variant,
            _ => false,
        };
        if pinned {
            return Ok(());
        }
        unsupported(format!(
            "SetValue of {} on a request page control bound to an Option or Enum field of a global record (not pinned)",
            arg.describe()
        ))
    }

    /// A Text inside a Variant on an Enum or Option control goes by ordinal digits: '1' selects the value of ordinal 1, as an
    /// Integer does; ' ', '' and 'Alpha' fail to evaluate, the control keeps its value and holds no validation error. BC 27.5
    /// names NavOption instead (pages/enum-setvalue-blank/BeEnumSetValueBlank.al:532-597 table field, Enum with a blank
    /// caption; the FStrict, FPlain, FOpt and V* twins). Pinned on a card with a row for digits of a value and for empty,
    /// blank or all-letter texts. On a request page control bound to a field of a global record (`request_record_choice_type`)
    /// the digits of an Option value select it, and every text without a digit fails to evaluate the same way: member names in
    /// any case, a prefix, a name with a space, a caption, the empty text (RequestPageOptionFieldOfGlobalRecord.al:205-259,
    /// :280-287, :339-346, :406-413, :465-468). Other texts and other shapes are refused. Turns digits into the Integer.
    pub(super) fn variant_choice_text(&self, st: &St, idx: usize, ty: &RtType, arg: &mut Value) -> Result<(), Signal> {
        let t: String = match &*arg {
            Value::Text(t) | Value::Code(t) => t.to_string(),
            Value::Char(c) => match char::from_u32(*c as u32) {
                Some(c) => String::from(c),
                None => return Ok(()),
            },
            _ => return Ok(()),
        };
        let request = self.request_record_choice_type(st, idx).is_some();
        if !request && !(self.has_source(st) && !self.is_list(st) && !self.shows_no_row(st, idx)) {
            return unsupported(
                "SetValue of a Text in a Variant on an Enum or Option control of a list or of a page without a row (not pinned)",
            );
        }
        if !t.is_empty() && t.chars().all(|c| c.is_ascii_digit()) {
            if request && !matches!(ty, RtType::Option(_)) {
                return unsupported(
                    "SetValue of a Variant holding digits on a request page Enum control bound to a field of a global record (not pinned)",
                );
            }
            let n: Option<i32> = t.parse().ok();
            let engine = self.engine();
            let is_value = match (ty, n) {
                (RtType::Enum(id), Some(n)) => engine.program.objects[*id as usize]
                    .enum_values
                    .iter()
                    .any(|(v, _)| i64::from(*v) == i64::from(n)),
                (RtType::Option(members), Some(n)) => usize::try_from(n).is_ok_and(|n| n < members.len()),
                _ => false,
            };
            return match n {
                Some(n) if is_value => {
                    *arg = Value::Integer(n);
                    Ok(())
                }
                _ => unsupported(
                    "SetValue of a Variant holding the digits of no value on an Enum or Option control (not pinned)",
                ),
            };
        }
        let fails = if request {
            !t.chars().any(|c| c.is_ascii_digit())
        } else {
            t.chars().all(|c| c == ' ') || t.chars().all(|c| c.is_ascii_alphabetic())
        };
        if !fails {
            return unsupported(
                "SetValue of a Variant holding a Text that is neither digits, blank nor letters on an Enum or Option control (not pinned)",
            );
        }
        Err(if self.has(al_version::Gate::ConversionErrorTextsFrom28) {
            error_code(
                format!("The value \"{t}\" can't be evaluated into type Option."),
                "Evaluate:Format",
            )
        } else {
            error_code("The type NavOption is unknown.".to_string(), "NotSupportedType")
        })
    }
}
