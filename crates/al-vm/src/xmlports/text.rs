//! VariableText and FixedText ports: import and export of delimited and fixed-width text.

use super::text_nested;
use super::*;

impl Session {
    /// The root and table element of a text-format port: `Root > tableelement > fieldelements`.
    /// `text_columns`: a leaf text element may be a column too (the import of a VariableText port, xmlports/shapes-properties/
    /// AjRunAndXmlPort.al:QS_VariableText_TempInteger_AutoSaveFalse: its value goes into the variable).
    /// The table elements of a text-format port with the columns of each. `many`: an export of a VariableText port may have two
    /// table elements below the root (streams/DgXmlPortLeads.al:Item106_TwoTablesDefaultSeparator); an import
    /// and other counts are not pinned.
    pub(super) fn xp_text_tables(
        &self,
        schema: &Schema,
        top: usize,
        text_columns: bool,
        many: bool,
    ) -> Result<Vec<(usize, Vec<usize>)>, Signal> {
        // A text-format port may have its table element at the top, with no root element
        // (P94Fixtures.al: "LT P94 Csv Plain", exported by P94XpDelim.al:Export_NothingDeclared).
        let direct = schema.node(top).kind == ir::XmlNodeKind::Table;
        let kids = if direct { vec![top] } else { schema.children(top) };
        // A FixedText port has two table elements only as the pair a LinkTable ties together (`linked_pair`).
        let two = many
            && kids.len() == 2
            && (schema.xp().format == ir::XmlFormat::VariableText || linked_pair(schema, &kids));
        if kids.len() != 1 && !two {
            return unsupported(
                "a VariableText or FixedText port whose root has other than one table element (not pinned)",
            );
        }
        if two && kids.iter().any(|&k| text_nested::child_table(schema, k).is_some()) {
            return unsupported("a VariableText export of two table elements with a nested one (not pinned)");
        }
        kids.into_iter()
            .map(|table| Ok((table, self.xp_text_columns(schema, table, text_columns)?)))
            .collect()
    }

    pub(super) fn xp_text_columns(
        &self,
        schema: &Schema,
        table: usize,
        text_columns: bool,
    ) -> Result<Vec<usize>, Signal> {
        if schema.node(table).kind != ir::XmlNodeKind::Table {
            return unsupported("a VariableText or FixedText port whose root has no table element (not pinned)");
        }
        let mut fields = schema.children(table);
        // The nested table element of a VariableText port is the caller's (`text_nested`).
        if schema.xp().format == ir::XmlFormat::VariableText && text_nested::child_table(schema, table).is_some() {
            fields.pop();
        }
        // A field attribute of a FixedText port is a column in schema order like a field element, cut and padded to its Width
        // (xmlports/shapes-properties/AyXmlPortAttributeWidth.al:163 export, :181 attributes only, :244 import).
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
        Ok(fields)
    }

    pub(super) fn xp_import_text(
        &mut self,
        inst: &Inst,
        schema: &Schema,
        top: usize,
        text: &str,
    ) -> Result<(), Signal> {
        let xp = schema.xp();
        let tables = self.xp_text_tables(schema, top, xp.format == ir::XmlFormat::VariableText, true)?;
        let [first, second] = &tables[..] else {
            return self.xp_import_text_table(inst, schema, (tables[0].0, &tables[0].1), text);
        };
        // Two table elements: the table separator ends the records of the first (Y83XpMiscTests.al:VariableText_TwoTables
        // `roundtrip two tables`, `two tables doc one` to `three`, needs the corpus bump). Without it the second table element
        // is expected where the document ends; with the separator twice the document is not pinned.
        let tsep = non_empty(assigned::separator(inst, 3, xp.table_separator.as_deref(), "\r\n\r\n")?)?;
        let mut parts = text.splitn(3, tsep.as_str());
        let (one, two) = (parts.next().unwrap_or(""), parts.next());
        if parts.next().is_some() {
            return unsupported(
                "a VariableText import of two table elements with the table separator twice (not pinned)",
            );
        }
        self.xp_import_text_table(inst, schema, (first.0, &first.1), one)?;
        match two {
            Some(t) => self.xp_import_text_table(inst, schema, (second.0, &second.1), t),
            None if schema.node(second.0).min_zero => Ok(()),
            None => Err(xerr(
                format!(
                    "The Element <{}> is expected by Min Occurs value: Once. Element received: <{}>.",
                    schema.node(second.0).xml_name,
                    schema.node(top).xml_name
                ),
                "XmlPortMetadata",
            )),
        }
    }

