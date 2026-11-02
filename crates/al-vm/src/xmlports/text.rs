//! VariableText and FixedText ports: import and export of delimited and fixed-width text.

use super::*;

impl Session {
    /// The root and table element of a text-format port: `Root > tableelement > fieldelements`.
    /// `text_columns`: a leaf text element may be a column too (the import of a VariableText port, xmlport/lt-aj-xmlport/
    /// AjRunAndXmlPort.al:QS_VariableText_TempInteger_AutoSaveFalse: its value goes into the variable).
    pub(super) fn xp_text_layout(
        &self,
        schema: &Schema,
        top: usize,
        text_columns: bool,
    ) -> Result<(usize, Vec<usize>), Signal> {
        // A text-format port may have its table element at the top, with no root element
        // (P94Fixtures.al: "LT P94 Csv Plain", exported by P94XpDelim.al:Export_NothingDeclared).
        let direct = schema.node(top).kind == ir::XmlNodeKind::Table;
        let kids = if direct { vec![top] } else { schema.children(top) };
        let [table] = kids[..] else {
            return unsupported(
                "a VariableText or FixedText port whose root has other than one table element (not pinned)",
            );
        };
        if schema.node(table).kind != ir::XmlNodeKind::Table {
            return unsupported("a VariableText or FixedText port whose root has no table element (not pinned)");
        }
        let fields = schema.children(table);
        // A field attribute of a FixedText port is a column in schema order like a field element, cut and padded to its Width
        // (xmlport/lt-ay-xmlport/AyXmlPortAttributeWidth.al:163 export, :181 attributes only, :244 import).
        let fixed = schema.xp().format == ir::XmlFormat::FixedText;
        let column = |f: usize| {
            let n = schema.node(f);
            n.kind == ir::XmlNodeKind::Field
                || (fixed && n.kind == ir::XmlNodeKind::FieldAttr)
                || (text_columns && n.kind == ir::XmlNodeKind::Text && schema.children(f).is_empty())
        };
        if fields.iter().any(|&f| !column(f)) || fields.is_empty() {
            return unsupported(
                "a VariableText or FixedText port whose table element holds more than field elements (not pinned)",
            );
        }
        if schema.xp().format == ir::XmlFormat::FixedText && fields.iter().any(|&f| schema.node(f).width.is_none()) {
            return unsupported("a FixedText field element without a Width");
        }
        Ok((table, fields))
    }

    pub(super) fn xp_import_text(
        &mut self,
        inst: &Inst,
        schema: &Schema,
        top: usize,
        text: &str,
    ) -> Result<(), Signal> {
        let xp = schema.xp();
        let (table, fields) = self.xp_text_layout(schema, top, xp.format == ir::XmlFormat::VariableText)?;
        // An encoding the triggers set on an import is not pinned (Y67XpSeparators.al pins it on an export only).
        if assigned::set_in_run(inst).is_some() {
            return unsupported("a VariableText import whose TextEncoding the program set (not pinned)");
        }
        let record_sep = non_empty(assigned::separator(inst, 2, xp.record_separator.as_deref(), "\r\n")?)?;
        let variable = xp.format == ir::XmlFormat::VariableText;
        let unbound = unbound_column(schema, &fields, variable)?;
        let (fsep, fdelim) = (
            non_empty(assigned::separator(inst, 1, xp.field_separator.as_deref(), ",")?)?,
            assigned::separator(inst, 0, xp.field_delimiter.as_deref(), "\"")?,
        );
        for line in text.split(record_sep.as_str()).filter(|l| !l.is_empty()) {
            let values: Vec<String> = if variable {
                split_variable(line, &fsep, &fdelim)
            } else {
                let chars: Vec<char> = line.chars().collect();
                let mut at = 0;
                fields
                    .iter()
                    .map(|&f| {
                        let w = schema.node(f).width.expect("checked by xp_text_layout");
                        let end = (at + w).min(chars.len());
                        let v: String = chars[at.min(end)..end].iter().collect();
                        at = end;
                        v
                    })
                    .collect()
            };
            let n = schema.node(table);
            let rec = self.xp_record(inst, n)?;
            let tbl = rec.borrow().table as usize;
            let fresh = self.initial_fields(tbl);
            rec.borrow_mut().fields = fresh;
            self.xp_run_trigger(inst, schema, table, "onafterinitrecord")?;
            // Fields are read as the values come; which line lengths are pinned: `line_length_pinned`.
            let length_pinned = line_length_pinned(schema, &fields, values.len(), variable);
            let mut assigned = Vec::new();
            let mut broke = false;
            for (&f, v) in fields.iter().zip(&values) {
                if schema.node(f).kind == ir::XmlNodeKind::Text {
                    self.xp_assign_variable(inst, schema, f, v.clone())?;
                    continue;
                }
                let is_text = {
                    let pos = schema.node(f).field.expect("field nodes have a field");
                    matches!(
                        self.engine.tables[tbl].as_ref().map(|t| &t.fields[pos].ty),
                        Some(RtType::Text(_) | RtType::Code(_))
                    )
                };
                // A fixed-width field is padded with spaces.
                let v = if variable {
                    v.as_str()
                } else if is_text {
                    v.trim_end()
                } else {
                    v.trim()
                };
                let (pos, flow) = self.xp_assign_field_flow(inst, schema, f, v)?;
                assigned.push(pos);
                // A Break after a field is assigned leaves the other fields of the record unassigned (P112XpCsv.al:
                // P112_Import_Break, idAfter2: `2=/0`); the record is still stored.
                if flow == Some(Flow::Break) {
                    broke = true;
                    break;
                }
            }
            // The unbound text element (the last column) takes the values after the others (`unbound_column`).
            if let Some(f) = unbound.filter(|_| !broke) {
                for v in values.iter().skip(fields.len()) {
                    self.xp_assign_variable(inst, schema, f, v.clone())?;
                }
            }
            if !broke && !length_pinned {
                return unsupported(format!(
                    "a record with {} values for a port of {} fields (not pinned)",
                    values.len(),
                    fields.len()
                ));
            }
            self.xp_save_record(inst, schema, table, &assigned, false)?;
            if Self::xp_import_stopped(inst) {
                break;
            }
        }
        Ok(())
    }

