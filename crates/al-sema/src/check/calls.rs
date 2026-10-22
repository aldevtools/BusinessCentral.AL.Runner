//! Overload picking and the lookup of callables.
use super::*;

/// The reason of an AR1007: a call in an app whose declared runtime is below the runtime floor, where the table of
/// what the AL compiler reports (`al-builtins/catalog/alc-runtimes.txt`) gives no AL0666 text (al-runner issue 5573).
fn below_floor(name: &str, at: al_version::Runtime, why: &str) -> String {
    format!(
        "`{name}` {why}; the app declares runtime {at}, older than the runtimes the corpus covers ({} and later), so AL Runner cannot tell what the AL compiler reports for the call",
        al_version::Runtime::floor()
    )
}

impl<'a> Checker<'a> {
    /// The runtime the app of `file` declares, when it is an app under test below the catalog's runtime.
    pub(crate) fn declared_runtime(&self, file: usize) -> Option<al_version::Runtime> {
        let app = self.ws.files.get(file)?.app;
        if self.ws.kinds.get(app) == Some(&crate::workspace::AppKind::Dependency) {
            return None;
        }
        self.catalog.app_runtime(&self.ws.apps.get(app)?.id)
    }

    /// Whether the file belongs to a dependency app (not gated by runtime: alc compiled it already).
    fn in_dependency(&self, file: usize) -> bool {
        self.ws
            .files
            .get(file)
            .is_some_and(|f| self.ws.kinds.get(f.app) == Some(&crate::workspace::AppKind::Dependency))
    }

    /// Whether alc compiles the builtin call `id` with `argc` arguments at some runtime before `at` but not at `at`:
    /// a method removed in an older runtime.
    fn removed_at(&self, id: &str, argc: usize, at: al_version::Runtime) -> bool {
        match self.catalog.alc_runtime(id, argc) {
            Some(al_builtins::catalog::AlcRuntime::Measured { ranges, .. }) => {
                !ranges
                    .iter()
                    .any(|(from, to)| *from <= at && to.is_none_or(|to| at < to))
                    && ranges.iter().any(|(_, to)| to.is_some_and(|to| to <= at))
            }
            _ => false,
        }
    }

    /// What the AL compiler reports for the builtin call `id` with `argc` arguments in an app at runtime `at`
    /// (`al-builtins/catalog/alc-runtimes.txt`, measured by compiling a probe of the call at every runtime alc accepts).
    /// Reports AL0666 with alc's text or, where the table has none, refuses with AR1007; true when it reported.
    fn alc_verdict(
        &mut self,
        sc: &Scope,
        node: &SyntaxNode,
        id: &str,
        argc: usize,
        name: &str,
        at: al_version::Runtime,
    ) -> bool {
        use al_builtins::catalog::{AlcOutcome, AlcRuntime};
        let alc = al_builtins::catalog::alc_runtimes_compiler();
        let why = match self.catalog.alc_runtime(id, argc) {
            Some(a) if a.compiles_at(at) == Some(true) => return false,
            Some(a @ AlcRuntime::Measured { .. }) => match a.outcome_at(at) {
                Some(AlcOutcome::Al0666(text)) => {
                    // The compiler's text: must-not-compile 0620-record-fullyqualifiedname-runtime-16.al:1 ('16.0'),
                    // 0561-testpage-prev.al:1 (a removed method); the part after "are: " is alc's own (alc-runtimes.txt).
                    let text = format!(
                        "'{name}' is not available in runtime version '{at}'. The supported runtime versions are: {text}."
                    );
                    self.diag(sc, node, "AL0666", text);
                    return true;
                }
                Some(AlcOutcome::Other(code)) => {
                    format!(
                        "does not compile at runtime {at} with AL compiler {alc}, which reports {code} there, not AL0666"
                    )
                }
                None => format!("is not measured at runtime {at} by AL compiler {alc}"),
            },
            Some(AlcRuntime::NotExpressible(e)) => format!("has no probe that compiles with AL compiler {alc} ({e})"),
            None => format!("with {argc} arguments has no row in the table of AL compiler {alc}"),
        };
        self.diag(sc, node, "AR1007", below_floor(name, at, &why));
        true
    }

