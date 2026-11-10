//! The bytes `Query.SaveAsXml` and `Query.SaveAsJson` write for the rows of a query.
//!
//! SaveAsXml writes a .NET DataSet: a UTF-8 byte order mark, then lines ended by CR LF and indented by two spaces per
//! level, a schema with one element per column and a `Result` element per row (queries/read-lifecycle/QuerySaveAs.al:116-120,
//! 1330 bytes, first byte 239, 34 CR and 34 LF; reports/request-page/P105QryXmlContentSM.al:53-54, the whole text, the
//! three byte order mark characters read apart). The query's locale is empty (`msdata:Locale=""`, same line).
//!
//! SaveAsJson writes one line without a byte order mark: an array of objects keyed by column name
//! (QuerySaveAs.al:150-155).
//!
//! The column types pinned by those files: Integer (`xs:int`, a JSON number), Text (`xs:string`, `<Name />` and `""` when
//! empty, `<>&` escaped in XML and not in JSON, a quote escaped in JSON only), Decimal (`xs:decimal`, the value as it is:
//! `12.5`, `0`, `-5`, `30.25`), Date (`xs:dateTime` with `msdata:DateTimeMode="Local"`, `2024-03-05T00:00:00+00:00` in
//! XML and `"2024-03-05T00:00:00Z"` in JSON, left out of both when blank) and Boolean (`xs:boolean`, `true` and `false`).
//! A Code column is `xs:string` like a Code column of a report data file (data_file_types.rs `declared`); its values are
//! pinned by streams/ChQueryLeads.al:350-389 (`<Grp>B</Grp>`). Every other type, a closing date, and a text with a
//! control character or (in JSON) a backslash are refused: how .NET writes them there is not pinned.

use al_builtins::native::{Signal, unsupported};
use al_builtins::value::{RtType, Value};

/// The type tag of a query column of type `ty`, or a refusal naming the column.
fn column_type(method: &str, name: &str, ty: &RtType) -> Result<&'static str, Signal> {
    Ok(match ty {
        RtType::Integer => "xs:int",
        RtType::Text(_) | RtType::Code(_) => "xs:string",
        RtType::Decimal => "xs:decimal",
        RtType::Date => "xs:dateTime",
        RtType::Boolean => "xs:boolean",
        _ => {
            return unsupported(format!(
                "{method} of the column {name} of type {} (not pinned)",
                al_builtins::describe::Describe::describe(ty)
            ));
        }
    })
}

/// What a cell writes: nothing (a blank date), or its text (`quoted` for a JSON string).
enum Cell {
    Omit,
    Text { text: String, quoted: bool },
}

fn cell(method: &str, name: &str, value: &Value, json: bool) -> Result<Cell, Signal> {
    let plain = |text: String| Ok(Cell::Text { text, quoted: false });
    match value {
        Value::Integer(i) => plain(i.to_string()),
        Value::Decimal(d) => plain(d.to_string()),
        Value::Boolean(b) => plain(b.to_string()),
        Value::Date(0, false) => Ok(Cell::Omit),
        Value::Date(d, false) => {
            let (y, m, day) = al_builtins::format::civil_from_day(*d);
            let text = if json {
                format!("{y:04}-{m:02}-{day:02}T00:00:00Z")
            } else {
                format!("{y:04}-{m:02}-{day:02}T00:00:00+00:00")
            };
            Ok(Cell::Text { text, quoted: json })
        }
        Value::Text(s) | Value::TextConst(s) | Value::Code(s) => {
            let text = if json {
                json_escape(method, name, s)?
            } else {
                crate::dataset::escape(s)?
            };
            Ok(Cell::Text { text, quoted: json })
        }
        other => unsupported(format!(
            "{method} of the column {name} holding {} (not pinned)",
            al_builtins::describe::Describe::describe(other)
        )),
    }
}

