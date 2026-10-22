//! Compile errors with the AL compiler's own codes (`AL0xxx`), for the must-not-compile corpus
//! (`tests/must-not-compile` of the corpus repo, branch `master`; ADR-009 and the owner
//! decision of 2026-10-14: our diagnostics for code BC rejects use BC's codes, and tests pin them).
//!
//! Rule 6 of `CLAUDE.md` applies in the other direction too: a check here rejects only a shape the
//! corpus case pins, never a guess at a wider rule. Where a check needs what the front end does not
//! have (the members of a .NET type, a dependency's source), the code is listed in [`NOT_CHECKED`]
//! and no diagnostic is produced.
//!
//! Two kinds of checks:
//! - [`check_workspace`] needs the source only. `check::check_with` runs it, so every compile has
//!   its diagnostics. It covers the apps of kind `Workspace`, never a dependency (the AL compiler
//!   has already vetted those).
//! - [`check_context`] needs facts about the app that `AppSource` does not carry (its target and
//!   runtime, the keys of its app.json). The caller supplies them; a compile without them skips
//!   these checks.
//!
//! The codes the checker itself reports (`check/`, where types are known) are AL0118, AL0122,
//! AL0132, AL0161 and AL0398 (and AL0175 for Char - Text); they are listed in [`IMPLEMENTED`] too.

use crate::workspace::{AppKind, Diagnostic, ObjKind, Resolution, Workspace};
use al_syntax::{SyntaxKind, SyntaxNode, ast};

/// The AL codes this front end reports, with what each one covers.
pub const IMPLEMENTED: &[(&str, &str)] = &[
    (
        "AL0104",
        "a parameter named begin, procedure or trigger without `var` (the first diagnostic, at the name); alc's full list for a file of two codeunits whose procedures take `<keyword>: Integer` and `var <keyword>: Integer` (cases 2016-2039), and for a variable \"If\" used unquoted (case 2040)",
    ),
    (
        "AL0105",
        "a parameter named by one of 20 other keywords without `var` (the first diagnostic, at the name); alc's full list for the two-codeunit keyword-parameter file (cases 2016-2039)",
    ),
    (
        "AL0107",
        "a parameter named by one of 24 keywords after `var` (the first diagnostic, at the name); alc's full list for the two-codeunit keyword-parameter file (cases 2016-2039)",
    ),
    (
        "AL0111",
        "alc's full list for the two-codeunit file whose procedures take a parameter named begin (case 2016)",
    ),
    (
        "AL0224",
        "a variable declared \"If\" used unquoted, `If := 1;`, as the one statement of the one procedure of a file's one codeunit (case 2040)",
    ),
    (
        "AL0118",
        "a bare call of a name that is not a procedure or system function, in a codeunit; an undeclared bare name in a codeunit, or assigned in a trigger of a table, page, report, query or xmlport; Codeunit::\"X\" or an event subscriber naming a codeunit nobody declares",
    ),
    (
        "AL0122",
        "an enum assigned to an enum of another type whose target does not state AssignmentCompatibility = true; an Integer variable assigned to a Text; a text literal of two or more characters assigned to an Integer; Decimal or Integer to Boolean, Date to Text, Boolean, Text or Decimal to Date, Variant or a one-character literal to an Enum, a Record to an Integer",
    ),
    (
        "AL0124",
        "app.json key `assemblyProbingPaths` of a Cloud app at runtime 15.0, 16.0, 16.1, 17.0, 17.1 and 18.0; the property OrderBy on a query data item; the property AutoCalcField on an xmlport table element; a property the object type does not have (any outside the 16 codeunit properties, SourceTable and others on a table, Editable on a report, TableNo on a page, ...); InitValue on a page field",
    ),
    (
        "AL0132",
        "ExecutionMode::Install, Uninstall, Upgrade, Report, Profile; a field or method of a record that the table and the catalog lack; a method the catalog lacks on Text, Code, Boolean, List, Dictionary, JsonObject, JsonToken, HttpClient, RecordRef or TextBuilder; a value an enum of the workspace lacks; a page field Rec.X the source table lacks",
    ),
    (
        "AL0161",
        "a page extension names a non-protected variable or a local procedure of its base page, or a report extension does; a Cloud app names an internal agent table of the platform; a subscriber to the internal event GetRoleCenterBannerPartID of System Action Triggers; a page customization names a variable of its page in Editable of a modify",
    ),
    (
        "AL0162",
        "OnInit, OnFindRecord and OnNextRecord in a page extension; a trigger the object type does not have (any but OnRun in a codeunit; page, table and report triggers of another object type)",
    ),
    ("AL0197", "two objects of one type and name in one app"),
    (
        "AL0296",
        "a Cloud app names a platform table that is Scope = OnPrem (the 123 of System.app that the corpus lists), calls GetLastErrorObject, SecretText.Unwrap, or File.Open, or sets the report property EnableExternalAssemblies to true",
    ),
    (
        "AL0398",
        "a Decimal literal as a case label of an Option variable, or in the `in` list (Decimal literals only) of an Option variable",
    ),
    (
        "AL0433",
        "a Cloud app names a removed platform table (the 14 of System.app that the corpus lists)",
    ),
    (
        "AL0365",
        "DataItemLink on a query data item with SqlJoinType = CrossJoin",
    ),
    (
        "AL0385",
        "IncludeCaption = true on a report column whose source is an expression",
    ),
    (
        "AL0440",
        "a page or report extension procedure, or a second procedure of a codeunit, table, page or report, with the signature of a procedure it could reach",
    ),
];

/// Codes the must-not-compile corpus expects and this front end cannot decide, with the reason. AL0175 is reported
/// for `Char - Text` and `Text - Char` on two variables only (case 0500), AL0666 for catalog methods (`check/calls.rs`) and
/// the features `catalog/alc-features.txt` measured (`features.rs`);
/// a case that finds its code matches.
pub const NOT_CHECKED: &[(&str, &str)] = &[
    (
        "AL0175",
        "needs the .NET type behind the alias (an enum is comparable with `=`, a class object is not); the front end has no assembly metadata.",
    ),
    (
        "AL0666",
        "reported for a catalog method newer than the app's runtime (check/calls.rs: cases 0364, 0365, 0576, 0620, 0621, 0640, 0664, 0665, 1200, 1824) and for the properties, data types, attributes and object types alc-features.txt measured (features.rs: cases 0684, 0777, 0778, 0799 to 0804, 0995 to 0997, 1431); a feature or element kind the table has no AL0666 row for is not decided.",
    ),
];

/// Why `code` is not checked, if it is listed in [`NOT_CHECKED`]; `Some("not implemented")` for a
/// code in neither list.
pub fn not_checked_reason(code: &str) -> Option<&'static str> {
    if IMPLEMENTED.iter().any(|(c, _)| *c == code) {
        return None;
    }
    Some(
        NOT_CHECKED
            .iter()
            .find(|(c, _)| *c == code)
            .map_or("not implemented", |(_, why)| why),
    )
}

/// The target of an app (`app.json` `target`; the compiler's default is Cloud).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppTarget {
    Cloud,
    OnPrem,
    Extension,
    Internal,
}

/// The app runtimes (major, minor) at which the compiler refuses `assemblyProbingPaths` (corpus 0007, 0071 to 0073).
///
/// 17.1 and 18.0 are the runtimes of the corpus apps on BC 28 and BC 29 (`oracle.toml` `app_runtime`); corpus
/// case 0007 (default runtime of its leg) gets AL0124 on them when the corpus checker runs with AL 30.0.42.60748-beta.
const ASSEMBLY_PROBING_REFUSED: &[(u32, u32)] = &[(15, 0), (16, 0), (16, 1), (17, 0), (17, 1), (18, 0)];

/// What the compiler knows of an app besides its source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppContext {
    pub target: AppTarget,
    /// `runtime` of app.json as (major, minor).
    pub runtime: Option<(u32, u32)>,
    /// The top-level keys of app.json.
    pub app_json_keys: Vec<String>,
    /// The BC major the app is compiled for (the platform symbols it sees); `None` when not known.
    pub bc_major: Option<u32>,
}

/// Platform tables declared `Scope = OnPrem`. Pinned by the must-not-compile corpus: cases 0006
/// (`Published Application`) and 0014 (`API Webhook Entity`), BC 27.5 compiler. Other platform
/// tables may be OnPrem too; none is guessed (docs/probes-needed.md).
pub const ONPREM_PLATFORM_TABLES: &[&str] = &["Published Application", "API Webhook Entity"];

/// The platform tables of System.app (BC 27.0) with `Scope = OnPrem`: naming one from a Cloud app is AL0296. Corpus
/// must-not-compile 0020 to 0024 name all 123, 30 per case, with the symbols of BC 27 only, so the verdict is not
/// known for another major. Sorted by name.
pub const ONPREM_PLATFORM_TABLES_27: &[&str] = &[
    "API Webhook Entity",
    "API Webhook Notification",
    "API Webhook Notification Aggr",
    "API Webhook Subscription",
    "Agent",
    "Agent Access Control Data",
    "Agent Data",
    "Agent Task Data",
    "Agent Task File Data",
    "Agent Task Log Entry",
    "Agent Task Log Entry Data",
    "Agent Task Memory Entry",
    "Agent Task Memory Entry Data",
    "Agent Task Message Data",
    "Agent Task Msg Attach Data",
    "Agent Task Page Script",
    "Agent Task Timeline",
    "Agent Task Timeline Step",
    "Agent Task Timeline Step Data",
    "Agent Task Timeline Step Det.",
    "Agent Task User Int. Data",
    "Agent Troubleshoot Info Data",
    "Application Dependency",
    "Application Object Metadata",
    "Application Resource",
    "Configuration Package File",
    "Data Sensitivity",
    "Database",
    "Debugger Breakpoint",
    "Debugger Call Stack",
    "Debugger Variable",
    "Debugger Watch",
    "Debugger Watch Value",
    "Designed Query",
    "Designed Query Caption",
    "Designed Query Category",
    "Designed Query Column",
    "Designed Query Column Filter",
    "Designed Query Data Item",
    "Designed Query Filter",
    "Designed Query Group",
    "Designed Query Join",
    "Designed Query Management",
    "Designed Query Obj",
    "Designed Query Order By",
    "Device",
    "Document Service",
    "Document Service Scenario",
    "Drive",
    "Entitlement",
    "Entitlement Set",
    "Ext. Business Event Definition",
    "Extension Database Snapshot",
    "Extension Execution Info",
    "External Event Activity Log",
    "External Event Log Entry",
    "External Event Notification",
    "External Event Subscription",
    "File",
    "Inplace Installed Application",
    "Installed Application",
    "Intelligent Cloud",
    "Intelligent Cloud Status",
    "Isolated Storage",
    "MCP Configuration",
    "MCP Configuration Tool",
    "Media",
    "Media Set",
    "Membership Entitlement",
    "NAV App",
    "NAV App Capabilities",
    "NAV App Data Archive",
    "NAV App Dependencies",
    "NAV App Extra",
    "NAV App Object Metadata",
    "NAV App Object Prerequisites",
    "NAV App Publish Reference",
    "NAV App Published App",
    "NAV App Resource",
    "NAV App Tenant App",
    "NAV App Tenant Operation",
    "Object",
    "Object Metadata",
    "Page Documentation",
    "Page Info And Fields",
    "Page Table Field",
    "Page Usage State",
    "Performance Profile Scheduler",
    "Performance Profiles",
    "Permission Range",
    "Power BI Blob",
    "Power BI Default Selection",
    "Profile",
    "Profile Configuration Symbols",
    "Profile Metadata",
    "Profile Page Metadata",
    "Published Application",
    "Query Navigation",
    "Report Printer",
    "Report Settings Override",
    "SID - Account ID",
    "Send-To Program",
    "Server Instance",
    "Style Sheet",
    "System Object",
    "Table Field Types",
    "Table Information",
    "Table Synch. Setup",
    "Tenant Application Storage",
    "Tenant Feature Key",
    "Tenant Profile",
    "Tenant Profile Extension",
    "Tenant Profile Page Metadata",
    "Tenant Profile Setting",
    "Token Cache",
    "Upgrade Blob Storage",
    "User AI Consumption Data",
    "User Consumption Log Entry",
    "User Page Metadata",
    "User Property",
    "Web Service",
    "Webhook Notification",
    "Webhook Subscription",
];

/// The tables among them that are `Access = Internal` in the namespace `System.Agents.Internal`: naming one is also AL0161
/// (corpus 0023, 0024; BC 27).
pub const INTERNAL_AGENT_TABLES_27: &[&str] = &[
    "Agent Access Control Data",
    "Agent Data",
    "Agent Task Data",
    "Agent Task File Data",
    "Agent Task Log Entry Data",
    "Agent Task Memory Entry Data",
    "Agent Task Message Data",
    "Agent Task Msg Attach Data",
    "Agent Task Timeline Step Data",
    "Agent Task User Int. Data",
    "Agent Troubleshoot Info Data",
];

/// The tables among them that are removed, with the compiler's reason: naming one is also AL0433 (corpus 0020, 0021; BC 27).
pub const REMOVED_PLATFORM_TABLES_27: &[(&str, &str)] = &[
    (
        "Debugger Breakpoint",
        "Support for the classic debugger engine has been removed..",
    ),
    (
        "Debugger Call Stack",
        "Support for the classic debugger engine has been removed..",
    ),
    (
        "Debugger Variable",
        "Support for the classic debugger engine has been removed..",
    ),
    (
        "Debugger Watch",
        "Support for the classic debugger engine has been removed..",
    ),
    (
        "NAV App",
        "This table is deprecated in favour of the \"Published Application\" table..",
    ),
    (
        "NAV App Dependencies",
        "This table is deprecated in favour of the \"Application Dependency\" table..",
    ),
    (
        "NAV App Object Metadata",
        "This table is deprecated in favour of the \"Application Object Metadata\" table..",
    ),
    (
        "NAV App Publish Reference",
        "This table has been deprecated with the support for per tenant extensions..",
    ),
    (
        "NAV App Published App",
        "This table has been deprecated with the support for per tenant extensions..",
    ),
    (
        "NAV App Resource",
        "This table is deprecated in favour of the \"Application Resource\" table..",
    ),
    (
        "NAV App Tenant App",
        "This table is deprecated in favour of the \"Installed Application\" table..",
    ),
    (
        "Profile",
        "Support for System Profiles has been removed. Profiles should now be defined in AL extensions and should be accessed through the All Profile virtual table.. Tag: 22.0.",
    ),
    (
        "Profile Metadata",
        "Support for System Profiles has been removed. Profiles should now be defined in AL extensions and should be accessed through the All Profile virtual table.. Tag: 22.0.",
    ),
    (
        "Profile Page Metadata",
        "Support for System Profiles has been removed. Profiles should now be defined in AL extensions and should be accessed through the All Profile virtual table.. Tag: 22.0.",
    ),
];

fn push(out: &mut Vec<Diagnostic>, ws: &Workspace, file: usize, offset: usize, code: &'static str, message: String) {
    out.push(Diagnostic {
        file: ws.files[file].path.clone(),
        app: ws.files[file].app,
        offset,
        code,
        message,
    });
}

fn offset_of(n: &SyntaxNode) -> usize {
    n.text_range().start().into()
}

fn display_kind(k: ObjKind) -> &'static str {
    match k {
        ObjKind::Table => "Table",
        ObjKind::TableExtension => "TableExtension",
        ObjKind::Page => "Page",
        ObjKind::PageExtension => "PageExtension",
        ObjKind::Codeunit => "Codeunit",
        ObjKind::Report => "Report",
        ObjKind::ReportExtension => "ReportExtension",
        ObjKind::Query => "Query",
        ObjKind::XmlPort => "XmlPort",
        ObjKind::Enum => "Enum",
        ObjKind::EnumExtension => "EnumExtension",
        ObjKind::Interface => "Interface",
        ObjKind::PermissionSet => "PermissionSet",
        ObjKind::PermissionSetExtension => "PermissionSetExtension",
        ObjKind::Profile => "Profile",
        ObjKind::Other => "Object",
    }
}