    /// Chooses an overload: by arity, then by how many argument types fit the
    /// declared parameter types (first in catalog order on a tie). Records the
    /// choice on `node`.
    pub(crate) fn pick(&mut self, sc: &Scope, node: &SyntaxNode, cands: &[Cand], args: &[Ty], name: &str) -> Ty {
        let argc = args.len();
        // The best arity match by how many arguments fit; the first wins a tie.
        // An app under test that declares a runtime below the catalog's sees only the overloads of its own runtime
        // (al-runner issue 5573): a tagged overload newer than it is hidden for it too.
        let declared = self.declared_runtime(sc.file);
        let since_of = |c: &Cand| match &c.target {
            Res::Builtin(id) => c.hidden.or_else(|| self.catalog.by_id(id).and_then(|s| s.since)),
            _ => c.hidden,
        };
        let is_hidden = |c: &Cand| match declared {
            Some(r) => since_of(c).is_some_and(|s| s > r),
            None => c.hidden.is_some(),
        };
        let best_of = |hidden: bool| {
            let mut best: Option<(&Cand, usize)> = None;
            for c in cands
                .iter()
                .filter(|c| is_hidden(c) == hidden && c.min <= argc && argc <= c.max)
            {
                // How many arguments fit decides; among equals, the overload whose parameter types are the arguments'
                // own types wins over one that only takes them as a Variant (Reservation Management has
                // `SetReservSource(Variant)` and `SetReservSource(RecordRef)`; passing a RecordRef picks the latter).
                let is_proc = matches!(c.target, Res::Proc { .. });
                let fit = args
                    .iter()
                    .zip(&c.param_tys)
                    .filter(|(a, p)| if is_proc { fits_proc(a, p) } else { fits(a, p) })
                    .count();
                // Only among the procedures AL code declares: the catalog's overloads keep their order.
                let exact = if matches!(c.target, Res::Proc { .. }) {
                    args.iter()
                        .zip(&c.param_tys)
                        .filter(|(a, p)| !matches!(a, Ty::Unknown) && a.to_string().eq_ignore_ascii_case(p))
                        .count()
                } else {
                    0
                };
                let score = fit * 1000 + exact;
                if best.is_none_or(|(_, s)| score > s) {
                    best = Some((c, score));
                }
            }
            best
        };
        let (best, newer) = (best_of(false), best_of(true));
        let floor = al_version::Runtime::floor();
        // An overload of a newer runtime that fits strictly better than every visible one
        // is what the author meant: BC's compiler refuses the call (ADR-015 3.3 point 4).
        if let Some((h, hs)) = newer {
            if best.is_none_or(|(_, s)| hs > s) {
                let since = since_of(h).expect("a hidden candidate has its runtime");
                let at = declared.or(self.catalog.runtime()).unwrap_or(since);
                if at < floor {
                    // Below the runtimes the corpus covers, alc's own measurement decides (al-runner issue 5573).
                    let reported = match &h.target {
                        Res::Builtin(id) => self.alc_verdict(sc, node, &id.clone(), argc, name, at),
                        _ => false,
                    };
                    if !reported {
                        let why = format!("is tagged {since} in the catalog, but the AL compiler compiles it at {at}");
                        self.diag(sc, node, "AR1007", below_floor(name, at, &why));
                    }
                } else {
                    // The AL compiler's text: must-not-compile 0620-record-fullyqualifiedname-runtime-16.al:1 and
                    // 0576-codeunit-run-by-name-runtime17.al:1 ('16.0' / '16.1'), 0364-jsonobject-selecttokens-runtime17.al:1.
                    self.diag(
                        sc,
                        node,
                        "AL0666",
                        format!(
                            "'{name}' is not available in runtime version '{at}'. The supported runtime versions are: '{since}' or greater."
                        ),
                    );
                }
                return Ty::Unknown;
            }
        }
        if let Some((c, _)) = best {
            if let Res::Builtin(id) = &c.target {
                let id = id.clone();
                // The app's runtime: the one it declares, else the catalog's (a dependency app is never gated).
                let at = declared
                    .or(self.catalog.runtime())
                    .filter(|_| !self.in_dependency(sc.file));
                if let Some(r) = at {
                    // Below the runtime floor the catalog's tags say nothing: the AL compiler's measurement decides.
                    // At or above it, only a method removed before the app's runtime (TestPage.Prev: must-not-compile
                    // 0561-testpage-prev.al:1, 0562-testpart-prev.al:1) is checked: alc reports AL0666 there.
                    if (r < floor || self.removed_at(&id, argc, r)) && self.alc_verdict(sc, node, &id, argc, name, r) {
                        return Ty::Unknown;
                    }
                }
            }
        }
        match best {
            Some((c, _)) => {
                let (mut ret, target, with) = (c.ret.clone(), c.target.clone(), c.with_off);
                // `Abs` of a Duration is a BigInteger, not the Decimal the catalog declares: Format(Abs(D)) reads `1500` and a Decimal
                // variable assigned from it reads `1,500` (misc/lt-protected-var-abs-query-views/P146AbsDuration.al:P146_Abs_Duration_ResultFormats).
                if name.eq_ignore_ascii_case("abs")
                    && !matches!(target, Res::Proc { .. })
                    && args.len() == 1
                    && matches!(args[0], Ty::Duration)
                {
                    ret = Ty::BigInteger;
                }
                self.record_with(sc, node, target, with);
                ret
            }
            None => {
                self.diag(
                    sc,
                    node,
                    "AR1003",
                    format!("no overload of `{name}` takes {argc} argument(s)"),
                );
                Ty::Unknown
            }
        }
    }

