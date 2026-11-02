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
//! AL0132, AL0161 and AL0398 (and AL0175 for Char - Text, AL0172 for unary minus on an Enum, Option or Duration; AL0126, AL0129 and AL0133 for the assignment form of a builtin method); they are listed in [`IMPLEMENTED`] or, where only some shapes are decided, [`NOT_CHECKED`].
//! The implicit-conversion rules of `check/conv.rs` add AL0129, AL0130, AL0133, AL0174, AL0175, AL0369, AL0698 and AL0777
//! for the shapes the compile matrix measured; those are listed in [`NOT_CHECKED`], with what is decided.

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
        "an enum assigned to an enum of another type whose target does not state AssignmentCompatibility = true; an Integer variable assigned to a Text; a text literal of two or more characters assigned to an Integer; an Enum variable, enum value or Enum field to a Decimal; Decimal or Integer to Boolean, Date to Text, Boolean, Text or Decimal to Date, Variant or a one-character literal to an Enum, a Record to an Integer; and every rejected cell of the implicit-conversion compile matrix in the contexts `:=`, `+=`, `exit`, `x in [...]`, a case label and the false branch of `c ? a : b` (check/conv.rs, tests/conversion_matrix.rs)",
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
        "a page extension names a non-protected variable or a local procedure of its base page, or a report extension does; a Cloud app names an internal agent table of the platform; a subscriber to the internal event GetRoleCenterBannerPartID of System Action Triggers; a page customization names a variable of its page in Editable of a modify; `CurrReport.RequestOptionsPage` (case 2514)",
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
        "a Cloud app names a platform table that is Scope = OnPrem (the 123 of System.app that the corpus lists, in code or in a permission set), calls GetLastErrorObject, SecretText.Unwrap, File.Open, a builtin overload the corpus pins as OnPrem (ONPREM_BUILTINS) or its own procedure with [Scope('OnPrem')] or [Scope('Internal')], uses the HttpClientHandler attribute, or sets the report property EnableExternalAssemblies to true",
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
        "AL0460",
        "a DotNet variable of a codeunit declared both [RunOnClient] and [WithEvents] (case 2335)",
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
        "AL0275",
        "reported for a variable of type DotNet N, N bare, in a file without a namespace, where the app declares the alias N outside a namespace and a loaded dependency declares it in namespace System (case 2359); a case is compiled without System Application, so the case cannot find it.",
    ),
    (
        "AL0126",
        "reported only for the assignment form `X.M := v` of a builtin method with two or more forms, each needing an argument (check/infer.rs; case 2368); other calls with no matching overload need overload resolution the front end does not judge.",
    ),
    (
        "AL0129",
        "reported only for the assignment form `X.M := v` of a builtin method without a parameter or with only optional ones, two or more (cases 2365, 2366, 2370, 2371), and a compound assignment `X.M += v` to a builtin method (case 2367; check/infer.rs); other left-hand sides are not judged.",
    ),
    (
        "AL0133",
        "reported only for the assignment form `X.M := v` of a builtin method whose one form takes one XML value, given an XML collection (check/infer.rs; case 2372); other argument conversions are not judged.",
    ),
    (
        "AL0196",
        "reported only for a bare call, without arguments, in a page extension of its own local or internal procedure without parameters that a procedure of the base page in the same app repeats (cases 2238-2240, 2244); other ambiguous calls need overload resolution the front end does not judge.",
    ),
    (
        "AL0129",
        "reported for the operand shapes and types of the implicit-conversion compile matrix (check/conv.rs, tests/conversion_matrix.rs: variables, parameters, fields, literals, enum values, option members and test page control values of the scalar, text, date, Guid, RecordId, Variant, TextBuilder, Media, enum and interface types), for `:=` and `+=` into a Label, a TextConst or a test page control's Value; any other target is not decided.",
    ),
    (
        "AL0130",
        "reported for the operand shapes and types of the implicit-conversion compile matrix (check/conv.rs, tests/conversion_matrix.rs: variables, parameters, fields, literals, enum values, option members and test page control values of the scalar, text, date, Guid, RecordId, Variant, TextBuilder, Media, enum and interface types), for a literal or option member passed to a `var` parameter of a procedure without overloads; other expressions are not decided.",
    ),
    (
        "AL0133",
        "reported for the operand shapes and types of the implicit-conversion compile matrix (check/conv.rs, tests/conversion_matrix.rs: variables, parameters, fields, literals, enum values, option members and test page control values of the scalar, text, date, Guid, RecordId, Variant, TextBuilder, Media, enum and interface types), for an argument of a procedure without overloads and `:=` into a test page control's Value; a call argument, an overloaded procedure and a builtin method are not decided.",
    ),
    (
        "AL0174",
        "reported for the operand shapes and types of the implicit-conversion compile matrix (check/conv.rs, tests/conversion_matrix.rs: variables, parameters, fields, literals, enum values, option members and test page control values of the scalar, text, date, Guid, RecordId, Variant, TextBuilder, Media, enum and interface types), for a one-character text literal plus or minus a Byte (and `+=`); other ambiguous operators are not decided.",
    ),
    (
        "AL0175",
        "reported for the operand shapes and types of the implicit-conversion compile matrix (check/conv.rs, tests/conversion_matrix.rs: variables, parameters, fields, literals, enum values, option members and test page control values of the scalar, text, date, Guid, RecordId, Variant, TextBuilder, Media, enum and interface types), for `=`, `<>`, `<`, `+`, `-` and `+=` (and `Char - Text` of must-not-compile 0500); a call operand, a literal right operand, other operators and DotNet operands (the .NET type behind the alias decides) are not decided.",
    ),
    (
        "AL0369",
        "reported for the operand shapes and types of the implicit-conversion compile matrix (check/conv.rs, tests/conversion_matrix.rs: variables, parameters, fields, literals, enum values, option members and test page control values of the scalar, text, date, Guid, RecordId, Variant, TextBuilder, Media, enum and interface types), for a BigInteger literal outside the Integer range into an Integer, Char, Byte, Option or Enum; other constants are not decided.",
    ),
    (
        "AL0698",
        "reported for the operand shapes and types of the implicit-conversion compile matrix (check/conv.rs, tests/conversion_matrix.rs: variables, parameters, fields, literals, enum values, option members and test page control values of the scalar, text, date, Guid, RecordId, Variant, TextBuilder, Media, enum and interface types), for a case statement whose selector is an interface; other selector types are not decided.",
    ),
    (
        "AL0777",
        "reported for the operand shapes and types of the implicit-conversion compile matrix (check/conv.rs, tests/conversion_matrix.rs: variables, parameters, fields, literals, enum values, option members and test page control values of the scalar, text, date, Guid, RecordId, Variant, TextBuilder, Media, enum and interface types), for a Guid into a Text or Code of length 10 or less (and a test page control's Value or a short text literal as the type of a set or conditional expression); a Text or Code between 11 and 37 is not decided.",
    ),
    (
        "AL0185",
        "reported only for the platform table Integer named bare by a Record variable or a report data item of a file in a namespace without `using System.Utilities;` (`platform_integer_without_using`: cases 2081, 2082), for a variable of a codeunit type nobody declares (`unknown_codeunit_variables`: case 0213), and for a TestRequestPage parameter typed on a name no report has (`test_request_pages_of_no_report`: case 2513); any other missing object is not decided.",
    ),
    (
        "AL0240",
        "reported only for a RequestPageHandler whose var TestRequestPage parameter is typed on a name no report has (`test_request_pages_of_no_report`: case 2513); other handler signatures are not judged.",
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

/// Why an expected diagnostic `code` with text `text` is not checked, if it is not: [`not_checked_reason`], and AL0301 for any
/// list but a Label declaration's. AL0301 is decided only for a trailing comma after a Label property (must-not-compile 2330,
/// 2331, corpus branch al-runtime-rs-staging-y12: "A list must end with a member; not a separator ,."); alc's AL0301 from its
/// recovery elsewhere (0357: "... separator ;.") is not reproduced.
pub fn not_checked_reason_for(code: &str, text: &str) -> Option<&'static str> {
    if code == "AL0301" && !text.starts_with("A list must end with a member; not a separator ,.") {
        return Some("AL0301 is decided only for a trailing comma after a property of a Label declaration");
    }
    not_checked_reason(code)
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
    let (mut out, objects, after) = check_workspace_parts(ws, 0);
    out.extend(objects);
    out.extend(after);
    out
}