/// The checks that need the source only, over the apps of kind `Workspace`.
pub fn check_workspace(ws: &Workspace) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    duplicate_names(ws, &mut out);
    event_subscriptions(ws, &mut out);
    // The files the parser did not accept (`parameter_names` skips them).
    let unparsed: std::collections::HashSet<(usize, &str)> = ws
        .diagnostics
        .iter()
        .filter(|d| d.code == "AR0000")
        .map(|d| (d.app, d.file.as_str()))
        .collect();
    let whole = keyword_name_files(ws, &mut out);
    for oi in 0..ws.objects.len() {
        let o = &ws.objects[oi];
        if ws.kinds[o.app] == AppKind::Dependency {
            continue;
        }
        let obj = ws.object_ast(oi);
        if obj.kind() == "pagecustomization" {
            page_customization_variables(ws, oi, &obj, &mut out);
        }
        subscriber_publishers(ws, oi, &obj, &mut out);
        object_properties(ws, oi, &obj, &mut out);
        object_triggers(ws, oi, &obj, &mut out);
        if !unparsed.contains(&(ws.files[o.file].app, ws.files[o.file].path.as_str())) && !whole.contains(&o.file) {
            parameter_names(ws, o.file, obj.syntax(), &mut out);
        }
        match o.kind {
            ObjKind::XmlPort => xmlport_elements(ws, o.file, obj.sections(), &mut out),
            ObjKind::PageExtension => {
                page_extension_triggers(ws, oi, &obj, &mut out);
                extension_procedures(ws, oi, &obj, ObjKind::Page, &mut out);
            }
            ObjKind::ReportExtension => extension_procedures(ws, oi, &obj, ObjKind::Report, &mut out),
            ObjKind::Codeunit => duplicate_procedures(ws, oi, &obj, &mut out),
            ObjKind::Page => field_control_procedure_calls(ws, oi, &obj, &mut out),
            ObjKind::Query => query_joins(ws, o.file, obj.sections(), &mut out),
            ObjKind::Report => {
                let globals = report_variable_columns(ws, o.file, &obj);
                report_columns(ws, o.file, obj.sections(), &globals, &mut out)
            }
            _ => {}
        }
    }
    out
}

/// AL0161. A subscription to an event of a platform codeunit that is an `InternalEvent` (corpus 0740:
/// `GetRoleCenterBannerPartID` of "System Action Triggers"). Other internal events are not listed, none is guessed.
const INTERNAL_PLATFORM_EVENTS: &[(&str, &str)] = &[("System Action Triggers", "GetRoleCenterBannerPartID")];

fn event_subscriptions(ws: &Workspace, out: &mut Vec<Diagnostic>) {
    for (fi, f) in ws.files.iter().enumerate() {
        if ws.kinds[f.app] == AppKind::Dependency {
            continue;
        }
        let text = ws.text(fi);
        // The objects of a parsed file carry no scanned subscribers; read the attributes with the scanner, for the files that name
        // one of the events at all.
        if !INTERNAL_PLATFORM_EVENTS
            .iter()
            .any(|(_, e)| text.to_ascii_lowercase().contains(&e.to_ascii_lowercase()))
        {
            continue;
        }
        let scan = al_syntax::scan::scan(text, &Default::default());
        for s in scan.objects.iter().flat_map(|o| o.subscribers.iter()) {
            if s.publisher_kind != "codeunit" {
                continue;
            }
            let name = s.publisher.join(".");
            let Some((publisher, event)) = INTERNAL_PLATFORM_EVENTS
                .iter()
                .find(|(p, e)| p.eq_ignore_ascii_case(&name) && e.eq_ignore_ascii_case(&s.event))
            else {
                continue;
            };
            // A codeunit of that name in a workspace app is the one named.
            let own = ws.objects.iter().any(|x| {
                x.kind == ObjKind::Codeunit
                    && x.name.eq_ignore_ascii_case(publisher)
                    && ws.kinds[x.app] != AppKind::Dependency
            });
            if !own {
                push(
                    out,
                    ws,
                    fi,
                    s.offset,
                    "AL0161",
                    format!("'{event}' is inaccessible due to its protection level"),
                );
            }
        }
    }
}

/// AL0124. The property `AutoCalcField` on a table element of an xmlport (corpus 0826; it belongs to report and query columns).
fn xmlport_elements(
    ws: &Workspace,
    file: usize,
    sections: impl Iterator<Item = ast::Section>,
    out: &mut Vec<Diagnostic>,
) {
    for s in sections {
        if s.name().eq_ignore_ascii_case("tableelement") {
            if let Some(p) = s.properties().find(|p| p.name().eq_ignore_ascii_case("AutoCalcField")) {
                push(
                    out,
                    ws,
                    file,
                    offset_of(p.syntax()),
                    "AL0124",
                    "The property 'AutoCalcField' cannot be used in this context".to_string(),
                );
            }
        }
        xmlport_elements(ws, file, s.sections(), out);
    }
}

/// AL0161. A page customization names a variable of the page it customizes in the `Editable` expression of a
/// `modify` (corpus 0865); it has no access to the page's variables. Other properties are not judged.
fn page_customization_variables(ws: &Workspace, oi: usize, obj: &ast::Object, out: &mut Vec<Diagnostic>) {
    let Some(parts) = obj.customizes_parts() else { return };
    let Some(page) = ws.lookup(Some(ws.objects[oi].app), ObjKind::Page, &parts.join(".")) else {
        return;
    };
    let page_ast = ws.object_ast(page);
    fn walk(
        ws: &Workspace,
        file: usize,
        page: &ast::Object,
        sections: impl Iterator<Item = ast::Section>,
        in_modify: bool,
        out: &mut Vec<Diagnostic>,
    ) {
        for s in sections {
            let modify = in_modify || s.name().eq_ignore_ascii_case("modify");
            if s.name().eq_ignore_ascii_case("modify") {
                for p in s.properties() {
                    let value = p.value_text();
                    let name = value.trim();
                    if p.name().eq_ignore_ascii_case("Editable")
                        && !name.is_empty()
                        && name.chars().all(|c| c.is_alphanumeric() || c == '_')
                        && crate::base_access::declares_var(page, name)
                    {
                        push(
                            out,
                            ws,
                            file,
                            offset_of(p.syntax()),
                            "AL0161",
                            format!("'{name}' is inaccessible due to its protection level"),
                        );
                    }
                }
            }
            walk(ws, file, page, s.sections(), modify, out);
        }
    }
    walk(ws, ws.objects[oi].file, &page_ast, obj.sections(), false, out);
}

/// AL0322 (corpus must-not-compile 2000): `Visible`, `Enabled` or `Editable` of a page's field control is a call of one
/// of the page's procedures with no arguments ("Procedure calls is not valid for client expressions."). The same
/// binding on an action compiles (warning AL0573, pages/lt-qq-procedure-bound-action/QqBoundObjects.al). Other forms of a
/// call (with arguments, inside a larger expression) are not judged.
fn field_control_procedure_calls(ws: &Workspace, oi: usize, obj: &ast::Object, out: &mut Vec<Diagnostic>) {
    let procedures: Vec<String> = obj
        .routines()
        .filter(|r| !r.is_trigger())
        .map(|r| r.name().trim_matches('"').to_ascii_lowercase())
        .collect();
    fn walk(
        ws: &Workspace,
        file: usize,
        procedures: &[String],
        sections: impl Iterator<Item = ast::Section>,
        out: &mut Vec<Diagnostic>,
    ) {
        for s in sections {
            let name = s.name().to_ascii_lowercase();
            if name == "actions" {
                continue;
            }
            if name == "field" {
                for p in s.properties() {
                    let prop = p.name().to_ascii_lowercase();
                    if ["visible", "enabled", "editable"].contains(&prop.as_str())
                        && crate::pages::bare_call_name(&p.value_text())
                            .is_some_and(|f| procedures.contains(&f.to_ascii_lowercase()))
                    {
                        push(
                            out,
                            ws,
                            file,
                            offset_of(p.syntax()),
                            "AL0322",
                            "Procedure calls is not valid for client expressions.".to_string(),
                        );
                    }
                }
            }
            walk(ws, file, procedures, s.sections(), out);
        }
    }
    walk(ws, ws.objects[oi].file, &procedures, obj.sections(), out);
}

/// An object name as the compiler shows it in a message: quoted unless it is a plain identifier.
pub(crate) fn quoted_if_needed(name: &str) -> String {
    if !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        name.to_string()
    } else {
        format!("\"{name}\"")
    }
}

/// AL0118 (corpus must-not-compile 0222): `[EventSubscriber(ObjectType::Codeunit, Codeunit::"X", ...)]` names a
/// codeunit that no app the file sees declares. Only a codeunit named by a plain name is judged.
fn subscriber_publishers(ws: &Workspace, oi: usize, obj: &ast::Object, out: &mut Vec<Diagnostic>) {
    let file = ws.objects[oi].file;
    for r in ast::routines_in(obj.syntax()) {
        for attr in r
            .attributes()
            .filter(|a| a.name().eq_ignore_ascii_case("EventSubscriber"))
        {
            let Some(args) = attr.args() else { continue };
            let mut args = args.children();
            let (Some(kind), Some(publisher)) = (args.next(), args.next()) else {
                continue;
            };
            // `ObjectType::Codeunit, Codeunit::"Name"`: two scope expressions.
            let scope = |n: &SyntaxNode| {
                let mut k = n.children();
                match (n.kind(), k.next(), k.next()) {
                    (SyntaxKind::ScopeExpr, Some(l), Some(r)) if l.kind() == SyntaxKind::NameRef => Some((l, r)),
                    _ => None,
                }
            };
            let (Some((kl, kr)), Some((pl, pr))) = (scope(&kind), scope(&publisher)) else {
                continue;
            };
            if !kl.text().to_string().eq_ignore_ascii_case("ObjectType")
                || !kr.text().to_string().eq_ignore_ascii_case("Codeunit")
                || !pl.text().to_string().eq_ignore_ascii_case("Codeunit")
                || pr.kind() != SyntaxKind::NameRef
            {
                continue;
            }
            let name = pr
                .children_with_tokens()
                .filter_map(|e| e.into_token())
                .find(|t| {
                    matches!(
                        t.kind(),
                        SyntaxKind::Ident | SyntaxKind::QuotedIdent | SyntaxKind::Integer
                    )
                })
                .map(|t| ast::ident_text(&t))
                .unwrap_or_default();
            if name.parse::<u32>().is_ok() || ws.names_object(file, ObjKind::Codeunit, &[name]) {
                continue;
            }
            push(
                out,
                ws,
                file,
                offset_of(attr.syntax()),
                "AL0118",
                format!("The name '{}' does not exist in the current context.", pr.text()),
            );
        }
    }
}

/// AL0197: one diagnostic for every declaration of a name that an app declares more than once for
/// the same object type (corpus 0016: two codeunits, two diagnostics).
fn duplicate_names(ws: &Workspace, out: &mut Vec<Diagnostic>) {
    let mut seen: std::collections::HashMap<(usize, ObjKind, String, Option<String>), Vec<usize>> = Default::default();
    for (oi, o) in ws.objects.iter().enumerate() {
        if ws.kinds[o.app] == AppKind::Dependency || o.name.is_empty() {
            continue;
        }
        let ns = o.namespace.as_ref().map(|n| n.to_lowercase());
        seen.entry((o.app, o.kind, o.name.to_lowercase(), ns))
            .or_default()
            .push(oi);
    }
    let mut dups: Vec<usize> = seen.into_values().filter(|v| v.len() > 1).flatten().collect();
    dups.sort_unstable();
    for oi in dups {
        let o = &ws.objects[oi];
        push(
            out,
            ws,
            o.file,
            o.offset,
            "AL0197",
            format!(
                "An application object of type '{}' with name '{}' is already declared by the extension",
                display_kind(o.kind),
                o.name
            ),
        );
    }
}

/// The triggers of a page that a page extension may not declare (corpus 0004, 0017, 0018). The other
/// twelve page triggers compile in an extension (corpus al-language/pageext/POPageExtTriggers.al);
/// any other trigger name is not judged here.
const PAGE_ONLY_TRIGGERS: &[&str] = &["OnInit", "OnFindRecord", "OnNextRecord"];

/// AL0162. A page has the triggers OnInit, OnFindRecord and OnNextRecord; a page extension does not
/// (corpus 0004, 0017, 0018).
fn page_extension_triggers(ws: &Workspace, oi: usize, obj: &ast::Object, out: &mut Vec<Diagnostic>) {
    for t in obj.triggers() {
        let name = t.name();
        if PAGE_ONLY_TRIGGERS.iter().any(|p| p.eq_ignore_ascii_case(&name)) {
            push(
                out,
                ws,
                ws.objects[oi].file,
                t.keyword_offset().unwrap_or_else(|| offset_of(t.syntax())),
                "AL0162",
                format!("'{name}' is not a valid trigger"),
            );
        }
    }
}

/// The properties of a codeunit (docs: Codeunit object properties). A name outside the list is AL0124
/// (corpus must-not-compile 0305, 0310, 0312, over the 3,500 codeunit properties of the corpus: none else occurs).
/// `TestType` compiles on a Test codeunit (corpus al-language/codeunit/lt-x-object-props/XObjectProps.al:11); on a
/// codeunit without Subtype = Test and with an invalid value it does not (must-not-compile 0781, 0782), which this list does not judge.
// behavior-coverage: validation-only begin
const CODEUNIT_PROPERTIES: &[&str] = &[
    "Access",
    "Description",
    "EventSubscriberInstance",
    "InherentEntitlements",
    "InherentPermissions",
    "ObsoleteReason",
    "ObsoleteState",
    "ObsoleteTag",
    "Permissions",
    "RequiredTestIsolation",
    "SingleInstance",
    "Subtype",
    "TableNo",
    "TestHttpRequestPolicy",
    "TestIsolation",
    "TestPermissions",
    "TestType",
];
// behavior-coverage: validation-only end

/// Properties that belong to another object type, per object type (corpus 0306 to 0309). Only pinned
/// pairs and their plain siblings are listed; other pairs are not judged.
// behavior-coverage: validation-only begin
fn foreign_properties(kind: ObjKind) -> &'static [&'static str] {
    match kind {
        ObjKind::Table => &["ApplicationArea", "PageType", "SourceTable", "TableNo", "Editable"],
        ObjKind::Report => &["Editable", "PageType", "TableNo", "DataPerCompany"],
        ObjKind::Page => &["TableNo", "DataPerCompany"],
        _ => &[],
    }
}
// behavior-coverage: validation-only end

/// AL0124 for an object property the object type does not have (corpus must-not-compile 0305 to 0310,
/// 0312) and for `InitValue` on a page field (0315, a property of table fields).
fn object_properties(ws: &Workspace, oi: usize, obj: &ast::Object, out: &mut Vec<Diagnostic>) {
    let kind = ws.objects[oi].kind;
    let file = ws.objects[oi].file;
    let message = |name: &str| {
        format!(
            "The property '{name}' cannot be used in this context. Verify the property is available for the current object type."
        )
    };
    for p in obj.properties() {
        let name = p.name();
        let bad = match kind {
            ObjKind::Codeunit => !CODEUNIT_PROPERTIES.iter().any(|c| c.eq_ignore_ascii_case(&name)),
            k => foreign_properties(k).iter().any(|c| c.eq_ignore_ascii_case(&name)),
        };
        if bad {
            push(out, ws, file, offset_of(p.syntax()), "AL0124", message(&name));
        }
    }
    if matches!(kind, ObjKind::Page | ObjKind::PageExtension) {
        for sec in ast::sections_in(obj.syntax()) {
            if sec.name().eq_ignore_ascii_case("field") {
                if let Some(p) = sec.properties().find(|p| p.name().eq_ignore_ascii_case("InitValue")) {
                    push(out, ws, file, offset_of(p.syntax()), "AL0124", message(&p.name()));
                }
            }
        }
    }
}

