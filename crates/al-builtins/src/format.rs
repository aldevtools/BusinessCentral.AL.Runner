//! `Format(Value [, Length] [, FormatNumber])`. Every rule cites the corpus
//! test or the doc table ("Formatting values, dates, and time") that pins it;
//! where the two disagree the corpus wins (it runs on real BC).

use crate::native::Signal;
use crate::value::Value;

/// What formatting needs from the running program.
pub trait FormatCtx {
    /// Member name of enum `id` at `ordinal`.
    fn enum_name(&self, id: u32, ordinal: i32) -> String;
    /// The language `GlobalLanguage` names (1033 until set).
    fn language(&self) -> i32 {
        1033
    }
    /// The format region a report set with `CurrReport.FormatRegion` while it runs (None: the session's own).
    fn region(&self) -> Option<String> {
        None
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
    fn record_id_table(&self, table: u32) -> Option<(String, Option<String>)> {
        self.0.record_id_table(table)
    }
    fn format_gate(&self, gate: al_version::Gate) -> bool {
        self.0.format_gate(gate)
    }
}

/// The format regions whose texts the corpus pins (exact spelling; any other region is refused).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Region {
    DeDE,
    EnGB,
    FrFR,
    JaJP,
    DaDK,
    /// `zz-ZZ`, a region no culture has: en-US texts, but `MM/dd/yyyy` dates and a 24-hour clock in a dataset column
    /// (report/lt-y-report-props/YReportPropsTests.al:FormatRegion_UnknownRegion_XmlOrError).
    ZzZZ,
}

impl Region {
    /// The region a `FormatRegion` text names, `None` when it is not pinned.
    pub fn parse(name: &str) -> Option<Region> {
        Some(match name {
            "de-DE" => Region::DeDE,
            "en-GB" => Region::EnGB,
            "fr-FR" => Region::FrFR,
            "ja-JP" => Region::JaJP,
            "da-DK" => Region::DaDK,
            "zz-ZZ" => Region::ZzZZ,
            _ => return None,
        })
    }

    /// The group separator and the decimal separator of an amount. fr-FR groups with U+202F on BC 27 and 28
    /// (PnH4CurrReport.al:252 `1 234,5`, bytes E2 80 AF; YReportPropsTests.al:80 shows the same bytes read back through code page 437)
    /// and with U+00A0 from BC 29 (PnH4CurrReport.al:250, YReportPropsTests.al:78; `no_break_space`).
    fn separators(self, no_break_space: bool) -> (char, char) {
        match self {
            Region::DeDE | Region::DaDK => ('.', ','),
            Region::FrFR if no_break_space => ('\u{A0}', ','),
            Region::FrFR => ('\u{202F}', ','),
            Region::EnGB | Region::JaJP | Region::ZzZZ => (',', '.'),
        }
    }

    /// A date as `Format` (two-digit year) or a dataset column (four-digit year) writes it, `None` when the
    /// shape is not pinned. Day and month are padded where a test measured it: month 01 in every region
    /// (YReportPropsTests.al:34-148), day 05 in de-DE, fr-FR and da-DK `Format` (PnH4CurrReport.al:241-249).
    fn date(self, y: i32, m: u32, d: u32, column: bool) -> Option<String> {
        // Day 05 in a zz-ZZ dataset column: BdRegionTests.al:213,220 `03/05/2024`.
        let day_padding_pinned =
            (!column && matches!(self, Region::DeDE | Region::FrFR | Region::DaDK)) || (column && self == Region::ZzZZ);
        if d < 10 && !day_padding_pinned {
            return None;
        }
        let year = if column {
            format!("{y:04}")
        } else {
            format!("{:02}", y.rem_euclid(100))
        };
        if !(1000..=9999).contains(&y) {
            return None;
        }
        Some(match self {
            Region::DeDE => format!("{d:02}.{m:02}.{year}"),
            Region::EnGB | Region::FrFR => format!("{d:02}/{m:02}/{year}"),
            Region::JaJP => format!("{year}/{m:02}/{d:02}"),
            // da-DK is measured through `Format` only (PnH4CurrReport.al:248 `05-03-24`).
            Region::DaDK if !column => format!("{d:02}-{m:02}-{year}"),
            // A dataset column of zz-ZZ is `MM/dd/yyyy` on BC 27, 28 and 29 (YReportPropsTests.al:128 `01/31/2025`,
            // BdRegionTests.al:213,220 `03/05/2024`; corpus bump 43, the older Linux image gave `1/31/2025`).
            Region::ZzZZ if column => format!("{m:02}/{d:02}/{year}"),
            Region::DaDK | Region::ZzZZ => return None,
        })
    }

