//! Page and report extension code that names a member of its base object (ADR-023).
//!
//! The mechanism is built (checker scope, `Place::BaseGlobal`, `Expr::BaseInstance`, the VM link),
//! but what real BC does is pinned only by probes (ADR-023, "Probes"). Each probe turns one kind
//! of access on: its id goes into [`PINNED`] in the commit that cites the corpus probe test. Until
//! then the lowering refuses the access with a reason that names the probe.
//!
//! For measurement and development only, the environment variable `AL_UNPINNED_BASE_ACCESS`
//! switches the checker scope on and pins cases without a probe: `1` or `all` pins every case,
//! any other value is a comma-separated list of probe ids to pin (`none` pins nothing). A run with
//! it set is not a conformance result.

use al_syntax::ast;

/// Probe ids whose behavior a corpus test pins. The ids are the ones of ADR-023: `P1` to `P10` for
/// pages, `R1` to `R4` for reports. Pinned on BC 27.5 by the corpus (tests/al-language/...):
///
/// - P1, P2, P5: handlers/TestPageExtensionBaseAccessContracts.al:275 (reads), :288 (writes seen
///   by the base and a second extension, also Record filters, List, Dictionary).
/// - P4, P10: the same file :310 (public and protected base procedures, return value, var
///   parameter, changing a variable, CurrPage.Update/Editable/Caption).
/// - P9: the same file :326 (control sources naming a base variable or a field of a base Record).
/// - R1, R3: reportextensiontrigger/TestReportExtensionBaseMembersContracts.al:256 and its header
///   comment (lines 9-15: protected Integer, Decimal, Record, Dialog, List, protected and public
///   procedure; Window.Open/Update/Close).
///
/// P3 and R4 (several extensions share the base's variables) are in the same files. P6 is settled
/// without a gate: an extension variable named like a base variable is a separate variable (line
/// 20 of the page file, `Shadow`); an extension procedure named like a base procedure is compile
/// error AL0440, a non-protected base variable or a local base procedure AL0161; both are refused
/// here because BC never runs them.
pub const PINNED: &[&str] = &["P1", "P2", "P4", "P5", "P9", "P10", "R1", "R3"];

/// The value of `AL_UNPINNED_BASE_ACCESS`, if set.
fn switch() -> Option<String> {
    std::env::var("AL_UNPINNED_BASE_ACCESS").ok()
}

/// Whether the behavior of probe `id` is pinned (or the measurement switch pins it).
pub fn pinned(id: &str) -> bool {
    match switch() {
        // The switch, when set, decides alone (`none` pins nothing: for the tests of the refusals).
        Some(v) => v == "1" || v.eq_ignore_ascii_case("all") || v.split(',').any(|x| x.trim().eq_ignore_ascii_case(id)),
        None => PINNED.contains(&id),
    }
}

/// Whether the checker puts the base object's members into an extension's scope. Off while
/// nothing is pinned, so that the compile reaches exactly what it did before.
pub fn scope_enabled() -> bool {
    !PINNED.is_empty() || switch().is_some()
}

/// The refusal for an access whose probe has not confirmed it.
pub fn unpinned_reason(what: &str, probe: &str, noun: &str) -> String {
    format!(
        "{what} is a member of the {noun} it extends: {noun} extension code on base {noun} members is not lowered yet \
         (not pinned: ADR-023 probe {probe})"
    )
}

/// The variables of the base object that extension code may name: those of its `protected var`
/// sections, as (lower-case name, declaration).
pub fn visible_vars(base: &ast::Object) -> Vec<(String, ast::VarDecl)> {
    base.protected_var_decls()
        .flat_map(|d| d.names().into_iter().map(move |n| (n.to_lowercase(), d.clone())))
        .collect()
}

/// Whether the base object has a variable `name` (any section), case-insensitive.
pub fn declares_var(base: &ast::Object, name: &str) -> bool {
    base.var_decls()
        .any(|v| v.names().iter().any(|x| x.eq_ignore_ascii_case(name)))
}

/// The routine `name` of the base object that extension code may call: a procedure that is not
/// `local` (public and `protected` ones work, pages and reports alike).
pub fn visible_procs<'a>(base: &'a ast::Object, name: &'a str) -> impl Iterator<Item = ast::Routine> + 'a {
    // A routine `pages::expand` generated for a control property is not an AL method of the base: an extension's own
    // generated routine may carry the same name (al-runner issue 6012), and AL code never calls one.
    base.routines().filter(move |r| {
        let n = r.name();
        n.eq_ignore_ascii_case(name) && !r.is_local() && !crate::pages::is_synthetic(&n)
    })
}

/// The slot of variable `name` among the globals of the base object, from its syntax only (the
/// lowered base object is not needed, so the two lower independently). A page lays out its
/// variables in declaration order. A report lays out `items` (the number of distinct data item
/// names), then one slot for the request-page flag, then its variables.
pub fn slot_of(base: &ast::Object, report_items: Option<usize>, name: &str) -> Option<usize> {
    let before = report_items.map_or(0, |n| n + 1);
    base.var_decls()
        .flat_map(|v| v.names())
        .position(|n| n.eq_ignore_ascii_case(name))
        .map(|i| before + i)
}
