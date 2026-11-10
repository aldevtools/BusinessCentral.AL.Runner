//! `Evaluate` into the platform's option types, and the names of the members AL cannot name. Pinned by
//! format-evaluate/SoEvaluateSystemOptions.al (corpus PR 1219, BC 27.5, 28.5, 29.0), one codeunit per type with the same tests:
//! - a member name in any case, with blanks and TABs around it, or a prefix of one: the first member in ordinal order the text
//!   begins (`Tab` is TableData :98, `Upd` UpdateNoLocks :1180, `Exc` Excel and not ExcelDataset :1446); an exact name first
//!   (`Table` is Table :69); an empty or blank text, or only a TAB, is the first member (:182-203); a name with a character
//!   after it, two names, a TAB inside are rejected (:99-100, :222);
//! - a number, with a `+` and leading zeros, blanks and TABs around it, is the member of that ordinal (:150-173); a number no
//!   member has, a sign `-`, a decimal point, a number past the integer range are rejected (:118-141, :150-173);
//! - members AL cannot name are read from their ordinal: ObjectType 2 Form and 10 Enum (:114-143), ReportFormat 0 None (also
//!   the empty text) and 6 to 8 Jpeg, Xps, Meta, ExecutionContext 2 Uninstall, FieldClass 3 KeyFieldReference from BC 28
//!   (:3560); the error lists them; their names read too (SoEvaluateSystemOptionsHidden.al, corpus PR 1225);
//! - WebServiceActionResultCode reads the numbers 0, 200, 201 and 204 (SoEvaluateSystemOptionsHidden.al:1519);
//! - a rejected text keeps the value and answers false; the statement form raises `'<text>' is not an option. The existing
//!   options are: <names>` (names joined by `, `, a TAB shown as `\t`, the blanks and TABs around the text left out) with
//!   code InvalidOptionString (:208-260, SoEvaluateSystemOptionsHidden.al:68);
//! - format numbers 0, 2 and 9 read alike (:264-289).

use crate::native::{Ctx, Signal, unsupported};
use crate::value::Value;

