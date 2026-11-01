//! The .NET types and texts of the column types of a TestRequestPage.SaveAsXml data file beyond those of
//! al-language-onprem/report/lt-aq/AqSaveAsXml.al:247 (gap G5-13), pinned by
//! al-language-onprem/report/lt-y51-saveasxml-time/Y51SaveAsXmlTime.al (BC 27.5, 28.5, 29.0):
//! - a Time is `xs:dateTime` (`msdata:DateTimeMode="Local"`) on 1899-12-30, whole seconds, `+00:00`
//!   (`1899-12-30T14:30:15+00:00` for 14:30:15.123), the element left out for 0T (:257-258, :276-277);
//! - a DateTime the same type, with its milliseconds (`2024-03-15T14:30:15.123+00:00`), left out for 0DT (:277);
//! - a Duration `xs:string`, the text of Format (`20 minutes 34 seconds 567 milliseconds`), `<DurC />` for 0 (:277);
//! - a BigInteger `xs:long` (`0` written);
//! - a Guid `xs:string` with `msdata:DataType` naming System.Guid of the .NET of the BC major (8.0.0.0 for BC 27 and 28,
//!   10.0.0.0 for BC 29), lower case without braces, the null Guid as zeros (:276, `#if BC29PLUS`).
//!
//! Each holds when no row is written too (:304-321, SaveAsXml_TimeColumn_NoRow).

use crate::dataset::DataFileText;
use al_builtins::value::{RtType, Value};

const GUID_NET8: &str =
    "xs:string;System.Guid, System.Private.CoreLib, Version=8.0.0.0, Culture=neutral, PublicKeyToken=7cec85d7bea7798e";
const GUID_NET10: &str =
    "xs:string;System.Guid, System.Private.CoreLib, Version=10.0.0.0, Culture=neutral, PublicKeyToken=7cec85d7bea7798e";

/// The type tag of a Guid column for BC `major`; another major is not pinned.
fn guid_type(major: u16) -> Option<&'static str> {
    match major {
        27 | 28 => Some(GUID_NET8),
        29 => Some(GUID_NET10),
        _ => None,
    }
}

/// The type tag of a column declared on a table field of type `ty` (the type the schema lists when no row is written).
pub(crate) fn declared(ty: &RtType, major: u16) -> Option<&'static str> {
    match ty {
        RtType::Integer => Some("xs:int"),
        RtType::Text(_) | RtType::Code(_) => Some("xs:string"),
        RtType::Decimal => Some("xs:decimal"),
        RtType::Boolean => Some("xs:boolean"),
        RtType::Date | RtType::Time | RtType::DateTime => Some("xs:dateTime"),
        RtType::Duration => Some("xs:string"),
        RtType::BigInteger => Some("xs:long"),
        RtType::Guid => guid_type(major),
        // An Option or Enum field: the member name (Y60SaveAsXmlNoRow.al Columns_TableFields_NoRow `KindC`, `CatC`).
        RtType::Option(_) | RtType::Enum(_) => Some("xs:string"),
        _ => None,
    }
}

/// The type tag of an expression column (a global, an array element, a field read through a record variable) of static type
/// `ty` when no row is written: the type of its AL type, pinned for Code, Text, Integer, Decimal, Boolean, Date, Option and Enum
/// (al-language-onprem/report/lt-y60-saveasxml-norow/Y60SaveAsXmlNoRow.al Columns_Expressions_NoRow, the same as with rows:
/// Columns_Expressions_WithRow). Other types are not pinned for an expression.
pub(crate) fn expression(ty: &RtType) -> Option<&'static str> {
    match ty {
        RtType::Code(_) | RtType::Text(_) | RtType::Option(_) | RtType::Enum(_) => Some("xs:string"),
        RtType::Integer => Some("xs:int"),
        RtType::Decimal => Some("xs:decimal"),
        RtType::Boolean => Some("xs:boolean"),
        RtType::Date => Some("xs:dateTime"),
        _ => None,
    }
}

/// The type tag of a written Option or Enum value whose text is its member name: `xs:string` (Y60SaveAsXmlNoRow.al
/// Columns_Expressions_WithRow `<VarOpt>Second</VarOpt>`, `<VarEnum>Y</VarEnum>`, `<RecOpt>C</RecOpt>`; Columns_OptionEnum_BlankAndNonBlank
/// `<KindC>A</KindC>`, `<CatC>X</CatC>`: the first member too). A caption other than the name is not pinned. `enum_members` are
/// the ordinals and names of the Enum's type.
pub(crate) fn member(value: &Value, text: &str, enum_members: Option<&[(i32, String)]>) -> Option<&'static str> {
    let name = match value {
        Value::Option(n, names) => usize::try_from(*n).ok().and_then(|i| names.get(i)).cloned(),
        Value::Enum(_, ord) => enum_members?.iter().find(|(o, _)| o == ord).map(|(_, n)| n.clone()),
        _ => None,
    }?;
    (name == text).then_some("xs:string")
}