    pub(super) fn xp_export_text(&mut self, inst: &Inst, schema: &Schema, top: usize) -> Result<Vec<u8>, Signal> {
        let xp = schema.xp();
        let (table, fields) = self.xp_text_layout(schema, top, false)?;
        let variable = xp.format == ir::XmlFormat::VariableText;
        // The separators are read for each row after its OnAfterGetRecord: one set there applies from that row on
        // (Y67XpSeparators.al:420-439 Assign_InOnAfterGetRecord_Export, m101, m111, m114).
        let mut record_sep: Option<String> = None;
        let mut lines = Vec::new();
        let rows = for_each_row(self, inst, schema, table, |me, inst, schema| {
            let (fsep, fdelim) = (
                assigned::separator(inst, 1, xp.field_separator.as_deref(), ",")?,
                assigned::separator(inst, 0, xp.field_delimiter.as_deref(), "\"")?,
            );
            let rsep = assigned::separator(inst, 2, xp.record_separator.as_deref(), "\r\n")?;
            match &record_sep {
                Some(r) if *r != rsep => {
                    return unsupported("a RecordSeparator set after the first row of an export (not pinned)");
                }
                _ => record_sep = Some(rsep),
            }
            let mut cells = Vec::new();
            for &f in &fields {
                let v = match me.xp_field_pass(inst, schema, f) {
                    Ok(FieldPass::Value(v)) => v,
                    Ok(FieldPass::Omit) => continue,
                    Ok(FieldPass::Stop) => break,
                    // A Quit in a field leaves what was written, the record cut short (P112_Export_Quit, nameBefore2).
                    Err(Signal::Exit(None)) if Session::xp_flow(inst) == Some(Flow::Quit) => {
                        if !cells.is_empty() {
                            lines.push(cells.join(if variable { fsep.as_str() } else { "" }));
                        }
                        return Err(Signal::Exit(None));
                    }
                    Err(e) => return Err(e),
                };
                cells.push(if variable {
                    format!("{fdelim}{v}{fdelim}")
                } else {
                    let w = schema.node(f).width.expect("checked by xp_text_layout");
                    let mut cut: String = v.chars().take(w).collect();
                    while cut.chars().count() < w {
                        cut.push(' ');
                    }
                    cut
                });
            }
            lines.push(cells.join(if variable { fsep.as_str() } else { "" }));
            Ok(())
        });
        // A Quit ends the export with what was written so far; the request stays set for `xp_run`.
        match rows {
            Err(Signal::Exit(None)) if Self::xp_flow(inst) == Some(Flow::Quit) => {}
            other => other?,
        }
        let record_sep = match record_sep {
            Some(r) => r,
            None => assigned::separator(inst, 2, xp.record_separator.as_deref(), "\r\n")?,
        };
        let text = lines.join(&record_sep);
        // An encoding the program set wins over the declared one (Y67XpSeparators.al:393-418 TextEncoding_AssignForms_Export:
        // UTF16 72 bytes, MSDos and Windows 35 without a byte order mark, UTF8 38 with one; :420 m121 set in OnAfterGetRecord).
        let (utf8, encoding) = match assigned::set_in_run(inst) {
            Some(_) if !variable => {
                return unsupported("a FixedText export whose TextEncoding the program set (not pinned)");
            }
            Some(1) => (true, None),
            Some(0) => (false, Some("msdos")),
            Some(2) => (false, Some("utf16")),
            Some(3) => (false, Some("windows")),
            Some(_) => return unsupported("a TextEncoding member the export does not know"),
            None => (xp.utf8, xp.text_encoding.as_deref()),
        };
        use al_builtins::stream::Encoding;
        if utf8 {
            let mut out = vec![0xEF, 0xBB, 0xBF];
            out.extend_from_slice(text.as_bytes());
            Ok(out)
        } else if variable && encoding == Some("utf16") {
            // UTF-16 little endian after FF FE (xmlport/W7XmlPortTests.al:368; XpTEncoding.al:135, 31 characters in 64 bytes).
            let mut out = vec![0xFF, 0xFE];
            out.extend(Encoding::Utf16.encode(&text));
            Ok(out)
        } else if variable && encoding == Some("windows") {
            // Code page 1252, `?` for a character it lacks (W7XmlPortTests.al:366 `E9`; XpTEncoding.al:90 `ü` and `ß`, :105
            // `€` as 80, :120 `✓` as `?`). A character outside the basic plane is not pinned.
            if text.chars().any(|c| (c as u32) > 0xFFFF) {
                return unsupported(
                    "a VariableText export with TextEncoding = WINDOWS of a character outside the basic plane (not pinned)",
                );
            }
            Ok(Encoding::Windows.encode(&text))
        } else if text.is_ascii() {
            Ok(text.into_bytes())
        } else if variable
            && matches!(encoding, None | Some("msdos"))
            && !text.contains('€')
            && Encoding::MsDos.encode_pinned(&text)
        {
            // Code page 437, also without a TextEncoding (W7XmlPortTests.al:365 and :369, `é` as 82; XpTEncoding.al:150
            // `ü` and `ß`). The euro sign and a character the code page lacks are not pinned for an xmlport.
            Ok(Encoding::MsDos.encode(&text))
        } else {
            unsupported(
                "a VariableText or FixedText export with characters beyond ASCII and no TextEncoding (not pinned)",
            )
        }
    }

