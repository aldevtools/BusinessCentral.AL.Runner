//! The text forms of DateFormula, BigInteger, RecordId, Blob, Media and MediaSet fields of an XmlPort, on export and import.
//! Every form is pinned by corpus xmlports/PWXmlPortFormatEvaluateMoreTypes.al (`LT AI XmlPort Types`); the cells it does not pin are refused.

use super::*;
use al_builtins::library_kind::LibraryKind;

const FILE: &str = "xmlports/PWXmlPortFormatEvaluateMoreTypes.al";

impl Session {
    /// The text an export writes for a field of one of these types, `None` for any other value. Export_Xml_PopulatedRow and
    /// Export_Xml_BlankRow (the same text under FormatEvaluate = Legacy: Export_Legacy_PopulatedAndBlankRow):
    /// `CM+1D` (no brackets) and an empty element for a blank DateFormula, the digits of a BigInteger, `LT AI Table: 1` and
    /// an empty element for a RecordId, `*` for a Blob with content and a space for an empty one, the empty Guid in braces for a
    /// Media and a MediaSet. Export_VariableText_XmlAndLegacy: the same text in a VariableText port, a Media is not pinned there.
    /// A field attribute with an empty text is left out by the caller (Export_Attributes_Xml_PopulatedAndBlankRow); a Blob
    /// or Media attribute is not pinned.
    pub(super) fn xp_new_type_text(
        &mut self,
        v: &Value,
        schema: &Schema,
        node: usize,
    ) -> Option<Result<String, Signal>> {
        let attribute = schema.node(node).kind == ir::XmlNodeKind::FieldAttr;
        match v {
            Value::DateFormula(_) => Some(
                al_builtins::format::try_format(v, 0, 0, self)
                    .or_else(|why| unsupported(format!("XmlPort export of a DateFormula: {why}"))),
            ),
            Value::BigInteger(i) if *i >= 0 => Some(Ok(i.to_string())),
            Value::Library(LibraryKind::RecordId, _) => Some(
                al_builtins::format::try_format(v, 0, 0, self)
                    .or_else(|why| unsupported(format!("XmlPort export of a RecordId: {why}"))),
            ),
            Value::Blob(_) if attribute => Some(unsupported(
                "XmlPort export of a Blob field attribute (not pinned, probes-needed: xmlport-blob-attribute)",
            )),
            Value::Blob(b) => {
                let buf = b.borrow();
                Some(match buf.observe() {
                    Ok(()) => Ok(if buf.bytes.is_empty() { " " } else { "*" }.to_string()),
                    Err(e) => Err(e),
                })
            }
            _ => None,
        }
    }

    /// A Media or MediaSet value: the empty Guid in braces on a Format = Xml port (Export_Xml_PopulatedRow `<Md>{0000...}</Md>`),
    /// anything else is not pinned. `kind` is the field's own type, the Guid is how a Media holds its id.
    pub(super) fn xp_media_text(&self, v: &Value, schema: &Schema, node: usize) -> Result<String, Signal> {
        match v {
            Value::Guid(g)
                if *g == [0; 16]
                    && schema.xp().format == ir::XmlFormat::Xml
                    && schema.node(node).kind == ir::XmlNodeKind::Field =>
            {
                Ok(al_builtins::format::format_guid(g))
            }
            _ => unsupported(format!(
                "XmlPort export of a Media or MediaSet field that holds content, as an attribute or in a text format (not pinned; only the empty one is, {FILE})"
            )),
        }
    }

    /// An import into a Format = Xml port with a Blob, Media or MediaSet field fails with NotSupportedType whatever the document
    /// holds, also a blank value (Import_Xml_Blob_MediaAndMediaSet: `The type BLOB is not supported.`, `The type Media is not
    /// supported.`, `The type MediaSet is not supported.`, nothing stored). Pinned for FormatEvaluate = Xml and Legacy.
    pub(super) fn xp_import_unsupported_types(&self, schema: &Schema) -> Result<(), Signal> {
        if schema.xp().format != ir::XmlFormat::Xml {
            return Ok(());
        }
        for n in &schema.xp().nodes {
            let (Some(tn), Some(pos)) = (n.table_node, n.field) else {
                continue;
            };
            let Some(table) = schema.node(tn).table else {
                continue;
            };
            let Some(t) = &self.engine.tables[table] else {
                continue;
            };
            let name = match &t.fields[pos].ty {
                RtType::Library(LibraryKind::Blob) => "BLOB",
                RtType::Library(LibraryKind::Media) => "Media",
                RtType::Library(LibraryKind::MediaSet) => "MediaSet",
                _ => continue,
            };
            if !schema.xp().format_evaluate_declared {
                return unsupported(format!(
                    "XmlPort import with a {name} field and no FormatEvaluate (not pinned, {FILE} declares it)"
                ));
            }
            return Err(xerr(format!("The type {name} is not supported."), "NotSupportedType"));
        }
        Ok(())
    }

