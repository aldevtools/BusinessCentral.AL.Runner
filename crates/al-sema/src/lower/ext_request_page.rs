//! The request page of a report whose reportextensions change the run: what the test client cannot drive yet (`reasons`),
//! and the one shape with triggers on the extensions' request pages that the corpus pins (`pinned_request_page`).
//!
//! A field of such an extension bound to its own variable reaches the run (reports/WdReportExtRequestPage.al:98
//! WD_ReportExt_RequestPageFieldsReachTheRun, al-runner issue 6302), as does one bound to a field of a data item or of its own
//! record variable (reports/extensions/W3RpExtRequestPage.al:325-490, :1250). A field bound to a variable of the report reads
//! apart from the report's own control (W3RpExtRequestPage.al:214 `after ext after=6: main=[5]`) and SaveValues keeps the
//! extension's variables (reports/AqTests.al:479 SavedExt_2_NextRunShows): neither is built, so both stay refused, as does a
//! field bound to anything else. SaveValues: the SaveAsXml and Cancel closes are pinned (al-language-onprem/reports/
//! layouts-saveas/Y60ReportExtSaveAsXml.al); the VM refuses the other closes (`refuse_ext_save_values_close`).
//!
//! Triggers on the extension's request page are pinned for one shape only (`pinned_request_page`); every other one stays
//! refused.

use crate::workspace::{ObjKind, Resolution, Workspace};
use al_ir as ir;
use al_syntax::SyntaxKind;
use al_syntax::ast;
use std::collections::HashSet;

/// `Some(order)` only for the request page of report `oi` that the corpus pins with triggers on its extensions' request
/// pages; each fact is required by equality:
/// - the report's request page is the measured one (`base_anchor`; R7358XaIndependentReports.al:88-111);
/// - its request page extensions `request_exts` are its run-changing extensions `changing`, and there are two
///   (R7358XbIndependentReportExts.al:3, R7358XdIndependentReportExts.al:3);
/// - they are of two apps with no dependency path between them (`ExtOrder::independent_pair`; xapp-b and xapp-d app.json);
/// - each extension's request page is the measured one (`extension_anchor`), and both add after the one field of the
///   report's request page (R7358XaIndependentReports.al:94, R7358XbIndependentReportExts.al:105, R7358XdIndependentReportExts.al:105).
///
/// `order` is the order the extensions apply to the request page: the run order (`ext_order`), app B before app D. Their
/// triggers run in it after the report's own (IndependentAppsReportExtensions.al:108, :138 `A:rp open;B:rp open;D:rp open;
/// A:rp close;B:rp close;D:rp close;`), and the parameters text lists their fields in it after the report's (:109, :139:
/// BaseOptionValue, BFieldValue, DFieldValue; `request_parameters` writes the base's controls, then each extension's in this
/// order). Both extensions add after the same field, so the later one lands next to it (controls: base, D, B): no pinned
/// call reads that order, and the VM lets the handler touch no control of this page (al-vm `pages/request_ok_only.rs`).
/// The test's call and the page's close are gated there as well. The run order rests on the assumption `ext_order` names,
/// which the report raises on every run (`ir::Report::assumes`).
pub(crate) fn pinned_request_page(
    ws: &Workspace,
    oi: usize,
    request_exts: &[usize],
    changing: &[usize],
) -> Option<Vec<usize>> {
    let base_anchor = base_anchor(ws, oi)?;
    let (mut a, mut b) = (request_exts.to_vec(), changing.to_vec());
    a.sort_unstable();
    b.sort_unstable();
    if a != b || a.len() != 2 {
        return None;
    }
    let order = super::ext_order::report_extension_order(ws, oi, changing).ok()?;
    if !order.independent_pair {
        return None;
    }
    let anchors: Vec<String> = order
        .exts
        .iter()
        .map(|&e| extension_anchor(ws, e))
        .collect::<Option<_>>()?;
    anchors.iter().all(|a| *a == base_anchor).then_some(order.exts)
}