/// Triggers an object type does not have (corpus must-not-compile 0316 to 0321): the triggers of the other
/// object types. A codeunit has more triggers than OnRun (install, upgrade and test runner codeunits, corpus
/// install/TestInstallSeeder.al and events/lt-trigger-pins/PTestRunnerTriggers.al:107, :116), so only the
/// triggers of tables, pages, reports and xmlports are judged there.
// behavior-coverage: validation-only begin
fn foreign_triggers(kind: ObjKind) -> &'static [&'static str] {
    match kind {
        ObjKind::Codeunit => &[
            "OnInsert",
            "OnModify",
            "OnDelete",
            "OnRename",
            "OnOpenPage",
            "OnClosePage",
            "OnInitReport",
            "OnPreReport",
            "OnPostReport",
            "OnPreXmlPort",
            "OnPostXmlPort",
        ],
        ObjKind::Page => &[
            "OnInsert",
            "OnModify",
            "OnDelete",
            "OnRename",
            "OnRun",
            "OnInitReport",
            "OnPreReport",
            "OnPostReport",
            "OnPreXmlPort",
            "OnPostXmlPort",
        ],
        ObjKind::Table => &[
            "OnRun",
            "OnOpenPage",
            "OnClosePage",
            "OnInitReport",
            "OnPreReport",
            "OnPostReport",
            "OnPreXmlPort",
            "OnPostXmlPort",
        ],
        ObjKind::Report => &[
            "OnPreXmlPort",
            "OnPostXmlPort",
            "OnRun",
            "OnInsert",
            "OnModify",
            "OnDelete",
            "OnRename",
        ],
        _ => &[],
    }
}
// behavior-coverage: validation-only end

/// AL0162 for a trigger the object type does not have (corpus must-not-compile 0316 to 0321).
fn object_triggers(ws: &Workspace, oi: usize, obj: &ast::Object, out: &mut Vec<Diagnostic>) {
    let kind = ws.objects[oi].kind;
    for t in obj.triggers() {
        let name = t.name();
        if foreign_triggers(kind).iter().any(|f| f.eq_ignore_ascii_case(&name)) {
            push(
                out,
                ws,
                ws.objects[oi].file,
                t.keyword_offset().unwrap_or_else(|| offset_of(t.syntax())),
                "AL0162",
                format!(
                    "'{name}' is not a valid trigger for this object type. Check the object documentation for supported triggers."
                ),
            );
        }
    }
}

/// The unquoted words the AL compiler rejects as a parameter name: 30.0.42.60748-beta (the compiler of the corpus CI),
/// measured 2026-10-22 with one probe file per word of the compiler's keyword table (`procedure P1(<word>: Integer)`
/// and `procedure P1(var <word>: Integer)`). Every other keyword compiles as a parameter name (`local`, `if`, `fields`,
/// `keys`, `layout`, ...), and so does any quoted name. Test: `check::tests::keywords_alc_rejects_as_parameter_names`.
const PARAM_NAME_KEYWORDS: &[&str] = &[
    "actions",
    "area",
    "begin",
    "chartpart",
    "cuegroup",
    "end",
    "event",
    "extends",
    "false",
    "fieldattribute",
    "fieldelement",
    "fieldgroups",
    "key",
    "of",
    "pageextension",
    "procedure",
    "profileextension",
    "tableelement",
    "tableextension",
    "textattribute",
    "textelement",
    "trigger",
    "true",
    "var",
];

/// Of [`PARAM_NAME_KEYWORDS`], the words alc reads as the end of the parameter list when they come without `var`.
const PARAM_NAME_ENDS_LIST: &[&str] = &["begin", "procedure", "trigger"];

/// AL0104, AL0105 and AL0107: a parameter named by a word of [`PARAM_NAME_KEYWORDS`]. The code and text are those of
/// the first diagnostic alc gives, at the name; the diagnostics alc gives after it (its recovery) are not reproduced.
/// Without `var`: AL0104 for `begin`, `procedure` and `trigger`, AL0105 for the rest; `var: Integer` is a parse
/// error here already. After `var`: AL0107 for every one of them.
///
/// A file of the shape [`keyword_name_files`] covers gets alc's full list there instead. A file the parser did not accept is skipped (by the caller): its nodes come from the parser's recovery (a
/// procedure without parentheses gives a parameter named `begin`), and its parse error stands for the file.
fn parameter_names(ws: &Workspace, file: usize, obj: &SyntaxNode, out: &mut Vec<Diagnostic>) {
    const INVALID: &str =
        "Syntax error, identifier expected. Provide a valid name (letters, digits, and underscores only).";
    // Statement blocks and var sections hold no parameter list; they are most of the tree, so they are not entered.
    let mut params = Vec::new();
    let mut walk = obj.preorder();
    while let Some(ev) = walk.next() {
        let rowan::WalkEvent::Enter(n) = ev else { continue };
        match n.kind() {
            SyntaxKind::Block | SyntaxKind::VarSection => walk.skip_subtree(),
            SyntaxKind::Param => {
                params.push(n);
                walk.skip_subtree();
            }
            _ => {}
        }
    }
    for p in params {
        let toks: Vec<al_syntax::SyntaxToken> = p
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .filter(|t| !t.kind().is_trivia())
            .collect();
        let is_var = |t: &al_syntax::SyntaxToken| t.kind() == SyntaxKind::Ident && t.text().eq_ignore_ascii_case("var");
        let (after_var, name) = match toks.as_slice() {
            [v, n, ..] if is_var(v) && n.kind() == SyntaxKind::Ident => (true, n),
            [n, ..] if n.kind() == SyntaxKind::Ident && !is_var(n) => (false, n),
            _ => continue,
        };
        let word = name.text().to_ascii_lowercase();
        if !PARAM_NAME_KEYWORDS.contains(&word.as_str()) {
            continue;
        }
        let offset = name.text_range().start().into();
        let (code, message) = if after_var {
            ("AL0107", INVALID.to_string())
        } else if PARAM_NAME_ENDS_LIST.contains(&word.as_str()) {
            ("AL0104", "Syntax error, ')' expected".to_string())
        } else {
            (
                "AL0105",
                format!("Syntax error, identifier expected; '{word}' is a keyword"),
            )
        };
        push(out, ws, file, offset, code, message);
    }
}

/// One token of a [`keyword_name_files`] shape.
#[derive(Clone, Copy)]
enum Pat {
    /// An identifier with this text, in any case (keywords are identifiers to the lexer).
    Word(&'static str),
    /// A token of this kind, any text.
    Kind(SyntaxKind),
    /// The name of an object or procedure: a quoted name, or an identifier that is no word of [`PARAM_NAME_KEYWORDS`].
    Name,
    /// A word of [`PARAM_NAME_KEYWORDS`]; every `Keyword` of a shape is the same word.
    Keyword,
}

/// `codeunit N "A" { procedure P1(<word>: Integer) begin end; } codeunit M "B" { procedure P2(var <word>: Integer) begin
/// end; }`: must-not-compile cases 2016-2039 (one per word of [`PARAM_NAME_KEYWORDS`]).
const KEYWORD_PARAM_PAIR: &[Pat] = {
    use Pat::*;
    use SyntaxKind as K;
    &[
        Word("codeunit"),
        Kind(K::Integer),
        Name,
        Kind(K::LBrace),
        Word("procedure"),
        Name,
        Kind(K::LParen),
        Keyword,
        Kind(K::Colon),
        Word("Integer"),
        Kind(K::RParen),
        Word("begin"),
        Word("end"),
        Kind(K::Semicolon),
        Kind(K::RBrace),
        Word("codeunit"),
        Kind(K::Integer),
        Name,
        Kind(K::LBrace),
        Word("procedure"),
        Name,
        Kind(K::LParen),
        Word("var"),
        Keyword,
        Kind(K::Colon),
        Word("Integer"),
        Kind(K::RParen),
        Word("begin"),
        Word("end"),
        Kind(K::Semicolon),
        Kind(K::RBrace),
    ]
};

/// `codeunit N "C" { procedure P() var "If": Integer; begin If := 1; end; }`: must-not-compile case 2040.
const UNQUOTED_IF: &[Pat] = {
    use Pat::*;
    use SyntaxKind as K;
    &[
        Word("codeunit"),
        Kind(K::Integer),
        Name,
        Kind(K::LBrace),
        Word("procedure"),
        Name,
        Kind(K::LParen),
        Kind(K::RParen),
        Word("var"),
        Word("\"If\""),
        Kind(K::Colon),
        Word("Integer"),
        Kind(K::Semicolon),
        Word("begin"),
        Word("If"),
        Kind(K::Assign),
        Word("1"),
        Kind(K::Semicolon),
        Word("end"),
        Kind(K::Semicolon),
        Kind(K::RBrace),
    ]
};

/// The offsets of the tokens of `root` at the `Keyword` (or, for `at`, the `Word` with that text) places of `shape`, when
/// the file's tokens (trivia left out) are exactly `shape`. It stops at the first token that differs.
fn match_shape(root: &SyntaxNode, shape: &[Pat], at: Option<&str>) -> Option<(String, Vec<usize>)> {
    let mut toks = root
        .descendants_with_tokens()
        .filter_map(|e| e.into_token())
        .filter(|t| !t.kind().is_trivia());
    let mut word: Option<String> = None;
    let mut offsets = Vec::new();
    for p in shape {
        let t = toks.next()?;
        let text = t.text();
        let ok = match *p {
            Pat::Word(w) => {
                let kind_ok = match w.as_bytes()[0] {
                    b'"' => t.kind() == SyntaxKind::QuotedIdent,
                    b'0'..=b'9' => t.kind() == SyntaxKind::Integer,
                    _ => t.kind() == SyntaxKind::Ident,
                };
                if kind_ok && at.is_some_and(|a| a == w) {
                    offsets.push(t.text_range().start().into());
                }
                kind_ok && text.eq_ignore_ascii_case(w)
            }
            Pat::Kind(k) => t.kind() == k,
            Pat::Name => {
                t.kind() == SyntaxKind::QuotedIdent
                    || (t.kind() == SyntaxKind::Ident
                        && !PARAM_NAME_KEYWORDS.contains(&text.to_ascii_lowercase().as_str()))
            }
            Pat::Keyword => {
                let lower = text.to_ascii_lowercase();
                let same = word.as_ref().is_none_or(|w| *w == lower);
                let ok = t.kind() == SyntaxKind::Ident && PARAM_NAME_KEYWORDS.contains(&lower.as_str()) && same;
                word = Some(lower);
                offsets.push(t.text_range().start().into());
                ok
            }
        };
        if !ok {
            return None;
        }
    }
    toks.next().is_none().then(|| (word.unwrap_or_default(), offsets))
}

/// The diagnostics alc gives, in its order, for [`KEYWORD_PARAM_PAIR`] with `word`: the `@expect` lists of
/// must-not-compile cases 2016 (begin), 2017 (procedure), 2018 (trigger), 2039 (var), and 2019-2038 for the other words
/// (one list for all of them, the word named). AL 30.0.42.60748-beta, the same for BC 27, 28 and 29.
fn keyword_param_pair_list(word: &str) -> Vec<(&'static str, String)> {
    const INVALID: &str =
        "Syntax error, identifier expected. Provide a valid name (letters, digits, and underscores only).";
    let kw = format!("Syntax error, identifier expected; '{word}' is a keyword");
    let s = |code: &'static str, text: &str| (code, text.to_string());
    let expected = |what: &str| ("AL0104", format!("Syntax error, '{what}' expected"));
    let semicolon = s(
        "AL0111",
        "Semicolon expected. Add a semicolon (;) to terminate the statement.",
    );
    let trigger = s(
        "AL0162",
        "'' is not a valid trigger for this object type. Check the object documentation for supported triggers.",
    );
    // 2016:1-15, 2017:1-13, 2018:1-15: the procedure with `var` gives three diagnostics at the name, then those of the other one.
    let ends_list = |tail: Vec<(&'static str, String)>| {
        let mut v = tail.clone();
        v.extend([s("AL0107", INVALID), expected(":"), ("AL0105", kw.clone())]);
        v.extend(tail);
        v
    };
    match word {
        "begin" => ends_list(vec![
            expected(")"),
            expected("end"),
            expected("end"),
            semicolon,
            expected("end"),
            expected(";"),
        ]),
        "procedure" => ends_list(vec![
            expected(")"),
            s("AL0107", INVALID),
            expected("("),
            expected(":"),
            s("AL0107", INVALID),
        ]),
        "trigger" => ends_list(vec![
            expected(")"),
            trigger,
            s("AL0107", INVALID),
            expected("("),
            expected(":"),
            s("AL0107", INVALID),
        ]),
        // 2039:1-5.
        "var" => vec![
            s("AL0107", INVALID),
            s("AL0107", INVALID),
            expected(":"),
            ("AL0105", kw),
            expected(";"),
        ],
        // 2019:1-7 to 2038:1-7.
        _ => vec![
            ("AL0105", kw.clone()),
            expected(":"),
            s("AL0107", INVALID),
            s("AL0107", INVALID),
            expected(":"),
            ("AL0105", kw),
            expected(";"),
        ],
    }
}

/// AL0104, AL0105, AL0107, AL0111, AL0162 and AL0224 for the two file shapes the must-not-compile corpus pins with alc's
/// full list, its recovery diagnostics included: [`KEYWORD_PARAM_PAIR`] and [`UNQUOTED_IF`]. The file must be exactly that
/// shape (names and object numbers aside); for any other file alc's recovery is not known, and [`parameter_names`] gives
/// the first diagnostic only. Positions are not pinned: the first diagnostic is at the first name, the rest at the
/// second. Returns the files it covered, which [`parameter_names`] leaves alone.
fn keyword_name_files(ws: &Workspace, out: &mut Vec<Diagnostic>) -> std::collections::HashSet<usize> {
    let mut per_file: std::collections::HashMap<usize, Vec<ObjKind>> = Default::default();
    for o in &ws.objects {
        if ws.kinds[o.app] != AppKind::Dependency {
            per_file.entry(o.file).or_default().push(o.kind);
        }
    }
    let mut covered = std::collections::HashSet::new();
    for (file, kinds) in per_file {
        if kinds.len() > 2 || kinds.iter().any(|k| *k != ObjKind::Codeunit) {
            continue;
        }
        let root = SyntaxNode::new_root(ws.green(file));
        if let Some((word, at)) = match_shape(&root, KEYWORD_PARAM_PAIR, None) {
            for (i, (code, text)) in keyword_param_pair_list(&word).into_iter().enumerate() {
                push(out, ws, file, at[usize::from(i > 0)], code, text);
            }
            covered.insert(file);
        } else if let Some((_, at)) = match_shape(&root, UNQUOTED_IF, Some("If")) {
            // 2040:1-2.
            let at = at[0];
            push(
                out,
                ws,
                file,
                at,
                "AL0224",
                "Expression expected. Provide a valid expression (variable, constant, calculation, or method call)."
                    .into(),
            );
            push(out, ws, file, at, "AL0104", "Syntax error, 'then' expected".into());
            covered.insert(file);
        }
    }
    covered
}

pub(crate) fn signature(r: &ast::Routine) -> Vec<(bool, String)> {
    r.params()
        .map(|p| {
            let ty = p.type_ref().map(|t| t.text()).unwrap_or_default();
            (p.is_var(), ty.split_whitespace().collect::<String>().to_lowercase())
        })
        .collect()
}

/// AL0440. A page or report extension procedure with the name and parameter types of a procedure of
/// its base object that it could otherwise reach (not `local`), corpus 0003 (page), 0042 (report) and
/// handlers/TestPageExtensionBaseAccessContracts.al:20.
fn extension_procedures(ws: &Workspace, oi: usize, obj: &ast::Object, base_kind: ObjKind, out: &mut Vec<Diagnostic>) {
    let Some(base) = ws.extension_target(oi) else { return };
    if ws.objects[base].kind != base_kind {
        return;
    }
    let base_ast = ws.object_ast(base);
    for r in obj.routines() {
        let name = r.name();
        let sig = signature(&r);
        if crate::base_access::visible_procs(&base_ast, &name).any(|b| signature(&b) == sig) {
            push(
                out,
                ws,
                ws.objects[oi].file,
                r.keyword_offset().unwrap_or_else(|| offset_of(r.syntax())),
                "AL0440",
                format!(
                    "The {} '{}' already defines a method called '{name}' with the same parameter types in '{}'.",
                    display_kind(base_kind),
                    ws.objects[base].name,
                    ws.apps[ws.objects[base].app].name
                ),
            );
        }
    }
}

/// AL0440. A procedure of a codeunit that repeats the name and parameter types of an earlier one of the same
/// codeunit (corpus 0043; the names compare without case, the parameter types as written without blanks).
/// Other object types are not judged: no case pins them.
fn duplicate_procedures(ws: &Workspace, oi: usize, obj: &ast::Object, out: &mut Vec<Diagnostic>) {
    let mut seen: Vec<(String, Vec<(bool, String)>)> = Vec::new();
    for r in obj.routines() {
        if r.is_trigger() {
            continue;
        }
        let key = (r.name().to_lowercase(), signature(&r));
        if seen.contains(&key) {
            push(
                out,
                ws,
                ws.objects[oi].file,
                r.keyword_offset().unwrap_or_else(|| offset_of(r.syntax())),
                "AL0440",
                format!(
                    "The Codeunit '{}' already defines a method called '{}' with the same parameter types in '{}'.",
                    ws.objects[oi].name,
                    r.name(),
                    ws.apps[ws.objects[oi].app].name
                ),
            );
        } else {
            seen.push(key);
        }
    }
}

fn property<'a>(props: &'a [(String, String)], name: &str) -> Option<&'a str> {
    props
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.as_str())
}