/// A member as Evaluate reads it: name, platform ordinal, whether AL can name it (`ObjectType::Form` is not in the member list
/// the runtime compiles against).
type Member = (&'static str, i32, bool);

const OBJECT_TYPE: &[Member] = &[
    ("TableData", 0, true),
    ("Table", 1, true),
    ("Form", 2, false),
    ("Report", 3, true),
    ("Codeunit", 5, true),
    ("XmlPort", 6, true),
    ("MenuSuite", 7, true),
    ("Page", 8, true),
    ("Query", 9, true),
    ("Enum", 10, false),
];
const VERBOSITY: &[Member] = &[
    ("Critical", 1, true),
    ("Error", 2, true),
    ("Warning", 3, true),
    ("Normal", 4, true),
    ("Verbose", 5, true),
];
const TELEMETRY_SCOPE: &[Member] = &[("ExtensionPublisher", 1, true), ("All", 2, true)];
// AnonymousData is no member Evaluate reads: 7 is rejected and the error does not list it (:932, :1023).
const DATA_CLASSIFICATION: &[Member] = &[
    ("CustomerContent", 0, true),
    ("ToBeClassified", 1, true),
    ("EndUserIdentifiableInformation", 2, true),
    ("AccountData", 3, true),
    ("EndUserPseudonymousIdentifiers", 4, true),
    ("OrganizationIdentifiableInformation", 5, true),
    ("SystemMetadata", 6, true),
];
const TRANSACTION_TYPE: &[Member] = &[
    ("UpdateNoLocks", 0, true),
    ("Update", 1, true),
    ("Snapshot", 2, true),
    ("Browse", 3, true),
    ("Report", 4, true),
];
const REPORT_FORMAT: &[Member] = &[
    ("None", 0, false),
    ("Excel", 1, true),
    ("Pdf", 2, true),
    ("Word", 3, true),
    ("Xml", 4, true),
    ("Html", 5, true),
    ("Jpeg", 6, false),
    ("Xps", 7, false),
    ("Meta", 8, false),
    ("ExcelDataset", 9, false),
    ("Custom", 10, false),
    ("Zip", 11, false),
];
const ERROR_TYPE: &[Member] = &[("Client", 0, true), ("Internal", 1, true)];
// Custom is no member Evaluate reads (:1888 Names `name Custom`).
const DEFAULT_LAYOUT: &[Member] = &[
    ("None", 0, true),
    ("RDLC", 1, true),
    ("Word", 2, true),
    ("Excel", 3, true),
];
const TEST_PERMISSIONS: &[Member] = &[
    ("InheritFromTestCodeunit", 0, true),
    ("Restrictive", 1, true),
    ("NonRestrictive", 2, true),
    ("Disabled", 3, true),
];
const DATA_SCOPE: &[Member] = &[
    ("Module", 0, true),
    ("Company", 1, true),
    ("User", 2, true),
    ("CompanyAndUser", 3, true),
];
const EXECUTION_CONTEXT: &[Member] = &[
    ("Normal", 0, true),
    ("Install", 1, true),
    ("Uninstall", 2, false),
    ("Upgrade", 3, true),
];
const NOTIFICATION_SCOPE: &[Member] = &[("LocalScope", 0, true), ("GlobalScope", 1, true)];
// Get and Updated are both 200; Updated reads as Get (:3190 Names).
const WEB_SERVICE_ACTION_RESULT_CODE: &[Member] = &[
    ("None", 0, true),
    ("Get", 200, true),
    ("Created", 201, true),
    ("Updated", 200, true),
    ("Deleted", 204, true),
];
const FIELD_CLASS: &[Member] = &[
    ("Normal", 0, true),
    ("FlowField", 1, true),
    ("FlowFilter", 2, true),
    ("KeyFieldReference", 3, false),
];

const TYPES: &[(&str, &[Member])] = &[
    ("objecttype", OBJECT_TYPE),
    ("verbosity", VERBOSITY),
    ("telemetryscope", TELEMETRY_SCOPE),
    ("dataclassification", DATA_CLASSIFICATION),
    ("transactiontype", TRANSACTION_TYPE),
    ("reportformat", REPORT_FORMAT),
    ("errortype", ERROR_TYPE),
    ("defaultlayout", DEFAULT_LAYOUT),
    ("testpermissions", TEST_PERMISSIONS),
    ("datascope", DATA_SCOPE),
    ("executioncontext", EXECUTION_CONTEXT),
    ("notificationscope", NOTIFICATION_SCOPE),
    ("webserviceactionresultcode", WEB_SERVICE_ACTION_RESULT_CODE),
    ("fieldclass", FIELD_CLASS),
];

/// The type and its Evaluate members, when `members` are those of one of the types this module reads.
fn type_of(members: &[String]) -> Option<(&'static str, &'static [Member])> {
    TYPES.iter().copied().find(|(name, _)| {
        crate::sysenum::members(name)
            .is_some_and(|known| members.len() == known.len() && members.iter().zip(known).all(|(a, b)| a == b))
    })
}

/// Whether `Evaluate` into a value with these members is read here.
pub fn reads(members: &[String]) -> bool {
    type_of(members).is_some()
}

/// The name of member `position` that the member list leaves blank or does not hold: one AL cannot name, read by Evaluate
/// from its ordinal (its position is its ordinal).
pub fn hidden_name(members: &[String], position: i32) -> Option<&'static str> {
    let (_, table) = type_of(members)?;
    if members
        .get(usize::try_from(position).ok()?)
        .is_some_and(|m| !m.is_empty())
    {
        return None;
    }
    table
        .iter()
        .find(|(_, o, named)| !named && *o == position)
        .map(|(n, _, _)| *n)
}

/// The name `Format` shows for member `name` of these members when another member has its ordinal first: Updated is the code
/// 200 of Get and shows as Get (SoEvaluateSystemOptionsHidden.al:1576 WebServiceActionResultCode_FormatOfMembers `Updated`
/// `Get/Get/200/200`).
pub fn shown_name(members: &[String], name: &str) -> Option<&'static str> {
    let (_, table) = type_of(members)?;
    let (_, ordinal, _) = table.iter().find(|(n, _, _)| *n == name)?;
    table
        .iter()
        .find(|(_, o, _)| o == ordinal)
        .map(|(n, _, _)| *n)
        .filter(|n| *n != name)
}

