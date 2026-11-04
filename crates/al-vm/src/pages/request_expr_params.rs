//! The parameters text of a report request page control bound to a place written as an expression: a field of a record
//! variable (`Buf.Name`), of an element of an array of records (`BufArr[1].Name`) or of the data item's record (`D.Name`), or
//! an element of a plain array (`TxtArr[2]`). Pinned by reports/request-page/R6960RequestParamBindings.al (al-runner
//! issue 6960):
//! - `Report.RunRequestPage` writes each such control into `<Options>`, in control order, named by its source as written
//!   (`Rrp_TypedValues_Text` :204, `Rrp_Defaults_Text` :221: an empty value is `<Field name="Buf.Name" />`);
//! - `Report.Execute` and `Report.RunRequestPage(Id, Parameters)` with that text set each place again
//!   (`Rrp_TypedValues_ThenExecute` :243, `Rrp_TypedValues_ThenSeededRequestPage` :266).
//!
//! Other expressions (a call, a quoted name, an index that is not a number, a control of a report extension) are not pinned
//! and are refused.

use super::*;

type Inst = Rc<RefCell<al_builtins::value::Instance>>;

/// Whether `text` has a form the probe pins: a name, an optional number index, and an optional unquoted field name.
fn pinned_shape(text: &str) -> bool {
    let ident = |s: &str| {
        let mut c = s.chars();
        c.next().is_some_and(|f| f.is_ascii_alphabetic() || f == '_')
            && c.all(|x| x.is_ascii_alphanumeric() || x == '_')
    };
    let (head, field) = match text.split_once('.') {
        Some((h, f)) => (h, Some(f)),
        None => (text, None),
    };
    if field.is_some_and(|f| !ident(f)) {
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

impl Session {
    /// For a request page control bound to an expression: its parameter name and the routines that read and write its
    /// place, when the shape is pinned; otherwise the refusal.
    pub(super) fn request_expr_place(&self, c: &ir::Control) -> Result<(String, usize, usize), Signal> {
        let refuse = || {
            unsupported(format!(
                "the parameters text for request page control {} bound to this expression (not pinned)",
                c.name
            ))
        };
        let ir::Binding::Expr(read) = c.binding else {
            return refuse();
        };
        let (Some(text), None, Some(write)) = (c.source_text.as_deref(), c.owner, c.trigger("__sourceset")) else {
            return refuse();
        };
        if !pinned_shape(text) {
            return refuse();
        }
        Ok((text.to_string(), read, write))
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
            assert!(pinned_shape(t), "{t}");
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
            assert!(!pinned_shape(t), "{t}");
        }
    }
}