impl crate::Session {
    /// `member` for a written column value with text `text`.
    pub(crate) fn data_file_member_type(&self, value: &Value, text: &str) -> Option<&'static str> {
        let members = match value {
            Value::Enum(id, _) => self
                .engine
                .program
                .objects
                .get(*id as usize)
                .map(|o| o.enum_values.as_slice()),
            _ => None,
        };
        member(value, text, members)
    }
}

/// The type tag of report column `c` of object `object` when no row is written: a field of the data item's `table` of a type
/// `declared` knows, else the static type of its source expression (`expression`).
pub(crate) fn column(
    vm: &crate::Session,
    object: usize,
    table: Option<&al_data::TableDef>,
    c: &al_ir::ReportColumn,
    major: u16,
) -> Option<&'static str> {
    if let Some(f) = c.field.zip(table).and_then(|(pos, t)| t.fields.get(pos)) {
        return declared(&f.ty, major);
    }
    let routine = vm.engine.program.objects[object].routines.get(c.routine?)?;
    match routine.body.stmts().as_slice() {
        [al_ir::Stmt::Exit(Some(al_ir::Expr::Load(place)))] => expression(&place_type(vm, object, place)?),
        _ => None,
    }
}

/// The static type of `place` in a routine of `object`: a global of the object, an element of a global array, a field of a
/// global record. Other places give `None`.
fn place_type(vm: &crate::Session, object: usize, place: &al_ir::Place) -> Option<RtType> {
    match place {
        al_ir::Place::Global(s) => vm.engine.program.objects[object].globals.get(*s).map(|g| g.ty.clone()),
        al_ir::Place::Index(p, _) => match place_type(vm, object, p)? {
            RtType::Array(_, elem) => Some(*elem),
            _ => None,
        },
        al_ir::Place::Field(p, pos) => match place_type(vm, object, p)? {
            RtType::Record(t, _) => vm
                .engine
                .tables
                .get(t as usize)?
                .as_ref()?
                .fields
                .get(*pos)
                .map(|f| f.ty.clone()),
            _ => None,
        },
        _ => None,
    }
}

/// The type tag and data file text of a Time, DateTime, Duration, BigInteger or Guid column value, or `None` for another
/// value or an unpinned shape (a DateTime whose milliseconds end in a zero).
pub(crate) fn of_value(value: &Value, major: u16) -> Option<(&'static str, DataFileText)> {
    Some(match value {
        Value::Time(0) | Value::DateTime(0) => ("xs:dateTime", DataFileText::Omit),
        Value::Time(t) => {
            let s = (t - 1) / 1000;
            (
                "xs:dateTime",
                DataFileText::Text(format!("1899-12-30T{}+00:00", hms(s))),
            )
        }
        Value::DateTime(v) => {
            let ms = v - 1;
            let (y, m, d) = al_builtins::format::civil_from_day(ms.div_euclid(86_400_000) as i32 + 1);
            let in_day = ms.rem_euclid(86_400_000);
            let frac = match in_day % 1000 {
                0 => String::new(),
                // .NET drops trailing zeros of the fraction; only a fraction of three digits is pinned (:277 `.123`).
                f if f % 10 == 0 => return None,
                f => format!(".{f:03}"),
            };
            let text = format!("{y:04}-{m:02}-{d:02}T{}{frac}+00:00", hms(in_day / 1000));
            ("xs:dateTime", DataFileText::Text(text))
        }
        Value::Duration(_) => ("xs:string", DataFileText::Same),
        Value::BigInteger(_) => ("xs:long", DataFileText::Same),
        Value::Guid(_) => (guid_type(major)?, DataFileText::Same),
        _ => return None,
    })
}

fn hms(seconds: i64) -> String {
    format!("{:02}:{:02}:{:02}", seconds / 3600, seconds / 60 % 60, seconds % 60)
}

