//! `Format(Value [, Length] [, FormatNumber])`. Every rule cites the corpus
//! test or the doc table ("Formatting values, dates, and time") that pins it;
//! where the two disagree the corpus wins (it runs on real BC).

use crate::native::Signal;
use crate::value::Value;

mod recordid_number;
mod region;
pub use region::{Region, region_column};
#[cfg(test)]
mod region_tests;

/// What formatting needs from the running program.
pub trait FormatCtx {
    /// Member name of enum `id` at `ordinal`.
    fn enum_name(&self, id: u32, ordinal: i32) -> String;
    /// The language `GlobalLanguage` names (1033 until set).
    fn language(&self) -> i32 {
        1033
    }
    /// Whether the language in force is a `CurrReport.Language := 2067` set in a running report (SaveAs, Execute, Run), the one
    /// place the Dutch month and weekday names are pinned (reports/runtime/ReportLanguageNotRunFormats.al:242, :370).
    /// `GlobalLanguage(2067)` is not.
    fn dutch_report_language(&self) -> bool {
        false
    }
    /// The format region a report set with `CurrReport.FormatRegion` while it runs (None: the session's own).
    fn region(&self) -> Option<String> {
        None
    }
    /// Whether the month names of the language in force are pinned with the format region: false where a report reads a
    /// language it did not set itself together with a region (only a trigger's `CurrReport.Language` then
    /// `CurrReport.FormatRegion` is measured, reports/runtime/BdRegionTests.al:1712-1912).
    fn region_language_pinned(&self) -> bool {
        true
    }
    /// Why the culture of `Format` is not known here (a report trigger whose region is not pinned), else None. A value whose text
    /// would depend on the culture is then refused.
    fn region_unpinned(&self) -> Option<String> {
        None
    }
    /// The caption and the namespace of the table with object index `table`, for the text of a RecordId; None where the
    /// context does not know the program (a RecordId format that names them is then refused).
    fn record_id_table(&self, table: u32) -> Option<(String, Option<String>)> {
        let _ = table;
        None
    }
    /// Whether the target BC version has the behavior of `gate` (false where the context knows no target).
    fn format_gate(&self, gate: al_version::Gate) -> bool {
        let _ = gate;
        false
    }
    /// Whether content behind a carried mark (`stream::is_carried_mark`) moves unread: only in the company build.
    fn carries_unread(&self) -> bool {
        false
    }
    /// Why a read of the caption of value `ordinal` of enum `id` (`None`: of any of its values) is refused: a translation
    /// file of its app could change it in the language in force (ADR-053 step 1). None where the context has no program.
    fn enum_caption_refused(&self, id: u32, ordinal: Option<i32>) -> Option<String> {
        let _ = (id, ordinal);
        None
    }
    /// The field numbers, in key order, of key `key` (0-based) of the table `rec` holds, for the text of a KeyRef; None where
    /// the context does not know the program or the key is not one the table declares.
    fn key_ref_fields(
        &self,
        rec: &std::rc::Rc<std::cell::RefCell<crate::value::RecordState>>,
        key: usize,
    ) -> Option<Vec<i32>> {
        let _ = (rec, key);
        None
    }
    /// The types of the primary key fields of the table with object index `table`, for the bookmark of a RecordId; None where
    /// the context does not know the program.
    fn record_id_key_types(&self, table: u32) -> Option<Vec<crate::value::RtType>> {
        let _ = table;
        None
    }
}

/// A context without the format region, for the plain en-US text a region then rewrites.
struct WithoutRegion<'a>(&'a dyn FormatCtx);

impl FormatCtx for WithoutRegion<'_> {
    fn enum_name(&self, id: u32, ordinal: i32) -> String {
        self.0.enum_name(id, ordinal)
    }
    fn language(&self) -> i32 {
        self.0.language()
    }
    fn dutch_report_language(&self) -> bool {
        self.0.dutch_report_language()
    }
    fn record_id_table(&self, table: u32) -> Option<(String, Option<String>)> {
        self.0.record_id_table(table)
    }
    fn format_gate(&self, gate: al_version::Gate) -> bool {
        self.0.format_gate(gate)
    }
    fn carries_unread(&self) -> bool {
        self.0.carries_unread()
    }
    fn key_ref_fields(
        &self,
        rec: &std::rc::Rc<std::cell::RefCell<crate::value::RecordState>>,
        key: usize,
    ) -> Option<Vec<i32>> {
        self.0.key_ref_fields(rec, key)
    }
    fn record_id_key_types(&self, table: u32) -> Option<Vec<crate::value::RtType>> {
        self.0.record_id_key_types(table)
    }
    fn enum_caption_refused(&self, id: u32, ordinal: Option<i32>) -> Option<String> {
        self.0.enum_caption_refused(id, ordinal)
    }
}

/// `Format` while a report's `FormatRegion` is set (by `CurrReport.FormatRegion` or the report property). Pinned: de-DE through
/// the setter (reports/PnC06ReportLanguage.al:Language_SetAndRead), de-DE, fr-FR and da-DK through the setter
/// (reports/PnH4CurrReport.al:Culture_FormatRegion_ChangesFormat: Decimal, Date, Boolean) and de-DE, en-GB,
/// fr-FR, ja-JP and zz-ZZ through the property (reports/runtime/YReportPropsTests.al:34-148: Decimal, Date, long Date,
/// DateTime, Integer). Texts are not culture sensitive; every other value, shape and region is refused.
fn format_in_region(
    v: &Value,
    length: i32,
    standard: i32,
    region: &str,
    ctx: &dyn FormatCtx,
) -> Result<String, FormatFail> {
    let plain = WithoutRegion(ctx);
    let refuse = || {
        FormatFail::Unsupported(format!(
            "Format of {} (standard format {standard}) while CurrReport.FormatRegion is '{region}' is not pinned (docs/probes-needed.md)",
            type_phrase(v)
        ))
    };
    match v {
        Value::Null | Value::Text(_) | Value::Code(_) => return format_value(v, length, standard, &plain),
        // An Integer has no thousands separator in any culture, as in en-US: `lang2:1033` under de-DE (PnC06ReportLanguage.al:Language_SetAndRead),
        // `1234567` in every region (YReportPropsTests.al:34-148).
        Value::Integer(_) | Value::BigInteger(_) if standard == 0 => return format_value(v, length, standard, &plain),
        _ => {}
    }
    let r = Region::parse_setter(region).ok_or_else(refuse)?;
    let text = match v {
        // `bool=Yes` under de-DE, fr-FR and da-DK (PnH4CurrReport.al:241-249).
        Value::Boolean(_) if standard == 0 && matches!(r, Region::DeDE | Region::FrFR | Region::DaDK) => {
            format_value(v, 0, standard, &plain)?
        }
        Value::Date(day, false) if *day != 0 && standard == 0 => {
            let (y, m, d) = civil_from_day(*day);
            match r {
                Region::ZzZZ | Region::EnUS => format_value(v, 0, standard, &plain)?,
                _ => r.date(y, m, d, false).ok_or_else(refuse)?,
            }
        }
        Value::Date(day, false) if *day != 0 && standard == 4 => {
            let (y, m, d) = civil_from_day(*day);
            if plain.language() != 1033 && !ctx.region_language_pinned() {
                return Err(refuse());
            }
            let month = region::long_month(plain.language(), m).ok_or_else(refuse)?;
            match r {
                Region::ZzZZ | Region::EnUS if plain.language() == 1033 => format_value(v, 0, standard, &plain)?,
                Region::ZzZZ | Region::EnUS => return Err(refuse()),
                _ => r.long_date(y, m, d, month).ok_or_else(refuse)?,
            }
        }
        // `31.01.25 14:30`: the short date, a space and the 24-hour time without seconds; zz-ZZ keeps en-US's `01/31/25 02:30 PM`
        // (YReportPropsTests.al:34-148, BdRegionTests.al:131-398 with the hour 09).
        Value::DateTime(t) if *t != 0 && standard == 0 => match r {
            Region::ZzZZ | Region::EnUS => format_value(v, 0, standard, &plain)?,
            _ => r.date_time_text(t - 1).ok_or_else(refuse)?,
        },
        // A Time: 24 hours, two digits each (reports/W3ReportObjectIdPreviewLanguage.al:912 `09:30:05`, BdRegionTests.al:131
        // `14:30:15#09:05:07`), ja-JP and es-ES ` 9:05:07` (:199, :344); zz-ZZ keeps en-US's `2:30:15 PM`. Milliseconds are not measured.
        Value::Time(t) if *t != 0 && standard == 0 => match r {
            Region::ZzZZ | Region::EnUS => format_value(v, 0, standard, &plain)?,
            _ => r.clock(t - 1, false).ok_or_else(refuse)?,
        },
        // The separators of the region (`1.234,5`, fr-FR groups with U+00A0); a negative amount is measured in every region of
        // BdRegionTests.al (`-1.234,5`).
        Value::Decimal(_) if standard == 0 => {
            let en = format_value(v, 0, standard, &plain)?;
            let (group, decimal) = r.separators();
            en.chars()
                .map(|c| match c {
                    ',' => group,
                    '.' => decimal,
                    c => c,
                })
                .collect()
        }
        _ => return Err(refuse()),
    };
    Ok(pad(text, length))
}

