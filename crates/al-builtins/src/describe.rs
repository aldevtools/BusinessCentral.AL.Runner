//! Readable descriptions of values and types for refusal reasons and internal errors.
//!
//! A refusal names what it refused in AL terms: the type, and for a simple value a short form of
//! it. Rust's debug format shows internal structure instead (`Record(RefCell { .. })`).

use crate::json::Scalar;
use crate::native::Signal;
use crate::ops::Arith;
use crate::value::{DotNetKind, LibraryValue, RtType, Value};

/// How a value or type reads in a message a user sees.
pub trait Describe {
    fn describe(&self) -> String;
}

/// Text longer than this is cut, so a reason stays on one line.
const TEXT_SHOWN: usize = 30;

fn quoted(s: &str) -> String {
    if s.chars().count() > TEXT_SHOWN {
        format!("'{}...'", s.chars().take(TEXT_SHOWN).collect::<String>())
    } else {
        format!("'{s}'")
    }
}

impl Describe for Value {
    fn describe(&self) -> String {
        match self {
            Value::Null => "an unassigned Variant".into(),
            Value::Unknown(why) => format!("an unknown value ({why})"),
            Value::SqlCount(_) => "a SQL counter value".into(),
            Value::Boolean(b) => format!("Boolean {b}"),
            Value::Integer(i) => format!("Integer {i}"),
            Value::BigInteger(i) => format!("BigInteger {i}"),
            Value::Decimal(d) => format!("Decimal {d}"),
            Value::Byte(b) => format!("Byte {b}"),
            Value::Char(c) => match char::from_u32(u32::from(*c)) {
                Some(ch) if !ch.is_control() => format!("Char '{ch}'"),
                _ => format!("Char {c}"),
            },
            Value::Text(s) => format!("Text {}", quoted(s)),
            Value::Code(s) => format!("Code {}", quoted(s)),
            Value::Date(0, _) => "an undefined Date".into(),
            Value::Date(_, true) => "a closing Date".into(),
            Value::Date(n, false) => format!("Date (day {n})"),
            Value::Time(0) => "an undefined Time".into(),
            Value::Time(_) => "Time".into(),
            Value::DateTime(0) => "an undefined DateTime".into(),
            Value::DateTime(_) => "DateTime".into(),
            Value::Duration(ms) => format!("Duration ({ms} ms)"),
            Value::Guid(_) => "Guid".into(),
            Value::DateFormula(s) => format!("DateFormula {}", quoted(s)),
            Value::Option(ord, names) => match usize::try_from(*ord).ok().and_then(|i| names.get(i)) {
                Some(name) => format!("Option member {}", quoted(name)),
                None => format!("Option member {ord}"),
            },
            Value::Enum(_, ord) => format!("Enum value {ord}"),
            Value::List(l) => match l.try_borrow() {
                Ok(l) => format!("List of {} elements", l.len()),
                Err(_) => "List".into(),
            },
            Value::Dictionary(d) => match d.try_borrow() {
                Ok(d) => format!(
                    "Dictionary of {} entries",
                    d.iter().filter(|(k, _)| !matches!(k, Value::Null)).count()
                ),
                Err(_) => "Dictionary".into(),
            },
            Value::Array(a) => match a.try_borrow() {
                Ok(a) => format!("array of {} elements", a.len()),
                Err(_) => "array".into(),
            },
            Value::Codeunit(_) => "Codeunit instance".into(),
            Value::Library(name, _) => name.to_string(),
            Value::Record(r) => {
                let (table, temporary) = match r.try_borrow() {
                    Ok(r) => (Some(r.table), r.temporary),
                    Err(_) => (None, false),
                };
                let temp = if temporary { "temporary " } else { "" };
                match table {
                    Some(t) => format!("{temp}Record of table object {t}"),
                    None => "Record".into(),
                }
            }
            Value::Blob(_) => "Blob".into(),
            Value::TestPage(_) => "TestPage".into(),
            Value::TestControl(_) => "TestPage control".into(),
            Value::Query(_) => "Query".into(),
            Value::DotNet(o) => match &o.kind {
                DotNetKind::Host { clr_type, .. } => format!("DotNet object of type {clr_type}"),
                DotNetKind::Enum { clr_type, name, .. } => format!("DotNet enum value {clr_type}.{name}"),
                DotNetKind::Internal { type_name, .. } => format!("DotNet object of type {type_name}"),
            },
        }
    }
}