    /// Procedures and functions named `n`, skipping variables and fields: a
    /// call `UserSecurityId()` means the function even in a table with a
    /// field of that name.
    pub(crate) fn lookup_callable(&mut self, sc: &mut Scope, node: &SyntaxNode, n: &str) -> Option<Vec<Cand>> {
        // The record of a `with` statement first (corpus events/lt-with-statement-scopes/WithStatementAmbiguous.al:
        // P195_With_ProcedureSameNameAsTableProcedure, P195_With_BuiltInMethodSameNameAsCodeunitProcedure).
        for (t, w) in sc.withs.clone().iter().rev() {
            if let Some(E::Callable(mut cs)) = self.member(sc, t, n) {
                cs.iter_mut().for_each(|c| c.with_off = Some(*w));
                return Some(cs);
            }
        }
        // In a report data item's trigger or column source, a procedure of the data item's table runs before the report's of
        // the same name; in a report trigger the report's runs (report/lt-cj-report-leads/CjDataset.al:215-241,
        // SameNameProcedure_ExtensionField_SaveAsXml: `FooCol = table 1,..`, `row 1 foo=table 1`, `prereport foo=report`).
        // Pinned only for one procedure on each side, declared by the table itself, with the same parameters. Any other
        // shape (other parameters, more overloads, a procedure of a table extension) is not pinned: AR1006 here, and the
        // lowering refuses the call (`Lowerer::report_item_same_name_refusal`).
        if sc.report_item && sc.rec_unqualified && !self.members[sc.self_obj].procedures_named(n).is_empty() {
            if let Some(rec @ Ty::Record { .. }) = sc.rec.clone() {
                let Ty::Record { table, .. } = &rec else { unreachable!() };
                let ti = self.resolve_quiet(sc.file, ObjKind::Table, table);
                if let Some(E::Callable(cs)) = self.member(sc, &rec, n) {
                    let procs: Vec<&Cand> = cs.iter().filter(|c| matches!(c.target, Res::Proc { .. })).collect();
                    let mine = self.members[sc.self_obj].procedures_named(n);
                    let pinned = mine.len() == 1
                        && cs.len() == 1
                        && procs.len() == 1
                        && matches!(procs[0].target, Res::Proc { object, .. } if Some(object) == ti)
                        && proc_cand(mine[0]).param_tys == procs[0].param_tys;
                    if pinned {
                        return Some(cs);
                    }
                    if !procs.is_empty() {
                        self.diag(
                            sc,
                            node,
                            "AR1006",
                            format!(
                                "`{n}` is declared by the report and by the data item's table (or its extension) with other parameters or overloads: which one runs is not pinned"
                            ),
                        );
                    }
                }
            }
        }
        let mut c: Vec<Cand> = self.members[sc.self_obj]
            .procedures_named(n)
            .iter()
            .map(|p| proc_cand(p))
            .collect();
        if sc.self_obj != sc.oi {
            c.extend(self.members[sc.oi].procedures_named(n).iter().map(|p| proc_cand(p)));
        }
        self.same_signature_in_two_objects(sc, node, n, &mut c);
        // In the OnRun of a codeunit with TableNo a bare IsDirty() is Record.IsDirty (runtime 18.0), not the codeunit's own
        // IsDirty (events/lt-with-statement-scopes/WithStatementAmbiguous.al:277-279, codeunit 82156 at :412-434; on runtime
        // 16.1 and 17.1 BC refuses the call, must-not-compile case 1407). Only IsDirty and only OnRun are pinned: another
        // procedure of the codeunit is refused by the lowering (`Lowerer::bare_is_dirty_unpinned`).
        if !c.is_empty() && sc.codeunit_on_run && sc.rec_unqualified && n.eq_ignore_ascii_case("isdirty") {
            if let Some(rec) = sc.rec.clone() {
                let record = self.catalog_cands("Record", n, true, &rec);
                if record.iter().any(|r| r.hidden.is_none()) {
                    c = record;
                }
            }
        }
        if c.is_empty() && sc.rec_unqualified {
            if let Some(rec) = sc.rec.clone() {
                if let Some(recv) = receiver_of(&rec) {
                    c = self.catalog_cands(&recv, n, true, &rec);
                }
                // `Modify()` written without `Rec.` in table, page or report code writes to the
                // implicit record like any other write.
                if !c.is_empty() {
                    if let Ty::Record { table, .. } = &rec {
                        if let Some(ti) = self.resolve_quiet(sc.file, ObjKind::Table, table) {
                            self.touch_ops(ti, demand::record_ops(n));
                        }
                    }
                }
            }
        }
        // A procedure of the extension next to procedures of its base with the same name: the base's
        // are overloads of it (corpus pageext/POPageExtOverload.al on corpus branch al-runtime-rs-staging:
        // `Describe(5)` calls the base's Integer procedure, `Describe('t')` the extension's Text one).
        let base = self.base_proc_cands(sc, n);
        if c.is_empty() {
            c = base;
        } else {
            // The same parameter types is AL0440 (the lowering refuses it): the extension's own
            // procedure stays the candidate then.
            let mut all: Vec<Cand> = base
                .into_iter()
                .filter(|b| !c.iter().any(|o| o.param_tys == b.param_tys))
                .collect();
            all.extend(c);
            c = all;
        }
        if c.is_empty() {
            c = self.global_cands(n);
        }
        (!c.is_empty()).then_some(c)
    }

