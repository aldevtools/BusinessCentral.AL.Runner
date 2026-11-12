//! What a TestPage SetValue makes of the entry on an Enum or Option control, before the generic conversion: a Text handed in a
//! Variant, and the entries a request page control bound to an Option or Enum field of a global record takes (moved out of
//! controls.rs; al-runner issue 7376).

use super::*;

/// The Option or Enum field a report's request page control is bound to, when the control has the shape
/// reports/request-page/RequestPageOptionFieldOfGlobalRecord.al pins (:93-101, BC 27.5, 28.5 and 29.0): the report's own field
/// control, directly in the content area, with no property but ApplicationArea and no trigger of its own, bound to an Option or
/// Enum field of a global record variable of the report that is not temporary and is no data item (`MySetup."Logo Position"`),
/// or to an Option field of an element of a global record array (`SetupArr[2]."Logo Position"`; `array_element_field` is set
/// only for an array of records that are not temporary). Such a control takes the field's type. Every other control keeps the
/// type of the value it shows (`control_type`): a data item's field, a temporary record, an Enum field of an array element, a
/// control in a group or with Editable, Caption, OptionCaption or OnValidate are not pinned.
pub(super) fn record_choice_field(program: &ir::Program, report: usize, ctl: &ir::Control) -> Option<RtType> {
    let object = &program.objects[report];
    let items = &object.report.as_ref()?.items;
    let plain = ctl.kind == ir::ControlKind::Field
        && ctl.owner.is_none()
        && ctl.parent.is_none()
        && matches!(ctl.binding, ir::Binding::Expr(_))
        && matches!(ctl.triggers.as_slice(), [(n, _)] if n == "__sourceset")
        && ctl.ext_triggers.is_empty()
        && ctl.props.iter().all(|(n, _)| n == "applicationarea");
    if !plain {
        return None;
    }
    let field_type = |(t, p): (usize, usize)| program.objects[t].table.as_ref().map(|t| t.fields[p].ty.clone());
    if let (Some(at), Some(slot)) = (ctl.var_field, ctl.var_slot) {
        let global_record = matches!(object.globals.get(slot).map(|g| &g.ty), Some(RtType::Record(_, false)))
            && !items.iter().any(|i| i.slot == Some(slot));
        return field_type(at).filter(|ty| global_record && matches!(ty, RtType::Option(_) | RtType::Enum(_)));
    }
    field_type(ctl.array_element_field?).filter(|ty| matches!(ty, RtType::Option(_)))
}

impl Session {
    /// The Option or Enum field of a request page control of the pinned shape (`record_choice_field`).
    pub(super) fn request_record_choice_type(&self, st: &St, idx: usize) -> Option<RtType> {
        if !self.is_request_page(st) {
            return None;
        }
        let engine = self.engine();
        let report = st.borrow().object as usize;
        record_choice_field(&engine.program, report, &page_of(&engine, st).controls[idx])
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
    /// the digits of an Option value select it, and a text of letters fails to evaluate the same way: member names in any case,
    /// a prefix, a name with a space, a caption, the empty text (RequestPageOptionFieldOfGlobalRecord.al:205-259, :280-287,
    /// :339-346, :406-413, :465-468; words of ASCII letters with single spaces between them). Other texts (punctuation, other
    /// spacing) and other shapes are refused. Turns digits into the Integer.
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
            t.is_empty()
                || t.split(' ')
                    .all(|w| !w.is_empty() && w.chars().all(|c| c.is_ascii_alphabetic()))
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
