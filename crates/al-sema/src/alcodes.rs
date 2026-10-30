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
//! The codes the checker itself reports (`check/`, where types are known) are AL0118, AL0122, AL0135, AL0139,
//! AL0132, AL0161 and AL0398 (and AL0175 for Char - Text, AL0172 for unary minus on an Enum, Option or Duration); they are listed in [`IMPLEMENTED`] too.

use crate::workspace::{AppKind, Diagnostic, ObjKind, Resolution, Workspace};
use al_syntax::{SyntaxKind, SyntaxNode, ast};

/// The AL codes this front end reports, with what each one covers.
pub const IMPLEMENTED: &[(&str, &str)] = &[
    (
        "AL0172",
        "unary minus on an Enum or Option variable or field, unary minus or plus on a Duration (check/infer.rs; cases 0584, 0585, 1531, 1532, 1728, 2268)",
    ),
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
        "a parameter named by one of 24 keywords after `var` (the first diagnostic, at the name); alc's full list for the two-codeunit keyword-parameter file (cases 2016-2039); two commas after the text of a Label declaration (case 2332)",
    ),
    (
        "AL0301",
        "a trailing comma after a property of a Label declaration, `Label 'L', Locked = true,;` (cases 2330, 2331)",
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
        "an enum assigned to an enum of another type whose target does not state AssignmentCompatibility = true; an Integer variable assigned to a Text; a text literal of two or more characters assigned to an Integer; an Enum variable, enum value or Enum field to a Decimal; Decimal or Integer to Boolean, Date to Text, Boolean, Text or Decimal to Date, Variant or a one-character literal to an Enum, a Record to an Integer",
    ),
    (
        "AL0124",
        "app.json key `assemblyProbingPaths` of a Cloud app at runtime 15.0, 16.0, 16.1, 17.0, 17.1 and 18.0; the property OrderBy on a query data item; the property AutoCalcField on an xmlport table element; a property the object type does not have (any outside the 16 codeunit properties, SourceTable and others on a table, Editable on a report, TableNo on a page, ...); InitValue on a page field",
    ),
    (
        "AL0132",
        "ExecutionMode::Install, Uninstall, Upgrade, Report, Profile; a field or method of a record that the table and the catalog lack; a method the catalog lacks on Text, Code, Boolean, List, Dictionary, JsonObject, JsonToken, HttpClient, RecordRef or TextBuilder; a method that a codeunit of an app under test does not declare, called on a Codeunit variable; a value an enum of the workspace lacks, also as a SubPageLink const(\"Enum\"::Value) (case 2276); a page field Rec.X the source table lacks",
    ),
    (
        "AL0383",
        "a SubPageLink const(Name) on an Enum field of the part's table that names no value of the enum, an enum of the workspace no extension adds to (case 2277)",
    ),
    (
        "AL0335",
        "a text attribute after a text element inside the same xmlport element (case 2253)",
    ),
    (
        "AL0161",
        "a page extension names a non-protected variable or a local procedure of its base page, or a report extension does; a Cloud app names an internal agent table of the platform; a subscriber to the internal event GetRoleCenterBannerPartID of System Action Triggers; a page customization names a variable of its page in Editable of a modify",
    ),
    (
        "AL0162",
        "OnInit, OnFindRecord and OnNextRecord in a page extension; a trigger the object type does not have (any but OnRun in a codeunit; page, table and report triggers of another object type)",
    ),
    (
        "AL0169",
        "a property set to a name that is none of its values, where the AL language server lists the values for that element kind (property_values.rs: PageType, MaskType, ObsoleteState, Style, Importance, UpdatePropagation and others; cases 0106 to 1180 and 2248)",
    ),
    ("AL0197", "two objects of one type and name in one app"),
    (
        "AL0264",
        "two objects of one type with one id in one app; a codeunit or table with the id of one in a direct dependency",
    ),
    (
        "AL0305",
        "an object name over 30 characters (codeunit, table, page, report, query, xmlport, table/page/report extension, profile) or over 20 (permission set)",
    ),
    (
        "AL0135",
        "a bare call of a procedure of the same object, the only one of its name, with fewer arguments than it has parameters",
    ),
    (
        "AL0139",
        "exit with a value in a procedure or trigger without a return value of a codeunit, table, page, report, xmlport, query, table or page extension",
    ),
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
        "AL0196",
        "reported only for a bare call, without arguments, in a page extension of its own local or internal procedure without parameters that a procedure of the base page in the same app repeats (cases 2238-2240, 2244); other ambiguous calls need overload resolution the front end does not judge.",
    ),
    (
        "AL0175",
        "needs the .NET type behind the alias (an enum is comparable with `=`, a class object is not); the front end has no assembly metadata.",
    ),
    (
        "AL0185",
        "reported only for the platform table Integer named bare by a Record variable or a report data item of a file in a namespace without `using System.Utilities;` (`platform_integer_without_using`: cases 2081, 2082), and for a variable of a codeunit type nobody declares (`unknown_codeunit_variables`: case 0213); any other missing object is not decided.",
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
    duplicate_ids(ws, &mut out);
    long_object_names(ws, &mut out);
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
        platform_integer_without_using(ws, oi, &obj, &mut out);
        unknown_codeunit_variables(ws, oi, &obj, &mut out);
        object_properties(ws, oi, &obj, &mut out);
        object_triggers(ws, oi, &obj, &mut out);
        label_trailing_commas(ws, o.file, obj.syntax(), &mut out);
        if !unparsed.contains(&(ws.files[o.file].app, ws.files[o.file].path.as_str())) && !whole.contains(&o.file) {
            parameter_names(ws, o.file, obj.syntax(), &mut out);
        }
        match o.kind {
            ObjKind::XmlPort => xmlport_elements(ws, o.file, obj.sections(), &mut out),
            ObjKind::PageExtension => {
                page_extension_triggers(ws, oi, &obj, &mut out);
                extension_procedures(ws, oi, &obj, ObjKind::Page, &mut out);
                ambiguous_bare_calls(ws, oi, &obj, &mut out);
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
    // AL0169: a property value that is none of the property's values.
    crate::property_values::check(ws, &mut out);
    out
}

/// AL0301 and AL0107 for the trailing commas of a Label declaration (al-runner issue 6436). One comma right after the text
/// compiles (text/lt-label-textbuilder/TxCLabel.al Y12_Label_TrailingComma); a trailing comma after a property is AL0301
/// (must-not-compile 2330 `Locked = true,;`, 2331 `Comment = 'c',;`), two commas after the text AL0107 (2332 `'T',,;`).
/// The diagnostic is at the comma that is wrong; the parser rejects both shapes too, so the file never runs.
fn label_trailing_commas(ws: &Workspace, file: usize, obj: &SyntaxNode, out: &mut Vec<Diagnostic>) {
    let significant = |t: &al_syntax::SyntaxToken| !t.kind().is_trivia();
    for n in obj.descendants().filter(|n| n.kind() == SyntaxKind::TypeRef) {
        let mut toks: Vec<al_syntax::SyntaxToken> = n
            .descendants_with_tokens()
            .filter_map(|e| e.into_token())
            .filter(significant)
            .collect();
        if !toks
            .first()
            .is_some_and(|t| t.kind() == SyntaxKind::Ident && t.text().eq_ignore_ascii_case("label"))
        {
            continue;
        }
        // The `;` that ends the declaration follows the type.
        let mut next = n.last_token().and_then(|t| t.next_token());
        while let Some(t) = next.clone().filter(|t| t.kind().is_trivia()) {
            next = t.next_token();
        }
        let Some(end) = next.filter(|t| t.kind() == SyntaxKind::Semicolon) else {
            continue;
        };
        toks.push(end);
        if let Some(w) = toks
            .windows(2)
            .find(|w| w[0].kind() == SyntaxKind::Comma && w[1].kind() == SyntaxKind::Comma)
        {
            push(
                out,
                ws,
                file,
                w[1].text_range().start().into(),
                "AL0107",
                "Syntax error, identifier expected.".into(),
            );
            continue;
        }
        let has_property = n.children().any(|c| c.kind() == SyntaxKind::Property);
        if let [.., comma, _] = toks.as_slice()
            && comma.kind() == SyntaxKind::Comma
            && has_property
        {
            push(
                out,
                ws,
                file,
                comma.text_range().start().into(),
                "AL0301",
                "A list must end with a member; not a separator ,.".into(),
            );
        }
    }
}

/// AL0185 `Table 'Integer' is missing`. The platform table Integer lives in namespace System.Utilities: a file in a namespace
/// names it bare only through `using System.Utilities;` (must-not-compile 2081: a Record variable, 2082: a report data item;
/// misc/lt-inferred-behaviors/RiNsPlatformTable.al:19 compiles with the using, al-runner issue 5694). Only those two shapes and
/// only Integer are pinned: other platform tables, other places that name a table, a qualified name and a file whose namespace
/// is System.Utilities or below it keep compiling. A table Integer the file finds in its own namespace, a using or among the
/// objects without a namespace is no error.
fn platform_integer_without_using(ws: &Workspace, oi: usize, obj: &ast::Object, out: &mut Vec<Diagnostic>) {
    const NS: &str = "system.utilities";
    let file = ws.objects[oi].file;
    let f = &ws.files[file];
    let Some(ns) = &f.namespace else {
        return;
    };
    let ns = ns.to_ascii_lowercase();
    if ns == NS || ns.starts_with("system.utilities.") || f.usings.iter().any(|u| u.eq_ignore_ascii_case(NS)) {
        return;
    }
    let bare_integer = |parts: &[String]| matches!(parts, [p] if p.eq_ignore_ascii_case("Integer"));
    let mut at: Vec<usize> = obj
        .syntax()
        .descendants()
        .filter_map(ast::TypeRef::cast)
        .filter(|t| t.type_name().eq_ignore_ascii_case("Record") && bare_integer(&t.object_name_parts()))
        .map(|t| offset_of(t.syntax()))
        .collect();
    if ws.objects[oi].kind == ObjKind::Report {
        fn dataitems(
            sections: impl Iterator<Item = ast::Section>,
            hit: &dyn Fn(&[String]) -> bool,
            at: &mut Vec<usize>,
        ) {
            for s in sections {
                if s.name().eq_ignore_ascii_case("dataitem") && s.arg_name_parts(1).is_some_and(|p| hit(&p)) {
                    at.push(offset_of(s.syntax()));
                }
                dataitems(s.sections(), hit, at);
            }
        }
        dataitems(obj.sections(), &bare_integer, &mut at);
    }
    if at.is_empty() || ws.table_in_scope_tiers(file, "Integer") {
        return;
    }
    for offset in at {
        push(
            out,
            ws,
            file,
            offset,
            "AL0185",
            "Table 'Integer' is missing".to_string(),
        );
    }
}

/// AL0185 `Codeunit 'X' is missing`: a variable declared with a codeunit type nobody declares (must-not-compile
/// 0213-unknown-codeunit-var.al:7, a local variable). Only a variable declaration with a one-segment name is judged; a
/// parameter, a return type, an id and a namespace-qualified name keep compiling (not pinned). `names_object` keeps the
/// name when an app the file sees, or the platform app, is not loaded.
fn unknown_codeunit_variables(ws: &Workspace, oi: usize, obj: &ast::Object, out: &mut Vec<Diagnostic>) {
    let file = ws.objects[oi].file;
    for t in obj
        .syntax()
        .descendants()
        .filter_map(ast::VarDecl::cast)
        .filter_map(|d| d.type_ref())
    {
        if !t.type_name().eq_ignore_ascii_case("Codeunit") {
            continue;
        }
        let parts = t.object_name_parts();
        if parts.len() != 1 || parts[0].parse::<u32>().is_ok() || ws.names_object(file, ObjKind::Codeunit, &parts) {
            continue;
        }
        push(
            out,
            ws,
            file,
            offset_of(t.syntax()),
            "AL0185",
            format!("Codeunit '{}' is missing", parts[0]),
        );
    }
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
/// AL0335. A text attribute after a text element inside the same element (case 2253).
fn xmlport_elements(
    ws: &Workspace,
    file: usize,
    sections: impl Iterator<Item = ast::Section>,
    out: &mut Vec<Diagnostic>,
) {
    for s in sections {
        // AL0335. A text attribute after a text element inside the same element (corpus must-not-compile 2253 on branch
        // al-runtime-rs-staging-pub-tp). Other attribute and element kinds in that order are not pinned and not judged.
        let mut seen_text_element = false;
        for c in s.sections() {
            if c.name().eq_ignore_ascii_case("textelement") {
                seen_text_element = true;
            } else if seen_text_element && c.name().eq_ignore_ascii_case("textattribute") {
                push(
                    out,
                    ws,
                    file,
                    offset_of(c.syntax()),
                    "AL0335",
                    "Attributes must be specified before elements inside of an element.".to_string(),
                );
            }
        }
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

/// AL0264: one diagnostic for every declaration of an id that an app declares more than once for the same object type
/// (must-not-compile 0036-two-codeunits-same-id.al:1-2, 0296-duplicate-table-id.al:1-2, and for every other type with
/// an id 2136-2146 (page, report, query, xmlport, enum, table/page/enum/report extension, permission set and
/// permission set extension), al-runner issues 5854, 5588). The text is alc's up to the name of the extension, which
/// names the app's publisher and version: the workspace has neither.
///
/// An object with the type and id of an object of a direct dependency is AL0264 as well, on this app's declaration
/// only (2148-codeunit-id-of-dependency-app.al:1, 2149-table-id-of-dependency-app.al:1). Only codeunits and tables
/// are pinned across apps, and only for a direct dependency; other types and transitive dependencies are not judged
/// (docs/probes-needed.md).
pub(crate) fn duplicate_ids(ws: &Workspace, out: &mut Vec<Diagnostic>) {
    let report = |out: &mut Vec<Diagnostic>, oi: usize| {
        let o = &ws.objects[oi];
        push(
            out,
            ws,
            o.file,
            o.offset,
            "AL0264",
            format!(
                "An application object of type '{}' with ID '{}' is already declared by the extension",
                display_kind(o.kind),
                o.id.unwrap_or_default()
            ),
        );
    };
    let dups = duplicate_id_objects(ws);
    for &oi in &dups {
        if ws.objects[oi].kind != ObjKind::Other {
            report(out, oi);
        }
    }
    let mut by_id: std::collections::HashSet<(usize, ObjKind, u32)> = Default::default();
    for o in &ws.objects {
        if let (Some(id), ObjKind::Codeunit | ObjKind::Table) = (o.id, o.kind) {
            by_id.insert((o.app, o.kind, id));
        }
    }
    for (oi, o) in ws.objects.iter().enumerate() {
        if ws.kinds[o.app] == AppKind::Dependency || !matches!(o.kind, ObjKind::Codeunit | ObjKind::Table) {
            continue;
        }
        let Some(id) = o.id else { continue };
        let in_dependency = ws.apps[o.app].dependencies.iter().any(|d| {
            ws.apps
                .iter()
                .enumerate()
                .any(|(ai, a)| ai != o.app && a.id.eq_ignore_ascii_case(d) && by_id.contains(&(ai, o.kind, id)))
        });
        if in_dependency && !dups.contains(&oi) {
            report(out, oi);
        }
    }
}

/// The objects of the workspace apps whose type and id another object of the same app also has, in object order.
pub(crate) fn duplicate_id_objects(ws: &Workspace) -> Vec<usize> {
    let mut seen: std::collections::HashMap<(usize, ObjKind, u32), Vec<usize>> = Default::default();
    for (oi, o) in ws.objects.iter().enumerate() {
        if ws.kinds[o.app] == AppKind::Dependency {
            continue;
        }
        if let Some(id) = o.id {
            seen.entry((o.app, o.kind, id)).or_default().push(oi);
        }
    }
    let mut dups: Vec<usize> = seen.into_values().filter(|v| v.len() > 1).flatten().collect();
    dups.sort_unstable();
    dups
}

/// The longest object name the AL compiler accepts, from alc's text (must-not-compile 0055-object-name-too-long.al:1,
/// 2120-codeunit-name-31-characters.al:1).
const AL0305_LIMIT: usize = 30;
/// The longest permission set name alc accepts (2131-permissionset-name-21-characters.al:1: 21 is refused, 20 compiles).
const AL0305_PERMISSION_SET_LIMIT: usize = 20;

/// The name limit AL0305 applies to an object type, where a case pins it: codeunits (0055, 2115, 2120, 2121), tables
/// (2122), pages (2123), reports (2124), queries (2125), xmlports (2126), table, page and report extensions
/// (2127-2129), profiles (2130) and permission sets (2131). Enums have no such limit (2132's comment, and
/// al-language/misc/lt-cv-odd-leads/CvEnumLongNames.al compiles a name of 38). Enum extensions, permission set
/// extensions and interfaces are not pinned and are not judged (docs/probes-needed.md).
fn name_limit(kind: ObjKind) -> Option<usize> {
    match kind {
        ObjKind::Codeunit
        | ObjKind::Table
        | ObjKind::Page
        | ObjKind::Report
        | ObjKind::Query
        | ObjKind::XmlPort
        | ObjKind::TableExtension
        | ObjKind::PageExtension
        | ObjKind::ReportExtension
        | ObjKind::Profile => Some(AL0305_LIMIT),
        ObjKind::PermissionSet => Some(AL0305_PERMISSION_SET_LIMIT),
        _ => None,
    }
}

/// AL0305: an object name longer than its type allows (al-runner issues 5845, 5588; see `name_limit`).
fn long_object_names(ws: &Workspace, out: &mut Vec<Diagnostic>) {
    for o in &ws.objects {
        if ws.kinds[o.app] == AppKind::Dependency {
            continue;
        }
        let Some(limit) = name_limit(o.kind) else { continue };
        if o.name.chars().count() > limit {
            push(
                out,
                ws,
                o.file,
                o.offset,
                "AL0305",
                format!(
                    "The length of the application object identifier '{}' cannot exceed {limit} characters.",
                    o.name
                ),
            );
        }
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
/// `TestHandlers` compiles on BC 29 (corpus al-language-testlibraries/lt-bi-test-framework/BiTestHandler.al:98); on BC 27 the
/// compiler answers AL0666 and AL0928 for it, not AL0124 (must-not-compile 0784), which this list does not judge either.
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
    "TestHandlers",
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

/// What alc does with a procedure `ext` of a page or report extension that has the name and parameter types of a
/// procedure `base` of the object it extends (`base` not `local`; a local base procedure is out of the extension's
/// reach). Probed with alc for every access pair, page and report, the same app and another app (al-runner issue 6043).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Twin {
    /// `ext` is `local`: it compiles; code outside the extension cannot reach it.
    Local,
    /// Error AL0440: both public or protected, in the same app (page and report) or a report extension of another app
    /// (corpus must-not-compile 0003 and 0042, handlers/TestPageExtensionBaseAccessContracts.al:20).
    Al0440,
    /// alc compiles both: a page extension of another app with both public or protected (warning AL0523 only), or
    /// `internal` on either side (same app or another app; in the same app a bare call of the name in the extension
    /// is AL0196). Which body a call runs is not pinned.
    Compiles,
}

pub(crate) fn twin(ext: &ast::Routine, base: &ast::Routine, same_app: bool, report: bool) -> Twin {
    if ext.is_local() {
        Twin::Local
    } else if ext.is_internal() || base.is_internal() {
        Twin::Compiles
    } else if same_app || report {
        Twin::Al0440
    } else {
        Twin::Compiles
    }
}

/// AL0440. A page or report extension procedure with the name and parameter types of a procedure of its base object,
/// where alc rejects it ([`Twin::Al0440`]).
fn extension_procedures(ws: &Workspace, oi: usize, obj: &ast::Object, base_kind: ObjKind, out: &mut Vec<Diagnostic>) {
    let Some(base) = ws.extension_target(oi) else { return };
    if ws.objects[base].kind != base_kind {
        return;
    }
    let base_ast = ws.object_ast(base);
    let same_app = ws.objects[oi].app == ws.objects[base].app;
    for r in obj.routines() {
        let name = r.name();
        let sig = signature(&r);
        if crate::base_access::visible_procs(&base_ast, &name)
            .any(|b| signature(&b) == sig && twin(&r, &b, same_app, base_kind == ObjKind::Report) == Twin::Al0440)
        {
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

/// AL0196. In one app, a page extension declares a `local` or `internal` procedure with the name and parameter types
/// of a procedure of its base page (which compiles) and calls it by its bare name: the call is ambiguous (must-not-compile
/// 2238 to 2240 and 2244 on corpus branch al-runtime-rs-staging-pub-ti, al-runner issue 6043). Only the pinned shape
/// is judged: a procedure without parameters, called with no arguments. Other arities are refused when lowered.
fn ambiguous_bare_calls(ws: &Workspace, oi: usize, obj: &ast::Object, out: &mut Vec<Diagnostic>) {
    let Some(base) = ws.extension_target(oi) else { return };
    if ws.objects[base].kind != ObjKind::Page || ws.objects[oi].app != ws.objects[base].app {
        return;
    }
    let base_ast = ws.object_ast(base);
    let ambiguous: Vec<String> = obj
        .routines()
        .filter(|r| (r.is_local() || r.is_internal()) && signature(r).is_empty())
        .filter(|r| {
            crate::base_access::visible_procs(&base_ast, &r.name())
                .any(|b| signature(&b).is_empty() && twin(r, &b, true, false) != Twin::Al0440)
        })
        .map(|r| r.name())
        .collect();
    if ambiguous.is_empty() {
        return;
    }
    let app = &ws.apps[ws.objects[oi].app].name;
    for call in obj.syntax().descendants().filter(|n| n.kind() == SyntaxKind::CallExpr) {
        let Some(callee) = call.children().next() else { continue };
        if callee.kind() != SyntaxKind::NameRef || call.children().skip(1).any(|a| a.children().next().is_some()) {
            continue;
        }
        let name = callee.text().to_string().trim().to_string();
        let Some(n) = ambiguous.iter().find(|n| n.eq_ignore_ascii_case(&name)) else {
            continue;
        };
        push(
            out,
            ws,
            ws.objects[oi].file,
            offset_of(&call),
            "AL0196",
            format!(
                "The call is ambiguous between the method '{n}()' defined in PageExtension '{}' by the extension '{app}' and the method '{n}()' defined in Page '{}' by the extension '{app}'.",
                ws.objects[oi].name, ws.objects[base].name
            ),
        );
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

    // Must-not-compile case 2253 (corpus branch al-runtime-rs-staging-pub-tp): a text attribute after a text element.
    #[test]
    fn a_text_attribute_after_a_text_element_is_al0335() {
        let bad = "xmlport 50000 X { schema { textelement(root) { textelement(entry) { textelement(inner) { } textattribute(kind) { } } } } }";
        assert_eq!(codes(&[bad]), vec!["AL0335".to_string()]);
        let good = "xmlport 50000 X { schema { textelement(root) { textelement(entry) { textattribute(kind) { } textelement(inner) { } } } } }";
        assert!(codes(&[good]).is_empty());
    }

    // Must-not-compile cases 2330-2332 (corpus branch al-runtime-rs-staging-y12, al-runner issue 6436): a trailing
    // comma after a Label property is AL0301, two commas after the text AL0107; one comma after the text compiles
    // (text/lt-label-textbuilder/TxCLabel.al Y12_Label_TrailingComma).
    #[test]
    fn label_trailing_commas_al0301_and_al0107() {
        let cu = |decl: &str| format!("codeunit 50000 \"MNC Codeunit\"\n{{\n    var\n        SetupLbl: {decl};\n}}\n");
        assert_eq!(codes(&[&cu("Label 'L', Locked = true,")]), vec!["AL0301".to_string()]);
        assert_eq!(codes(&[&cu("Label 'C', Comment = 'c',")]), vec!["AL0301".to_string()]);
        assert_eq!(codes(&[&cu("Label 'T',,")]), vec!["AL0107".to_string()]);
        assert!(codes(&[&cu("Label 'Document Setup',")]).is_empty());
        assert!(codes(&[&cu("Label 'x' ,")]).is_empty());
        assert!(codes(&[&cu("Label 'x', Locked = true")]).is_empty());
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

    /// The platform table Integer lives in namespace System.Utilities: code in a namespace names it bare only through
    /// `using System.Utilities;` (must-not-compile 2081, 2082: AL0185 `Table 'Integer' is missing`; misc/lt-inferred-behaviors/
    /// RiNsPlatformTable.al:19 compiles with the using; al-runner issue 5694).
    #[test]
    fn al0185_integer_from_a_namespace_without_its_using() {
        const PLATFORM: &str =
            "namespace System.Utilities; table 2000000026 Integer { fields { field(1; Number; Integer) { } } }";
        let rec = |head: &str| format!("{head} codeunit 50100 C {{ procedure F() var N: Record Integer; begin end; }}");
        let dataitem = |head: &str| {
            format!("{head} report 50101 R {{ ProcessingOnly = true; dataset {{ dataitem(Number; Integer) {{ }} }} }}")
        };
        for platform in [&[PLATFORM][..], &[][..]] {
            let with = |src: String| {
                let mut f: Vec<&str> = platform.to_vec();
                f.push(Box::leak(src.into_boxed_str()));
                codes(&f)
            };
            assert_eq!(with(rec("namespace Repro;")), vec!["AL0185"]);
            assert_eq!(with(dataitem("namespace Repro;")), vec!["AL0185"]);
            assert!(with(rec("namespace Repro; using System.Utilities;")).is_empty());
            assert!(with(dataitem("namespace Repro; using System.Utilities;")).is_empty());
            assert!(with(rec("")).is_empty());
            assert!(with(dataitem("")).is_empty());
        }
        let ws = load(&[PLATFORM, &rec("namespace Repro;")]);
        let r = crate::check::check(&ws, &crate::members::build(&ws), &Catalog::load());
        let d = r.diagnostics.iter().find(|d| d.code == "AL0185").expect("AL0185");
        assert_eq!(d.message, "Table 'Integer' is missing");
        // A table Integer of the file's own namespace is found.
        let own = "namespace Repro; table 50102 Integer { fields { field(1; Number; Integer) { } } }";
        assert!(codes(&[PLATFORM, own, &rec("namespace Repro;")]).is_empty());
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
        // al-runner issue 6043: a local procedure of the extension is not reachable from outside it, so it does not
        // clash with a protected or public base procedure (alc compiles it, same app or another app).
        assert!(codes(&[BASE, &ext("local procedure Bump(): Integer begin exit(2); end;")]).is_empty());
        assert!(
            codes(&[
                BASE,
                &ext("local procedure Plus(a: Integer): Integer begin exit(2); end;")
            ])
            .is_empty()
        );
        // A protected extension procedure still clashes (alc: AL0440 in the same app).
        assert_eq!(
            codes(&[BASE, &ext("protected procedure Bump(): Integer begin exit(2); end;")]),
            vec!["AL0440"]
        );
        // An internal extension procedure compiles next to a protected base procedure in the same app (alc: only a bare
        // call of it in the extension is AL0196). Another app and the other access pairs: al-test-cli
        // tests/user_reports.rs, issue_6043_extension_procedure_named_like_a_base_procedure_compiles_as_alc_does_and_its_calls_refuse.
        assert!(codes(&[BASE, &ext("internal procedure Bump(): Integer begin exit(2); end;")]).is_empty());
    }

    /// Must-not-compile 2238 to 2240 and 2244 (corpus branch al-runtime-rs-staging-pub-ti): in one app, a bare call of a
    /// local or internal extension procedure that a base page procedure repeats is ambiguous (AL0196).
    #[test]
    fn al0196_bare_call_of_an_extension_procedure_the_base_page_repeats() {
        let call = |decl: &str| {
            ext(&format!(
                "{decl} trigger OnOpenPage() var i: Integer; begin i := Bump(); end;"
            ))
        };
        assert_eq!(
            codes(&[BASE, &call("local procedure Bump(): Integer begin exit(2); end;")]),
            vec!["AL0196"]
        );
        assert_eq!(
            codes(&[BASE, &call("internal procedure Bump(): Integer begin exit(2); end;")]),
            vec!["AL0196"]
        );
        // Declared and not called: it compiles.
        assert!(codes(&[BASE, &ext("local procedure Bump(): Integer begin exit(2); end;")]).is_empty());
        // A call on the page from elsewhere is not the bare call.
        assert!(codes(&[BASE, &ext("local procedure Other(): Integer begin exit(2); end;")]).is_empty());
    }

    /// al-runner issue 6012: a page and its extension each get a generated `__visible_N` routine for a `Visible = Flag;`
    /// property, named from the property's offset in its own object. When both offsets are equal the names are equal, but
    /// the routines are not AL methods: alc compiles the app, so neither AL0440 nor any other AL code is reported. The
    /// offsets must not depend on the line endings either (the reporter's file failed with CRLF and compiled with LF).
    #[test]
    fn al0440_not_reported_for_generated_control_routines_of_a_page_and_its_extension() {
        let table = "table 50000 T { fields { field(1; Id; Integer) { } } }";
        let page = |nl: &str, pad: usize| {
            format!(
                "page 50001 BP{nl}{{{nl}PageType = Card; SourceTable = T;{nl}layout {{ area(Content) {{{nl}{}field(Id; Rec.Id) {{ Visible = Flag; }}{nl}}} }}{nl}var Flag: Boolean;{nl}}}",
                " ".repeat(pad)
            )
        };
        let ext = |nl: &str, pad: usize| {
            format!(
                "pageextension 50002 BPE extends BP{nl}{{{nl}layout {{ addlast(Content) {{{nl}{}field(Id2; Rec.Id) {{ Visible = Flag2; }}{nl}}} }}{nl}var Flag2: Boolean;{nl}}}",
                " ".repeat(pad)
            )
        };
        let at = |s: &str| s.find("Visible").unwrap();
        for (page_nl, ext_nl) in [("\n", "\n"), ("\r\n", "\r\n"), ("\r\n", "\n"), ("\n", "\r\n")] {
            // Pad the earlier of the two so the Visible properties sit at the same offset of their objects.
            let (p0, e0) = (at(&page(page_nl, 0)), at(&ext(ext_nl, 0)));
            let (p, e) = (page(page_nl, e0.saturating_sub(p0)), ext(ext_nl, p0.saturating_sub(e0)));
            assert_eq!(at(&p), at(&e));
            let got = codes(&[table, &p, &e]);
            assert!(got.is_empty(), "{got:?} {:?}", (page_nl, ext_nl));
        }
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

    /// must-not-compile 0213-unknown-codeunit-var.al:7: a variable of a codeunit type nobody declares is AL0185
    /// `Codeunit 'X' is missing`. Judged only when every app the file sees is loaded, the platform app included.
    #[test]
    fn al0185_variable_of_an_unknown_codeunit_type() {
        let local = "codeunit 50000 C { procedure P() var X: Codeunit \"No Such\"; begin end; }";
        let global = "codeunit 50001 G { var X: Codeunit NoSuch; }";
        let other = "codeunit 2000000001 Other { }";
        assert_eq!(codes_with_platform(&[local], other), vec!["AL0185"]);
        assert_eq!(codes_with_platform(&[global], other), vec!["AL0185"]);
        assert_eq!(
            messages_with_platform(&[local], other),
            vec!["AL0185 Codeunit 'No Such' is missing".to_string()]
        );
        // Found in the app itself, in the platform app, or named by id: no error.
        let own = "codeunit 50002 \"No Such\" { }";
        assert!(codes_with_platform(&[local, own], other).is_empty());
        assert!(codes_with_platform(&[local], "codeunit 2000000002 \"No Such\" { }").is_empty());
        let by_id = "codeunit 50003 I { procedure P() var X: Codeunit 50002; begin end; }";
        assert!(codes_with_platform(&[by_id, own], other).is_empty());
        // Without the platform app, or with a dependency that is not loaded, the name may exist.
        assert!(codes(&[local]).is_empty());
        assert!(codes_with_dependency(&[local], None).is_empty());
        assert_eq!(codes_with_dependency(&[local], Some(other)), vec!["AL0185"]);
        assert!(codes_with_dependency(&[local], Some("codeunit 60000 \"No Such\" { }")).is_empty());
    }

    /// A real app states an `application` floor, so the runner gives it the `Application` aggregate as a dependency next
    /// to the apps the aggregate propagates (`al_deps::load::Loaded::implied_dependencies`). The aggregate has no source
    /// and is never loaded; it declares no objects, so it hides no name once Base Application, whose dependencies are
    /// the rest of the aggregate, is loaded. Without Base Application the name stays undecided.
    #[test]
    fn al0185_is_judged_when_only_the_application_aggregate_is_not_loaded() {
        let local = "codeunit 50000 C { procedure P() var X: Codeunit \"No Such\"; begin end; }";
        let base = crate::workspace::BASE_APPLICATION_APP_ID;
        let aggregate = crate::workspace::APPLICATION_AGGREGATE_APP_ID;
        let run = |deps: Vec<&str>, with_base: bool| -> Vec<String> {
            let file = |s: &str| vec![("f.al".to_string(), s.to_string())];
            let mut apps = vec![AppSource {
                id: "a".into(),
                name: "a".into(),
                dependencies: deps.iter().map(|d| d.to_string()).collect(),
                files: file(local),
            }];
            let mut kinds = vec![crate::workspace::AppKind::Workspace];
            if with_base {
                apps.push(AppSource {
                    id: base.into(),
                    name: "Base Application".into(),
                    dependencies: vec![],
                    files: file("codeunit 80 \"Sales-Post\" { }"),
                });
                kinds.push(crate::workspace::AppKind::Dependency);
            }
            apps.push(AppSource {
                id: crate::workspace::PLATFORM_APP_ID.into(),
                name: "System".into(),
                dependencies: vec![],
                files: file("codeunit 2000000001 Other { }"),
            });
            kinds.push(crate::workspace::AppKind::Dependency);
            let ws = Workspace::load_apps(apps, kinds, &Default::default());
            let r = crate::check::check(&ws, &crate::members::build(&ws), &Catalog::load());
            r.diagnostics
                .iter()
                .filter(|d| d.code.starts_with("AL"))
                .map(|d| d.code.to_string())
                .collect()
        };
        assert_eq!(run(vec![aggregate, base], true), vec!["AL0185"]);
        // Base Application not loaded: the aggregate may hold the name.
        assert!(run(vec![aggregate], false).is_empty());
        // Another dependency that is not loaded still keeps the name.
        assert!(run(vec![aggregate, base, "other"], true).is_empty());
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

    // must-not-compile 2291 (corpus PR 808 at branch commit 910da83e; al-runner issue 6215): a codeunit true branch and an
    // interface false branch is AL0122; the other order compiles (types/lt-up-ternary/UpTernary.al:60, :70, :81).
    #[test]
    fn al0122_ternary_codeunit_true_branch_interface_false_branch() {
        let src = |ternary: &str| {
            format!(
                "interface \"MNC UP Preview\" {{ procedure GetPreview(): Text; }}
                 codeunit 50001 \"MNC UP Default\" implements \"MNC UP Preview\" {{ procedure GetPreview(): Text begin exit('d'); end; }}
                 codeunit 50002 \"MNC UP Factory\" {{
                     procedure Get(UseDefault: Boolean; Other: Interface \"MNC UP Preview\"): Interface \"MNC UP Preview\"
                     var Default: Codeunit \"MNC UP Default\";
                     begin exit({ternary}); end; }}"
            )
        };
        assert_eq!(
            messages(&src("UseDefault ? Default : Other")),
            vec![
                "AL0122 Cannot implicitly convert type 'Interface \"MNC UP Preview\"' to 'Codeunit \"MNC UP Default\"'"
                    .to_string()
            ]
        );
        assert!(messages(&src("UseDefault ? Other : Default")).is_empty());
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

    // must-not-compile 1250: `I := T`, `C := I` into a Code[20] and `T := I + 1` are AL0122. `exit(1 + 2)` in a procedure
    // returning Text is the third shape through exit (al-runner issue 5588, comment of 2026-10-24).
    #[test]
    fn al0122_integer_text_code_assignments() {
        let cu = |body: &str| {
            format!(
                "codeunit 50000 \"MNC BB Codeunit\" {{ procedure DoIt() var I: Integer; T: Text; C: Code[20]; begin {body} end;
                   procedure Name(): Text begin exit(1 + 2); end; }}"
            )
        };
        let only = |body: &str| -> Vec<String> {
            messages(&cu(body))
                .into_iter()
                .filter(|m| m.starts_with("AL0122"))
                .collect()
        };
        let exit_msg = "AL0122 Cannot implicitly convert type 'Integer' to 'Text'".to_string();
        assert_eq!(
            only("I := T;"),
            vec![
                "AL0122 Cannot implicitly convert type 'Text' to 'Integer'".to_string(),
                exit_msg.clone()
            ]
        );
        assert_eq!(
            only("C := I;"),
            vec![
                "AL0122 Cannot implicitly convert type 'Integer' to 'Code[20]'".to_string(),
                exit_msg.clone()
            ]
        );
        assert_eq!(only("T := I + 1;"), vec![exit_msg.clone(), exit_msg.clone()]);
        for ok in [
            "T := Format(I + 1);",
            "I := I + 1;",
            "C := Format(I);",
            "Evaluate(I, T);",
        ] {
            assert_eq!(only(ok), vec![exit_msg.clone()], "{ok}");
        }
    }

    // must-not-compile 1190 (Boolean into Text), 1191 (Boolean into Code[10]), 1193 (Date into DateTime), 1251 (a text
    // literal of two characters into a BigInteger and a Decimal) and 1833 (BigText into Text): AL0122.
    #[test]
    fn al0122_more_scalar_pairs() {
        let cu = |body: &str| {
            format!(
                "codeunit 50000 \"MNC C\" {{ procedure DoIt() var B: Boolean; T: Text; C: Code[10]; D: Date; DT: DateTime;
                   BI: BigInteger; Dec: Decimal; BT: BigText; begin {body} end; }}"
            )
        };
        let only = |body: &str| -> Vec<String> {
            messages(&cu(body))
                .into_iter()
                .filter(|m| m.starts_with("AL0122"))
                .collect()
        };
        for (body, want) in [
            ("T := B;", vec!["Boolean' to 'Text'"]),
            ("C := B;", vec!["Boolean' to 'Code[10]'"]),
            ("DT := D;", vec!["Date' to 'DateTime'"]),
            (
                "BI := 'xy'; Dec := 'xy';",
                vec!["Text' to 'BigInteger'", "Text' to 'Decimal'"],
            ),
            ("T := BT;", vec!["BigText' to 'Text'"]),
        ] {
            let want: Vec<String> = want
                .iter()
                .map(|w| format!("AL0122 Cannot implicitly convert type '{w}"))
                .collect();
            assert_eq!(only(body), want, "{body}");
        }
        for ok in [
            "T := Format(B);",
            "DT := CreateDateTime(D, 0T);",
            "BI := 'x';",
            "BT.GetSubText(T, 1);",
            "B := true;",
        ] {
            assert!(only(ok).is_empty(), "{ok}");
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

    // Writer TL's must-not-compile 2249-2251 (branch al-runtime-rs-staging-pub-tl, al-runner issue 6060): an Enum variable,
    // an enum value and an Enum field into a Decimal are AL0122; an Option into a Decimal compiles
    // (types/lt-implicit-conversions/P79OptionDecimal.al on the same branch).
    #[test]
    fn al0122_enum_into_decimal() {
        let src = |body: &str| {
            format!(
                "enum 52249 \"MNC E\" {{ value(0; A) {{ }} value(5; B) {{ }} }}
                 table 52254 \"MNC R\" {{ fields {{ field(1; Id; Integer) {{ }} field(2; Kind; Enum \"MNC E\") {{ }}
                   field(3; Amount; Decimal) {{ }} field(4; Opt; Option) {{ OptionMembers = X,Y; }} }}
                   keys {{ key(PK; Id) {{ Clustered = true; }} }} }}
                 codeunit 52250 \"MNC C\" {{ procedure DoIt() var K: Enum \"MNC E\"; D: Decimal; O: Option P,Q; S: Record \"MNC R\";
                   T: Record \"MNC R\" temporary; begin {body} end; }}"
            )
        };
        let want = "AL0122 Cannot implicitly convert type 'Enum \"MNC E\"' to 'Decimal'".to_string();
        for bad in ["D := K;", "D := \"MNC E\"::B;", "T.Amount := S.Kind;"] {
            assert_eq!(messages(&src(bad)), vec![want.clone()], "{bad}");
        }
        for ok in ["D := O;", "T.Amount := S.Opt;", "D := K.AsInteger();"] {
            assert!(messages(&src(ok)).is_empty(), "{ok}");
        }
    }

    // Writer TJ's must-not-compile 2248 (branch al-runtime-rs-staging-pub-tj): ReadState = NoLocks is AL0169; the
    // values ReadUncommitted, ReadCommitted and ReadShared compile.
    #[test]
    fn al0169_query_read_state_value() {
        let src = |v: &str| {
            format!(
                "table 50000 \"MNC T\" {{ fields {{ field(1; Id; Integer) {{ }} }} keys {{ key(PK; Id) {{ Clustered = true; }} }} }}
                 query 50001 \"MNC Q\" {{ QueryType = Normal; ReadState = {v}; elements {{ dataitem(T; \"MNC T\") {{ column(Id; Id) {{ }} }} }} }}"
            )
        };
        assert_eq!(
            messages(&src("NoLocks")),
            vec![
                "AL0169 The option value 'NoLocks' is not valid. Check the enum definition for valid values."
                    .to_string()
            ]
        );
        for ok in ["ReadUncommitted", "ReadCommitted", "ReadShared"] {
            assert!(messages(&src(ok)).is_empty(), "{ok}");
        }
    }

    // must-not-compile 0036 (two codeunits) and 0296 (two tables): AL0264 for every declaration of an id one app
    // declares twice for one object type (al-runner issue 5854).
    #[test]
    fn al0264_every_declaration_of_a_repeated_id_is_reported() {
        let want = |kind: &str, id: u32| {
            format!("AL0264 An application object of type '{kind}' with ID '{id}' is already declared by the extension")
        };
        assert_eq!(
            messages("codeunit 50000 \"MNC First\" { } codeunit 50000 \"MNC Second\" { }"),
            vec![want("Codeunit", 50000), want("Codeunit", 50000)]
        );
        let table = |name: &str| {
            format!(
                "table 50109 \"{name}\" {{ fields {{ field(1; Id; Integer) {{ }} }} keys {{ key(PK; Id) {{ Clustered = true; }} }} }}"
            )
        };
        assert_eq!(
            messages(&format!("{} {}", table("MNC T296A"), table("MNC T296B"))),
            vec![want("Table", 50109), want("Table", 50109)]
        );
        // One id for objects of two types is fine.
        assert!(codes(&["codeunit 50000 A { } table 50000 B { fields { field(1; Id; Integer) { } } }"]).is_empty());
    }

    // must-not-compile 2136-2146: AL0264 for every object type with an id, not only codeunits and tables (al-runner
    // issue 5588). The type is spelled as alc spells it ('XmlPort', 'PermissionSetExtension').
    #[test]
    fn al0264_every_object_type_with_an_id() {
        let want = |kind: &str| {
            format!(
                "AL0264 An application object of type '{kind}' with ID '50001' is already declared by the extension"
            )
        };
        let base = "table 50000 \"MNC T\" { fields { field(1; Id; Integer) { } } }
                    page 50000 \"MNC P\" { SourceTable = \"MNC T\"; }
                    report 50000 \"MNC R\" { }
                    enum 50000 \"MNC E\" { value(0; A) { } }";
        for (kind, a, b) in [
            (
                "Page",
                "page 50001 \"MNC A\" { SourceTable = \"MNC T\"; }",
                "page 50001 \"MNC B\" { SourceTable = \"MNC T\"; }",
            ),
            ("Report", "report 50001 \"MNC A\" { }", "report 50001 \"MNC B\" { }"),
            (
                "Query",
                "query 50001 \"MNC A\" { elements { dataitem(T; \"MNC T\") { column(Id; Id) { } } } }",
                "query 50001 \"MNC B\" { elements { dataitem(T; \"MNC T\") { column(Id; Id) { } } } }",
            ),
            ("XmlPort", "xmlport 50001 \"MNC A\" { }", "xmlport 50001 \"MNC B\" { }"),
            (
                "Enum",
                "enum 50001 \"MNC A\" { value(0; A) { } }",
                "enum 50001 \"MNC B\" { value(0; A) { } }",
            ),
            (
                "TableExtension",
                "tableextension 50001 \"MNC A\" extends \"MNC T\" { }",
                "tableextension 50001 \"MNC B\" extends \"MNC T\" { }",
            ),
            (
                "PageExtension",
                "pageextension 50001 \"MNC A\" extends \"MNC P\" { }",
                "pageextension 50001 \"MNC B\" extends \"MNC P\" { }",
            ),
            (
                "EnumExtension",
                "enumextension 50001 \"MNC A\" extends \"MNC E\" { value(10; X) { } }",
                "enumextension 50001 \"MNC B\" extends \"MNC E\" { value(11; Y) { } }",
            ),
            (
                "ReportExtension",
                "reportextension 50001 \"MNC A\" extends \"MNC R\" { }",
                "reportextension 50001 \"MNC B\" extends \"MNC R\" { }",
            ),
            (
                "PermissionSet",
                "permissionset 50001 \"MNC A\" { }",
                "permissionset 50001 \"MNC B\" { }",
            ),
        ] {
            let got: Vec<String> = messages(&format!("{base} {a} {b}"))
                .into_iter()
                .filter(|m| m.starts_with("AL0264"))
                .collect();
            assert_eq!(got, vec![want(kind), want(kind)], "{kind}");
        }
    }

    // must-not-compile 2148 (a codeunit) and 2149 (a table): an object with the id of an object of the same type in a
    // direct dependency is AL0264, reported once, on this app's declaration. Without the dependency it compiles.
    #[test]
    fn al0264_an_id_a_dependency_declares() {
        let dep = "codeunit 61100 \"Dep C\" { }
                   table 61101 \"Dep T\" { fields { field(1; Id; Integer) { } } }";
        assert_eq!(
            codes_with_dependency(&["codeunit 61100 \"MNC Same\" { }"], Some(dep)),
            vec!["AL0264"]
        );
        assert_eq!(
            codes_with_dependency(
                &["table 61101 \"MNC Same\" { fields { field(1; Id; Integer) { } } }"],
                Some(dep)
            ),
            vec!["AL0264"]
        );
        // Another type with the id is fine, and so is the same file without the dependency loaded.
        assert!(codes_with_dependency(&["codeunit 61101 \"MNC Other\" { }"], Some(dep)).is_empty());
        assert!(codes_with_dependency(&["codeunit 61100 \"MNC Same\" { }"], None).is_empty());
    }

    // must-not-compile 2120-2131: AL0305 for a name over 30 characters of a codeunit (31 and 36 characters), table,
    // page, report, query, xmlport, table, page and report extension and profile; 20 for a permission set.
    #[test]
    fn al0305_every_pinned_object_type() {
        let name = "MNC SH Name Length Probe Object"; // 31 characters
        let want =
            format!("AL0305 The length of the application object identifier '{name}' cannot exceed 30 characters.");
        let base = "table 50000 \"MNC T\" { fields { field(1; Id; Integer) { } } }
                    page 50002 \"MNC P\" { SourceTable = \"MNC T\"; }
                    report 50002 \"MNC R\" { }";
        for obj in [
            format!("codeunit 50001 \"{name}\" {{ }}"),
            "codeunit 50001 \"MNC SH Name Length Probe Object Abcd\" { }".to_string(),
            format!("table 50001 \"{name}\" {{ fields {{ field(1; Id; Integer) {{ }} }} }}"),
            format!("page 50001 \"{name}\" {{ SourceTable = \"MNC T\"; }}"),
            format!("report 50001 \"{name}\" {{ }}"),
            format!("query 50001 \"{name}\" {{ elements {{ dataitem(T; \"MNC T\") {{ column(Id; Id) {{ }} }} }} }}"),
            format!("xmlport 50001 \"{name}\" {{ }}"),
            format!("tableextension 50001 \"{name}\" extends \"MNC T\" {{ }}"),
            format!("pageextension 50001 \"{name}\" extends \"MNC P\" {{ }}"),
            format!("reportextension 50001 \"{name}\" extends \"MNC R\" {{ }}"),
            format!("profile \"{name}\" {{ Caption = 'X'; RoleCenter = \"MNC P\"; }}"),
        ] {
            let got: Vec<String> = messages(&format!("{base} {obj}"))
                .into_iter()
                .filter(|m| m.starts_with("AL0305"))
                .collect();
            assert_eq!(got.len(), 1, "{obj}: {got:?}");
            assert!(got[0].starts_with(&want[..want.find('\'').unwrap()]), "{obj}: {got:?}");
        }
        assert_eq!(
            messages("permissionset 50001 \"MNC SH Name Length Pr\" { Assignable = true; }"),
            vec![
                "AL0305 The length of the application object identifier 'MNC SH Name Length Pr' cannot exceed 20 characters."
                    .to_string()
            ]
        );
        // 30 characters compile, a permission set of 20 compiles, and an enum has no such limit
        // (al-language/misc/lt-cv-odd-leads/CvEnumLongNames.al, 38 characters).
        for ok in [
            "table 50001 \"MNC SH Name Length Probe Objec\" { fields { field(1; Id; Integer) { } } }",
            "permissionset 50001 \"MNC SH Name Length P\" { Assignable = true; }",
            "enum 50001 \"MNC SH Enum Name That Is Rather Long Ok\" { value(0; A) { } }",
        ] {
            assert!(codes(&[ok]).iter().all(|c| c != "AL0305"), "{ok}");
        }
    }

    // must-not-compile 0055: AL0305 for an object name of 37 characters (al-runner issue 5845).
    #[test]
    fn al0305_an_object_name_too_long() {
        assert_eq!(
            messages("codeunit 50000 \"MNC Object Name That Is Much Too Long\" { }"),
            vec![
                "AL0305 The length of the application object identifier 'MNC Object Name That Is Much Too Long' cannot exceed 30 characters."
                    .to_string()
            ]
        );
        assert!(codes(&["codeunit 50000 \"Thirty Characters Name Is Fine\" { }"]).is_empty());
        // A test codeunit of 31 characters (must-not-compile 2115, al-runner issue 5845); 30 compiles
        // (codeunit/TestScopeIsolationContracts.al:1).
        assert_eq!(
            codes(&["codeunit 50000 \"MNC Test Codeunit Name Length31\" { Subtype = Test; }"]),
            vec!["AL0305"]
        );
        assert!(codes(&["codeunit 60170 \"Test Scope Isolation Contracts\" { Subtype = Test; }"]).is_empty());
    }

    // must-not-compile 0048 (a procedure) and 0348 (the OnRun trigger): AL0139 for exit(value) in a codeunit routine
    // without a return value, also where nothing calls it (al-runner issue 5852).
    #[test]
    fn al0139_exit_with_a_value_without_a_return_value() {
        assert_eq!(
            messages("codeunit 50001 \"MNC Codeunit\" { procedure DoIt() begin exit(1); end; }"),
            vec!["AL0139 Since 'DoIt' doesn't have a return value, EXIT cannot be called with a value".to_string()]
        );
        assert_eq!(
            messages("codeunit 50166 \"MNC C348\" { trigger OnRun() begin exit(1); end; }"),
            vec!["AL0139 Since 'OnRun' doesn't have a return value, EXIT cannot be called with a value".to_string()]
        );
        // Inside a statement, as in the issue's `Bump`.
        assert_eq!(
            codes(&["codeunit 50001 C { procedure Bump() var I: Integer; begin if I = 0 then exit(5); end; }"]),
            vec!["AL0139"]
        );
        for ok in [
            "codeunit 50001 C { procedure DoIt() begin exit; end; }",
            "codeunit 50001 C { procedure DoIt(): Integer begin exit(1); end; }",
            "codeunit 50001 C { procedure DoIt() R: Integer begin exit(1); end; }",
        ] {
            assert!(codes(&[ok]).is_empty(), "{ok}");
        }
    }

    // must-not-compile 2150-2165: AL0139 in the triggers and procedures of tables, fields, pages, actions, reports, data
    // items, xmlports, queries, table and page extensions and in an event subscriber (al-runner issue 5588).
    #[test]
    fn al0139_in_every_pinned_object_type() {
        let t =
            "table 50000 \"MNC T\" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { Clustered = true; } } }";
        let p = "page 50001 \"MNC P\" { SourceTable = \"MNC T\"; }";
        for (name, src) in [
            ("OnInsert", "table 50009 \"MNC X\" { fields { field(1; Id; Integer) { } } trigger OnInsert() begin exit(1); end; }".to_string()),
            ("OnValidate", "table 50009 \"MNC X\" { fields { field(1; Id; Integer) { trigger OnValidate() begin exit(true); end; } } }".to_string()),
            ("DoIt", "table 50009 \"MNC X\" { fields { field(1; Id; Integer) { } } procedure DoIt() begin exit('x'); end; }".to_string()),
            ("OnOpenPage", "page 50009 \"MNC X\" { SourceTable = \"MNC T\"; trigger OnOpenPage() begin exit(1); end; }".to_string()),
            ("DoIt", "page 50009 \"MNC X\" { SourceTable = \"MNC T\"; local procedure DoIt() begin exit(1); end; }".to_string()),
            ("OnAction", "page 50009 \"MNC X\" { SourceTable = \"MNC T\"; actions { area(Processing) { action(Go) { trigger OnAction() begin exit(1); end; } } } }".to_string()),
            ("OnPreReport", "report 50009 \"MNC X\" { ProcessingOnly = true; trigger OnPreReport() begin exit(1); end; }".to_string()),
            ("OnAfterGetRecord", "report 50009 \"MNC X\" { ProcessingOnly = true; dataset { dataitem(B; \"MNC T\") { trigger OnAfterGetRecord() begin exit(1); end; } } }".to_string()),
            ("DoIt", "report 50009 \"MNC X\" { ProcessingOnly = true; procedure DoIt() begin exit(1); end; }".to_string()),
            ("OnPreXmlPort", "xmlport 50009 \"MNC X\" { schema { textelement(Root) { } } trigger OnPreXmlPort() begin exit(1); end; }".to_string()),
            ("DoIt", "xmlport 50009 \"MNC X\" { schema { textelement(Root) { } } procedure DoIt() begin exit(1); end; }".to_string()),
            ("OnBeforeOpen", "query 50009 \"MNC X\" { elements { dataitem(B; \"MNC T\") { column(Id; Id) { } } } trigger OnBeforeOpen() begin exit(1); end; }".to_string()),
            ("DoIt", "query 50009 \"MNC X\" { elements { dataitem(B; \"MNC T\") { column(Id; Id) { } } } procedure DoIt() begin exit(1); end; }".to_string()),
            ("OnAfterInsert", "tableextension 50009 \"MNC X\" extends \"MNC T\" { trigger OnAfterInsert() begin exit(1); end; }".to_string()),
            ("DoIt", "pageextension 50009 \"MNC X\" extends \"MNC P\" { procedure DoIt() begin exit(1); end; }".to_string()),
        ] {
            let got: Vec<String> = messages(&format!("{t} {p} {src}"))
                .into_iter()
                .filter(|m| m.starts_with("AL0139"))
                .collect();
            assert_eq!(
                got,
                vec![format!("AL0139 Since '{name}' doesn't have a return value, EXIT cannot be called with a value")],
                "{src}"
            );
        }
        // A page trigger that returns a value keeps compiling.
        let ok = format!(
            "{t} page 50009 \"MNC X\" {{ SourceTable = \"MNC T\"; trigger OnQueryClosePage(CloseAction: Action): Boolean begin exit(true); end; }}"
        );
        assert!(codes(&[&ok]).iter().all(|c| c != "AL0139"));
    }

    // must-not-compile 2167-2188: AL0135 through a codeunit variable, a record variable, `this`, a bare call inside a
    // table, and alc's rendering of every pinned parameter type in the signature (al-runner issue 5588).
    #[test]
    fn al0135_calls_on_variables_and_parameter_types() {
        let src = |lib: &str, call: &str| {
            format!(
                "table 50000 \"MNC SH Table\" {{ fields {{ field(1; Id; Integer) {{ }} }} keys {{ key(PK; Id) {{ Clustered = true; }} }}
                   procedure Take(A: Integer; B: Integer) begin end; }}
                 enum 50000 \"MNC SH Enum\" {{ value(0; None) {{ }} }}
                 interface \"MNC SH Iface\" {{ procedure X(); }}
                 page 50003 \"MNC SH Page\" {{ SourceTable = \"MNC SH Table\"; }}
                 codeunit 50001 \"MNC SH Lib\" {{ {lib} }}
                 codeunit 50002 \"MNC SH Caller\" {{ procedure Run1() var Lib: Codeunit \"MNC SH Lib\"; R: Record \"MNC SH Table\"; begin {call} end; }}"
            )
        };
        let al0135 =
            |src: &str| -> Vec<String> { messages(src).into_iter().filter(|m| m.starts_with("AL0135")).collect() };
        let want = |p: &str, sig: &str| {
            format!(
                "AL0135 There is no argument given that corresponds to the required formal parameter '{p}' of '{sig}'"
            )
        };
        assert_eq!(
            al0135(&src(
                "procedure Add(A: Integer; B: Integer): Integer begin exit(A + B); end;",
                "Lib.Add(1);"
            )),
            vec![want("B", "Add(Integer, Integer)")]
        );
        assert_eq!(
            al0135(&src("", "R.Take(1);")),
            vec![want("B", "Take(Integer, Integer)")]
        );
        assert_eq!(
            al0135(&src("procedure Sig(A: Integer) begin end;", "Lib.Sig();")),
            vec![want("A", "Sig(Integer)")]
        );
        for (decl, sig) in [
            ("R: Record \"MNC SH Table\"", "Record \"MNC SH Table\""),
            ("var R: Record \"MNC SH Table\"", "var Record \"MNC SH Table\""),
            (
                "var R: Record \"MNC SH Table\" temporary",
                "var Record \"MNC SH Table\" temporary",
            ),
            ("R: Code[20]", "Code[20]"),
            ("R: Option Alpha,Beta", "Option"),
            ("R: Enum \"MNC SH Enum\"", "Enum \"MNC SH Enum\""),
            ("R: Decimal", "Decimal"),
            ("R: Date", "Date"),
            ("var R: Integer", "var Integer"),
            ("R: Boolean", "Boolean"),
            ("R: Time", "Time"),
            ("R: DateTime", "DateTime"),
            ("R: Guid", "Guid"),
            ("R: BigInteger", "BigInteger"),
            ("R: Char", "Char"),
            ("R: Text[30]", "Text[30]"),
            ("R: Duration", "Duration"),
            ("R: DateFormula", "DateFormula"),
            ("R: Variant", "Variant"),
            ("R: RecordRef", "RecordRef"),
            ("R: FieldRef", "FieldRef"),
            ("R: List of [Text]", "List of [Text]"),
            ("R: Dictionary of [Integer, Text]", "Dictionary of [Integer, Text]"),
            ("R: Codeunit \"MNC SH Lib\"", "Codeunit \"MNC SH Lib\""),
            ("R: Page \"MNC SH Page\"", "Page \"MNC SH Page\""),
            ("R: JsonObject", "JsonObject"),
            ("R: array[3] of Integer", "array[3] of Integer"),
            ("R: SecretText", "SecretText"),
            ("R: Interface \"MNC SH Iface\"", "Interface \"MNC SH Iface\""),
            ("R: InStream", "InStream"),
            ("R: ErrorInfo", "ErrorInfo"),
        ] {
            assert_eq!(
                al0135(&src(
                    &format!("procedure Sig(X: Integer; {decl}) begin end;"),
                    "Lib.Sig(1);"
                )),
                vec![want("R", &format!("Sig(Integer, {sig})"))],
                "{decl}"
            );
        }
        // A bare call and a `this.` call inside the table itself (2169, 2170).
        let table = |call: &str| {
            format!(
                "table 50000 \"MNC T\" {{ fields {{ field(1; Id; Integer) {{ trigger OnValidate() begin {call} end; }} }}
                   procedure Take(A: Integer; B: Integer) begin end; procedure Run1() begin {call} end; }}"
            )
        };
        assert_eq!(al0135(&table("Take(1);")), vec![want("B", "Take(Integer, Integer)"); 2]);
        assert_eq!(
            al0135(&table("this.Take(1);")),
            vec![want("B", "Take(Integer, Integer)"); 2]
        );
        // Enough arguments compile.
        assert!(
            al0135(&src(
                "procedure Add(A: Integer; B: Integer) begin end;",
                "Lib.Add(1, 2); R.Take(1, 2);"
            ))
            .is_empty()
        );
    }

    // must-not-compile 0242 and 0066: AL0135 for a call of an own procedure with too few arguments, also where
    // nothing calls the caller (al-runner issue 5853).
    #[test]
    fn al0135_own_procedure_called_with_too_few_arguments() {
        assert_eq!(
            messages(
                "codeunit 50048 \"MNC C242\" { local procedure Take(A: Integer; B: Integer) begin end; procedure Run1() begin Take(1); end; }"
            ),
            vec![
                "AL0135 There is no argument given that corresponds to the required formal parameter 'B' of 'Take(Integer, Integer)'"
                    .to_string()
            ]
        );
        // The issue's shape: the call is an argument of exit, the procedure returns a value.
        assert_eq!(
            messages(
                "codeunit 50600 L { procedure Add(A: Integer; B: Integer): Integer begin exit(A + B); end; procedure Twice(A: Integer): Integer begin exit(Add(A)); end; }"
            ),
            vec![
                "AL0135 There is no argument given that corresponds to the required formal parameter 'B' of 'Add(Integer, Integer)'"
                    .to_string()
            ]
        );
        assert!(
            codes(&["codeunit 50048 C { local procedure Take(A: Integer; B: Integer) begin end; procedure Run1() begin Take(1, 2); end; }"])
                .is_empty()
        );
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

    // TestHandlers is a codeunit property: BC 29 compiles a test codeunit that names one (corpus
    // al-language-testlibraries/lt-bi-test-framework/BiTestHandler.al:98, CI job test-testlibraries on BC 29), and on BC 27 the
    // compiler answers AL0666 and AL0928 for it, never AL0124 (must-not-compile 0784).
    #[test]
    fn testhandlers_is_a_codeunit_property() {
        let src = "codeunit 50000 C { Subtype = Test; TestHandlers = \"My Handler\"; }";
        assert!(!codes(&[src]).contains(&"AL0124".to_string()), "{:?}", codes(&[src]));
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

    // must-not-compile 1780: a method the codeunit of a Codeunit variable does not declare (al-runner issue 5917). A
    // procedure of it (also a local one, which is AL0161), and the built-in Run, are no AL0132.
    #[test]
    fn al0132_method_the_codeunit_does_not_have() {
        let logic =
            "codeunit 50600 \"Probe Logic\" { procedure Add(A: Integer; B: Integer): Integer begin exit(A + B); end;
            local procedure Hidden() begin end; }";
        let call = |stmt: &str| {
            format!(
                "codeunit 50650 \"Probe Tests\" {{ procedure T() var Logic: Codeunit \"Probe Logic\"; begin {stmt}; end; }}"
            )
        };
        assert_eq!(
            messages(&format!(
                "{logic} {}",
                call("if Logic.NoSuch(1, 1) <> 2 then Error('x')")
            )),
            vec!["AL0132 'Codeunit \"Probe Logic\"' does not contain a definition for 'NoSuch'"]
        );
        for ok in ["Logic.Add(1, 1)", "Logic.Run()"] {
            assert!(codes(&[logic, &call(ok)]).is_empty(), "{ok}");
        }
        assert_eq!(codes(&[logic, &call("Logic.Hidden()")]), vec!["AL0161"]);
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