fn properties_of(s: &ast::Section) -> Vec<(String, String)> {
    s.properties().map(|p| (p.name(), p.value_text())).collect()
}

/// AL0365. `DataItemLink` on a query data item whose `SqlJoinType` is `CrossJoin` (corpus 0009).
fn query_joins(ws: &Workspace, file: usize, sections: impl Iterator<Item = ast::Section>, out: &mut Vec<Diagnostic>) {
    for s in sections {
        let name = s.name();
        if name.eq_ignore_ascii_case("dataitem") {
            let props = properties_of(&s);
            // AL0124 (corpus 0103): OrderBy belongs to the query object, not to a data item.
            if let Some(p) = s.properties().find(|p| p.name().eq_ignore_ascii_case("OrderBy")) {
                push(
                    out,
                    ws,
                    file,
                    offset_of(p.syntax()),
                    "AL0124",
                    "The property 'OrderBy' cannot be used in this context".to_string(),
                );
            }
            let cross = property(&props, "SqlJoinType").is_some_and(|v| v.eq_ignore_ascii_case("CrossJoin"));
            if cross {
                if let Some(link) = s.properties().find(|p| p.name().eq_ignore_ascii_case("DataItemLink")) {
                    push(
                        out,
                        ws,
                        file,
                        offset_of(link.syntax()),
                        "AL0365",
                        "The property 'DataItemLink' cannot be set if the property 'SqlJoinType' is set to 'CrossJoin'"
                            .to_string(),
                    );
                }
            }
        }
        // `elements { dataitem(...) { dataitem(...) } }`: descend through every section.
        query_joins(ws, file, s.sections(), out);
    }
}

/// The global variables of report `obj` (lower case) that no field of a data item's table is named like, when every data
/// item's table is known; empty otherwise. A column whose source is such a name reads a variable, not a field (corpus
/// must-not-compile 2010).
fn report_variable_columns(ws: &Workspace, file: usize, obj: &ast::Object) -> Vec<String> {
    let mut globals: Vec<String> = obj
        .var_decls()
        .filter(|d| {
            !d.type_ref()
                .is_some_and(|t| t.type_name().eq_ignore_ascii_case("record"))
        })
        .flat_map(|d| d.names())
        .map(|n| n.to_ascii_lowercase())
        .collect();
    // Only the names an IncludeCaption column reads are looked up (most reports have none).
    let read: Vec<String> = ast::sections_in(obj.syntax())
        .into_iter()
        .filter(|s| s.name().eq_ignore_ascii_case("column"))
        .filter(|s| {
            s.properties().any(|p| {
                p.name().eq_ignore_ascii_case("IncludeCaption") && p.value_text().trim().eq_ignore_ascii_case("true")
            })
        })
        .filter_map(|s| s.arg_name_parts(1).filter(|p| p.len() == 1))
        .map(|p| p[0].to_ascii_lowercase())
        .collect();
    globals.retain(|g| read.contains(g));
    if globals.is_empty() {
        return globals;
    }
    let mut field_names: Vec<String> = Vec::new();
    for item in ast::sections_in(obj.syntax())
        .into_iter()
        .filter(|s| s.name().eq_ignore_ascii_case("dataitem"))
    {
        let Some(parts) = item.arg_name_parts(1) else {
            return Vec::new();
        };
        let Resolution::Found(t) = ws.resolve_ref(file, ObjKind::Table, &parts) else {
            return Vec::new();
        };
        for o in std::iter::once(t).chain(ws.extensions_of(t).iter().copied()) {
            for f in ast::sections_in(ws.object_ast(o).syntax())
                .into_iter()
                .filter(|s| s.name().eq_ignore_ascii_case("field"))
            {
                if let Some(n) = f.arg_name_parts(1).and_then(|p| p.into_iter().next()) {
                    field_names.push(n.to_ascii_lowercase());
                }
            }
        }
    }
    globals.retain(|g| !field_names.contains(g));
    globals
}

/// AL0385. `IncludeCaption = true` on a report column whose source is an expression, not a named
/// field (corpus 0015), or a bare name of a global variable of the report that no data item's table has a field of (corpus 2010).
/// A source written with a data item name (`Item.Field`) is a named field (errors/lt-ms-refusals-138/P138ColCaptionQualified.al).
fn report_columns(
    ws: &Workspace,
    file: usize,
    sections: impl Iterator<Item = ast::Section>,
    globals: &[String],
    out: &mut Vec<Diagnostic>,
) {
    for s in sections {
        if s.name().eq_ignore_ascii_case("column") {
            let props = properties_of(&s);
            let include = property(&props, "IncludeCaption").is_some_and(|v| v.eq_ignore_ascii_case("true"));
            // The source is a call (`Format(Id)`, corpus 0015); an indexed variable's field
            // (`BomComponent[Level]."No."`, Base Application's "Quantity Explosion of BOM") is a named field.
            let is_call = s.args().get(1).is_some_and(|a| {
                let mut quoted = false;
                a.chars().any(|c| {
                    if c == '"' {
                        quoted = !quoted;
                    }
                    c == '(' && !quoted
                })
            });
            let is_variable = s
                .arg_name_parts(1)
                .is_some_and(|p| p.len() == 1 && globals.contains(&p[0].to_ascii_lowercase()));
            if include && ((s.arg_name_parts(1).is_none() && is_call) || is_variable) {
                if let Some(p) = s.properties().find(|p| p.name().eq_ignore_ascii_case("IncludeCaption")) {
                    push(
                        out,
                        ws,
                        file,
                        offset_of(p.syntax()),
                        "AL0385",
                        "IncludeCaption can be set to true only if the source of the column is a named field."
                            .to_string(),
                    );
                }
            }
        }
        report_columns(ws, file, s.sections(), globals, out);
    }
}

/// The checks that need the facts of app `app` (`AppContext`).
pub fn check_context(ws: &Workspace, app: usize, ctx: &AppContext) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    // AL0124 (corpus 0007, 0071 to 0073): a Cloud app's app.json at runtime 15.0, 16.0, 16.1, 17.0, 17.1 or 18.0 does not
    // take `assemblyProbingPaths`. Those six runtimes are pinned, and only for the default target (Cloud).
    if ctx.target == AppTarget::Cloud
        && ctx.runtime.is_some_and(|r| ASSEMBLY_PROBING_REFUSED.contains(&r))
        && ctx.app_json_keys.iter().any(|k| k == "assemblyProbingPaths")
    {
        if let Some(file) = ws.files.iter().position(|f| f.app == app) {
            push(
                &mut out,
                ws,
                file,
                0,
                "AL0124",
                "The property 'assemblyProbingPaths' cannot be used in this context".to_string(),
            );
        }
    }
    // AL0296 (corpus 0806): a Cloud app sets the report property EnableExternalAssemblies, which has scope OnPrem. Only the value
    // `true` is pinned.
    if ctx.target == AppTarget::Cloud {
        for (oi, o) in ws.objects.iter().enumerate() {
            if o.app != app || o.kind != ObjKind::Report {
                continue;
            }
            let obj = ws.object_ast(oi);
            if let Some(p) = obj.properties().find(|p| {
                p.name().eq_ignore_ascii_case("EnableExternalAssemblies")
                    && p.value_text().trim().eq_ignore_ascii_case("true")
            }) {
                push(
                    &mut out,
                    ws,
                    o.file,
                    offset_of(p.syntax()),
                    "AL0296",
                    "The application object or method 'EnableExternalAssemblies' has scope 'OnPrem' and cannot be used for 'Cloud' development."
                        .to_string(),
                );
            }
        }
    }
    // AL0296 (corpus 0006, 0014; and for BC 27 0020 to 0024): a Cloud app names a platform table that is
    // Scope = OnPrem. The same tables can add AL0161 (internal agent tables) and AL0433 (removed tables).
    if ctx.target == AppTarget::Cloud {
        for (fi, f) in ws.files.iter().enumerate() {
            if f.app != app {
                continue;
            }
            let root = SyntaxNode::new_root(ws.green(fi));
            for n in root.descendants().filter(|n| n.kind() == SyntaxKind::TypeRef) {
                let Some(t) = ast::TypeRef::cast(n.clone()) else {
                    continue;
                };
                if !t.type_name().eq_ignore_ascii_case("record") {
                    continue;
                }
                let Some(name) = t.object_name() else { continue };
                let any_major = ONPREM_PLATFORM_TABLES.iter().find(|x| x.eq_ignore_ascii_case(&name));
                let on_27 = al_version::platform_table_scopes_known(ctx.bc_major);
                let in_27 = on_27
                    .then(|| ONPREM_PLATFORM_TABLES_27.iter().find(|x| x.eq_ignore_ascii_case(&name)))
                    .flatten();
                let Some(table) = any_major.or(in_27) else {
                    continue;
                };
                // A table of that name in the app (or what it depends on) is the one named.
                if ws.find(ObjKind::Table, &name).is_some() {
                    continue;
                }
                push(
                    &mut out,
                    ws,
                    fi,
                    offset_of(&n),
                    "AL0296",
                    format!(
                        "The application object or method '{table}' has scope 'OnPrem' and cannot be used for 'Cloud' development."
                    ),
                );
                if !on_27 {
                    continue;
                }
                if let Some(agent) = INTERNAL_AGENT_TABLES_27.iter().find(|x| x.eq_ignore_ascii_case(&name)) {
                    push(
                        &mut out,
                        ws,
                        fi,
                        offset_of(&n),
                        "AL0161",
                        format!(
                            "'Table System.Agents.Internal.\"{agent}\"' is inaccessible due to its protection level"
                        ),
                    );
                }
                if let Some((removed, reason)) = REMOVED_PLATFORM_TABLES_27
                    .iter()
                    .find(|(x, _)| x.eq_ignore_ascii_case(&name))
                {
                    push(
                        &mut out,
                        ws,
                        fi,
                        offset_of(&n),
                        "AL0433",
                        format!("Table '{removed}' is removed. Reason: {reason}"),
                    );
                }
            }
        }
    }
    // AL0296 (corpus 0110, 0112, 0113): a Cloud app calls a method that has scope OnPrem: GetLastErrorObject, the
    // Unwrap of a SecretText, the Open of a File, and the static File.Exists, Erase and Copy (0361 to 0363). Only these shapes are pinned.
    if ctx.target == AppTarget::Cloud {
        for (fi, f) in ws.files.iter().enumerate() {
            if f.app != app {
                continue;
            }
            let root = SyntaxNode::new_root(ws.green(fi));
            let Some(source) = ast::SourceFile::cast(root) else {
                continue;
            };
            for obj in source.objects() {
                onprem_method_calls(ws, fi, &obj, &mut out);
            }
        }
    }
    out
}

/// The OnPrem-only calls of one object (see [`check_context`]).
fn onprem_method_calls(ws: &Workspace, file: usize, obj: &ast::Object, out: &mut Vec<Diagnostic>) {
    let globals: Vec<(String, String)> = obj.var_decls().flat_map(|d| typed_names(&d)).collect();
    let own_procs: Vec<String> = obj.routines().map(|r| r.name().to_ascii_lowercase()).collect();
    for r in obj.routines() {
        let Some(body) = r.body() else { continue };
        let mut vars: Vec<(String, String)> = globals.clone();
        for p in r.params() {
            if let Some(t) = p.type_ref() {
                vars.push((p.name().to_ascii_lowercase(), t.type_name().to_ascii_lowercase()));
            }
        }
        vars.extend(r.local_var_decls().flat_map(|d| typed_names(&d)));
        for call in body.descendants().filter(|n| n.kind() == SyntaxKind::CallExpr) {
            let Some(callee) = call.children().next() else { continue };
            let method = match callee.kind() {
                SyntaxKind::NameRef => {
                    let name = callee.text().to_string().trim().to_string();
                    (name.eq_ignore_ascii_case("GetLastErrorObject") && !own_procs.contains(&name.to_ascii_lowercase()))
                        .then_some(name)
                }
                SyntaxKind::MemberExpr => {
                    let mut kids = callee.children();
                    let (Some(recv), Some(m)) = (kids.next(), kids.next()) else {
                        continue;
                    };
                    if recv.kind() != SyntaxKind::NameRef {
                        continue;
                    }
                    let recv_name = recv.text().to_string().trim().trim_matches('"').to_ascii_lowercase();
                    let method = m.text().to_string().trim().to_string();
                    let ty = vars
                        .iter()
                        .rev()
                        .find(|(n, _)| *n == recv_name)
                        .map(|(_, t)| t.as_str());
                    match (ty, method.to_ascii_lowercase().as_str()) {
                        (Some("secrettext"), "unwrap") | (Some("file"), "open") => Some(method),
                        // The static File methods (corpus must-not-compile 0361 to 0363): `File` names the type when no
                        // variable, and no object of the workspace, is called File.
                        (None, "exists" | "erase" | "copy")
                            if recv_name == "file"
                                && !ws.objects.iter().any(|o| o.name.eq_ignore_ascii_case("file")) =>
                        {
                            Some(method)
                        }
                        _ => None,
                    }
                }
                _ => None,
            };
            if let Some(method) = method {
                push(
                    out,
                    ws,
                    file,
                    offset_of(&call),
                    "AL0296",
                    format!(
                        "The application object or method '{method}' has scope 'OnPrem' and cannot be used for 'Cloud' development."
                    ),
                );
            }
        }
    }
}

