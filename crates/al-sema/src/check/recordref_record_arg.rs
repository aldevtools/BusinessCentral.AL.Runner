//! A RecordRef in the place of the record of the static run calls (al-runner issue 9280).
//!
//! These builtins declare a record, and the AL compiler converts no RecordRef to one. From runtime 17.0 it reports
//! AL0133 at the argument (must-not-compile 1634-page-run-recordref.al:1-3). Below 17.0, for `Page.Run`,
//! `Page.RunModal` and `Report.Run`, it reports AL0666 at the call, for the overloads by name of runtime 17.0 it tries
//! instead (1633-page-run-recordref-bc27.al:1-3, at runtime 16.1). A Variant that holds the RecordRef compiles and runs
//! (tests/al-language/pages/lt-ce-page-run/CePageRun.al).
//!
//! 1633 and 1634 pin a local RecordRef with those three calls. The other forms rest on the assumption
//! `recordref-as-run-record` (must-not-compile 3862-3868 on corpus pull request 1645): a parameter, `var` parameter or
//! global RecordRef; `Page.Run` with a field number, `Report.RunModal`, `Xmlport.Run` and `Codeunit.Run`; and AL0666
//! at the runtimes from 4.1 to below 17.0 other than 16.1.
//!
//! Judged only in the measured shapes: the object given by an Integer (`Page::"X"`), the record a RecordRef variable
//! or parameter in scope (a local, a parameter, a `var` parameter or a global).
use super::*;
use al_builtins::assumptions::Assumption;

/// How a call is judged below runtime 17.0.
#[derive(Clone, Copy, PartialEq)]
enum Below17 {
    /// AL0666 down to 4.1 (1633; corpus pull request 1645).
    Al0666,
    /// AL0133 from the runtime floor: `Xmlport.Run` has no overload by name, and the AL compilers give AL0133 at 16.0
    /// and 16.1 too (3867).
    Al0133,
    /// Not judged: the compilers disagree there (3865, 3866, 3868: AL0666 at 16.1 with AL 17 and 18, AL0133 at 16.0
    /// with AL 16), so the call compiles as before.
    NotJudged,
}

/// One call the cases pin: receiver, method, argument count, the place of the record (0-based), the parameter as the
/// AL compiler names it, and how it is judged below runtime 17.0.
struct RecordArg {
    recv: &'static str,
    method: &'static str,
    argc: usize,
    at: usize,
    param: &'static str,
    below_17: Below17,
}

const fn arg(
    recv: &'static str,
    method: &'static str,
    argc: usize,
    at: usize,
    param: &'static str,
    below_17: Below17,
) -> RecordArg {
    RecordArg {
        recv,
        method,
        argc,
        at,
        param,
        below_17,
    }
}

const RECORD_ARGS: &[RecordArg] = &[
    // 1633 and 1634, 3862-3864.
    arg("Page", "Run", 2, 1, "Table", Below17::Al0666),
    arg("Page", "RunModal", 2, 1, "Table", Below17::Al0666),
    arg("Report", "Run", 4, 3, "var Table", Below17::Al0666),
    // 3865-3868.
    arg("Page", "Run", 3, 1, "Table", Below17::NotJudged),
    arg("Report", "RunModal", 4, 3, "var Table", Below17::NotJudged),
    arg("Xmlport", "Run", 4, 3, "var Table", Below17::Al0133),
    arg("Codeunit", "Run", 2, 1, "var Table", Below17::NotJudged),
];

/// The lowest runtime at which an app that depends on the Base Application compiles: at 4.0 the AL compilers report
/// `'dependencies.id' is not available in runtime version '4.0'` for its app.json (corpus pull request 1645).
const LOWEST_COMPILING: al_version::Runtime = al_version::Runtime { major: 4, minor: 1 };

/// The runtime from which 3865, 3866 and 3868 are judged: 17.0, where every compiler gives AL0133.
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
        // Beyond 1633 and 1634 (a local RecordRef, the three calls at runtime 16.1, 17.1 and 18.0) the forms rest on the
        // assumption (corpus pull request 1645).
        let _rests_on = Assumption::RecordrefAsRunRecord;
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
            Some(at) if call.below_17 == Below17::Al0666 && at < LOWEST_COMPILING => {
                let why = "takes a RecordRef as its record, which the AL compiler rejects";
                self.diag(sc, node, "AR1007", super::calls::below_floor(method, at, why));
            }
            Some(at) if call.below_17 == Below17::Al0666 && below(at) => self.diag(
                sc,
                node,
                "AL0666",
                format!(
                    "'{method}' is not available in runtime version '{at}'. The supported runtime versions are: '{}' or greater.",
                    since.expect("below a tagged runtime")
                ),
            ),
            // 3867 below the runtime floor, and 3865, 3866 and 3868 below 17.0: not judged, the call compiles as before.
            Some(at) if call.below_17 == Below17::Al0133 && at < al_version::Runtime::floor() => return false,
            Some(at) if call.below_17 == Below17::NotJudged && at < OTHERS_FROM => return false,
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