impl Describe for RtType {
    fn describe(&self) -> String {
        let sized = |name: &str, n: &Option<u32>| match n {
            Some(n) => format!("{name}[{n}]"),
            None => name.to_string(),
        };
        match self {
            RtType::Boolean => "Boolean".into(),
            RtType::Integer => "Integer".into(),
            RtType::BigInteger => "BigInteger".into(),
            RtType::Decimal => "Decimal".into(),
            RtType::Byte => "Byte".into(),
            RtType::Char => "Char".into(),
            RtType::Text(n) => sized("Text", n),
            RtType::Code(n) => sized("Code", n),
            RtType::Date => "Date".into(),
            RtType::Time => "Time".into(),
            RtType::DateTime => "DateTime".into(),
            RtType::Duration => "Duration".into(),
            RtType::Guid => "Guid".into(),
            RtType::DateFormula => "DateFormula".into(),
            RtType::Option(_) => "Option".into(),
            RtType::Enum(_) => "Enum".into(),
            RtType::Variant => "Variant".into(),
            RtType::List(t) => format!("List of [{}]", t.describe()),
            RtType::Dictionary(k, v) => format!("Dictionary of [{}, {}]", k.describe(), v.describe()),
            RtType::Array(_, t) => format!("array of {}", t.describe()),
            RtType::Record(_, true) => "temporary Record".into(),
            RtType::Record(_, false) => "Record".into(),
            RtType::Codeunit(_) => "Codeunit".into(),
            RtType::TestPage(_) => "TestPage".into(),
            RtType::Page(_) => "Page".into(),
            RtType::Query(_) => "Query".into(),
            RtType::Interface(_) => "Interface".into(),
            RtType::Library(name) => name.to_string(),
            RtType::DotNet(_) => "DotNet".into(),
        }
    }
}

impl Describe for LibraryValue {
    fn describe(&self) -> String {
        match self {
            LibraryValue::Empty => "an untouched handle".into(),
            LibraryValue::Text(_) | LibraryValue::TextCap(..) => "a text handle".into(),
            LibraryValue::RecordRef(_) => "RecordRef".into(),
            LibraryValue::FieldRef(..) | LibraryValue::KeyFieldRef(..) => "FieldRef".into(),
            LibraryValue::KeyRef(..) => "KeyRef".into(),
            LibraryValue::SystemTable(n) => format!("RecordRef on system table {n}"),
            LibraryValue::RecordId { .. } => "RecordId".into(),
            LibraryValue::WebServiceAction(_) => "WebServiceActionContext".into(),
            LibraryValue::SessionSettings(_) => "SessionSettings".into(),
            LibraryValue::FilterPageBuilder(_) => "FilterPageBuilder".into(),
            LibraryValue::Notification(_) => "Notification".into(),
            LibraryValue::ModuleInfo(_) => "ModuleInfo".into(),
            LibraryValue::DataTransfer(_) => "DataTransfer".into(),
            LibraryValue::Cookie(_) => "Cookie".into(),
            LibraryValue::HttpRequest(_) => "HttpRequestMessage".into(),
            LibraryValue::HttpContent(_) => "HttpContent".into(),
            LibraryValue::HttpHeaders(_) => "HttpHeaders".into(),
            LibraryValue::HttpResponse(_) => "HttpResponseMessage".into(),
            LibraryValue::HttpClient(_) => "HttpClient".into(),
            LibraryValue::TestHttpRequest(_) => "TestHttpRequestMessage".into(),
            LibraryValue::TestHttpResponse(_) => "TestHttpResponseMessage".into(),
            LibraryValue::Version(_) => "Version".into(),
            LibraryValue::ErrorInfo(_) => "ErrorInfo".into(),
            LibraryValue::Json(_) => "a JSON node".into(),
            LibraryValue::Xml(_) => "an XML node".into(),
            LibraryValue::XmlList(_) => "XmlNodeList".into(),
            LibraryValue::XmlAttrs(_) => "XmlAttributeCollection".into(),
            LibraryValue::XmlNameTable => "XmlNameTable".into(),
            LibraryValue::XmlReadOptions { .. } => "XmlReadOptions".into(),
            LibraryValue::XmlWriteOptions { .. } => "XmlWriteOptions".into(),
            LibraryValue::File(_) => "File".into(),
            LibraryValue::XmlNamespaces(_) => "XmlNamespaceManager".into(),
            LibraryValue::InStream(_) => "InStream".into(),
            LibraryValue::OutStream(..) => "OutStream".into(),
        }
    }
}