    /// A table and a table extension of another app may declare a procedure with the same name and parameter types (BC
    /// compiles it with warning AL0523). A bare call in the code of one of them runs that object's own procedure: the
    /// table's in the table's code, the extension's in the extension's (tableextension/lt-bw-cross-app/BwCrossAppTests.al:34-39,
    /// fixtures tests/al-language-xapp-a/BwXaObjects.al:19-37 and tests/al-language-xapp-b/BwXbSameNames.al). In any other
    /// object's code (a third extension) which one runs is not pinned: AR1006, and the lowering refuses the call.
    fn same_signature_in_two_objects(&mut self, sc: &Scope, node: &SyntaxNode, n: &str, c: &mut Vec<Cand>) {
        let object_of = |c: &Cand| match c.target {
            Res::Proc { object, .. } => Some(object),
            _ => None,
        };
        let mut i = 0;
        while i < c.len() {
            let twins: Vec<usize> = (0..c.len())
                .filter(|&j| {
                    c[j].param_tys == c[i].param_tys
                        && object_of(&c[j]).is_some()
                        && object_of(&c[j]) != object_of(&c[i])
                })
                .collect();
            if twins.is_empty() || object_of(&c[i]).is_none() {
                i += 1;
                continue;
            }
            let group: Vec<usize> = std::iter::once(i).chain(twins.iter().copied()).collect();
            // A third table extension in the app of the extension twin runs that extension's body (`same_app_ext_twin`).
            let mut objects: Vec<usize> = group.iter().filter_map(|&j| object_of(&c[j])).collect();
            objects.sort_unstable();
            objects.dedup();
            let same_app_twin = self
                .same_app_ext_twin(sc, &objects)
                .and_then(|own| group.iter().copied().find(|&j| object_of(&c[j]) == Some(own)));
            match group
                .iter()
                .copied()
                .find(|&j| object_of(&c[j]) == Some(sc.oi))
                .or(same_app_twin)
            {
                Some(own) => {
                    let keep = c[own].clone();
                    c.retain(|x| x.param_tys != keep.param_tys || object_of(x).is_none());
                    // At the position of the first of the twins, so the overload order stays as it was.
                    c.insert(i.min(c.len()), keep);
                    i += 1;
                }
                None => {
                    self.diag(
                        sc,
                        node,
                        "AR1006",
                        format!(
                            "`{n}` is declared with the same parameters by two objects other than the calling one (a table and an extension of another app): which one runs is not pinned"
                        ),
                    );
                    return;
                }
            }
        }
    }

