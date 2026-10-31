//! Language features newer (or older) than an app's runtime: AL0666 where the AL compiler reports it (al-runner issue
//! 5573, the follow-up of the builtin gate in `check/calls.rs`).
//!
//! What alc reports for each property (per element kind, and per value where a value differs), data type, method
//! attribute and object type at each runtime is measured by `tools/alc-runtimes/alc_features.py` with the corpus's AL
//! compiler (`al-builtins/catalog/alc-features.txt`). A feature is gated only where that table has a row for the same
//! element kind and alc reported AL0666 on the feature's own line; everything else compiles as before, so no code alc
//! accepts is rejected.
//!
//! The sentence is the one the must-not-compile corpus pins: `'ObsoleteState' is not available in runtime version
//! '16.1'. The supported runtime versions are: '4.0' until, but not including '13.0'.` (0799-obsoletestate-on-page-areas.al:2),
//! `'Subtype' is not available in runtime version '` (0803-layout-subtype-bc27-28.al:1),
//! `'TestHandlerContext' is not available in runtime version '16.0'. The supported runtime versions are: '18.0' or
//! greater.` (0996-testhandlercontext-runtime-16.al:1). The quoted name and the versions part are alc's own (the table).
use crate::workspace::{Diagnostic, ObjKind, Workspace};
use al_builtins::catalog::Catalog;
use al_builtins::features::{FeatureKind, al0666_at, feature};
use al_syntax::{SyntaxKind, SyntaxNode, ast};
use al_version::Runtime;

fn offset_of(n: &SyntaxNode) -> usize {
    n.text_range().start().into()
}

struct Out<'a> {
    ws: &'a Workspace,
    file: usize,
    at: Runtime,
    diags: Vec<Diagnostic>,
}

impl Out<'_> {
    fn gate(&mut self, kind: FeatureKind, ctx: &str, name: &str, value: Option<&str>, node: &SyntaxNode) {
        let Some(row) = feature(kind, ctx, name, value) else {
            return;
        };
        for (quoted, text) in al0666_at(row, self.at) {
            self.diags.push(Diagnostic {
                file: self.ws.files[self.file].path.clone(),
                app: self.ws.files[self.file].app,
                offset: offset_of(node),
                code: "AL0666",
                message: format!(
                    "'{quoted}' is not available in runtime version '{}'. The supported runtime versions are: {text}.",
                    self.at
                ),
            });
        }
    }
}

/// The AL0666 diagnostics of the apps under test (never a dependency: alc compiled it already), each at the runtime
/// its app declares, else the catalog's.
pub fn check(ws: &Workspace, catalog: &Catalog) -> Vec<Diagnostic> {
    check_from(ws, catalog, 0)
}

/// [`check`] for the objects from `first` on: each object is checked by itself (ADR-063: a compile that grows by apps
/// checks the new objects only).
pub fn check_from(ws: &Workspace, catalog: &Catalog, first: usize) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    for (oi, o) in ws.objects.iter().enumerate().skip(first) {
        if ws.is_dependency(oi) {
            continue;
        }
        let Some(app) = ws.apps.get(o.app) else { continue };
        let Some(at) = catalog.app_runtime(&app.id).or(catalog.runtime()) else {
            continue;
        };
        let mut out = Out {
            ws,
            file: o.file,
            at,
            diags: Vec::new(),
        };
        let obj = ws.object_ast(oi);
        let kind = obj.kind();
        out.gate(FeatureKind::Object, "-", &kind, None, obj.syntax());
        for_each_property(ws, oi, |ctx, p| {
            out.gate(FeatureKind::Property, ctx, &p.name(), Some(&p.value_text()), p.syntax())
        });
        // Data types as a variable declares them (the form alc was asked about).
        for n in obj.syntax().descendants().filter(|n| n.kind() == SyntaxKind::VarDecl) {
            let Some(t) = ast::VarDecl::cast(n.clone()).and_then(|d| d.type_ref()) else {
                continue;
            };
            out.gate(FeatureKind::DataType, "-", &t.type_name(), None, t.syntax());
        }
        // Method attributes, in a codeunit (the object alc was asked about).
        if o.kind == ObjKind::Codeunit {
            for r in obj.routines() {
                for a in r.attributes() {
                    out.gate(FeatureKind::Attribute, "-", &a.name(), None, a.syntax());
                }
            }
        }
        diags.extend(out.diags);
    }
    diags
}