    /// The records of one table element of a text-format import.
    fn xp_import_text_table(
        &mut self,
        inst: &Inst,
        schema: &Schema,
        (table, fields): (usize, &[usize]),
        text: &str,
    ) -> Result<(), Signal> {
        let xp = schema.xp();
        // An encoding the triggers set on an import is pinned for a port that declares WINDOWS, set in OnPreXmlPort before the source
        // was decoded with it (`import_encoding`, XpVarTextWindowsImport.al:337, :349, :385); for other ports it is not
        // (Y67XpSeparators.al pins it on an export only).
        if assigned::set_in_run(inst).is_some() && !import_encoding::decoded_in_run(inst) {
            return unsupported("a VariableText import whose TextEncoding the program set (not pinned)");
        }
        let record_sep = non_empty(assigned::separator(inst, 2, xp.record_separator.as_deref(), "\r\n")?)?;
        let variable = xp.format == ir::XmlFormat::VariableText;
        let nested = text_nested::child_table(schema, table).filter(|_| variable);
        let unbound = unbound_column(schema, fields, variable)?;
        let (fsep, fdelim) = (
            non_empty(assigned::separator(inst, 1, xp.field_separator.as_deref(), ",")?)?,
            assigned::separator(inst, 0, xp.field_delimiter.as_deref(), "\"")?,
        );
        // A field separator that holds the record separator (CR/LF and LF) or the other way round: the document is read
        // separator by separator, not cut into records first.
        let presplit = (variable && (fsep.contains(record_sep.as_str()) || record_sep.contains(fsep.as_str())))
            .then(|| split_overlapping(text, &fsep, &record_sep, &fdelim));
        let mut lines: Vec<std::borrow::Cow<str>> = match &presplit {
            Some(p) => p.iter().map(|v| std::borrow::Cow::Owned(v.join(&fsep))).collect(),
            None => text
                .split(record_sep.as_str())
                .map(std::borrow::Cow::Borrowed)
                .collect(),
        };
        // The text after the last record separator is no record. A FixedText import skips every empty line as before.
        if presplit.is_none() && lines.last().is_some_and(|l| l.is_empty()) {
            lines.pop();
        }
        for (line_no, line) in lines.iter().enumerate() {
            let line: &str = line;
            if line.is_empty() {
                if !variable {
                    continue;
                }
                if let Some(e) = nested
                    .filter(|_| line_no > 0)
                    .and_then(|c| Self::xp_nested_import_error(schema, table, c, true))
                {
                    return Err(e);
                }
                // An empty first line of a VariableText document is a record of one empty value
                // (xmlports/shapes-properties/Y83XpMiscTests.al:VariableText_ImportValueCount_1 `vt empty first line`, needs the
                // corpus bump); pinned when the second column is a field element that must occur, so the import fails there. An
                // empty line elsewhere is not pinned.
                let second_needed = fields
                    .get(1)
                    .is_some_and(|&f| schema.node(f).kind == ir::XmlNodeKind::Field && !schema.node(f).min_zero);
                if line_no != 0 || !second_needed {
                    return unsupported("an empty line in a VariableText import (not pinned)");
                }
            }
            // `quoted`: which values of a VariableText line stood in the field delimiter (`text_element_value`).
            let (values, quoted): (Vec<String>, Vec<bool>) = if let Some(p) = &presplit {
                (p[line_no].clone(), Vec::new())
            } else if variable {
                split_variable_marked(line, &fsep, &fdelim).into_iter().unzip()
            } else {
                let values = {
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
                (values, Vec::new())
            };
            // A TAB inside a value: the evaluate error of such a value shows the TAB as a space (XpSeparatorTokens.al
            // VariableText_Import_SepTabLower_Plain, corpus branch al-runtime-rs-staging-r6963, `1 Alpha`); where BC changes it is not
            // pinned.
            if variable && values.iter().any(|v| v.contains('\t')) {
                return unsupported("a TAB inside a VariableText value (not pinned)");
            }
            if xp.preserve_whitespace && variable {
                if let Some(v) = values.iter().find(|v| {
                    v.as_str() != v.trim() && (fdelim.is_empty() || !line.contains(&format!("{fdelim}{v}{fdelim}")))
                }) {
                    return unsupported(format!(
                        "a VariableText import with PreserveWhiteSpace of the unquoted value `{v}` with white space around it (not pinned)"
                    ));
                }
            }
            let n = schema.node(table);
            let rec = self.xp_record(inst, n)?;
            let tbl = rec.borrow().table as usize;
            let fresh = self.initial_fields(tbl);
            rec.borrow_mut().fields = fresh;
            self.xp_run_trigger(inst, schema, table, "onafterinitrecord")?;
            import_encoding::encoding_unchanged_since_decoding(inst)?;
            // Fields are read as the values come; which line lengths are pinned: `line_length_pinned`.
            let length_pinned = line_length_pinned(schema, fields, values.len(), variable);
            let mut assigned = Vec::new();
            let mut broke = false;
            for (k, (&f, v)) in fields.iter().zip(&values).enumerate() {
                if schema.node(f).kind == ir::XmlNodeKind::Text {
                    let v = text_element_value(inst, xp, variable, v, quoted.get(k) == Some(&true))?;
                    self.xp_assign_variable(inst, schema, f, v)?;
                    continue;
                }
                let is_text = {
                    let pos = schema.node(f).field.expect("field nodes have a field");
                    matches!(
                        self.engine.tables[tbl].as_ref().map(|t| &t.fields[pos].ty),
                        Some(RtType::Text(_) | RtType::Code(_))
                    )
                };
                // A line feed inside a field value, not at its end (a trailing one is dropped, AgXmlPortImport.al:678), reads as
                // a space: `b<LF>2` under a CR/LF record separator gives `b 2`
                // (xmlports/shapes-properties/Y67XpSeparators.al:Assign_Import `record sep crlf token`). A carriage return
                // inside the value is not pinned and refused. Also for a Decimal: `7.25<LF>8` fails as `7.25 8`
                // (Y83XpMiscTests.al:VariableText_ImportValueCount_2 `vt lf only`, needs the corpus bump).
                let spaced;
                let v = if variable && v.trim_end_matches(['\n', '\r']).contains(['\n', '\r']) {
                    let body = v.trim_end_matches(['\n', '\r']);
                    if body.contains('\r') {
                        return unsupported(
                            "a VariableText import text value with a carriage return inside (not pinned)",
                        );
                    }
                    spaced = body.replace('\n', " ");
                    spaced.as_str()
                } else {
                    v.as_str()
                };
                // A fixed-width field is padded with spaces.
                let v = if variable {
                    v
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
                for (k, v) in values.iter().enumerate().skip(fields.len()) {
                    let v = text_element_value(inst, xp, variable, v, quoted.get(k) == Some(&true))?;
                    self.xp_assign_variable(inst, schema, f, v)?;
                }
            }
            // A line with fewer values than the port has columns fails at the first missing column that must occur, a field
            // element: `The Element <Amt> is expected by Min Occurs value: Once. Element received: <Item>.` before anything of the
            // record is stored (Y83XpMiscTests.al:VariableText_ImportValueCount_1 `vt two values`, `vt one value`; needs the corpus
            // bump). Several values too many are left out (`vt five values`, VariableText_ImportValueCount_2 `vt five values two
            // rows`).
            if !broke && variable && values.len() < fields.len() {
                let lacking = schema.node(fields[values.len()]);
                if lacking.kind == ir::XmlNodeKind::Field && !lacking.min_zero {
                    return Err(xerr(
                        format!(
                            "The Element <{}> is expected by Min Occurs value: Once. Element received: <{}>.",
                            lacking.xml_name,
                            schema.node(table).xml_name
                        ),
                        "XmlPortMetadata",
                    ));
                }
            }
            if !broke && !length_pinned && !(variable && values.len() > fields.len()) {
                return unsupported(format!(
                    "a record with {} values for a port of {} fields (not pinned)",
                    values.len(),
                    fields.len()
                ));
            }
            self.xp_save_record(inst, schema, table, &assigned, false)?;
            if let Some(e) = nested.and_then(|c| Self::xp_nested_import_error(schema, table, c, false)) {
                return Err(e);
            }
            if Self::xp_import_stopped(inst) {
                break;
            }
        }
        import_encoding::encoding_unchanged_since_decoding(inst)
    }

    pub(super) fn xp_export_text(&mut self, inst: &Inst, schema: &Schema, top: usize) -> Result<Vec<u8>, Signal> {
        let xp = schema.xp();
        let tables = self.xp_text_tables(schema, top, false, true)?;
        let variable = xp.format == ir::XmlFormat::VariableText;
        // The separators are read for each row after its OnAfterGetRecord: one set there applies from that row on
        // (Y67XpSeparators.al:420-439 Assign_InOnAfterGetRecord_Export, m101, m111, m114).
        let nested = match &tables[..] {
            [(table, _)] if variable => text_nested::child_table(schema, *table),
            _ => None,
        };
        let text = if let Some(child) = nested {
            let (table, fields) = &tables[0];
            self.xp_nested_text(inst, schema, *table, fields, child)?
        } else {
            let mut record_sep: Option<String> = None;
            let mut parts = Vec::new();
            for &(table, ref fields) in &tables {
                let mut lines = Vec::new();
                let rows = self.xp_text_rows(inst, schema, (table, fields), &mut lines, &mut record_sep);
                let sep = match &record_sep {
                    Some(r) => r.clone(),
                    None => assigned::separator(inst, 2, xp.record_separator.as_deref(), "\r\n")?,
                };
                parts.push((lines.join(&sep), lines.is_empty()));
                // A Quit ends the export with what was written so far; the request stays set for `xp_run`.
                match rows {
                    Err(Signal::Exit(None)) if Self::xp_flow(inst) == Some(Flow::Quit) => break,
                    other => other?,
                }
            }
            if let [(only, _)] = &parts[..] {
                only.clone()
            } else {
                // The table separator takes the place of the record separator between two tables (DgXmlPortLeads.al:
                // Item106_TwoTablesDefaultSeparator `export`: `"2","Beta"\r\n\r\n"7"`, _TwoTablesNewLineSeparator); the last record has
                // none. A table without a record is not pinned.
                // Pinned for the pair a LinkTable ties together when the first has no record, so the second has none either:
                // the text is the table separator alone (R6924eLinkSibling.al:956 VT_Export_NoHeader `||`, :1028 FT_Export_NoHeader).
                let no_header =
                    parts.iter().all(|(_, empty)| *empty) && linked_pair(schema, &[tables[0].0, tables[1].0]);
                if !no_header && parts.iter().any(|(_, empty)| *empty) {
                    return unsupported(
                        "a VariableText export of two table elements where one has no record (not pinned)",
                    );
                }
                let tsep = assigned::separator(inst, 3, xp.table_separator.as_deref(), "\r\n\r\n")?;
                parts.into_iter().map(|(t, _)| t).collect::<Vec<_>>().join(&tsep)
            }
        };
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
            // UTF-16 little endian after FF FE (xmlports/W7XmlPortTests.al:368; XpTEncoding.al:135, 31 characters in 64 bytes).
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

    /// The lines of one table element of a text-format export; a row's record separator is read after its OnAfterGetRecord
    /// (`record_sep`: one set there applies from that row on).
    fn xp_text_rows(
        &mut self,
        inst: &Inst,
        schema: &Schema,
        (table, fields): (usize, &[usize]),
        lines: &mut Vec<String>,
        record_sep: &mut Option<String>,
    ) -> Result<(), Signal> {
        let xp = schema.xp();
        let variable = xp.format == ir::XmlFormat::VariableText;
        for_each_row(self, inst, schema, table, |me, inst, schema| {
            let (fsep, fdelim) = (
                assigned::separator(inst, 1, xp.field_separator.as_deref(), ",")?,
                assigned::separator(inst, 0, xp.field_delimiter.as_deref(), "\"")?,
            );
            let rsep = assigned::separator(inst, 2, xp.record_separator.as_deref(), "\r\n")?;
            match &record_sep {
                Some(r) if *r != rsep => {
                    return unsupported("a RecordSeparator set after the first row of an export (not pinned)");
                }
                _ => *record_sep = Some(rsep),
            }
            let mut cells = Vec::new();
            for &f in fields {
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
        })
    }
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
        // The tokens are case-sensitive, and <> is no token: <>, <tab> and <none> are written as these characters
        // (xmlports/shapes-properties/XpSeparatorTokens.al, corpus branch al-runtime-rs-staging-r6963:
        // VariableText_Export_DelimEmptySepTab, _DelimNoneSepEmpty, _SepTabLower, _DelimNoneLower). Other spellings are not
        // pinned.
        match &rest[start + 1..start + end] {
            "LF" => out.push('\n'),
            "CR" => out.push('\r'),
            "TAB" => out.push('\t'),
            // <NewLine> and <CR/LF> write CR LF (XpTVariableText.al:Csv_ExportThreeRows reads the lines back as <NL>-separated;
            // XpSeparatorTokens.al VariableText_Export_SepNewLine, _SepCRLF).
            "NewLine" | "CR/LF" => out.push_str("\r\n"),
            // <None> is the empty text (XmlPortFixtures.al "LT XpT Csv Semi" has no field delimiter; XpSeparatorTokens.al
            // VariableText_Export_SepNone).
            "None" => {}
            literal @ ("" | "tab" | "none") => {
                out.push('<');
                out.push_str(literal);
                out.push('>');
            }
            other => return unsupported(format!("the separator token <{other}> (not pinned)")),
        }
        rest = &rest[start + end + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

/// The text a text element of a VariableText import gets for `v` (`quoted`: it stood in the field delimiter). A port without
/// TextEncoding keeps the value as it is, spaces too, as before. A port that declares WINDOWS (no encoding set, no PreserveWhiteSpace) drops the
/// one space at the end of a quoted value: `"say ""hi"""` gives `say`, not `say ` (xmlports/import/XpVarTextWindowsImport.al:409
/// VTW_Columns_WindowsFile_Semicolon_Quotes_Import `1/3=[say {}]`). Other spaces around a value of a WINDOWS port are not pinned
/// and refused. A FixedText import keeps the value as it was.
fn text_element_value(inst: &Inst, xp: &ir::XmlPort, variable: bool, v: &str, quoted: bool) -> Result<String, Signal> {
    let spaced = v.starts_with(' ') || v.ends_with(' ');
    if !variable || !spaced || xp.text_encoding.as_deref() != Some("windows") {
        return Ok(v.to_string());
    }
    let one_trailing = !v.starts_with(' ') && !v.ends_with("  ");
    if quoted && one_trailing && !xp.preserve_whitespace && assigned::set_encoding(inst).is_none() {
        return Ok(v[..v.len() - 1].to_string());
    }
    unsupported(
        "a text element value with spaces around it in a VariableText import under TextEncoding = WINDOWS, other than one space at the end of a quoted value (not pinned)",
    )
}

/// Splits a VariableText record at `sep`. A value that starts with the delimiter runs to the next
/// delimiter and may hold the separator; a doubled delimiter is not an escape, so the text after
/// the closing delimiter starts the next value
/// (xmlports/TestXmlPortTextFormatsContracts.al:VariableText_Import_DoubledQuoteInsideAQuotedValue_IsNotAnEscape).
/// Each value comes with whether it stood in the delimiter.
pub(super) fn split_variable_marked(line: &str, sep: &str, delim: &str) -> Vec<(String, bool)> {
    if delim.is_empty() {
        return line.split(sep).map(|v| (v.to_string(), false)).collect();
    }
    let mut out = Vec::new();
    let mut rest = line;
    loop {
        if let Some(after) = rest.strip_prefix(delim) {
            match after.find(delim) {
                Some(end) => {
                    out.push((after[..end].to_string(), true));
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
                    out.push((after.to_string(), false));
                    return out;
                }
            }
        } else {
            match rest.find(sep) {
                Some(end) => {
                    out.push((rest[..end].to_string(), false));
                    rest = &rest[end + sep.len()..];
                }
                None => {
                    out.push((rest.to_string(), false));
                    return out;
                }
            }
        }
    }
}

/// Reads a VariableText document separator by separator when the field separator and the record separator overlap (the field
/// separator `<NewLine>` with the record separator `<LF>`): at each place the field separator is tried before the record separator
/// (xmlports/shapes-properties/Y83XpMiscTests.al:VariableText_NewLineSeparator `roundtrip newline`, `newline doc crlf`, needs
/// the corpus bump: `7<CR/LF>seven<CR/LF>7.25<CR/LF>` is one record). A value that starts with the delimiter runs to the next one.
pub(super) fn split_overlapping(text: &str, fsep: &str, rsep: &str, delim: &str) -> Vec<Vec<String>> {
    let mut records: Vec<Vec<String>> = Vec::new();
    let (mut record, mut value) = (Vec::<String>::new(), String::new());
    let mut rest = text;
    while !rest.is_empty() {
        if value.is_empty() && !delim.is_empty() && rest.starts_with(delim) {
            let after = &rest[delim.len()..];
            match after.find(delim) {
                Some(end) => {
                    value.push_str(&after[..end]);
                    rest = &after[end + delim.len()..];
                    continue;
                }
                None => {
                    value.push_str(after);
                    break;
                }
            }
        }
        if let Some(r) = rest.strip_prefix(fsep) {
            record.push(std::mem::take(&mut value));
            rest = r;
        } else if let Some(r) = rest.strip_prefix(rsep) {
            record.push(std::mem::take(&mut value));
            records.push(std::mem::take(&mut record));
            rest = r;
        } else {
            let c = rest.chars().next().expect("not empty");
            value.push(c);
            rest = &rest[c.len_utf8()..];
        }
    }
    if !value.is_empty() || !record.is_empty() {
        record.push(value);
        records.push(record);
    }
    records
}

/// Whether `kids` are two table elements of which the second names the first in LinkTable (the second is the first's following
/// sibling, R6924eLinkSibling.al xmlports 95355 VariableText and 95356 FixedText).
fn linked_pair(schema: &Schema, kids: &[usize]) -> bool {
    let [first, second] = kids else { return false };
    schema.node(*second).link_node == Some(*first) && super::pass::link_sibling(schema, *second)
}
