//! The document side of XmlPort import and export (Format = Xml): a plain element tree, a
//! reader built on the XML DOM parser, a writer, and the text of field values.
//!
//! What the corpus pins is the content of the document (element and attribute names, values,
//! nesting). Its whitespace, XML declaration and line endings are not pinned by any test; the
//! choices below are listed in docs/probes-needed.md.

use crate::describe::Describe;
use crate::value::{RtType, Value};
use rust_decimal::Decimal;

#[derive(Debug, Clone, PartialEq)]
pub struct Elem {
    pub name: String,
    pub attrs: Vec<(String, String)>,
    pub children: Vec<Node>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Elem(Elem),
    Text(String),
}

impl Elem {
    pub fn new(name: &str) -> Elem {
        Elem {
            name: name.to_string(),
            attrs: Vec::new(),
            children: Vec::new(),
        }
    }

    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs.iter().find(|(n, _)| n == name).map(|(_, v)| v.as_str())
    }

    /// The text directly inside the element (text and CDATA nodes, concatenated).
    pub fn text(&self) -> String {
        self.children
            .iter()
            .filter_map(|c| match c {
                Node::Text(t) => Some(t.as_str()),
                Node::Elem(_) => None,
            })
            .collect()
    }

    pub fn elements(&self) -> impl Iterator<Item = &Elem> {
        self.children.iter().filter_map(|c| match c {
            Node::Elem(e) => Some(e),
            Node::Text(_) => None,
        })
    }
}

/// Why a document could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadFailure {
    /// Not well-formed XML, with the .NET exception message where the runtime knows it.
    Malformed(Option<String>),
    /// Well-formed, but uses something the runtime does not read yet.
    Unsupported(String),
}

/// Reads the root element of `text`.
pub fn parse(text: &str) -> Result<Elem, ReadFailure> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    match crate::xml::read_tree(text) {
        Ok(Ok(e)) => Ok(e),
        Ok(Err(message)) => Err(ReadFailure::Malformed(message)),
        Err(why) => Err(ReadFailure::Unsupported(why)),
    }
}

/// The document text as an export writes it: the declaration, a carriage return and a line feed, then the root with
/// two spaces of indentation per level. An element without content is `<X />`; an element with
/// text only is on one line (xmlport/TestXmlPortExportImportContracts.al:XmlPortExport_Document_FullText reads it line by line, which drops the CR; the lengths in xmlport/lt-xmlport-query-datatransfer/XpTExportBasic.al count it).
pub fn serialize(root: &Elem) -> String {
    serialize_as(root, "UTF-16")
}

/// [`serialize`] with the encoding named in the declaration.
pub fn serialize_as(root: &Elem, encoding: &str) -> String {
    let mut out = format!("<?xml version=\"1.0\" encoding=\"{encoding}\" standalone=\"no\"?>\r\n");
    write_elem(root, 0, &mut out);
    out
}

/// A document of several root elements one after the other, as an xmlport whose root is a table element writes one root
/// element per row (corpus platform/lt-ag-contoso/AgXmlPortImport.al:584 QX_RootTableElement_Export_TwoRows).
pub fn serialize_roots(roots: &[Elem], encoding: &str) -> String {
    let mut out = format!("<?xml version=\"1.0\" encoding=\"{encoding}\" standalone=\"no\"?>\r\n");
    for (i, r) in roots.iter().enumerate() {
        if i > 0 {
            out.push_str("\r\n");
        }
        write_elem(r, 0, &mut out);
    }
    out
}

/// An element on one line with no indentation: `<Item><Id>1</Id></Item>`. `None` when an element has no text (how an
/// empty value is written without indentation is not pinned) or holds attributes.
pub fn compact(e: &Elem) -> Option<String> {
    fn go(e: &Elem, out: &mut String) -> Option<()> {
        if !e.attrs.is_empty() || e.children.is_empty() {
            return None;
        }
        out.push('<');
        out.push_str(&e.name);
        out.push('>');
        for c in &e.children {
            match c {
                Node::Elem(c) => go(c, out)?,
                Node::Text(t) if t.is_empty() => return None,
                Node::Text(t) => crate::xml::escape_text(t, out),
            }
        }
        out.push_str("</");
        out.push_str(&e.name);
        out.push('>');
        Some(())
    }
    let mut out = String::new();
    go(e, &mut out)?;
    Some(out)
}

