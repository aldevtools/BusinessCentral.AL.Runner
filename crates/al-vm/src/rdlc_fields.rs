//! The stream `Report.RdlcLayout` gives for a report with an RDLC layout file: the file with its `<Fields>` list replaced by the
//! report's data set columns (reports/request-page/Y86LayoutStreams.al:82 Y86_RdlcLayout_StreamIsTheFile `length` 1917
//! for a 2316-byte file with four fields and a report without columns, `lines` 73, `line 3` and `line 4` unchanged;
//! :230 Y86_RdlcLayout_FieldsSectionFollowsTheDataset `<Fields />` for no columns and, for `NumberCol` (Integer) and `TextCol`
//! ('abc'), one `<Field Name=..><DataField>..</DataField></Field>` per column, nested one level deeper each, in the line
//! ending and indent of the file).
//!
//! Pinned for columns whose type is Integer, BigInteger, Boolean, Text or Code and whose name is made of letters, digits and
//! `_`, and for a file with CRLF line ends and one `<Fields>` element. A Decimal column adds a format field in BC's dataset
//! (not pinned for this stream), a name with other characters is renamed by BC (how is not pinned): the stream is then refused.

use crate::Session;
use al_builtins::native::{Signal, unsupported};
use al_builtins::value::RtType;

impl Session {
    /// The bytes of the `Report.RdlcLayout` stream of report `object` when its layout file is `file`: the file with the data set
    /// columns in its Fields list; a file without one stays as it is, marked as the unprobed assumption
    /// `rdlc-layout-stream-is-the-layout-file`.
    pub(crate) fn rdlc_layout_stream(&self, object: usize, file: Vec<u8>) -> Result<Vec<u8>, Signal> {
        if let Some(bytes) = self.rdlc_stream_with_dataset_fields(object, &file)? {
            return Ok(bytes);
        }
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::RdlcLayoutStreamIsTheLayoutFile);
        Ok(file)
    }

    /// The bytes of the `Report.RdlcLayout` stream of report `object` when its layout file is `file`. `None`: the file holds no
    /// `<Fields>` element, so nothing is pinned and the caller keeps the file as it was (and marks the assumption).
    pub(crate) fn rdlc_stream_with_dataset_fields(
        &self,
        object: usize,
        file: &[u8],
    ) -> Result<Option<Vec<u8>>, Signal> {
        let Ok(text) = std::str::from_utf8(file) else {
            return Ok(None);
        };
        let open = ["<Fields>", "<Fields />"];
        let found: Vec<(usize, &str)> = open
            .iter()
            .flat_map(|o| text.match_indices(o).map(|(i, _)| (i, *o)))
            .collect();
        let [(at, tag)] = found[..] else {
            return if found.is_empty() {
                Ok(None)
            } else {
                unsupported("Report.RdlcLayout of a layout file with several Fields elements (not pinned)")
            };
        };
        let end = if tag == "<Fields>" {
            match text[at..].find("</Fields>") {
                Some(e) => at + e + "</Fields>".len(),
                None => return Ok(None),
            }
        } else {
            at + tag.len()
        };
        if !text.contains("\r\n") || text.replace("\r\n", "").contains(['\r', '\n']) {
            return unsupported("Report.RdlcLayout of a layout file without CRLF line ends (not pinned)");
        }
        let line_start = text[..at].rfind('\n').map_or(0, |i| i + 1);
        let indent = &text[line_start..at];
        if !indent.chars().all(|c| c == ' ') {
            return unsupported(
                "Report.RdlcLayout of a layout file whose Fields element does not start its line (not pinned)",
            );
        }
        let names = self.rdlc_dataset_column_names(object)?;
        let mut fields = String::new();
        if names.is_empty() {
            fields.push_str("<Fields />");
        } else {
            fields.push_str("<Fields>\r\n");
            for n in &names {
                fields.push_str(&format!(
                    "{indent}  <Field Name=\"{n}\">\r\n{indent}    <DataField>{n}</DataField>\r\n{indent}  </Field>\r\n"
                ));
            }
            fields.push_str(&format!("{indent}</Fields>"));
        }
        let mut out = String::with_capacity(text.len());
        out.push_str(&text[..at]);
        out.push_str(&fields);
        out.push_str(&text[end..]);
        Ok(Some(out.into_bytes()))
    }

    /// The names of the columns of the report's data items, in source order; refused for a column the stream is not pinned for.
    fn rdlc_dataset_column_names(&self, object: usize) -> Result<Vec<String>, Signal> {
        let o = &self.engine.program.objects[object];
        let Some(r) = o.report.as_ref() else {
            return Ok(Vec::new());
        };
        let mut names = Vec::new();
        for item in &r.items {
            for c in &item.columns {
                let plain_name = !c.name.is_empty() && c.name.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_');
                let ty = self.rdlc_column_type(o, item.table, c);
                let plain_type = matches!(
                    ty,
                    Some(RtType::Integer | RtType::BigInteger | RtType::Boolean | RtType::Text(_) | RtType::Code(_))
                );
                if c.unsupported.is_some()
                    || !plain_name
                    || !plain_type
                    || c.decimal_places.is_some()
                    || c.option_caption_ml.is_some()
                    || c.auto_format_type.is_some()
                    || c.auto_format_expression.is_some()
                    || c.include_caption
                {
                    return unsupported(format!(
                        "Report.RdlcLayout of a report whose column {} the layout's Fields list is not pinned for",
                        c.name
                    ));
                }
                names.push(c.name.clone());
            }
        }
        Ok(names)
    }

    /// The type of a column's source: the type of the table field it names, or of the constant it is.
    fn rdlc_column_type(&self, o: &al_ir::Object, table: Option<usize>, c: &al_ir::ReportColumn) -> Option<RtType> {
        if let Some(pos) = c.field.or(c.item_field) {
            let t = self.engine.program.objects.get(table?)?.table.as_ref()?;
            return t.fields.get(pos).map(|f| f.ty.clone());
        }
        let routine = o.routines.get(c.routine?)?;
        let [al_ir::Stmt::Exit(Some(e))] = routine.body.stmts().as_slice() else {
            return None;
        };
        let mut e = e;
        while let al_ir::Expr::Convert(_, inner) = e {
            e = inner;
        }
        match e {
            al_ir::Expr::Const(al_ir::Const::Text(_)) => Some(RtType::Text(None)),
            al_ir::Expr::Const(al_ir::Const::Integer(_)) => Some(RtType::Integer),
            al_ir::Expr::Const(al_ir::Const::Boolean(_)) => Some(RtType::Boolean),
            _ => None,
        }
    }
}