/// The text of a multi-language property (`ENU = 'a,b', DEU = 'x,y'`) in session language `language`: the entry of its
/// language code, else the ENU entry. Pinned for OptionCaptionML (record/table-properties/XTablePropsTests.al:175-232 on a
/// table field: 1033 ENU, 1031 DEU, 1036 falls back to ENU; :741-785 on a report column: an ENU-only list in 1031 gives ENU).
/// Another language and a list without the entry and without ENU are not pinned: the error says why.
pub fn ml_text_in_language(ml: &str, language: i32) -> Result<String, String> {
    let code = match language {
        1033 => "ENU",
        1031 => "DEU",
        1036 => "FRA",
        // format-evaluate/EvalLanguageBoolean.al EL_Option_* (a list with an entry for each of these languages).
        1030 => "DAN",
        1043 => "NLD",
        2067 => "NLB",
        // format-evaluate/EvalLanguageBoolean.al:1063-1318 (WA_BlankEnum*_ES_1034: `Primero` is the ESP caption of member First).
        1034 => "ESP",
        n => {
            return Err(format!(
                "`{ml}` in language {n} (only 1033, 1031, 1036, 1030, 1034, 1043 and 2067 are pinned)"
            ));
        }
    };
    let mut entries = Vec::new();
    let (mut start, mut quoted) = (0, false);
    let mut parts = Vec::new();
    for (i, c) in ml.char_indices() {
        match c {
            '\'' => quoted = !quoted,
            ',' if !quoted => {
                parts.push(&ml[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(&ml[start..]);
    for part in parts {
        let Some((k, v)) = part.split_once('=') else {
            return Err(format!("`{ml}` is not a list of language = text entries"));
        };
        let v = v.trim();
        let Some(text) = v.strip_prefix('\'').and_then(|r| r.strip_suffix('\'')) else {
            return Err(format!("`{ml}`: the {} entry is not a literal", k.trim()));
        };
        entries.push((k.trim().to_string(), text.replace("''", "'")));
    }
    let find = |c: &str| {
        entries
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(c))
            .map(|(_, t)| t.clone())
    };
    find(code)
        .or_else(|| find("ENU"))
        .ok_or_else(|| format!("`{ml}` has neither a {code} nor an ENU entry (not pinned)"))
}

/// Why a format did not produce a text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormatFail {
    /// The form is not pinned yet: the test is not run.
    Unsupported(String),
    /// BC raises this error.
    Error { message: String, code: String },
}

impl From<String> for FormatFail {
    fn from(s: String) -> Self {
        FormatFail::Unsupported(s)
    }
}

/// `Err` names the value whose format is not pinned yet.
/// The ordinal values of the Field table's `Type` option (`OptionOrdinalValues` of System.app's table 2000000041), by
/// member index: Integer is 34559 and Text 31488 (metadata/W6FieldKeyRows.al:Field_Type_*, BC 27.5 and 29.0).
const FIELD_TABLE_TYPE_ORDINALS: [i32; 21] = [
    4912, 4988, 11519, 11775, 11776, 11797, 12799, 26207, 26208, 31488, 31489, 33791, 33793, 34047, 34559, 35071,
    35583, 36095, 36863, 37119, 37375,
];

pub fn try_format(v: &Value, length: i32, standard: i32, ctx: &dyn FormatCtx) -> Result<String, String> {
    format_value(v, length, standard, ctx).map_err(|f| match f {
        FormatFail::Unsupported(s) => s,
        FormatFail::Error { message, .. } => message,
    })
}

/// `Format(BigText)`: the text it holds, all of it (streams/StTBigText.al:248 ConvertTextAndBack `direct`;
/// streams/DdStreamLeads.al:595, two AddText calls; xmlports/shapes-properties/XmlPortBigText.al: 218, 0 to 50000 characters).
/// Only the plain Format(BigText) is pinned. Rendered content comes out with its marks in the changed form (owner decision
/// 2026-11-12, ADR-040 amendment 2026-11-12, P3): whether BC returns a long rendered text whole is not probed.
fn bigtext_text(l: &crate::value::LibraryValue) -> Result<String, String> {
    match l {
        crate::value::LibraryValue::Empty => Ok(String::new()),
        crate::value::LibraryValue::Text(s) if crate::rendered_text::holds(s) => {
            Ok(crate::rendered_text::to_changed(s))
        }
        crate::value::LibraryValue::Text(s) => Ok(s.clone()),
        _ => Err("Format of a BigText of this kind is not pinned".to_string()),
    }
}

/// The values `Format` does not read: a column a platform row leaves unknown (refused), and a SQL counter value.
fn format_unread(v: &Value, length: i32, standard: i32) -> Option<Result<String, FormatFail>> {
    match v {
        Value::Unknown(why) => Some(Err(FormatFail::Unsupported(why.to_string()))),
        // `Format(x)` of a counter value gives digits only (SqlCounters.al P6924_Counters_Format_DigitsOnly), which digits
        // nothing pins: a marked text that may be passed on and logged, and that no text function reads (`Txt::has_marker`).
        // Standard format 9 of a whole number gives its digits too, with no error (format-evaluate/EvalDecimalEn.al:205
        // `Format(1000, 0, 9)`, types/decimal-integer/ScT70052.al:123 `Format(0, 0, 9)`; al-runner issue 6924 logs
        // `Format(x, 0, 9)` of a difference).
        Value::SqlCount(_) if length == 0 && (standard == 0 || standard == 9) => {
            Some(Ok(crate::native::SQL_COUNT_TEXT_MARK.to_string()))
        }
        // A placeholder's size: its digits would be the placeholder's (`placeholder_output::size`).
        Value::PlaceholderSize(m) => Some(Err(FormatFail::Unsupported(
            crate::placeholder_output::size_refusal_text(m, "Format"),
        ))),
        Value::SqlCount(_) => Some(Err(FormatFail::Unsupported(crate::sqlcount::refusal_text(
            "Format with a length or a standard format",
        )))),
        _ => None,
    }
}

/// A member with an empty name (a value named `""`) formats as its ordinal, at any position, and so does an ordinal no member has
/// (format-evaluate/EvalLanguageBoolean.al:1233 WA_BlankEnum3_ES_1034, `Format blank` is `[0]`; format-evaluate/format-matrix/
/// FMOptionBlankEnum.al `EnumBlank_MQ_Format`, `EnumBlank_LQ_Format`). A Caption on the value replaces the empty name before this.
fn enum_member_text(name: String, ordinal: i32) -> String {
    if name.is_empty() { ordinal.to_string() } else { name }
}

/// The text of a Char. Half of a surrogate pair is no character of a Rust string: `native::format_fns::format_1` keeps it
/// (ADR-068); any other way here refuses instead of giving the empty text.
fn char_text(c: u16) -> Result<String, FormatFail> {
    char::from_u32(u32::from(c))
        .map(String::from)
        .ok_or_else(|| FormatFail::Unsupported("Format of half of a surrogate pair here (not pinned)".into()))
}

/// An option value with no member name at its place: the name of a platform member AL cannot name
/// (format-evaluate/SoEvaluateSystemOptions.al:114 Numbers `num 2` Form), else its number.
fn unnamed_member_text(members: &[String], o: i32) -> String {
    crate::system_option_eval::hidden_name(members, o).map_or_else(|| o.to_string(), str::to_string)
}

/// The text of an Option value in a standard format (an arm of [`format_value`]).
fn option_text(o: &i32, members: &[String], standard: i32) -> Result<String, FormatFail> {
    Ok(match standard {
        2 | 9 if crate::sysenum::is_field_table_type(members) => FIELD_TABLE_TYPE_ORDINALS
            .get(*o as usize)
            .map_or_else(|| o.to_string(), |v| v.to_string()),
        2 | 9 if crate::sysenum::platform_ordinal(members, *o).is_some() => {
            crate::sysenum::platform_ordinal(members, *o).unwrap_or(*o).to_string()
        }
        // The IsolationLevel option shows member names for format 2 (format-evaluate/PnAPlatformOptions.al:
        // IsolationLevel_Members: Default, UpdLock, ReadCommitted, RepeatableRead). ReadUncommitted is not pinned.
        2 | 9 if crate::sysenum::is_isolation_level(members) => match members.get(*o as usize).map(String::as_str) {
            Some("ReadUncommitted") | None => {
                return Err(FormatFail::Unsupported(
                    "Format 2 of IsolationLevel::ReadUncommitted (not pinned)".into(),
                ));
            }
            Some(m) => m.to_string(),
        },
        2 | 9 if !crate::sysenum::ordinals_hidden(members, standard) => o.to_string(),
        // `Option` declared without members is the one empty name: it shows the number
        // (types/decimal-integer/ScT70062.al:Option_Defaults `nomembers_len`, `nomembers_3`).
        _ if members.len() == 1 && members[0].is_empty() => o.to_string(),
        // A member with a blank name shows as one space (types/decimal-integer/ScT70053.al:Option_Format `blank_len`).
        _ => match members.get(*o as usize) {
            // An unquoted empty member shows its number (text/P69OptionInText.al:Option_StrSubstNo
            // `subst_2`; record/crud/RcBInit.al:Props_BlankZero_Option_Caption `opt blank member`).
            Some(m) if m.is_empty() => unnamed_member_text(members, *o),
            Some(m) => crate::system_option_eval::shown_name(members, m).map_or_else(|| m.clone(), str::to_string),
            None => unnamed_member_text(members, *o),
        },
    })
}

/// The text of a Guid in a standard format (an arm of [`format_value`]).
fn guid_text(g: &[u8; 16], standard: i32) -> Result<String, FormatFail> {
    Ok(match standard {
        0 | 1 | 2 | 9 => format_guid(g),
        3 => format_guid(g).replace(['{', '}', '-'], ""),
        4 => format_guid(g).replace(['{', '}'], ""),
        // 5 is in parentheses, 6 the C struct form `{0X12345678,0X1234,0X5678,{0X9A,...}}`; 7, 8 and 10 do not exist
        // (format-evaluate/format-matrix/FMGuid.al:78-83, :168-173, :258-263, corpus PR 1256, English and German).
        5 => format!("({})", format_guid(g).trim_matches(['{', '}'])),
        6 => {
            let hex = format_guid(g).replace(['{', '}', '-'], "");
            let bytes: Vec<String> = (16..32).step_by(2).map(|i| format!("0X{}", &hex[i..i + 2])).collect();
            format!(
                "{{0X{},0X{},0X{},{{{}}}}}",
                &hex[..8],
                &hex[8..12],
                &hex[12..16],
                bytes.join(",")
            )
        }
        7 | 8 | 10 => return Err(no_such_format_unpinned()),
        n => {
            return Err(FormatFail::Unsupported(format!(
                "Format of a Guid with the standard format {n} (not pinned)"
            )));
        }
    })
}

/// `Format(Value, Length, Standard)` with BC's own errors kept apart from refusals.
pub fn format_value(v: &Value, length: i32, standard: i32, ctx: &dyn FormatCtx) -> Result<String, FormatFail> {
    if let Some(r) = format_unread(v, length, standard) {
        return r;
    }
    if let Some(why) = ctx.region_unpinned() {
        // A text, and an Integer in the standard format, read the same in every region the corpus measured
        // (reports/runtime/YReportPropsTests.al:34-148 `1234567`); any other value is refused.
        let same_everywhere = match v {
            Value::Null | Value::Text(_) | Value::Code(_) => true,
            Value::Integer(_) | Value::BigInteger(_) => standard == 0,
            _ => false,
        };
        if !same_everywhere {
            return Err(FormatFail::Unsupported(format!("Format of {} {why}", type_phrase(v))));
        }
    }
    if let Some(region) = ctx.region().filter(|r| !r.is_empty()) {
        return format_in_region(v, length, standard, &region, ctx);
    }
    let s = match v {
        Value::Null => String::new(),
        // The standard formats 3 to 8 do not exist for a Boolean, which the platform names 'Char' (types/lt-boolean-format/
        // Boolean_FormatNumbers: `Standard format number 3 does not exist for the type 'Char'.`).
        Value::Boolean(_) if (3..=8).contains(&standard) => return Err(no_such_format(standard, "Char")),
        // Standard format 10 does not exist for a Boolean or an Enum, nor 3 to 8 and 10 for a Text, a Code, a Char, an Option, an
        // Integer and a BigInteger (format-evaluate/format-matrix/FMBoolean.al:68, FMEnum.al:68, FMText.al:61-83, FMCode.al:61-83,
        // FMChar.al:61-83, FMOption.al:61-83, FMNumeric.al:134 `<Standard Format,3>`, corpus PRs 1249 and 1256). The type name
        // in the error text is not pinned for them.
        Value::Boolean(_) | Value::Enum(..) if standard == 10 => return Err(no_such_format_unpinned()),
        Value::Text(_)
        | Value::TextConst(_)
        | Value::Code(_)
        | Value::Char(_)
        | Value::Option(..)
        | Value::Integer(_)
        | Value::BigInteger(_)
            if matches!(standard, 3..=8 | 10) =>
        {
            return Err(no_such_format_unpinned());
        }
        Value::Boolean(b) => crate::format_boolean::boolean_text(*b, standard, ctx.language())?,
        Value::Integer(i) => i.to_string(),
        Value::BigInteger(i) => i.to_string(),
        // A Byte formats as the character of that code (streams/TestProbeStreamContracts.al:Read_IntoTypes,
        // `integer_into_byte`: 2 is `\u0002`).
        // A byte from 0x80 up reads as the default (code page 437) stream encoding reads it: the bytes of a UTF-8 text
        // `C3 A9 E2 82 AC` format as the characters `\u201C \u0014 \u00CB \u00E9 \u00BC` (misc/lt-probes-needed-3/
        // PnC20JsonStreams.al:WriteTo_NonAscii_PerEncoding).
        // Byte 200 formats as U+02DC (streams/instream-outstream/StreamLengthIntegers.al:Byte_WriteReadLength_1 `n200_rd` 1/˜/Yes/2,
        // streams/media/P102ReadTyped.al:187-188 `r1_v`).
        // Also in every standard format that exists for it (types/decimal-integer/ScT70052.al:173-187).
        Value::Byte(200) => "\u{2DC}".into(),
        Value::Byte(b) if *b >= 0x80 && crate::stream::Encoding::MsDos.decode_pinned(&[*b]) => {
            crate::stream::Encoding::MsDos.decode(&[*b])
        }
        // Kept, not pinned: a byte from 0x80 up whose character no corpus test pins shows as the Latin-1 character of its
        // code; json/TestJsonComplete.al:JsonObject_GetByte_ReturnsValue (Assert.AreEqual of Byte 200) passes through it.
        Value::Byte(b) => char::from(*b).to_string(),
        Value::Char(c) => char_text(*c)?,
        // Under GlobalLanguage 1031 a standard format a Decimal does not have raises the German text
        // (format-evaluate/FormatPrecStdDe.al:52-55, `Die Standardformatnummer 5 ist für den Typ 'Decimal'
        // nicht vorhanden.`).
        Value::Decimal(d) => format_decimal(d, standard).map_err(|f| match f {
            FormatFail::Error { code, .. } if code == "InvalidStandardFormat" && ctx.language() == 1031 => {
                FormatFail::Error {
                    message: format!("Die Standardformatnummer {standard} ist für den Typ 'Decimal' nicht vorhanden."),
                    code,
                }
            }
            f => f,
        })?,
        // A NUL character inside a text is kept (format-evaluate/format-matrix/FMEvalText.al:141-143 and FMEvalCode3.al:141-143,
        // corpus PR 1256: `Format(V, 0, 9)` of a text holding NUL shows it). LTP89StreamText.al `nul_between` gets no NUL here:
        // `Wide[1] := 0` ends the text (interp `write_char`).
        Value::Text(t) | Value::TextConst(t) | Value::Code(t) => t.to_string(),
        Value::Option(o, members) => option_text(o, members, standard)?,
        // Nor for an enum, named 'Option' (the enum test Enum_FormatNumbers `blue3`).
        Value::Enum(..) if (3..=8).contains(&standard) => return Err(no_such_format(standard, "Option")),
        Value::Enum(id, o) => match standard {
            2 | 9 => o.to_string(),
            _ => {
                if let Some(why) = ctx.enum_caption_refused(*id, Some(*o)) {
                    return Err(FormatFail::Unsupported(why));
                }
                enum_member_text(ctx.enum_name(*id, *o), *o)
            }
        },
        // Numbers 0, 1, 2 and 9 are the braced form; 3 has neither braces nor dashes, 4 only the dashes (metadata/platform-tables/
        // PlTGuid.al:Format_Numbers, NullGuid_Values); the others are not pinned for a Guid.
        Value::Guid(g) => guid_text(g, standard)?,
        Value::Blob(b) => blob_text(b, ctx),
        // The bookmark (`recordid_number`).
        Value::Library(crate::library_kind::LibraryKind::RecordId, l) if standard == 10 => {
            recordid_number::bookmark_text(&l.borrow(), ctx)?
        }
        Value::Date(0, _) | Value::Time(0) | Value::DateTime(0) => String::new(),
        Value::Date(day, closing) => format_date(*day, *closing, standard, ctx.language())?,
        Value::Time(t) => format_time(t - 1, standard)?,
        Value::DateTime(t) => format_datetime(t - 1, standard)?,
        Value::Duration(ms) => format_duration(*ms, standard, ctx.language())?,
        Value::DateFormula(f) => date_formula_text(f, standard, ctx.language())?,
        // A Version reads `major.minor.build.revision` (session-system/TestBCPlatformContracts.al:120-136
        // compares the formatted AppVersion with the manifest's `1.0.0.0`).
        Value::Library(crate::library_kind::LibraryKind::Version, _) if standard == 9 => {
            // session-system/TestProbeSessionMiscContracts.al:Version_Forms.
            return Err(no_such_format(9, "Version"));
        }
        // A BigText formats as the text it holds (`bigtext_text`).
        Value::Library(crate::library_kind::LibraryKind::BigText, l) if standard == 0 && length == 0 => {
            bigtext_text(&l.borrow())?
        }
        Value::Library(crate::library_kind::LibraryKind::TextBuilder, l) => {
            crate::format_textbuilder::text(&l.borrow(), standard)?
        }
        Value::Library(crate::library_kind::LibraryKind::Version, l) => match &*l.borrow() {
            crate::value::LibraryValue::Version(v) => format!("{}.{}.{}.{}", v[0], v[1], v[2], v[3]),
            _ => return Err("Format of a Version nobody set".to_string().into()),
        },
        // A SessionSettings renders its properties in this layout (a fresh instance: session-system/session/
        // SessionSettingsFreshReads.al:57); it shows the time zone and differs when a property differs
        // (session-system/TestSessionSettings.al:457-493). ProfileSystemScope reads No (its setter keeps nothing, sessiontypes.rs).
        Value::Library(crate::library_kind::LibraryKind::SessionSettings, l) => {
            let fresh = crate::value::SessionSettingsState::default();
            let b = l.borrow();
            let s = match &*b {
                crate::value::LibraryValue::SessionSettings(s) => s,
                _ => &fresh,
            };
            format!(
                "Company={}, ProfileId={}, LanguageId={}, LocaleId={}, TimeZone={}, ProfileAppId={}, ProfileSystemScope=False",
                s.company,
                s.profile_id.clone().unwrap_or_default(),
                s.language_id.unwrap_or(0),
                s.locale_id.unwrap_or(0),
                s.time_zone.clone().unwrap_or_default(),
                format_guid(&s.profile_app_id.unwrap_or([0; 16])),
            )
        }
        // The text form of a FilterPageBuilder is only known to be non-empty
        // (pages/filterpagebuilder/TestFilterPageBuilder.al:FilterPageBuilder_Format_ReturnsANonEmptyRendering).
        // A builder nobody added a control to formats as an empty text (pages/filterpagebuilder/W3FilterPageBuilder.al:
        // PageCaption_DefaultOfAFreshBuilder, BC 27.5 and 29.0).
        Value::Library(crate::library_kind::LibraryKind::FilterPageBuilder, l) => match &*l.borrow() {
            crate::value::LibraryValue::FilterPageBuilder(s) if !s.controls.is_empty() => {
                "FilterPageBuilder".to_string()
            }
            _ => String::new(),
        },
        // A TableFilter field nobody set holds no filter: empty text. Base App's Change Log
        // Management.HasValue formats such a value on every insert and the tests pass on BC
        // (permissions/TestMetadataExpandedPermission.al:219-235); the text of a set filter is not
        // pinned (docs/probes-needed.md).
        Value::Library(crate::library_kind::LibraryKind::Media, l)
            if matches!(&*l.borrow(), crate::value::LibraryValue::Text(_)) =>
        {
            match &*l.borrow() {
                crate::value::LibraryValue::Text(s) => s.clone(),
                _ => unreachable!(),
            }
        }
        Value::Library(name, l)
            if name == "TableFilter" && matches!(&*l.borrow(), crate::value::LibraryValue::Empty) =>
        {
            String::new()
        }
        // A TableFilter Evaluate set formats as the text it was given
        // (permissions/permission-sets/AvSetPermissionFilter.al:131,158; evaluate.rs into_table_filter).
        Value::Library(name, l)
            if name == "TableFilter" && matches!(&*l.borrow(), crate::value::LibraryValue::Text(_)) =>
        {
            match &*l.borrow() {
                crate::value::LibraryValue::Text(s) => s.clone(),
                _ => unreachable!(),
            }
        }
        // A document type or an XML declaration formats as its markup (xml/xml/DocTypeErrors.al:
        // FormatOfUninitialisedAndInitialised); an unassigned one raises.
        Value::Library(name, l) if matches!(name.as_str(), "XmlDocumentType" | "XmlDeclaration") => {
            match crate::xml::format_node(name, l) {
                Ok(s) => s,
                Err(Signal::Error(e)) => {
                    return Err(FormatFail::Error {
                        message: e.message,
                        code: e.code,
                    });
                }
                Err(Signal::Unsupported(why)) => return Err(why.into_reason().into()),
                Err(_) => return Err(format!("Format of a {name} that is neither a result nor an error").into()),
            }
        }
        Value::Library(name, l) if matches!(name.as_str(), "JsonObject" | "JsonArray" | "JsonValue" | "JsonToken") => {
            crate::format_json::format_json(name.as_str(), l, standard)?
        }
        // A WebServiceActionContext has no text form (session-system/TestWebServiceActionContext.al:615-650).
        Value::Library(crate::library_kind::LibraryKind::WebServiceActionContext, _) => String::new(),
        // A RecordId reads `Table: key1,key2` (corpus record/TestRecordRecordId.al:42-56 only needs
        // the key values in it; the layout is docs/probes-needed.md).
        Value::Library(crate::library_kind::LibraryKind::RecordId, l) => match &*l.borrow() {
            crate::value::LibraryValue::RecordId { table, name, key, .. } => {
                // The numbers 3 to 8 do not exist for a RecordId; 2 and 9 format the key values with that number (types/
                // lt-implicit-conversions/P79RecordId.al:CodeKey_FormatNumbers, DateKey_FormatNumbers); others are refused.
                let key_standard = recordid_number::recordid_key_standard(standard)?;
                // A key that is empty or holds a comma is quoted (record/TestProbeRecordRefContracts.al:RecordId_FormatForms;
                // P79RecordId.al:NeverRead_BlankRecordId, a blank Date too).
                let key = key
                    .iter()
                    .map(|k| {
                        format_value(k, 0, key_standard, ctx).map(|t| {
                            // A text that starts with a double quote is quoted and doubles its quotes (format-evaluate/
                            // EvaluateRecordIdComposite.al:RoundTrip_KeyValueWithComma `dquote`: `"Q"` reads `"""Q"""`); a quote further in
                            // is left as it is (types/operators-expressions/P79RecordId.al:CompositeKey_SpecialCharacters: `A"B`).
                            // An Option key value whose member is a blank (` `) is quoted too: `" "`
                            // (errors/WcUnpinnedSingles.al:129, :140).
                            if t.is_empty()
                                || (matches!(k, Value::Option(..)) && t.trim().is_empty())
                                || (matches!(k, Value::Text(_) | Value::Code(_))
                                    && (t.contains(',') || t.starts_with('"')))
                                // A Text key that starts with a blank is quoted: `" lead"` (format-evaluate/Y14Small.al:173,
                                // corpus master). A trailing blank is not pinned.
                                || (matches!(k, Value::Text(_)) && t.starts_with(' '))
                            {
                                format!("\"{}\"", t.replace('"', "\"\""))
                            } else {
                                t
                            }
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                // Format 1 names the table by its caption (record/crud/RcBMeta.al:572, :578, also under
                // GlobalLanguage 1031); format 9 of a table in a namespace names it `[<namespace>.<name>]`
                // (language/namespaces/P69NsConsumers.al:214, a name with spaces; :210 format 1 of the same table
                // gives the plain name, which is its caption), a one-word name too, and a name with a dot is quoted
                // inside the brackets (corpus branch al-runtime-rs-staging-r6985, language/namespaces/
                // R7003RecordIdFormat9.al: Format9_NamespacedOwnTables, Format9_BaseAppContactAndItem `[Microsoft.CRM.Contact.Contact]`;
                // a table outside a namespace gives the plain name, Format9_TablesWithoutNamespace). The other numbers give
                // the plain name (RcBMeta.al:570-574).
                let shown = match standard {
                    1 | 9 => {
                        let Some((caption, namespace)) = ctx.record_id_table(*table) else {
                            return Err(format!("Format of a RecordId with standard format {standard} where the table caption is not known").into());
                        };
                        match (standard, namespace) {
                            (1, _) => caption,
                            (_, Some(ns)) => format!("[{ns}.{}]", recordid_number::namespaced_name(name)?),
                            (_, None) => name.clone(),
                        }
                    }
                    _ => name.clone(),
                };
                // A RecordId made by Evaluate from a table name alone has no key values and reads as the name
                // (format-evaluate/P128EvalRecordId.al:P128_Evaluate_IntoRecordId_OtherTexts `table name only`); format 9 of a
                // table in a namespace reads as the bracketed name alone (language/namespaces/RecordIdFormat9Evaluate.al:285-286,
                // `[LT.R7003.Data."R7003.Dot"]`). Format 1 is not pinned.
                if key.is_empty() {
                    if shown != *name && standard != 9 {
                        return Err(format!(
                            "Format(RecordId, 0, {standard}) of a RecordId with no key values (not pinned)"
                        )
                        .into());
                    }
                    shown
                } else {
                    format!("{shown}: {}", key.join(","))
                }
            }
            _ => String::new(),
        },
        // A KeyRef reads `Field<no>` for each field of the key, in key order, joined by commas; format numbers 0, 1, 2 and 9 give
        // the same text, and a length shorter than the text cuts it (recordref-fieldref/recordref/Y12KeyRefFormat.al:33-87, corpus
        // branch al-runtime-rs-staging-y12, al-runner issue 6437). Other numbers, a length that would pad, and the implicit
        // SystemId key are not pinned.
        Value::Library(crate::library_kind::LibraryKind::KeyRef, l) => {
            let crate::value::LibraryValue::KeyRef(rec, key) = &*l.borrow() else {
                return Err(format!(
                    "Format of a KeyRef that was never assigned (standard format {standard}) is not pinned yet"
                )
                .into());
            };
            if !matches!(standard, 0 | 1 | 2 | 9) {
                return Err(format!("Format of a KeyRef (standard format {standard}) is not pinned yet").into());
            }
            let Some(fields) = ctx.key_ref_fields(rec, *key) else {
                return Err(
                    "Format of a KeyRef of a key the table does not declare (the SystemId key) is not pinned yet"
                        .to_string()
                        .into(),
                );
            };
            let text = fields.iter().map(|f| format!("Field{f}")).collect::<Vec<_>>().join(",");
            let n = text.chars().count();
            if length == 0 || (length > 0 && n == length as usize) {
                return Ok(text);
            }
            if length > 0 && n > length as usize {
                return Ok(text.chars().take(length as usize).collect());
            }
            return Err(format!("Format of a KeyRef with length {length} (padding is not pinned yet)").into());
        }
        // A BigText formats as its text (streams/StTBigText.al:ConvertTextAndBack `format` direct).
        Value::Library(name, l) if name.as_str() == "BigText" && standard == 0 => match &*l.borrow() {
            crate::value::LibraryValue::Text(t) => t.clone(),
            crate::value::LibraryValue::Empty => String::new(),
            _ => return Err("Format of a BigText with a capacity".to_string().into()),
        },
        // Duration and the other reference types get their formats with
        // the corpus tests that pin them.
        other => {
            return Err(format!(
                "Format of {} (standard format {standard}) is not pinned yet",
                type_phrase(other)
            )
            .into());
        }
    };
    // A Boolean pads on the right and cuts at the length (types/TestProbeScalarContracts.al:FormatBoolean_Numbers:
    // `Format(true, 10)` is `Yes       `, `Format(true, 2)` is `Ye`).
    if matches!(
        v,
        Value::Boolean(_) | Value::Option(..) | Value::Enum(..) | Value::Guid(_)
    ) && length > 0
    {
        let cut: String = s.chars().take(length as usize).collect();
        return Ok(format!(
            "{cut}{}",
            " ".repeat((length as usize).saturating_sub(cut.chars().count()))
        ));
    }
    // A Text shorter than the length is padded on the right (types/decimal-integer/ScT70053.al:Text_FormatNumbers `len30`).
    if let (Value::Text(_) | Value::Code(_), true) = (v, length > 0) {
        let n = s.chars().count();
        if n < length as usize {
            return Ok(format!("{s}{}", " ".repeat(length as usize - n)));
        }
    }
    if let (Value::Library(crate::library_kind::LibraryKind::TextBuilder, _), true) = (v, length != 0) {
        return crate::format_textbuilder::with_length(&s, length);
    }
    // A RecordId is padded on the right too (types/operators-expressions/P79RecordId.al:409,421 `len-30`).
    if let (Value::Library(crate::library_kind::LibraryKind::RecordId, _), true) = (v, length > 0) {
        let n = s.chars().count();
        if n < length as usize {
            return Ok(format!("{s}{}", " ".repeat(length as usize - n)));
        }
    }
    // A negative length cuts a Text the same way (types/decimal-integer/ScT70053.al:Text_FormatNumbers `len-5`), and a Date
    // (types/date-time/Format.al:525,535: `Format(20240305D, -1, 9)` is `2`), and a RecordId (P79RecordId.al:410,422).
    if let (
        Value::Text(_)
        | Value::Code(_)
        | Value::Date(..)
        | Value::Library(crate::library_kind::LibraryKind::RecordId, _),
        true,
    ) = (v, length < 0)
    {
        if s.chars().count() > length.unsigned_abs() as usize {
            return Ok(s.chars().take(length.unsigned_abs() as usize).collect());
        }
    }
    // A text longer than the length is cut to it (text/textbuilder-bigtext/TxCLabel.al:Label_Value_And_Format `Format(Plain, 5)`).
    if matches!(
        v,
        Value::Text(_)
            | Value::Code(_)
            | Value::Integer(_)
            | Value::BigInteger(_)
            | Value::Decimal(_)
            | Value::Date(..)
            | Value::Time(_)
            | Value::DateTime(_)
            // types/operators-expressions/P79RecordId.al:RecordId_InLongerTexts `comp-subst-width`.
            | Value::Library(crate::library_kind::LibraryKind::RecordId, _)
    ) && length > 0
        && s.chars().count() > length as usize
    {
        return Ok(s.chars().take(length as usize).collect());
    }
    Ok(pad(s, length))
}

/// The value's type in a refusal text: a name, never the value's contents (a Record holds its
/// whole row, filters and state, which made one refusal reason of a single test a page long).
/// A Blob shows `*` when it holds data and one space when it does not (record/TestFormatRecordContracts.al:132-136, the cleared
/// record shows the blank form).
fn blob_text(b: &crate::stream::BlobRef, ctx: &dyn FormatCtx) -> String {
    let buf = b.borrow();
    // ADR-040: the text shows whether the buffer holds data, which a pending buffer cannot say.
    match &buf.render_pending {
        // Content carried unread in the company build is never empty (a re-encoded picture, a re-zipped package):
        // it shows `*` (assumption `company-build-carried-content-not-empty`).
        Some(report) if ctx.carries_unread() && crate::stream::is_carried_mark(report) => {
            crate::assumptions::assume(crate::assumptions::Assumption::CompanyBuildCarriedContentNotEmpty);
            "*".to_string()
        }
        // A placeholder report output is not empty, as BC's rendering (ADR-040 amendment 2026-11-11, `placeholder_output`).
        Some(mark) if crate::placeholder_output::is_placeholder(mark) => "*".to_string(),
        pending => {
            if let Some(report) = pending {
                crate::stream::note_pending_observation(report);
            }
            (if buf.bytes.is_empty() && !buf.unread_content {
                " "
            } else {
                "*"
            })
            .to_string()
        }
    }
}

/// A formula prints without its angle brackets in formats 0 and 9 (types/TestDateRoundFormatStringContracts.al:
/// DateFormula_Evaluate_ThenFormat_PrintsTheNormalizedFormula; format-evaluate/RandomFormat.al:484-505). Formats 0 and 1
/// print the letters of the global language, format 9 the English letters, format 2 the English letters in angle brackets (the
/// empty formula `<>`), and formats 3 to 8 raise (format-evaluate/Y62DateFormulaFormat.al, writer Y62; see
/// `date_formula_language`). Formats 3 to 8 in another language than 1033 and the other numbers are refused.
fn date_formula_text(f: &str, standard: i32, language: i32) -> Result<String, FormatFail> {
    let text = crate::datefns::formula_text(f);
    match standard {
        9 => Ok(text),
        0 => crate::date_formula_language::format_zero(&text, language).map_err(FormatFail::Unsupported),
        1 => crate::date_formula_language::format_one(&text, language).map_err(FormatFail::Unsupported),
        2 => Ok(format!("<{text}>")),
        3..=8 if language == 1033 => Err(no_such_format(standard, "Duration")),
        n => Err(FormatFail::Unsupported(format!(
            "Format of a DateFormula with standard format {n} in the language {language} (not pinned)"
        ))),
    }
}

fn type_phrase(v: &Value) -> String {
    let name = match v {
        Value::Null => "an unassigned Variant",
        Value::Record(_) => "a Record",
        Value::List(_) => "a List",
        Value::Dictionary(_) => "a Dictionary",
        Value::Array(_) => "an Array",
        Value::Codeunit(_) => "a Codeunit",
        Value::Blob(_) => "a Blob",
        Value::TestPage(_) => "a TestPage",
        Value::TestControl(_) => "a TestPage control",
        Value::Query(_) => "a Query",
        Value::DotNet(_) => "a DotNet value",
        Value::Library(name, _) => return format!("a {name}"),
        // The remaining types have a text form, so this is only reached when one is added
        // without a case above.
        _ => "a value",
    };
    name.to_string()
}

/// A positive length right-aligns in that many characters (text/TestFormat.al:
/// `Format(42, 5) = '   42'`).
pub(crate) fn pad(s: String, length: i32) -> String {
    let n = crate::value::Value::text_len(&s);
    if length > 0 && (length as usize) > n {
        format!("{}{s}", " ".repeat(length as usize - n))
    } else {
        s
    }
}

/// The error of a Decimal outside +-999,999,999,999,999.99 (format-evaluate/RandomFormat.al:Format_Decimal `af191`, `af193`;
/// a Duration converted into a Decimal: types/duration/D_Duration.al:Duration_To_Integer_BeyondRange `af126`).
pub(crate) fn decimal_range_error(d: &rust_decimal::Decimal) -> Option<FormatFail> {
    if d.abs() <= rust_decimal::Decimal::new(99_999_999_999_999_999, 2) {
        return None;
    }
    // The shown value keeps 18 significant digits (types/decimal-integer/ScT70058.al:Decimal_Digits `af83`).
    let kept = d
        .round_sf_with_strategy(18, rust_decimal::RoundingStrategy::MidpointAwayFromZero)
        .unwrap_or(*d);
    let whole = kept.round().to_string();
    let (sign, digits) = whole.strip_prefix('-').map_or(("", whole.as_str()), |r| ("-", r));
    let mut grouped = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(c);
    }
    Some(FormatFail::Error {
        message: format!(
            "The value of Decimal parameter  is outside of the permitted range. The current value is: {sign}{grouped}. The permitted range is: from -999,999,999,999,999.99 to 999,999,999,999,999.99."
        ),
        code: "OutsidePermittedRange".into(),
    })
}

/// Format 0 groups thousands (`1,234,567.891`); formats 1, 2 and 9 do not
/// (types/TestFormatEvaluateTextContracts.al:Format_Decimal_StandardFormats). Trailing zeros of the
/// decimals are not printed. The other numbers are not pinned.
fn format_decimal(d: &rust_decimal::Decimal, standard: i32) -> Result<String, FormatFail> {
    if let Some(e) = decimal_range_error(d) {
        return Err(e);
    }
    // A Decimal keeps 18 significant digits: 123456789012345.123456 shows as 123456789012345.123 (types/decimal-integer/ScT70050.al:
    // FormatNumbers_Large).
    let int_digits = if d.trunc().is_zero() {
        0
    } else {
        d.trunc().abs().to_string().len() as u32
    };
    let orig = *d;
    let d = &if int_digits < 18 {
        d.round_dp_with_strategy(18 - int_digits, rust_decimal::RoundingStrategy::MidpointAwayFromZero)
    } else {
        *d
    };
    let n = d.normalize();
    // A negative value that is too small to show reads `-0` in the XML form, not `0` (misc/lt-probes-needed-1/
    // PnAStreamDecimalBytes.al:Write_Decimal_Tiny `minus-1E-28`), and in the default format 0 too (types/decimal-integer/
    // DecimalDigits.al:480-541 QE_Decimal_SmallResults_DecimalPlaces, `neg tiny quotient default format`). The other
    // standard formats are not pinned for it.
    if n.is_zero() && orig.is_sign_negative() && !orig.is_zero() {
        if standard == 9 || standard == 0 {
            return Ok("-0".to_string());
        }
        return Err(FormatFail::Unsupported(
            "Format of a negative Decimal too small to show, in a standard format other than 0 and 9 (not pinned)"
                .into(),
        ));
    }
    let plain = n.to_string();
    if matches!(standard, 1 | 2 | 9) {
        return Ok(plain);
    }
    // Format 4 is format 1 and 3 is format 0 with the sign behind the number: a trailing blank when it is not negative, a
    // trailing minus when it is (format-evaluate/P86FormatDecimal.al:Num_*; text/
    // TestProbeTextDateFormatContracts.al:Format_FormatStrings_Number). Numbers 5 to 8 do not exist for a Decimal.
    if (5..=8).contains(&standard) {
        return Err(no_such_format(standard, "Decimal"));
    }
    if standard >= 10 {
        return Err(no_such_format(standard, "Decimal"));
    }
    if standard != 0 && standard != 3 && standard != 4 {
        return Err(format!("Format of a Decimal with standard format {standard} (not pinned)").into());
    }
    let (sign, digits) = plain.strip_prefix('-').map_or(("", plain.as_str()), |r| ("-", r));
    if standard == 4 {
        return Ok(format!("{digits}{}", if sign.is_empty() { " " } else { "-" }));
    }
    let tail = match (standard, sign) {
        (3, "-") => "-",
        (3, _) => " ",
        _ => "",
    };
    let sign = if standard == 3 { "" } else { sign };
    let (int, frac) = digits.split_once('.').map_or((digits, None), |(i, f)| (i, Some(f)));
    let mut grouped = String::new();
    for (i, c) in int.chars().enumerate() {
        if i > 0 && (int.len() - i) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(c);
    }
    Ok(match frac {
        Some(f) => format!("{sign}{grouped}.{f}{tail}"),
        None => format!("{sign}{grouped}{tail}"),
    })
}

pub fn format_guid(g: &[u8; 16]) -> String {
    let h: String = g.iter().map(|b| format!("{b:02X}")).collect();
    format!(
        "{{{}-{}-{}-{}-{}}}",
        &h[0..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..32]
    )
}

/// Day number 1 = 0001-01-01. Returns (year, month, day).
pub fn civil_from_day(day: i32) -> (i32, u32, u32) {
    // Days since 0000-03-01 style algorithm (Howard Hinnant), shifted so day 1 = 0001-01-01.
    let z = i64::from(day) - 1 + 306; // 0001-01-01 is 306 days after 0000-03-01
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = (yoe + era * 400) as i32 + i32::from(m <= 2);
    (y, m, d)
}

/// Inverse of [`civil_from_day`].
pub fn day_from_civil(y: i32, m: u32, d: u32) -> i32 {
    let y = i64::from(y) - i64::from(m <= 2);
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let mp = (i64::from(m) + 9) % 12;
    let doy = (153 * mp + 2) / 5 + i64::from(d) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    (era * 146_097 + doe - 306 + 1) as i32
}

fn hour12(h: i64) -> (i64, &'static str) {
    (if h % 12 == 0 { 12 } else { h % 12 }, if h < 12 { "AM" } else { "PM" })
}

/// `hh:mm AM/PM` for a time of day in milliseconds, the hour padded with a zero
/// (types/TestFormatEvaluateTextContracts.al:Format_DateTime_StandardFormats).
fn format_time_12h(ms_of_day: i64) -> String {
    let (h, m) = (ms_of_day / 3_600_000, ms_of_day / 60_000 % 60);
    let (h12, ap) = hour12(h);
    format!("{h12:02}:{m:02} {ap}")
}

/// The milliseconds of a time of day as `.fff` without trailing zeros, nothing when there are none
/// (text/TestProbeTextDateFormatContracts.al:Format_MillisecondsTimeDateTimeDuration: `.007`, `.12`, `.5`).
fn millis(ms: i64) -> String {
    if ms == 0 {
        String::new()
    } else {
        format!(".{ms:03}").trim_end_matches('0').to_string()
    }
}

/// A Time, from its milliseconds since midnight (types/TestFormatEvaluateTextContracts.al:
/// Format_Time_StandardFormats). Format 0 and 1 are a 12-hour clock whose hour is padded with a
/// space; format 1 adds the milliseconds (a time without any is not measured: refused); format 2 is
/// the AL literal; format 9 the round-trip form.
/// A standard format number the type does not have (text/TestProbeTextDateFormatContracts.al:Format_StandardNumbers_Time).
/// [`no_such_format`] where the type name the platform puts in the text is not pinned: the code is, the text is marked unpinned.
pub(crate) fn no_such_format_unpinned() -> FormatFail {
    FormatFail::Error {
        message: crate::native::unpinned_text_error(
            "InvalidStandardFormat",
            "the type name in the error of a standard format number a Text, Code, Char, Option, Integer or BigInteger lacks",
        )
        .message,
        code: "InvalidStandardFormat".into(),
    }
}

pub(crate) fn no_such_format(n: i32, ty: &str) -> FormatFail {
    FormatFail::Error {
        message: format!("Standard format number {n} does not exist for the type '{ty}'."),
        code: "InvalidStandardFormat".into(),
    }
}

fn format_time(ms_of_day: i64, standard: i32) -> Result<String, FormatFail> {
    let (h, m, s, f) = (
        ms_of_day / 3_600_000,
        ms_of_day / 60_000 % 60,
        ms_of_day / 1000 % 60,
        ms_of_day % 1000,
    );
    let (h12, ap) = hour12(h);
    Ok(match standard {
        0 => format!("{h12:>2}:{m:02}:{s:02} {ap}"),
        1 => format!("{h12:>2}:{m:02}:{s:02}{} {ap}", millis(f)),
        2 => format!("{h:02}{m:02}{s:02}{}T", millis(f)),
        9 => format!("{h:02}:{m:02}:{s:02}{}", millis(f)),
        n => return Err(no_such_format(n, "Time")),
    })
}

/// A DateTime, from its milliseconds since 0001-01-01. Formats 0 and 2 render to the minute
/// (`09/20/26 01:10 PM`, pages/record-save/TestPageSaveDataLayer.al:199-206); format 1 adds seconds and
/// milliseconds (refused without milliseconds); format 9 is the round-trip form
/// (`2026-09-20T11:10:12.851Z`, no fraction when there are no milliseconds:
/// types/TestFormatEvaluateTextContracts.al:Format_DateTime_StandardFormats). The session time zone
/// is taken to be UTC.
/// The years a short date shows with two digits (the en-US two-digit year window).
const TWO_DIGIT_YEARS: std::ops::RangeInclusive<i32> = 1950..=2049;

fn format_datetime(ms: i64, standard: i32) -> Result<String, FormatFail> {
    let day = ms.div_euclid(86_400_000);
    let in_day = ms.rem_euclid(86_400_000);
    let (y, m, d) = civil_from_day(day as i32 + 1);
    let (h, mi, s, f) = (
        in_day / 3_600_000,
        in_day / 60_000 % 60,
        in_day / 1000 % 60,
        in_day % 1000,
    );
    // The year has two digits from 1950 to 2049 and four otherwise (format 2 for 1753 to 5000, and formats 0 to 3 in 9998 and
    // 9999: types/date-time/Y49DateTime9999.al:64-209; the largest DateTime is `12/31/9999 11:59 PM`,
    // types/date-time/Format.al:377, 396).
    let date = if TWO_DIGIT_YEARS.contains(&y) {
        format!("{m:02}/{d:02}/{:02}", y.rem_euclid(100))
    } else {
        format!("{m:02}/{d:02}/{y}")
    };
    Ok(match standard {
        0 | 2 => format!("{date} {}", format_time_12h(in_day)),
        1 => {
            let (h12, ap) = hour12(h);
            format!("{date} {h12:02}:{mi:02}:{s:02}{} {ap}", millis(f))
        }
        // Format 3 shows the seconds, never the milliseconds (text/TestProbeTextDateFormatContracts.al:Format_StandardNumbers_DateTime).
        3 => {
            let (h12, ap) = hour12(h);
            format!("{date} {h12:02}:{mi:02}:{s:02} {ap}")
        }
        9 => format!("{y:04}-{m:02}-{d:02}T{h:02}:{mi:02}:{s:02}{}Z", millis(f)),
        n => return Err(no_such_format(n, "DateTime")),
    })
}

/// A Duration. Formats 0 and 1 spell the units (`1 day 1 hour 1 minute 1 second 1 millisecond`, a
/// zero unit left out, a zero duration blank, every unit negative for a negative one:
/// testpage/TestPageFieldFormats_Tests.al:599-602, testpage/TestPageRuntimeContracts.al:162);
/// format 9 is `P1DT1H1M1.001S` (types/TestFormatEvaluateTextContracts.al:Format_Duration_StandardFormats)
/// with `.0` for whole seconds (json/TestJsonRuntimeContracts.al:214, `P0DT0H0M5.0S`).
/// The words of a Duration (`1 day 1 hour`, every unit negative for a negative one, blank for zero).
/// A TestPage control shows the same text (`pagefmt`).
pub(crate) fn duration_words(ms: i64) -> String {
    if ms == 0 {
        return String::new();
    }
    let sign = if ms < 0 { "-" } else { "" };
    let mut left = ms.abs();
    let mut parts = Vec::new();
    for (unit, size) in [
        ("day", 86_400_000),
        ("hour", 3_600_000),
        ("minute", 60_000),
        ("second", 1000),
        ("millisecond", 1),
    ] {
        let n = left / size;
        left %= size;
        if n > 0 {
            parts.push(format!("{sign}{n} {unit}{}", if n == 1 { "" } else { "s" }));
        }
    }
    parts.join(" ")
}

/// The units of a Duration with their size in milliseconds, largest first.
const DURATION_UNITS: [(&str, i64); 5] = [
    ("day", 86_400_000),
    ("hour", 3_600_000),
    ("minute", 60_000),
    ("second", 1000),
    ("millisecond", 1),
];

/// `(count, unit index)` of each non-zero unit of `ms`, largest first, by absolute value.
fn duration_parts(ms: i64) -> Vec<(i64, usize)> {
    let mut left = ms.unsigned_abs() as i64;
    let mut parts = Vec::new();
    for (i, (_, size)) in DURATION_UNITS.iter().enumerate() {
        let n = left / size;
        left %= size;
        if n > 0 {
            parts.push((n, i));
        }
    }
    parts
}

/// Formats 0 and 1 of a Duration, and 2: the words of the units, a zero unit left out, blank for
/// zero, and the sign on the first unit only (types/TestDateFormulaDurationFormatContracts.al:75-120).
/// Format 2 is a list of pairs `<count> <unit code>`, the code being 0 day, 1 days, 2 hour, 3 hours,
/// 4 minute, 5 minutes, 6 second, 7 seconds, 8 millisecond, 9 milliseconds (:11-15, :83-100).
fn duration_format(ms: i64, codes: bool, lang: i32) -> Result<String, FormatFail> {
    let mut out: Vec<String> = Vec::new();
    for (k, (n, i)) in duration_parts(ms).into_iter().enumerate() {
        let sign = if k == 0 && ms < 0 { "-" } else { "" };
        let plural = n != 1;
        if codes {
            out.push(format!("{sign}{n} {}", i * 2 + usize::from(plural)));
        } else {
            let unit = unit_word(lang, i, plural).ok_or_else(|| {
                FormatFail::Unsupported(format!(
                    "Format of a Duration in language {lang}: the word for {} is not pinned (docs/probes-needed.md)",
                    DURATION_UNITS[i].0
                ))
            })?;
            out.push(format!("{sign}{n} {unit}"));
        }
    }
    Ok(out.join(" "))
}

/// The word for a Duration unit (index into `DURATION_UNITS`) in a language. English has all of them; German and French only the
/// forms the probe measured: `1 Stunde 2 Minuten 3 Sekunden` (types/operators-expressions/P142ToTextLang.al:190) and
/// `1 heure 2 minutes 3 secondes` (:214). Every other form is refused.
fn unit_word(lang: i32, unit: usize, plural: bool) -> Option<String> {
    match (lang, unit, plural) {
        (1031, 1, false) => Some("Stunde".into()),
        (1031, 2, true) => Some("Minuten".into()),
        (1031, 3, true) => Some("Sekunden".into()),
        (1036, 1, false) => Some("heure".into()),
        (1036, 2, true) => Some("minutes".into()),
        (1036, 3, true) => Some("secondes".into()),
        // format-evaluate/AxLanguageFormats.al:134-163: `1 Tag;2 Tage;2 Stunden;1 Minute;1 Sekunde;1 Millisekunde`
        // and `1 jour;2 jours;2 heures;1 minute;1 seconde;1 milliseconde`.
        (1031, 0, false) => Some("Tag".into()),
        (1031, 0, true) => Some("Tage".into()),
        (1031, 1, true) => Some("Stunden".into()),
        (1031, 2, false) => Some("Minute".into()),
        (1031, 3, false) => Some("Sekunde".into()),
        (1031, 4, false) => Some("Millisekunde".into()),
        // `7 Millisekunden` (format-evaluate/format-matrix/FMEvalDuration.al:353-433, corpus PR 1250).
        (1031, 4, true) => Some("Millisekunden".into()),
        (1036, 0, false) => Some("jour".into()),
        (1036, 0, true) => Some("jours".into()),
        (1036, 1, true) => Some("heures".into()),
        (1036, 2, false) => Some("minute".into()),
        (1036, 3, false) => Some("seconde".into()),
        (1036, 4, false) => Some("milliseconde".into()),
        (1031 | 1036, ..) => None,
        _ => Some(format!("{}{}", DURATION_UNITS[unit].0, if plural { "s" } else { "" })),
    }
}

fn format_duration(ms: i64, standard: i32, lang: i32) -> Result<String, FormatFail> {
    // A Duration of 9223372028264841218 ms raises the overflow with the default format (D_Duration.al:Duration_To_Integer_BeyondRange
    // `af128`, `Format(D)`). Up to 1.8e16 ms it formats: 922337203685478 ms, 3e15 ms and 5000000000 hours (types/date-time/
    // Evaluate.al:640-670; D_Duration.al:421-443 `big_number`). Other values above 1.8e16 ms, that value with another format and a
    // negative Duration below -922337203685477 ms (the largest .NET TimeSpan) are not pinned.
    if ms == 9_223_372_028_264_841_218 && standard == 0 {
        return Err(FormatFail::Error {
            message: "Arithmetic operation resulted in an overflow.".into(),
            code: "AL:Overflow".into(),
        });
    }
    if !(-922_337_203_685_477..=18_000_000_000_000_000).contains(&ms) {
        return Err(FormatFail::Unsupported(format!(
            "Format of a Duration of {ms} ms with format {standard} (not pinned)"
        )));
    }
    match standard {
        0 | 1 => duration_format(ms, false, lang),
        2 => duration_format(ms, true, lang),
        9 => {
            // The sign goes in front of the P (types/TestDateFormulaDurationFormatContracts.al:114-120).
            let sign = if ms < 0 { "-" } else { "" };
            let a = ms.unsigned_abs() as i64;
            let (d, h, m, s, f) = (
                a / 86_400_000,
                a / 3_600_000 % 24,
                a / 60_000 % 60,
                a / 1000 % 60,
                a % 1000,
            );
            // The fraction has three digits with the trailing zeros trimmed, one digit at least:
            // `.0`, `.5` (:88), `.001`, `.012` (:97-103).
            let frac = format!("{f:03}");
            let frac = frac.trim_end_matches('0');
            let frac = if frac.is_empty() { "0" } else { frac };
            Ok(format!("{sign}P{d}DT{h}H{m}M{s}.{frac}S"))
        }
        n => Err(no_such_format(n, "Duration")),
    }
}

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

// The month names of AxLanguageFormats.al:79-131 (formats 4 and 7 in German, French and Spanish).
pub(crate) const MONTHS_DE: [&str; 12] = [
    "Januar",
    "Februar",
    "März",
    "April",
    "Mai",
    "Juni",
    "Juli",
    "August",
    "September",
    "Oktober",
    "November",
    "Dezember",
];
const MONTHS_DE_SHORT: [&str; 12] = [
    "Jan.", "Feb.", "März", "Apr.", "Mai", "Juni", "Juli", "Aug.", "Sept.", "Okt.", "Nov.", "Dez.",
];
pub(crate) const MONTHS_FR: [&str; 12] = [
    "janvier",
    "février",
    "mars",
    "avril",
    "mai",
    "juin",
    "juillet",
    "août",
    "septembre",
    "octobre",
    "novembre",
    "décembre",
];
const MONTHS_FR_SHORT: [&str; 12] = [
    "janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.", "nov.", "déc.",
];
const MONTHS_ES: [&str; 12] = [
    "enero",
    "febrero",
    "marzo",
    "abril",
    "mayo",
    "junio",
    "julio",
    "agosto",
    "septiembre",
    "octubre",
    "noviembre",
    "diciembre",
];
const MONTHS_ES_SHORT: [&str; 12] = [
    "ene", "feb", "mar", "abr", "may", "jun", "jul", "ago", "sept", "oct", "nov", "dic",
];

/// Dutch month names of formats 4 and 7, the same in 1043 and 2067 (assumption month-names-dutch: MonthNamesLanguages.al
/// Months_Dutch_Belgium_2067, Months_Dutch_1043).
const MONTHS_NL: [&str; 12] = [
    "januari",
    "februari",
    "maart",
    "april",
    "mei",
    "juni",
    "juli",
    "augustus",
    "september",
    "oktober",
    "november",
    "december",
];
const MONTHS_NL_SHORT: [&str; 12] = [
    "jan", "feb", "mrt", "apr", "mei", "jun", "jul", "aug", "sep", "okt", "nov", "dec",
];

/// The standard formats of a Date, en-US (types/TestFormatEvaluateTextContracts.al:Format_Date_StandardFormats):
/// 0 and 1 `mm/dd/yy`, 2 `mmddyyD`, 3 `yy/mm/dd`, 4 `March 5, 2024`, 5 `mmddyy`, 6 `yymmdd`,
/// 7 `Mar 5, 2024`, 9 `yyyy-mm-dd`. Number 8 does not exist
/// (:Format_Date_Format8_DoesNotExist). A closing date is marked with `C` in the formats the corpus
/// shows it in (0, 1 and 2); the others are not pinned for one.
fn format_date(day: i32, closing: bool, standard: i32, lang: i32) -> Result<String, FormatFail> {
    let (y, m, d) = civil_from_day(day);
    let yy = y.rem_euclid(100);
    let c = if closing { "C" } else { "" };
    if closing && !matches!(standard, 0 | 1 | 2 | 9) {
        return Err(format!("Format of a closing date with standard format {standard} (not pinned)").into());
    }
    // The month names of formats 4 and 7 follow the language: English, German, French and Spanish are pinned for every month
    // (format-evaluate/AxLanguageFormats.al:52-132; P142ToTextLang.al:109,117). Another language is refused.
    let month = match (lang, standard) {
        (1033, _) => MONTHS[m as usize - 1],
        (1031, 4) => MONTHS_DE[m as usize - 1],
        (1031, 7) => MONTHS_DE_SHORT[m as usize - 1],
        (1036, 4) => MONTHS_FR[m as usize - 1],
        (1036, 7) => MONTHS_FR_SHORT[m as usize - 1],
        (3082, 4) => MONTHS_ES[m as usize - 1],
        (3082, 7) => MONTHS_ES_SHORT[m as usize - 1],
        // Danish December, format 4 only (format-evaluate/EvalLanguageBoolean.al EL_Date_DAN_1030: `december 31, 2026`); the other
        // Danish months are not pinned. Dutch (1043, 2067) December of format 4 is pinned there too (EL_Date_NLD_1043, EL_Date_NLB_2067).
        (1030, 4) if m == 12 => "december",
        (1043 | 2067, 4) if m == 12 => MONTHS_NL[11],
        // Every Dutch month of formats 4 and 7: measured by the corpus probe, not pinned yet (assumption month-names-dutch).
        (1043 | 2067, 4) if crate::assumptions::hit(crate::assumptions::Assumption::MonthNamesDutch) => {
            MONTHS_NL[m as usize - 1]
        }
        (1043 | 2067, 7) if crate::assumptions::hit(crate::assumptions::Assumption::MonthNamesDutch) => {
            MONTHS_NL_SHORT[m as usize - 1]
        }
        (_, 4 | 7) => {
            return Err(FormatFail::Unsupported(format!(
                "Format {standard} of a Date in language {lang}, month {m}: the month name is not pinned (docs/probes-needed.md)"
            )));
        }
        _ => MONTHS[m as usize - 1],
    };
    let short = |month: &str| {
        if lang == 1033 {
            month[..3].to_string()
        } else {
            month.to_string()
        }
    };
    Ok(match standard {
        // The year has two digits from 1950 to 2049 and four otherwise
        // (pages/controls/W2ValueControls.al:Date_SessionFormatTwoDigitYear_*: 1929 and 1930 give 10 characters, 1999
        // to 2030 give 8, BC 27.5 and 29.0; format 1: 1949 and 2050 four digits, 1950 and 2049 two,
        // types/date-time/Y49DateTime9999.al:172-209).
        0 | 1 if TWO_DIGIT_YEARS.contains(&y) => format!("{c}{m:02}/{d:02}/{yy:02}"),
        0 | 1 => format!("{c}{m:02}/{d:02}/{y:04}"),
        // Format 2 too: 1753-01-01 and 9999-12-31 give `01011753D` and `12319999D` (format-evaluate/format-matrix/FMDateTime.al:599,
        // :721, corpus PR 1250); the two-digit form from 1950 to 2049 is the one format 0 has.
        2 if TWO_DIGIT_YEARS.contains(&y) => format!("{m:02}{d:02}{yy:02}{c}D"),
        2 => format!("{m:02}{d:02}{y:04}{c}D"),
        3 => format!("{yy:02}/{m:02}/{d:02}"),
        // The year has four digits: `June 15, 0001` (types/Y81DMY2Date.al:94-114 Y81_DMY2Date_Year1_Arithmetic
        // `format 4`; needs corpus bump).
        4 => format!("{month} {d}, {y:04}"),
        5 => format!("{m:02}{d:02}{yy:02}"),
        6 => format!("{yy:02}{m:02}{d:02}"),
        7 => format!("{} {d}, {y}", short(month)),
        9 => format!("{y:04}-{m:02}-{d:02}"),
        n => return Err(no_such_format(n, "Date")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `Format` for a value the tests expect to be pinned.
    fn format(v: &Value, length: i32, standard: i32, ctx: &dyn FormatCtx) -> String {
        try_format(v, length, standard, ctx).unwrap_or_else(|e| panic!("{e}"))
    }
    use rust_decimal::Decimal;
    use std::rc::Rc;
    use std::str::FromStr;

    struct Ctx;
    impl FormatCtx for Ctx {
        fn enum_name(&self, _id: u32, ordinal: i32) -> String {
            ["Open", "In Progress"][ordinal as usize].to_string()
        }
    }

    fn f(v: Value) -> String {
        format(&v, 0, 0, &Ctx)
    }
    fn f2(v: Value) -> String {
        format(&v, 0, 2, &Ctx)
    }
    fn dec(s: &str) -> Value {
        Value::Decimal(Decimal::from_str(s).unwrap())
    }

    #[test]
    fn dutch_month_names_of_formats_4_and_7() {
        // Probe MonthNamesLanguages.al Months_Dutch_Belgium_2067 and Months_Dutch_1043 (assumption month-names-dutch).
        let day = |m: u32, d: u32| day_from_civil(2026, m, d);
        let long = [
            "januari",
            "februari",
            "maart",
            "april",
            "mei",
            "juni",
            "juli",
            "augustus",
            "september",
            "oktober",
        ];
        let short = [
            "jan", "feb", "mrt", "apr", "mei", "jun", "jul", "aug", "sep", "okt", "nov", "dec",
        ];
        for lang in [2067, 1043] {
            for (i, (l, s)) in long.iter().zip(short).enumerate() {
                let d = day(i as u32 + 1, 5);
                assert_eq!(format_date(d, false, 4, lang).unwrap(), format!("{l} 5, 2026"));
                assert_eq!(format_date(d, false, 7, lang).unwrap(), format!("{s} 5, 2026"));
            }
            assert_eq!(format_date(day(11, 5), false, 4, lang).unwrap(), "november 5, 2026");
            assert_eq!(format_date(day(12, 5), false, 7, lang).unwrap(), "dec 5, 2026");
            assert_eq!(format_date(day(3, 15), false, 7, lang).unwrap(), "mrt 15, 2026");
        }
    }

    #[test]
    fn integers() {
        // text/TestFormat.al: Format(42) = '42', Format(-7) = '-7', Format(0) = '0'.
        assert_eq!(f(Value::Integer(42)), "42");
        assert_eq!(f(Value::Integer(-7)), "-7");
        assert_eq!(f(Value::Integer(0)), "0");
        assert_eq!(f2(Value::BigInteger(-567)), "-567");
    }

    #[test]
    fn length_right_aligns() {
        // text/TestFormat.al: Format(42, 5) = '   42'.
        assert_eq!(format(&Value::Integer(42), 5, 0, &Ctx), "   42");
    }

    #[test]
    fn booleans() {
        // text/TestFormat.al: Format(true) = 'Yes', Format(false) = 'No' (the doc's
        // True/False is wrong for BC); <Standard Format,2> gives 1/0.
        assert_eq!(f(Value::Boolean(true)), "Yes");
        assert_eq!(f(Value::Boolean(false)), "No");
        assert_eq!(f2(Value::Boolean(true)), "1");
        assert_eq!(f2(Value::Boolean(false)), "0");
        // Doc: standard format 9 is true/false.
        assert_eq!(format(&Value::Boolean(true), 0, 9, &Ctx), "true");
    }

    #[test]
    fn text_and_code_are_themselves() {
        assert_eq!(f(Value::Text("ABC".into())), "ABC");
        assert_eq!(f2(Value::Code("X1".into())), "X1");
    }

    #[test]
    fn undefined_dates_and_times_are_empty() {
        // types/TestDate*.al etc.: Format(0D) = Format(0T) = Format(0DT) = ''.
        assert_eq!(f(Value::Date(0, false)), "");
        assert_eq!(f(Value::Time(0)), "");
        assert_eq!(f(Value::DateTime(0)), "");
        assert_eq!(f2(Value::Date(0, false)), "");
    }

    #[test]
    fn options_and_enums() {
        // record tests: Format(option 1 of " ,Draft") = 'Draft'; quoting stripped.
        let members: std::sync::Arc<[String]> = std::sync::Arc::from(vec!["".to_string(), "Draft".to_string()]);
        assert_eq!(f(Value::Option(1, members.clone())), "Draft");
        assert_eq!(
            f2(Value::Option(1, members)),
            "1",
            "doc: option standard format 2 is the number"
        );
        assert_eq!(f(Value::Enum(7, 1)), "In Progress");
        assert_eq!(f2(Value::Enum(7, 1)), "1");
    }

    #[test]
    fn guids_are_upper_case_in_braces() {
        // Corpus: '{63CA2FA4-4F03-4F2B-A480-172FEF340D3F}'.
        let g = [
            0x63, 0xca, 0x2f, 0xa4, 0x4f, 0x03, 0x4f, 0x2b, 0xa4, 0x80, 0x17, 0x2f, 0xef, 0x34, 0x0d, 0x3f,
        ];
        assert_eq!(f(Value::Guid(g)), "{63CA2FA4-4F03-4F2B-A480-172FEF340D3F}");
        assert_eq!(f2(Value::Guid(g)), "{63CA2FA4-4F03-4F2B-A480-172FEF340D3F}");
    }

    #[test]
    fn calendar_round_trips() {
        for (y, m, d) in [(1, 1, 1), (2021, 4, 5), (2000, 2, 29), (1900, 3, 1), (9999, 12, 31)] {
            assert_eq!(civil_from_day(day_from_civil(y, m, d)), (y, m, d));
        }
        assert_eq!(day_from_civil(1, 1, 1), 1);
    }

    #[test]
    fn dates_per_doc_table() {
        let day = day_from_civil(2021, 4, 5);
        assert_eq!(f2(Value::Date(day, false)), "040521D");
        assert_eq!(format(&Value::Date(day, false), 0, 9, &Ctx), "2021-04-05");
    }

    #[test]
    fn date_times_in_the_two_standard_formats_the_corpus_measured() {
        // pages/record-save/TestPageSaveDataLayer.al:199-206 (measured on BC): Format(_, 0, 2) of a DateTime
        // renders to the minute ('09/20/26 01:10 PM'); Format(_, 0, 9) is the round-trip form
        // ('2026-09-20T11:10:12.851Z'). The session time zone is taken to be UTC here.
        let day = i64::from(day_from_civil(2026, 9, 20)) - 1;
        let ms = day * 86_400_000 + (13 * 3600 + 10 * 60 + 12) * 1000 + 851;
        assert_eq!(f2(Value::DateTime(ms + 1)), "09/20/26 01:10 PM");
        assert_eq!(format(&Value::DateTime(ms + 1), 0, 9, &Ctx), "2026-09-20T13:10:12.851Z");
        let midnight = day * 86_400_000;
        assert_eq!(f2(Value::DateTime(midnight + 1)), "09/20/26 12:00 AM");
        // Two instants in the same minute format alike: Assert compares DateTimes at minute resolution.
        assert_eq!(f2(Value::DateTime(ms + 1)), f2(Value::DateTime(ms - 5_000 + 1)));
        assert_ne!(f2(Value::DateTime(ms + 1)), f2(Value::DateTime(ms + 120_000 + 1)));
    }

    #[test]
    fn times_are_distinct_per_minute_in_format_2() {
        // Assert.AreEqual/AreNotEqual compare Times through Format(_, 0, 2) (Assert.al Equal);
        // types/TestFormatEvaluateTextContracts.al:Format_Time_StandardFormats: format 2 is the AL literal.
        assert_eq!(f2(Value::Time(13 * 3_600_000 + 10 * 60_000 + 1)), "131000T");
        assert_eq!(f2(Value::Time(1)), "000000T");
    }

    #[test]
    fn language_changes_month_names_duration_words_and_booleans_as_measured() {
        // types/operators-expressions/P142ToTextLang.al:109,117,153,190,214
        struct L(i32);
        impl FormatCtx for L {
            fn enum_name(&self, _: u32, o: i32) -> String {
                o.to_string()
            }
            fn language(&self) -> i32 {
                self.0
            }
        }
        let d = Value::Date(day_from_civil(2024, 3, 15), false);
        assert_eq!(format_value(&d, 0, 4, &L(1031)).unwrap(), "März 15, 2024");
        assert_eq!(format_value(&d, 0, 4, &L(1036)).unwrap(), "mars 15, 2024");
        assert_eq!(format_value(&d, 0, 0, &L(1036)).unwrap(), "03/15/24");
        let jan = Value::Date(day_from_civil(2024, 1, 15), false);
        // format-evaluate/AxLanguageFormats.al:79.
        assert_eq!(format_value(&jan, 0, 4, &L(1031)).unwrap(), "Januar 15, 2024");
        assert!(matches!(
            format_value(&d, 0, 4, &L(1030)),
            Err(FormatFail::Unsupported(_))
        ));
        let du = Value::Duration(3_723_000);
        assert_eq!(
            format_value(&du, 0, 0, &L(1031)).unwrap(),
            "1 Stunde 2 Minuten 3 Sekunden"
        );
        assert_eq!(
            format_value(&du, 0, 0, &L(1036)).unwrap(),
            "1 heure 2 minutes 3 secondes"
        );
        assert_eq!(format_value(&du, 0, 0, &L(1033)).unwrap(), "1 hour 2 minutes 3 seconds");
        // AxLanguageFormats.al:151; the plural of a millisecond in German: format-matrix/FMEvalDuration.al:353.
        assert_eq!(
            format_value(&Value::Duration(86_400_000), 0, 0, &L(1031)).unwrap(),
            "1 Tag"
        );
        assert_eq!(
            format_value(&Value::Duration(7), 0, 0, &L(1031)).unwrap(),
            "7 Millisekunden"
        );
        assert!(matches!(
            format_value(&Value::Duration(2), 0, 0, &L(1036)),
            Err(FormatFail::Unsupported(_))
        ));
        assert_eq!(format_value(&Value::Boolean(false), 0, 0, &L(1036)).unwrap(), "Non");
        assert_eq!(format_value(&Value::Boolean(true), 0, 9, &L(1036)).unwrap(), "true");
    }

    #[test]
    fn a_format_region_rewrites_date_and_decimal_and_refuses_the_rest() {
        // reports/PnC06ReportLanguage.al:Language_SetAndRead
        struct R;
        impl FormatCtx for R {
            fn enum_name(&self, _: u32, o: i32) -> String {
                o.to_string()
            }
            fn region(&self) -> Option<String> {
                Some("de-DE".into())
            }
        }
        let d = Value::Date(day_from_civil(2024, 3, 5), false);
        assert_eq!(format_value(&d, 0, 0, &R).unwrap(), "05.03.24");
        assert_eq!(format_value(&dec("1234.5"), 0, 0, &R).unwrap(), "1.234,5");
        assert_eq!(format_value(&Value::Integer(1033), 0, 0, &R).unwrap(), "1033");
        assert!(matches!(
            format_value(&Value::Time(5), 0, 0, &R),
            Err(FormatFail::Unsupported(_))
        ));
        // A long date with a one-digit day is pinned (BdRegionTests.al:131 `5. March 2024`).
        assert_eq!(format_value(&d, 0, 4, &R).unwrap(), "5. March 2024");
    }

    #[test]
    fn the_report_format_regions_the_corpus_pins() {
        // reports/runtime/YReportPropsTests.al:34-148 (31 January 2025 14:30:15, 1234567.891, 1234567) and
        // reports/PnH4CurrReport.al:241-249 (5 March 2024, 1234.5, true).
        struct R(&'static str);
        impl FormatCtx for R {
            fn enum_name(&self, _: u32, o: i32) -> String {
                o.to_string()
            }
            fn region(&self) -> Option<String> {
                Some(self.0.into())
            }
        }
        let day = Value::Date(day_from_civil(2025, 1, 31), false);
        let stamp = Value::DateTime((i64::from(day_from_civil(2025, 1, 31)) - 1) * 86_400_000 + 52_215_000 + 1);
        let amt = dec("1234567.891");
        let row = |r: &'static str| {
            [
                format(&amt, 0, 0, &R(r)),
                format(&day, 0, 0, &R(r)),
                format(&stamp, 0, 0, &R(r)),
                format(&day, 0, 4, &R(r)),
                format(&Value::Integer(1234567), 0, 0, &R(r)),
            ]
            .join("|")
        };
        assert_eq!(
            row("de-DE"),
            "1.234.567,891|31.01.25|31.01.25 14:30|31. January 2025|1234567"
        );
        assert_eq!(
            row("en-GB"),
            "1,234,567.891|31/01/25|31/01/25 14:30|31 January 2025|1234567"
        );
        assert_eq!(
            row("fr-FR"),
            "1\u{A0}234\u{A0}567,891|31/01/25|31/01/25 14:30|31 January 2025|1234567"
        );
        assert_eq!(
            row("ja-JP"),
            "1,234,567.891|25/01/31|25/01/31 14:30|2025年1月31日|1234567"
        );
        assert_eq!(
            row("zz-ZZ"),
            "1,234,567.891|01/31/25|01/31/25 02:30 PM|January 31, 2025|1234567"
        );
        let small = dec("1234.5");
        let march = Value::Date(day_from_civil(2024, 3, 5), false);
        for (r, want) in [
            ("de-DE", "1.234,5|05.03.24|Yes"),
            ("fr-FR", "1\u{A0}234,5|05/03/24|Yes"),
            ("da-DK", "1.234,5|05-03-24|Yes"),
        ] {
            let got = [&small, &march, &Value::Boolean(true)]
                .map(|v| format(v, 0, 0, &R(r)))
                .join("|");
            assert_eq!(got, want, "{r}");
        }
        // Neighbours no test pins stay refused: a region that is not in the table and a Time with milliseconds. The texts of en-GB,
        // da-DK and nl-NL are the same on BC 27 and 28 (BdRegionTests.al:1506-1710 Y89_Old_*).
        let refused = |v: &Value, n: i32, r: &'static str| {
            matches!(format_value(v, 0, n, &R(r)), Err(FormatFail::Unsupported(_)))
        };
        assert!(refused(&amt, 0, "pt-PT"));
        assert!(refused(&amt, 0, "en-gb"));
        assert!(!refused(&amt, 0, "nl-NL"));
        assert!(refused(&Value::Time(5), 0, "fr-FR"));
    }

    #[test]
    fn a_zz_zz_dataset_column_pads_month_and_day() {
        // reports/runtime/YReportPropsTests.al:128 (`01/31/2025`, `01/31/2025 14:30:15`, BC 27, 28, 29) and
        // reports/runtime/BdRegionTests.al:213,220 (`03/05/2024`).
        let day = Value::Date(day_from_civil(2025, 1, 31), false);
        let stamp = Value::DateTime((i64::from(day_from_civil(2025, 1, 31)) - 1) * 86_400_000 + 52_215_000 + 1);
        let march = Value::Date(day_from_civil(2024, 3, 5), false);
        assert_eq!(region_column(Region::ZzZZ, &day).as_deref(), Some("01/31/2025"));
        assert_eq!(
            region_column(Region::ZzZZ, &stamp).as_deref(),
            Some("01/31/2025 14:30:15")
        );
        assert_eq!(region_column(Region::ZzZZ, &march).as_deref(), Some("03/05/2024"));
    }

    #[test]
    fn fr_fr_groups_with_a_no_break_space_on_every_version() {
        // reports/PnH4CurrReport.al:247 and :311 and reports/runtime/YReportPropsTests.al:74: U+00A0 on
        // BC 27, 28 and 29 (corpus bump 62; the older Linux image gave U+202F on BC 27 and 28).
        struct R;
        impl FormatCtx for R {
            fn enum_name(&self, _: u32, o: i32) -> String {
                o.to_string()
            }
            fn region(&self) -> Option<String> {
                Some("fr-FR".into())
            }
        }
        assert_eq!(format(&dec("1234567.891"), 0, 0, &R), "1\u{A0}234\u{A0}567,891");
        assert_eq!(format(&dec("1234.5"), 0, 0, &R), "1\u{A0}234,5");
    }

    #[test]
    fn the_standard_formats_the_probe_pins() {
        // types/TestFormatEvaluateTextContracts.al:Format_Decimal_StandardFormats, Format_Date_StandardFormats,
        // Format_Time_StandardFormats, Format_DateTime_StandardFormats, Format_Duration_StandardFormats.
        let fmt = |v: Value, n: i32| format(&v, 0, n, &Ctx);
        assert_eq!(fmt(dec("1234567.891"), 0), "1,234,567.891");
        for n in [1, 2, 9] {
            assert_eq!(fmt(dec("1234567.891"), n), "1234567.891");
        }
        let day = Value::Date(day_from_civil(2024, 3, 5), false);
        let want = [
            "03/05/24",
            "03/05/24",
            "030524D",
            "24/03/05",
            "March 5, 2024",
            "030524",
            "240305",
            "Mar 5, 2024",
        ];
        for (n, w) in want.iter().enumerate() {
            assert_eq!(fmt(day.clone(), n as i32), *w, "date format {n}");
        }
        assert_eq!(fmt(day.clone(), 9), "2024-03-05");
        let Err(FormatFail::Error { message, code }) = format_value(&day, 0, 8, &Ctx) else {
            panic!("format 8 must raise");
        };
        assert_eq!(message, "Standard format number 8 does not exist for the type 'Date'.");
        assert_eq!(code, "InvalidStandardFormat");
        let time = Value::Time(52_205_124);
        assert_eq!(fmt(time.clone(), 0), " 2:30:05 PM");
        assert_eq!(fmt(time.clone(), 1), " 2:30:05.123 PM");
        assert_eq!(fmt(time.clone(), 2), "143005.123T");
        assert_eq!(fmt(Value::Time(86_399_001), 2), "235959T");
        assert_eq!(fmt(time, 9), "14:30:05.123");
        assert_eq!(fmt(Value::Time(1), 0), "12:00:00 AM");
        let dt = Value::DateTime((i64::from(day_from_civil(2024, 3, 5)) - 1) * 86_400_000 + 52_205_123 + 1);
        assert_eq!(fmt(dt.clone(), 0), "03/05/24 02:30 PM");
        assert_eq!(fmt(dt.clone(), 1), "03/05/24 02:30:05.123 PM");
        assert_eq!(fmt(dt.clone(), 9), "2024-03-05T14:30:05.123Z");
        let midnight = Value::DateTime((i64::from(day_from_civil(2024, 3, 5)) - 1) * 86_400_000 + 1);
        assert_eq!(fmt(midnight, 9), "2024-03-05T00:00:00Z");
        let dur = Value::Duration(90_061_001);
        assert_eq!(fmt(dur.clone(), 0), "1 day 1 hour 1 minute 1 second 1 millisecond");
        assert_eq!(fmt(dur, 9), "P1DT1H1M1.001S");
        // json/TestJsonRuntimeContracts.al:214.
        assert_eq!(fmt(Value::Duration(5000), 9), "P0DT0H0M5.0S");
        // types/TestDateFormulaDurationFormatContracts.al:114-120: Format carries the sign on the
        // first unit only (a TestPage control shows every unit negative, testpage/TestPageRuntimeContracts.al:162).
        assert_eq!(
            fmt(Value::Duration(-90_061_001), 0),
            "-1 day 1 hour 1 minute 1 second 1 millisecond"
        );
    }

    // types/TestDateFormulaDurationFormatContracts.al:75-120: formats 0, 1, 2 and 9 of a Duration.
    #[test]
    fn duration_standard_formats_0_1_2_and_9() {
        let fmt = |v: Value, n: i32| format(&v, 0, n, &Ctx);
        let all = |ms: i64| {
            format!(
                "{} | {} | {} | {}",
                fmt(Value::Duration(ms), 0),
                fmt(Value::Duration(ms), 1),
                fmt(Value::Duration(ms), 2),
                fmt(Value::Duration(ms), 9)
            )
        };
        assert_eq!(all(0), " |  |  | P0DT0H0M0.0S");
        assert_eq!(all(1), "1 millisecond | 1 millisecond | 1 8 | P0DT0H0M0.001S");
        assert_eq!(all(999), "999 milliseconds | 999 milliseconds | 999 9 | P0DT0H0M0.999S");
        assert_eq!(
            all(1500),
            "1 second 500 milliseconds | 1 second 500 milliseconds | 1 6 500 9 | P0DT0H0M1.5S"
        );
        assert_eq!(all(120_000), "2 minutes | 2 minutes | 2 5 | P0DT0H2M0.0S");
        assert_eq!(
            all(86_399_999),
            "23 hours 59 minutes 59 seconds 999 milliseconds | 23 hours 59 minutes 59 seconds 999 milliseconds | 23 3 59 5 59 7 999 9 | P0DT23H59M59.999S"
        );
        assert_eq!(
            all(172_800_005),
            "2 days 5 milliseconds | 2 days 5 milliseconds | 2 1 5 9 | P2DT0H0M0.005S"
        );
        assert_eq!(
            all(123_456_789_012),
            "1428 days 21 hours 33 minutes 9 seconds 12 milliseconds | 1428 days 21 hours 33 minutes 9 seconds 12 milliseconds | 1428 1 21 3 33 5 9 7 12 9 | P1428DT21H33M9.012S"
        );
        assert_eq!(all(-1), "-1 millisecond | -1 millisecond | -1 8 | -P0DT0H0M0.001S");
        assert_eq!(all(-1000), "-1 second | -1 second | -1 6 | -P0DT0H0M1.0S");
        assert_eq!(
            all(-90_061_001),
            "-1 day 1 hour 1 minute 1 second 1 millisecond | -1 day 1 hour 1 minute 1 second 1 millisecond | -1 0 1 2 1 4 1 6 1 8 | -P1DT1H1M1.001S"
        );
    }

    #[test]
    fn a_date_formula_prints_without_brackets() {
        // types/TestDateRoundFormatStringContracts.al:DateFormula_Evaluate_ThenFormat_PrintsTheNormalizedFormula.
        assert_eq!(f(Value::DateFormula("<1D>".into())), "1D");
        assert_eq!(f(Value::DateFormula("<+1M>".into())), "+1M");
        assert_eq!(f(Value::DateFormula("1D+1M".into())), "1D+1M");
    }

    #[test]
    fn decimals_in_code_constant_format() {
        // Doc: format 2 is <Sign><Integer><Decimals> with '.' as the separator.
        assert_eq!(f2(dec("-76543.21")), "-76543.21");
        assert_eq!(f2(dec("5")), "5");
        assert_eq!(f2(dec("1.50")), "1.5");
    }
    // Mutation-testing additions (docs/status.md debt).

    #[test]
    fn a_length_that_is_not_positive_or_not_longer_changes_nothing() {
        assert_eq!(pad("42".into(), 0), "42");
        assert_eq!(pad("42".into(), -1), "42");
        assert_eq!(pad("42".into(), -100), "42");
        assert_eq!(pad("42".into(), 2), "42");
        assert_eq!(pad("42".into(), 1), "42");
        assert_eq!(pad("42".into(), 3), " 42");
    }

    #[test]
    fn thousands_are_grouped_by_threes_from_the_right() {
        for (d, want) in [
            ("1", "1"),
            ("12", "12"),
            ("123", "123"),
            ("1234", "1,234"),
            ("12345", "12,345"),
            ("123456", "123,456"),
            ("1234567.89", "1,234,567.89"),
            ("-1234", "-1,234"),
            ("-999", "-999"),
        ] {
            assert_eq!(f(dec(d)), want, "{d}");
        }
        // Formats 2 and 9 never group.
        assert_eq!(f2(dec("1234567.5")), "1234567.5");
    }

    #[test]
    fn a_datetime_shows_its_milliseconds_exactly() {
        // The value is one more than the milliseconds since the start of 0001-01-01.
        let at = |ms: i64| format(&Value::DateTime(ms + 1), 0, 9, &Ctx);
        assert!(at(0).ends_with("00:00:00Z"), "{}", at(0));
        assert!(at(7).ends_with("00:00:00.007Z"), "{}", at(7));
        assert!(at(86_399_999).ends_with("23:59:59.999Z"), "{}", at(86_399_999));
        assert!(at(86_400_000).ends_with("00:00:00Z"), "{}", at(86_400_000));
    }

    #[test]
    fn a_library_value_without_a_pinned_format_is_refused() {
        let v = Value::Library("Foo".into(), Rc::default());
        assert!(try_format(&v, 0, 0, &Ctx).is_err());
        // A RecordId that holds nothing is empty, not an error.
        let r = Value::Library("RecordId".into(), Rc::default());
        assert_eq!(try_format(&r, 0, 0, &Ctx).unwrap(), "");
    }

    #[test]
    fn a_time_is_one_more_than_its_milliseconds_since_midnight() {
        // 12:00:00.000 is Time(43_200_001); the millisecond before it is still 11:59 AM.
        assert_eq!(f2(Value::Time(43_200_000)), "115959.999T");
        assert_eq!(f2(Value::Time(43_200_001)), "120000T");
        assert_eq!(f2(Value::Time(1)), "000000T");
        assert_eq!(f2(Value::Time(86_400_000)), "235959.999T");
    }
    // D_Duration.al:Duration_To_Integer_BeyondRange `af128`; types/date-time/Evaluate.al:640-670.
    #[test]
    fn duration_overflow_only_where_pinned() {
        use super::format_duration;
        assert!(matches!(
            format_duration(9_223_372_028_264_841_218, 0, 1033),
            Err(FormatFail::Error { .. })
        ));
        assert!(format_duration(922_337_203_685_478, 0, 1033).is_ok());
        assert!(format_duration(18_000_000_000_000_000, 9, 1033).is_ok());
        for (ms, n) in [
            (9_223_372_028_264_841_218, 9),
            (9_223_372_028_264_841_219, 0),
            (i64::MAX, 0),
            (18_000_000_000_000_001, 0),
            (-922_337_203_685_478, 0),
        ] {
            assert!(
                matches!(format_duration(ms, n, 1033), Err(FormatFail::Unsupported(_))),
                "{ms} {n}"
            );
        }
    }

    // format-evaluate/AxLanguageFormats.al:52-163.
    #[test]
    fn month_names_and_duration_words_in_other_languages() {
        use super::{day_from_civil, duration_format, format_date};
        let sep = day_from_civil(2024, 9, 1);
        let f = |lang, n| format_date(sep, false, n, lang).ok().unwrap();
        assert_eq!(f(1031, 7), "Sept. 1, 2024");
        assert_eq!(f(1036, 4), "septembre 1, 2024");
        assert_eq!(f(3082, 7), "sept 1, 2024");
        assert_eq!(f(1033, 7), "Sep 1, 2024");
        assert_eq!(duration_format(172_800_000, false, 1031).ok().unwrap(), "2 Tage");
        assert_eq!(duration_format(1, false, 1036).ok().unwrap(), "1 milliseconde");
    }

    // format-evaluate/EvalLanguageBoolean.al:1063 WA_BlankEnum1_ES_1034: `Primero` is the ESP entry, read under 1034.
    #[test]
    fn spanish_caption_is_read_under_1034() {
        assert_eq!(
            ml_text_in_language("ENU = 'First', ESP = 'Primero'", 1034).unwrap(),
            "Primero"
        );
    }
}
