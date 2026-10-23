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

use super::mat::Mat;
use super::{option, ranges, refused};
use al_builtins::filter::{FieldFilter, FilterExpr};
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

// ---- Language Selection --------------------------------------------------------------------------

/// Every row of Language Selection on a fresh tenant, every column (platform/lt-platform-rows-fresh/
/// PlatformRowsLanguageSelection.al:76 and the 26 tests after it): (Name, Language Tag, Language ID, Primary Language ID,
/// Abbreviated Name, Enabled, only from BC 29). The table holds 212 rows from BC 29 and 208 before (:574-586); the four
/// rows only BC 29 has are `<absent>` before it (:238, :377, :401, :489).
const LANGUAGE_SELECTION: &[(&str, &str, i32, i32, &str, bool, bool)] = &[
    ("Afrikaans (South Africa)", "af-ZA", 1078, 1078, "AFK", false, false),
    ("Amharic (Ethiopia)", "am-ET", 1118, 1118, "AMH", false, false),
    (
        "Arabic (United Arab Emirates)",
        "ar-AE",
        14337,
        1025,
        "ARU",
        false,
        false,
    ),
    ("Arabic (Bahrain)", "ar-BH", 15361, 1025, "ARH", false, false),
    ("Arabic (Algeria)", "ar-DZ", 5121, 1025, "ARG", false, false),
    ("Arabic (Egypt)", "ar-EG", 3073, 1025, "ARE", false, false),
    ("Arabic (Iraq)", "ar-IQ", 2049, 1025, "ARI", false, false),
    ("Arabic (Jordan)", "ar-JO", 11265, 1025, "ARJ", false, false),
    ("Arabic (Kuwait)", "ar-KW", 13313, 1025, "ARK", false, false),
    ("Arabic (Lebanon)", "ar-LB", 12289, 1025, "ARB", false, false),
    ("Arabic (Libya)", "ar-LY", 4097, 1025, "ARL", false, false),
    ("Arabic (Morocco)", "ar-MA", 6145, 1025, "ARM", false, false),
    ("Arabic (Oman)", "ar-OM", 8193, 1025, "ARO", false, false),
    ("Arabic (Qatar)", "ar-QA", 16385, 1025, "ARQ", false, false),
    ("Arabic (Saudi Arabia)", "ar-SA", 1025, 1025, "ARA", false, false),
    ("Arabic (Syria)", "ar-SY", 10241, 1025, "ARS", false, false),
    ("Arabic (Tunisia)", "ar-TN", 7169, 1025, "ART", false, false),
    ("Arabic (Yemen)", "ar-YE", 9217, 1025, "ARY", false, false),
    ("Assamese (India)", "as-IN", 1101, 1101, "ASM", false, false),
    (
        "Azerbaijani (Cyrillic, Azerbaijan)",
        "az-Cyrl-AZ",
        2092,
        1068,
        "AZC",
        false,
        false,
    ),
    (
        "Azerbaijani (Latin, Azerbaijan)",
        "az-Latn-AZ",
        1068,
        1068,
        "AZE",
        false,
        false,
    ),
    ("Belarusian (Belarus)", "be-BY", 1059, 1059, "BEL", false, false),
    ("Bulgarian (Bulgaria)", "bg-BG", 1026, 1026, "BGR", true, false),
    ("Bangla (India)", "bn-IN", 1093, 1093, "BNG", false, false),
    ("Tibetan (China)", "bo-CN", 1105, 1105, "BOB", false, false),
    ("Breton (France)", "br-FR", 1150, 1150, "BRE", false, false),
    (
        "Bosnian (Cyrillic, Bosnia & Herzegovina)",
        "bs-Cyrl-BA",
        8218,
        1050,
        "BSC",
        false,
        false,
    ),
    (
        "Bosnian (Latin, Bosnia & Herzegovina)",
        "bs-Latn-BA",
        5146,
        1050,
        "BSB",
        false,
        false,
    ),
    ("Catalan (Spain)", "ca-ES", 1027, 1027, "CAT", false, false),
    ("Czech (Czechia)", "cs-CZ", 1029, 1029, "CSY", true, false),
    ("Welsh (United Kingdom)", "cy-GB", 1106, 1106, "CYM", false, false),
    ("Danish (Denmark)", "da-DK", 1030, 1030, "DAN", true, false),
    ("German (Austria)", "de-AT", 3079, 1031, "DEA", true, false),
    ("German (Germany)", "de-DE", 1031, 1031, "DEU", true, false),
    ("German (Luxembourg)", "de-LU", 4103, 1031, "DEL", false, false),
    ("Lower Sorbian (Germany)", "dsb-DE", 2094, 1070, "DSB", false, false),
    ("Greek (Greece)", "el-GR", 1032, 1032, "ELL", true, false),
    ("English (Australia)", "en-AU", 3081, 1033, "ENA", true, false),
    ("English (Belize)", "en-BZ", 10249, 1033, "ENL", false, false),
    ("English (Canada)", "en-CA", 4105, 1033, "ENC", true, false),
    ("English (United Kingdom)", "en-GB", 2057, 1033, "ENG", true, false),
    (
        "English (Hong Kong SAR China)",
        "en-HK",
        15369,
        1033,
        "ENH",
        false,
        false,
    ),
    ("English (India)", "en-IN", 16393, 1033, "ENN", false, false),
    ("English (Jamaica)", "en-JM", 8201, 1033, "ENJ", false, false),
    ("English (Malaysia)", "en-MY", 17417, 1033, "ENM", false, false),
    ("English (New Zealand)", "en-NZ", 5129, 1033, "ENZ", true, false),
    ("English (Philippines)", "en-PH", 13321, 1033, "ENP", false, false),
    ("English (Singapore)", "en-SG", 18441, 1033, "ENE", false, false),
    ("English (Trinidad & Tobago)", "en-TT", 11273, 1033, "ENT", false, false),
    ("English (United States)", "en-US", 1033, 1033, "ENU", true, false),
    ("English (South Africa)", "en-ZA", 7177, 1033, "ENS", true, false),
    ("English (Zimbabwe)", "en-ZW", 12297, 1033, "ENW", false, false),
    ("Spanish (Latin America)", "es-419", 22538, 1034, "ESJ", false, false),
    ("Spanish (Argentina)", "es-AR", 11274, 1034, "ESS", false, false),
    ("Spanish (Bolivia)", "es-BO", 16394, 1034, "ESB", false, false),
    ("Spanish (Chile)", "es-CL", 13322, 1034, "ESL", false, false),
    ("Spanish (Colombia)", "es-CO", 9226, 1034, "ESO", false, false),
    ("Spanish (Costa Rica)", "es-CR", 5130, 1034, "ESC", false, false),
    ("Spanish (Cuba)", "es-CU", 23562, 1034, "ESK", false, false),
    ("Spanish (Dominican Republic)", "es-DO", 7178, 1034, "ESD", false, false),
    ("Spanish (Ecuador)", "es-EC", 12298, 1034, "ESF", false, false),
    ("Spanish (Spain)", "es-ES", 3082, 1034, "ESN", true, false),
    (
        "Spanish (Spain, Sort Order=tradnl)",
        "es-ES_tradnl",
        1034,
        1034,
        "ESP",
        true,
        false,
    ),
    ("Spanish (Guatemala)", "es-GT", 4106, 1034, "ESG", false, false),
    ("Spanish (Honduras)", "es-HN", 18442, 1034, "ESH", false, false),
    ("Spanish (Nicaragua)", "es-NI", 19466, 1034, "ESI", false, false),
    ("Spanish (Panama)", "es-PA", 6154, 1034, "ESA", false, false),
    ("Spanish (Peru)", "es-PE", 10250, 1034, "ESR", false, false),
    ("Spanish (Puerto Rico)", "es-PR", 20490, 1034, "ESU", false, false),
    ("Spanish (Paraguay)", "es-PY", 15370, 1034, "ESZ", false, false),
    ("Spanish (United States)", "es-US", 21514, 1034, "EST", false, false),
    ("Spanish (Uruguay)", "es-UY", 14346, 1034, "ESY", false, false),
    ("Spanish (Venezuela)", "es-VE", 8202, 1034, "ESV", false, false),
    ("Basque (Spain)", "eu-ES", 1069, 1069, "EUQ", false, false),
    ("Persian (Iran)", "fa-IR", 1065, 1065, "FAR", false, false),
    ("Fula (Latin, Senegal)", "ff-Latn-SN", 2151, 1127, "FUL", false, false),
    ("Fula (Nigeria)", "ff-NG", 1127, 1127, "ZZZ", false, true),
    ("Finnish (Finland)", "fi-FI", 1035, 1035, "FIN", true, false),
    ("Filipino (Philippines)", "fil-PH", 1124, 1124, "FPO", false, false),
    ("French (Belgium)", "fr-BE", 2060, 1036, "FRB", true, false),
    ("French (Canada)", "fr-CA", 3084, 1036, "FRC", true, false),
    ("French (Congo - Kinshasa)", "fr-CD", 9228, 1036, "FRD", false, false),
    ("French (Switzerland)", "fr-CH", 4108, 1036, "FRS", true, false),
    ("French (Côte d’Ivoire)", "fr-CI", 12300, 1036, "FRI", false, false),
    ("French (Cameroon)", "fr-CM", 11276, 1036, "FRE", false, false),
    ("French (France)", "fr-FR", 1036, 1036, "FRA", true, false),
    ("French (Haiti)", "fr-HT", 15372, 1036, "FRH", false, false),
    ("French (Luxembourg)", "fr-LU", 5132, 1036, "FRL", false, false),
    ("French (Morocco)", "fr-MA", 14348, 1036, "FRO", false, false),
    ("French (Réunion)", "fr-RE", 8204, 1036, "FRR", false, false),
    ("French (Senegal)", "fr-SN", 10252, 1036, "FRN", false, false),
    (
        "Western Frisian (Netherlands)",
        "fy-NL",
        1122,
        1122,
        "FYN",
        false,
        false,
    ),
    ("Irish (Ireland)", "ga-IE", 2108, 1084, "IRE", false, false),
    (
        "Scottish Gaelic (United Kingdom)",
        "gd-GB",
        1169,
        1169,
        "GLA",
        false,
        false,
    ),
    ("Galician (Spain)", "gl-ES", 1110, 1110, "GLC", false, false),
    ("Swiss German (France)", "gsw-FR", 1156, 1156, "GSW", false, false),
    ("Gujarati (India)", "gu-IN", 1095, 1095, "GUJ", false, false),
    ("Hawaiian (United States)", "haw-US", 1141, 1141, "HAW", false, false),
    ("Hebrew (Israel)", "he-IL", 1037, 1037, "HEB", false, false),
    ("Hindi (India)", "hi-IN", 1081, 1081, "HIN", false, false),
    (
        "Croatian (Bosnia & Herzegovina)",
        "hr-BA",
        4122,
        1050,
        "HRB",
        false,
        false,
    ),
    ("Croatian (Croatia)", "hr-HR", 1050, 1050, "HRV", true, false),
    ("Upper Sorbian (Germany)", "hsb-DE", 1070, 1070, "HSB", false, false),
    ("Hungarian (Hungary)", "hu-HU", 1038, 1038, "HUN", true, false),
    ("Armenian (Armenia)", "hy-AM", 1067, 1067, "HYE", false, false),
    ("Igbo (Nigeria)", "ig-NG", 1136, 1136, "IBO", false, false),
    ("Sichuan Yi (China)", "ii-CN", 1144, 1144, "III", false, false),
    ("Icelandic (Iceland)", "is-IS", 1039, 1039, "ISL", true, false),
    ("Italian (Switzerland)", "it-CH", 2064, 1040, "ITS", true, false),
    ("Italian (Italy)", "it-IT", 1040, 1040, "ITA", true, false),
    ("Japanese (Japan)", "ja-JP", 1041, 1041, "JPN", true, false),
    ("Georgian (Georgia)", "ka-GE", 1079, 1079, "KAT", false, false),
    ("Kazakh (Kazakhstan)", "kk-KZ", 1087, 1087, "KKZ", false, false),
    ("Kalaallisut (Greenland)", "kl-GL", 1135, 1135, "KAL", false, false),
    ("Khmer (Cambodia)", "km-KH", 1107, 1107, "KHM", false, false),
    ("Kannada (India)", "kn-IN", 1099, 1099, "KDI", false, false),
    ("Korean (South Korea)", "ko-KR", 1042, 1042, "KOR", true, false),
    ("Konkani (India)", "kok-IN", 1111, 1111, "KNK", false, false),
    ("Luxembourgish (Luxembourg)", "lb-LU", 1134, 1134, "LBX", false, false),
    ("Lao (Laos)", "lo-LA", 1108, 1108, "LAO", false, false),
    ("Lithuanian (Lithuania)", "lt-LT", 1063, 1063, "LTH", true, false),
    ("Latvian (Latvia)", "lv-LV", 1062, 1062, "LVI", true, false),
    ("Māori (New Zealand)", "mi-NZ", 1153, 1153, "MRI", false, false),
    ("Macedonian (North Macedonia)", "mk-MK", 1071, 1071, "MKI", false, false),
    ("Malayalam (India)", "ml-IN", 1100, 1100, "MYM", false, false),
    ("Mongolian (Mongolia)", "mn-MN", 1104, 1104, "MNN", false, false),
    ("Malay (Brunei)", "ms-BN", 2110, 1086, "MSB", false, false),
    ("Malay (Malaysia)", "ms-MY", 1086, 1086, "MSL", false, false),
    ("Maltese (Malta)", "mt-MT", 1082, 1082, "MLT", false, false),
    ("Burmese (Myanmar [Burma])", "my-MM", 1109, 1109, "MYA", false, false),
    ("Norwegian Bokmål (Norway)", "nb-NO", 1044, 1044, "NOR", true, false),
    ("Nepali (Nepal)", "ne-NP", 1121, 1121, "NEP", false, false),
    ("Dutch (Belgium)", "nl-BE", 2067, 1043, "NLB", true, false),
    ("Dutch (Netherlands)", "nl-NL", 1043, 1043, "NLD", true, false),
    ("Norwegian Nynorsk (Norway)", "nn-NO", 2068, 1044, "NON", false, false),
    ("Occitan (France)", "oc-FR", 1154, 1154, "OCI", false, false),
    ("Oromo (Ethiopia)", "om-ET", 1138, 1138, "ORM", false, false),
    ("Odia (India)", "or-IN", 1096, 1096, "ORI", false, false),
    (
        "Punjabi (Arabic, Pakistan)",
        "pa-Arab-PK",
        2118,
        1094,
        "PAP",
        false,
        false,
    ),
    ("Punjabi (India)", "pa-IN", 1094, 1094, "PAN", false, true),
    ("Portuguese (Brazil)", "pt-BR", 1046, 1046, "PTB", true, false),
    ("Portuguese (Portugal)", "pt-PT", 2070, 1046, "PTG", true, false),
    ("Romansh (Switzerland)", "rm-CH", 1047, 1047, "RMC", false, false),
    ("Romanian (Moldova)", "ro-MD", 2072, 1048, "ROD", false, false),
    ("Romanian (Romania)", "ro-RO", 1048, 1048, "ROM", true, false),
    ("Russian (Moldova)", "ru-MD", 2073, 1049, "RUM", false, false),
    ("Russian (Russia)", "ru-RU", 1049, 1049, "RUS", true, false),
    ("Kinyarwanda (Rwanda)", "rw-RW", 1159, 1159, "KIN", false, false),
    ("Yakut (Russia)", "sah-RU", 1157, 1157, "SAH", false, false),
    (
        "Sindhi (Arabic, Pakistan)",
        "sd-Arab-PK",
        2137,
        1113,
        "SIP",
        false,
        false,
    ),
    (
        "Sindhi (Devanagari, India)",
        "sd-Deva-IN",
        1113,
        1113,
        "ZZZ",
        false,
        true,
    ),
    ("Northern Sami (Finland)", "se-FI", 3131, 1083, "SMG", false, false),
    ("Northern Sami (Norway)", "se-NO", 1083, 1083, "SME", false, false),
    ("Northern Sami (Sweden)", "se-SE", 2107, 1083, "SMF", false, false),
    ("Sinhala (Sri Lanka)", "si-LK", 1115, 1115, "SIN", false, false),
    ("Slovak (Slovakia)", "sk-SK", 1051, 1051, "SKY", true, false),
    ("Slovenian (Slovenia)", "sl-SI", 1060, 1060, "SLV", true, false),
    ("Inari Sami (Finland)", "smn-FI", 9275, 1083, "SMN", false, false),
    ("Somali (Somalia)", "so-SO", 1143, 1143, "SOM", false, false),
    ("Albanian (Albania)", "sq-AL", 1052, 1052, "SQI", false, false),
    ("Serbian", "sr", 31770, 1050, "SRB", false, false),
    (
        "Serbian (Cyrillic, Montenegro)",
        "sr-Cyrl-ME",
        12314,
        1050,
        "SRQ",
        false,
        false,
    ),
    (
        "Serbian (Cyrillic, Serbia)",
        "sr-Cyrl-RS",
        10266,
        1050,
        "SRO",
        false,
        false,
    ),
    (
        "Serbian (Latin, Bosnia & Herzegovina)",
        "sr-Latn-BA",
        6170,
        1050,
        "SRS",
        false,
        false,
    ),
    (
        "Serbian (Latin, Montenegro)",
        "sr-Latn-ME",
        11290,
        1050,
        "SRP",
        false,
        false,
    ),
    ("Serbian (Latin, Serbia)", "sr-Latn-RS", 9242, 1050, "SRM", true, false),
    ("Swedish (Finland)", "sv-FI", 2077, 1053, "SVF", false, false),
    ("Swedish (Sweden)", "sv-SE", 1053, 1053, "SVE", true, false),
    ("Swahili (Kenya)", "sw-KE", 1089, 1089, "SWK", false, false),
    ("Syriac (Syria)", "syr-SY", 1114, 1114, "SYR", false, false),
    ("Tamil (India)", "ta-IN", 1097, 1097, "TAI", false, false),
    ("Tamil (Sri Lanka)", "ta-LK", 2121, 1097, "TAM", false, false),
    ("Telugu (India)", "te-IN", 1098, 1098, "TEL", false, false),
    ("Thai (Thailand)", "th-TH", 1054, 1054, "THA", true, false),
    ("Tigrinya (Eritrea)", "ti-ER", 2163, 1139, "TIR", false, false),
    ("Tigrinya (Ethiopia)", "ti-ET", 1139, 1139, "TIE", false, false),
    ("Turkmen (Turkmenistan)", "tk-TM", 1090, 1090, "TUK", false, false),
    ("Turkish (Türkiye)", "tr-TR", 1055, 1055, "TRK", true, false),
    ("Tatar (Russia)", "tt-RU", 1092, 1092, "TTT", false, false),
    ("Uyghur (China)", "ug-CN", 1152, 1152, "UIG", false, false),
    ("Ukrainian (Ukraine)", "uk-UA", 1058, 1058, "UKR", true, false),
    ("Urdu (India)", "ur-IN", 2080, 1056, "URI", false, false),
    ("Urdu (Pakistan)", "ur-PK", 1056, 1056, "URD", false, false),
    (
        "Uzbek (Cyrillic, Uzbekistan)",
        "uz-Cyrl-UZ",
        2115,
        1091,
        "UZC",
        false,
        false,
    ),
    ("Vietnamese (Vietnam)", "vi-VN", 1066, 1066, "VIT", true, false),
    ("Xhosa (South Africa)", "xh-ZA", 1076, 1076, "XHO", false, false),
    ("Yoruba (Nigeria)", "yo-NG", 1130, 1130, "YOR", false, false),
    ("Chinese (China)", "zh-CN", 2052, 1028, "CHS", true, false),
    ("Chinese (Traditional)", "zh-Hant", 31748, 1028, "ZHH", false, false),
    ("Chinese (Taiwan)", "zh-TW", 1028, 1028, "CHT", true, true),
    ("Zulu (South Africa)", "zu-ZA", 1077, 1077, "ZUL", false, false),
    ("Bangla (Bangladesh)", "bn-BD", 2117, 1093, "BNB", false, false),
    ("German (Switzerland)", "de-CH", 2055, 1031, "DES", true, false),
    ("German (Liechtenstein)", "de-LI", 5127, 1031, "DEC", false, false),
    ("English (Indonesia)", "en-ID", 14345, 1033, "ZZZ", false, false),
    ("English (Ireland)", "en-IE", 6153, 1033, "ENI", false, false),
    ("Spanish (Mexico)", "es-MX", 2058, 1034, "ESM", true, false),
    ("Spanish (El Salvador)", "es-SV", 17418, 1034, "ESE", false, false),
    ("Estonian (Estonia)", "et-EE", 1061, 1061, "ETI", true, false),
    ("Faroese (Faroe Islands)", "fo-FO", 1080, 1080, "FOS", false, false),
    ("French (Monaco)", "fr-MC", 6156, 1036, "FRM", false, false),
    ("French (Mali)", "fr-ML", 13324, 1036, "FRF", false, false),
    ("Indonesian (Indonesia)", "id-ID", 1057, 1057, "IND", false, false),
    ("Kyrgyz (Kyrgyzstan)", "ky-KG", 1088, 1088, "KYR", false, false),
    ("Marathi (India)", "mr-IN", 1102, 1102, "MAR", false, false),
    ("Nepali (India)", "ne-IN", 2145, 1121, "NEI", false, false),
    ("Polish (Poland)", "pl-PL", 1045, 1045, "PLK", true, false),
    ("Pashto (Afghanistan)", "ps-AF", 1123, 1123, "PAS", false, false),
    ("Sanskrit (India)", "sa-IN", 1103, 1103, "SAN", false, false),
    (
        "Serbian (Cyrillic, Bosnia & Herzegovina)",
        "sr-Cyrl-BA",
        7194,
        1050,
        "SRN",
        false,
        false,
    ),
    (
        "Uzbek (Latin, Uzbekistan)",
        "uz-Latn-UZ",
        1091,
        1091,
        "UZB",
        false,
        false,
    ),
    ("Wolof (Senegal)", "wo-SN", 1160, 1160, "WOL", false, false),
];