fn json_escape(method: &str, name: &str, s: &str) -> Result<String, Signal> {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => {
                return unsupported(format!(
                    "{method} of a backslash in column {name} (its escaping is not pinned)"
                ));
            }
            c if c.is_control() => {
                return unsupported(format!(
                    "{method} of the control character U+{:04X} in column {name} (its escaping is not pinned)",
                    u32::from(c)
                ));
            }
            c => out.push(c),
        }
    }
    Ok(out)
}

/// The bytes of the file: `columns` are the names and types of the columns written, `rows` the values of those columns.
pub(crate) fn render(
    method: &str,
    columns: &[(&str, &RtType)],
    rows: &[Vec<&Value>],
    json: bool,
) -> Result<Vec<u8>, Signal> {
    let types = columns
        .iter()
        .map(|(name, ty)| column_type(method, name, ty))
        .collect::<Result<Vec<_>, _>>()?;
    let mut cells: Vec<Vec<Cell>> = Vec::with_capacity(rows.len());
    for row in rows {
        let mut out = Vec::with_capacity(columns.len());
        for (k, value) in row.iter().enumerate() {
            out.push(cell(method, columns[k].0, value, json)?);
        }
        cells.push(out);
    }
    if json {
        Ok(render_json(columns, &cells).into_bytes())
    } else {
        Ok(render_xml(columns, &types, &cells))
    }
}

fn render_json(columns: &[(&str, &RtType)], rows: &[Vec<Cell>]) -> String {
    let mut out = String::from("[");
    for (i, row) in rows.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push('{');
        let mut first = true;
        for (k, c) in row.iter().enumerate() {
            let Cell::Text { text, quoted } = c else { continue };
            if !first {
                out.push(',');
            }
            first = false;
            let name = columns[k].0;
            if *quoted {
                out.push_str(&format!("\"{name}\":\"{text}\""));
            } else {
                out.push_str(&format!("\"{name}\":{text}"));
            }
        }
        out.push('}');
    }
    out.push(']');
    out
}

fn render_xml(columns: &[(&str, &RtType)], types: &[&'static str], rows: &[Vec<Cell>]) -> Vec<u8> {
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" standalone=\"yes\"?>\r\n<DataSet>\r\n");
    out.push_str("  <xs:schema id=\"NewDataSet\" xmlns=\"\" xmlns:xs=\"http://www.w3.org/2001/XMLSchema\" xmlns:msdata=\"urn:schemas-microsoft-com:xml-msdata\">\r\n");
    out.push_str("    <xs:element name=\"NewDataSet\" msdata:IsDataSet=\"true\" msdata:MainDataTable=\"Result\" msdata:Locale=\"\">\r\n");
    out.push_str("      <xs:complexType>\r\n        <xs:choice minOccurs=\"0\" maxOccurs=\"unbounded\">\r\n");
    out.push_str("          <xs:element name=\"Result\" msdata:Locale=\"\">\r\n            <xs:complexType>\r\n              <xs:sequence>\r\n");
    for ((name, _), ty) in columns.iter().zip(types) {
        let attributes = crate::data_file_types::element_attributes(ty);
        out.push_str(&format!(
            "                <xs:element name=\"{name}\" {attributes} minOccurs=\"0\" />\r\n"
        ));
    }
    out.push_str("              </xs:sequence>\r\n            </xs:complexType>\r\n          </xs:element>\r\n");
    out.push_str("        </xs:choice>\r\n      </xs:complexType>\r\n    </xs:element>\r\n  </xs:schema>\r\n");
    for row in rows {
        out.push_str("  <Result>\r\n");
        for (k, c) in row.iter().enumerate() {
            let Cell::Text { text, .. } = c else { continue };
            let name = columns[k].0;
            if text.is_empty() {
                out.push_str(&format!("    <{name} />\r\n"));
            } else {
                out.push_str(&format!("    <{name}>{text}</{name}>\r\n"));
            }
        }
        out.push_str("  </Result>\r\n");
    }
    // No line end after the last line: 34 CR LF for 35 lines (QuerySaveAs.al:118-120).
    out.push_str("</DataSet>");
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(out.as_bytes());
    bytes
}