/// The rules of [`check_workspace`] in their order, in three parts: those over the whole workspace that come first, the
/// rules of each workspace object from object `first` on (a rule of an object reads that object and what its app
/// sees), and those over the whole workspace that come last. A compile that grows by apps (ADR-063) keeps the object
/// rules' findings for the objects it had and runs them for the new objects only.
pub fn check_workspace_parts(ws: &Workspace, first: usize) -> (Vec<Diagnostic>, Vec<Diagnostic>, Vec<Diagnostic>) {
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
    let before = std::mem::take(&mut out);
    for oi in first..ws.objects.len() {
        let o = &ws.objects[oi];
        if ws.kinds[o.app] == AppKind::Dependency {
            continue;
        }
        let obj = ws.object_ast(oi);
        let (decls, types) = declarations(obj.syntax());
        if obj.kind() == "pagecustomization" {
            page_customization_variables(ws, oi, &obj, &mut out);
        }
        subscriber_publishers(ws, oi, &obj, &mut out);
        platform_integer_without_using(ws, oi, &obj, &types, &mut out);
        unknown_codeunit_variables(ws, oi, &decls, &mut out);
        test_request_pages_of_no_report(ws, oi, &obj, &mut out);
        ambiguous_dotnet_aliases(ws, oi, &decls, &mut out);
        object_properties(ws, oi, &obj, &mut out);
        object_triggers(ws, oi, &obj, &mut out);
        label_trailing_commas(ws, o.file, &types, &mut out);
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
            ObjKind::Codeunit => {
                duplicate_procedures(ws, oi, &obj, &mut out);
                client_events_outside_pages(ws, oi, &decls, &mut out);
            }
            ObjKind::Page => {
                field_control_procedure_calls(ws, oi, &obj, &mut out);
                control_visible::check(ws, oi, &obj, &page_procedures(&obj), &mut out);
            }
            ObjKind::Query => query_joins(ws, o.file, obj.sections(), &mut out),
            ObjKind::Report => {
                let globals = report_variable_columns(ws, o.file, &obj);
                report_columns(ws, o.file, obj.sections(), &globals, &mut out)
            }
            _ => {}
        }
    }
    // AL0169: a property value that is none of the property's values.
    let mut after = Vec::new();
    crate::property_values::check(ws, &mut after);
    (before, out, after)
}