/// The name (lower case) of the one field of report `oi`'s request page when that page is exactly the measured one
/// (R7358XaIndependentReports.al:88-111): no property and no variable; the triggers OnOpenPage and OnClosePage and no other
/// routine, each only calling a procedure of a SingleInstance codeunit variable of the report with one text literal
/// (`calls_single_instance_codeunit`); a `layout` that holds one `area(Content)` that holds one field, bound to a Boolean
/// variable of the report (:135), stating ApplicationArea and Caption and nothing else, with no trigger.
fn base_anchor(ws: &Workspace, oi: usize) -> Option<String> {
    let report = ws.object_ast(oi);
    let rp = only_request_page(&report)?;
    logging_open_and_close(ws, oi, &report, &rp)?;
    let layout = only_section(&rp, "layout")?;
    no_properties(&layout)?;
    let area = only_section(&layout, "area")?;
    no_properties(&area)?;
    let [content]: [String; 1] = area.args().try_into().ok()?;
    if !content.eq_ignore_ascii_case("Content") {
        return None;
    }
    pinned_field(&report, &only_section(&area, "field")?)
}

/// The anchor (lower case) of reportextension `e`'s request page when it is exactly the measured one
/// (R7358XbIndependentReportExts.al:101-124): no property and no variable; the triggers OnOpenPage and OnClosePage and no
/// other routine, each only calling a procedure of a SingleInstance codeunit variable of the extension with one text
/// literal (`calls_single_instance_codeunit`); a `layout` with one `addafter(<field>)` that adds one field, bound to a
/// Boolean variable of the extension (R7358XbIndependentReportExts.al:144), stating ApplicationArea and Caption and nothing
/// else, with no trigger.
fn extension_anchor(ws: &Workspace, e: usize) -> Option<String> {
    let ext = ws.object_ast(e);
    let rp = only_request_page(&ext)?;
    logging_open_and_close(ws, e, &ext, &rp)?;
    let layout = only_section(&rp, "layout")?;
    no_properties(&layout)?;
    let add = only_section(&layout, "addafter")?;
    no_properties(&add)?;
    let [anchor] = add.args().try_into().ok()?;
    pinned_field(&ext, &only_section(&add, "field")?)?;
    Some(anchor.to_ascii_lowercase())
}

/// The one `requestpage` section of `object`, when it states no property and declares no variable.
fn only_request_page(object: &ast::Object) -> Option<ast::Section> {
    let mut sections = object
        .sections()
        .filter(|s| s.name().eq_ignore_ascii_case("requestpage"));
    let (Some(rp), None) = (sections.next(), sections.next()) else {
        return None;
    };
    (rp.properties().next().is_none() && rp.var_decls().next().is_none()).then_some(rp)
}

/// The request page `rp` of object `o` declares exactly the triggers OnOpenPage and OnClosePage, each calling a procedure
/// of a SingleInstance codeunit with one text literal (R7358XaIndependentReports.al:102-110,
/// R7358XbIndependentReportExts.al:115-123).
fn logging_open_and_close(ws: &Workspace, o: usize, object: &ast::Object, rp: &ast::Section) -> Option<()> {
    let routines: Vec<ast::Routine> = rp.triggers().collect();
    let mut names: Vec<String> = routines.iter().map(|r| r.name().to_ascii_lowercase()).collect();
    names.sort_unstable();
    (names == ["onclosepage", "onopenpage"]
        && routines
            .iter()
            .all(|r| r.is_trigger() && calls_single_instance_codeunit(ws, o, object, r)))
    .then_some(())
}

/// The name (lower case) of `field` when it is bound to a Boolean variable of `object` and states ApplicationArea and
/// Caption and nothing else (R7358XaIndependentReports.al:94-98, R7358XbIndependentReportExts.al:107-111).
fn pinned_field(object: &ast::Object, field: &ast::Section) -> Option<String> {
    let [name, source]: [String; 2] = field.args_raw().try_into().ok()?;
    let source = crate::pages::plain_name(&source)?;
    let boolean = object
        .var_decls()
        .find(|v| v.names().iter().any(|n| n.eq_ignore_ascii_case(&source)))
        .and_then(|v| v.type_ref())
        .is_some_and(|t| t.text().eq_ignore_ascii_case("Boolean"));
    let mut props: Vec<String> = field.properties().map(|p| p.name().to_ascii_lowercase()).collect();
    props.sort_unstable();
    (boolean && props == ["applicationarea", "caption"] && field.sections().next().is_none())
        .then(|| al_syntax::ast::unquote_ident(&name).to_ascii_lowercase())
}