/// The attributes of the schema element of a column with type tag `ty`, `type="..."` included.
pub(crate) fn element_attributes(ty: &str) -> String {
    match ty.split_once(';') {
        Some((xs, data_type)) => format!("msdata:DataType=\"{data_type}\" type=\"{xs}\""),
        // AqSaveAsXml.al:247 `<xs:element name="DayC" msdata:DateTimeMode="Local" type="xs:dateTime"`.
        None if ty == "xs:dateTime" => format!("msdata:DateTimeMode=\"Local\" type=\"{ty}\""),
        None => format!("type=\"{ty}\""),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(v: &Value) -> String {
        match of_value(v, 29) {
            Some((_, DataFileText::Text(t))) => t,
            Some((_, DataFileText::Omit)) => "<omit>".into(),
            Some((_, DataFileText::Same)) => "<same>".into(),
            None => "<none>".into(),
        }
    }

    #[test]
    fn a_time_is_a_date_time_on_1899_12_30_in_whole_seconds() {
        let t = |h: i64, m: i64, s: i64, ms: i64| Value::Time(((h * 60 + m) * 60 + s) * 1000 + ms + 1);
        assert_eq!(text(&t(14, 30, 15, 123)), "1899-12-30T14:30:15+00:00");
        assert_eq!(text(&t(0, 0, 0, 1)), "1899-12-30T00:00:00+00:00");
        assert_eq!(text(&Value::Time(0)), "<omit>");
        assert_eq!(of_value(&t(1, 0, 0, 0), 27).map(|(ty, _)| ty), Some("xs:dateTime"));
    }

    #[test]
    fn a_date_time_keeps_its_milliseconds() {
        let day = i64::from(al_builtins::format::day_from_civil(2024, 3, 15));
        let v = (day - 1) * 86_400_000 + (14 * 3600 + 30 * 60 + 15) * 1000 + 123 + 1;
        assert_eq!(text(&Value::DateTime(v)), "2024-03-15T14:30:15.123+00:00");
        assert_eq!(text(&Value::DateTime(v - 123)), "2024-03-15T14:30:15+00:00");
        assert_eq!(text(&Value::DateTime(v - 3)), "<none>");
        assert_eq!(text(&Value::DateTime(0)), "<omit>");
    }

    #[test]
    fn the_guid_type_names_the_dotnet_of_the_major() {
        let g = Value::Guid([0; 16]);
        assert_eq!(
            element_attributes(of_value(&g, 28).unwrap().0),
            "msdata:DataType=\"System.Guid, System.Private.CoreLib, Version=8.0.0.0, Culture=neutral, PublicKeyToken=7cec85d7bea7798e\" type=\"xs:string\""
        );
        assert!(element_attributes(of_value(&g, 29).unwrap().0).contains("Version=10.0.0.0"));
        assert!(of_value(&g, 30).is_none());
        assert_eq!(declared(&RtType::BigInteger, 29), Some("xs:long"));
        assert_eq!(text(&Value::Duration(5)), "<same>");
    }

    // al-language-onprem/report/lt-y60-saveasxml-norow/Y60SaveAsXmlNoRow.al Columns_TableFields_NoRow,
    // Columns_Expressions_NoRow: Option and Enum fields are xs:string; an expression column has the type of its AL type.
    #[test]
    fn option_enum_fields_and_expression_columns_have_their_type_without_a_row() {
        assert_eq!(declared(&RtType::Option(vec!["A".into()]), 29), Some("xs:string"));
        assert_eq!(declared(&RtType::Enum(5), 27), Some("xs:string"));
        let want = [
            (RtType::Code(Some(10)), Some("xs:string")),
            (RtType::Text(None), Some("xs:string")),
            (RtType::Integer, Some("xs:int")),
            (RtType::Decimal, Some("xs:decimal")),
            (RtType::Boolean, Some("xs:boolean")),
            (RtType::Date, Some("xs:dateTime")),
            (RtType::Option(vec!["A".into()]), Some("xs:string")),
            (RtType::Enum(3), Some("xs:string")),
            (RtType::Time, None),
            (RtType::Guid, None),
            (RtType::Variant, None),
        ];
        for (ty, tag) in want {
            assert_eq!(expression(&ty), tag, "{ty:?}");
        }
    }

    // Columns_Expressions_WithRow, Columns_OptionEnum_BlankAndNonBlank: an Option or Enum value written as its member name is
    // xs:string, the first member too.
    #[test]
    fn an_option_written_as_its_member_name_is_a_string() {
        let names: std::sync::Arc<[String]> = std::sync::Arc::from(vec!["First".to_string(), "Second".to_string()]);
        assert_eq!(
            member(&Value::Option(1, names.clone()), "Second", None),
            Some("xs:string")
        );
        assert_eq!(
            member(&Value::Option(0, names.clone()), "First", None),
            Some("xs:string")
        );
        assert_eq!(member(&Value::Option(1, names), "Zweite", None), None);
        let enum_names = vec![(0, "X".to_string()), (1, "Y".to_string())];
        assert_eq!(member(&Value::Enum(9, 1), "Y", Some(&enum_names)), Some("xs:string"));
        assert_eq!(member(&Value::Enum(9, 1), "Why", Some(&enum_names)), None);
        assert_eq!(member(&Value::Integer(1), "1", None), None);
    }
}
