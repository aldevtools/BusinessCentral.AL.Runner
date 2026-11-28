//! A RecordRef in the place of the record of the static run calls (al-runner issue 9280).
//!
//! These builtins declare a record, and the AL compiler converts no RecordRef to one. From runtime 17.0 it reports
//! AL0133 at the argument (must-not-compile 1634-page-run-recordref.al:1-3; 3862-3868 on corpus pull request 1645,
//! measured with the AL compilers of BC 27, 28 and 29). Below 17.0, for `Page.Run`, `Page.RunModal` and `Report.Run`,
//! it reports AL0666 at the call, for the overloads by name of runtime 17.0 it tries instead
//! (1633-page-run-recordref-bc27.al:1-3, at runtime 16.1; corpus pull request 1645 measured it at every runtime from
//! 4.0 to 16.1 with the compilers of BC 28 and 29). A Variant that holds the RecordRef compiles and runs
//! (tests/al-language/pages/lt-ce-page-run/CePageRun.al).
//!
//! Judged only in the shapes the cases show: the page, report, xmlport or codeunit given by an Integer (`Page::"X"`),
//! the record a RecordRef variable or parameter in scope (a local, a parameter, a `var` parameter or a global).
use super::*;

/// One call the cases pin: receiver, method, argument count, the place of the record (0-based), the parameter as the
/// AL compiler names it, and whether AL0666 below runtime 17.0 is measured for it (1633) or the call is judged from
/// 17.0 only (3865-3868).
struct RecordArg {
    recv: &'static str,
    method: &'static str,
    argc: usize,
    at: usize,
    param: &'static str,
    below_17_measured: bool,
}

const fn arg(
    recv: &'static str,
    method: &'static str,
    argc: usize,
    at: usize,
    param: &'static str,
    below: bool,
) -> RecordArg {
    RecordArg {
        recv,
        method,
        argc,
        at,
        param,
        below_17_measured: below,
    }
}

const RECORD_ARGS: &[RecordArg] = &[
    // 1633 and 1634, 3862-3864.
    arg("Page", "Run", 2, 1, "Table", true),
    arg("Page", "RunModal", 2, 1, "Table", true),
    arg("Report", "Run", 4, 3, "var Table", true),
    // 3865-3868.
    arg("Page", "Run", 3, 1, "Table", false),
    arg("Report", "RunModal", 4, 3, "var Table", false),
    arg("Xmlport", "Run", 4, 3, "var Table", false),
    arg("Codeunit", "Run", 2, 1, "var Table", false),
];

/// The lowest runtime at which an app that depends on the Base Application compiles: at 4.0 the AL compilers report
/// `'dependencies.id' is not available in runtime version '4.0'` for its app.json (corpus pull request 1645).
const LOWEST_COMPILING: al_version::Runtime = al_version::Runtime { major: 4, minor: 1 };

/// The lowest runtime 3865-3868 are measured at: 17.0 (the AL compiler of BC 28) and 18.0 (BC 29). The BC 27 compiler
/// gave AL0133 at 16.0 too, but its corpus leg runs at 16.1, which no measurement covers.
const OTHERS_FROM: al_version::Runtime = al_version::Runtime { major: 17, minor: 0 };

impl Checker<'_> {
    /// AL0133 or AL0666 for the builtin call `node`, resolved to `id`, with `args` a RecordRef in the place of the
    /// record; `declared` is the runtime the app declares. True when it reported.
    pub(crate) fn al_recordref_as_record(
        &mut self,
        sc: &Scope,
        node: &SyntaxNode,
        id: &str,
        args: &[Ty],
        declared: Option<al_version::Runtime>,
    ) -> bool {
        let Some((recv, method)) = id.split_once('#').and_then(|(q, _)| q.split_once('.')) else {
            return false;
        };
        let Some(call) = RECORD_ARGS
            .iter()
            .find(|c| c.recv == recv && c.method == method && c.argc == args.len())
        else {
            return false;
        };
        // Page.Run(Page, Record, FieldNo): the field number an Integer (3865).
        let others_integer = args
            .iter()
            .enumerate()
            .filter(|(i, _)| *i == 0 || (call.argc == 3 && *i == 2))
            .all(|(_, t)| matches!(t, Ty::Integer));
        let is_recordref = matches!(&args[call.at], Ty::Builtin(n) if n.eq_ignore_ascii_case("RecordRef"));
        let arg = (node.kind() == SyntaxKind::CallExpr)
            .then(|| node.children().nth(1))
            .flatten()
            .and_then(|list| list.children().nth(call.at))
            .filter(|a| a.kind() == SyntaxKind::NameRef && sc_has_var(sc, &name_of(a).to_lowercase()));
        let Some(arg) = arg.filter(|_| others_integer && is_recordref && !self.in_dependency(sc.file)) else {
            return false;
        };
        // The app's runtime: the one it declares, else the catalog's; neither known is the newest compiler's, AL0133.
        let at = declared.or(self.catalog.runtime());
        // The runtime of the overloads by name the AL compiler tries instead (1633-page-run-recordref-bc27.al:5): those
        // whose first parameter is the object's name and that take this many arguments, not any tagged overload.
        let since = self
            .catalog
            .overloads(recv, method)
            .into_iter()
            .chain(self.catalog.hidden(recv, method))
            .filter(|s| {
                let required = s.params.iter().filter(|p| !p.optional).count();
                s.params.first().is_some_and(|p| p.name == "FullyQualifiedName")
                    && required <= call.argc
                    && call.argc <= s.params.len()
            })
            .filter_map(|s| s.since)
            .min();
        let below = |at: al_version::Runtime| since.is_some_and(|s| at < s);
        match at {
            // An app.json at this runtime does not compile, whatever the code says: not measured.
            Some(at) if call.below_17_measured && at < LOWEST_COMPILING => {
                let why = "takes a RecordRef as its record, which the AL compiler rejects";
                self.diag(sc, node, "AR1007", super::calls::below_floor(method, at, why));
            }
            Some(at) if call.below_17_measured && below(at) => self.diag(
                sc,
                node,
                "AL0666",
                format!(
                    "'{method}' is not available in runtime version '{at}'. The supported runtime versions are: '{}' or greater.",
                    since.expect("below a tagged runtime")
                ),
            ),
            // 3865-3868 are measured from runtime 17.0 only: below it the call compiles as before.
            Some(at) if !call.below_17_measured && at < OTHERS_FROM => return false,
            _ => self.diag(
                sc,
                &arg,
                "AL0133",
                format!("Argument {}: cannot convert from 'RecordRef' to '{}'", call.at + 1, call.param),
            ),
        }
        true
    }
}
