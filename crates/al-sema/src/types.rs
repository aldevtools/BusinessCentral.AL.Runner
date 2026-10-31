//! Semantic types.

use crate::workspace::ObjKind;
use al_syntax::ast::TypeRef;

/// A name a type carries. Cloning a type clones this, so it is shared, not copied.
pub type Name = std::sync::Arc<str>;

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Ty {
    Integer,
    BigInteger,
    Decimal,
    Boolean,
    Char,
    Byte,
    Text(Option<u32>),
    Code(Option<u32>),
    Date,
    Time,
    DateTime,
    Duration,
    DateFormula,
    Guid,
    Option(Vec<String>),
    Enum(Name),
    Record {
        table: Name,
        temporary: bool,
    },
    /// Codeunit, Page, Report, Query, XmlPort, Interface variables.
    Object(ObjKind, Name),
    TestPage(Name),
    TestRequestPage(Name),
    /// A part on a test page, carrying the page it hosts.
    TestPart(Name),
    /// `TestPage.Filter` or a request page data item: a filter on this table.
    TestFilter(Name),
    List(Box<Ty>),
    Dictionary(Box<Ty>, Box<Ty>),
    Array(Vec<u32>, Box<Ty>),
    Label,
    Variant,
    /// A library type from the builtins catalog: `RecordRef`, `JsonObject`, `InStream`, ...
    Builtin(Name),
    /// A `DotNet` value (ADR-014): the alias (as written; compare case-insensitively) a `dotnet { }` block declares, or the
    /// empty string for the result of a `DotNet` member, whose type is only known at run time.
    DotNet(Name),
    /// The type could not be determined; errors on it are not reported again.
    Unknown,
}

/// The members of the platform's `TextEncoding` option, in ordinal order (docs: MSDos, UTF8, UTF16,
/// Windows); the corpus uses `TextEncoding::Windows` and `::UTF8` (text/TestTextEncodingWindows.al).
pub fn text_encoding_members() -> Vec<String> {
    ["MSDos", "UTF8", "UTF16", "Windows"].map(String::from).to_vec()
}

/// Joins the parts of a qualified object name in a type (`A.B."Ship. Method"`).
/// A dot cannot be used: quoted names may contain one. Split with [`qualified_parts`].
pub const QUAL_SEP: &str = "\u{1f}";

/// The parts of an object name stored in a type.
pub fn qualified_parts(name: &str) -> Vec<String> {
    name.split(QUAL_SEP).map(str::to_string).collect()
}

/// Argument `i` of a section as the name of an object, in the form types carry: the parts of
/// `Ns.Sub."Name"` joined by `QUAL_SEP`. `dataitem(Integer; System.Utilities.Integer)` names a
/// namespace-qualified table; the text of the argument alone cannot tell that from a quoted name
/// that holds dots (`"Pstd. Phys. Invt. Order Hdr"`).
pub fn section_object_name(sec: &al_syntax::ast::Section, i: usize) -> Option<String> {
    match sec.arg_name_parts(i) {
        Some(parts) => Some(parts.join(QUAL_SEP)),
        None => sec.args().get(i).cloned(),
    }
}

/// An object name from a type, as written (`A.B.Ship. Method`), for messages.
pub fn display_name(name: &str) -> String {
    name.replace(QUAL_SEP, ".")
}