    /// Of two objects that declare a procedure with the same name and parameters (a table and a table extension of
    /// another app), the one a call in a third table extension runs when that extension is in the app of the extension
    /// twin: app B's 61114, next to its twin 61113, the other twin table 61101 of app A, runs B's extension body, bare
    /// and through `Rec` (tableextension/lt-bw-cross-app/BwCrossAppTests.al:214, fixture
    /// tests/al-language-xapp-b/QyXbObjects.al:38). From a third app the same calls do not compile (AL0196,
    /// must-not-compile 2071). Any other shape: `None`.
    pub(crate) fn same_app_ext_twin(&self, sc: &Scope, objects: &[usize]) -> Option<usize> {
        let app_of = |o: usize| self.ws.objects.get(o).map(|x| x.app);
        let kind = |o: usize| self.ws.objects.get(o).map(|x| x.kind);
        let own_app = app_of(sc.oi)?;
        let &[x, y] = objects else { return None };
        if kind(sc.oi) != Some(ObjKind::TableExtension) || objects.contains(&sc.oi) {
            return None;
        }
        let (own, other) = if app_of(x) == Some(own_app) { (x, y) } else { (y, x) };
        (app_of(own) == Some(own_app)
            && app_of(other) != Some(own_app)
            && kind(own) == Some(ObjKind::TableExtension)
            && kind(other) == Some(ObjKind::Table))
        .then_some(own)
    }

    /// The resolution of global `key`: the base object's variable in an extension that sees it
    /// (ADR-023), else the object's own.
    pub(crate) fn global_res(sc: &Scope, key: String) -> Res {
        if sc.base_vars.contains(&key) {
            Res::BaseGlobal(key)
        } else {
            Res::Global(key)
        }
    }

    /// The procedures of the page or report an extension extends that its code may call (ADR-023).
    pub(crate) fn base_proc_cands(&self, sc: &Scope, n: &str) -> Vec<Cand> {
        let Some(b) = sc.base_obj else { return Vec::new() };
        let ast = self.ws.object_ast(b);
        let visible: Vec<usize> = crate::base_access::visible_procs(&ast, n)
            .map(|r| usize::from(r.syntax().text_range().start()))
            .collect();
        self.members[b]
            .procedures_named(n)
            .iter()
            .filter(|p| visible.contains(&p.offset))
            .map(|p| proc_cand(p))
            .collect()
    }

