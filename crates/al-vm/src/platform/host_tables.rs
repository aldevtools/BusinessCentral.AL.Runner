//! Tables whose rows are facts about the host: Time Zone and Windows Language.

use super::mat::Entry;
use super::{Ident, blank, pos};
use al_builtins::value::Value;
use al_data::TableDef;

/// The Time Zone table lists the host's zones, `No.` from 1 (record/TestTimeZoneVirtualTable.al:
/// more than one row, `No.` running 1 to N without gaps, a non-blank `ID`, `Get(1)` the first row of
/// a scan, `Get` past the end finds nothing).
pub(super) fn time_zone_rows(t: &TableDef, i: &Ident) -> Vec<Entry> {
    i.time_zones
        .iter()
        .enumerate()
        .map(|(n, z)| {
            let mut r = blank(t);
            r[pos(t, 1)] = Value::Text(z.id.clone().into());
            r[pos(t, 2)] = Value::Text(z.display_name.clone().into());
            r[pos(t, 3)] = Value::Integer(n as i32 + 1);
            Entry::ok(r)
        })
        .collect()
}

/// The Feature Key table: ID, `Enabled` (None or All Users per row), `Mandatory By` (the wave the mandatory-by version falls in),
/// `Mandatory By Version`, `Can Try`, `Is One Way` and `Data Update Required`, per major from record/PFFeatureKeyColumns.al
/// (FeatureKey_RowsPerId_PerMeasuredVersion; the state is per tenant); `Enabled` and `Data Update Required` of the first major
/// also record/TestFieldLenAndFeatureKeyContracts.al:83-125. The three lists are the newest minor of each major; other
/// minors differ in a few rows and are refused. The Description columns are not pinned.
pub(super) fn feature_key_rows(t: &TableDef, target: Option<&al_version::Target>) -> Vec<Entry> {
    // (ID, Enabled is All Users, Mandatory By Version major, Can Try, Is One Way, Data Update Required)
    type Row = (&'static str, bool, u32, bool, bool, bool);
    const ROWS_FIRST: &[Row] = &[
        ("AdvancedTellMe", false, 28, false, false, false),
        ("CalcOnlyVisibleFlowFields", true, 28, true, false, false),
        ("ConcealedMaskType", true, 29, false, false, false),
        ("ConcurrentInventoryPosting", true, 29, false, false, false),
        ("ConcurrentJobPosting", true, 29, false, false, false),
        ("ConcurrentResourcePosting", true, 29, false, false, false),
        (
            "DisableSoapWebServicesOnMicrosoftUIPages",
            true,
            29,
            false,
            false,
            false,
        ),
        ("EnableMcpAccess", true, 28, false, false, false),
        ("FinancialReportDefaults", true, 29, true, false, false),
        ("FullTextSearch", false, 28, true, false, false),
        ("GLCurrencyRevaluation", false, 28, false, true, true),
        (
            "Manufacturing_FlushingMethod_ActivateManualWoPick",
            false,
            29,
            false,
            true,
            true,
        ),
        ("ReminderTermsCommunicationTexts", true, 28, false, false, false),
        ("SalesPrices", false, 28, false, true, true),
        ("SaveValueToDatabasePromptly", true, 29, true, false, false),
        ("SemanticMetadataSearch", true, 28, false, false, false),
    ];
    const ROWS_SECOND: &[Row] = &[
        ("AdvancedTellMe", false, 29, false, false, false),
        ("ConcealedMaskType", true, 29, false, false, false),
        ("ConcurrentInventoryPosting", true, 29, false, false, false),
        ("ConcurrentJobPosting", true, 29, false, false, false),
        ("ConcurrentResourcePosting", true, 29, false, false, false),
        (
            "DisableSoapWebServicesOnMicrosoftUIPages",
            true,
            29,
            false,
            false,
            false,
        ),
        ("EXRPerformantTrialBalance", true, 31, true, false, false),
        ("FinancialReportDefaults", true, 29, true, false, false),
        ("FullTextSearch", false, 29, true, false, false),
        (
            "Manufacturing_FlushingMethod_ActivateManualWoPick",
            false,
            29,
            false,
            true,
            true,
        ),
        ("ReminderTermsCommunicationTexts", true, 29, false, false, false),
        ("SalesPrices", false, 31, false, true, true),
        ("SaveValueToDatabasePromptly", true, 29, true, false, false),
        ("SemanticMetadataSearch", false, 29, false, false, false),
    ];
    const ROWS_THIRD: &[Row] = &[
        ("AcceleratedDepreciation", false, 31, false, true, true),
        ("AdvancedTellMe", true, 30, false, false, false),
        ("ConcurrentInventoryPosting", true, 31, false, false, false),
        ("ConcurrentJobPosting", true, 31, false, false, false),
        ("ConcurrentResourcePosting", true, 31, false, false, false),
        (
            "DisableSoapWebServicesOnMicrosoftUIPages",
            true,
            29,
            false,
            false,
            false,
        ),
        ("DocumentReportExperience", true, 31, false, false, false),
        ("EXRPerformantTrialBalance", true, 31, true, false, false),
        ("FinancialReportDefaults", true, 29, true, false, false),
        ("FullTextSearch", false, 30, true, false, false),
        ("SalesPrices", false, 31, false, true, true),
        ("SaveValueToDatabasePromptly", true, 30, true, false, false),
        ("SemanticMetadataSearch", true, 30, false, false, false),
    ];
    use al_version::FeatureKeyRows;
    let rows = match al_version::feature_key_rows(target) {
        Some(FeatureKeyRows::Bc27) => ROWS_FIRST,
        Some(FeatureKeyRows::Bc28) => ROWS_SECOND,
        Some(FeatureKeyRows::Bc29) => ROWS_THIRD,
        None => {
            return vec![Entry::poisoned_prefix(
                blank(t),
                0,
                "Feature Key: its rows are served for the minors they were measured on only (docs/probes-needed.md)"
                    .into(),
            )];
        }
    };
    rows.iter()
        .map(|(id, all_users, version, can_try, one_way, data_update)| {
            let mut r = blank(t);
            r[pos(t, 1)] = Value::Text((*id).into());
            r[pos(t, 2)] = option(t, 2, i32::from(*all_users));
            // The wave text of the mandatory-by version: 28.0 is 2026 Wave 1, then one wave per half year.
            let wave = match version {
                28 => "2026 Wave 1 (from April to September 2026)",
                29 => "2026 Wave 2 (from October 2026 to March 2027)",
                30 => "2027 Wave 1 (from April to September 2027)",
                _ => "2027 Wave 2 (from October 2027 to March 2028)",
            };
            r[pos(t, 5)] = Value::Text(wave.into());
            r[pos(t, 6)] = Value::Boolean(*can_try);
            r[pos(t, 7)] = Value::Boolean(*one_way);
            r[pos(t, 8)] = Value::Boolean(*data_update);
            r[pos(t, 9)] = Value::Text(format!("{version}.0").into());
            // Description (3), "Learn More Link" (4) and "Description In English" (10): no corpus test reads them.
            for col in [3, 4, 10] {
                if t.fields.iter().any(|f| f.id == col) {
                    r[pos(t, col)] = super::unknown(t, col, &format!("feature key '{id}'"));
                }
            }
            Entry::ok(r)
        })
        .collect()
}

// ---- Windows Language -----------------------------------------------------------------------------

use super::{option, ranges, refused};
use al_builtins::filter::{FieldFilter, FilterExpr};
use super::mat::Mat;
use al_data::{Row, Scan, StoreError};

/// The languages the corpus pins, as (LCID, name, abbreviated name, primary code page, tag). The table lists every
/// language of the host (212 rows on BC 29, 208 before: platform/lt-ag-contoso/AgWindowsLanguage.al:61-68), of which these
/// are pinned: 1031, 1033 and 2057 (record/TestWindowsLanguageTable.al:19-60), 1025, the first row
/// (AgWindowsLanguage.al:65), and the row of each of the DemoTool's 54 codes (AgWindowsLanguage.al:167-184, 209-226,
/// 251-268, QX_WindowsLanguage_ByAbbreviatedName_Part1-3, the same on every version). A read that would have to answer
/// for any other language of the Windows range is refused.
const LANGUAGES: &[(i32, &str, &str, &str, &str)] = &[
    (1025, "Arabic (Saudi Arabia)", "ARA", "720", "ar-SA"),
    (1026, "Bulgarian (Bulgaria)", "BGR", "866", "bg-BG"),
    (1029, "Czech (Czechia)", "CSY", "852", "cs-CZ"),
    (1030, "Danish (Denmark)", "DAN", "850", "da-DK"),
    (1031, "German (Germany)", "DEU", "850", "de-DE"),
    (1032, "Greek (Greece)", "ELL", "737", "el-GR"),
    (1033, "English (United States)", "ENU", "437", "en-US"),
    (1034, "Spanish (Spain, Sort Order=tradnl)", "ESP", "850", "es-ES"),
    (1035, "Finnish (Finland)", "FIN", "850", "fi-FI"),
    (1036, "French (France)", "FRA", "850", "fr-FR"),
    (1038, "Hungarian (Hungary)", "HUN", "852", "hu-HU"),
    (1039, "Icelandic (Iceland)", "ISL", "850", "is-IS"),
    (1040, "Italian (Italy)", "ITA", "850", "it-IT"),
    (1041, "Japanese (Japan)", "JPN", "932", "ja-JP"),
    (1042, "Korean (South Korea)", "KOR", "949", "ko-KR"),
    (1043, "Dutch (Netherlands)", "NLD", "850", "nl-NL"),
    (1044, "Norwegian Bokmål (Norway)", "NOR", "850", "nb-NO"),
    (1045, "Polish (Poland)", "PLK", "852", "pl-PL"),
    (1046, "Portuguese (Brazil)", "PTB", "850", "pt-BR"),
    (1048, "Romanian (Romania)", "ROM", "852", "ro-RO"),
    (1049, "Russian (Russia)", "RUS", "866", "ru-RU"),
    (1050, "Croatian (Croatia)", "HRV", "852", "hr-HR"),
    (1051, "Slovak (Slovakia)", "SKY", "852", "sk-SK"),
    (1053, "Swedish (Sweden)", "SVE", "850", "sv-SE"),
    (1054, "Thai (Thailand)", "THA", "874", "th-TH"),
    (1055, "Turkish (Türkiye)", "TRK", "857", "tr-TR"),
    (1057, "Indonesian (Indonesia)", "IND", "850", "id-ID"),
    (1058, "Ukrainian (Ukraine)", "UKR", "866", "uk-UA"),
    (1060, "Slovenian (Slovenia)", "SLV", "852", "sl-SI"),
    (1061, "Estonian (Estonia)", "ETI", "775", "et-EE"),
    (1062, "Latvian (Latvia)", "LVI", "775", "lv-LV"),
    (1063, "Lithuanian (Lithuania)", "LTH", "775", "lt-LT"),
    (1086, "Malay (Malaysia)", "MSL", "850", "ms-MY"),
    (2052, "Chinese (China)", "CHS", "936", "zh-CN"),
    (2055, "German (Switzerland)", "DES", "850", "de-CH"),
    (2057, "English (United Kingdom)", "ENG", "850", "en-GB"),
    (2058, "Spanish (Mexico)", "ESM", "850", "es-MX"),
    (2060, "French (Belgium)", "FRB", "850", "fr-BE"),
    (2064, "Italian (Switzerland)", "ITS", "850", "it-CH"),
    (2067, "Dutch (Belgium)", "NLB", "850", "nl-BE"),
    (2068, "Norwegian Nynorsk (Norway)", "NON", "850", "nn-NO"),
    (2070, "Portuguese (Portugal)", "PTG", "850", "pt-PT"),
    (3079, "German (Austria)", "DEA", "850", "de-AT"),
    (3081, "English (Australia)", "ENA", "850", "en-AU"),
    (3082, "Spanish (Spain)", "ESN", "850", "es-ES"),
    (3084, "French (Canada)", "FRC", "850", "fr-CA"),
    (4105, "English (Canada)", "ENC", "850", "en-CA"),
    (4108, "French (Switzerland)", "FRS", "850", "fr-CH"),
    (5129, "English (New Zealand)", "ENZ", "850", "en-NZ"),
    (6153, "English (Ireland)", "ENI", "850", "en-IE"),
    (9226, "Spanish (Colombia)", "ESO", "850", "es-CO"),
    (10250, "Spanish (Peru)", "ESR", "850", "es-PE"),
    (11274, "Spanish (Argentina)", "ESS", "850", "es-AR"),
    (11290, "Serbian (Latin, Montenegro)", "SRP", "852", "sr-Latn-ME"),
    (13321, "English (Philippines)", "ENP", "437", "en-PH"),
];
/// The abbreviated names a filter may name alone: the table holds exactly one row with each (`count=1` at
/// AgWindowsLanguage.al:167-268), so a filter on one of them takes in no language beyond the row listed.
/// Every listed row but 1025 (ARA, whose count is not pinned).
fn unique_abbreviation(abbr: &str) -> Option<i64> {
    LANGUAGES
        .iter()
        .find(|l| l.2 == abbr && l.0 != 1025)
        .map(|l| i64::from(l.0))
}
/// "STX File Exist" is pinned true for these rows (AgWindowsLanguage.al:110-114); for every other row it is not known.
const STX_FILE_EXISTS: [i32; 5] = [1030, 1031, 1033, 1036, 2057];
/// "Localization Exist" is pinned true for these rows (errors/lt-ms-refusals-128/P128WindowsLanguage.al:46-50); for every
/// other row it is not known.
const LOCALIZATION_EXISTS: [i32; 5] = [1031, 1033, 1036, 1040, 2057];
/// The ids a culture can have: a number above this is no language (TestWindowsLanguageTable.al:63-70
/// asks for 999999 and expects no row).
const MAX_LCID: i64 = 0xFFFF;
const LANGUAGE_ID: u32 = 1;
const ABBREVIATED_NAME: u32 = 11;
/// "Globally Enabled" is true on every row: a filter on it keeps the whole table (AgWindowsLanguage.al:116-118).
const GLOBALLY_ENABLED: u32 = 21;
const STX_FILE_EXIST: u32 = 31;
const LOCALIZATION_EXIST: u32 = 34;

fn num(v: &Value) -> Option<i64> {
    match v {
        Value::Integer(i) => Some(i64::from(*i)),
        _ => None,
    }
}

/// The columns of a Windows Language row the corpus pins: Language ID, Primary Language ID, Name, Abbreviated Name,
/// Primary CodePage and Language Tag (record/TestWindowsLanguageTable.al:9-13), "Globally Enabled" and, for some rows,
/// "STX File Exist" and "Localization Exist" (see above). Every other license and translation column depends on the installation and is not
/// known: a read of it, or a filter on it, is refused.
const LANGUAGE_PINNED: [u32; 9] = [1, 2, 10, 11, 21, 30, 31, 34, 35];

/// Why a filter on a column of Windows Language that no test pins is refused, if one is. A filter on "STX File Exist" or
/// "Localization Exist" passes here; the listing refuses it for a row whose value is not known.
fn language_filter_refusal(t: &TableDef, filters: &[FieldFilter]) -> Result<(), StoreError> {
    match filters
        .iter()
        .find(|f| !LANGUAGE_PINNED.contains(&t.fields[f.field].id))
    {
        Some(f) => Err(refused(format!(
            "Windows Language: a filter on {}, a column no corpus test pins (it depends on the installation)",
            t.fields[f.field].name
        ))),
        None => Ok(()),
    }
}

fn language_row(t: &TableDef, lcid: i32, name: &str, abbr: &str, code_page: &str, tag: &str) -> Row {
    let mut r = blank(t);
    for f in &t.fields {
        if !LANGUAGE_PINNED.contains(&f.id) && f.id < al_builtins::platform::SYSTEM_OBJECT_BASE {
            r[pos(t, f.id)] = super::unknown(t, f.id, &format!("language {lcid}"));
        }
    }
    r[pos(t, 1)] = Value::Integer(lcid);
    // The primary language is the low ten bits of the LCID.
    r[pos(t, 2)] = Value::Integer(lcid & 0x3FF);
    r[pos(t, 10)] = Value::Text(name.into());
    r[pos(t, 11)] = Value::Text(abbr.into());
    r[pos(t, GLOBALLY_ENABLED)] = Value::Boolean(true);
    r[pos(t, STX_FILE_EXIST)] = if STX_FILE_EXISTS.contains(&lcid) {
        Value::Boolean(true)
    } else {
        super::unknown(t, STX_FILE_EXIST, &format!("language {lcid}"))
    };
    r[pos(t, LOCALIZATION_EXIST)] = if LOCALIZATION_EXISTS.contains(&lcid) {
        Value::Boolean(true)
    } else {
        super::unknown(t, LOCALIZATION_EXIST, &format!("language {lcid}"))
    };
    r[pos(t, 30)] = Value::Text(code_page.into());
    r[pos(t, 35)] = Value::Text(tag.into());
    let _ = option;
    r
}

fn known(t: &TableDef, lcid: i64) -> Option<Row> {
    LANGUAGES
        .iter()
        .find(|l| i64::from(l.0) == lcid)
        .map(|l| language_row(t, l.0, l.1, l.2, l.3, l.4))
}

fn unknown_in(set: &[ranges::Iv]) -> Option<i64> {
    let known_ids: Vec<i64> = LANGUAGES.iter().map(|l| i64::from(l.0)).collect();
    for &(a, b) in set {
        let (a, b) = (a.max(1), b.min(MAX_LCID));
        if a > b {
            continue;
        }
        // The first id of the interval that is not a known row.
        let mut id = a;
        while id <= b {
            if !known_ids.contains(&id) {
                return Some(id);
            }
            id += 1;
        }
    }
    None
}

fn language_set(t: &TableDef, filters: &[FieldFilter]) -> Result<ranges::Set, StoreError> {
    ranges::of_field(filters, pos(t, LANGUAGE_ID), &num, i64::MIN / 4, i64::MAX / 4)
        .map_err(|e| refused(format!("Windows Language: {e}")))
}

/// The one language a filter on "Abbreviated Name" equal to a code of [`unique_abbreviation`] can take in.
fn abbreviated_name_row(t: &TableDef, filters: &[FieldFilter]) -> Option<i64> {
    let at = pos(t, ABBREVIATED_NAME);
    filters
        .iter()
        .filter(|f| f.field == at && f.group != al_builtins::filter::OR_GROUP)
        .find_map(|f| {
            let text = |v: &Value| match v {
                Value::Text(s) | Value::Code(s) => Some(s.to_string()),
                _ => None,
            };
            let code = match &f.expr {
                FilterExpr::Eq(v) | FilterExpr::RangeSame(v) => text(v)?,
                FilterExpr::Range(Some(a), Some(b)) if text(a)? == text(b)? => text(a)?,
                _ => return None,
            };
            unique_abbreviation(&code)
        })
}

/// The rows a read with `filters` may take in, listed; refused when the filters take in a language whose row is not pinned.
fn language_listing(t: &TableDef, filters: &[FieldFilter]) -> Result<Mat, StoreError> {
    language_filter_refusal(t, filters)?;
    let set = language_set(t, filters)?;
    let only = abbreviated_name_row(t, filters);
    if only.is_none() {
        if let Some(id) = unknown_in(&set) {
            return Err(refused(format!(
                "Windows Language: the filter takes in language {id}, whose row is not pinned (only the rows of AgWindowsLanguage.al are)"
            )));
        }
    }
    let rows = LANGUAGES
        .iter()
        .filter(|l| only.is_none_or(|id| id == i64::from(l.0)))
        .filter_map(|l| known(t, i64::from(l.0)))
        .map(Entry::ok)
        .collect();
    Ok(Mat::new(t, rows))
}

pub(super) fn windows_language_get(t: &TableDef, pk: &[Value]) -> Result<Option<Row>, StoreError> {
    let id = pk
        .first()
        .and_then(num)
        .ok_or_else(|| refused("Windows Language: a key that is not a number"))?;
    if !(1..=MAX_LCID).contains(&id) {
        return Ok(None);
    }
    match known(t, id) {
        Some(r) => Ok(Some(r)),
        None => Err(refused(format!(
            "Windows Language: whether language {id} is a row is not pinned (only the rows of AgWindowsLanguage.al are)"
        ))),
    }
}

pub(super) fn windows_language_next(t: &TableDef, scan: &Scan) -> Result<Option<Row>, StoreError> {
    language_listing(t, scan.filters)?.next(t, scan)
}

pub(super) fn windows_language_count(t: &TableDef, filters: &[FieldFilter]) -> Result<usize, StoreError> {
    language_listing(t, filters)?.count(t, filters)
}

// ---- License Permission ---------------------------------------------------------------------------

/// The rows the corpus pins (platform/lt-platform-tables/PtCloudTables.al:60-76, LicensePermission_InsertModifyDelete):
/// the first row of the table, `TableData` 0 with every permission blank, and `Table` 99999 with Read, Insert, Modify, Delete
/// and Execute `Yes` and Limited Usage blank. The table lists the permissions of the license, which no test pins beyond
/// these two rows: a read that would have to answer for any other object is refused.
const LICENSE_ROWS: &[(i32, i32, i32)] = &[(0, 0, 0), (1, 99999, 1)];
const LICENSE_OBJECT_TYPE: u32 = 1;
const LICENSE_OBJECT_NUMBER: u32 = 2;
const LICENSE_PERMISSIONS: [u32; 5] = [10, 11, 12, 13, 14];

fn license_row(t: &TableDef, object_type: i32, number: i32, permission: i32) -> Row {
    let mut r = blank(t);
    r[pos(t, LICENSE_OBJECT_TYPE)] = option(t, LICENSE_OBJECT_TYPE, object_type);
    r[pos(t, LICENSE_OBJECT_NUMBER)] = Value::Integer(number);
    for f in LICENSE_PERMISSIONS {
        r[pos(t, f)] = option(t, f, permission);
    }
    r
}

fn license_known(t: &TableDef, object_type: i64, number: i64) -> Option<Row> {
    LICENSE_ROWS
        .iter()
        .find(|(ot, n, _)| i64::from(*ot) == object_type && i64::from(*n) == number)
        .map(|&(ot, n, p)| license_row(t, ot, n, p))
}

fn license_num(v: &Value) -> Option<i64> {
    match v {
        Value::Integer(i) => Some(i64::from(*i)),
        Value::Option(n, _) => Some(i64::from(*n)),
        _ => None,
    }
}

fn license_unpinned(object_type: i64, number: i64) -> StoreError {
    refused(format!(
        "License Permission: whether object type {object_type} number {number} is a row is not pinned (only TableData 0 and Table 99999 are)"
    ))
}

/// The one object a filter names by both key fields, if it does.
fn license_exact(t: &TableDef, filters: &[FieldFilter]) -> Result<Option<(i64, i64)>, StoreError> {
    let one = |id: u32| -> Result<Option<i64>, StoreError> {
        let set = ranges::of_field(filters, pos(t, id), &license_num, i64::MIN / 4, i64::MAX / 4)
            .map_err(|e| refused(format!("License Permission: {e}")))?;
        Ok(match set.as_slice() {
            [(a, b)] if a == b => Some(*a),
            _ => None,
        })
    };
    Ok(one(LICENSE_OBJECT_TYPE)?.zip(one(LICENSE_OBJECT_NUMBER)?))
}

pub(super) fn license_permission_get(t: &TableDef, pk: &[Value]) -> Result<Option<Row>, StoreError> {
    let (Some(ot), Some(n)) = (pk.first().and_then(license_num), pk.get(1).and_then(license_num)) else {
        return Err(refused(
            "License Permission: a key that is not an object type and a number",
        ));
    };
    license_known(t, ot, n).map(Some).ok_or_else(|| license_unpinned(ot, n))
}

pub(super) fn license_permission_next(t: &TableDef, scan: &Scan) -> Result<Option<Row>, StoreError> {
    if let Some((ot, n)) = license_exact(t, scan.filters)? {
        let Some(r) = license_known(t, ot, n) else {
            return Err(license_unpinned(ot, n));
        };
        let after_it = scan.after.is_some();
        return Ok((!after_it && scan.filters.iter().all(|f| f.expr.matches(&r[f.field]))).then_some(r));
    }
    // Nothing else is pinned but the first row of the whole table.
    if scan.filters.is_empty() && scan.ascending && scan.after.is_none() {
        return Ok(Some(license_row(t, 0, 0, 0)));
    }
    Err(refused(
        "License Permission: only a read that names one object by type and number, or the first row, is pinned",
    ))
}

pub(super) fn license_permission_count(t: &TableDef, filters: &[FieldFilter]) -> Result<usize, StoreError> {
    let Some((ot, n)) = license_exact(t, filters)? else {
        return Err(refused(
            "License Permission: only a count that names one object by type and number is pinned",
        ));
    };
    let Some(r) = license_known(t, ot, n) else {
        return Err(license_unpinned(ot, n));
    };
    Ok(usize::from(filters.iter().all(|f| f.expr.matches(&r[f.field]))))
}

/// Why `Get(No.)` of the Time Zone table is refused, if it is. BC lists the 144 Windows time zones, numbered as Windows
/// sorts them (record/lt-at-partner-gaps/AtTimeZoneNumbering.al:41-47: `count = 144`, `2 = UTC-11`, `3 = Aleutian Standard
/// Time`); the runtime lists a few host zones in another order, so which zone a number from 2 to 144 names is not known
/// here. Get(1) keeps the first row of a scan (record/TestTimeZoneVirtualTable.al:GetOne_AgreesWithTheFirstRowOfFindSet)
/// and a number past 144 finds nothing (:GetOnANumberPastTheEnd_ReturnsFalse).
pub(crate) fn time_zone_get_refusal(no: &Value) -> Option<&'static str> {
    matches!(no, Value::Integer(n) if (2..=144).contains(n)).then_some(
        "Get of a Time Zone row by a number from 2 to 144: BC lists 144 Windows time zones in Windows order, the runtime only a few host zones (not pinned which zone a number names)",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_of_a_time_zone_number_after_the_first_is_refused() {
        assert!(time_zone_get_refusal(&Value::Integer(1)).is_none());
        assert!(time_zone_get_refusal(&Value::Integer(2)).is_some());
        assert!(time_zone_get_refusal(&Value::Integer(144)).is_some());
        assert!(time_zone_get_refusal(&Value::Integer(145)).is_none());
        assert!(time_zone_get_refusal(&Value::Integer(999_999)).is_none());
    }
}