impl Ty {
    pub fn from_type_ref(t: &TypeRef) -> Ty {
        let name = t.type_name();
        let lower = name.to_ascii_lowercase();
        let obj = || t.object_name_parts().join(QUAL_SEP);
        let size = || t.size_args().first().copied();
        let args = t.type_args();
        let arg = |i: usize| Box::new(args.get(i).map(Ty::from_type_ref).unwrap_or(Ty::Unknown));
        match lower.as_str() {
            "integer" => Ty::Integer,
            "biginteger" => Ty::BigInteger,
            "decimal" => Ty::Decimal,
            "boolean" => Ty::Boolean,
            "char" => Ty::Char,
            "byte" => Ty::Byte,
            "text" => Ty::Text(size()),
            "code" => Ty::Code(size()),
            "date" => Ty::Date,
            "time" => Ty::Time,
            "datetime" => Ty::DateTime,
            "duration" => Ty::Duration,
            "dateformula" => Ty::DateFormula,
            "guid" => Ty::Guid,
            "label" | "textconst" => Ty::Label,
            "variant" => Ty::Variant,
            "option" => Ty::Option(t.option_members()),
            "action" => Ty::Option(
                al_builtins::value::ACTION_MEMBERS
                    .iter()
                    .map(|m| m.to_string())
                    .collect(),
            ),
            "promptmode" => Ty::Option(
                al_builtins::value::PROMPT_MODE_MEMBERS
                    .iter()
                    .map(|m| m.to_string())
                    .collect(),
            ),
            "textencoding" => Ty::Option(text_encoding_members()),
            "enum" => Ty::Enum(obj().into()),
            "record" => Ty::Record {
                table: obj().into(),
                temporary: t.is_temporary(),
            },
            "testpage" => Ty::TestPage(obj().into()),
            "testrequestpage" => Ty::TestRequestPage(obj().into()),
            "list" => Ty::List(arg(0)),
            "dictionary" => Ty::Dictionary(arg(0), arg(1)),
            "array" => Ty::Array(t.size_args(), arg(0)),
            // The parts stay joined by `QUAL_SEP`: `DotNet A.B.C` (namespace `A.B`, alias `C`) is not `DotNet "A.B.C"` (one alias).
            "dotnet" => Ty::DotNet(obj().into()),
            _ => match ObjKind::from_type_name(&name) {
                Some(kind) => Ty::Object(kind, obj().into()),
                None => Ty::Builtin(canonical_builtin(&name).into()),
            },
        }
    }
}

impl std::fmt::Display for Ty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Ty::Text(Some(n)) => write!(f, "Text[{n}]"),
            Ty::Code(Some(n)) => write!(f, "Code[{n}]"),
            Ty::Code(None) => write!(f, "Code"),
            Ty::Text(None) => write!(f, "Text"),
            Ty::Enum(n) => write!(f, "Enum \"{}\"", display_name(n)),
            Ty::Record { table, .. } => write!(f, "Record \"{}\"", display_name(table)),
            Ty::Object(k, n) => write!(f, "{} \"{}\"", k.label(), display_name(n)),
            Ty::TestPage(n) => write!(f, "TestPage \"{}\"", display_name(n)),
            Ty::TestRequestPage(n) => write!(f, "TestRequestPage \"{}\"", display_name(n)),
            Ty::TestPart(n) => write!(f, "TestPart \"{}\"", display_name(n)),
            Ty::TestFilter(n) => write!(f, "TestFilter \"{}\"", display_name(n)),
            Ty::List(t) => write!(f, "List of [{t}]"),
            Ty::Dictionary(k, v) => write!(f, "Dictionary of [{k}, {v}]"),
            Ty::Array(d, t) => {
                let dims: Vec<String> = d.iter().map(u32::to_string).collect();
                write!(f, "array[{}] of {t}", dims.join(", "))
            }
            Ty::Builtin(n) => write!(f, "{n}"),
            Ty::DotNet(a) if a.is_empty() => write!(f, "DotNet"),
            Ty::DotNet(a) => write!(f, "DotNet {}", display_name(a)),
            Ty::Integer => f.write_str("Integer"),
            Ty::BigInteger => f.write_str("BigInteger"),
            Ty::Decimal => f.write_str("Decimal"),
            Ty::Boolean => f.write_str("Boolean"),
            Ty::Char => f.write_str("Char"),
            Ty::Byte => f.write_str("Byte"),
            Ty::Date => f.write_str("Date"),
            Ty::Time => f.write_str("Time"),
            Ty::DateTime => f.write_str("DateTime"),
            Ty::Duration => f.write_str("Duration"),
            Ty::DateFormula => f.write_str("DateFormula"),
            Ty::Guid => f.write_str("Guid"),
            Ty::Option(_) => f.write_str("Option"),
            Ty::Label => f.write_str("Label"),
            Ty::Variant => f.write_str("Variant"),
            Ty::Unknown => f.write_str("an unknown type"),
        }
    }
}