    /// `this.Name` in a codeunit: a global variable, else a procedure of the codeunit. The
    /// resolution is recorded on the member expression, as a member access would.
    pub(crate) fn this_member(&mut self, sc: &mut Scope, node: &SyntaxNode, member: &SyntaxNode) -> Option<E> {
        let n = name_of(member);
        let key = n.to_lowercase();
        if let Some(t) = sc.globals.get(&key).cloned() {
            let res = if sc.base_vars.contains(&key) {
                Res::ThisBaseGlobal(key)
            } else {
                Self::global_res(sc, key)
            };
            self.record(sc, node, res);
            return Some(E::Val(t));
        }
        // `this.Id` in a query's own code: the running query's column (corpus query/PRQueryOpenColumn.al `PeekThis`).
        if let Some(rec) = sc.rec.clone() {
            if matches!(rec, Ty::Object(ObjKind::Query, _)) && sc.self_obj == sc.oi {
                if let Some(r) = self.field_res(sc, &rec, &n) {
                    if let Some(e) = self.member(sc, &rec, &n) {
                        self.record(sc, node, r);
                        return Some(e);
                    }
                }
            }
        }
        let own: Vec<Cand> = self.members[sc.self_obj]
            .procedures_named(&n)
            .iter()
            .map(|p| proc_cand(p))
            .collect();
        // `this.BaseVar` where the base report's or page's variable is not protected, or `this.LocalProc()` where the
        // procedure is local: AL0161, as for the bare name (corpus must-not-compile 0122).
        if own.is_empty()
            && !self.ws.is_dependency(sc.oi)
            && matches!(
                self.ws.objects[sc.oi].kind,
                ObjKind::ReportExtension | ObjKind::PageExtension
            )
            && let Some(b) = self.ws.extension_target(sc.oi)
            && self.base_proc_cands(sc, &n).is_empty()
        {
            let base = self.ws.object_ast(b);
            let is_var = crate::base_access::declares_var(&base, &n);
            let is_local_proc = base
                .routines()
                .any(|r| r.name().eq_ignore_ascii_case(&n) && r.is_local());
            if is_var || is_local_proc {
                let shown = if is_var { n.clone() } else { format!("{n}()") };
                self.diag(
                    sc,
                    member,
                    "AL0161",
                    format!("'{shown}' is inaccessible due to its protection level"),
                );
                return Some(E::Val(Ty::Unknown));
            }
        }
        // In an extension `this` is the base object, and both the base's procedures and the extension's own are
        // reachable through it (types/PYThisExtensions.al: this.BaseProc, this.ExtProc, this.BasePageProc, this.ExtPageProc).
        if sc.self_obj != sc.oi || sc.base_obj.is_some() {
            let own_obj: Vec<Cand> = if sc.self_obj != sc.oi {
                self.members[sc.oi]
                    .procedures_named(&n)
                    .iter()
                    .map(|p| proc_cand(p))
                    .collect()
            } else {
                Vec::new()
            };
            let base = self.base_proc_cands(sc, &n);
            let mut all: Vec<Cand> = base
                .into_iter()
                .filter(|b| !own.iter().chain(&own_obj).any(|o| o.param_tys == b.param_tys))
                .collect();
            all.extend(own);
            all.extend(own_obj);
            return (!all.is_empty()).then_some(E::Callable(all));
        }
        (!own.is_empty()).then_some(E::Callable(own))
    }