/// The one section of `parent` when it is named `name` and declares no trigger (`parent` holds no other section).
fn only_section(parent: &ast::Section, name: &str) -> Option<ast::Section> {
    let mut sections = parent.sections();
    let (Some(s), None) = (sections.next(), sections.next()) else {
        return None;
    };
    (s.name().eq_ignore_ascii_case(name) && s.triggers().next().is_none()).then_some(s)
}

/// `s` states no property.
fn no_properties(s: &ast::Section) -> Option<()> {
    s.properties().next().is_none().then_some(())
}

/// Whether trigger `r` of object `e` (a report or a reportextension) has no parameter and no local variable, and its body is
/// one statement that calls a procedure of a global variable of the object whose type is a SingleInstance codeunit, with one
/// text literal (R7358XbIndependentReportExts.al:115-123 `Log.Add('B:rp open');`, R7358XaIndependentReports.al:102-110),
/// and that procedure is the only one of its name in the codeunit, has no attribute and takes one Text by value
/// (R7358XaIndependentReports.al:14-24). What the procedure's body does is not checked here: an error it raises on the page
/// is refused at run time (al-vm `pages/request_ok_only.rs`).
fn calls_single_instance_codeunit(ws: &Workspace, e: usize, ext: &ast::Object, r: &ast::Routine) -> bool {
    if r.params().next().is_some() || r.local_var_decls().next().is_some() {
        return false;
    }
    let Some(body) = r.body() else {
        return false;
    };
    let kids = |n: &al_syntax::SyntaxNode, kind: SyntaxKind| -> Option<Vec<al_syntax::SyntaxNode>> {
        (n.kind() == kind).then(|| n.children().collect())
    };
    let Some([stmt]) = kids(&body, SyntaxKind::Block).and_then(|k| <[_; 1]>::try_from(k).ok()) else {
        return false;
    };
    let Some([call]) = kids(&stmt, SyntaxKind::ExprStmt).and_then(|k| <[_; 1]>::try_from(k).ok()) else {
        return false;
    };
    let Some([callee, args]) = kids(&call, SyntaxKind::CallExpr).and_then(|k| <[_; 2]>::try_from(k).ok()) else {
        return false;
    };
    let Some([var, member]) = kids(&callee, SyntaxKind::MemberExpr).and_then(|k| <[_; 2]>::try_from(k).ok()) else {
        return false;
    };
    let Some([arg]) = kids(&args, SyntaxKind::ArgList).and_then(|k| <[_; 1]>::try_from(k).ok()) else {
        return false;
    };
    let literal = arg.kind() == SyntaxKind::Literal
        && al_syntax::ast::first_significant_green_token(&arg).is_some_and(|t| t.kind() == SyntaxKind::StringLit);
    if var.kind() != SyntaxKind::NameRef || !literal {
        return false;
    }
    let name = al_syntax::ast::unquote_ident(&var.text().to_string());
    let Some(ty) = ext
        .var_decls()
        .find(|v| v.names().iter().any(|n| n.eq_ignore_ascii_case(&name)))
        .and_then(|v| v.type_ref())
    else {
        return false;
    };
    if !ty.type_name().eq_ignore_ascii_case("codeunit") {
        return false;
    }
    let Resolution::Found(cu) = ws.resolve_ref(ws.objects[e].file, ObjKind::Codeunit, &ty.object_name_parts()) else {
        return false;
    };
    let codeunit = ws.object_ast(cu);
    let single_instance = codeunit
        .properties()
        .any(|p| p.name().eq_ignore_ascii_case("SingleInstance") && p.value_text().trim().eq_ignore_ascii_case("true"));
    let procedure = al_syntax::ast::unquote_ident(member.text().to_string().trim());
    let mut named = codeunit
        .routines()
        .filter(|p| p.name().eq_ignore_ascii_case(&procedure));
    let (Some(callee), None) = (named.next(), named.next()) else {
        return false;
    };
    let params: Vec<ast::Param> = callee.params().collect();
    let text_by_value = matches!(params.as_slice(), [p] if !p.is_var()
        && p.type_ref().is_some_and(|t| t.text().eq_ignore_ascii_case("Text")));
    single_instance && callee.attributes().next().is_none() && text_by_value
}