/// The catalog's spelling of a library type name (`recordref` → `RecordRef`).
fn canonical_builtin(name: &str) -> String {
    static RECEIVERS: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    let receivers = RECEIVERS.get_or_init(|| {
        let mut r: Vec<String> = al_builtins::catalog::Catalog::load()
            .signatures
            .iter()
            .map(|s| s.receiver.clone())
            .collect();
        r.dedup();
        r
    });
    receivers
        .iter()
        .find(|r| r.eq_ignore_ascii_case(name))
        .cloned()
        .unwrap_or_else(|| name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ty(src: &str) -> Ty {
        Ty::from_type_ref(&al_syntax::parse_type(src).unwrap_or_else(|| panic!("`{src}` does not parse")))
    }

    #[test]
    fn primitives_case_insensitive() {
        assert_eq!(ty("Integer"), Ty::Integer);
        assert_eq!(ty("boolean"), Ty::Boolean);
        assert_eq!(ty("BigInteger"), Ty::BigInteger);
        assert_eq!(ty("DateTime"), Ty::DateTime);
        assert_eq!(ty("Guid"), Ty::Guid);
    }

    #[test]
    fn sized_text_and_code() {
        assert_eq!(ty("Text"), Ty::Text(None));
        assert_eq!(ty("Text[30]"), Ty::Text(Some(30)));
        assert_eq!(ty("Code[20]"), Ty::Code(Some(20)));
    }

    #[test]
    fn object_types() {
        assert_eq!(
            ty("Record \"ALT Universal\" temporary"),
            Ty::Record {
                table: "ALT Universal".into(),
                temporary: true
            }
        );
        assert_eq!(ty("Codeunit Assert"), Ty::Object(ObjKind::Codeunit, "Assert".into()));
        assert_eq!(
            ty("Interface IALTCompute"),
            Ty::Object(ObjKind::Interface, "IALTCompute".into())
        );
        assert_eq!(ty("Enum \"ALT Status\""), Ty::Enum("ALT Status".into()));
        assert_eq!(ty("TestPage \"ALT Card\""), Ty::TestPage("ALT Card".into()));
    }

    #[test]
    fn collections_and_arrays() {
        assert_eq!(ty("List of [Integer]"), Ty::List(Box::new(Ty::Integer)));
        assert_eq!(
            ty("Dictionary of [Text, List of [Code[20]]]"),
            Ty::Dictionary(
                Box::new(Ty::Text(None)),
                Box::new(Ty::List(Box::new(Ty::Code(Some(20)))))
            )
        );
        assert_eq!(
            ty("array[2, 3] of Decimal"),
            Ty::Array(vec![2, 3], Box::new(Ty::Decimal))
        );
    }

    #[test]
    fn option_label_variant_and_library_types() {
        assert_eq!(ty("Option A,\"B C\""), Ty::Option(vec!["A".into(), "B C".into()]));
        assert_eq!(ty("Option ,A"), Ty::Option(vec!["".into(), "A".into()]));
        assert_eq!(ty("Label 'x', Locked = true"), Ty::Label);
        assert_eq!(ty("Variant"), Ty::Variant);
        assert_eq!(ty("JsonObject"), Ty::Builtin("JsonObject".into()));
        assert_eq!(ty("recordref"), Ty::Builtin("RecordRef".into()));
    }

    #[test]
    fn display_reads_as_al_and_never_as_a_rust_dump() {
        assert_eq!(ty("Option").to_string(), "Option");
        assert_eq!(ty("Integer").to_string(), "Integer");
        assert_eq!(Ty::Label.to_string(), "Label");
        assert_eq!(ty("Variant").to_string(), "Variant");
        assert_eq!(Ty::Unknown.to_string(), "an unknown type");
        assert_eq!(Ty::Option(vec!["A".into(), "B".into()]).to_string(), "Option");
        assert_eq!(
            Ty::Array(vec![3, 4], Box::new(Ty::Integer)).to_string(),
            "array[3, 4] of Integer"
        );
    }
}
