//! AL0169: a property set to a value that is not one of its values (`PageType = Sideways;`).
//!
//! The values of each property come from the AL language server of every supported major
//! (`docs/language-catalog.json`, the completion list it offers after `<Property> = `), one row per element kind the
//! list was asked for; `catalog_rows_match_the_language_catalog` keeps the rows equal to that file. The lists are the
//! same for each major, so no row is gated by major. A value is also accepted where `catalog/alc-features.txt` has a
//! row of its own for it (alc compiled it at some runtime: `Importance = Optional` gets AL0666 there, not AL0169), and
//! a value that is not a plain name (a number, a quoted text) is not checked. A property at an element kind without a
//! row is not checked, so no code alc accepts is rejected.
//!
//! The message is the one the must-not-compile corpus pins: "The option value 'Sideways' is not valid. Check the enum
//! definition for valid values." (0313-pagetype-bad-value.al:1, 0762-field-masktype-invalid-value.al:1,
//! 1180-report-layout-obsoletestate-removed.al:1; the other cases of this family pin its first sentence).
use crate::workspace::{AppKind, Diagnostic, Workspace};

/// One property at the element kinds of one owner of the language catalog.
struct Row {
    /// The owner in `docs/language-catalog.json` (`PageField`); read by the test that keeps the rows current.
    #[cfg_attr(not(test), allow(dead_code))]
    owner: &'static str,
    property: &'static str,
    /// The element kinds, as `features::for_each_property` names them (`page/layout/field`).
    ctxs: &'static [&'static str],
    values: &'static [&'static str],
}

const fn row(
    owner: &'static str,
    property: &'static str,
    ctxs: &'static [&'static str],
    values: &'static [&'static str],
) -> Row {
    Row {
        owner,
        property,
        ctxs,
        values,
    }
}

/// Values that are not in the completion list but that a corpus case states: (element kind, property, value).
const PINNED_EXTRA: &[(&str, &str, &str)] = &[
    // Writer TJ's must-not-compile 2248 (branch al-runtime-rs-staging-pub-tj, al-runner issue 6050) names the
    // values of ReadState: ReadUncommitted, ReadCommitted, ReadShared and the default, `Default`.
    ("query", "ReadState", "Default"),
];