/// Calls `f` with every property of object `oi` and its element kind: the object type, then the keywords of the
/// sections without arguments above (`layout`, `actions`), then the keyword of the innermost section with arguments
/// (`field`, `area`), and for a `modify` the kind of what it modifies (`modify:field`); the form
/// `catalog/alc-features.txt` keys its rows by. A page extension's or page customization's `modify` whose target is
/// not found is skipped with everything in it.
pub(crate) fn for_each_property(ws: &Workspace, oi: usize, mut f: impl FnMut(&str, &ast::Property)) {
    let obj = ws.object_ast(oi);
    let kind = obj.kind();
    for p in obj.properties() {
        f(&kind, &p);
    }
    for s in obj.sections() {
        section(ws, oi, &kind, &s, &mut Vec::new(), None, &mut f);
    }
}

/// The properties of `s` and of the sections in it. `structural`: the keywords of the sections without arguments
/// above (`layout`, `actions`); `element`: the keyword of the innermost section with arguments above, if any.
fn section(
    ws: &Workspace,
    oi: usize,
    kind: &str,
    s: &ast::Section,
    structural: &mut Vec<String>,
    element: Option<String>,
    f: &mut impl FnMut(&str, &ast::Property),
) {
    let name = s.name().to_ascii_lowercase();
    let has_args = !s.args().is_empty();
    let pushed = !has_args;
    if pushed {
        structural.push(name.clone());
    }
    let element = if has_args {
        Some(if name == "modify" {
            match modified_kind(ws, oi, kind, structural, s) {
                Some(k) => format!("modify:{k}"),
                // What a modify changes decides which properties it takes: unknown, nothing is gated.
                None if kind == "pageextension" || kind == "pagecustomization" => String::new(),
                None => name.clone(),
            }
        } else {
            name.clone()
        })
    } else {
        element
    };
    if element.as_deref() != Some("") {
        let ctx = std::iter::once(kind.to_string())
            .chain(structural.iter().cloned())
            .chain(element.clone())
            .collect::<Vec<_>>()
            .join("/");
        for p in s.properties() {
            f(&ctx, &p);
        }
    }
    for inner in s.sections() {
        section(ws, oi, kind, &inner, structural, element.clone(), f);
    }
    if pushed {
        structural.pop();
    }
}

/// The keyword of the control or action a page extension's or page customization's `modify(Name)` changes, found in
/// the page it extends; `None` when it cannot be found. A table or report extension's `modify` is not looked up.
fn modified_kind(ws: &Workspace, oi: usize, kind: &str, structural: &[String], s: &ast::Section) -> Option<String> {
    if kind != "pageextension" && kind != "pagecustomization" {
        return None;
    }
    let target = s.args().into_iter().next()?;
    // A page customization is no extension: its page is the one `customizes` names.
    let base = match ws.extension_target(oi) {
        Some(b) => b,
        None if kind == "pagecustomization" => {
            let parts = ws.object_ast(oi).customizes_parts()?;
            ws.lookup(Some(ws.objects[oi].app), ObjKind::Page, &parts.join("."))?
        }
        None => return None,
    };
    let top = structural.first()?;
    let page = ws.object_ast(base);
    let mut stack: Vec<ast::Section> = page.sections().filter(|x| x.name().eq_ignore_ascii_case(top)).collect();
    while let Some(x) = stack.pop() {
        for inner in x.sections() {
            if inner.args().first().is_some_and(|a| a.eq_ignore_ascii_case(&target)) {
                return Some(inner.name().to_ascii_lowercase());
            }
            stack.push(inner);
        }
    }
    None
}