    // ---------------------------------------------------------------- export
}

pub(super) fn declared_separator(v: &str) -> Result<String, Signal> {
    Ok(match v.to_ascii_uppercase().as_str() {
        "<NONE>" => String::new(),
        "<CR/LF>" => "\\r\\n".into(),
        "<LF>" => "\\n".into(),
        "<CR>" => "\\r".into(),
        "<TAB>" => "\\t".into(),
        u if u.starts_with('<') && u.ends_with('>') => {
            return unsupported(format!("the separator token {v} read back (not pinned)"));
        }
        _ => v.to_string(),
    })
}

/// A field or record separator has to hold text; an empty one is not pinned.
pub(super) fn non_empty(s: String) -> Result<String, Signal> {
    if s.is_empty() {
        unsupported("an empty field or record separator (not pinned)")
    } else {
        Ok(s)
    }
}

pub(super) fn text_separator(value: Option<&str>, default: &str) -> Result<String, Signal> {
    let Some(v) = value else { return Ok(default.to_string()) };
    let mut out = String::new();
    let mut rest = v;
    while let Some(start) = rest.find('<') {
        out.push_str(&rest[..start]);
        let Some(end) = rest[start..].find('>') else {
            out.push_str(&rest[start..]);
            rest = "";
            break;
        };
        match rest[start + 1..start + end].to_ascii_uppercase().as_str() {
            "LF" => out.push('\n'),
            "CR" => out.push('\r'),
            "TAB" => out.push('\t'),
            // <NewLine> writes CR LF (XpTVariableText.al:Csv_ExportThreeRows reads the lines back as <NL>-separated).
            "NEWLINE" | "CR/LF" => out.push_str("\r\n"),
            // <None> is the empty text (XmlPortFixtures.al "LT XpT Csv Semi" has no field delimiter).
            "NONE" => {}
            other => return unsupported(format!("the separator token <{other}> (not pinned)")),
        }
        rest = &rest[start + end + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

/// Splits a VariableText record at `sep`. A value that starts with the delimiter runs to the next
/// delimiter and may hold the separator; a doubled delimiter is not an escape, so the text after
/// the closing delimiter starts the next value
/// (xmlport/TestXmlPortTextFormatsContracts.al:VariableText_Import_DoubledQuoteInsideAQuotedValue_IsNotAnEscape,
/// corpus: .
pub(super) fn split_variable(line: &str, sep: &str, delim: &str) -> Vec<String> {
    if delim.is_empty() {
        return line.split(sep).map(str::to_string).collect();
    }
    let mut out = Vec::new();
    let mut rest = line;
    loop {
        if let Some(after) = rest.strip_prefix(delim) {
            match after.find(delim) {
                Some(end) => {
                    out.push(after[..end].to_string());
                    rest = &after[end + delim.len()..];
                    match rest.strip_prefix(sep) {
                        Some(r) => rest = r,
                        None if rest.is_empty() => return out,
                        None => {}
                    }
                    if rest.is_empty() && !line.ends_with(sep) {
                        return out;
                    }
                }
                None => {
                    out.push(after.to_string());
                    return out;
                }
            }
        } else {
            match rest.find(sep) {
                Some(end) => {
                    out.push(rest[..end].to_string());
                    rest = &rest[end + sep.len()..];
                }
                None => {
                    out.push(rest.to_string());
                    return out;
                }
            }
        }
    }
}
