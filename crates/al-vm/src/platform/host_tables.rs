//! Tables whose rows are facts about the host: Time Zone and Windows Language.

use super::mat::Entry;
use super::{Ident, blank, pos};
use al_builtins::value::Value;
use al_data::TableDef;

/// The Time Zone table lists the host's zones, `No.` from 1 (record/TestTimeZoneVirtualTable.al:
/// more than one row, `No.` running 1 to N without gaps, a non-blank `ID`, `Get(1)` the first row of
/// a scan, `Get` past the end finds nothing). The default host serves the 144 rows of the corpus CI
/// (record/lt-uv-time-zone-table/UvTimeZoneRows.al:42-240; `al_host::time_zone_table`). The real table depends
/// on the host Windows version (UvTimeZoneRows.al:7-11: a Windows SaaS sandbox lists 141 rows); the corpus CI
/// answer is served.
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
/// also record/TestFieldLenAndFeatureKeyContracts.al:83-125. The three lists are the newest minor of each major; 27.0 and
/// 28.0 differ in one row, minors the corpus did not measure are refused. The Description columns are not pinned.
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
        Some(FeatureKeyRows::Bc27 | FeatureKeyRows::Bc27_0) => ROWS_FIRST,
        Some(FeatureKeyRows::Bc28 | FeatureKeyRows::Bc28_0) => ROWS_SECOND,
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
    // 27.0 and 28.0 differ from the newest minor of their major in one row: SemanticMetadataSearch is None on 27.0 (RowsA)
    // and All Users on 28.0 (RowsC), record/PFFeatureKeyColumns.al:96-160.
    let flipped = matches!(
        al_version::feature_key_rows(target),
        Some(FeatureKeyRows::Bc27_0 | FeatureKeyRows::Bc28_0)
    );
    rows.iter()
        .map(|&(id, all_users, version, can_try, one_way, data_update)| {
            let all_users = all_users != (flipped && id == "SemanticMetadataSearch");
            let mut r = blank(t);
            r[pos(t, 1)] = Value::Text(id.into());
            r[pos(t, 2)] = option(t, 2, i32::from(all_users));
            // The wave text of the mandatory-by version: 28.0 is 2026 Wave 1, then one wave per half year.
            let wave = match version {
                28 => "2026 Wave 1 (from April to September 2026)",
                29 => "2026 Wave 2 (from October 2026 to March 2027)",
                30 => "2027 Wave 1 (from April to September 2027)",
                _ => "2027 Wave 2 (from October 2027 to March 2028)",
            };
            r[pos(t, 5)] = Value::Text(wave.into());
            r[pos(t, 6)] = Value::Boolean(can_try);
            r[pos(t, 7)] = Value::Boolean(one_way);
            r[pos(t, 8)] = Value::Boolean(data_update);
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
/// Rows the corpus pins without their code page and tag (a read of either is refused), as (LCID, name, abbreviated name):
/// 12297 English (Zimbabwe), ENW, Globally Enabled, no STX file (types/lt-wc/WcUnpinnedSingles.al:148-173). They are not in
/// [`LANGUAGES`], so a filter on their abbreviated name alone is not taken as naming one row.
const NAME_ONLY_LANGUAGES: &[(i32, &str, &str)] = &[(12297, "English (Zimbabwe)", "ENW")];
/// Rows pinned with their tag but not their code page (a read of it is refused), as (LCID, name, abbreviated name, tag):
/// 1078 Afrikaans (South Africa), AFK, af-ZA (corpus master, corpus PR 871: misc/lt-np4-probes/Np4Misc.al
/// NP4_WindowsLanguage_Get1078). Not in [`LANGUAGES`] either.
const TAG_ONLY_LANGUAGES: &[(i32, &str, &str, &str)] = &[(1078, "Afrikaans (South Africa)", "AFK", "af-ZA")];
/// "STX File Exist" is pinned false for these rows (WcUnpinnedSingles.al:160, :169).
const STX_FILE_ABSENT: [i32; 1] = [12297];
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
/// Ids of the Windows range that are no row: 1, 127 and 4096 (errors/lt-ms-refusals-129/P129WindowsLanguage.al:63-96),
/// 1, 2, 7, 9, 10 and 12 (errors/lt-ms-refusals-137/P137WindowsLanguage.al:24-54, every version). Every other id that is
/// not listed above stays unknown.
const NO_ROW: [i64; 8] = [1, 2, 7, 9, 10, 12, 127, 4096];
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

/// Windows Language."Primary Language ID" per LCID, as measured on real BC (writer Y11, corpus branch
/// al-runtime-rs-staging-y11, record/lt-y11-windows-language/Y11WindowsLanguagePrimary.al; al-runner #6410). It is the
/// LCID of a culture family, not a rule on the LCID's bits (2052 zh-CN answers 1028, 9242 sr-Latn-RS answers 1050):
/// only these ids are answered, and the column of any other row is unknown. Runs on assumption
/// windows-language-primary-language-id until the probe is in the pinned corpus.
/// LCIDs 1028, 3076, 4100 and 5124 are served from the SaaS values, but corpus PR 899 holds them as Linux suspects
/// (non-[Test] procedure), so CI does not assert them.
const PRIMARY_LANGUAGE_IDS: &[(i32, i32)] = &[
    (1025, 1025),
    (1026, 1026),
    (1027, 1027),
    (1028, 1028),
    (1029, 1029),
    (1030, 1030),
    (1031, 1031),
    (1032, 1032),
    (1033, 1033),
    (1034, 1034),
    (1035, 1035),
    (1036, 1036),
    (1037, 1037),
    (1038, 1038),
    (1039, 1039),
    (1040, 1040),
    (1041, 1041),
    (1042, 1042),
    (1043, 1043),
    (1044, 1044),
    (1045, 1045),
    (1046, 1046),
    (1048, 1048),
    (1049, 1049),
    (1050, 1050),
    (1051, 1051),
    (1053, 1053),
    (1054, 1054),
    (1055, 1055),
    (1057, 1057),
    (1058, 1058),
    (1060, 1060),
    (1061, 1061),
    (1062, 1062),
    (1063, 1063),
    (1066, 1066),
    (1069, 1069),
    (1078, 1078),
    (1081, 1081),
    (1086, 1086),
    (1106, 1106),
    (1110, 1110),
    (2052, 1028),
    (2055, 1031),
    (2057, 1033),
    (2058, 1034),
    (2060, 1036),
    (2064, 1040),
    (2067, 1043),
    (2068, 1044),
    (2070, 1046),
    (2077, 1053),
    (2108, 1084),
    (2110, 1086),
    (3073, 1025),
    (3076, 1028),
    (3079, 1031),
    (3081, 1033),
    (3082, 1034),
    (3084, 1036),
    (4100, 1028),
    (4103, 1031),
    (4105, 1033),
    (4108, 1036),
    (5124, 1028),
    (5127, 1031),
    (5129, 1033),
    (5132, 1036),
    (6153, 1033),
    (7177, 1033),
    (8201, 1033),
    (9242, 1050),
    (10249, 1033),
    (10266, 1050),
    (11273, 1033),
    (11274, 1034),
    (12297, 1033),
    (13321, 1033),
    (13322, 1034),
    (14337, 1025),
    (16393, 1033),
    (17417, 1033),
    (18441, 1033),
];

fn language_row(t: &TableDef, lcid: i32, name: &str, abbr: &str, code_page: Option<&str>, tag: Option<&str>) -> Row {
    let mut r = blank(t);
    for f in &t.fields {
        if !LANGUAGE_PINNED.contains(&f.id) && f.id < al_builtins::platform::SYSTEM_OBJECT_BASE {
            r[pos(t, f.id)] = super::unknown(t, f.id, &format!("language {lcid}"));
        }
    }
    r[pos(t, 1)] = Value::Integer(lcid);
    r[pos(t, 2)] = match PRIMARY_LANGUAGE_IDS.binary_search_by_key(&lcid, |p| p.0) {
        Ok(i) => Value::Integer(PRIMARY_LANGUAGE_IDS[i].1),
        Err(_) => super::unknown(t, 2, &format!("language {lcid}")),
    };
    r[pos(t, 10)] = Value::Text(name.into());
    r[pos(t, 11)] = Value::Text(abbr.into());
    r[pos(t, GLOBALLY_ENABLED)] = Value::Boolean(true);
    r[pos(t, STX_FILE_EXIST)] = if STX_FILE_EXISTS.contains(&lcid) {
        Value::Boolean(true)
    } else if STX_FILE_ABSENT.contains(&lcid) {
        Value::Boolean(false)
    } else {
        super::unknown(t, STX_FILE_EXIST, &format!("language {lcid}"))
    };
    r[pos(t, LOCALIZATION_EXIST)] = if LOCALIZATION_EXISTS.contains(&lcid) {
        Value::Boolean(true)
    } else {
        super::unknown(t, LOCALIZATION_EXIST, &format!("language {lcid}"))
    };
    r[pos(t, 30)] = match code_page {
        Some(v) => Value::Text(v.into()),
        None => super::unknown(t, 30, &format!("language {lcid}")),
    };
    r[pos(t, 35)] = match tag {
        Some(v) => Value::Text(v.into()),
        None => super::unknown(t, 35, &format!("language {lcid}")),
    };
    let _ = option;
    r
}

fn known(t: &TableDef, lcid: i64) -> Option<Row> {
    LANGUAGES
        .iter()
        .find(|l| i64::from(l.0) == lcid)
        .map(|l| language_row(t, l.0, l.1, l.2, Some(l.3), Some(l.4)))
        .or_else(|| {
            NAME_ONLY_LANGUAGES
                .iter()
                .find(|l| i64::from(l.0) == lcid)
                .map(|l| language_row(t, l.0, l.1, l.2, None, None))
        })
        .or_else(|| {
            TAG_ONLY_LANGUAGES.iter().find(|l| i64::from(l.0) == lcid).map(|l| {
                let mut r = language_row(t, l.0, l.1, l.2, None, Some(l.3));
                // Whether the row is globally enabled is not pinned for it.
                r[pos(t, GLOBALLY_ENABLED)] = super::unknown(t, GLOBALLY_ENABLED, &format!("language {}", l.0));
                r
            })
        })
}

/// The ids of every pinned row: [`LANGUAGES`] and [`NAME_ONLY_LANGUAGES`].
fn known_ids() -> impl Iterator<Item = i64> {
    LANGUAGES
        .iter()
        .map(|l| i64::from(l.0))
        .chain(NAME_ONLY_LANGUAGES.iter().map(|l| i64::from(l.0)))
        .chain(TAG_ONLY_LANGUAGES.iter().map(|l| i64::from(l.0)))
}

fn unknown_in(set: &[ranges::Iv]) -> Option<i64> {
    let known_ids: Vec<i64> = known_ids().collect();
    for &(a, b) in set {
        let (a, b) = (a.max(1), b.min(MAX_LCID));
        if a > b {
            continue;
        }
        // The first id of the interval that is not a known row.
        let mut id = a;
        while id <= b {
            if !known_ids.contains(&id) && !NO_ROW.contains(&id) {
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
    let rows = known_ids()
        .filter(|&id| only.is_none_or(|o| o == id))
        .filter_map(|id| known(t, id))
        .map(|r| Entry::ok(r).assuming(al_builtins::assumptions::Assumption::WindowsLanguagePrimaryLanguageId))
        .collect();
    Ok(Mat::new(t, rows))
}

pub(super) fn windows_language_get(t: &TableDef, pk: &[Value]) -> Result<Option<Row>, StoreError> {
    let id = pk
        .first()
        .and_then(num)
        .ok_or_else(|| refused("Windows Language: a key that is not a number"))?;
    if !(1..=MAX_LCID).contains(&id) || NO_ROW.contains(&id) {
        return Ok(None);
    }
    match known(t, id) {
        Some(r) => {
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::WindowsLanguagePrimaryLanguageId);
            Ok(Some(r))
        }
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

/// The rows of an app's own objects read by key, pinned on corpus master since PR 809: platform/lt-ag-contoso/AgLicensePermission.al LicensePermission_ByKey_CustomerAndIsvRanges reads
/// Codeunit 50000, 50123, 99999, 70000123 and 2000000000, Table 50000 and 70000123, TableData 70000123, Page 50100 and Report
/// 70000001, each with every permission `Yes`, and Codeunit 0 with every permission blank.
const LICENSE_ROWS_UQ: &[(i32, i32, i32)] = &[
    (5, 50000, 1),
    (5, 50123, 1),
    (5, 99999, 1),
    (5, 70_000_123, 1),
    (5, 2_000_000_000, 1),
    (1, 50000, 1),
    (1, 70_000_123, 1),
    (0, 70_000_123, 1),
    (8, 50100, 1),
    (3, 70_000_001, 1),
    (5, 0, 0),
];

/// The object types pinned by lt-ss1030/SsLicensePermissionObjects.al (corpus branch al-runtime-rs-staging-ss1030 at
/// c36d14ea, not yet on corpus master): TableData, Table, Report, Codeunit, XMLport, Page and Query.
const LICENSE_TYPES_SS: &[i64] = &[0, 1, 3, 5, 6, 8, 9];

/// The Read, Insert, Modify, Delete and Execute permissions lt-ss1030/SsLicensePermissionObjects.al pins for an object of
/// [`LICENSE_TYPES_SS`] that no app declares, and whether it pins Limited Usage (blank) too:
/// - 50123, 99999 and 70000456: every permission `Yes`, Limited Usage blank (:22-33, the `UnusedNumbers` tests).
/// - TableData and Table at 50001, 60000, 70000000, 70000500, 74999999, 80000, 99999998 and 99999999: every permission `Yes`,
///   Limited Usage blank (:56-87, LicensePermission_Table_IsvRangeNumbers and _TableData_IsvRangeNumbers; al-runner issue 6214).
/// - 100000: every permission `Yes`; 1999999999 and 2147483647: every permission blank (:99-103, the `Top` tests, which
///   do not read Limited Usage).
/// - 2000000000: every permission `Yes`; 2000000001: every permission `Yes`, but Table has Delete blank (:104-120). Page
///   at these two numbers has only Execute from the second major on (:106-108, :114-116): not modelled, refused.
fn license_pinned_ss(object_type: i64, number: i64) -> Option<([i32; 5], bool)> {
    if !LICENSE_TYPES_SS.contains(&object_type) {
        return None;
    }
    const YES: [i32; 5] = [1; 5];
    match number {
        50123 | 99999 | 70_000_456 => Some((YES, true)),
        50001 | 60000 | 80000 | 70_000_000 | 70_000_500 | 74_999_999 | 99_999_998 | 99_999_999
            if matches!(object_type, 0 | 1) =>
        {
            Some((YES, true))
        }
        100_000 => Some((YES, false)),
        1_999_999_999 | 2_147_483_647 => Some(([0; 5], false)),
        2_000_000_000 | 2_000_000_001 if object_type == 8 => None,
        2_000_000_001 if object_type == 1 => Some(([1, 1, 1, 0, 1], false)),
        2_000_000_000 | 2_000_000_001 => Some((YES, false)),
        _ => None,
    }
}

/// What a License Permission read answers from: the license of the run (ADR-060), if one was loaded, and the BC target.
#[derive(Clone, Copy)]
pub(super) struct LicenseCx<'a> {
    pub(super) license: Option<&'a al_host::license::License>,
    pub(super) target: Option<&'a al_version::Target>,
}

/// The first number of the system object range. Rows from here on come from the corpus pins only, applied on top of the
/// license (ADR-060 decision 3): BC narrows some system objects itself, and which ones is pinned per number.
const LICENSE_SYSTEM_FIRST: i64 = 2_000_000_000;

/// The highest number a License Permission row can have below the system range.
const LICENSE_BELOW_SYSTEM: i64 = LICENSE_SYSTEM_FIRST - 1;

/// The pinned rows of the tables above, as Read, Insert, Modify, Delete and Execute, and whether Limited Usage is pinned
/// (blank) too. Page at 2000000000 and 2000000001 has Execute only from BC 28, every permission before
/// (lt-ss1030/SsLicensePermissionObjects.al:106-109, :114-117, gate `LicensePermissionTopRangePageFrom28`).
fn license_pinned_perms(
    object_type: i64,
    number: i64,
    target: Option<&al_version::Target>,
) -> Option<([i32; 5], bool)> {
    if let Some(&(_, _, p)) = LICENSE_ROWS
        .iter()
        .chain(LICENSE_ROWS_UQ)
        .find(|(ot, n, _)| i64::from(*ot) == object_type && i64::from(*n) == number)
    {
        return Some(([p; 5], true));
    }
    if object_type == 8 && matches!(number, 2_000_000_000 | 2_000_000_001) {
        return Some(
            if al_version::gate_on(target, al_version::Gate::LicensePermissionTopRangePageFrom28) {
                ([0, 0, 0, 0, 1], false)
            } else {
                ([1; 5], false)
            },
        );
    }
    license_pinned_ss(object_type, number)
}

fn license_row_of(t: &TableDef, object_type: i64, number: i64, perms: [i32; 5], limited: Option<Value>) -> Row {
    let mut r = license_row(t, object_type as i32, number as i32, 0);
    for (f, p) in LICENSE_PERMISSIONS.into_iter().zip(perms) {
        r[pos(t, f)] = option(t, f, p);
    }
    // Limited Usage Permission is looked up by name: no pin names its field number.
    if let (Some(v), Some(f)) = (limited, t.fields.iter().find(|f| f.name == "Limited Usage Permission")) {
        r[pos(t, f.id)] = v;
    }
    r
}

/// The row of a pinned object, when no license file is loaded: what the corpus pins, nothing else (ADR-060 decision 2, "No license
/// available").
fn license_pinned(t: &TableDef, object_type: i64, number: i64, target: Option<&al_version::Target>) -> Option<Row> {
    let (perms, limited_pinned) = license_pinned_perms(object_type, number, target)?;
    let limited = t
        .fields
        .iter()
        .find(|f| f.name == "Limited Usage Permission")
        .filter(|_| !limited_pinned)
        .map(|f| super::unknown(t, f.id, "this object number (the probe does not read it)"));
    Some(license_row_of(t, object_type, number, perms, limited))
}

/// The license list that answers object type `object_type` (ADR-060, "Mapping to the License Permission columns"): the
/// list named as the type, and `Table` from `TableDescription` (inferred: the file has no list named `Table`, and every
/// pinned Table row agrees with TableDescription; docs/probes-needed.md). Other types are refused.
fn license_list(t: &TableDef, object_type: i64) -> Result<&'static str, StoreError> {
    const LISTS: &[(&str, &str)] = &[
        ("TableData", "TableData"),
        ("Table", "TableDescription"),
        ("Report", "Report"),
        ("Codeunit", "Codeunit"),
        ("XMLport", "XMLPort"),
        ("MenuSuite", "MenuSuite"),
        ("Page", "Page"),
        ("Query", "Query"),
        ("System", "System"),
    ];
    let name = match option(t, LICENSE_OBJECT_TYPE, 0) {
        Value::Option(_, m) => usize::try_from(object_type)
            .ok()
            .and_then(|i| m.get(i).cloned())
            .unwrap_or_default(),
        _ => String::new(),
    };
    LISTS
        .iter()
        .find(|(ty, _)| ty.eq_ignore_ascii_case(&name))
        .map(|(_, list)| *list)
        .ok_or_else(|| {
            refused(format!(
                "License Permission: which list of the license answers object type {object_type} ('{name}') is not pinned"
            ))
        })
}

/// The row of `object_type` `number` from the license (ADR-060 decisions 1 and 4): the levels of the range that holds
/// the number, every level blank outside every range; in the system range only what the corpus pins, narrowed further
/// by the pin. Limited Usage is blank for the license of the BC artifacts and unknown for any other (decision 2).
fn license_from_file(
    t: &TableDef,
    object_type: i64,
    number: i64,
    lic: &al_host::license::License,
    target: Option<&al_version::Target>,
) -> Result<Row, StoreError> {
    use al_host::license::Level;
    if !(0..=i64::from(i32::MAX)).contains(&number) {
        return Err(refused(format!(
            "License Permission: whether object type {object_type} number {number} is a row is not pinned (the first row of the table is number 0, platform/lt-platform-rows-fresh/PlatformRowsLicensePermission.al:66)"
        )));
    }
    let list = license_list(t, object_type)?;
    let levels = lic
        .levels(list, number)
        .map_err(|e| refused(format!("License Permission: {e}")))?
        .unwrap_or([Level::Blank; 5]);
    let mut perms = levels.map(|l| match l {
        Level::Blank => 0,
        Level::Yes => 1,
        Level::Indirect => 2,
    });
    if number >= LICENSE_SYSTEM_FIRST {
        let Some((pin, _)) = license_pinned_perms(object_type, number, target) else {
            return Err(refused(format!(
                "License Permission: object type {object_type} number {number} is in the system object range, whose rows are served only where the corpus pins them (ADR-060 decision 3)"
            )));
        };
        for (p, narrowed) in perms.iter_mut().zip(pin) {
            if narrowed == 0 {
                *p = 0;
            }
        }
    }
    let limited_id = t
        .fields
        .iter()
        .find(|f| f.name == "Limited Usage Permission")
        .map(|f| f.id);
    let limited = limited_id.map(|id| {
        if lic.limited_usage_is_artifact_default() {
            option(t, id, 0)
        } else {
            super::unknown(
                t,
                id,
                "a license whose limited-usage list differs from the one of the BC artifacts (ADR-060 open question 2)",
            )
        }
    });
    Ok(license_row_of(t, object_type, number, perms, limited))
}

fn license_known(t: &TableDef, object_type: i64, number: i64, cx: LicenseCx) -> Result<Row, StoreError> {
    match cx.license {
        Some(lic) => license_from_file(t, object_type, number, lic, cx.target),
        None => license_pinned(t, object_type, number, cx.target).ok_or_else(|| license_unpinned(object_type, number)),
    }
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
        "License Permission: whether object type {object_type} number {number} is a row is not pinned (ADR-060: with no license file loaded, only the rows the corpus pins are served)"
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

/// With a license: the object type a read covers and its numbers. Served: filters on the two key fields only, naming one
/// object type; or no filter at all, which starts at TableData 0, the first row of the table
/// (platform/lt-platform-rows-fresh/PlatformRowsLicensePermission.al:66-73).
fn license_domain(t: &TableDef, filters: &[FieldFilter]) -> Result<(i64, ranges::Set), StoreError> {
    let (ty, no) = (pos(t, LICENSE_OBJECT_TYPE), pos(t, LICENSE_OBJECT_NUMBER));
    if filters.iter().any(|f| f.field != ty && f.field != no) {
        return Err(refused(
            "License Permission: a filter on a column other than Object Type and Object Number is not served",
        ));
    }
    if filters.is_empty() {
        // The rows of TableData from number 0 on; a read past them is refused when it gets there.
        return Ok((0, vec![(0, i64::MAX / 4)]));
    }
    let numbers = ranges::of_field(filters, no, &license_num, i64::MIN / 4, i64::MAX / 4)
        .map_err(|e| refused(format!("License Permission: {e}")))?;
    let types = ranges::of_field(filters, ty, &license_num, i64::MIN / 4, i64::MAX / 4)
        .map_err(|e| refused(format!("License Permission: {e}")))?;
    match types.as_slice() {
        [(a, b)] if a == b => Ok((*a, numbers)),
        _ => Err(refused(
            "License Permission: a read over several object types is not served (only one object type, or the first rows of the whole table)",
        )),
    }
}

fn license_out_of_domain(object_type: i64, number: i64) -> StoreError {
    refused(format!(
        "License Permission: a read that reaches object type {object_type} number {number} is not served (with a license, a read over a range covers the numbers 0 to {LICENSE_BELOW_SYSTEM}; ADR-060 decision 3 for the system range)"
    ))
}

pub(super) fn license_permission_get(t: &TableDef, pk: &[Value], cx: LicenseCx) -> Result<Option<Row>, StoreError> {
    let (Some(ot), Some(n)) = (pk.first().and_then(license_num), pk.get(1).and_then(license_num)) else {
        return Err(refused(
            "License Permission: a key that is not an object type and a number",
        ));
    };
    license_known(t, ot, n, cx).map(Some)
}

pub(super) fn license_permission_next(t: &TableDef, scan: &Scan, cx: LicenseCx) -> Result<Option<Row>, StoreError> {
    if let Some((ot, n)) = license_exact(t, scan.filters)? {
        let r = license_known(t, ot, n, cx)?;
        let after_it = scan.after.is_some();
        return Ok((!after_it && scan.filters.iter().all(|f| f.expr.matches(&r[f.field]))).then_some(r));
    }
    if scan.key == 0
        && let Some(lic) = cx.license
    {
        if scan.filters.is_empty() && !scan.ascending {
            return Err(refused(
                "License Permission: the last rows of the whole table are not pinned",
            ));
        }
        let (ot, numbers) = license_domain(t, scan.filters)?;
        let after = scan.after.and_then(|a| {
            let at = (a.first().and_then(license_num)?, a.get(1).and_then(license_num)?);
            Some(at)
        });
        let after_n = match after {
            Some((a_ot, n)) if a_ot == ot => Some(n),
            Some(_) => return Err(license_out_of_domain(ot, -1)),
            None => None,
        };
        let Some(n) = ranges::step(&numbers, after_n, scan.ascending) else {
            return Ok(None);
        };
        if !(0..=LICENSE_BELOW_SYSTEM).contains(&n) {
            return Err(license_out_of_domain(ot, n));
        }
        return license_from_file(t, ot, n, lic, cx.target).map(Some);
    }
    // Nothing else is pinned but the first row of the whole table.
    if scan.filters.is_empty() && scan.ascending && scan.after.is_none() {
        return Ok(Some(license_row(t, 0, 0, 0)));
    }
    Err(refused(
        "License Permission: only a read that names one object by type and number, or the first row, is pinned (no license is loaded)",
    ))
}

pub(super) fn license_permission_count(
    t: &TableDef,
    filters: &[FieldFilter],
    cx: LicenseCx,
) -> Result<usize, StoreError> {
    if let Some((ot, n)) = license_exact(t, filters)? {
        let r = license_known(t, ot, n, cx)?;
        return Ok(usize::from(filters.iter().all(|f| f.expr.matches(&r[f.field]))));
    }
    if cx.license.is_some() && !filters.is_empty() {
        let (ot, numbers) = license_domain(t, filters)?;
        if let Some(&(a, b)) = numbers.iter().find(|&&(a, b)| a < 0 || b > LICENSE_BELOW_SYSTEM) {
            return Err(license_out_of_domain(ot, if a < 0 { a } else { b }));
        }
        // The object type must have a list, or the rows are not served.
        license_list(t, ot)?;
        return Ok(ranges::size(&numbers) as usize);
    }
    Err(refused(
        "License Permission: only a count that names one object by type and number, or one object type over numbers below the system range with a license loaded, is served",
    ))
}

/// A read of the Time Zone table filtered to one Windows time zone id that is not among the host rows: one row with that
/// ID, whose `No.` and `Display Name` are unknown (refused when read). The default host lists every Windows id of CLDR's
/// list (record/lt-uv-time-zone-table/UvTimeZoneRows.al), so this only matters for a host that lists fewer zones. BC on Windows lists every Windows zone, the runtime
/// a few host zones; Pacific, UTC, Arabian, GMT, Eastern and Tokyo each find one row and an unknown id none (corpus branch
/// al-runtime-rs-staging-uq at d651ec93, bump PR 809: record/lt-at-partner-gaps/AtTimeZoneNumbering.al
/// UQ_TimeZone_LookupById). That every other Windows id of CLDR's list is a row is assumed (`time-zone-windows-id-is-a-row`,
/// al-runner issue 6218). `None` leaves the read to the host rows: no such filter, an id among the host rows, or one that
/// is no Windows id (absent, as an unknown id is). An id that differs from a Windows id only in case is refused: whether
/// the filter is case-sensitive depends on the host (not pinned).
pub(crate) fn time_zone_by_id(t: &TableDef, i: &Ident, filters: &[FieldFilter]) -> Result<Option<Mat>, StoreError> {
    let id_col = pos(t, 1);
    let text = |v: &Value| match v {
        Value::Text(s) | Value::Code(s) => Some(s.to_string()),
        _ => None,
    };
    let mut id: Option<String> = None;
    for f in filters.iter().filter(|f| f.field == id_col) {
        let one = match &f.expr {
            FilterExpr::Eq(v) | FilterExpr::RangeSame(v) => text(v),
            FilterExpr::Range(Some(a), Some(b)) if text(a).is_some() && text(a) == text(b) => text(a),
            _ => None,
        };
        match (one, &id) {
            (Some(v), None) if f.group != al_builtins::filter::OR_GROUP => id = Some(v),
            (Some(v), Some(seen)) if &v == seen => {}
            _ => return Ok(None),
        }
    }
    let Some(id) = id else {
        return Ok(None);
    };
    if i.time_zones.iter().any(|z| z.id == id) {
        return Ok(None);
    }
    let zones = al_host::windows_zones::WINDOWS_ZONES;
    if !zones.iter().any(|(w, _)| *w == id) {
        if zones.iter().any(|(w, _)| w.eq_ignore_ascii_case(&id)) {
            return Err(refused(format!(
                "Time Zone: a filter on ID '{id}' that differs from a Windows time zone id only in case (whether the filter is case-sensitive is not pinned)"
            )));
        }
        return Ok(None);
    }
    if filters.iter().any(|f| f.field != id_col) {
        return Err(refused(format!(
            "Time Zone: a read filtered to Windows time zone '{id}', which the runtime does not list, and to another field (its No. and Display Name are not pinned)"
        )));
    }
    let whose = format!("Windows time zone '{id}' (the runtime does not list it; BC on Windows does)");
    let mut r = blank(t);
    r[pos(t, 1)] = Value::Text(id.clone().into());
    r[pos(t, 2)] = super::unknown(t, 2, &whose);
    r[pos(t, 3)] = super::unknown(t, 3, &whose);
    Ok(Some(Mat::new(t, vec![Entry::ok(r)])))
}

/// The culture name of Windows language `id` (`nl-BE` for 2067) from the Windows Language table, for the translation file
/// of that language (ADR-053 decision 3).
pub(crate) fn culture_name(id: i32) -> Option<&'static str> {
    LANGUAGES.iter().find(|l| l.0 == id).map(|l| l.4)
}