impl Describe for Scalar {
    fn describe(&self) -> String {
        match self {
            Scalar::Null => "a JSON null".into(),
            Scalar::Undefined => "an undefined JSON value".into(),
            Scalar::Raw(t) => format!("the JSON number {t}"),
            Scalar::Val(v) => v.describe(),
        }
    }
}

impl Describe for Arith {
    fn describe(&self) -> String {
        match self {
            Arith::Add => "addition",
            Arith::Sub => "subtraction",
            Arith::Mul => "multiplication",
            Arith::Div => "division",
            Arith::IntDiv => "integer division (div)",
            Arith::Mod => "modulo (mod)",
        }
        .into()
    }
}

/// An AL error reads as its message; the other signals as what they are.
impl Describe for Signal {
    fn describe(&self) -> String {
        match self {
            Signal::Error(e) => e.message.clone(),
            Signal::Exit(_) => "an Exit".into(),
            Signal::Break => "a Break".into(),
            Signal::Continue => "a Continue".into(),
            Signal::Unsupported(why) => why.clone(),
        }
    }
}

macro_rules! describe_by_display {
    ($($t:ty),*) => {$(
        impl Describe for $t {
            fn describe(&self) -> String {
                self.to_string()
            }
        }
    )*};
}
describe_by_display!(bool, i32, i64, u32, u64, usize, str, String, rust_decimal::Decimal);

impl<T: Describe + ?Sized> Describe for &T {
    fn describe(&self) -> String {
        (**self).describe()
    }
}

impl<T: Describe + ?Sized> Describe for &mut T {
    fn describe(&self) -> String {
        (**self).describe()
    }
}

/// `None` is the missing argument of a call such as `args.first()`.
impl<T: Describe> Describe for Option<T> {
    fn describe(&self) -> String {
        match self {
            Some(v) => v.describe(),
            None => "no value".into(),
        }
    }
}