#[rustfmt::skip]
const ROWS: &[Row] = &[
    row("Field", "AllowInCustomizations", &["table/fields/field", "tableextension/fields/field"], &["Always", "AsReadOnly", "AsReadWrite", "Never", "ToBeClassified"]),
    row("Table", "AllowInCustomizations", &["table"], &["AsReadOnly", "AsReadWrite", "Never", "ToBeClassified"]),
    row("TableExtension", "AllowInCustomizations", &["tableextension"], &["AsReadOnly", "AsReadWrite", "Never", "ToBeClassified"]),
    row("PageCueGroup", "CuegroupLayout", &["page/layout/cuegroup", "pageextension/layout/cuegroup"], &["Wide"]),
    row("PageCustomAction", "CustomActionType", &["page/actions/customaction", "pageextension/actions/customaction"], &["Flow", "FlowTemplate", "FlowTemplateGallery"]),
    row("XmlPort", "Encoding", &["xmlport"], &["ISO88592", "UTF16", "UTF8"]),
    row("PageAction", "Gesture", &["page/actions/action", "pageextension/actions/action"], &["ContextMenu", "LeftSwipe", "None", "RightSwipe"]),
    row("PageFileUploadAction", "Gesture", &["page/actions/fileuploadaction", "pageextension/actions/fileuploadaction"], &["ContextMenu", "LeftSwipe", "None", "RightSwipe"]),
    row("PageCueGroup", "GridLayout", &["page/layout/cuegroup", "pageextension/layout/cuegroup"], &["Columns", "Rows"]),
    row("PageFixed", "GridLayout", &["page/layout/fixed", "pageextension/layout/fixed"], &["Columns", "Rows"]),
    row("PageGrid", "GridLayout", &["page/layout/grid", "pageextension/layout/grid"], &["Columns", "Rows"]),
    row("PageGroup", "GridLayout", &["page/layout/group", "pageextension/layout/group"], &["Columns", "Rows"]),
    row("PageRepeater", "GridLayout", &["page/layout/repeater", "pageextension/layout/repeater"], &["Columns", "Rows"]),
    row("AllPageCustomization", "Importance", &["pagecustomization/layout/modify:field"], &["Additional", "Promoted", "Standard"]),
    row("PageControlModify", "Importance", &["pageextension/layout/modify:field"], &["Additional", "Promoted", "Standard"]),
    row("PageField", "Importance", &["page/layout/field", "pageextension/layout/field"], &["Additional", "Promoted", "Standard"]),
    row("PageLabel", "Importance", &["page/layout/label", "pageextension/layout/label"], &["Additional", "Promoted", "Standard"]),
    row("AllPageCustomization", "MaskType", &["pagecustomization/layout/modify:field"], &["Concealed", "None"]),
    row("Field", "MaskType", &["table/fields/field", "tableextension/fields/field"], &["Concealed", "None"]),
    row("FieldModify", "MaskType", &["tableextension/fields/modify"], &["Concealed", "None"]),
    row("PageControlModify", "MaskType", &["pageextension/layout/modify:field"], &["Concealed", "None"]),
    row("PageField", "MaskType", &["page/layout/field", "pageextension/layout/field"], &["Concealed", "None"]),
    row("PagePart", "Multiplicity", &["page/layout/part", "pageextension/layout/part"], &["Many", "ZeroOrOne"]),
    row("Codeunit", "ObsoleteState", &["codeunit"], &["No", "Pending"]),
    row("ControlAddInObject", "ObsoleteState", &["controladdin"], &["No", "Pending"]),
    row("EnumType", "ObsoleteState", &["enum"], &["No", "Pending"]),
    row("EnumValue", "ObsoleteState", &["enum/value", "enumextension/value"], &["No", "Pending"]),
    row("Field", "ObsoleteState", &["table/fields/field", "tableextension/fields/field"], &["Moved", "No", "Pending", "PendingMove", "Removed"]),
    row("FieldGroup", "ObsoleteState", &["table/fieldgroups/fieldgroup"], &["No", "Pending"]),
    row("Interface", "ObsoleteState", &["interface"], &["No", "Pending"]),
    row("Key", "ObsoleteState", &["table/keys/key"], &["No", "Pending", "Removed"]),
    row("Page", "ObsoleteState", &["page"], &["No", "Pending"]),
    row("PageAction", "ObsoleteState", &["page/actions/action", "pageextension/actions/action"], &["No", "Pending"]),
    row("PageActionArea", "ObsoleteState", &["page/actions/area", "pageextension/actions/area"], &["No", "Pending"]),
    row("PageActionGroup", "ObsoleteState", &["page/actions/group", "pageextension/actions/group"], &["No", "Pending"]),
    row("PageActionRef", "ObsoleteState", &["page/actions/actionref", "pageextension/actions/actionref"], &["No", "Pending"]),
    row("PageActionSeparator", "ObsoleteState", &["page/actions/separator", "pageextension/actions/separator"], &["No", "Pending"]),
    row("PageArea", "ObsoleteState", &["page/layout/area", "pageextension/layout/area"], &["No", "Pending"]),
    row("PageCueGroup", "ObsoleteState", &["page/layout/cuegroup", "pageextension/layout/cuegroup"], &["No", "Pending"]),
    row("PageCustomAction", "ObsoleteState", &["page/actions/customaction", "pageextension/actions/customaction"], &["No", "Pending"]),
    row("PageCustomizationObject", "ObsoleteState", &["pagecustomization"], &["No", "Pending"]),
    row("PageExtension", "ObsoleteState", &["pageextension"], &["No", "Pending"]),
    row("PageField", "ObsoleteState", &["page/layout/field", "pageextension/layout/field"], &["No", "Pending"]),
    row("PageFileUploadAction", "ObsoleteState", &["page/actions/fileuploadaction", "pageextension/actions/fileuploadaction"], &["No", "Pending"]),
    row("PageFixed", "ObsoleteState", &["page/layout/fixed", "pageextension/layout/fixed"], &["No", "Pending"]),
    row("PageGrid", "ObsoleteState", &["page/layout/grid", "pageextension/layout/grid"], &["No", "Pending"]),
    row("PageGroup", "ObsoleteState", &["page/layout/group", "pageextension/layout/group"], &["No", "Pending"]),
    row("PageLabel", "ObsoleteState", &["page/layout/label", "pageextension/layout/label"], &["No", "Pending"]),
    row("PagePart", "ObsoleteState", &["page/layout/part", "pageextension/layout/part"], &["No", "Pending"]),
    row("PageRepeater", "ObsoleteState", &["page/layout/repeater", "pageextension/layout/repeater"], &["No", "Pending"]),
    row("PageSystemPart", "ObsoleteState", &["page/layout/systempart", "pageextension/layout/systempart"], &["No", "Pending"]),
    row("PageUserControl", "ObsoleteState", &["page/layout/usercontrol", "pageextension/layout/usercontrol"], &["No", "Pending"]),
    row("PageView", "ObsoleteState", &["page/views/view", "pageextension/views/view"], &["No", "Pending"]),
    row("PermissionSet", "ObsoleteState", &["permissionset"], &["No", "Pending"]),
    row("Profile", "ObsoleteState", &["profile"], &["No", "Pending"]),
    row("Query", "ObsoleteState", &["query"], &["No", "Pending"]),
    row("QueryColumn", "ObsoleteState", &["query/elements/column"], &["No", "Pending"]),
    row("QueryFilter", "ObsoleteState", &["query/elements/filter"], &["No", "Pending"]),
    row("Report", "ObsoleteState", &["report"], &["No", "Pending"]),
    row("ReportColumn", "ObsoleteState", &["report/dataset/column"], &["No", "Pending"]),
    row("ReportDataItem", "ObsoleteState", &["report/dataset/dataitem"], &["No", "Pending"]),
    row("ReportLayout", "ObsoleteState", &["report/rendering/layout", "reportextension/rendering/layout"], &["No", "Pending"]),
    row("RequestPage", "ObsoleteState", &["report/requestpage"], &["No", "Pending"]),
    row("Table", "ObsoleteState", &["table"], &["Moved", "No", "Pending", "PendingMove", "Removed"]),
    row("XmlPort", "ObsoleteState", &["xmlport"], &["No", "Pending"]),
    row("XmlPortFieldAttribute", "Occurrence", &["xmlport/schema/fieldattribute"], &["Optional", "Required"]),
    row("XmlPortTextAttribute", "Occurrence", &["xmlport/schema/textattribute"], &["Optional", "Required"]),
    row("Page", "PageType", &["page"], &["API", "Card", "CardPart", "ConfigurationDialog", "ConfirmationDialog", "Document", "HeadlinePart", "List", "ListPart", "ListPlus", "NavigatePage", "PromptDialog", "ReportPreview", "ReportProcessingOnly", "RoleCenter", "StandardDialog", "UserControlHost", "Worksheet", "XmlPort"]),
    row("Report", "PaperSourceDefaultPage", &["report"], &["AutomaticFeed", "Cassette", "Custom1", "Custom10", "Custom11", "Custom12", "Custom13", "Custom14", "Custom15", "Custom16", "Custom2", "Custom3", "Custom4", "Custom5", "Custom6", "Custom7", "Custom8", "Custom9", "Envelope", "FormSource", "LargeCapacity", "LargeFormat", "Lower", "Manual", "ManualFeed", "Middle", "SmallFormat", "TractorFeed", "Upper"]),
    row("Report", "PaperSourceFirstPage", &["report"], &["AutomaticFeed", "Cassette", "Custom1", "Custom10", "Custom11", "Custom12", "Custom13", "Custom14", "Custom15", "Custom16", "Custom2", "Custom3", "Custom4", "Custom5", "Custom6", "Custom7", "Custom8", "Custom9", "Envelope", "FormSource", "LargeCapacity", "LargeFormat", "Lower", "Manual", "ManualFeed", "Middle", "SmallFormat", "TractorFeed", "Upper"]),
    row("Report", "PaperSourceLastPage", &["report"], &["AutomaticFeed", "Cassette", "Custom1", "Custom10", "Custom11", "Custom12", "Custom13", "Custom14", "Custom15", "Custom16", "Custom2", "Custom3", "Custom4", "Custom5", "Custom6", "Custom7", "Custom8", "Custom9", "Envelope", "FormSource", "LargeCapacity", "LargeFormat", "Lower", "Manual", "ManualFeed", "Middle", "SmallFormat", "TractorFeed", "Upper"]),
    row("Report", "PdfFontEmbedding", &["report"], &["Default", "No", "Yes"]),
    row("Page", "PromptMode", &["page"], &["Content", "Generate", "Prompt"]),
    row("Query", "ReadState", &["query"], &["ReadCommitted", "ReadExclusive", "ReadShared", "ReadUncommitted"]),
    row("Codeunit", "RequiredTestIsolation", &["codeunit"], &["Codeunit", "Disabled", "Function", "None"]),
    row("Entitlement", "RoleType", &["entitlement"], &["Delegated", "Local"]),
    row("PageAction", "Scope", &["page/actions/action", "pageextension/actions/action"], &["Page", "Repeater"]),
    row("PageCustomAction", "Scope", &["page/actions/customaction", "pageextension/actions/customaction"], &["Page", "Repeater"]),
    row("PageFileUploadAction", "Scope", &["page/actions/fileuploadaction", "pageextension/actions/fileuploadaction"], &["Page", "Repeater"]),
    row("PageActionGroup", "ShowAs", &["page/actions/group", "pageextension/actions/group"], &["SplitButton", "Standard"]),
    row("PageControlModify", "Style", &["pageextension/layout/modify:field"], &["Ambiguous", "Attention", "AttentionAccent", "Favorable", "None", "Standard", "StandardAccent", "Strong", "StrongAccent", "Subordinate", "Unfavorable"]),
    row("PageField", "Style", &["page/layout/field", "pageextension/layout/field"], &["Ambiguous", "Attention", "AttentionAccent", "Favorable", "None", "Standard", "StandardAccent", "Strong", "StrongAccent", "Subordinate", "Unfavorable"]),
    row("PageLabel", "Style", &["page/layout/label", "pageextension/layout/label"], &["Ambiguous", "Attention", "AttentionAccent", "Favorable", "None", "Standard", "StandardAccent", "Strong", "StrongAccent", "Subordinate", "Unfavorable"]),
    row("Codeunit", "TestType", &["codeunit"], &["AITest", "IntegrationTest", "Uncategorized", "UnitTest"]),
    row("PageRepeater", "TreeInitialState", &["page/layout/repeater", "pageextension/layout/repeater"], &["CollapseAll", "ExpandAll"]),
    row("PagePart", "UpdatePropagation", &["page/layout/part", "pageextension/layout/part"], &["Both", "SubPart"]),
    row("PageSystemPart", "UpdatePropagation", &["page/layout/systempart", "pageextension/layout/systempart"], &["Both", "SubPart"]),
    row("XmlPort", "XmlVersionNo", &["xmlport"], &["V10", "V11"]),
];

