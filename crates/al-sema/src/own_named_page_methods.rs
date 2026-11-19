//! A page method named like one of the page's own procedures: which of the two a site names (the checker) and which
//! of those sites the lowering runs. The one owner of that rule (al-runner issue 8452) for `SaveRecord` (pinned) and
//! `Caption` (assumption `currpage-caption-own-procedure`).

use al_builtins::assumptions::Assumption;
use al_syntax::ast;
use al_syntax::{SyntaxKind, SyntaxNode};

use crate::workspace::{ObjKind, Workspace};

// ---- SaveRecord ----

/// Whether page `page` declares an internal `SaveRecord()` without parameters: the procedure of corpus PR 1349 (issue
/// 7887) that meets the platform method.
fn has_internal_parameterless_save_record(ws: &Workspace, page: usize) -> bool {
    ws.object_ast(page).routines().any(|r| {
        !r.is_trigger() && r.name().eq_ignore_ascii_case("SaveRecord") && r.is_internal() && r.params().next().is_none()
    })
}

/// `CurrPage.SaveRecord()` without arguments in the code of page `page` itself names the platform method, not the
/// page's internal parameterless procedure of that name; through `CurrPage.Part.Page` the procedure is called
/// (pages/lifecycle/AutoSplitSaveRecord.al). `call` is the call node, `currpage` whether its receiver is `CurrPage`.
pub(crate) fn save_record_names_platform(
    ws: &Workspace,
    page: usize,
    code_obj: usize,
    member: &str,
    call: &SyntaxNode,
    currpage: bool,
) -> bool {
    page == code_obj
        && member.eq_ignore_ascii_case("SaveRecord")
        && currpage
        && call_args(call) == Some(0)
        && has_internal_parameterless_save_record(ws, page)
}

/// The lowering of builtin `id` called in the code of object `code_obj`: `CurrPage.SaveRecord()` beside the internal
/// procedure calls the platform method on a ListPart (pages/lifecycle/TestCurrPageSaveRecordName.al:13-37, page
/// TestCurrPageSaveRecordName1Fixture51744.al:9,28,53). Other page types are not measured: refused.
pub(crate) fn save_record_refusal(
    ws: &Workspace,
    code_obj: usize,
    id: &str,
    currpage: bool,
    args: usize,
) -> Result<(), String> {
    if !(id == "Page.SaveRecord#1"
        && ws.objects[code_obj].kind == ObjKind::Page
        && currpage
        && args == 0
        && has_internal_parameterless_save_record(ws, code_obj))
    {
        return Ok(());
    }
    if page_type(ws, code_obj).eq_ignore_ascii_case("ListPart") {
        return Ok(());
    }
    Err(
        "CurrPage.SaveRecord() on a page other than a ListPart that declares an internal SaveRecord \
         procedure (which one BC calls is pinned on a ListPart only)"
            .to_string(),
    )
}

// ---- Caption ----
//
// BC 29.0 (local run of corpus PR 1456, pages/lifecycle/CurrPageCaptionOwn*.al): beside page procedures named Caption,
// every `CurrPage.Caption` form is the platform method; a bare or page-variable call with one argument is the platform
// setter; a call without arguments names the own parameterless procedure the code sees, and the platform getter where it
// sees none. Compile facts: must-not-compile 3210-3215 of that PR.

/// How a site names `Caption`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum CaptionReceiver {
    CurrPage,
    /// A variable of the page type (`P.Caption`).
    PageVariable,
    /// `Caption` without a receiver in the page's or a page extension's code.
    Bare,
}

/// A procedure named Caption of a page or of one of its page extensions.
pub(crate) struct OwnCaption {
    /// The page or page extension that declares it.
    pub declared_in: usize,
    routine: ast::Routine,
}

impl OwnCaption {
    fn parameterless(&self) -> bool {
        self.routine.params().next().is_none()
    }
}