/// The rows of Language Selection for the target's major, as [`LANGUAGE_SELECTION`] lists them; the listing sorts them.
pub(super) fn language_selection_rows(t: &TableDef, target: Option<&al_version::Target>) -> Vec<Entry> {
    let from_29 = al_version::gate_on(target, al_version::Gate::LanguageSelectionRowsFrom29);
    LANGUAGE_SELECTION
        .iter()
        .filter(|r| from_29 || !r.6)
        .map(|&(name, tag, id, primary, abbr, enabled, _)| {
            let mut r = blank(t);
            r[pos(t, 1)] = Value::Text(name.into());
            r[pos(t, 2)] = Value::Integer(id);
            r[pos(t, 3)] = Value::Integer(primary);
            r[pos(t, 4)] = Value::Text(abbr.into());
            r[pos(t, 5)] = Value::Boolean(enabled);
            r[pos(t, 6)] = Value::Text(tag.into());
            Entry::ok(r)
        })
        .collect()
}

// ---- License Permission ---------------------------------------------------------------------------

/// The rows the corpus pins: the first row of the table, `TableData` 0 with every permission blank, and `Table` 99999 with
/// Read, Insert, Modify, Delete and Execute `Yes` and Limited Usage blank (platform/lt-platform-tables/PtCloudTables.al:60-76,
/// LicensePermission_InsertModifyDelete); and, read by key, the rows of platform/lt-ag-contoso/AgLicensePermission.al:43-67
/// (LicensePermission_CodeunitRows_ByKey), every one with the same permissions: Codeunit 410 "Update Analysis View"
/// (:43, :55; Cash Flow Wksh.-Register Batch reads it while the Contoso company is built), Codeunit 1, 80 and 99999999,
/// the probe app's own Codeunit 78401, Table 18, TableData 18, Page 21, Report 1, XMLport 1, Query 1 and System 1. The
/// table lists the permissions of the license, which no test pins beyond these rows: a read that would have to answer for
/// any other object is refused. (object type ordinal: TableData 0, Table 1, Report 3, Codeunit 5, XMLport 6, Page 8,
/// Query 9, System 10.)
const LICENSE_ROWS: &[(i32, i32, i32)] = &[
    (0, 0, 0),
    (1, 99999, 1),
    (5, 410, 1),
    (5, 1, 1),
    (5, 80, 1),
    (5, 99_999_999, 1),
    (5, 78401, 1),
    (1, 18, 1),
    (0, 18, 1),
    (8, 21, 1),
    (3, 1, 1),
    (6, 1, 1),
    (9, 1, 1),
    (10, 1, 1),
];
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
        "License Permission: whether object type {object_type} number {number} is a row is not pinned (only the rows platform/lt-platform-tables/PtCloudTables.al and platform/lt-ag-contoso/AgLicensePermission.al read are)"
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
