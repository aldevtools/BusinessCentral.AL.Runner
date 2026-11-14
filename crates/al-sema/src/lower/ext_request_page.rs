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
use al_builtins::assumptions::Assumption;
use al_ir as ir;
use al_syntax::SyntaxKind;
use al_syntax::ast;
use std::collections::HashSet;

/// The order the request page of a report takes from its two run-changing extensions, in the shape
/// reports/extensions/IndependentAppsReportExtensions.al pins (al-runner issue 7358).
pub(crate) struct PinnedRequestPage {
    /// The order the extensions apply to the request page: the run order (`ext_order`), app B before app D. Their triggers run
    /// in it after the report's own (IndependentAppsReportExtensions.al:108, :138 `A:rp open;B:rp open;D:rp open;A:rp close;
    /// B:rp close;D:rp close;`), and the parameters text lists their fields in it after the report's (:109, :139:
    /// BaseOptionValue, BFieldValue, DFieldValue; `request_parameters` writes the base's controls, then each extension's in
    /// this order).
    pub(crate) order: Vec<usize>,
    /// The unprobed assumptions the run order rests on (`ext_order`).
    pub(crate) assumes: Vec<Assumption>,
}

/// `Some` only for the request page of report `base` (written in `file`) that the corpus pins with triggers on its
/// extensions' request pages; each fact is required by equality:
/// - the report's request page states no SaveValues (al-language-xapp-a/R7358XaIndependentReports.al:88-111);
/// - its request page extensions `request_exts` are its run-changing extensions `changing`, and there are two
///   (R7358XbIndependentReportExts.al:3, R7358XdIndependentReportExts.al:3);
/// - they are of two apps with no dependency path between them (`ExtOrder::independent_pair`; xapp-b and xapp-d app.json);
/// - each extension's request page is the measured one (`extension_anchor`), and both add after the same field of the
///   report's request page (R7358XaIndependentReports.al:94, R7358XbIndependentReportExts.al:105, R7358XdIndependentReportExts.al:105).
pub(crate) fn pinned_request_page(
    ws: &Workspace,
    base: &ast::Object,
    request_exts: &[usize],
    changing: &[usize],
) -> Option<PinnedRequestPage> {
    let base_request = base.sections().find(|s| s.name().eq_ignore_ascii_case("requestpage"))?;
    if save_values(Some(&base_request)) {
        return None;
    }
    let (mut a, mut b) = (request_exts.to_vec(), changing.to_vec());
    a.sort_unstable();
    b.sort_unstable();
    if a != b || a.len() != 2 {
        return None;
    }
    let order = super::ext_order::report_extension_order(ws, changing).ok()?;
    if !order.independent_pair {
        return None;
    }
    let anchors: Vec<String> = order
        .exts
        .iter()
        .map(|&e| extension_anchor(ws, e))
        .collect::<Option<_>>()?;
    if anchors[0] != anchors[1] || !has_field(&base_request, &anchors[0]) {
        return None;
    }
    Some(PinnedRequestPage {
        order: order.exts,
        assumes: order.assumes,
    })
}

/// The anchor (lower case) of reportextension `e`'s request page when it is exactly the measured one
/// (R7358XbIndependentReportExts.al:101-124): no property and no variable; the triggers OnOpenPage and OnClosePage and no
/// other routine, each only calling a procedure of a SingleInstance codeunit variable of the extension with one text
/// literal (`calls_single_instance_codeunit`); a `layout` with one `addafter(<field>)` that adds one field, bound to a
/// variable of the extension, stating ApplicationArea and Caption and nothing else, with no trigger.
fn extension_anchor(ws: &Workspace, e: usize) -> Option<String> {
    let ext = ws.object_ast(e);
    let mut sections = ext.sections().filter(|s| s.name().eq_ignore_ascii_case("requestpage"));
    let (Some(rp), None) = (sections.next(), sections.next()) else {
        return None;
    };
    if rp.properties().next().is_some() || rp.var_decls().next().is_some() {
        return None;
    }
    let routines: Vec<ast::Routine> = rp.triggers().collect();
    let mut names: Vec<String> = routines.iter().map(|r| r.name().to_ascii_lowercase()).collect();
    names.sort_unstable();
    if names != ["onclosepage", "onopenpage"] || !routines.iter().all(|r| r.is_trigger()) {
        return None;
    }
    if !routines.iter().all(|r| calls_single_instance_codeunit(ws, e, &ext, r)) {
        return None;
    }
    let layout = only_section(&rp, "layout")?;
    no_properties(&layout)?;
    let add = only_section(&layout, "addafter")?;
    no_properties(&add)?;
    let field = only_section(&add, "field")?;
    let [anchor] = add.args().try_into().ok()?;
    let [_, source]: [String; 2] = field.args_raw().try_into().ok()?;
    let own = crate::pages::plain_name(&source).is_some_and(|n| {
        ext.var_decls()
            .flat_map(|v| v.names())
            .any(|v| v.eq_ignore_ascii_case(&n))
    });
    let mut props: Vec<String> = field.properties().map(|p| p.name().to_ascii_lowercase()).collect();
    props.sort_unstable();
    (own && props == ["applicationarea", "caption"] && field.sections().next().is_none())
        .then(|| anchor.to_ascii_lowercase())
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

/// Whether trigger `r` of reportextension `e` has no parameter and no local variable, and its body is one statement that
/// calls a procedure of a variable of the extension whose type is a SingleInstance codeunit, with one text literal
/// (R7358XbIndependentReportExts.al:115-123 `Log.Add('B:rp open');`, the log codeunit R7358XaIndependentReports.al:14-16).
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
    let Some([var, _]) = kids(&callee, SyntaxKind::MemberExpr).and_then(|k| <[_; 2]>::try_from(k).ok()) else {
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
    ws.object_ast(cu)
        .properties()
        .any(|p| p.name().eq_ignore_ascii_case("SingleInstance") && p.value_text().trim().eq_ignore_ascii_case("true"))
}

/// Whether the request page `section` of a report declares a field control named `name` (lower case).
fn has_field(section: &ast::Section, name: &str) -> bool {
    section.sections().any(|s| {
        (s.name().eq_ignore_ascii_case("field") && s.args().first().is_some_and(|a| a.eq_ignore_ascii_case(name)))
            || has_field(&s, name)
    })
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