/// Whether `value` is a value of `property` at element kind `ctx`; `None` when no row covers the property there.
fn allowed(ctx: &str, property: &str, value: &str) -> Option<bool> {
    let mut covered = false;
    for r in ROWS
        .iter()
        .filter(|r| r.property.eq_ignore_ascii_case(property) && r.ctxs.contains(&ctx))
    {
        covered = true;
        if r.values.iter().any(|v| v.eq_ignore_ascii_case(value)) {
            return Some(true);
        }
    }
    if !covered {
        return None;
    }
    let extra = PINNED_EXTRA
        .iter()
        .any(|(c, p, v)| *c == ctx && p.eq_ignore_ascii_case(property) && v.eq_ignore_ascii_case(value));
    Some(extra || al_builtins::features::has_value_row(ctx, property, value))
}

/// The AL0169 diagnostics of the apps under test (never a dependency: alc compiled it already).
pub fn check(ws: &Workspace, out: &mut Vec<Diagnostic>) {
    for (oi, o) in ws.objects.iter().enumerate() {
        if ws.kinds[o.app] == AppKind::Dependency {
            continue;
        }
        crate::features::for_each_property(ws, oi, |ctx, p| {
            let value = p.value_text();
            let value = value.trim();
            // Only a plain name: a number (`PaperSourceFirstPage = 1`) or any other form is not checked.
            if !value.starts_with(|c: char| c.is_ascii_alphabetic())
                || !value.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
            {
                return;
            }
            if allowed(ctx, &p.name(), value) == Some(false) {
                out.push(Diagnostic {
                    file: ws.files[o.file].path.clone(),
                    app: ws.files[o.file].app,
                    offset: p.syntax().text_range().start().into(),
                    code: "AL0169",
                    message: format!(
                        "The option value '{value}' is not valid. Check the enum definition for valid values."
                    ),
                });
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every row holds the values the language server lists for its owner and property, for every major.
    #[test]
    fn catalog_rows_match_the_language_catalog() {
        let text = include_str!("../../../docs/language-catalog.json");
        let json: serde_json::Value = serde_json::from_str(text).expect("language-catalog.json parses");
        let items = json["items"].as_array().expect("items");
        for r in ROWS {
            let item = items
                .iter()
                .find(|i| i["area"] == "properties" && i["owner"] == r.owner && i["name"] == r.property)
                .unwrap_or_else(|| panic!("{} {} is not in the language catalog", r.owner, r.property));
            let values: Vec<&str> = item["values"]
                .as_array()
                .expect("values")
                .iter()
                .map(|v| v.as_str().expect("a value"))
                .collect();
            assert_eq!(values, r.values, "{} {}", r.owner, r.property);
            let majors: Vec<u64> = item["bc"]
                .as_array()
                .expect("bc")
                .iter()
                .filter_map(|v| v.as_u64())
                .collect();
            assert_eq!(
                majors,
                [27, 28, 29],
                "{} {}: the values differ by major",
                r.owner,
                r.property
            );
        }
    }

    #[test]
    fn values_outside_the_rows_are_not_checked() {
        assert_eq!(allowed("page", "PageType", "Sideways"), Some(false));
        assert_eq!(allowed("page", "pagetype", "card"), Some(true));
        assert_eq!(allowed("page/layout/field", "Importance", "Optional"), Some(true));
        assert_eq!(allowed("query", "ReadState", "Default"), Some(true));
        // No row: not checked.
        assert_eq!(allowed("report/requestpage/layout/field", "Style", "Red"), None);
        assert_eq!(allowed("page/layout/cuegroup", "TreeInitialState", "HalfOpen"), None);
    }
}
