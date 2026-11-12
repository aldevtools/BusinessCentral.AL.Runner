//! The parameters text of a report request page control bound to a place written as an expression: a field of a record
//! variable (`Buf.Name`), of an element of an array of records (`BufArr[1].Name`) or of the data item's record (`D.Name`), or
//! an element of a plain array (`TxtArr[2]`). Pinned by reports/request-page/R6960RequestParamBindings.al (al-runner
//! issue 6960):
//! - `Report.RunRequestPage` writes each such control into `<Options>`, in control order, named by its source as written
//!   (`Rrp_TypedValues_Text` :204, `Rrp_Defaults_Text` :221: an empty value is `<Field name="Buf.Name" />`);
//! - `Report.Execute` and `Report.RunRequestPage(Id, Parameters)` with that text set each place again
//!   (`Rrp_TypedValues_ThenExecute` :243, `Rrp_TypedValues_ThenSeededRequestPage` :266).
//!
//! A control of a report extension that changes the run, bound to a field of a data item (`BindRow.Name`), is listed the same way
//! (reports/extensions/W3RpExtRequestPage.al Bind_RunRequestPage_ParametersText_AllBindings); setting it through a parameters
//! text is not pinned. A control of such an extension bound to a field of the extension's own record variable (`Params.Factor`,
//! `Params."Per Zone"`) is listed by its source as written, quotes kept, and a parameters text that names it so sets it
//! (reports/extensions/W3RpExtRequestPage.al:1277 RunRequestPage_OwnRecordControls_ParametersText, :1409
//! RunRequestPage_SeededByQuotedFieldName, :1385 Execute_ParametersText_VariableAndFieldNames, corpus PR 1202). Other
//! expressions (a call, a quoted name elsewhere, an index that is not a number) are not pinned and are refused.

use super::*;

type Inst = Rc<RefCell<al_builtins::value::Instance>>;

/// Whether `text` has a form the probe pins: a name, an optional number index, and an optional unquoted field name; with
/// `quoted` (a control of a report extension, or one bound to a field of a global record) the field name may also be in double
/// quotes (W3RpExtRequestPage.al:1281).
fn pinned_shape(text: &str, quoted: bool) -> bool {
    let ident = |s: &str| {
        let mut c = s.chars();
        c.next().is_some_and(|f| f.is_ascii_alphabetic() || f == '_')
            && c.all(|x| x.is_ascii_alphanumeric() || x == '_')
    };
    let (head, field) = match text.split_once('.') {
        Some((h, f)) => (h, Some(f)),
        None => (text, None),
    };
    let quoted_field = |f: &str| {
        quoted
            && f.strip_prefix('"')
                .and_then(|f| f.strip_suffix('"'))
                .is_some_and(|n| !n.is_empty() && !n.contains('"'))
    };
    if field.is_some_and(|f| !ident(f) && !quoted_field(f)) {
        return false;
    }
    match head.split_once('[') {
        Some((name, index)) => {
            ident(name)
                && index
                    .strip_suffix(']')
                    .is_some_and(|i| !i.is_empty() && i.bytes().all(|b| b.is_ascii_digit()))
        }
        None => ident(head),
    }
}

/// Whether the source of control `c` (an expression) starts with the name of a data item of the request page `page`.
pub(super) fn names_data_item(page: &ir::Page, c: &ir::Control) -> bool {
    let head = c
        .source_text
        .as_deref()
        .and_then(|t| t.split(['.', '[']).next())
        .unwrap_or("");
    page.controls
        .iter()
        .any(|d| d.kind == ir::ControlKind::DataItem && d.name.eq_ignore_ascii_case(head))
}

/// Whether a parameters text name sets nothing and is skipped: the control name of a report extension control bound to a field
/// of the extension's own record (`PerZoneCtl`), or its source with the quotes of the field name left out (`Params.Per Zone`);
/// BC keeps the values (reports/extensions/W3RpExtRequestPage.al:1329 RunRequestPage_SeededByControlName, :1341
/// RunRequestPage_SeededByVariableAndFieldName; Report.Execute skips every unknown name anyway).
pub(super) fn ignored_extension_name(page: &ir::Page, name: &str) -> bool {
    page.controls.iter().any(|c| {
        c.kind == ir::ControlKind::Field
            && c.owner.is_some()
            && matches!(c.binding, ir::Binding::Expr(_))
            && !names_data_item(page, c)
            && (c.name == name || c.source_text.as_deref().is_some_and(|t| t.replace('"', "") == name))
    })
}

impl Session {
    /// For a request page control bound to an expression: its parameter name and the routines that read and write its
    /// place, when the shape is pinned; otherwise the refusal.
    /// `report` is the report object the request page belongs to.
    pub(super) fn request_expr_place(
        &self,
        report: usize,
        c: &ir::Control,
    ) -> Result<(String, usize, usize, Option<usize>), Signal> {
        let refuse = || {
            unsupported(format!(
                "the parameters text for request page control {} bound to this expression (not pinned)",
                c.name
            ))
        };
        let ir::Binding::Expr(read) = c.binding else {
            return refuse();
        };
        let (Some(text), Some(write)) = (c.source_text.as_deref(), c.trigger("__sourceset")) else {
            return refuse();
        };
        // An Option field of the report's own global record or of an element of its record array, on a control of the pinned
        // shape (`choice_entry::record_choice_field`), may be quoted too (`MySetup."Logo Position"`, `SetupArr[2]."Logo
        // Position"`: reports/request-page/RequestPageOptionFieldOfGlobalRecord.al:514, RequestPage_FieldsOfGlobalRecord_
        // ParametersText). A quoted field of another type is not pinned.
        let option_field = || {
            let engine = self.engine();
            matches!(
                super::choice_entry::record_choice_field(&engine.program, report, c),
                Some(RtType::Option(_))
            )
        };
        let quoted = c.owner.is_some() || option_field();
        if !pinned_shape(text, quoted) {
            return refuse();
        }
        Ok((text.to_string(), read, write, c.owner))
    }

    /// The value of the place behind a request page control bound to an expression, read on the report's instance.
    pub(super) fn request_expr_read(&mut self, object: usize, inst: &Inst, read: usize) -> Result<Value, Signal> {
        self.call(object, read, inst.clone(), vec![]).map(|(v, _)| v)
    }

    /// Writes `text` from a parameters text into the place behind a request page control bound to an expression.
    pub(super) fn request_expr_write(
        &mut self,
        object: usize,
        inst: &Inst,
        name: &str,
        text: &str,
        (read, write): (usize, usize),
    ) -> Result<(), Signal> {
        let current = self.request_expr_read(object, inst, read)?;
        let value = self.request_option_value(name, text, &current)?;
        self.call(object, write, inst.clone(), vec![value]).map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::pinned_shape;

    #[test]
    fn only_the_probed_shapes_are_pinned() {
        for t in ["Buf.Name", "BufArr[1].Name", "TxtArr[2]", "D.Name", "PlainVal"] {
            assert!(pinned_shape(t, false), "{t}");
        }
        for t in [
            "Buf.\"No.\"",
            "Arr[i].Name",
            "Arr[1,2]",
            "Format(X)",
            "A.B.C",
            "X + Y",
            "Buf. Name",
        ] {
            assert!(!pinned_shape(t, false), "{t}");
        }
        assert!(pinned_shape("Params.\"Per Zone\"", true));
        assert!(!pinned_shape("Params.\"Per Zone\"", false));
        assert!(!pinned_shape("Params.\"A\"B\"", true));
    }
}