    /// Name lookup without reporting: None if the name does not resolve.
    pub(crate) fn lookup_name(&mut self, sc: &mut Scope, node: &SyntaxNode) -> Option<E> {
        // One copy of the name for the common case (a variable); the spelling as written only when it is not one.
        if !sc.withs.is_empty() {
            let n = name_of(node);
            for (t, w) in sc.withs.clone().iter().rev() {
                let Some(mut e) = self.member(sc, t, &n) else { continue };
                if let Some(r) = self.field_res(sc, t, &n) {
                    self.record_with(sc, node, r, Some(*w));
                }
                if let E::Callable(cs) = &mut e {
                    cs.iter_mut().for_each(|c| c.with_off = Some(*w));
                }
                return Some(e);
            }
        }
        let mut key = name_of(node);
        if key.is_ascii() {
            key.make_ascii_lowercase();
        } else {
            key = key.to_lowercase();
        }
        if let Some(t) = sc.vars.iter().rev().find_map(|frame| frame.get(&key)) {
            let t = t.clone();
            self.record(sc, node, Res::Local(key));
            return Some(E::Val(t));
        }
        if let Some(t) = sc.globals.get(&key) {
            let t = t.clone();
            let res = Self::global_res(sc, key);
            self.record(sc, node, res);
            return Some(E::Val(t));
        }
        let n = name_of(node);
        // `this` is the running instance of a codeunit (`this.EmitLengthWarning(..)`).
        if key == "this" && self.ws.objects[sc.oi].kind == ObjKind::Codeunit && sc.self_obj == sc.oi {
            return Some(E::Val(Ty::Object(
                ObjKind::Codeunit,
                self.ws.objects[sc.oi].name.as_str().into(),
            )));
        }
        if sc.rec_unqualified {
            if let Some(rec) = sc.rec.clone() {
                if rec == Ty::Unknown {
                    return Some(E::Val(Ty::Unknown));
                }
                if let Some(e) = self.member(sc, &rec, &n) {
                    if let Some(r) = self.field_res(sc, &rec, &n) {
                        self.record(sc, node, r);
                    }
                    return Some(e);
                }
            }
        }
        let own: Vec<Cand> = self.members[sc.self_obj]
            .procedures_named(&n)
            .iter()
            .map(|p| proc_cand(p))
            .collect();
        let own_obj: Vec<Cand> = if sc.self_obj != sc.oi {
            self.members[sc.oi]
                .procedures_named(&n)
                .iter()
                .map(|p| proc_cand(p))
                .collect()
        } else {
            Vec::new()
        };
        let base = self.base_proc_cands(sc, &n);
        if !own.is_empty() || !own_obj.is_empty() {
            let own: Vec<Cand> = own.into_iter().chain(own_obj).collect();
            let mut all: Vec<Cand> = base
                .into_iter()
                .filter(|b| !own.iter().any(|o| o.param_tys == b.param_tys))
                .collect();
            all.extend(own);
            return Some(E::Callable(all));
        }
        if !base.is_empty() {
            return Some(E::Callable(base));
        }
        let globals: Vec<Cand> = self.global_cands(&n);
        if !globals.is_empty() {
            return Some(E::Callable(globals));
        }
        let keyword = match key.as_str() {
            "database" | "table" => Some(E::ObjKeyword(ObjKind::Table)),
            "codeunit" => Some(E::ObjKeyword(ObjKind::Codeunit)),
            "page" => Some(E::ObjKeyword(ObjKind::Page)),
            "report" => Some(E::ObjKeyword(ObjKind::Report)),
            "query" => Some(E::ObjKeyword(ObjKind::Query)),
            "xmlport" => Some(E::ObjKeyword(ObjKind::XmlPort)),
            "enum" => Some(E::EnumKeyword),
            _ => None,
        };
        if keyword.is_some() {
            return keyword;
        }
        if let Some(i) = self.resolve(sc, ObjKind::Enum, &n) {
            return Some(E::TypeName(Ty::Enum(self.ws.objects[i].name.as_str().into())));
        }
        if key == "action" {
            return Some(E::TypeName(Ty::Option(
                al_builtins::value::ACTION_MEMBERS
                    .iter()
                    .map(|m| m.to_string())
                    .collect(),
            )));
        }
        if key == "promptmode" {
            return Some(E::TypeName(Ty::Option(
                al_builtins::value::PROMPT_MODE_MEMBERS
                    .iter()
                    .map(|m| m.to_string())
                    .collect(),
            )));
        }
        if key == "textencoding" {
            return Some(E::TypeName(Ty::Option(crate::types::text_encoding_members())));
        }
        if self.system_enums.contains(&key) {
            return Some(E::TypeName(Ty::Builtin(n.into())));
        }
        // Library types used statically: `JsonObject`, `NavApp`, `TaskScheduler`, ...
        if self
            .catalog
            .signatures
            .iter()
            .any(|s| s.receiver.eq_ignore_ascii_case(&n))
        {
            return Some(E::TypeName(Ty::Builtin(n.into())));
        }
        None
    }

    /// Static methods reachable through a type name.
    pub(crate) fn static_members(&self, t: &Ty, name: &str) -> Option<Vec<Cand>> {
        let recv = receiver_of(t)?;
        let c = self.catalog_cands(&recv, name, false, t);
        (!c.is_empty()).then_some(c)
    }

    /// Visible overloads first, then the ones only a newer runtime has (marked `hidden`).
    pub(crate) fn catalog_cands(&self, receiver: &str, name: &str, instance_only: bool, recv_ty: &Ty) -> Vec<Cand> {
        let keep = |s: &&Signature| !instance_only || s.instance;
        let visible = self.catalog.overloads(receiver, name).into_iter().filter(keep);
        let hidden = self.catalog.hidden(receiver, name).into_iter().filter(keep);
        visible
            .map(|s| sig_cand(s, recv_ty, self.catalog.id(s)))
            .chain(hidden.map(|s| hidden_cand(s, recv_ty, self.catalog.id(s))))
            .collect()
    }

    /// Unqualified builtins (`Message`, `Commit`): visible, then hidden.
    pub(crate) fn global_cands(&self, name: &str) -> Vec<Cand> {
        self.catalog
            .global(name)
            .iter()
            .map(|s| sig_cand(s, &Ty::Unknown, self.catalog.id(s)))
            .chain(
                self.catalog
                    .hidden_global(name)
                    .iter()
                    .map(|s| hidden_cand(s, &Ty::Unknown, self.catalog.id(s))),
            )
            .collect()
    }
}
