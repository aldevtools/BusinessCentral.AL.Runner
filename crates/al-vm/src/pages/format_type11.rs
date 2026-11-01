//! The format string of a Decimal table field with AutoFormatType 11, for `Format` in code (al-builtins `autoformat11`):
//! the AutoFormatExpression evaluated once per call, a text literal as written (types/lt-y43-auto-format-11/Y43AfType11.al).
//! A procedure runs on the formatted record itself, so the globals of its table and table extensions are the record's
//! (`Ext11`: a global setup record of the extension, read again at each call after the setup changed).

use super::*;

impl Session {
    pub(super) fn auto_format_type11_string(&mut self, f: &al_ir::Field, rec: &Rec) -> Result<String, Signal> {
        let text = f
            .properties
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("AutoFormatExpression"))
            .map(|(_, v)| v.trim().to_string())
            .unwrap_or_default();
        if let Some((object, routine)) = f.auto_format_expression {
            return match self.call_on_record(object, routine, rec.clone(), None, Vec::new())?.0 {
                Value::Text(t) | Value::Code(t) => Ok(t.into_string()),
                other => unsupported(format!(
                    "the AutoFormatExpression of field {} gave a {}",
                    f.name,
                    other.describe()
                )),
            };
        }
        match text.strip_prefix('\'').and_then(|t| t.strip_suffix('\'')) {
            Some(t) if text.len() >= 2 => Ok(t.replace("''", "'")),
            _ => unsupported(format!(
                "Format of field {} whose AutoFormatExpression `{text}` did not compile",
                f.name
            )),
        }
    }
}