fn write_elem(e: &Elem, level: usize, out: &mut String) {
    for _ in 0..level {
        out.push_str("  ");
    }
    out.push('<');
    out.push_str(&e.name);
    for (n, v) in &e.attrs {
        out.push(' ');
        out.push_str(n);
        out.push_str("=\"");
        crate::xml::escape_attr(v, out);
        out.push('"');
    }
    if e.children.iter().all(|c| matches!(c, Node::Text(t) if t.is_empty())) {
        out.push_str(" />");
        return;
    }
    out.push('>');
    // Text next to elements (a field element's value before the nodes below it) puts the whole element on one line, without
    // indentation inside it (xmlport/lt-af-xmlport/AfXmlPortAttrChildren.al:285, 313: `<Name>Alpha<Child>c1</Child></Name>`).
    let mixed = e.children.iter().any(|c| matches!(c, Node::Text(t) if !t.is_empty()))
        && e.children.iter().any(|c| matches!(c, Node::Elem(_)));
    if mixed {
        for c in &e.children {
            match c {
                Node::Elem(c) => write_elem(c, 0, out),
                Node::Text(t) => crate::xml::escape_text(t, out),
            }
        }
    } else if e.children.iter().any(|c| matches!(c, Node::Elem(_))) {
        out.push_str("\r\n");
        for c in &e.children {
            match c {
                Node::Elem(c) => {
                    write_elem(c, level + 1, out);
                    out.push_str("\r\n");
                }
                Node::Text(t) => crate::xml::escape_text(t, out),
            }
        }
        for _ in 0..level {
            out.push_str("  ");
        }
    } else {
        for c in &e.children {
            if let Node::Text(t) = c {
                crate::xml::escape_text(t, out);
            }
        }
    }
    out.push_str("</");
    out.push_str(&e.name);
    out.push('>');
}

/// The bytes of an exported document: UTF-16 little endian after a byte order mark.
pub fn utf16_bom_bytes(text: &str) -> Vec<u8> {
    let mut out = vec![0xFF, 0xFE];
    out.extend(text.encode_utf16().flat_map(u16::to_le_bytes));
    out
}

/// The text an exported field value is written as. `Err` names a type whose text is not pinned.
///
/// Pinned: Integer as its digits (xmlport/TestXmlPortObject.al:`<EntryNo>1</EntryNo>`), Text
/// and Code as they are (`<TextValue>Second</TextValue>`), Decimal with two decimal places
/// (xmlport/TestXmlPortAdvanced.al:`Amount="15.00"`; only values that need no rounding to two
/// places are written).
pub fn field_text(v: &Value) -> Result<String, String> {
    match v {
        Value::Integer(i) => Ok(i.to_string()),
        Value::Text(s) | Value::Code(s) => Ok(s.to_string()),
        // Two places and a comma between thousands: `1,234.50`, `-0.50`
        // (xmlport/TestXmlPortExportImportContracts.al:XmlPortExport_Document_FullText).
        Value::Decimal(d) if d.normalize().scale() <= 2 => {
            let text = format!("{:.2}", d.round_dp(2));
            let (sign, digits) = text.strip_prefix('-').map_or(("", text.as_str()), |t| ("-", t));
            let (int, frac) = digits.split_once('.').unwrap_or((digits, ""));
            let mut grouped = String::new();
            for (i, c) in int.chars().enumerate() {
                if i > 0 && (int.len() - i) % 3 == 0 {
                    grouped.push(',');
                }
                grouped.push(c);
            }
            Ok(format!("{sign}{grouped}.{frac}"))
        }
        Value::Decimal(d) => Err(format!("the Decimal {d}, which needs rounding to two places")),
        other => Err(format!("a value of type {}", type_name(other))),
    }
}