    /// `Format(Date, 0, 4)`: the month name in the session's language (en-US), the order of the region
    /// (YReportPropsTests.al:40-96: `31. January 2025`, `31 January 2025`, `2025年1月31日`). A one-digit day is not measured.
    fn long_date(self, y: i32, m: u32, d: u32, month: &str) -> Option<String> {
        if d < 10 {
            return None;
        }
        Some(match self {
            Region::DeDE => format!("{d}. {month} {y}"),
            Region::EnGB | Region::FrFR => format!("{d} {month} {y}"),
            Region::JaJP => format!("{y}\u{5E74}{m}\u{6708}{d}\u{65E5}"),
            Region::DaDK | Region::ZzZZ => return None,
        })
    }
}

/// `Format` while a report's `FormatRegion` is set (by `CurrReport.FormatRegion` or the report property). Pinned: de-DE through
/// the setter (misc/lt-probes-needed-3/PnC06ReportLanguage.al:Language_SetAndRead), de-DE, fr-FR and da-DK through the setter
/// (misc/lt-probes-needed-8/PnH4CurrReport.al:Culture_FormatRegion_ChangesFormat: Decimal, Date, Boolean) and de-DE, en-GB,
/// fr-FR, ja-JP and zz-ZZ through the property (report/lt-y-report-props/YReportPropsTests.al:34-148: Decimal, Date, long Date,
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
    let r = Region::parse(region).ok_or_else(refuse)?;
    let text = match v {
        // `bool=Yes` under de-DE, fr-FR and da-DK (PnH4CurrReport.al:241-249).
        Value::Boolean(_) if standard == 0 && matches!(r, Region::DeDE | Region::FrFR | Region::DaDK) => {
            format_value(v, 0, standard, &plain)?
        }
        Value::Date(day, false) if *day != 0 && standard == 0 => {
            let (y, m, d) = civil_from_day(*day);
            match r {
                Region::ZzZZ => format_value(v, 0, standard, &plain)?,
                _ => r.date(y, m, d, false).ok_or_else(refuse)?,
            }
        }
        Value::Date(day, false) if *day != 0 && standard == 4 && plain.language() == 1033 => {
            let (y, m, d) = civil_from_day(*day);
            match r {
                Region::ZzZZ => format_value(v, 0, standard, &plain)?,
                _ => r.long_date(y, m, d, MONTHS[m as usize - 1]).ok_or_else(refuse)?,
            }
        }
        // `31.01.25 14:30`: the short date, a space and the 24-hour time without seconds; zz-ZZ keeps en-US's `01/31/25 02:30 PM`
        // (YReportPropsTests.al:34-148). An hour below 10 is not measured.
        Value::DateTime(t) if *t != 0 && standard == 0 && r != Region::DaDK => match r {
            Region::ZzZZ => format_value(v, 0, standard, &plain)?,
            _ => {
                let ms = t - 1;
                let (h, mi) = (ms.rem_euclid(86_400_000) / 3_600_000, ms.rem_euclid(3_600_000) / 60_000);
                if h < 10 {
                    return Err(refuse());
                }
                let (y, m, d) = civil_from_day(ms.div_euclid(86_400_000) as i32 + 1);
                format!("{} {h:02}:{mi:02}", r.date(y, m, d, false).ok_or_else(refuse)?)
            }
        },
        // de-DE and da-DK swap the separators (`1.234,5`), fr-FR groups with U+202F; a negative amount is measured nowhere
        // (the de-DE rule predates this guard and keeps its sign).
        Value::Decimal(_) if standard == 0 => {
            let en = format_value(v, 0, standard, &plain)?;
            if en.starts_with('-') && r != Region::DeDE {
                return Err(refuse());
            }
            let (group, decimal) =
                r.separators(ctx.format_gate(al_version::Gate::FrFrGroupSeparatorNoBreakSpaceFrom29));
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
/// language code, else the ENU entry. Pinned for OptionCaptionML (record/lt-x-table-props/XTablePropsTests.al:175-232 on a
/// table field: 1033 ENU, 1031 DEU, 1036 falls back to ENU; :741-785 on a report column: an ENU-only list in 1031 gives ENU).
/// Another language and a list without the entry and without ENU are not pinned: the error says why.
pub fn ml_text_in_language(ml: &str, language: i32) -> Result<String, String> {
    let code = match language {
        1033 => "ENU",
        1031 => "DEU",
        1036 => "FRA",
        n => return Err(format!("`{ml}` in language {n} (only 1033, 1031 and 1036 are pinned)")),
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

/// The text of a dataset column under the report's `FormatRegion` (report/lt-y-report-props/YReportPropsTests.al:34-148), `None`
/// when the shape is not pinned: a Date `31.01.2025`, a DateTime `31.01.2025 14:30:15` (24-hour time with seconds; an hour below
/// 10 is not measured), a Decimal with the region's decimal separator and no grouping (`1234567,891`).
pub fn region_column(r: Region, v: &Value) -> Option<String> {
    match v {
        Value::Date(day, false) if *day != 0 => {
            let (y, m, d) = civil_from_day(*day);
            r.date(y, m, d, true)
        }
        Value::DateTime(t) if *t != 0 => {
            let ms = t - 1;
            let of_day = ms.rem_euclid(86_400_000);
            let (h, mi, s) = (of_day / 3_600_000, of_day / 60_000 % 60, of_day / 1000 % 60);
            if h < 10 || of_day % 1000 != 0 {
                return None;
            }
            let (y, m, d) = civil_from_day(ms.div_euclid(86_400_000) as i32 + 1);
            Some(format!("{} {h:02}:{mi:02}:{s:02}", r.date(y, m, d, true)?))
        }
        Value::Decimal(d) if !d.is_sign_negative() => {
            // The decimal separator does not depend on the version.
            let (_, decimal) = r.separators(false);
            Some(d.to_string().replace('.', &decimal.to_string()))
        }
        _ => None,
    }
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

/// `Format(Value, Length, Standard)` with BC's own errors kept apart from refusals.
pub fn format_value(v: &Value, length: i32, standard: i32, ctx: &dyn FormatCtx) -> Result<String, FormatFail> {
    if let Value::Unknown(why) = v {
        return Err(FormatFail::Unsupported(why.to_string()));
    }
    if let Some(why) = ctx.region_unpinned() {
        // A text, and an Integer in the standard format, read the same in every region the corpus measured
        // (report/lt-y-report-props/YReportPropsTests.al:34-148 `1234567`); any other value is refused.
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
        Value::Boolean(b) => match (standard, b) {
            (2, true) => "1".into(),
            (2, false) => "0".into(),
            (9, b) => b.to_string(),
            // German after GlobalLanguage(1031): `Ja` / `Nein` (misc/lt-probes-needed-3/PnC*: Identity_GlobalLanguage_SetsAndRestores
            // `set_ok = Ja`, ObjectId_WithAnotherGlobalLanguage `set 1031 ok = Ja`, P142_Date_Language).
            (_, true) if ctx.language() == 1031 => "Ja".into(),
            (_, false) if ctx.language() == 1031 => "Nein".into(),
            // French after GlobalLanguage(1036): `Oui` / `Non` (types/lt-conversions-hidden-parts-doctype/P142ToTextLang.al:153).
            (_, true) if ctx.language() == 1036 => "Oui".into(),
            (_, false) if ctx.language() == 1036 => "Non".into(),
            (_, true) => "Yes".into(),
            (_, false) => "No".into(),
        },
        Value::Integer(i) => i.to_string(),
        Value::BigInteger(i) => i.to_string(),
        // A Byte formats as the character of that code (streams/TestProbeStreamContracts.al:Read_IntoTypes,
        // `integer_into_byte`: 2 is `\u0002`).
        // A byte from 0x80 up reads as the default (code page 437) stream encoding reads it: the bytes of a UTF-8 text
        // `C3 A9 E2 82 AC` format as the characters `\u201C \u0014 \u00CB \u00E9 \u00BC` (misc/lt-probes-needed-3/
        // PnC20JsonStreams.al:WriteTo_NonAscii_PerEncoding).
        Value::Byte(b) if *b >= 0x80 && crate::stream::Encoding::MsDos.decode_pinned(&[*b]) => {
            crate::stream::Encoding::MsDos.decode(&[*b])
        }
        // Kept, not pinned: a byte from 0x80 up whose character no corpus test pins shows as the Latin-1 character of its
        // code; json/TestJsonComplete.al:JsonObject_GetByte_ReturnsValue (Assert.AreEqual of Byte 200) passes through it.
        Value::Byte(b) => char::from(*b).to_string(),
        Value::Char(c) => char::from_u32(u32::from(*c)).map(String::from).unwrap_or_default(),
        Value::Decimal(d) => format_decimal(d, standard)?,
        // A NUL character does not show (types/lt-variant-is-text-streams/LTP89StreamText.al:Text_BytesWritten `nul_between`).
        Value::Text(t) | Value::Code(t) if t.contains('\0') => t.replace('\0', ""),
        Value::Text(t) | Value::Code(t) => t.to_string(),
        Value::Option(o, members) => match standard {
            2 | 9 if crate::sysenum::is_field_table_type(members) => FIELD_TABLE_TYPE_ORDINALS
                .get(*o as usize)
                .map_or_else(|| o.to_string(), |v| v.to_string()),
            2 | 9 if crate::sysenum::platform_ordinal(members, *o).is_some() => {
                crate::sysenum::platform_ordinal(members, *o).unwrap_or(*o).to_string()
            }
            // The IsolationLevel option shows member names for format 2 (misc/lt-probes-needed-1/PnAPlatformOptions.al:
            // IsolationLevel_Members: Default, UpdLock, ReadCommitted, RepeatableRead). ReadUncommitted is not pinned.
            2 | 9 if crate::sysenum::is_isolation_level(members) => {
                match members.get(*o as usize).map(String::as_str) {
                    Some("ReadUncommitted") | None => {
                        return Err(FormatFail::Unsupported(
                            "Format 2 of IsolationLevel::ReadUncommitted (not pinned)".into(),
                        ));
                    }
                    Some(m) => m.to_string(),
                }
            }
            2 | 9 if !crate::sysenum::ordinals_hidden(members) => o.to_string(),
            // `Option` declared without members is the one empty name: it shows the number
            // (types/lt-scalars/ScT70062.al:Option_Defaults `nomembers_len`, `nomembers_3`).
            _ if members.len() == 1 && members[0].is_empty() => o.to_string(),
            // A member with a blank name shows as one space (types/lt-scalars/ScT70053.al:Option_Format `blank_len`).
            _ => match members.get(*o as usize) {
                // An unquoted empty member shows its number (misc/lt-inferred-behaviors/P69OptionInText.al:Option_StrSubstNo
                // `subst_2`; record/lt-record-write/RcBInit.al:Props_BlankZero_Option_Caption `opt blank member`).
                Some(m) if m.is_empty() => o.to_string(),
                Some(m) => m.clone(),
                None => o.to_string(),
            },
        },
        // Nor for an enum, named 'Option' (the enum test Enum_FormatNumbers `blue3`).
        Value::Enum(..) if (3..=8).contains(&standard) => return Err(no_such_format(standard, "Option")),
        Value::Enum(id, o) => match standard {
            2 | 9 => o.to_string(),
            _ => ctx.enum_name(*id, *o),
        },
        // Numbers 0, 1, 2 and 9 are the braced form; 3 has neither braces nor dashes, 4 only the dashes (platform/lt-platform-members/
        // PlTGuid.al:Format_Numbers, NullGuid_Values); the others are not pinned for a Guid.
        Value::Guid(g) => match standard {
            0 | 1 | 2 | 9 => format_guid(g),
            3 => format_guid(g).replace(['{', '}', '-'], ""),
            4 => format_guid(g).replace(['{', '}'], ""),
            n => {
                return Err(FormatFail::Unsupported(format!(
                    "Format of a Guid with the standard format {n} (not pinned)"
                )));
            }
        },
        // A Blob shows `*` when it holds data and one space when it does not (record/
        // TestFormatRecordContracts.al:132-136, the cleared record shows the blank form).
        Value::Blob(b) => {
            let buf = b.borrow();
            // ADR-040: the text shows whether the buffer holds data, which a pending buffer cannot say.
            if let Some(report) = &buf.render_pending {
                crate::stream::note_pending_observation(report);
            }
            (if buf.bytes.is_empty() { " " } else { "*" }).to_string()
        }
        Value::Date(0, _) | Value::Time(0) | Value::DateTime(0) => String::new(),
        Value::Date(day, closing) => format_date(*day, *closing, standard, ctx.language())?,
        Value::Time(t) => format_time(t - 1, standard)?,
        Value::DateTime(t) => format_datetime(t - 1, standard)?,
        Value::Duration(ms) => format_duration(*ms, standard, ctx.language())?,
        // A formula prints without its angle brackets (types/TestDateRoundFormatStringContracts.al:
        // DateFormula_Evaluate_ThenFormat_PrintsTheNormalizedFormula).
        Value::DateFormula(f) => crate::datefns::formula_text(f),
        // A Version reads `major.minor.build.revision` (session/TestBCPlatformContracts.al:120-136
        // compares the formatted AppVersion with the manifest's `1.0.0.0`).
        Value::Library(crate::library_kind::LibraryKind::Version, _) if standard == 9 => {
            // session/TestProbeSessionMiscContracts.al:Version_Forms.
            return Err(no_such_format(9, "Version"));
        }
        Value::Library(crate::library_kind::LibraryKind::Version, l) => match &*l.borrow() {
            crate::value::LibraryValue::Version(v) => format!("{}.{}.{}.{}", v[0], v[1], v[2], v[3]),
            _ => return Err("Format of a Version nobody set".to_string().into()),
        },
        // A SessionSettings renders its properties in this layout (a fresh instance: session/lt-session-settings-fresh/
        // SessionSettingsFreshReads.al:57); it shows the time zone and differs when a property differs
        // (session/TestSessionSettings.al:457-493). ProfileSystemScope reads No (its setter keeps nothing, sessiontypes.rs).
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
        // (filterpage/TestFilterPageBuilder.al:FilterPageBuilder_Format_ReturnsANonEmptyRendering).
        // A builder nobody added a control to formats as an empty text (filterpage/W3FilterPageBuilder.al:
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
        // A document type or an XML declaration formats as its markup (xml/lt-xml-doctype-declaration/DocTypeErrors.al:
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
                Err(Signal::Unsupported(why)) => return Err(why.into()),
                Err(_) => return Err(format!("Format of a {name} that is neither a result nor an error").into()),
            }
        }
        // A JSON value formats as its compact text. Pinned: an empty JsonObject is `{}` and a null token `null`
        // (types/ProbeTypeFormsAndFilters.al:Clear_JsonTypes_GiveFreshUnassigned, BC 27.5); an object or array
        // gives its compact text; any other value is not pinned.
        Value::Library(name, l) if matches!(name.as_str(), "JsonObject" | "JsonArray" | "JsonValue" | "JsonToken") => {
            if let crate::value::LibraryValue::Json(c) = &*l.borrow() {
                if let Some(why) = c.unpinned {
                    return Err(FormatFail::Unsupported(why.to_string()));
                }
            }
            let container = matches!(&*l.borrow(), crate::value::LibraryValue::Json(c)
                if matches!(&c.node, crate::json::JsonNode::Object(k) if !k.is_empty())
                    || matches!(&c.node, crate::json::JsonNode::Array(_)));
            if container {
                let mut out = String::new();
                crate::json::write_text(l, &mut out)
                    .map_err(|_| "Format of a JSON value the writer refuses".to_string())?;
                out
            } else {
                match &*l.borrow() {
                    crate::value::LibraryValue::Json(c) => match &c.node {
                        crate::json::JsonNode::Object(k) if k.is_empty() => "{}".to_string(),
                        // {"a":1} formats as `{"a":1}` (streams/ProbeJsonAndInStreamBytes.al:ReadFrom_InStream_WithBomAndWithout).
                        crate::json::JsonNode::Object(_) | crate::json::JsonNode::Array(_) => String::new(),
                        crate::json::JsonNode::Scalar(crate::json::Scalar::Null) => "null".to_string(),
                        _ => {
                            return Err(format!(
                                "Format of {name} with content (standard format {standard}) is not pinned yet"
                            )
                            .into());
                        }
                    },
                    crate::value::LibraryValue::Empty if name == "JsonObject" => "{}".to_string(),
                    crate::value::LibraryValue::Empty if matches!(name.as_str(), "JsonValue" | "JsonToken") => {
                        "null".to_string()
                    }
                    _ => {
                        return Err(
                            format!("Format of a {name} (standard format {standard}) is not pinned yet").into(),
                        );
                    }
                }
            }
        }
        // A WebServiceActionContext has no text form (session/TestWebServiceActionContext.al:615-650).
        Value::Library(crate::library_kind::LibraryKind::WebServiceActionContext, _) => String::new(),
        // A RecordId reads `Table: key1,key2` (corpus record/TestRecordRecordId.al:42-56 only needs
        // the key values in it; the layout is docs/probes-needed.md).
        Value::Library(crate::library_kind::LibraryKind::RecordId, l) => match &*l.borrow() {
            crate::value::LibraryValue::RecordId { table, name, key, .. } => {
                // The numbers 3 to 8 do not exist for a RecordId; 2 and 9 format the key values with that number (types/
                // lt-implicit-conversions/P79RecordId.al:CodeKey_FormatNumbers, DateKey_FormatNumbers).
                if (3..=8).contains(&standard) {
                    return Err(no_such_format(standard, "RecordID"));
                }
                let key_standard = if matches!(standard, 2 | 9) { standard } else { 0 };
                // A key that is empty or holds a comma is quoted (record/TestProbeRecordRefContracts.al:RecordId_FormatForms;
                // P79RecordId.al:NeverRead_BlankRecordId, a blank Date too).
                let key = key
                    .iter()
                    .map(|k| {
                        format_value(k, 0, key_standard, ctx).map(|t| {
                            // A text that starts with a double quote is quoted and doubles its quotes (types/lt-evaluate-recordid/
                            // EvaluateRecordIdComposite.al:RoundTrip_KeyValueWithComma `dquote`: `"Q"` reads `"""Q"""`); a quote further in
                            // is left as it is (types/lt-implicit-conversions/P79RecordId.al:CompositeKey_SpecialCharacters: `A"B`).
                            if t.is_empty()
                                || (matches!(k, Value::Text(_) | Value::Code(_))
                                    && (t.contains(',') || t.starts_with('"')))
                            {
                                format!("\"{}\"", t.replace('"', "\"\""))
                            } else {
                                t
                            }
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                // Format 1 names the table by its caption (record/lt-record-write/RcBMeta.al:572, :578, also under
                // GlobalLanguage 1031); format 9 of a table in a namespace names it `[<namespace>.<name>]`
                // (misc/lt-inferred-behaviors/P69NsConsumers.al:214, a name with spaces; :210 format 1 of the same table
                // gives the plain name, which is its caption). The other numbers give the plain name (RcBMeta.al:570-574).
                let shown = match standard {
                    1 | 9 => {
                        let Some((caption, namespace)) = ctx.record_id_table(*table) else {
                            return Err(format!("Format of a RecordId with standard format {standard} where the table caption is not known").into());
                        };
                        match (standard, namespace) {
                            (1, _) => caption,
                            (_, Some(ns)) if name.contains(' ') => format!("[{ns}.{name}]"),
                            (_, Some(_)) => {
                                return Err(format!(
                                    "Format(RecordId, 0, 9) of table {name} in a namespace, whose name has no space (whether it is bracketed is not pinned)"
                                )
                                .into());
                            }
                            (_, None) => name.clone(),
                        }
                    }
                    _ => name.clone(),
                };
                // A RecordId made by Evaluate from a table name alone has no key values and reads as the name
                // (errors/lt-ms-refusals-128/P128EvalRecordId.al:P128_Evaluate_IntoRecordId_OtherTexts `table name only`).
                if key.is_empty() {
                    if shown != *name {
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
    // A Text shorter than the length is padded on the right (types/lt-scalars/ScT70053.al:Text_FormatNumbers `len30`).
    if let (Value::Text(_) | Value::Code(_), true) = (v, length > 0) {
        let n = s.chars().count();
        if n < length as usize {
            return Ok(format!("{s}{}", " ".repeat(length as usize - n)));
        }
    }
    // A negative length cuts a Text the same way (types/lt-scalars/ScT70053.al:Text_FormatNumbers `len-5`).
    if let (Value::Text(_) | Value::Code(_), true) = (v, length < 0) {
        if s.chars().count() > length.unsigned_abs() as usize {
            return Ok(s.chars().take(length.unsigned_abs() as usize).collect());
        }
    }
    // A text longer than the length is cut to it (text/lt-label-textbuilder/TxCLabel.al:Label_Value_And_Format `Format(Plain, 5)`).
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
            // types/lt-implicit-conversions/P79RecordId.al:RecordId_InLongerTexts `comp-subst-width`.
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

/// The error of a Decimal outside +-999,999,999,999,999.99 (misc/lt-system-functions/RandomFormat.al:Format_Decimal `af191`, `af193`;
/// a Duration converted into a Decimal: types/lt-filter-enum-relation-duration-guid/D_Duration.al:Duration_To_Integer_BeyondRange `af126`).
pub(crate) fn decimal_range_error(d: &rust_decimal::Decimal) -> Option<FormatFail> {
    if d.abs() <= rust_decimal::Decimal::new(99_999_999_999_999_999, 2) {
        return None;
    }
    // The shown value keeps 18 significant digits (types/lt-scalars/ScT70058.al:Decimal_Digits `af83`).
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
    // A Decimal keeps 18 significant digits: 123456789012345.123456 shows as 123456789012345.123 (types/lt-scalars/ScT70050.al:
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
    // PnAStreamDecimalBytes.al:Write_Decimal_Tiny `minus-1E-28`), and in the default format 0 too (types/lt-decimal-digits/
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
    // trailing minus when it is (types/lt-evaluate-format-lasterror-setview/P86FormatDecimal.al:Num_*; text/
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
fn no_such_format(n: i32, ty: &str) -> FormatFail {
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
/// (`09/20/26 01:10 PM`, pagesave/TestPageSaveDataLayer.al:199-206); format 1 adds seconds and
/// milliseconds (refused without milliseconds); format 9 is the round-trip form
/// (`2026-09-20T11:10:12.851Z`, no fraction when there are no milliseconds:
/// types/TestFormatEvaluateTextContracts.al:Format_DateTime_StandardFormats). The session time zone
/// is taken to be UTC.
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
    let date = format!("{m:02}/{d:02}/{:02}", y.rem_euclid(100));
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
/// handlers/TestPageFieldFormats_Tests.al:599-602, handlers/TestPageRuntimeContracts.al:162);
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
/// forms the probe measured: `1 Stunde 2 Minuten 3 Sekunden` (types/lt-conversions-hidden-parts-doctype/P142ToTextLang.al:190) and
/// `1 heure 2 minutes 3 secondes` (:214). Every other form is refused.
fn unit_word(lang: i32, unit: usize, plural: bool) -> Option<String> {
    match (lang, unit, plural) {
        (1031, 1, false) => Some("Stunde".into()),
        (1031, 2, true) => Some("Minuten".into()),
        (1031, 3, true) => Some("Sekunden".into()),
        (1036, 1, false) => Some("heure".into()),
        (1036, 2, true) => Some("minutes".into()),
        (1036, 3, true) => Some("secondes".into()),
        (1031 | 1036, ..) => None,
        _ => Some(format!("{}{}", DURATION_UNITS[unit].0, if plural { "s" } else { "" })),
    }
}

fn format_duration(ms: i64, standard: i32, lang: i32) -> Result<String, FormatFail> {
    // A Duration beyond what a .NET TimeSpan holds (ticks of 100 ns in an Int64) raises (D_Duration.al:Duration_To_Integer_BeyondRange `af128`).
    if ms.unsigned_abs() > (i64::MAX / 10_000) as u64 {
        return Err(FormatFail::Error {
            message: "Arithmetic operation resulted in an overflow.".into(),
            code: "AL:Overflow".into(),
        });
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
    // The month name of format 4 follows the language; only March is measured for German and French (P142ToTextLang.al:109
    // `März 15, 2024`, :117 `mars 15, 2024`). Another month or language, and format 7, are refused.
    let month = match (lang, m, standard) {
        (1033, ..) => MONTHS[m as usize - 1],
        (1031, 3, 4) => "März",
        (1036, 3, 4) => "mars",
        (_, _, 4 | 7) => {
            return Err(FormatFail::Unsupported(format!(
                "Format {standard} of a Date in language {lang}, month {m}: the month name is not pinned (docs/probes-needed.md)"
            )));
        }
        _ => MONTHS[m as usize - 1],
    };
    Ok(match standard {
        // The year has two digits from 1931 to 2030 and four otherwise, so a text reads back into the same year
        // (controls/W2ValueControls.al:Date_SessionFormatTwoDigitYear_*: 1929 and 1930 give 10 characters, 1999
        // to 2030 give 8, BC 27.5 and 29.0).
        0 | 1 if (1931..=2030).contains(&y) => format!("{c}{m:02}/{d:02}/{yy:02}"),
        0 | 1 => format!("{c}{m:02}/{d:02}/{y:04}"),
        2 => format!("{m:02}{d:02}{yy:02}{c}D"),
        3 => format!("{yy:02}/{m:02}/{d:02}"),
        4 => format!("{month} {d}, {y}"),
        5 => format!("{m:02}{d:02}{yy:02}"),
        6 => format!("{yy:02}{m:02}{d:02}"),
        7 => format!("{} {d}, {y}", &month[..3]),
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
        // pagesave/TestPageSaveDataLayer.al:199-206 (measured on BC): Format(_, 0, 2) of a DateTime
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
        // types/lt-conversions-hidden-parts-doctype/P142ToTextLang.al:109,117,153,190,214
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
        assert!(matches!(
            format_value(&jan, 0, 4, &L(1031)),
            Err(FormatFail::Unsupported(_))
        ));
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
        assert!(matches!(
            format_value(&Value::Duration(86_400_000), 0, 0, &L(1031)),
            Err(FormatFail::Unsupported(_))
        ));
        assert_eq!(format_value(&Value::Boolean(false), 0, 0, &L(1036)).unwrap(), "Non");
        assert_eq!(format_value(&Value::Boolean(true), 0, 9, &L(1036)).unwrap(), "true");
    }

    #[test]
    fn a_format_region_rewrites_date_and_decimal_and_refuses_the_rest() {
        // misc/lt-probes-needed-3/PnC06ReportLanguage.al:Language_SetAndRead
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
        assert!(matches!(format_value(&d, 0, 4, &R), Err(FormatFail::Unsupported(_))));
    }

    #[test]
    fn the_report_format_regions_the_corpus_pins() {
        // report/lt-y-report-props/YReportPropsTests.al:34-148 (31 January 2025 14:30:15, 1234567.891, 1234567) and
        // misc/lt-probes-needed-8/PnH4CurrReport.al:241-249 (5 March 2024, 1234.5, true).
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
            "1\u{202F}234\u{202F}567,891|31/01/25|31/01/25 14:30|31 January 2025|1234567"
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
            ("fr-FR", "1\u{202F}234,5|05/03/24|Yes"),
            ("da-DK", "1.234,5|05-03-24|Yes"),
        ] {
            let got = [&small, &march, &Value::Boolean(true)]
                .map(|v| format(v, 0, 0, &R(r)))
                .join("|");
            assert_eq!(got, want, "{r}");
        }
        // Neighbours no test pins stay refused: another region, a day or an hour below 10 where the region's padding is not
        // measured, a long date with a one-digit day, a Time, a negative amount outside de-DE.
        let refused = |v: &Value, n: i32, r: &'static str| {
            matches!(format_value(v, 0, n, &R(r)), Err(FormatFail::Unsupported(_)))
        };
        assert!(refused(&amt, 0, "nl-NL"));
        assert!(refused(&amt, 0, "en-gb"));
        assert!(refused(&march, 0, "en-GB"));
        assert!(refused(&march, 0, "ja-JP"));
        assert!(refused(&march, 4, "fr-FR"));
        let early = Value::DateTime((i64::from(day_from_civil(2025, 1, 31)) - 1) * 86_400_000 + 9 * 3_600_000 + 1);
        assert!(refused(&early, 0, "en-GB"));
        assert!(refused(&Value::Time(5), 0, "fr-FR"));
        assert!(refused(&dec("-1.5"), 0, "fr-FR"));
        assert!(refused(&stamp, 0, "da-DK"));
    }

    #[test]
    fn a_zz_zz_dataset_column_pads_month_and_day() {
        // report/lt-y-report-props/YReportPropsTests.al:128 (`01/31/2025`, `01/31/2025 14:30:15`, BC 27, 28, 29) and
        // report/lt-bd-format-region/BdRegionTests.al:213,220 (`03/05/2024`).
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
    fn on_bc_29_fr_fr_groups_with_a_no_break_space() {
        // misc/lt-probes-needed-8/PnH4CurrReport.al:247 and :311 (#if BC29PLUS: U+00A0; BC 27 and 28 keep U+202F) and
        // report/lt-y-report-props/YReportPropsTests.al:74 (BC 29 AmtText).
        struct R(bool);
        impl FormatCtx for R {
            fn enum_name(&self, _: u32, o: i32) -> String {
                o.to_string()
            }
            fn region(&self) -> Option<String> {
                Some("fr-FR".into())
            }
            fn format_gate(&self, gate: al_version::Gate) -> bool {
                gate == al_version::Gate::FrFrGroupSeparatorNoBreakSpaceFrom29 && self.0
            }
        }
        assert_eq!(format(&dec("1234567.891"), 0, 0, &R(true)), "1\u{A0}234\u{A0}567,891");
        assert_eq!(format(&dec("1234.5"), 0, 0, &R(false)), "1\u{202F}234,5");
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
        // first unit only (a TestPage control shows every unit negative, handlers/TestPageRuntimeContracts.al:162).
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
}