/// The position in `members` of the member with platform ordinal `ordinal`.
fn position_of(members: &[String], table: &[Member], ordinal: i32) -> Option<i32> {
    let (name, _, named) = table.iter().find(|(_, o, _)| *o == ordinal)?;
    if !named {
        return Some(ordinal);
    }
    members
        .iter()
        .position(|m| m.eq_ignore_ascii_case(name))
        .and_then(|p| i32::try_from(p).ok())
}

/// `Evaluate(V, text)` into a platform option of these members: the new value, or `None` when the text is rejected (the
/// caller answers false or raises `rejected_error`).
pub fn evaluate(c: &dyn Ctx, members: &std::sync::Arc<[String]>, text: &str) -> Result<Option<Value>, Signal> {
    let Some((name, table)) = type_of(members) else {
        return unsupported("Evaluate into a platform option this runtime does not read");
    };
    let table = live_members(c, name, table);
    if text.chars().any(|ch| ch.is_control() && ch != '\t') {
        return unsupported(format!(
            "Evaluate into {name} of a text with a control character other than TAB (not pinned)"
        ));
    }
    let t = text.trim_matches([' ', '\t']);
    let digits = t.strip_prefix('+').unwrap_or(t);
    let found = if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) {
        // The HTTP codes of WebServiceActionResultCode read as numbers like any ordinal, 200 as Get
        // (SoEvaluateSystemOptionsHidden.al:1519 HttpNumbers, :1538 HttpNumberForms).
        digits
            .parse::<i32>()
            .ok()
            .and_then(|o| table.iter().find(|m| m.1 == o).copied())
    } else {
        let exact = table.iter().find(|m| m.0.eq_ignore_ascii_case(t));
        let prefix = || {
            table.iter().find(|m| {
                m.0.len() >= t.len() && m.0.is_char_boundary(t.len()) && m.0[..t.len()].eq_ignore_ascii_case(t)
            })
        };
        // The name of a member AL cannot name reads like any other, in any case, with blanks and as a prefix
        // (SoEvaluateSystemOptionsHidden.al:120 ObjectType_UnnamableNames1, :640-806 ReportFormat, :1286 ExecutionContext,
        // :1722 FieldClass).
        exact.or_else(prefix).copied()
    };
    Ok(match found {
        Some((_, ordinal, _)) => match position_of(members, table, ordinal) {
            Some(p) => Some(Value::Option(p, members.clone())),
            None => {
                return unsupported(format!(
                    "Evaluate into {name}: ordinal {ordinal} has no place in the member list"
                ));
            }
        },
        None => None,
    })
}

/// The error of the statement form for a rejected `text`.
pub fn rejected_error<T>(c: &dyn Ctx, members: &[String], text: &str) -> Result<T, Signal> {
    let Some((name, table)) = type_of(members) else {
        return unsupported("Evaluate into a platform option this runtime does not read");
    };
    // The message names the text without the blanks and TABs around it (SoEvaluateSystemOptionsHidden.al:68
    // ObjectType_StatementBlanksText, :96 _StatementBlanksNumber; the same for all 14 types).
    let text = text.trim_matches([' ', '\t']);
    let names: Vec<&str> = live_members(c, name, table).iter().map(|m| m.0).collect();
    crate::native::raise(
        format!(
            "'{}' is not an option. The existing options are: {}",
            text.replace('\t', "\\t"),
            names.join(", ")
        ),
        "InvalidOptionString",
    )
}

/// The members of the run's BC version: FieldClass has KeyFieldReference from BC 28 (:3560-3564, :3657-3661).
fn live_members(c: &dyn Ctx, name: &str, table: &'static [Member]) -> &'static [Member] {
    if name == "fieldclass" && !c.gate(al_version::Gate::FieldClassKeyFieldReferenceFrom28) {
        &table[..3]
    } else {
        table
    }
}