/// AL0301 and AL0107 for the trailing commas of a Label declaration (al-runner issue 6436). One comma right after the text
/// compiles (text/lt-label-textbuilder/TxCLabel.al Y12_Label_TrailingComma); a trailing comma after a property is AL0301
/// (must-not-compile 2330 `Locked = true,;`, 2331 `Comment = 'c',;`), two commas after the text AL0107 (2332 `'T',,;`).
/// The diagnostic is at the comma that is wrong; the parser rejects both shapes too, so the file never runs.
fn label_trailing_commas(ws: &Workspace, file: usize, types: &[SyntaxNode], out: &mut Vec<Diagnostic>) {
    let significant = |t: &al_syntax::SyntaxToken| !t.kind().is_trivia();
    for n in types {
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
fn platform_integer_without_using(
    ws: &Workspace,
    oi: usize,
    obj: &ast::Object,
    types: &[SyntaxNode],
    out: &mut Vec<Diagnostic>,
) {
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
    let mut at: Vec<usize> = types
        .iter()
        .cloned()
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
fn unknown_codeunit_variables(ws: &Workspace, oi: usize, decls: &[ast::VarDecl], out: &mut Vec<Diagnostic>) {
    let file = ws.objects[oi].file;
    for t in decls.iter().filter_map(|d| d.type_ref()) {
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

/// AL0185 `Report 'X' is missing`: a `TestRequestPage` parameter typed on a name no report has (must-not-compile
/// 2513-testrequestpage-typed-on-xmlport.al: an xmlport's name; TestRequestPage takes a report only). On a procedure with the
/// RequestPageHandler attribute the handler's signature is then AL0240, its parameter's type shown blank. Only a one-segment
/// name is judged; an id, a namespace-qualified name and a variable declaration keep compiling (not pinned).
fn test_request_pages_of_no_report(ws: &Workspace, oi: usize, obj: &ast::Object, out: &mut Vec<Diagnostic>) {
    let file = ws.objects[oi].file;
    for p in obj.routines() {
        for (i, param) in p.params().enumerate() {
            let Some(t) = param.type_ref() else { continue };
            if !t.type_name().eq_ignore_ascii_case("TestRequestPage") {
                continue;
            }
            let parts = t.object_name_parts();
            if parts.len() != 1 || parts[0].parse::<u32>().is_ok() || ws.names_object(file, ObjKind::Report, &parts) {
                continue;
            }
            push(
                out,
                ws,
                file,
                offset_of(t.syntax()),
                "AL0185",
                format!("Report '{}' is missing", parts[0]),
            );
            // The pinned handler takes the page by var; another shape of the signature is not pinned.
            if param.is_var()
                && p.attributes()
                    .any(|a| a.name().eq_ignore_ascii_case("RequestPageHandler"))
            {
                let name = p.name();
                push(
                    out,
                    ws,
                    file,
                    offset_of(p.syntax()),
                    "AL0240",
                    format!(
                        "The signature of procedure '{name}' does not match the signature required by attribute 'RequestPageHandler': parameter {} is expected to be of type 'var TestRequestPage' but found type 'var '. The expected signature is: {name}(var RequestPage: TestRequestPage).",
                        i + 1
                    ),
                );
            }
        }
    }
}

/// AL0460 (corpus must-not-compile 2335): a DotNet variable of a codeunit declared both `[RunOnClient]` and `[WithEvents]`.
/// The same declaration on a page compiles (al-language-onprem-dotnet/lt-bg-runonclient/Y16RunOnClientShapes.al, page
/// 99475). Other object kinds are not judged.
fn client_events_outside_pages(ws: &Workspace, oi: usize, decls: &[ast::VarDecl], out: &mut Vec<Diagnostic>) {
    for d in decls {
        let names: Vec<String> = d.attributes().iter().map(|a| a.name().to_ascii_lowercase()).collect();
        if names.iter().any(|n| n == "runonclient") && names.iter().any(|n| n == "withevents") {
            push(
                out,
                ws,
                ws.objects[oi].file,
                offset_of(d.syntax()),
                "AL0460",
                "Client-side events are supported only on pages and page extensions.".into(),
            );
        }
    }
}

/// AL0275: a variable of type `DotNet N`, N bare, where the app declares the alias N outside a namespace and an app it
/// depends on declares N in namespace System (must-not-compile 2359, corpus writer Y20, al-runner issue 6488: System
/// Application declares String, DateTime and StringBuilder there). The dependency's alias alone, and `DotNet System.N`,
/// compile (al-language-onprem-dotnet/lt-y20-dotnet-fullname/Y20DotNetFullTypeName.al:27-58). Only that shape is pinned: a
/// file or a declaration in a namespace, another namespace of the dependency, a parameter or a return type keep compiling.
/// A dependency that is not loaded declares nothing here, so nothing is reported.
fn ambiguous_dotnet_aliases(ws: &Workspace, oi: usize, decls: &[ast::VarDecl], out: &mut Vec<Diagnostic>) {
    let file = ws.objects[oi].file;
    let app = ws.files[file].app;
    if ws.files[file].namespace.is_some() || !ws.dotnet.iter().any(|d| d.app == app) {
        return;
    }
    let declared = |name: &str, in_app: &dyn Fn(usize) -> bool, ns: Option<&str>| {
        ws.dotnet.iter().any(|d| {
            in_app(d.app)
                && d.decl.alias.eq_ignore_ascii_case(name)
                && match (ns, ws.files[d.file].namespace.as_deref()) {
                    (None, None) => true,
                    (Some(want), Some(have)) => want.eq_ignore_ascii_case(have),
                    _ => false,
                }
        })
    };
    for t in decls.iter().filter_map(|d| d.type_ref()) {
        if !t.type_name().eq_ignore_ascii_case("DotNet") {
            continue;
        }
        let [name] = t.object_name_parts().try_into().unwrap_or_default();
        if name.is_empty()
            || !declared(&name, &|a| a == app, None)
            || !declared(&name, &|a| a != app && ws.app_sees(app, a), Some("System"))
        {
            continue;
        }
        push(
            out,
            ws,
            file,
            offset_of(t.syntax()),
            "AL0275",
            format!("'{name}' is an ambiguous reference between '{name}' defined by the extension"),
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
    let procedures = page_procedures(obj);
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

/// The names of the procedures of a page, lower case without quotes.
fn page_procedures(obj: &ast::Object) -> Vec<String> {
    obj.routines()
        .filter(|r| !r.is_trigger())
        .map(|r| r.name().trim_matches('"').to_ascii_lowercase())
        .collect()
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
/// The longest name of a permission set with `Assignable = true` (2131-permissionset-name-21-characters.al:3-5:
/// 21 is refused, 20 compiles). Other permission sets have the limit of 30 (2333-permissionset-not-assignable-name-31-characters.al,
/// 2334-permissionset-no-assignable-name-31-characters.al: 31 is refused; al-language/permissions/lt-y12-permset-name-length/
/// Y12PermSetNames.al: 21 to 30 compile; al-runner issue 6445).
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
    for (oi, o) in ws.objects.iter().enumerate() {
        if ws.kinds[o.app] == AppKind::Dependency {
            continue;
        }
        let Some(mut limit) = name_limit(o.kind) else { continue };
        // The 20-character limit holds only with an explicit `Assignable = true` (case 2131); without it the limit is 30
        // (cases 2333, 2334; issue 6445).
        if o.kind == ObjKind::PermissionSet
            && !ws.object_ast(oi).properties().any(|p| {
                p.name().eq_ignore_ascii_case("Assignable") && p.value_text().trim().eq_ignore_ascii_case("true")
            })
        {
            limit = AL0305_LIMIT;
        }
        if o.name.chars().count() <= limit {
            continue;
        }
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
/// The variable declarations of an object and its type references (a nested one after the one that holds it), in source
/// order. The grammar reads a var section only in an object body and before a routine's statement block
/// (`al_syntax::grammar::declarations::var_section`), and a type reference only in a variable declaration, a parameter, a
/// return type and another type reference (`declarations::type_ref`), so a statement block holds none of them; the blocks
/// are most of the tree, so they are not entered.
fn declarations(obj: &SyntaxNode) -> (Vec<ast::VarDecl>, Vec<SyntaxNode>) {
    let (mut decls, mut types) = (Vec::new(), Vec::new());
    let mut walk = obj.preorder();
    while let Some(ev) = walk.next() {
        let rowan::WalkEvent::Enter(n) = ev else { continue };
        match n.kind() {
            SyntaxKind::Block => walk.skip_subtree(),
            SyntaxKind::VarDecl => decls.extend(ast::VarDecl::cast(n)),
            SyntaxKind::TypeRef => types.push(n),
            _ => {}
        }
    }
    (decls, types)
}

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
    // AL0296 (corpus 0006, 0014; from BC 27 0020 to 0024, from BC 28 1240): a Cloud app names a platform table that is
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
    // AL0296 (corpus 1712-permissionset-tabledata-onprem-table.al:1, 1713-permissionset-table-object-onprem-table.al:1):
    // a permission set of a Cloud app names, as `tabledata` or `table`, a platform table that is Scope = OnPrem. Only
    // the tables of the OnPrem lists that are neither removed nor internal: for those alc may add AL0433 or AL0161 in a
    // permission set too, which no case pins.
    if ctx.target == AppTarget::Cloud {
        let known = al_version::platform_table_scopes_known(ctx.bc_major);
        for (oi, o) in ws.objects.iter().enumerate() {
            if o.app != app || o.kind != ObjKind::PermissionSet {
                continue;
            }
            let obj = ws.object_ast(oi);
            for p in obj
                .properties()
                .filter(|p| p.name().eq_ignore_ascii_case("Permissions"))
            {
                for (kind, name) in permission_entries(&p.value_text()) {
                    if !kind.eq_ignore_ascii_case("tabledata") && !kind.eq_ignore_ascii_case("table") {
                        continue;
                    }
                    let find = |l: &[&'static str]| l.iter().copied().find(|x| x.eq_ignore_ascii_case(&name));
                    let Some(table) =
                        find(ONPREM_PLATFORM_TABLES).or(known.then(|| find(ONPREM_PLATFORM_TABLES_27)).flatten())
                    else {
                        continue;
                    };
                    let removed = REMOVED_PLATFORM_TABLES_27
                        .iter()
                        .any(|(x, _)| x.eq_ignore_ascii_case(table));
                    let internal = INTERNAL_AGENT_TABLES_27.iter().any(|x| x.eq_ignore_ascii_case(table));
                    if removed || internal || ws.find(ObjKind::Table, &name).is_some() {
                        continue;
                    }
                    push(
                        &mut out,
                        ws,
                        o.file,
                        offset_of(p.syntax()),
                        "AL0296",
                        format!(
                            "The application object or method '{table}' has scope 'OnPrem' and cannot be used for 'Cloud' development."
                        ),
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

/// The entries `<kind> <name> = <letters>` of a `Permissions` property value, as (kind, unquoted name).
fn permission_entries(value: &str) -> Vec<(String, String)> {
    // Split at the commas outside quotes.
    let mut parts = Vec::new();
    let (mut cur, mut quoted) = (String::new(), false);
    for ch in value.chars() {
        match ch {
            '"' => {
                quoted = !quoted;
                cur.push(ch);
            }
            ',' if !quoted => parts.push(std::mem::take(&mut cur)),
            _ => cur.push(ch),
        }
    }
    parts.push(cur);
    parts
        .iter()
        .filter_map(|e| {
            let lhs = e.rsplit_once('=').map_or(e.as_str(), |(l, _)| l).trim();
            let (kind, name) = lhs.split_once(char::is_whitespace)?;
            let name = name.trim();
            let name = name.strip_prefix('"').and_then(|n| n.strip_suffix('"')).unwrap_or(name);
            Some((kind.to_string(), name.to_string()))
        })
        .collect()
}

/// The builtin overloads (catalog ids) that have scope OnPrem, by the name alc reports. Per overload: a Cloud app may
/// call another overload of the same method. Pinned by must-not-compile (each case's line 1 holds the text):
/// 0563 Blob.Export, 0564 Blob.Import, 0565 SessionInformation.AITokensUsed, 0566 Database.AlterKey, 0567
/// Database.ChangeUserPassword, 0568 Database.SetUserPassword, 0569 Database.ExportData, 0570 Database.ImportData, 0571
/// CanLoadType, 0572 ExportEncryptionKey, 0573 ImportEncryptionKey, 0574 ImportStreamWithUrlAccess, 0575
/// Session.LogSecurityAudit, 0643 Media.ExportFile, 0644 Media.ImportFile, 0645 MediaSet.ExportFile, 0646
/// MediaSet.ImportFile, 0651 HttpClient.UseDefaultNetworkWindowsAuthentication, 0652 and 0653 both
/// HttpClient.UseWindowsAuthentication overloads, 0660 (static) and 0661 (instance) Report.SaveAsExcel, SaveAsHtml,
/// SaveAsPdf, SaveAsWord and SaveAsXml with a file name, 1310 File.IsPathTemporary, 1311 File.Download, 1312
/// Database.DataFileInformation.
const ONPREM_BUILTINS: &[(&str, &str)] = &[
    ("Blob.Export#1", "Export"),
    ("Blob.Import#1", "Import"),
    ("SessionInformation.AITokensUsed#1", "AITokensUsed"),
    ("Database.AlterKey#1", "AlterKey"),
    ("Database.ChangeUserPassword#1", "ChangeUserPassword"),
    ("Database.SetUserPassword#1", "SetUserPassword"),
    ("Database.ExportData#1", "ExportData"),
    ("Database.ImportData#1", "ImportData"),
    ("Database.DataFileInformation#1", "DataFileInformation"),
    ("System.CanLoadType#1", "CanLoadType"),
    ("System.ExportEncryptionKey#1", "ExportEncryptionKey"),
    ("System.ImportEncryptionKey#1", "ImportEncryptionKey"),
    ("System.ImportStreamWithUrlAccess#1", "ImportStreamWithUrlAccess"),
    ("Session.LogSecurityAudit#1", "LogSecurityAudit"),
    ("Media.ExportFile#1", "ExportFile"),
    ("Media.ImportFile#1", "ImportFile"),
    ("MediaSet.ExportFile#1", "ExportFile"),
    ("MediaSet.ImportFile#1", "ImportFile"),
    (
        "HttpClient.UseDefaultNetworkWindowsAuthentication#1",
        "UseDefaultNetworkWindowsAuthentication",
    ),
    ("HttpClient.UseWindowsAuthentication#1", "UseWindowsAuthentication"),
    ("HttpClient.UseWindowsAuthentication#2", "UseWindowsAuthentication"),
    ("Report.SaveAsExcel#1", "SaveAsExcel"),
    ("Report.SaveAsExcel#2", "SaveAsExcel"),
    ("Report.SaveAsHtml#1", "SaveAsHtml"),
    ("Report.SaveAsHtml#2", "SaveAsHtml"),
    ("Report.SaveAsPdf#1", "SaveAsPdf"),
    ("Report.SaveAsPdf#2", "SaveAsPdf"),
    ("Report.SaveAsWord#1", "SaveAsWord"),
    ("Report.SaveAsWord#2", "SaveAsWord"),
    ("Report.SaveAsXml#1", "SaveAsXml"),
    ("Report.SaveAsXml#2", "SaveAsXml"),
    ("File.IsPathTemporary#1", "IsPathTemporary"),
    ("File.Download#1", "Download"),
];

/// AL0296 that needs what the check resolved: a Cloud app calls a builtin overload with scope OnPrem
/// ([`ONPREM_BUILTINS`]), calls a procedure of its own declared `[Scope('OnPrem')]` (must-not-compile
/// 0981-scope-onprem-procedure-called-in-cloud-app.al:1) or `[Scope('Internal')]` (0982-scope-internal-procedure-called-in-cloud-app.al:1),
/// or puts the attribute `[HttpClientHandler]` on a procedure (0983-httpclienthandler-cloud.al:1).
pub fn check_resolved_scope(
    ws: &Workspace,
    app: usize,
    ctx: &AppContext,
    checked: &crate::check::CheckResult,
) -> Vec<Diagnostic> {
    resolved_scope(ws, app, ctx, &checked.resolutions)
}

/// [`check_resolved_scope`] over the resolutions of one check (the check of the code a selection left out has its own).
pub(crate) fn resolved_scope(
    ws: &Workspace,
    app: usize,
    ctx: &AppContext,
    resolutions: &crate::check::PosMap<crate::check::Res>,
) -> Vec<Diagnostic> {
    use crate::check::Res;
    let mut out = Vec::new();
    if ctx.target != AppTarget::Cloud {
        return out;
    }
    let msg = |name: &str, scope: &str| {
        format!(
            "The application object or method '{name}' has scope '{scope}' and cannot be used for 'Cloud' development."
        )
    };
    let mut scopes: std::collections::HashMap<(usize, usize), Option<(String, &'static str)>> = Default::default();
    for (fi, f) in ws.files.iter().enumerate() {
        if f.app != app {
            continue;
        }
        let root = SyntaxNode::new_root(ws.green(fi));
        for n in root.descendants() {
            match n.kind() {
                SyntaxKind::CallExpr => {
                    let r = n.text_range();
                    let Some(res) = resolutions.get(&(fi, r.start().into(), r.end().into())) else {
                        continue;
                    };
                    match res.inner() {
                        Res::Builtin(id) => {
                            if let Some((_, name)) = ONPREM_BUILTINS.iter().find(|(i, _)| i == id) {
                                push(&mut out, ws, fi, offset_of(&n), "AL0296", msg(name, "OnPrem"));
                            }
                        }
                        Res::Proc { object, offset } if ws.objects[*object].app == app => {
                            // Worked out once per routine: a routine is called from many places.
                            let scoped = scopes
                                .entry((*object, *offset))
                                .or_insert_with(|| routine_scope(ws, *object, *offset));
                            if let Some((name, scope)) = scoped {
                                push(&mut out, ws, fi, offset_of(&n), "AL0296", msg(name, scope));
                            }
                        }
                        _ => {}
                    }
                }
                SyntaxKind::Attribute => {
                    let Some(a) = ast::Attribute::cast(n.clone()) else {
                        continue;
                    };
                    if a.name().eq_ignore_ascii_case("HttpClientHandler") {
                        push(
                            &mut out,
                            ws,
                            fi,
                            offset_of(&n),
                            "AL0296",
                            msg("HttpClientHandler", "OnPrem"),
                        );
                    }
                }
                _ => {}
            }
        }
    }
    out
}

/// The OnPrem-only calls of one object (see [`check_context`]).
/// The name and the `Scope` (OnPrem or Internal) of the routine of object `object` at `offset`, if it has one of them.
fn routine_scope(ws: &Workspace, object: usize, offset: usize) -> Option<(String, &'static str)> {
    let obj = ws.object_ast(object);
    let routine = ast::routines_in(obj.syntax())
        .into_iter()
        .find(|r| usize::from(r.syntax().text_range().start()) == offset)?;
    let scope = routine.attributes().find_map(|a| {
        if !a.name().eq_ignore_ascii_case("Scope") {
            return None;
        }
        let arg = a.args()?.text().to_string();
        let arg = arg
            .trim()
            .trim_start_matches('(')
            .trim_end_matches(')')
            .trim()
            .trim_matches('\'');
        ["OnPrem", "Internal"].into_iter().find(|s| s.eq_ignore_ascii_case(arg))
    })?;
    Some((routine.name(), scope))
}

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

mod control_visible;
#[cfg(test)]
mod tests;