/// A list of arguments, as `(a, b)`.
impl<T: Describe> Describe for [T] {
    fn describe(&self) -> String {
        let parts: Vec<String> = self.iter().map(Describe::describe).collect();
        format!("({})", parts.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::{LibraryValue, RecordState};
    use std::cell::RefCell;
    use std::rc::Rc;

    fn record(table: u32, temporary: bool) -> Value {
        Value::Record(Rc::new(RefCell::new(RecordState {
            table,
            temporary,
            temp_id: None,
            company: None,
            crm: false,
            fields: Vec::new(),
            filters: Vec::new(),
            filter_group: 0,
            marks: Vec::new(),
            marked_only: false,
            read_isolation: 0,
            security_filtering: 0,
            key: 0,
            adhoc: None,
            ascending: true,
            field_descending: 0,
            position: None,
            auto_calc: Vec::new(),
            load: Default::default(),
            touched: false,
            assigned: false,
            inserted: false,
            dirty_base: None,
            is_dirty_base: Default::default(),
            last_modify: None,
            globals: Default::default(),
            unpinned_buffer: None,
        })))
    }

    #[test]
    fn simple_values_name_their_type_and_value() {
        assert_eq!(Value::Boolean(true).describe(), "Boolean true");
        assert_eq!(Value::Integer(5).describe(), "Integer 5");
        assert_eq!(Value::BigInteger(-7).describe(), "BigInteger -7");
        assert_eq!(Value::Text("abc".into()).describe(), "Text 'abc'");
        assert_eq!(Value::Code("X1".into()).describe(), "Code 'X1'");
        assert_eq!(Value::Null.describe(), "an unassigned Variant");
    }

    #[test]
    fn long_text_is_cut() {
        let d = Value::Text("x".repeat(100).into()).describe();
        assert!(d.starts_with("Text 'xxxx"), "{d}");
        assert!(d.len() < 60, "{d}");
        assert!(d.ends_with("...'"), "{d}");
    }

    #[test]
    fn dates_say_whether_they_are_defined() {
        assert_eq!(Value::Date(0, false).describe(), "an undefined Date");
        assert_eq!(Value::Date(10, true).describe(), "a closing Date");
        assert_eq!(Value::Date(10, false).describe(), "Date (day 10)");
        assert_eq!(Value::Time(0).describe(), "an undefined Time");
        assert_eq!(Value::DateTime(0).describe(), "an undefined DateTime");
    }

    #[test]
    fn option_names_its_member() {
        let members: std::sync::Arc<[String]> = std::sync::Arc::from(vec!["A".to_string(), "B".to_string()]);
        assert_eq!(Value::Option(1, members.clone()).describe(), "Option member 'B'");
        assert_eq!(Value::Option(9, members).describe(), "Option member 9");
    }

    #[test]
    fn reference_values_do_not_dump_their_contents() {
        let list = Value::List(Rc::new(RefCell::new(vec![Value::Integer(1), Value::Integer(2)].into())));
        assert_eq!(list.describe(), "List of 2 elements");
        assert_eq!(Value::Dictionary(Rc::default()).describe(), "Dictionary of 0 entries");
        assert_eq!(record(7, false).describe(), "Record of table object 7");
        assert_eq!(record(7, true).describe(), "temporary Record of table object 7");
        let json = Value::Library("JsonObject".into(), Rc::new(RefCell::new(LibraryValue::Empty)));
        assert_eq!(json.describe(), "JsonObject");
        for v in [list, record(1, false), json] {
            let d = v.describe();
            assert!(!d.contains("RefCell") && !d.contains("Rc(") && !d.contains('{'), "{d}");
        }
    }

    #[test]
    fn a_borrowed_record_still_describes() {
        let r = record(3, false);
        let Value::Record(rc) = &r else { unreachable!() };
        let _held = rc.borrow_mut();
        assert_eq!(r.describe(), "Record");
    }

    #[test]
    fn missing_and_several_arguments() {
        let none: Option<&Value> = None;
        assert_eq!(none.describe(), "no value");
        assert_eq!(Some(&Value::Integer(1)).describe(), "Integer 1");
        let args = [Value::Integer(1), Value::Text("a".into())];
        assert_eq!(args.describe(), "(Integer 1, Text 'a')");
        assert_eq!(args.first().describe(), "Integer 1");
    }

    #[test]
    fn json_scalars_arithmetic_and_signals_read_plainly() {
        assert_eq!(Scalar::Null.describe(), "a JSON null");
        assert_eq!(Scalar::Val(Value::Integer(2)).describe(), "Integer 2");
        assert_eq!(Arith::IntDiv.describe(), "integer division (div)");
        assert_eq!(Signal::Break.describe(), "a Break");
        assert_eq!(Signal::Unsupported("why".into()).describe(), "why");
    }

    #[test]
    fn typical_refusals_name_the_types() {
        let why = |r: Result<Value, Signal>| match r {
            Err(Signal::Unsupported(m)) => m,
            other => panic!(
                "expected a refusal, got {}",
                other.map_or_else(|e| e.describe(), |v| v.describe())
            ),
        };
        let m = why(crate::ops::arith(
            Arith::Add,
            &record(7, false),
            &Value::Text("a".into()),
        ));
        assert_eq!(m, "addition of Record of table object 7 and Text 'a'");
        let m = why(crate::ops::convert(record(2, true), &RtType::Integer));
        assert!(
            m.contains("temporary Record of table object 2") && m.contains("Integer"),
            "{m}"
        );
        for m in [m, why(crate::ops::arith(Arith::Mod, &Value::Null, &record(1, false)))] {
            assert!(
                !m.contains("RefCell") && !m.contains("Rc(") && !m.contains("Some("),
                "{m}"
            );
        }
    }

    #[test]
    fn types_read_as_al_types() {
        assert_eq!(RtType::Integer.describe(), "Integer");
        assert_eq!(RtType::Text(Some(30)).describe(), "Text[30]");
        assert_eq!(RtType::Code(None).describe(), "Code");
        assert_eq!(RtType::List(Box::new(RtType::Integer)).describe(), "List of [Integer]");
        assert_eq!(RtType::Record(4, true).describe(), "temporary Record");
        assert_eq!(RtType::Library("JsonArray".into()).describe(), "JsonArray");
        assert_eq!(RtType::Option(vec!["A".into()]).describe(), "Option");
    }
}