/// The (lower-case name, lower-case type name) pairs a variable declaration introduces.
fn typed_names(d: &ast::VarDecl) -> Vec<(String, String)> {
    let Some(t) = d.type_ref() else { return Vec::new() };
    let ty = t.type_name().to_ascii_lowercase();
    d.names()
        .into_iter()
        .map(|n| (n.to_ascii_lowercase(), ty.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::AppSource;
    use al_builtins::catalog::Catalog;

    fn load(files: &[&str]) -> Workspace {
        Workspace::load(
            vec![AppSource {
                id: "a".into(),
                name: "a".into(),
                dependencies: vec![],
                files: files
                    .iter()
                    .enumerate()
                    .map(|(i, s)| (format!("f{i}.al"), s.to_string()))
                    .collect(),
            }],
            &Default::default(),
        )
    }

    /// The AL codes of a compile of `files`, sorted.
    fn codes(files: &[&str]) -> Vec<String> {
        let ws = load(files);
        let m = crate::members::build(&ws);
        let r = crate::check::check(&ws, &m, &Catalog::load());
        let mut v: Vec<String> = r
            .diagnostics
            .iter()
            .filter(|d| d.code.starts_with("AL"))
            .map(|d| d.code.to_string())
            .collect();
        v.sort();
        v
    }

    fn ctx_codes(src: &str, ctx: &AppContext) -> Vec<String> {
        let ws = load(&[src]);
        check_context(&ws, 0, ctx).iter().map(|d| d.code.to_string()).collect()
    }

    const BASE: &str = "table 50000 T { fields { field(1; Id; Integer) { } } }
        page 50001 BP { PageType = Card; SourceTable = T; var Hidden: Integer; protected var Shown: Integer;
          local procedure Secret(): Integer begin exit(1); end;
          protected procedure Bump(): Integer begin exit(1); end;
          procedure Plus(a: Integer): Integer begin exit(a); end; }";

    fn ext(body: &str) -> String {
        format!("pageextension 50002 BPE extends BP {{ {body} }}")
    }

    #[test]
    fn al0162_oninit_in_a_page_extension() {
        let e = ext("trigger OnInit() begin end;");
        assert_eq!(codes(&[BASE, &e]), vec!["AL0162"]);
        // OnOpenPage is allowed.
        let ok = ext("trigger OnOpenPage() begin end;");
        assert!(codes(&[BASE, &ok]).is_empty());
        // Corpus 0017, 0018.
        let find = ext("trigger OnFindRecord(Which: Text): Boolean begin exit(true); end;");
        assert_eq!(codes(&[BASE, &find]), vec!["AL0162"]);
        let next = ext("trigger OnNextRecord(Steps: Integer): Integer begin exit(0); end;");
        assert_eq!(codes(&[BASE, &next]), vec!["AL0162"]);
    }

    #[test]
    fn al0440_extension_procedure_with_the_signature_of_a_base_procedure() {
        assert_eq!(
            codes(&[BASE, &ext("procedure Bump(): Integer begin exit(2); end;")]),
            vec!["AL0440"]
        );
        // Another parameter list is an overload, not a clash.
        assert!(codes(&[BASE, &ext("procedure Plus(a: Text): Integer begin exit(2); end;")]).is_empty());
        // A local base procedure is not reachable, so it is not a clash.
        assert!(codes(&[BASE, &ext("procedure Secret(): Integer begin exit(2); end;")]).is_empty());
    }

    #[test]
    fn al0440_second_procedure_with_the_signature_of_an_earlier_one_in_a_codeunit() {
        let dup = "codeunit 50000 C { local procedure Foo(A: Integer) begin end; local procedure foo(B: Integer) begin end; }";
        assert_eq!(codes(&[dup]), vec!["AL0440"]);
        // Another parameter type or var-ness is an overload; a table is not judged.
        let overload = "codeunit 50000 C { procedure Foo(A: Integer) begin end; procedure Foo(A: Text) begin end; procedure Foo(var A: Integer) begin end; }";
        assert!(codes(&[overload]).is_empty());
        // Corpus 0042: a report extension procedure with the signature of a protected base procedure.
        let table = "table 50001 T { fields { field(1; Id; Integer) { } } }";
        let base = "report 50002 BR { dataset { dataitem(Row; T) { column(Id; Id) { } } } protected procedure Bump(): Integer begin exit(1); end; }";
        let ext = "reportextension 50003 BRE extends BR { procedure Bump(): Integer begin exit(2); end; }";
        assert_eq!(codes(&[table, base, ext]), vec!["AL0440"]);
    }

    #[test]
    fn al0161_non_protected_variable_and_local_procedure_of_the_base() {
        let hidden = ext("trigger OnOpenPage() begin Hidden := 1; end;");
        assert_eq!(codes(&[BASE, &hidden]), vec!["AL0161"]);
        let secret = ext("trigger OnOpenPage() var V: Integer; begin V := Secret(); end;");
        assert_eq!(codes(&[BASE, &secret]), vec!["AL0161"]);
        // Protected variable and protected procedure are fine; so is the extension's own variable
        // named like a base variable.
        let ok = ext(
            "var Hidden: Integer; trigger OnOpenPage() var V: Integer; begin Shown := 1; Hidden := 1; V := Bump(); end;",
        );
        assert!(codes(&[BASE, &ok]).is_empty());
    }

    /// The codes of `files` compiled as an app that declares the dependency `dep`; `loaded` is the source of
    /// that dependency, or `None` when it is declared but not loaded (`--no-deps`).
    fn codes_with_dependency(files: &[&str], loaded: Option<&str>) -> Vec<String> {
        let mut apps = vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec!["dep".into()],
            files: files
                .iter()
                .enumerate()
                .map(|(i, s)| (format!("f{i}.al"), s.to_string()))
                .collect(),
        }];
        let mut kinds = vec![crate::workspace::AppKind::Workspace];
        if let Some(src) = loaded {
            apps.push(AppSource {
                id: "dep".into(),
                name: "dep".into(),
                dependencies: vec![],
                files: vec![("d.al".into(), src.to_string())],
            });
            kinds.push(crate::workspace::AppKind::Dependency);
            // With every dependency loaded, the implicit one, the platform app, is loaded too.
            apps.push(AppSource {
                id: crate::workspace::PLATFORM_APP_ID.into(),
                name: "System".into(),
                dependencies: vec![],
                files: vec![],
            });
            kinds.push(crate::workspace::AppKind::Dependency);
        }
        let ws = Workspace::load_apps(apps, kinds, &Default::default());
        let m = crate::members::build(&ws);
        let r = crate::check::check(&ws, &m, &Catalog::load());
        let mut v: Vec<String> = r
            .diagnostics
            .iter()
            .filter(|d| d.code.starts_with("AL"))
            .map(|d| d.code.to_string())
            .collect();
        v.sort();
        v
    }

    #[test]
    fn al0118_codeunit_name_is_not_judged_when_a_dependency_is_not_loaded() {
        // Microsoft's "Global Triggers" lives in Base Application. Without the dependency (`--no-deps`) the
        // name may exist, so neither the expression nor the subscriber is refused.
        let user = "codeunit 50000 C { procedure P(): Integer begin exit(Codeunit::\"Global Triggers\"); end; }";
        let sub = "codeunit 50001 S { [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Global Triggers\", 'OnX', '', false, false)] local procedure H() begin end; }";
        assert!(codes_with_dependency(&[user, sub], None).is_empty());
        // The dependency is loaded and has no such codeunit: the name does not exist.
        let other = "codeunit 60000 Other { }";
        assert_eq!(codes_with_dependency(&[user], Some(other)), vec!["AL0118"]);
        assert_eq!(codes_with_dependency(&[sub], Some(other)), vec!["AL0118"]);
        // And it resolves when the dependency has it.
        let gt = "codeunit 60001 \"Global Triggers\" { }";
        assert!(codes_with_dependency(&[user, sub], Some(gt)).is_empty());
    }

    /// The codes of `files` compiled as an app next to the platform app System (`PLATFORM_APP_ID`), whose
    /// source is `platform`. Every app sees the platform app without declaring it.
    fn codes_with_platform(files: &[&str], platform: &str) -> Vec<String> {
        let mut v: Vec<String> = messages_with_platform(files, platform)
            .into_iter()
            .map(|m| m.split(' ').next().unwrap_or_default().to_string())
            .collect();
        v.sort();
        v
    }

    /// The `code message` lines of `files` compiled next to the platform app (see `codes_with_platform`).
    fn messages_with_platform(files: &[&str], platform: &str) -> Vec<String> {
        let apps = vec![
            AppSource {
                id: "a".into(),
                name: "a".into(),
                dependencies: vec![],
                files: files
                    .iter()
                    .enumerate()
                    .map(|(i, s)| (format!("f{i}.al"), s.to_string()))
                    .collect(),
            },
            AppSource {
                id: crate::workspace::PLATFORM_APP_ID.into(),
                name: "System".into(),
                dependencies: vec![],
                files: vec![("s.al".into(), platform.to_string())],
            },
        ];
        let kinds = vec![
            crate::workspace::AppKind::Workspace,
            crate::workspace::AppKind::Dependency,
        ];
        let ws = Workspace::load_apps(apps, kinds, &Default::default());
        let m = crate::members::build(&ws);
        crate::check::check(&ws, &m, &Catalog::load())
            .diagnostics
            .iter()
            .filter(|d| d.code.starts_with("AL"))
            .map(|d| format!("{} {}", d.code, d.message))
            .collect()
    }

    #[test]
    fn al0118_codeunit_name_is_not_judged_when_the_platform_app_is_not_loaded() {
        // must-not-compile 0740-subscribe-internal-event-get-role-center-banner.al:5 subscribes to
        // Codeunit::"System Action Triggers", a codeunit of the platform app System. BC always provides that
        // app, so without its source the name may exist and is not refused.
        let user = "codeunit 50000 C { procedure P(): Integer begin exit(Codeunit::\"System Action Triggers\"); end; }";
        let sub = "codeunit 50001 S { [EventSubscriber(ObjectType::Codeunit, Codeunit::\"System Action Triggers\", 'OnX', '', false, false)] local procedure H() begin end; }";
        assert!(codes(&[user, sub]).is_empty());
        // The platform app is loaded and has no such codeunit: the name does not exist.
        let other = "codeunit 2000000001 Other { }";
        assert_eq!(codes_with_platform(&[user], other), vec!["AL0118"]);
        assert_eq!(codes_with_platform(&[sub], other), vec!["AL0118"]);
        // And it resolves when the platform app has it.
        let sat = "codeunit 2000000002 \"System Action Triggers\" { }";
        assert!(codes_with_platform(&[user, sub], sat).is_empty());
    }

    #[test]
    fn al0322_a_field_control_property_bound_to_a_procedure_call() {
        // must-not-compile 2000: Visible, Enabled and Editable of a field control bound to a call of a page procedure.
        let t = "table 52000 T { fields { field(1; Id; Integer) { } field(2; A; Integer) { } } }";
        let page = "page 52000 P { PageType = Card; SourceTable = T; layout { area(Content) { field(Id; Rec.Id) { Visible = IsOk(); } \
                    field(A; Rec.A) { Enabled = IsOk(); Editable = IsOk(); } } } procedure IsOk(): Boolean begin exit(true); end; }";
        assert_eq!(codes(&[t, page]), vec!["AL0322", "AL0322", "AL0322"]);
        // On an action the same binding compiles (warning AL0573, pages/lt-qq-procedure-bound-action/QqBoundObjects.al).
        let action = "page 52001 Q { PageType = Card; actions { area(Processing) { action(X) { Visible = IsOk(); Enabled = IsOk(); } } } \
                      procedure IsOk(): Boolean begin exit(true); end; }";
        assert!(codes(&[action]).is_empty());
    }

    #[test]
    fn al0118_bare_names_are_not_judged_when_a_dependency_is_not_loaded() {
        // A codeunit with `TableNo` of a table of the missing dependency has an implicit `Rec`; a page with a
        // source table of it assigns the table's fields by their bare names.
        let cu = "codeunit 50000 C { TableNo = \"Sales Header\"; trigger OnRun() begin Rec.Foo := 1; end; }";
        let page = "page 50001 P { PageType = Card; SourceTable = \"Sales Header\"; trigger OnOpenPage() begin Foo := 1; Bar(); end; }";
        assert!(codes_with_dependency(&[cu, page], None).is_empty());
        // With the dependency loaded (a table that has no such field) the page's names are judged.
        let t = "table 60000 \"Sales Header\" { fields { field(1; Id; Integer) { } } }";
        assert!(codes_with_dependency(&[page], Some(t)).contains(&"AL0118".to_string()));
        // A page extension of a base page of the missing dependency calls the base page's procedure.
        let ext = "pageextension 50002 PE extends \"Customer Card\" { trigger OnOpenPage() begin BaseProc(); Rec.Foo := 1; end; }";
        assert!(codes_with_dependency(&[ext], None).is_empty());
    }

    #[test]
    fn al0118_bare_call_of_an_unknown_name_in_a_codeunit() {
        let cu = "codeunit 50000 C { procedure Make(): Guid begin exit(CreateSequentialGuid()); end; }";
        assert_eq!(codes(&[cu]), vec!["AL0118"]);
        let ok = "codeunit 50000 C { procedure Make(): Guid begin exit(CreateGuid()); end; procedure Other() begin Make(); end; }";
        assert!(codes(&[ok]).is_empty());
    }

    #[test]
    fn al0118_bare_call_of_an_unknown_name_in_a_table_page_report_and_page_extension() {
        let t = "table 50000 T { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { Clustered = true; } }
            procedure Run() begin NoSuchProcedure(); end; }";
        assert_eq!(codes(&[t]), vec!["AL0118"]);
        let t = "table 50000 T { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { Clustered = true; } } }";
        let page =
            "page 50001 P { PageType = Card; SourceTable = T; trigger OnOpenPage() begin NoSuchProcedure(); end; }";
        assert_eq!(codes(&[t, page]), vec!["AL0118"]);
        let report = "report 50002 R { dataset { dataitem(Row; T) { column(Id; Id) { } } } trigger OnPreReport() begin NoSuchProcedure(); end; }";
        assert_eq!(codes(&[t, report]), vec!["AL0118"]);
        let ext = ext("trigger OnOpenPage() begin NoSuchProcedure(); end;");
        assert_eq!(codes(&[BASE, &ext]), vec!["AL0118"]);
        // What a table, page or report can call is not judged: an own procedure, an implicit record method,
        // a page method, a system function.
        let ok = "table 50000 T { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { Clustered = true; } }
            procedure Own() begin end;
            procedure Run() begin Own(); Init(); SetRange(Id, 1); Message('x'); end; }";
        assert!(codes(&[ok]).is_empty());
        let ok_page = "page 50001 P { PageType = Card; SourceTable = T; trigger OnOpenPage() begin SetRange(Id, 1); Update(false); end; }";
        assert!(codes(&[t, ok_page]).is_empty());
    }

    #[test]
    fn al0118_is_not_given_for_the_codeunits_own_run() {
        // A bare `Run()` in a codeunit runs the codeunit itself (a method of the Codeunit type, called on the
        // running instance); real BC compiles it (NP Retail's scanner import codeunits do).
        let cu = "codeunit 50000 C { trigger OnRun() begin end;
            procedure Go() begin if not Run() then Message('failed'); end; }";
        assert!(codes(&[cu]).is_empty(), "{:?}", codes(&[cu]));
        // A name that is a method of no type is still refused.
        let bad = "codeunit 50000 C { procedure Go() begin NoSuchProcedure(); end; }";
        assert_eq!(codes(&[bad]), vec!["AL0118"]);
    }

    #[test]
    fn al0118_is_not_given_for_a_procedure_of_a_sibling_page_extension() {
        // Microsoft's test library calls `RefreshFactboxes`, an `internal procedure` of another page extension
        // of the same base page (Tests-TestLibraries UsersPBTTest.PageExt.al, Base Application
        // UsersPerfFactboxes.PageExt.al); real BC compiles it.
        let sibling = "pageextension 50003 BPE2 extends BP { internal procedure Refresh(a: Integer) begin end; }";
        let caller = "pageextension 50002 BPE extends BP { trigger OnOpenPage() begin Refresh(1); end; }";
        assert!(
            codes(&[BASE, sibling, caller]).is_empty(),
            "{:?}",
            codes(&[BASE, sibling, caller])
        );
        // Without the sibling the name is unknown.
        assert_eq!(codes(&[BASE, caller]), vec!["AL0118"]);
    }

    #[test]
    fn al0118_rec_in_a_codeunit_without_table_no() {
        let cu = "table 50000 T { fields { field(1; Id; Integer) { } } }
            codeunit 50001 C { procedure P() begin Rec.Id := 1; end; }";
        assert_eq!(codes(&[cu]), vec!["AL0118"]);
        let ok = "table 50000 T { fields { field(1; Id; Integer) { } } }
            codeunit 50001 C { TableNo = T; trigger OnRun() begin Rec.Id := 1; end; }";
        assert!(codes(&[ok]).is_empty());
    }

    #[test]
    fn al0132_record_field_or_method_that_does_not_exist() {
        let t = "table 50000 T { fields { field(1; Id; Integer) { } } }";
        let body = |stmt: &str| format!("codeunit 50001 C {{ procedure P() var R: Record T; begin {stmt} end; }}");
        assert_eq!(codes(&[t, &body("R.NoSuchField := 1;")]), vec!["AL0132"]);
        assert_eq!(codes(&[t, &body("R.NoSuchMethod();")]), vec!["AL0132"]);
        assert!(codes(&[t, &body("R.Id := 1; R.Init(); R.SetRange(Id, 1);")]).is_empty());
        // A table that an extension adds to is not judged.
        let ext = "tableextension 50002 TE extends T { fields { field(50000; Extra; Integer) { } } }";
        assert!(codes(&[t, ext, &body("R.Extra := 1;")]).is_empty());
    }

    #[test]
    fn al0161_in_a_report_extension() {
        let t = "table 50000 T { fields { field(1; Id; Integer) { } } }";
        let base = "report 50001 BR { dataset { dataitem(Row; T) { column(Id; Id) { } } } var Hidden: Integer;
            local procedure SecretLocal(): Integer begin exit(1); end; protected var Shown: Integer; }";
        let var = "reportextension 50002 E extends BR { trigger OnPreReport() begin Hidden := 1; end; }";
        assert_eq!(codes(&[t, base, var]), vec!["AL0161"]);
        let proc_ = "reportextension 50002 E extends BR { trigger OnPreReport() var V: Integer; begin V := SecretLocal(); end; }";
        assert_eq!(codes(&[t, base, proc_]), vec!["AL0161"]);
        let ok = "reportextension 50002 E extends BR { trigger OnPreReport() begin Shown := 1; end; }";
        assert!(codes(&[t, base, ok]).is_empty());
    }

    #[test]
    fn al0132_execution_mode_members_that_do_not_exist() {
        for m in ["Install", "Uninstall", "Upgrade", "Report", "Profile"] {
            let cu =
                format!("codeunit 50000 C {{ procedure M(): ExecutionMode begin exit(ExecutionMode::{m}); end; }}");
            assert_eq!(codes(&[&cu]), vec!["AL0132"], "{m}");
        }
        let ok = "codeunit 50000 C { procedure M(): ExecutionMode begin exit(ExecutionMode::Standard); end; }";
        assert!(codes(&[ok]).is_empty());
    }

    fn enum_pair(src_compat: bool, dst_compat: bool) -> String {
        format!(
            "enum 50000 \"MNC Src\" {{ Extensible = true; AssignmentCompatibility = {src_compat}; value(0; None) {{ }} value(1; One) {{ }} value(7; Seven) {{ }} }}
             enum 50001 \"MNC Dst\" {{ Extensible = true; AssignmentCompatibility = {dst_compat}; value(0; None) {{ }} value(1; One) {{ }} value(2; Two) {{ }} }}
             codeunit 50002 \"MNC Assign\" {{ procedure Do(): Integer var S: Enum \"MNC Src\"; D: Enum \"MNC Dst\"; begin D := S; exit(D.AsInteger()); end; }}"
        )
    }

    /// The AL diagnostics of a compile of `src`, as `code message`.
    fn messages(src: &str) -> Vec<String> {
        let ws = load(&[src]);
        let m = crate::members::build(&ws);
        crate::check::check(&ws, &m, &Catalog::load())
            .diagnostics
            .iter()
            .filter(|d| d.code.starts_with("AL"))
            .map(|d| format!("{} {}", d.code, d.message))
            .collect()
    }

    // must-not-compile 0074 (only the source states AssignmentCompatibility) and 0075 (neither does):
    // AL0122. A target that states it compiles whatever the source says
    // (types/PXEnumRecordRef.al:7-10, :74-108); so does an Integer into any enum (:11-12, :137-196).
    #[test]
    fn al0122_an_enum_of_another_type_into_a_target_without_assignment_compatibility() {
        let want = vec!["AL0122 Cannot implicitly convert type 'Enum \"MNC Src\"' to 'Enum \"MNC Dst\"'".to_string()];
        assert_eq!(messages(&enum_pair(true, false)), want, "0074");
        assert_eq!(messages(&enum_pair(false, false)), want, "0075");
        assert!(messages(&enum_pair(false, true)).is_empty());
        assert!(messages(&enum_pair(true, true)).is_empty());
        // The property left out (false by default), into a table field.
        let field = "enum 50000 \"MNC Src\" { value(0; None) { } value(1; One) { } }
             enum 50001 \"MNC Dst\" { value(0; None) { } value(1; One) { } }
             table 50003 \"MNC Row\" { fields { field(1; Id; Integer) { } field(2; Kind; Enum \"MNC Dst\") { } } }
             codeunit 50002 \"MNC Assign\" { procedure Do() var S: Enum \"MNC Src\"; R: Record \"MNC Row\"; begin R.Kind := S; end; }";
        assert_eq!(messages(field), want, "field");
        let same = "enum 50001 \"MNC Dst\" { value(0; None) { } value(1; One) { } }
             codeunit 50002 \"MNC Assign\" { procedure Do() var D: Enum \"MNC Dst\"; E: Enum \"MNC Dst\"; I: Integer; begin D := E; I := 1; D := I; end; }";
        assert!(messages(same).is_empty());
    }

    // must-not-compile 0069 (`T := I`, an Integer variable into a Text variable) and 0070 (`I := 'text'`, a
    // text literal into an Integer variable): AL0122. A one-character literal is a Char, which an Integer takes;
    // the other shapes (a Text variable into an Integer, an Integer into a Code) are not judged.
    #[test]
    fn al0122_integer_into_text_and_text_literal_into_integer() {
        let cu = |body: &str| {
            format!(
                "codeunit 50001 \"MNC Codeunit\" {{ procedure DoIt() var I: Integer; T: Text; L: Text[30]; C: Code[20]; begin {body} end; }}"
            )
        };
        assert_eq!(
            messages(&cu("T := I;")),
            vec!["AL0122 Cannot implicitly convert type 'Integer' to 'Text'".to_string()]
        );
        assert_eq!(
            messages(&cu("I := 'text';")),
            vec!["AL0122 Cannot implicitly convert type 'Text' to 'Integer'".to_string()]
        );
        assert_eq!(messages(&cu("L := I;")).len(), 1);
        for ok in [
            "I := 'A';",
            "T := Format(I);",
            "I := 5; T := 'x';",
            "C := 'x';",
            "I := T.IndexOf('x');",
        ] {
            assert!(messages(&cu(ok)).is_empty(), "{ok}");
        }
    }

    // must-not-compile 0500 (corpus tests/must-not-compile/cases/0500-char-minus-text.al:1-2): `C - T` and `T - C`
    // with a Char variable and an unbounded Text variable. AL0175. A Text literal with a Char compiles (an
    // Integer); Text[30], Code and other shapes are not pinned and not judged.
    #[test]
    fn al0175_char_minus_text_variable() {
        let cu = |body: &str| {
            format!(
                "codeunit 50001 \"MNC Codeunit\" {{ procedure DoIt() var C: Char; T: Text; L: Text[30]; I: Integer; begin {body} end; }}"
            )
        };
        assert_eq!(
            messages(&cu("I := C - T;")),
            vec!["AL0175 Operator '-' cannot be applied to operands of type 'Char' and 'Text'".to_string()]
        );
        assert_eq!(
            messages(&cu("I := T - C;")),
            vec!["AL0175 Operator '-' cannot be applied to operands of type 'Text' and 'Char'".to_string()]
        );
        for ok in [
            "I := C - 'a';",
            "I := 'a' - C;",
            "I := C - I;",
            "I := C - C;",
            "I := C - L;",
            "T := T + T;",
        ] {
            assert!(messages(&cu(ok)).iter().all(|m| !m.starts_with("AL0175")), "{ok}");
        }
    }

    // must-not-compile 0501: a Text literal and a Text variable assigned to a Decimal. AL0122.
    #[test]
    fn al0122_text_into_decimal() {
        let cu = |body: &str| {
            format!(
                "codeunit 50001 \"MNC Codeunit\" {{ procedure DoIt() var D: Decimal; T: Text; V: Variant; begin {body} end; }}"
            )
        };
        let want = "AL0122 Cannot implicitly convert type 'Text' to 'Decimal'".to_string();
        assert_eq!(messages(&cu("D := '100';")), vec![want.clone()]);
        assert_eq!(messages(&cu("D := T;")), vec![want]);
        for ok in ["D := V;", "Evaluate(D, T);", "D := 5;"] {
            assert!(messages(&cu(ok)).is_empty(), "{ok}");
        }
    }

    #[test]
    fn al0197_every_declaration_of_a_repeated_name_is_reported() {
        let cu = "codeunit 50000 \"Same\" { } codeunit 50001 \"same\" { }";
        assert_eq!(codes(&[cu]), vec!["AL0197", "AL0197"]);
        // Another object type with the name is fine.
        let ok = "codeunit 50000 \"Same\" { } table 50001 \"Same\" { fields { field(1; Id; Integer) { } } }";
        assert!(codes(&[ok]).is_empty());
    }

    const PARENT: &str = "table 50000 P { fields { field(1; Id; Integer) { } field(2; Pid; Integer) { } } }";

    #[test]
    fn al0365_crossjoin_with_a_data_item_link() {
        let q = |props: &str| {
            format!(
                "query 50002 Q {{ elements {{ dataitem(Parent; P) {{ column(Id; Id) {{ }}
                   dataitem(Child; P) {{ {props} column(CId; Id) {{ }} }} }} }} }}"
            )
        };
        assert_eq!(
            codes(&[PARENT, &q("SqlJoinType = CrossJoin; DataItemLink = Pid = Parent.Id;")]),
            vec!["AL0365"]
        );
        assert!(codes(&[PARENT, &q("SqlJoinType = InnerJoin; DataItemLink = Pid = Parent.Id;")]).is_empty());
        assert!(codes(&[PARENT, &q("SqlJoinType = CrossJoin;")]).is_empty());
    }

    // must-not-compile 2010: a global variable is no named field; a field written with a data item name compiles
    // (errors/lt-ms-refusals-138/P138ColCaptionQualified.al, QS_*).
    #[test]
    fn al0385_include_caption_on_a_global_variable_column() {
        let r = |col: &str, vars: &str| {
            format!("report 50001 R {{ dataset {{ dataitem(Row; P) {{ {col} }} }} var {vars} }}")
        };
        assert_eq!(
            codes(&[
                PARENT,
                &r("column(C; GCounter) { IncludeCaption = true; }", "GCounter: Integer;")
            ]),
            vec!["AL0385"]
        );
        assert!(codes(&[PARENT, &r("column(C; GCounter) { }", "GCounter: Integer;")]).is_empty());
        // A field of a data item's table with the name of a global is not judged.
        assert!(codes(&[PARENT, &r("column(C; Pid) { IncludeCaption = true; }", "Pid: Integer;")]).is_empty());
        assert!(
            codes(&[
                PARENT,
                &r("column(C; Row.Id) { IncludeCaption = true; }", "GCounter: Integer;")
            ])
            .is_empty()
        );
    }

    #[test]
    fn al0385_include_caption_on_an_expression_column() {
        let r = |col: &str| format!("report 50001 R {{ dataset {{ dataitem(Row; P) {{ {col} }} }} }}");
        assert_eq!(
            codes(&[PARENT, &r("column(IdText; Format(Id)) { IncludeCaption = true; }")]),
            vec!["AL0385"]
        );
        assert!(codes(&[PARENT, &r("column(IdText; Id) { IncludeCaption = true; }")]).is_empty());
        // An indexed variable's field is a named field (Base Application's "Quantity Explosion of BOM").
        let indexed =
            "report 50001 R { dataset { dataitem(Row; P) { column(IdText; Arr[Level].Id) { IncludeCaption = true; } } }
            var Arr: array[3] of Record P; Level: Integer; }";
        assert!(codes(&[PARENT, indexed]).is_empty(), "{:?}", codes(&[PARENT, indexed]));
        assert!(codes(&[PARENT, &r("column(IdText; Format(Id)) { }")]).is_empty());
    }

    fn ctx(target: AppTarget, runtime: Option<(u32, u32)>, keys: &[&str]) -> AppContext {
        AppContext {
            target,
            runtime,
            app_json_keys: keys.iter().map(|k| k.to_string()).collect(),
            bc_major: None,
        }
    }

    #[test]
    fn al0296_al0161_and_al0433_for_the_platform_tables_of_bc_27_only() {
        let cu = |t: &str| {
            format!("codeunit 50000 C {{ procedure P(): Integer var R: Record \"{t}\"; begin exit(R.Count()); end; }}")
        };
        let at = |major, t: &str| {
            let mut c = ctx(AppTarget::Cloud, None, &[]);
            c.bc_major = Some(major);
            let mut v = ctx_codes(&cu(t), &c);
            v.sort();
            v
        };
        assert_eq!(at(27, "Designed Query"), vec!["AL0296"]);
        assert_eq!(at(27, "Agent Data"), vec!["AL0161", "AL0296"]);
        assert_eq!(at(27, "Profile"), vec!["AL0296", "AL0433"]);
        // Not pinned for another major, and a table that is not on the list is not judged.
        assert!(at(28, "Designed Query").is_empty());
        assert!(at(27, "Company").is_empty());
        // Cases 0006 and 0014 hold for every major.
        assert_eq!(at(28, "Published Application"), vec!["AL0296"]);
    }

    #[test]
    fn al0296_cloud_app_names_an_onprem_platform_table() {
        let cu = "codeunit 50000 C { procedure P(): Integer var R: Record \"Published Application\"; begin exit(R.Count()); end; }";
        assert_eq!(ctx_codes(cu, &ctx(AppTarget::Cloud, None, &[])), vec!["AL0296"]);
        assert!(ctx_codes(cu, &ctx(AppTarget::OnPrem, None, &[])).is_empty());
        let other =
            "codeunit 50000 C { procedure P(): Integer var R: Record \"Company\"; begin exit(R.Count()); end; }";
        assert!(ctx_codes(other, &ctx(AppTarget::Cloud, None, &[])).is_empty());
    }

    #[test]
    fn al0124_assembly_probing_paths_at_runtime_16() {
        let cu = "codeunit 50000 C { }";
        let keys = ["assemblyProbingPaths"];
        assert_eq!(
            ctx_codes(cu, &ctx(AppTarget::Cloud, Some((16, 0)), &keys)),
            vec!["AL0124"]
        );
        for r in [(15, 0), (16, 1), (17, 0), (17, 1), (18, 0)] {
            assert_eq!(
                ctx_codes(cu, &ctx(AppTarget::Cloud, Some(r), &keys)),
                vec!["AL0124"],
                "{r:?}"
            );
        }
        // Another runtime, another target and no key are not judged.
        assert!(ctx_codes(cu, &ctx(AppTarget::Cloud, Some((19, 0)), &keys)).is_empty());
        assert!(ctx_codes(cu, &ctx(AppTarget::OnPrem, Some((16, 0)), &keys)).is_empty());
        assert!(ctx_codes(cu, &ctx(AppTarget::Cloud, Some((16, 0)), &[])).is_empty());
        assert!(ctx_codes(cu, &ctx(AppTarget::Cloud, None, &keys)).is_empty());
    }

    #[test]
    fn every_implemented_code_is_listed_once() {
        let mut v: Vec<_> = IMPLEMENTED.iter().map(|(c, _)| *c).collect();
        v.sort();
        v.dedup();
        assert_eq!(v.len(), IMPLEMENTED.len());
        assert_eq!(not_checked_reason("AL0161"), None);
        assert_eq!(not_checked_reason("AL9999"), Some("not implemented"));
    }

    fn cloud() -> AppContext {
        AppContext {
            target: AppTarget::Cloud,
            runtime: Some((16, 0)),
            app_json_keys: Vec::new(),
            bc_major: Some(27),
        }
    }

    #[test]
    fn al0296_a_cloud_app_calls_a_method_with_scope_onprem() {
        let last = "codeunit 50000 C { procedure P() var I: ErrorInfo; begin I := GetLastErrorObject(); end; }";
        assert_eq!(ctx_codes(last, &cloud()), vec!["AL0296"]);
        // A procedure of the app with that name is not the platform's.
        let own = "codeunit 50000 C { procedure GetLastErrorObject(): Integer begin end; procedure P() begin GetLastErrorObject(); end; }";
        assert!(ctx_codes(own, &cloud()).is_empty());
        let unwrap = "codeunit 50000 C { procedure P(S: SecretText): Text begin exit(S.Unwrap()); end; }";
        assert_eq!(ctx_codes(unwrap, &cloud()), vec!["AL0296"]);
        let file = "codeunit 50000 C { procedure P(): Boolean var F: File; begin exit(F.Open('x')); end; }";
        assert_eq!(ctx_codes(file, &cloud()), vec!["AL0296"]);
        // Another type's Unwrap, and an OnPrem app, are fine.
        let other = "codeunit 50000 C { procedure P(S: Text): Text begin exit(S.Unwrap()); end; }";
        assert!(ctx_codes(other, &cloud()).is_empty());
        let onprem = AppContext {
            target: AppTarget::OnPrem,
            ..cloud()
        };
        assert!(ctx_codes(file, &onprem).is_empty());
    }

    #[test]
    fn al0124_orderby_on_a_query_data_item_and_al0132_length_on_a_text() {
        let q = "table 50000 T { fields { field(1; Id; Integer) { } } }
            query 50001 Q { elements { dataitem(Row; T) { OrderBy = ascending(Id); column(Id; Id) { } } } }";
        assert_eq!(codes(&[q]), vec!["AL0124"]);
        let len = "codeunit 50000 C { procedure P(): Integer var V: Text; begin exit(V.Length()); end; }";
        assert_eq!(codes(&[len]), vec!["AL0132"]);
        let strlen = "codeunit 50000 C { procedure P(): Integer var V: Text; begin exit(StrLen(V)); end; }";
        assert!(codes(&[strlen]).is_empty());
    }

    #[test]
    fn al0161_this_names_a_base_variable_that_is_not_protected() {
        let base = "table 50000 T { fields { field(1; Id; Integer) { } } }
            report 50001 R { dataset { dataitem(Row; T) { } } var BaseVar: Text; protected var Shown: Text; }";
        let bad = "reportextension 50002 E extends R { trigger OnPreReport() begin this.BaseVar := 'x'; end; }";
        assert_eq!(codes(&[base, bad]), vec!["AL0161"]);
        let ok = "reportextension 50002 E extends R { trigger OnPreReport() begin Message(this.Shown); end; }";
        assert!(codes(&[base, ok]).is_empty());
    }

    // must-not-compile 0200 and 0205: an undeclared name in a codeunit (assigned, or passed as an argument).
    // Declared variables, labels, procedures, system functions without parentheses and the implicit Rec of a
    // codeunit with TableNo are names.
    #[test]
    fn al0118_undeclared_name_in_a_codeunit() {
        let assign = "codeunit 50001 C { procedure Run1() begin Missing := 1; end; }";
        assert_eq!(
            messages(assign),
            vec!["AL0118 The name 'Missing' does not exist in the current context."]
        );
        let arg = "codeunit 50001 C { procedure Run1() begin Error(MissingLbl); end; }";
        assert_eq!(codes(&[arg]), vec!["AL0118"]);
        let ok = "table 50000 T { fields { field(1; Id; Integer) { } } }
            codeunit 50001 C { var G: Integer; Lbl: Label 'x';
              procedure Run1(P: Integer): Integer var L: Integer; begin L := P + G; Error(Lbl); L := Today; exit(Helper()); end;
              procedure Helper(): Integer begin exit(1); end; }
            codeunit 50002 D { TableNo = T; trigger OnRun() begin Id := 1; end; }";
        assert!(codes(&[ok]).is_empty(), "{:?}", messages(ok));
    }

    // must-not-compile 0206 to 0210: an undeclared name assigned in an object-level trigger of a table, page,
    // report, query or xmlport. Fields, data items, columns and text elements of the object are names.
    #[test]
    fn al0118_undeclared_name_in_an_object_trigger() {
        let t = "table 50000 T { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { Clustered = true; } } }";
        let bodies = [
            "table 50009 T2 { fields { field(1; Id; Integer) { } } trigger OnInsert() begin Undeclared := 5; end; }",
            "page 50010 P { PageType = Card; trigger OnOpenPage() begin Undeclared := 5; end; }",
            "report 50011 R { trigger OnPreReport() begin Undeclared := 5; end; }",
            "query 50013 Q { elements { dataitem(Row; T) { column(Id; Id) { } } } trigger OnBeforeOpen() begin Undeclared := 5; end; }",
            "xmlport 50014 X { trigger OnPreXmlPort() begin Undeclared := 5; end; }",
        ];
        for b in bodies {
            assert_eq!(codes(&[t, b]), vec!["AL0118"], "{b}");
        }
        let ok = [
            "table 50009 T2 { fields { field(1; Id; Integer) { } } trigger OnInsert() begin Id := 5; Rec.Id := 1; xRec.Id := 2; end; }",
            "page 50010 P { PageType = Card; SourceTable = T; trigger OnOpenPage() begin Id := 5; end; }",
            "report 50011 R { dataset { dataitem(Row; T) { } } trigger OnPreReport() begin Row.Id := 5; end; }",
            "query 50013 Q { elements { dataitem(Row; T) { column(Id; Id) { } } } trigger OnBeforeOpen() begin SetFilter(Id, '1'); end; }",
            "xmlport 50014 X { schema { textelement(Root) { } } trigger OnPreXmlPort() begin Root := 'x'; end; }",
        ];
        for b in ok {
            assert!(codes(&[t, b]).is_empty(), "{b}: {:?}", messages(b));
        }
    }

    // must-not-compile 0223 and 0222: a codeunit named by an id-or-name that does not exist.
    #[test]
    fn al0118_codeunit_that_does_not_exist() {
        // The name is judged only with the platform app loaded (see
        // al0118_codeunit_name_is_not_judged_when_the_platform_app_is_not_loaded).
        let platform = "codeunit 2000000001 \"Some Platform Codeunit\" { }";
        let run = "codeunit 50001 C { procedure Run1() begin Codeunit.Run(Codeunit::\"MNC No Such Codeunit\"); end; }";
        assert_eq!(
            messages_with_platform(&[run], platform),
            vec!["AL0118 The name '\"MNC No Such Codeunit\"' does not exist in the current context."]
        );
        let sub = "codeunit 50001 C { [EventSubscriber(ObjectType::Codeunit, Codeunit::\"MNC No Such Codeunit\", 'OnX', '', false, false)] local procedure Handle() begin end; }";
        assert_eq!(codes_with_platform(&[sub], platform), vec!["AL0118"]);
        let ok =
            "codeunit 50002 Other { } codeunit 50001 C { procedure Run1() begin Codeunit.Run(Codeunit::Other); end; }";
        assert!(codes_with_platform(&[ok], platform).is_empty());
    }

    // must-not-compile 0224 to 0226, 0228 to 0230, 0233 to 0236: a method the type does not have.
    #[test]
    fn al0132_method_that_a_library_type_does_not_have() {
        let cases = [
            (
                "T: Text",
                "T.Contain('x')",
                "'Text' does not contain a definition for 'Contain'",
            ),
            ("T: Text", "T.Size()", "'Text' does not contain a definition for 'Size'"),
            (
                "L: List of [Text]",
                "L.Length()",
                "'List of [Text]' does not contain a definition for 'Length'",
            ),
            (
                "D: Dictionary of [Text, Integer]",
                "D.Size()",
                "'Dictionary of [Text, Integer]' does not contain a definition for 'Size'",
            ),
            (
                "J: JsonObject",
                "J.Put('a', 1)",
                "'JsonObject' does not contain a definition for 'Put'",
            ),
            (
                "J: JsonToken",
                "J.GetName()",
                "'JsonToken' does not contain a definition for 'GetName'",
            ),
            (
                "H: HttpClient; M: HttpRequestMessage; R: HttpResponseMessage",
                "H.Send2(M, R)",
                "'HttpClient' does not contain a definition for 'Send2'",
            ),
            (
                "RR: RecordRef",
                "RR.Fieldd(1)",
                "'RecordRef' does not contain a definition for 'Fieldd'",
            ),
            (
                "B: Boolean",
                "B.Invert()",
                "'Boolean' does not contain a definition for 'Invert'",
            ),
            (
                "TB: TextBuilder",
                "TB.Add('x')",
                "'TextBuilder' does not contain a definition for 'Add'",
            ),
            // must-not-compile 2048: JsonArray has RemoveAt(Index), no Remove(JsonToken).
            (
                "J: JsonArray; T: JsonToken",
                "J.Remove(T)",
                "'JsonArray' does not contain a definition for 'Remove'",
            ),
        ];
        for (var, stmt, message) in cases {
            let cu = format!("codeunit 50001 C {{ procedure Run1() var {var}; begin {stmt}; end; }}");
            assert_eq!(messages(&cu), vec![format!("AL0132 {message}")], "{stmt}");
        }
        let ok = [
            ("T: Text", "T.Contains('x')"),
            ("L: List of [Text]", "L.Count()"),
            ("D: Dictionary of [Text, Integer]", "D.ContainsKey('a')"),
            ("J: JsonObject", "J.Add('a', 1)"),
            ("J: JsonToken", "J.IsObject()"),
            (
                "H: HttpClient; M: HttpRequestMessage; R: HttpResponseMessage",
                "H.Send(M, R)",
            ),
            ("RR: RecordRef", "RR.Field(1)"),
            ("TB: TextBuilder", "TB.Append('x')"),
        ];
        for (var, stmt) in ok {
            let cu = format!("codeunit 50001 C {{ procedure Run1() var {var}; begin {stmt}; end; }}");
            assert!(codes(&[&cu]).is_empty(), "{stmt}: {:?}", messages(&cu));
        }
    }

    // must-not-compile 0204: an enum value the enum does not have. A value an enum extension adds is one.
    #[test]
    fn al0132_enum_value_that_the_enum_does_not_have() {
        let e = "enum 50006 \"MNC E204\" { Extensible = true; value(0; Red) { } value(1; Green) { } }";
        let cu = |v: &str| {
            format!(
                "codeunit 50007 C {{ procedure Run1() var E: Enum \"MNC E204\"; begin E := \"MNC E204\"::{v}; end; }}"
            )
        };
        let bad = cu("Purple");
        assert_eq!(
            messages(&format!("{e} {bad}")),
            vec!["AL0132 '\"MNC E204\"' does not contain a definition for 'Purple'"]
        );
        assert!(codes(&[e, &cu("Green")]).is_empty());
        let ext = "enumextension 50008 \"MNC E204 X\" extends \"MNC E204\" { value(50000; Purple) { } }";
        assert!(codes(&[e, ext, &bad]).is_empty());
    }

    // must-not-compile 0212: a page field bound to a column the source table does not have.
    #[test]
    fn al0132_page_field_bound_to_a_column_the_table_does_not_have() {
        let t = "table 50016 \"MNC T212\" { fields { field(1; Id; Integer) { } field(2; \"Long Name\"; Text[30]) { } }
            procedure Calc(): Integer begin end; }";
        let page = |src: &str| {
            format!(
                "page 50017 \"MNC P212\" {{ PageType = List; SourceTable = \"MNC T212\";
                  layout {{ area(Content) {{ repeater(Rep) {{ field(X; {src}) {{ ApplicationArea = All; }} }} }} }} }}"
            )
        };
        assert_eq!(
            messages(&format!("{t} {}", page("Rec.NoSuchColumn"))),
            vec!["AL0132 'Record \"MNC T212\"' does not contain a definition for 'NoSuchColumn'"]
        );
        for ok in [
            "Rec.Id",
            "Rec.\"Long Name\"",
            "Rec.SystemId",
            "Rec.Calc()",
            "Id",
            "Rec.Count",
        ] {
            assert!(codes(&[t, &page(ok)]).is_empty(), "{ok}");
        }
        let ext = "tableextension 50018 TE extends \"MNC T212\" { fields { field(50000; Extra; Integer) { } } }";
        assert!(codes(&[t, ext, &page("Rec.Extra")]).is_empty());
    }

    // must-not-compile 0260 to 0268: a variable assigned to a variable of a type it does not convert to.
    #[test]
    fn al0122_variable_assigned_to_a_type_it_does_not_convert_to() {
        let defs = "enum 50071 \"MNC E\" { value(0; A) { } }
            table 50075 \"MNC T\" { fields { field(1; Id; Integer) { } } }";
        let cases = [
            ("B: Boolean; V: Decimal", "B := V", "Decimal", "Boolean"),
            ("B: Boolean; V: Integer", "B := V", "Integer", "Boolean"),
            ("B: Text; V: Date", "B := V", "Date", "Text"),
            ("B: Date; V: Boolean", "B := V", "Boolean", "Date"),
            ("B: Date; V: Text", "B := V", "Text", "Date"),
            ("B: Date; V: Decimal", "B := V", "Decimal", "Date"),
            ("B: Enum \"MNC E\"; V: Variant", "B := V", "Variant", "Enum \"MNC E\""),
            (
                "B: Integer; V: Record \"MNC T\"",
                "B := V",
                "Record \"MNC T\"",
                "Integer",
            ),
            ("B: Enum \"MNC E\"", "B := 'A'", "Char", "Enum \"MNC E\""),
        ];
        for (vars, stmt, from, to) in cases {
            let cu = format!("{defs} codeunit 50001 C {{ procedure Run1() var {vars}; begin {stmt}; end; }}");
            assert_eq!(
                messages(&cu),
                vec![format!("AL0122 Cannot implicitly convert type '{from}' to '{to}'")],
                "{vars}: {stmt}"
            );
        }
        let ok = [
            "B: Boolean; V: Boolean",
            "B: Text; V: Text",
            "B: Date; V: Date",
            "B: Decimal; V: Integer",
            "B: Variant; V: Date",
        ];
        for vars in ok {
            let cu = format!("{defs} codeunit 50001 C {{ procedure Run1() var {vars}; begin B := V; end; }}");
            assert!(codes(&[&cu]).is_empty(), "{vars}: {:?}", messages(&cu));
        }
        let text = format!("{defs} codeunit 50001 C {{ procedure Run1() var B: Text; begin B := 'A'; end; }}");
        assert!(codes(&[&text]).is_empty());
    }

    // must-not-compile 0269 to 0273, 0285, 0286: the scalar pairs, a condition that is no Boolean.
    #[test]
    fn al0122_more_scalar_pairs_and_conditions() {
        let cases = [
            ("D: Date; M: DateTime", "D := M", "DateTime", "Date"),
            ("B: Boolean", "B := 'true'", "Text", "Boolean"),
            ("I: Integer; M: Time", "I := M", "Time", "Integer"),
            ("J: JsonObject; K: JsonToken", "J := K", "JsonToken", "JsonObject"),
            ("I: Integer", "if I then I := 0", "Integer", "Boolean"),
            ("T: Text", "while T do T := ''", "Text", "Boolean"),
        ];
        for (vars, stmt, from, to) in cases {
            let cu = format!("codeunit 50001 C {{ procedure Run1() var {vars}; begin {stmt}; end; }}");
            assert_eq!(
                messages(&cu),
                vec![format!("AL0122 Cannot implicitly convert type '{from}' to '{to}'")],
                "{vars}: {stmt}"
            );
        }
        let ok = [
            "B: Boolean; V: Boolean; if V then B := false",
            "T: Text; V: Boolean; while V do T := ''",
            "J: JsonToken; K: JsonObject; J := K",
            "B: Boolean; V: Text; B := V = 'x'",
            "B: Boolean; T: Text; B := T = ''",
        ];
        for stmt in ok {
            let (vars, body) = stmt.rsplit_once("; ").unwrap();
            let cu = format!("codeunit 50001 C {{ procedure Run1() var {vars}; begin {body}; end; }}");
            assert!(codes(&[&cu]).is_empty(), "{stmt}: {:?}", messages(&cu));
        }
    }

    // must-not-compile 0274 and 0275.
    #[test]
    fn al0122_exit_value_and_a_procedure_without_a_result() {
        let exit = "codeunit 50001 C { procedure Run1(): Integer begin exit('abc'); end; }";
        assert_eq!(
            messages(exit),
            vec!["AL0122 Cannot implicitly convert type 'Text' to 'Integer'"]
        );
        let none = "codeunit 50001 C { local procedure Nothing() begin end; procedure Run1() var I: Integer; begin I := Nothing(); end; }";
        assert_eq!(
            messages(none),
            vec!["AL0122 Cannot implicitly convert type 'None' to 'Integer'"]
        );
        let ok = "codeunit 50001 C { local procedure One(): Integer begin exit(1); end; procedure Run1(): Text var I: Integer; begin I := One(); exit('a'); end; procedure Run2(): Integer begin exit(One()); end; }";
        assert!(codes(&[ok]).is_empty(), "{:?}", messages(ok));
        // A TryFunction returns a Boolean without declaring one (corpus errors/lt-error-handling/ErTTryBasics.al:81).
        let try_fn = "codeunit 50001 C { [TryFunction] local procedure TryOk() begin end; procedure Run1() var Ok: Boolean; begin Ok := TryOk(); end; }";
        assert!(codes(&[try_fn]).is_empty(), "{:?}", messages(try_fn));
    }

    // TestType on a Test codeunit compiles on BC (corpus al-language/codeunit/lt-x-object-props/XObjectProps.al:11,
    // codeunits 75020-75023, every value of the enum: UnitTest, IntegrationTest, Uncategorized, AITest).
    #[test]
    fn al0124_testtype_on_a_test_codeunit_compiles() {
        for value in ["UnitTest", "IntegrationTest", "Uncategorized", "AITest"] {
            let cu =
                format!("codeunit 50001 C {{ Subtype = Test; TestType = {value}; RequiredTestIsolation = None; }}");
            assert!(codes(&[&cu]).is_empty(), "{value}: {:?}", messages(&cu));
        }
    }

    // must-not-compile 0305 to 0310, 0312, 0315: a property the object type does not have.
    #[test]
    fn al0124_a_property_the_object_does_not_have() {
        let table = "table 50120 T { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { Clustered = true; } } }";
        let cases = [
            (
                format!("{table} codeunit 50121 C {{ SourceTable = T; }}"),
                "SourceTable",
            ),
            (
                "table 50122 T { ApplicationArea = All; fields { field(1; Id; Integer) { } } }".to_string(),
                "ApplicationArea",
            ),
            ("report 50123 R { Editable = false; }".to_string(), "Editable"),
            (
                "table 50124 T { PageType = List; fields { field(1; Id; Integer) { } } }".to_string(),
                "PageType",
            ),
            ("page 50125 P { TableNo = 18; }".to_string(), "TableNo"),
            (
                "codeunit 50126 C { DataPerCompany = false; }".to_string(),
                "DataPerCompany",
            ),
            (
                "codeunit 50128 C { NoSuchProperty = true; }".to_string(),
                "NoSuchProperty",
            ),
            (
                format!(
                    "{table} page 50132 P {{ PageType = List; SourceTable = T; layout {{ area(Content) {{ repeater(Rep) {{ field(Id; Rec.Id) {{ ApplicationArea = All; InitValue = 5; }} }} }} }} }}"
                ),
                "InitValue",
            ),
        ];
        for (src, prop) in cases {
            assert_eq!(
                messages(&src),
                vec![format!(
                    "AL0124 The property '{prop}' cannot be used in this context. Verify the property is available for the current object type."
                )],
                "{src}"
            );
        }
        let ok = [
            "codeunit 50001 C { Subtype = Test; TestPermissions = Disabled; SingleInstance = true; TableNo = 18; Permissions = tabledata T = r; Access = Internal; Description = 'x'; EventSubscriberInstance = Manual; }",
            "table 50001 T { DataPerCompany = false; Caption = 'x'; LookupPageId = 5; fields { field(1; Id; Integer) { InitValue = 5; } } }",
            "page 50001 P { PageType = List; ApplicationArea = All; Editable = false; UsageCategory = Lists; SourceTable = T; }",
            "report 50001 R { ApplicationArea = All; UsageCategory = Lists; ProcessingOnly = true; }",
        ];
        for src in ok {
            let full = format!("table 50000 T {{ fields {{ field(1; Id; Integer) {{ }} }} }} {src}")
                .replace("table 50001 T", "table 50002 T2");
            assert!(codes(&[&full]).is_empty(), "{src}: {:?}", messages(&full));
        }
    }

    // must-not-compile 0316 to 0321: a trigger the object type does not have.
    #[test]
    fn al0162_a_trigger_the_object_does_not_have() {
        let cases = [
            ("codeunit 50001 C", "OnOpenPage"),
            ("page 50001 P { PageType = Card; }", "OnInsert"),
            ("table 50001 T { fields { field(1; Id; Integer) { } } }", "OnRun"),
            ("codeunit 50001 C", "OnPreReport"),
            ("table 50001 T { fields { field(1; Id; Integer) { } } }", "OnOpenPage"),
            ("report 50001 R", "OnPreXmlPort"),
        ];
        for (head, trg) in cases {
            let head = head.strip_suffix('}').map_or(format!("{head} {{"), |h| format!("{h} "));
            let src = format!("{head} trigger {trg}() begin end; }}");
            assert_eq!(
                messages(&src),
                vec![format!(
                    "AL0162 '{trg}' is not a valid trigger for this object type. Check the object documentation for supported triggers."
                )],
                "{src}"
            );
        }
        let ok = [
            "codeunit 50001 C { trigger OnRun() begin end; }",
            // The install and upgrade triggers (corpus install/TestInstallSeeder.al).
            "codeunit 50001 C { Subtype = Install; trigger OnInstallAppPerCompany() begin end; trigger OnInstallAppPerDatabase() begin end; }",
            "codeunit 50001 C { Subtype = \"Install\"; trigger OnInstallAppPerCompany() begin end; }",
            "codeunit 50001 C { Subtype = Upgrade; trigger OnUpgradePerCompany() begin end; trigger OnValidateUpgradePerDatabase() begin end; }",
            // The triggers of a test runner codeunit (corpus events/lt-trigger-pins/PTestRunnerTriggers.al:107, :116).
            "codeunit 50001 C { Subtype = TestRunner; trigger OnBeforeTestRun(CodeunitId: Integer; CodeunitName: Text; FunctionName: Text; FunctionTestPermissions: TestPermissions): Boolean begin exit(true); end;
               trigger OnAfterTestRun(CodeunitId: Integer; CodeunitName: Text; FunctionName: Text; FunctionTestPermissions: TestPermissions; IsSuccess: Boolean) begin end; }",
            "table 50001 T { fields { field(1; Id; Integer) { } } trigger OnInsert() begin end; trigger OnModify() begin end; trigger OnDelete() begin end; trigger OnRename() begin end; }",
            "page 50001 P { PageType = Card; trigger OnOpenPage() begin end; trigger OnInit() begin end; trigger OnNewRecord(BelowxRec: Boolean) begin end; }",
            "report 50001 R { trigger OnPreReport() begin end; trigger OnPostReport() begin end; trigger OnInitReport() begin end; }",
        ];
        for src in ok {
            assert!(codes(&[src]).is_empty(), "{src}: {:?}", messages(src));
        }
    }

    // must-not-compile 0324.
    #[test]
    fn al0122_a_codeunit_that_does_not_implement_the_interface() {
        let head = "interface I { procedure Calc(A: Integer): Integer; } codeunit 50001 C { }";
        let user = "codeunit 50002 D { procedure Run1() var I: Interface I; C: Codeunit C; begin I := C; end; }";
        assert_eq!(
            messages(&format!("{head} {user}")),
            vec!["AL0122 Cannot implicitly convert type 'Codeunit \"C\"' to 'Interface \"I\"'"]
        );
        let ok = "interface I { procedure Calc(A: Integer): Integer; } interface J extends I { } \
            codeunit 50001 C implements I { procedure Calc(A: Integer): Integer begin end; } \
            codeunit 50003 C2 implements J { procedure Calc(A: Integer): Integer begin end; } \
            codeunit 50002 D { procedure Run1() var I: Interface I; C: Codeunit C; C2: Codeunit C2; begin I := C; I := C2; end; }";
        assert!(codes(&[ok]).is_empty(), "{:?}", messages(ok));
    }

    // must-not-compile 0351 to 0355: system variables an object type does not have.
    #[test]
    fn al0118_system_variables_the_object_does_not_have() {
        let cases = [
            (
                "codeunit 50001 C { procedure Run1() begin CurrPage.Update(); end; }",
                "CurrPage",
            ),
            (
                "codeunit 50001 C { procedure Run1() begin CurrReport.Skip(); end; }",
                "CurrReport",
            ),
            ("codeunit 50001 C { procedure Run1() begin xRec.Init(); end; }", "xRec"),
            (
                "table 50001 T { fields { field(1; Id; Integer) { } } trigger OnInsert() begin CurrPage.Update(); end; }",
                "CurrPage",
            ),
            ("report 50001 R { trigger OnPreReport() begin Rec.Init(); end; }", "Rec"),
        ];
        for (src, name) in cases {
            assert_eq!(
                messages(src),
                vec![format!(
                    "AL0118 The name '{name}' does not exist in the current context."
                )],
                "{src}"
            );
        }
        let ok = [
            "table 50000 T { fields { field(1; Id; Integer) { } } } codeunit 50001 C { TableNo = T; procedure Run1() begin xRec.Init(); Rec.Init(); end; }",
            "table 50001 T { fields { field(1; Id; Integer) { } } trigger OnInsert() begin Rec.Init(); xRec.Init(); end; }",
            "report 50001 R { trigger OnPreReport() begin CurrReport.Skip(); end; }",
            "page 50001 P { trigger OnOpenPage() begin CurrPage.Update(); end; }",
        ];
        for src in ok {
            assert!(codes(&[src]).is_empty(), "{src}: {:?}", messages(src));
        }
    }

    // must-not-compile 0356 and 0358: a local procedure and a global variable of another codeunit.
    #[test]
    fn al0161_a_local_procedure_or_a_variable_of_another_codeunit() {
        let local = "codeunit 50001 C { local procedure Secret() begin end; } \
            codeunit 50002 D { procedure Run1() var C: Codeunit C; begin C.Secret(); end; }";
        assert_eq!(
            messages(local),
            vec!["AL0161 'Secret()' is inaccessible due to its protection level"]
        );
        let var = "codeunit 50001 C { var Shown: Integer; } \
            codeunit 50002 D { procedure Run1() var C: Codeunit C; I: Integer; begin I := C.Shown; end; }";
        assert_eq!(
            messages(var),
            vec!["AL0161 'Shown' is inaccessible due to its protection level"]
        );
        let ok = "codeunit 50001 C { var Shown: Integer; procedure Open() begin end; local procedure Secret() begin end; \
            procedure Self() var Me: Codeunit C; begin Me.Secret(); end; } \
            codeunit 50002 D { procedure Run1() var C: Codeunit C; begin C.Open(); C.Run(); end; }";
        assert!(codes(&[ok]).is_empty(), "{:?}", messages(ok));
    }

    // must-not-compile 0401, 0403, 0405, 0407, 0412 to 0419, 0430 to 0438, 0445, 0448: more scalar, option and enum pairs.
    #[test]
    fn al0122_enum_option_and_boolean_pairs() {
        let e = "enum 50010 E { Extensible = true; AssignmentCompatibility = true; value(0; None) { } value(1; One) { } } \
            table 50011 T { fields { field(1; Id; Integer) { } field(2; Kind; Enum E) { } } } ";
        let cases = [
            ("C: Code[20]; T: Enum E", "C := T", "Enum \"E\"", "Code[20]"),
            ("X: Text; T: Enum E", "X := T", "Enum \"E\"", "Text"),
            ("X: Text; O: Option A,B,C", "O := X", "Text", "Option"),
            ("D: Decimal; T: Enum E", "T := D", "Decimal", "Enum \"E\""),
            ("C: Code[20]; T: Enum E", "T := C", "Code[20]", "Enum \"E\""),
            ("X: Text; T: Enum E", "T := X", "Text", "Enum \"E\""),
            ("D: Decimal; R: Record T", "R.Kind := D", "Decimal", "Enum \"E\""),
            ("X: Text; R: Record T", "R.Kind := X", "Text", "Enum \"E\""),
            ("B: Boolean; T: Enum E", "T := B", "Boolean", "Enum \"E\""),
            ("D: Date; T: Enum E", "T := D", "Date", "Enum \"E\""),
            ("C: Char; T: Enum E", "T := C", "Char", "Enum \"E\""),
            ("B: Boolean; G: BigInteger", "B := G", "BigInteger", "Boolean"),
            ("B: Boolean", "B := 1", "Integer", "Boolean"),
            ("B: Boolean; I: Integer", "I := B", "Boolean", "Integer"),
            ("B: Boolean; G: BigInteger", "G := B", "Boolean", "BigInteger"),
            ("B: Boolean; O: Option A,B,C", "B := O", "Option", "Boolean"),
            ("B: Boolean; O: Option A,B,C", "O := B", "Boolean", "Option"),
            ("G: BigInteger", "if G then G := 0", "BigInteger", "Boolean"),
            ("O: Option A,B,C", "if O then O := O", "Option", "Boolean"),
            ("B: Boolean; T: Enum E", "B := T", "Enum \"E\"", "Boolean"),
        ];
        for (vars, stmt, from, to) in cases {
            let cu = format!("{e} codeunit 50001 C {{ procedure Run1() var {vars}; begin {stmt}; end; }}");
            assert_eq!(
                messages(&cu),
                vec![format!("AL0122 Cannot implicitly convert type '{from}' to '{to}'")],
                "{vars}: {stmt}"
            );
        }
        let ok = [
            "B: Boolean; V: Boolean; B := V",
            "I: Integer; O: Option A,B; I := O",
            "O: Option A,B; I: Integer; O := I",
            "T: Enum E; I: Integer; I := T.AsInteger()",
            "T: Enum E; X: Text; X := Format(T)",
            "T: Enum E; U: Enum E; T := U",
            "B: Boolean; O: Option A,B; B := O = O::A",
            "G: BigInteger; I: Integer; G := I",
            "G: BigInteger; I: Integer; I := G",
        ];
        for stmt in ok {
            let (vars, body) = stmt.rsplit_once("; ").unwrap();
            let cu = format!("{e} codeunit 50001 C {{ procedure Run1() var {vars}; begin {body}; end; }}");
            assert!(codes(&[&cu]).is_empty(), "{stmt}: {:?}", messages(&cu));
        }
        // exit(value) of an enum into a procedure whose enum does not state AssignmentCompatibility (0405).
        let exit = "enum 50020 A { Extensible = true; value(0; None) { } } enum 50021 B { Extensible = true; value(0; None) { } } \
            codeunit 50001 C { procedure Run1(): Enum B var S: Enum A; begin exit(S); end; }";
        assert_eq!(
            messages(exit),
            vec!["AL0122 Cannot implicitly convert type 'Enum \"A\"' to 'Enum \"B\"'"]
        );
    }

    // must-not-compile 0359: a local procedure of a table, called through a record.
    #[test]
    fn al0161_a_local_procedure_of_a_table_through_a_record() {
        let t = "table 50000 T { fields { field(1; Id; Integer) { } } local procedure Secret() begin end; procedure Open() begin end; \
            procedure Self() var R: Record T; begin R.Secret(); end; } ";
        let bad = format!("{t} codeunit 50001 C {{ procedure Run1() var R: Record T; begin R.Secret(); end; }}");
        assert_eq!(
            messages(&bad),
            vec!["AL0161 'Secret()' is inaccessible due to its protection level"]
        );
        let ok = format!("{t} codeunit 50001 C {{ procedure Run1() var R: Record T; begin R.Open(); end; }}");
        assert!(codes(&[&ok]).is_empty(), "{:?}", messages(&ok));
    }

    // must-not-compile 0361 to 0363: File.Exists, File.Erase and File.Copy have scope OnPrem.
    #[test]
    fn al0296_static_file_methods_in_a_cloud_app() {
        for call in ["B := File.Exists('x')", "File.Erase('x')", "File.Copy('a', 'b')"] {
            let src = format!("codeunit 50000 C {{ procedure P() var B: Boolean; begin {call}; end; }}");
            assert_eq!(ctx_codes(&src, &cloud()), vec!["AL0296"], "{call}");
            let onprem = AppContext {
                target: AppTarget::OnPrem,
                ..cloud()
            };
            assert!(ctx_codes(&src, &onprem).is_empty(), "{call}");
        }
        let var = "codeunit 50000 C { procedure P() var File: Codeunit D; begin File.Exists('x'); end; } codeunit 50001 D { procedure Exists(S: Text) begin end; }";
        assert!(ctx_codes(var, &cloud()).is_empty());
    }
}