/// The request page `section` states `SaveValues = true`.
fn save_values(section: Option<&ast::Section>) -> bool {
    section.is_some_and(|s| {
        s.properties()
            .any(|p| p.name().eq_ignore_ascii_case("SaveValues") && p.value_text().trim().eq_ignore_ascii_case("true"))
    })
}

/// Why the test client cannot drive the request page of `base` (its `request` layout and `request_section`) for each
/// extension of `exts` that is one of the run-changing extensions `changing`. `pinned`: `pinned_request_page` admitted it,
/// and the triggers on the extensions' request pages are no reason.
pub(crate) fn reasons(
    ws: &Workspace,
    (request, request_section): (&crate::pages::Layout, Option<&ast::Section>),
    (base, items): (&ast::Object, &[crate::reports::RawItem]),
    exts: &[(usize, ast::Object)],
    changing: &[usize],
    pinned: bool,
) -> Vec<String> {
    let save_values = save_values(request_section);
    let mut out = Vec::new();
    for (e, ext) in exts.iter().filter(|(e, _)| changing.contains(e)) {
        let own: HashSet<String> = ext
            .var_decls()
            .flat_map(|v| v.names())
            .map(|n| n.to_lowercase())
            .collect();
        let foreign = foreign_request_control(request, *e, &own, base, items);
        fn has_trigger(s: &ast::Section) -> bool {
            s.triggers().next().is_some() || s.sections().any(|c| has_trigger(&c))
        }
        let triggers = !pinned
            && ext
                .sections()
                .filter(|s| s.name().eq_ignore_ascii_case("requestpage"))
                .any(|s| has_trigger(&s));
        let why = match (save_values, foreign, triggers) {
            (true, Some(_), _) | (true, None, true) => Some("a request page with SaveValues".to_string()),
            (false, Some(c), _) => Some(format!("field {} not bound to a variable of the extension", c.name)),
            (false, None, true) => Some("a trigger on the extension's request page".to_string()),
            (_, None, false) => None,
        };
        if let Some(why) = why {
            out.push(format!(
                "reportextension `{}` changes the run and extends the request page: {why} (not supported yet)",
                ws.objects[*e].name
            ));
        }
    }
    out
}

/// The first field control of reportextension `e`'s request page that is bound to none of what a reportextension that changes
/// the run may bind (reports/extensions/W3RpExtRequestPage.al:325-490): its own variable (`own`), a variable of the base report
/// `base`, a field of one of the report's data items `items`, or a field of its own record variable (W3RpExtRequestPage.al:1250
/// Run_OwnRecordControls_ReachOnBeforePreDataItemAndPreReport, corpus PR 1202, al-runner issue 6924).
fn foreign_request_control<'a>(
    request: &'a crate::pages::Layout,
    e: usize,
    own: &HashSet<String>,
    base: &ast::Object,
    items: &[crate::reports::RawItem],
) -> Option<&'a crate::pages::RawControl> {
    let base_vars: HashSet<String> = base
        .var_decls()
        .flat_map(|v| v.names())
        .map(|n| n.to_lowercase())
        .collect();
    request.controls.iter().find(|c| {
        let source = c.section.args_raw().get(1).cloned();
        let named = |names: &HashSet<String>| {
            source
                .as_deref()
                .and_then(crate::pages::plain_name)
                .is_some_and(|n| names.contains(&n.to_lowercase()))
        };
        let item_field = source
            .as_deref()
            .and_then(crate::pages::var_and_field)
            .is_some_and(|(i, _)| items.iter().any(|x| x.name.eq_ignore_ascii_case(&i)));
        let own_record_field = source
            .as_deref()
            .and_then(crate::pages::var_and_field)
            .is_some_and(|(v, _)| own.contains(&v.to_lowercase()));
        c.owner == Some(e)
            && !own_record_field
            && c.kind == ir::ControlKind::Field
            && !c.section.name().eq_ignore_ascii_case("label")
            && !named(own)
            && !named(&base_vars)
            && !item_field
    })
}
