//! A RecordRef in the place of the record of `Page.Run`, `Page.RunModal` and `Report.Run` (al-runner issue 9280).
//!
//! These builtins declare a record, and the AL compiler converts no RecordRef to one. From runtime 17.0 it reports
//! AL0133 at the argument (must-not-compile 1634-page-run-recordref.al:1-3, at runtime 17.1 and 18.0); below it, AL0666
//! at the call, for the overloads by name of runtime 17.0 it tries instead (1633-page-run-recordref-bc27.al:1-3, at
//! runtime 16.1). A Variant that holds the RecordRef compiles and runs (tests/al-language/pages/lt-ce-page-run/
//! CePageRun.al). Judged only in the shape the cases show: the call with the record as its last argument, two
//! arguments for a page and four for a report, the record a RecordRef variable or parameter.
use super::*;

/// The receiver, method, argument count and record place of each call the cases pin, with the parameter as the AL
/// compiler names it (1634-page-run-recordref.al:1-3).
const RECORD_ARGS: &[(&str, &str, usize, &str)] = &[
    ("Page", "Run", 2, "Table"),
    ("Page", "RunModal", 2, "Table"),
    ("Report", "Run", 4, "var Table"),
];

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
        let Some(&(recv, method, argc, param)) = RECORD_ARGS
            .iter()
            .find(|(r, m, n, _)| *r == recv && *m == method && *n == args.len())
        else {
            return false;
        };
        let is_recordref = matches!(args.last(), Some(Ty::Builtin(n)) if n.eq_ignore_ascii_case("RecordRef"));
        let arg = (node.kind() == SyntaxKind::CallExpr)
            .then(|| node.children().nth(1))
            .flatten()
            .and_then(|list| list.children().nth(argc - 1))
            .filter(|a| a.kind() == SyntaxKind::NameRef);
        let Some(arg) = arg.filter(|_| is_recordref && !self.in_dependency(sc.file)) else {
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
                    && required <= argc
                    && argc <= s.params.len()
            })
            .filter_map(|s| s.since)
            .min();
        match (at, since) {
            (Some(at), _) if at < al_version::Runtime::floor() => {
                let why = "takes a RecordRef as its record, which the AL compiler rejects";
                self.diag(sc, node, "AR1007", super::calls::below_floor(method, at, why));
            }
            (Some(at), Some(since)) if at < since => self.diag(
                sc,
                node,
                "AL0666",
                format!(
                    "'{method}' is not available in runtime version '{at}'. The supported runtime versions are: '{since}' or greater."
                ),
            ),
            _ => self.diag(
                sc,
                &arg,
                "AL0133",
                format!("Argument {argc}: cannot convert from 'RecordRef' to '{param}'"),
            ),
        }
        true
    }
}