    /// A DateFormula text read as Evaluate does: `<1D>`, `<CM+1D>`, `<-1W>` and `1D` store the formula, the empty text the empty
    /// formula, and a text that is no formula raises the error of Evaluate (Import_Xml_DateFormula, Import_Legacy_DateFormula).
    pub(super) fn xp_date_formula(&mut self, text: &str) -> Result<Value, Signal> {
        if text.trim().is_empty() {
            return Ok(Value::DateFormula(String::new()));
        }
        // The formula and its error texts are the ones of Evaluate in English; another language is not pinned.
        if self.global_language != 1033 {
            return unsupported(format!(
                "XmlPort import of a DateFormula under the language {} (not pinned)",
                self.global_language
            ));
        }
        use al_builtins::datefns::Evaluated;
        match al_builtins::datefns::evaluate_formula(text)? {
            Evaluated::Stored(f) => Ok(Value::DateFormula(f)),
            Evaluated::Rejected(Some(m)) | Evaluated::Failed(m) => Err(xerr(m, "Evaluate")),
            Evaluated::Rejected(None) => match al_builtins::datefns::time_unit_error(text) {
                Some(m) => Err(xerr(m, "Evaluate")),
                None => unsupported(format!(
                    "XmlPort import of the DateFormula text `{text}` (the error text is not pinned)"
                )),
            },
        }
    }

    /// A RecordId text under FormatEvaluate = Xml: the display text, `Table65200: 1`, `65200: 1`, and the empty text for the blank one
    /// (Import_Xml_RecordId); a text that is none of them raises `The value "garbage" can't be evaluated into type RecordID.`.
    pub(super) fn xp_record_id_xml(&mut self, text: &str, ty: &RtType) -> Result<Value, Signal> {
        if text.trim().is_empty() {
            return Ok(self.default_value(ty));
        }
        // `Table65200: 1` and `65200: 1` name the table by its number (Import_Xml_RecordId `rid_table_id`, `rid_number`): read
        // as the table's name, so that the last error is not set by a first try.
        let mut shown = text.to_string();
        if let Some((head, key)) = text.split_once(':') {
            let head = head.trim();
            let digits = head
                .get(..5)
                .filter(|p| p.eq_ignore_ascii_case("table"))
                .map_or(head, |_| &head[5..]);
            if let Ok(id) = digits.trim().parse::<u32>() {
                let names: Vec<String> = self
                    .engine
                    .program
                    .objects
                    .heads()
                    .iter()
                    .filter(|o| o.is_kind(ir::ObjKind::Table) && o.id == Some(id))
                    .map(|o| o.name.clone())
                    .collect();
                if let [name] = names.as_slice() {
                    shown = format!("{name}:{key}");
                }
            }
        }
        let mut args = [self.default_value(ty), Value::Text(shown.into())];
        self.ret_used = true;
        if let Value::Boolean(true) = al_builtins::evaluate::evaluate(self, None, &mut args)? {
            return Ok(args[0].clone());
        }
        Err(evaluate_error(text, "RecordID"))
    }
}

/// A BigInteger element of a FormatEvaluate = Legacy port that holds digit groups (`1,234`) reads as a number and then fails the
/// schema's `long` check (Import_Legacy_BigInteger `big_thousands`); a text with letters fails with Evaluate (`big_text`).
pub(super) fn legacy_big_thousands(text: &str) -> bool {
    let digits = text.strip_prefix('-').unwrap_or(text);
    let mut parts = digits.split(',');
    let first = parts.next().unwrap_or("");
    let rest: Vec<&str> = parts.collect();
    !rest.is_empty()
        && (1..=3).contains(&first.len())
        && first.bytes().all(|b| b.is_ascii_digit())
        && rest
            .iter()
            .all(|p| p.len() == 3 && p.bytes().all(|b| b.is_ascii_digit()))
}

#[cfg(test)]
mod tests {
    use super::legacy_big_thousands;

    #[test]
    fn digit_groups_are_thousands() {
        assert!(legacy_big_thousands("1,234"));
        assert!(legacy_big_thousands("-12,345,678"));
        assert!(!legacy_big_thousands("1234"));
        assert!(!legacy_big_thousands("12,34"));
        assert!(!legacy_big_thousands("abc"));
    }
}
