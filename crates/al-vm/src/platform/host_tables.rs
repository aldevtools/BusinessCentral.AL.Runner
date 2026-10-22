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
use al_builtins::filter::FieldFilter;
use al_data::{Row, Scan, StoreError};

/// The languages the corpus pins, as (LCID, name, abbreviated name, primary code page, tag). The table
/// lists every language of the host, which is not pinned beyond these three rows
/// (record/TestWindowsLanguageTable.al:19-60); a read that would have to answer for any other language
/// of the Windows range is refused.
const LANGUAGES: &[(i32, &str, &str, &str, &str)] = &[
    (1031, "German (Germany)", "DEU", "850", "de-DE"),
    (1033, "English (United States)", "ENU", "437", "en-US"),
    (2057, "English (United Kingdom)", "ENG", "850", "en-GB"),
];
/// The ids a culture can have: a number above this is no language (TestWindowsLanguageTable.al:63-70
/// asks for 999999 and expects no row).
const MAX_LCID: i64 = 0xFFFF;
const LANGUAGE_ID: u32 = 1;

fn num(v: &Value) -> Option<i64> {
    match v {
        Value::Integer(i) => Some(i64::from(*i)),
        _ => None,
    }
}

/// The columns of a Windows Language row the corpus pins: Language ID, Primary Language ID, Name, Abbreviated Name,
/// Primary CodePage and Language Tag (record/TestWindowsLanguageTable.al:9-13 asserts only the columns of the culture;
/// the license and translation columns, such as "Globally Enabled" and "STX File Exist", depend on the installation
/// and are left out on purpose). Every other column is not known: a read of it, or a filter on it, is refused.
const LANGUAGE_PINNED: [u32; 6] = [1, 2, 10, 11, 30, 35];

/// Why a filter on a column of Windows Language that no test pins is refused, if one is.
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
            "Windows Language: whether language {id} is a row is not pinned (only 1031, 1033 and 2057 are)"
        ))),
    }
}

pub(super) fn windows_language_next(t: &TableDef, scan: &Scan) -> Result<Option<Row>, StoreError> {
    language_filter_refusal(t, scan.filters)?;
    let set = language_set(t, scan.filters)?;
    if let Some(id) = unknown_in(&set) {
        return Err(refused(format!(
            "Windows Language: the filter takes in language {id}, whose row is not pinned (only 1031, 1033 and 2057 are)"
        )));
    }
    let after = scan.after.and_then(|a| a.first()).and_then(num);
    let mut cur = after;
    while let Some(id) = ranges::step(&set, cur, scan.ascending) {
        if let Some(r) = known(t, id) {
            if scan.filters.iter().all(|f| f.expr.matches(&r[f.field])) {
                return Ok(Some(r));
            }
        }
        cur = Some(id);
    }
    Ok(None)
}

pub(super) fn windows_language_count(t: &TableDef, filters: &[FieldFilter]) -> Result<usize, StoreError> {
    language_filter_refusal(t, filters)?;
    let set = language_set(t, filters)?;
    if let Some(id) = unknown_in(&set) {
        return Err(refused(format!(
            "Windows Language: the filter takes in language {id}, whose row is not pinned (only 1031, 1033 and 2057 are)"
        )));
    }
    Ok(LANGUAGES
        .iter()
        .filter_map(|l| known(t, i64::from(l.0)))
        .filter(|r| filters.iter().all(|f| f.expr.matches(&r[f.field])))
        .count())
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