/// The value an imported text becomes in a field of type `ty`.
pub fn field_value(text: &str, ty: &RtType) -> Result<Value, FieldFailure> {
    match ty {
        RtType::Integer => text
            .parse::<i32>()
            .map(Value::Integer)
            .map_err(|_| FieldFailure::Invalid(format!("'{text}' is not an Integer"))),
        RtType::Decimal => text
            .parse::<Decimal>()
            .map(Value::Decimal)
            .map_err(|_| FieldFailure::Invalid(format!("'{text}' is not a Decimal"))),
        RtType::Text(_) | RtType::Code(_) => crate::ops::convert(Value::Text(text.to_string().into()), ty)
            .map_err(|e| FieldFailure::Unsupported(e.describe())),
        other => Err(FieldFailure::Unsupported(format!(
            "a field of type {}",
            other.describe()
        ))),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldFailure {
    /// The text is not a value of the field's type.
    Invalid(String),
    /// Assigning to this field type is not pinned.
    Unsupported(String),
}

fn type_name(v: &Value) -> &'static str {
    match v {
        Value::Boolean(_) => "Boolean",
        Value::BigInteger(_) => "BigInteger",
        Value::Date(..) => "Date",
        Value::Time(_) => "Time",
        Value::DateTime(_) => "DateTime",
        Value::Duration(_) => "Duration",
        Value::Guid(_) => "Guid",
        Value::Option(..) => "Option",
        Value::Enum(..) => "Enum",
        Value::Blob(_) => "Blob",
        _ => "this type",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_document_is_written_indented_with_a_declaration() {
        let mut row = Elem::new("U");
        row.attrs.push(("Attr".into(), "a\"b&<c>".into()));
        let mut id = Elem::new("EntryNo");
        id.children.push(Node::Text("1".into()));
        row.children.push(Node::Elem(id));
        row.children.push(Node::Elem(Elem::new("Txt")));
        let mut root = Elem::new("Root");
        root.children.push(Node::Elem(row));
        // xmlport/TestXmlPortExportImportContracts.al:XmlPortExport_Document_FullText
        assert_eq!(
            serialize(&root),
            "<?xml version=\"1.0\" encoding=\"UTF-16\" standalone=\"no\"?>\r\n<Root>\r\n  <U Attr=\"a&quot;b&amp;&lt;c&gt;\">\r\n    <EntryNo>1</EntryNo>\r\n    <Txt />\r\n  </U>\r\n</Root>"
        );
    }

    #[test]
    fn the_bytes_start_with_a_byte_order_mark() {
        assert_eq!(utf16_bom_bytes("<a"), vec![0xFF, 0xFE, b'<', 0, b'a', 0]);
    }

    #[test]
    fn malformed_and_namespaced_documents_are_told_apart() {
        assert!(matches!(parse("<a><b></a>"), Err(ReadFailure::Malformed(Some(_)))));
        // An element that inherits a default namespace is not carried over; the element that declares it keeps the declaration
        // as its `xmlns` attribute (the port names or skips it, YXmlPortProps.al:UseLax_Import_AcceptsUnknownElements).
        assert!(matches!(
            parse("<a xmlns=\"u\"><b/></a>"),
            Err(ReadFailure::Unsupported(_))
        ));
        assert!(matches!(parse("<a xmlns=\"u\"/>"), Ok(e) if e.attr("xmlns") == Some("u")));
    }

    #[test]
    fn field_values_use_the_pinned_text_forms() {
        assert_eq!(field_text(&Value::Integer(7)).unwrap(), "7");
        assert_eq!(field_text(&Value::Decimal(Decimal::from(15))).unwrap(), "15.00");
        assert_eq!(field_text(&Value::Decimal("2.5".parse().unwrap())).unwrap(), "2.50");
        assert_eq!(
            field_text(&Value::Decimal("1234.5".parse().unwrap())).unwrap(),
            "1,234.50"
        );
        assert_eq!(field_text(&Value::Decimal("-0.5".parse().unwrap())).unwrap(), "-0.50");
        assert!(field_text(&Value::Decimal("1.234".parse().unwrap())).is_err());
        assert!(field_text(&Value::Boolean(true)).is_err());
        assert!(matches!(field_value("15.00", &RtType::Decimal), Ok(Value::Decimal(d)) if d == Decimal::from(15)));
        assert!(matches!(
            field_value("x", &RtType::Integer),
            Err(FieldFailure::Invalid(_))
        ));
    }
    #[test]
    fn attr_finds_an_attribute_by_exact_name() {
        let mut e = Elem::new("X");
        e.attrs.push(("a".into(), "1".into()));
        e.attrs.push(("b".into(), "2".into()));
        assert_eq!(e.attr("a"), Some("1"));
        assert_eq!(e.attr("b"), Some("2"));
        assert_eq!(e.attr("c"), None);
        assert_eq!(e.attr("A"), None);
    }

    #[test]
    fn an_unpinned_field_type_is_refused_by_naming_the_type() {
        for (v, name) in [
            (Value::Boolean(true), "Boolean"),
            (Value::BigInteger(1), "BigInteger"),
            (Value::Date(0, false), "Date"),
            (Value::Time(0), "Time"),
            (Value::DateTime(0), "DateTime"),
            (Value::Duration(0), "Duration"),
            (Value::Guid([0; 16]), "Guid"),
            (Value::Option(0, Vec::<String>::new().into()), "Option"),
            (Value::Enum(1, 0), "Enum"),
            (Value::Blob(crate::stream::new_blob()), "Blob"),
            (Value::Null, "this type"),
        ] {
            assert_eq!(field_text(&v), Err(format!("a value of type {name}")), "{name}");
        }
    }
}