/// The procedures named Caption of page `page` and of its page extensions.
pub(crate) fn own_captions(ws: &Workspace, page: usize) -> Vec<OwnCaption> {
    std::iter::once(page)
        .chain(ws.extensions_of(page).iter().copied())
        .flat_map(|o| {
            ws.object_ast(o)
                .routines()
                .filter(|r| !r.is_trigger() && r.name().eq_ignore_ascii_case("Caption"))
                .map(move |routine| OwnCaption {
                    declared_in: o,
                    routine,
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Whether a `Caption` site names the platform method `Page.Caption` rather than one of the page's procedures `own`
/// (not empty). `call_args` is the argument count of a call, None for a form without parentheses; `code_obj` the
/// object whose code holds the site. A form without parentheses, or a call with two or more arguments, of a bare name
/// or a page variable keeps the procedure lookup it had before (the lowering's gate refuses it unless it names a
/// parameterless own procedure).
pub(crate) fn caption_names_platform(
    ws: &Workspace,
    receiver: CaptionReceiver,
    call_args: Option<usize>,
    own: &[OwnCaption],
    code_obj: usize,
) -> bool {
    match (receiver, call_args) {
        (CaptionReceiver::CurrPage, _) => true,
        (_, None) => false,
        (_, Some(0)) => !own.iter().any(|o| o.parameterless() && sees(ws, receiver, code_obj, o)),
        (_, Some(1)) => true,
        (_, Some(_)) => false,
    }
}

/// Whether code of `code_obj` naming `Caption` through `receiver` sees procedure `o`: a bare name sees the procedures
/// of its own object and, in a page extension, of the extended page (CurrPageCaptionOwnExtension.al:176-219: the base
/// page's bare `Caption()` reads the platform caption beside a procedure only its extension declares).
fn sees(ws: &Workspace, receiver: CaptionReceiver, code_obj: usize, o: &OwnCaption) -> bool {
    match receiver {
        CaptionReceiver::Bare => o.declared_in == code_obj || ws.extension_target(code_obj) == Some(o.declared_in),
        _ => true,
    }
}

/// The argument count of call `node`, or of the call whose callee `node` is; None for a form without parentheses.
pub(crate) fn call_args(node: &SyntaxNode) -> Option<usize> {
    let call = match node.kind() {
        SyntaxKind::CallExpr => node.clone(),
        _ => node
            .parent()
            .filter(|p| p.kind() == SyntaxKind::CallExpr && p.first_child().as_ref() == Some(node))?,
    };
    Some(
        call.children()
            .find(|n| n.kind() == SyntaxKind::ArgList)
            .map_or(0, |a| a.children().count()),
    )
}

/// What the lowering does with a `Caption` site.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum CaptionGate {
    /// The resolution main gave: no procedure named Caption, or a parameterless own procedure named by a bare name or
    /// a page variable without arguments (`Caption()`, `Caption`, `P.Caption()`, `P.Caption`), the forms BC calls it
    /// in (CurrPageCaptionOwnPublic.al:385, CurrPageCaptionOwnAccess.al:390, CurrPageCaptionOwnExtension.al:176,222).
    Keep,
    /// A form BC 29.0 measured: runs through the assumption.
    Assume(Assumption),
    Refuse(String),
}

/// The lowering's gate for `Caption` named through `receiver` at `node` (the call, or the member or name read or
/// assigned) in the code of `code_obj`, on page `page`. `platform` is whether the checker named the platform method;
/// `proc` the procedure it named instead, as (declaring object, offset of its routine node).
pub(crate) fn caption_gate(
    ws: &Workspace,
    code_obj: usize,
    page: usize,
    receiver: CaptionReceiver,
    node: &SyntaxNode,
    platform: bool,
    proc: Option<(usize, usize)>,
) -> CaptionGate {
    let own = own_captions(ws, page);
    if own.is_empty() {
        return CaptionGate::Keep;
    }
    let unseen_proc = receiver == CaptionReceiver::Bare
        && proc.is_some_and(|(d, _)| {
            !own.iter()
                .any(|o| o.declared_in == d && sees(ws, receiver, code_obj, o))
        });
    if unseen_proc {
        return CaptionGate::Refuse(refusal(node));
    }
    if !(receiver == CaptionReceiver::CurrPage || platform) {
        return if names_parameterless_own_without_argument(&own, proc, node) {
            CaptionGate::Keep
        } else {
            CaptionGate::Refuse(refusal(node))
        };
    }
    if measured(ws, code_obj, page, receiver, node, &own) {
        CaptionGate::Assume(Assumption::CurrpageCaptionOwnProcedure)
    } else {
        CaptionGate::Refuse(refusal(node))
    }
}

/// Whether `proc` is a parameterless procedure of `own` and the site at `node` passes no argument: a call without
/// arguments, or a name read without parentheses that is not the target of an assignment (which alc reads as a call
/// with one argument, record/ProbePropertyAssignmentForms.al:375-377).
fn names_parameterless_own_without_argument(
    own: &[OwnCaption],
    proc: Option<(usize, usize)>,
    node: &SyntaxNode,
) -> bool {
    let Some((object, offset)) = proc else { return false };
    let parameterless_own = own.iter().any(|o| {
        o.declared_in == object && usize::from(o.routine.syntax().text_range().start()) == offset && o.parameterless()
    });
    let assign_target = node
        .parent()
        .is_some_and(|p| p.kind() == SyntaxKind::AssignStmt && p.first_child().as_ref() == Some(node));
    let no_argument = match call_args(node) {
        Some(n) => n == 0,
        None => !assign_target,
    };
    parameterless_own && no_argument
}

fn refusal(node: &SyntaxNode) -> String {
    format!(
        "`{}` beside a page procedure named Caption, in a form BC was not measured on (assumption \
         currpage-caption-own-procedure covers Card and List pages, one procedure of a measured signature, the triggers \
         the probe used)",
        node.text().to_string().trim()
    )
}

/// The signatures of the own procedure the probe measured (CurrPageCaptionOwnPublic.al:176,256,
/// CurrPageCaptionOwnAccess.al:101,181,261, CurrPageCaptionOwnSignature.al:101,181,260).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum OwnShape {
    PublicText,
    LocalText,
    ProtectedText,
    InternalText,
    PublicInteger,
    PublicNoReturn,
    LocalNoReturn,
    TextParameter,
}

fn own_shape(r: &ast::Routine) -> Option<OwnShape> {
    if r.name() != "Caption" || r.attributes().next().is_some() || r.return_name().is_some() {
        return None;
    }
    let access = (r.is_local(), r.is_protected(), r.is_internal());
    let params: Vec<ast::Param> = r.params().collect();
    let ret = r.return_type().map(|t| t.text());
    let text_param =
        params.len() == 1 && !params[0].is_var() && params[0].type_ref().is_some_and(|t| t.text() == "Text");
    match (access, params.is_empty(), ret.as_deref()) {
        ((false, false, false), true, Some("Text")) => Some(OwnShape::PublicText),
        ((true, false, false), true, Some("Text")) => Some(OwnShape::LocalText),
        ((false, true, false), true, Some("Text")) => Some(OwnShape::ProtectedText),
        ((false, false, true), true, Some("Text")) => Some(OwnShape::InternalText),
        ((false, false, false), true, Some("Integer")) => Some(OwnShape::PublicInteger),
        ((false, false, false), true, None) => Some(OwnShape::PublicNoReturn),
        ((true, false, false), true, None) => Some(OwnShape::LocalNoReturn),
        ((false, false, false), false, None) if text_param => Some(OwnShape::TextParameter),
        _ => None,
    }
}

/// How the site uses the member.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Use {
    /// A value read: `CurrPage.Caption` or `Caption()` in an expression.
    Read {
        parens: bool,
    },
    /// The target of a plain `:=`.
    Assign,
    /// A call statement with one argument.
    SetCall,
    Other,
}

fn site_use(node: &SyntaxNode) -> Use {
    let parent = node.parent();
    let parent_kind = parent.as_ref().map(|p| p.kind());
    match node.kind() {
        SyntaxKind::CallExpr => match (call_args(node), parent_kind) {
            (Some(0), Some(k)) if k != SyntaxKind::ExprStmt => Use::Read { parens: true },
            (Some(1), Some(SyntaxKind::ExprStmt)) => Use::SetCall,
            _ => Use::Other,
        },
        SyntaxKind::MemberExpr | SyntaxKind::NameRef => match parent {
            Some(p) if p.kind() == SyntaxKind::AssignStmt && p.first_child().as_ref() == Some(node) => {
                let plain = p.children_with_tokens().any(|t| t.kind() == SyntaxKind::Assign);
                if plain { Use::Assign } else { Use::Other }
            }
            Some(p) if p.kind() == SyntaxKind::ExprStmt || p.kind() == SyntaxKind::CallExpr => Use::Other,
            Some(_) => Use::Read { parens: false },
            None => Use::Other,
        },
        _ => Use::Other,
    }
}

/// Whether the receiver and member at `node` are spelled as the probe wrote them: `CurrPage`, `Caption`, unquoted.
fn spelled_as_measured(node: &SyntaxNode, receiver: CaptionReceiver) -> bool {
    let callee = match node.kind() {
        SyntaxKind::CallExpr => node.first_child(),
        _ => Some(node.clone()),
    };
    let Some(callee) = callee else { return false };
    let text = |n: &SyntaxNode| n.text().to_string().trim().to_string();
    match callee.kind() {
        SyntaxKind::NameRef => text(&callee) == "Caption",
        SyntaxKind::MemberExpr => {
            let mut kids = callee.children();
            let (Some(recv), Some(member)) = (kids.next(), kids.next()) else {
                return false;
            };
            text(&member) == "Caption" && (receiver != CaptionReceiver::CurrPage || text(&recv) == "CurrPage")
        }
        _ => false,
    }
}

/// The name of the trigger whose code holds `node`.
fn trigger_of(node: &SyntaxNode) -> Option<String> {
    let r = node.ancestors().find_map(ast::Routine::cast)?;
    r.is_trigger().then(|| r.name())
}

/// Whether `node` is in a `[Test]` procedure of a codeunit with `Subtype = Test`.
fn in_test_method(ws: &Workspace, code_obj: usize, node: &SyntaxNode) -> bool {
    ws.objects[code_obj].kind == ObjKind::Codeunit
        && ws
            .object_ast(code_obj)
            .properties()
            .any(|p| p.name() == "Subtype" && p.value_text().trim() == "Test")
        && node
            .ancestors()
            .find_map(ast::Routine::cast)
            .is_some_and(|r| !r.is_trigger() && r.attributes().any(|a| a.name() == "Test"))
}

fn page_type(ws: &Workspace, page: usize) -> String {
    ws.object_ast(page)
        .properties()
        .find(|p| p.name().eq_ignore_ascii_case("PageType"))
        .map(|p| p.value_text().trim().to_string())
        .unwrap_or_default()
}

/// The cells BC 29.0 measured (corpus PR 1456). Card pages, code in the page and the procedure in the page: the
/// reads in OnOpenPage (both forms) and OnAction (without parentheses), `:=` in OnOpenPage, `CurrPage.Caption('X')` and
/// the bare `Caption('X')` in OnAction, for every measured signature (CurrPageCaptionOwnPublic.al:307,346,
/// CurrPageCaptionOwnAccess.al:273,312,351, CurrPageCaptionOwnSignature.al:272,311,350). A public `Caption(): Text`
/// split between the page and an extension: the reads and `:=` in OnOpenPage, and the base page's bare `Caption()`
/// in OnOpenPage beside the extension's procedure (CurrPageCaptionOwnExtension.al:176,199,222). A List page with a
/// public `Caption(): Text`: the reads in OnOpenPage, `:=` in OnAfterGetCurrRecord (CurrPageCaptionOwnList.al:74).
/// A page variable in a test method: `P.Caption('X')` beside a public `Caption(): Text` or `Caption(Text)`,
/// `P.Caption()` beside `Caption(Text)` (CurrPageCaptionOwnPublic.al:385, CurrPageCaptionOwnSignature.al:389).
fn measured(
    ws: &Workspace,
    code_obj: usize,
    page: usize,
    receiver: CaptionReceiver,
    node: &SyntaxNode,
    own: &[OwnCaption],
) -> bool {
    use CaptionReceiver::*;
    let [one] = own else { return false };
    let Some(shape) = own_shape(&one.routine) else {
        return false;
    };
    if !spelled_as_measured(node, receiver) {
        return false;
    }
    let page_type = page_type(ws, page);
    let own_in_page = one.declared_in == page;
    let code_kind = ws.objects[code_obj].kind;
    let code_in_page = code_obj == page;
    let code_in_extension = code_kind == ObjKind::PageExtension && ws.extension_target(code_obj) == Some(page);
    let site = site_use(node);
    let trigger = trigger_of(node);
    let t = trigger.as_deref();
    let public_text = shape == OwnShape::PublicText;
    match (receiver, page_type.as_str()) {
        (CurrPage, "Card") if own_in_page && code_in_page => matches!(
            (site, t),
            (Use::Read { .. } | Use::Assign, Some("OnOpenPage"))
                | (Use::Read { parens: false } | Use::SetCall, Some("OnAction"))
        ),
        (CurrPage, "Card") => {
            public_text
                && (code_in_page || (code_in_extension && (own_in_page || one.declared_in == code_obj)))
                && matches!((site, t), (Use::Read { .. } | Use::Assign, Some("OnOpenPage")))
        }
        (CurrPage, "List") => {
            public_text
                && own_in_page
                && code_in_page
                && matches!(
                    (site, t),
                    (Use::Read { .. }, Some("OnOpenPage")) | (Use::Assign, Some("OnAfterGetCurrRecord"))
                )
        }
        (Bare, "Card") if own_in_page && code_in_page => matches!((site, t), (Use::SetCall, Some("OnAction"))),
        (Bare, "Card") => {
            public_text && code_in_page && matches!((site, t), (Use::Read { parens: true }, Some("OnOpenPage")))
        }
        (PageVariable, "Card") => {
            own_in_page
                && in_test_method(ws, code_obj, node)
                && match site {
                    Use::SetCall => matches!(shape, OwnShape::PublicText | OwnShape::TextParameter),
                    Use::Read { parens: true } => shape == OwnShape::TextParameter,
                    _ => false,
                }
        }
        _ => false,
    }
}

/// A report request page whose report (or report extension) declares a procedure named Caption: `CurrPage.Caption`
/// and a bare `Caption(..)` with arguments there were not measured, so the lowering refuses them.
pub(crate) fn request_page_caption_refusal(
    ws: &Workspace,
    code_obj: usize,
    node: &SyntaxNode,
    currpage: bool,
) -> Result<(), String> {
    let report_code = matches!(ws.objects[code_obj].kind, ObjKind::Report | ObjKind::ReportExtension);
    if !(report_code && crate::check::in_request_page(node)) {
        return Ok(());
    }
    let bare_with_args = !currpage && call_args(node).is_some_and(|n| n > 0);
    if !(currpage || bare_with_args) {
        return Ok(());
    }
    let declares = std::iter::once(code_obj).chain(ws.extension_target(code_obj)).any(|o| {
        ws.object_ast(o)
            .syntax()
            .descendants()
            .filter_map(ast::Routine::cast)
            .any(|r| !r.is_trigger() && r.name().eq_ignore_ascii_case("Caption"))
    });
    if declares { Err(refusal(node)) } else { Ok(()) }
}
