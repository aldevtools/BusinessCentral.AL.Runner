//! `--tdd` (ADR-051): in a test app, a call of a procedure that the target object does not declare becomes a stub site
//! instead of an error. The stub's return type comes from how the call's result is used (decision 3); a site whose
//! type no rule gives is refused (decision 7). Lowering turns a site into `ir::Expr::TddStub`.
//!
//! This is tooling, not AL semantics: no corpus case pins it, and without `--tdd` nothing here runs.
use super::*;

/// A stub site: the call `receiver.name(args)` on `object`, which declares no procedure `name`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TddSite {
    /// The object (index into `Workspace::objects`) the procedure is missing from.
    pub object: usize,
    /// The procedure name as the call writes it.
    pub name: String,
    /// The receiver as written (`Target`), for the refusal reasons.
    pub receiver: String,
    /// The types of the arguments, for the signature.
    pub args: Vec<Ty>,
    pub ret: TddRet,
}

/// The return type of a stub site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TddRet {
    /// Not known yet: the use that gives it has not been checked.
    Pending,
    /// The call is a statement: nothing reads a result.
    NoValue,
    Value(Ty),
    /// No rule gives one type: the reason the routine is refused for.
    Refused(String),
}

/// Per app: whether its call sites may become stubs, that is a workspace app that declares a test codeunit
/// (ADR-051 decision 2: call sites in the app under test stay compile errors).
pub(crate) fn test_apps(ws: &Workspace) -> Vec<bool> {
    let mut apps = vec![false; ws.kinds.len()];
    for (oi, o) in ws.objects.iter().enumerate() {
        if o.kind != ObjKind::Codeunit || ws.is_dependency(oi) || apps.get(o.app).copied().unwrap_or(true) {
            continue;
        }
        let test = ws
            .object_ast(oi)
            .properties()
            .any(|p| p.name().eq_ignore_ascii_case("Subtype") && p.value_text().eq_ignore_ascii_case("Test"));
        if test {
            apps[o.app] = true;
        }
    }
    apps
}

/// The signature a site is reported under (decision 6): `<Object>: procedure "<Name>"(Arg1: <Type>, ...): <Type>`.
/// `None` for a site that is refused or whose type is not known.
pub fn signature(ws: &Workspace, site: &TddSite) -> Option<String> {
    let ret = match &site.ret {
        TddRet::NoValue => String::new(),
        TddRet::Value(t) => format!(": {}", shown(t)),
        TddRet::Pending | TddRet::Refused(_) => return None,
    };
    let params: Vec<String> = site
        .args
        .iter()
        .enumerate()
        .map(|(i, t)| format!("Arg{}: {}", i + 1, shown(t)))
        .collect();
    Some(format!(
        "{}: procedure \"{}\"({}){ret}",
        crate::types::display_name(&ws.objects[site.object].name),
        site.name,
        params.join(", ")
    ))
}

/// The stubs of a checked program by the object whose code holds the site: (object, signature). The incremental
/// compile keeps those of the objects it does not check again ([`stub_list`]).
pub fn sites_by_object(ws: &Workspace, resolutions: &PosMap<Res>) -> Vec<(usize, String)> {
    resolutions
        .iter()
        .filter_map(|(&(file, start, _), r)| match r.inner() {
            Res::TddStub(site) => {
                let sig = signature(ws, site)?;
                let holder = (0..ws.objects.len()).find(|&oi| {
                    ws.objects[oi].file == file && {
                        let range = ws.object_ast(oi).syntax().text_range();
                        u32::from(range.start()) as usize <= start && start <= u32::from(range.end()) as usize
                    }
                })?;
                Some((holder, sig))
            }
            _ => None,
        })
        .collect()
}

/// Every stub of the program, sorted, each once (`ir::Program::tdd_stubs`).
pub fn stub_list(sites: &[(usize, String)]) -> Vec<String> {
    let mut out: Vec<String> = sites.iter().map(|(_, s)| s.clone()).collect();
    out.sort();
    out.dedup();
    out
}

/// Every stub of a checked program, sorted, each once (`ir::Program::tdd_stubs`).
pub fn signatures(ws: &Workspace, resolutions: &PosMap<Res>) -> Vec<String> {
    stub_list(&sites_by_object(ws, resolutions))
}

fn shown(t: &Ty) -> String {
    match t {
        Ty::Unknown => "Variant".into(),
        t => t.to_string(),
    }
}

/// A type as a stub takes it: a text literal's length is not the length of what the stub returns.
fn norm(t: &Ty) -> Ty {
    match t {
        Ty::Text(Some(_)) => Ty::Text(None),
        Ty::Label => Ty::Text(None),
        t => t.clone(),
    }
}

/// The node under any parentheses.
fn unparen(n: &SyntaxNode) -> SyntaxNode {
    let mut n = n.clone();
    while n.kind() == SyntaxKind::ParenExpr {
        match n.children().next() {
            Some(c) => n = c,
            None => break,
        }
    }
    n
}

/// The type a parameter of a catalog method declares, when it is one a stub can return; `None` for the rest.
fn catalog_param_ty(p: &str) -> Option<Ty> {
    let base = p.split('[').next().unwrap_or("").trim().to_ascii_lowercase();
    Some(match base.as_str() {
        "integer" => Ty::Integer,
        "biginteger" => Ty::BigInteger,
        "decimal" => Ty::Decimal,
        "boolean" => Ty::Boolean,
        "text" => Ty::Text(None),
        "code" => Ty::Code(None),
        "date" => Ty::Date,
        "time" => Ty::Time,
        "datetime" => Ty::DateTime,
        "duration" => Ty::Duration,
        "guid" => Ty::Guid,
        "char" => Ty::Char,
        _ => return None,
    })
}

fn variant_like(p: &str) -> bool {
    matches!(p.trim().to_ascii_lowercase().as_str(), "variant" | "any" | "joker")
}

impl<'a> Checker<'a> {
    fn tdd_key(sc: &Scope, n: &SyntaxNode) -> posmap::PosKey {
        let r = n.text_range();
        (sc.file, r.start().into(), r.end().into())
    }

    /// `Target.Missing` on a codeunit variable whose codeunit declares no `Missing`, as the callee of a call in a
    /// test app: noted as a stub site on the member node (the call completes it, [`Self::tdd_call`]).
    pub(crate) fn tdd_member(
        &mut self,
        sc: &Scope,
        node: &SyntaxNode,
        receiver: &SyntaxNode,
        base: &Ty,
        name: &str,
    ) -> bool {
        let Some(apps) = &self.tdd_apps else { return false };
        if !apps.get(self.ws.objects[sc.oi].app).copied().unwrap_or(false) || self.ws.is_dependency(sc.oi) {
            return false;
        }
        let Ty::Object(ObjKind::Codeunit, cu) = base else {
            return false;
        };
        let Some(object) = self.resolve(sc, ObjKind::Codeunit, cu) else {
            return false;
        };
        let is_callee = node
            .parent()
            .is_some_and(|p| p.kind() == SyntaxKind::CallExpr && p.children().next().as_ref() == Some(node));
        if !is_callee {
            return false;
        }
        let site = TddSite {
            object,
            name: name.to_string(),
            receiver: receiver.text().to_string().trim().to_string(),
            args: Vec::new(),
            ret: TddRet::Pending,
        };
        self.record(sc, node, Res::TddStub(Box::new(site)));
        true
    }

    /// The call `node` whose callee [`Self::tdd_member`] noted: the stub site, recorded on the call with its
    /// argument types. A call whose result is the receiver of another member is refused (decision 7).
    pub(crate) fn tdd_call(&mut self, sc: &Scope, node: &SyntaxNode, callee: &SyntaxNode, arg_tys: &[Ty]) -> bool {
        if self.tdd_apps.is_none() {
            return false;
        }
        let Some(Res::TddStub(site)) = self.resolutions.get(&Self::tdd_key(sc, callee)).cloned() else {
            return false;
        };
        let mut site = *site;
        site.args = arg_tys.iter().map(norm).collect();
        let receiver_use = node
            .parent()
            .is_some_and(|p| p.kind() == SyntaxKind::MemberExpr && p.children().next().as_ref() == Some(node));
        let k = Self::tdd_key(sc, node);
        if receiver_use {
            site.ret = TddRet::Refused(format!(
                "--tdd: cannot infer the return type of {}.{} (its result is used as a receiver)",
                site.receiver, site.name
            ));
        } else {
            self.tdd_pending.push(k);
        }
        self.resolutions.insert(k, Res::TddStub(Box::new(site)));
        true
    }

    fn tdd_site(&self, sc: &Scope, node: &SyntaxNode) -> Option<(posmap::PosKey, TddSite)> {
        self.tdd_apps.as_ref()?;
        let k = Self::tdd_key(sc, &unparen(node));
        match self.resolutions.get(&k) {
            Some(Res::TddStub(site)) => Some((k, (**site).clone())),
            _ => None,
        }
    }

    /// Whether `node` is a stub site whose type is not known yet.
    fn tdd_is_pending(&self, sc: &Scope, node: &SyntaxNode) -> bool {
        self.tdd_site(sc, node).is_some_and(|(_, s)| s.ret == TddRet::Pending)
    }

    fn tdd_set(&mut self, k: posmap::PosKey, mut site: TddSite, ret: TddRet) {
        if let TddRet::Value(t) = &ret {
            self.types.insert(k, t.clone());
        }
        site.ret = ret;
        self.resolutions.insert(k, Res::TddStub(Box::new(site)));
    }

    /// The use of `node` wants a value of type `want`: a pending stub site there returns that type.
    pub(crate) fn tdd_settle(&mut self, sc: &Scope, node: &SyntaxNode, want: &Ty) {
        if matches!(want, Ty::Unknown | Ty::Variant) {
            return;
        }
        if let Some((k, site)) = self.tdd_site(sc, node) {
            if site.ret == TddRet::Pending {
                self.tdd_set(k, site, TddRet::Value(norm(want)));
            }
        }
    }

    fn tdd_refuse_both(&mut self, sc: &Scope, node: &SyntaxNode) {
        if let Some((k, site)) = self.tdd_site(sc, node) {
            if site.ret == TddRet::Pending {
                let why = format!(
                    "--tdd: cannot infer the return type of {}.{} (both sides are unknown)",
                    site.receiver, site.name
                );
                self.tdd_set(k, site, TddRet::Refused(why));
            }
        }
    }

    /// `Target.Missing();`: a stub site that is a statement returns nothing.
    pub(crate) fn tdd_settle_statement(&mut self, sc: &Scope, node: &SyntaxNode) {
        if let Some((k, site)) = self.tdd_site(sc, node) {
            if site.ret == TddRet::Pending {
                self.tdd_set(k, site, TddRet::NoValue);
            }
        }
    }

    /// The operands of a binary operator: `and`, `or` and `xor` take Booleans; a comparison or arithmetic gives a stub
    /// the type of the other operand, and two stubs are refused.
    pub(crate) fn tdd_settle_operands(&mut self, sc: &Scope, op: &str, a: &SyntaxNode, b: &SyntaxNode, l: &Ty, r: &Ty) {
        if self.tdd_apps.is_none() {
            return;
        }
        match op {
            "and" | "or" | "xor" => {
                self.tdd_settle(sc, a, &Ty::Boolean);
                self.tdd_settle(sc, b, &Ty::Boolean);
            }
            "=" | "<>" | "<" | ">" | "<=" | ">=" | "+" | "-" | "*" | "/" | "div" | "mod" => {
                match (self.tdd_is_pending(sc, a), self.tdd_is_pending(sc, b)) {
                    (true, true) => {
                        self.tdd_refuse_both(sc, a);
                        self.tdd_refuse_both(sc, b);
                    }
                    (true, false) => self.tdd_settle(sc, a, r),
                    (false, true) => self.tdd_settle(sc, b, l),
                    (false, false) => {}
                }
            }
            _ => {}
        }
    }

    /// `case Target.Missing() of`: the type of the first label whose type is known.
    pub(crate) fn tdd_settle_case(&mut self, sc: &Scope, case: &SyntaxNode, selector: &SyntaxNode) {
        if !self.tdd_is_pending(sc, selector) {
            return;
        }
        let label = case
            .children()
            .filter(|c| c.kind() == SyntaxKind::CaseBranch)
            .flat_map(|b| b.children().filter(|c| is_expr(c.kind())).collect::<Vec<_>>())
            .filter_map(|l| self.types.get(&Self::tdd_key(sc, &l)).cloned())
            .find(|t| !matches!(t, Ty::Unknown));
        if let Some(t) = label {
            self.tdd_settle(sc, selector, &t);
        }
    }

    /// The arguments of the call `node`, once its overload is picked: a stub site passed to a parameter takes the
    /// parameter's type; to a `Variant` parameter, the type of the call's other `Variant` arguments when they agree
    /// (v2's rule); to `Format`, Text.
    pub(crate) fn tdd_settle_args(
        &mut self,
        sc: &Scope,
        node: &SyntaxNode,
        callee: &SyntaxNode,
        cands: &[Cand],
        args: &[SyntaxNode],
        arg_tys: &[Ty],
    ) {
        if !args.iter().any(|a| self.tdd_is_pending(sc, a)) {
            return;
        }
        let picked = self
            .resolutions
            .get(&Self::tdd_key(sc, node))
            .map(|r| r.inner().clone());
        let Some(picked) = picked else { return };
        let Some(cand) = cands.iter().find(|c| c.target == picked) else {
            return;
        };
        // The declared type of each parameter: `None` for a Variant one, `Some(None)` for a type a stub cannot take.
        let params: Vec<Option<Option<Ty>>> = match &picked {
            Res::Proc { object, offset } => {
                let sig = self.members[*object].procedures.iter().find(|p| p.offset == *offset);
                (0..args.len())
                    .map(|i| match sig.and_then(|s| s.params.get(i)) {
                        Some(p) if p.ty == Ty::Variant => None,
                        Some(p) => Some(Some(p.ty.clone())),
                        None => Some(None),
                    })
                    .collect()
            }
            _ => {
                let format = member_name(callee).eq_ignore_ascii_case("format");
                (0..args.len())
                    .map(|i| {
                        let p = cand.param_tys.get(i).map(String::as_str).unwrap_or("");
                        if i == 0 && format {
                            Some(Some(Ty::Text(None)))
                        } else if variant_like(p) {
                            None
                        } else {
                            Some(catalog_param_ty(p))
                        }
                    })
                    .collect()
            }
        };
        for (i, a) in args.iter().enumerate() {
            if !self.tdd_is_pending(sc, a) {
                continue;
            }
            match &params[i] {
                Some(Some(t)) => {
                    let t = t.clone();
                    self.tdd_settle(sc, a, &t);
                }
                Some(None) => {}
                None => {
                    let mut known: Vec<Ty> = Vec::new();
                    let mut stub_sibling = false;
                    for (j, b) in args.iter().enumerate() {
                        if j == i || params[j].is_some() {
                            continue;
                        }
                        if self.tdd_site(sc, b).is_some_and(|(_, s)| s.ret == TddRet::Pending) {
                            stub_sibling = true;
                        } else if !matches!(arg_tys[j], Ty::Unknown) {
                            known.push(norm(&arg_tys[j]));
                        }
                    }
                    known.dedup();
                    match known.as_slice() {
                        [t] => {
                            let t = t.clone();
                            self.tdd_settle(sc, a, &t);
                        }
                        [] if stub_sibling => self.tdd_refuse_both(sc, a),
                        [] => {}
                        _ => {
                            if let Some((k, site)) = self.tdd_site(sc, a) {
                                let why = format!(
                                    "--tdd: cannot infer the return type of {}.{} (the other Variant arguments have different types)",
                                    site.receiver, site.name
                                );
                                self.tdd_set(k, site, TddRet::Refused(why));
                            }
                        }
                    }
                }
            }
        }
        // Both sides of `AreEqual(Target.A(), Target.B(), ...)` are stubs: the second is refused too.
        for a in args {
            if self.tdd_is_pending(sc, a)
                && args.iter().any(|b| {
                    b != a
                        && self
                            .tdd_site(sc, b)
                            .is_some_and(|(_, s)| matches!(s.ret, TddRet::Refused(_)))
                })
            {
                self.tdd_refuse_both(sc, a);
            }
        }
    }

    /// The end of a routine: a stub site whose type no use gave is refused (decision 7).
    pub(crate) fn tdd_finish_routine(&mut self) {
        for k in std::mem::take(&mut self.tdd_pending) {
            if let Some(Res::TddStub(site)) = self.resolutions.get(&k).cloned() {
                if site.ret == TddRet::Pending {
                    let why = format!(
                        "--tdd: cannot infer the return type of {}.{} from its use",
                        site.receiver, site.name
                    );
                    self.tdd_set(k, *site, TddRet::Refused(why));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::workspace::{AppKind, AppSource};

    const TARGET: &str = r#"
codeunit 50100 "Target"
{
    procedure Existing(): Integer
    begin
        exit(1);
    end;
}
codeunit 50101 "Helper"
{
    procedure Three(A: Variant; B: Variant; C: Variant)
    begin
    end;
    procedure TakesDate(D: Date)
    begin
    end;
}
"#;

    /// Compiles `TARGET` and a test codeunit with `body` as its test's statements and `vars` as its locals.
    fn compile(vars: &str, body: &str, tdd: bool) -> (crate::Workspace, crate::lower::Lowered) {
        compile_with(vars, body, RET, tdd)
    }

    /// A procedure whose stub's type is the routine's result type (`exit`).
    const RET: &str = "local procedure R(): Decimal begin exit(Target.Ret()); end;";

    fn compile_with(vars: &str, body: &str, extra: &str, tdd: bool) -> (crate::Workspace, crate::lower::Lowered) {
        compile_full("", vars, body, extra, tdd)
    }

    /// As [`compile_with`], with `objects` as one more file of the app.
    fn compile_full(
        objects: &str,
        vars: &str,
        body: &str,
        extra: &str,
        tdd: bool,
    ) -> (crate::Workspace, crate::lower::Lowered) {
        let test = format!(
            "codeunit 50110 \"T\"\n{{\n    Subtype = Test;\n    var\n        Target: Codeunit \"Target\";\n        Helper: Codeunit \"Helper\";\n    [Test]\n    procedure P()\n    var\n        I: Integer;\n        T: Text;\n        B: Boolean;\n{vars}\n    begin\n{body}\n    end;\n    {extra}\n}}\n"
        );
        crate::lower::compile_apps_with(
            vec![AppSource {
                id: "a".into(),
                name: "a".into(),
                dependencies: vec![],
                files: vec![
                    ("t.al".into(), TARGET.into()),
                    ("o.al".into(), objects.into()),
                    ("test.al".into(), test),
                ],
            }],
            vec![AppKind::Workspace],
            &Default::default(),
            None,
            crate::check::Roots {
                tdd,
                ..Default::default()
            },
        )
    }

    /// Objects whose procedures a test calls through each kind of receiver.
    const RECEIVERS: &str = r#"
table 50120 "Item"
{
    fields { field(1; "No."; Code[20]) { } }
    procedure Existing(): Integer
    begin
        exit(1);
    end;
    procedure UsesRec(): Integer
    begin
        exit(Rec.ViaRec());
    end;
}
page 50121 "Card"
{
    SourceTable = "Item";
    procedure Existing()
    begin
    end;
    procedure UsesCurrPage(): Integer
    begin
        exit(CurrPage.ViaCurrPage());
    end;
}
report 50122 "Rep"
{
    procedure Existing()
    begin
    end;
}
interface "Ifc"
{
    procedure Existing();
}
"#;

    fn full_stubs(objects: &str, vars: &str, body: &str) -> Vec<String> {
        compile_full(objects, vars, body, RET, true).1.program.tdd_stubs.clone()
    }

    fn full_reasons(objects: &str, vars: &str, body: &str) -> Vec<String> {
        let (ws, l) = compile_full(objects, vars, body, RET, true);
        let oi = ws.objects.iter().position(|o| o.id == Some(50110)).expect("test codeunit");
        l.unsupported[oi].iter().flatten().cloned().collect()
    }

    #[test]
    fn a_missing_procedure_on_a_record_page_report_rec_currpage_this_or_the_object_itself_is_a_stub() {
        let vars = "Item: Record \"Item\"; Pg: Page \"Card\"; Rp: Report \"Rep\";";
        let body = "I := Item.OnRecord(); I := Pg.OnPage(); Rp.OnReport(I); I := Local1(2); B := this.OnThis();";
        let mut want: Vec<String> = [
            r#"Item: procedure "OnRecord"(): Integer"#,
            r#"Item: procedure "ViaRec"(): Integer"#,
            r#"Card: procedure "OnPage"(): Integer"#,
            r#"Card: procedure "ViaCurrPage"(): Integer"#,
            r#"Rep: procedure "OnReport"(Arg1: Integer)"#,
            r#"T: procedure "Local1"(Arg1: Integer): Integer"#,
            r#"T: procedure "OnThis"(): Boolean"#,
            r#"Target: procedure "Ret"(): Decimal"#,
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        want.sort();
        assert_eq!(full_stubs(RECEIVERS, vars, body), want);
        assert!(full_reasons(RECEIVERS, vars, body).is_empty(), "{:?}", full_reasons(RECEIVERS, vars, body));
    }

    #[test]
    fn no_overload_with_the_call_s_argument_count_gives_a_stub_overload() {
        let got = stubs("", "I := Target.Existing(5); I := Target.Existing();");
        assert_eq!(
            got,
            vec![
                r#"Target: procedure "Existing"(Arg1: Integer): Integer"#.to_string(),
                r#"Target: procedure "Ret"(): Decimal"#.to_string()
            ]
        );
        // A local procedure called with another count, too.
        let got = full_stubs("", "", "I := R(1);");
        assert!(got.contains(&r#"T: procedure "R"(Arg1: Integer): Integer"#.to_string()), "{got:?}");
    }

    #[test]
    fn a_testpage_an_interface_or_a_builtin_type_gets_no_stub_and_stays_refused() {
        let cases: &[(&str, &str)] = &[
            ("TP: TestPage \"Card\";", "TP.Missing();"),
            ("If: Interface \"Ifc\";", "If.Missing();"),
            ("", "I := T.Missing();"),
            ("L: List of [Integer];", "I := L.Missing();"),
        ];
        for (vars, body) in cases {
            let got = full_stubs(RECEIVERS, vars, body);
            assert!(!got.iter().any(|s| s.contains("Missing")), "{body}: {got:?}");
            assert!(!full_reasons(RECEIVERS, vars, body).is_empty(), "{body}");
        }
    }

    #[test]
    fn without_tdd_the_new_receivers_stay_errors() {
        let vars = "Item: Record \"Item\";";
        for body in ["I := Item.OnRecord();", "I := Local1(2);", "I := Target.Existing(5);"] {
            let (ws, l) = compile_full(RECEIVERS, vars, body, "", false);
            let oi = ws.objects.iter().position(|o| o.id == Some(50110)).unwrap();
            assert!(l.unsupported[oi].iter().flatten().next().is_some(), "{body}");
            assert!(l.program.tdd_stubs.is_empty());
        }
    }

    fn stubs(vars: &str, body: &str) -> Vec<String> {
        compile(vars, body, true).1.program.tdd_stubs.clone()
    }

    fn reasons(vars: &str, body: &str) -> Vec<String> {
        let (ws, l) = compile(vars, body, true);
        let oi = ws
            .objects
            .iter()
            .position(|o| o.id == Some(50110))
            .expect("test codeunit");
        l.unsupported[oi].iter().flatten().cloned().collect()
    }

    #[test]
    fn the_return_type_comes_from_the_use() {
        let cases: &[(&str, &str)] = &[
            ("I := Target.A();", r#"Target: procedure "A"(): Integer"#),
            ("I += Target.A();", r#"Target: procedure "A"(): Integer"#),
            ("if Target.A() then;", r#"Target: procedure "A"(): Boolean"#),
            ("while Target.A() do;", r#"Target: procedure "A"(): Boolean"#),
            ("repeat until Target.A();", r#"Target: procedure "A"(): Boolean"#),
            ("B := not Target.A();", r#"Target: procedure "A"(): Boolean"#),
            ("if B and Target.A() then;", r#"Target: procedure "A"(): Boolean"#),
            ("if I < Target.A() then;", r#"Target: procedure "A"(): Integer"#),
            ("I := Target.A() * 2;", r#"Target: procedure "A"(): Integer"#),
            ("T := 'x' + Target.A();", r#"Target: procedure "A"(): Text"#),
            ("T := Format(Target.A());", r#"Target: procedure "A"(): Text"#),
            ("case Target.A() of 1: ; end;", r#"Target: procedure "A"(): Integer"#),
            ("Helper.TakesDate(Target.A());", r#"Target: procedure "A"(): Date"#),
            ("Helper.Three(1, 2, Target.A());", r#"Target: procedure "A"(): Integer"#),
            (
                "Target.A(I, 'x', true);",
                r#"Target: procedure "A"(Arg1: Integer, Arg2: Text, Arg3: Boolean)"#,
            ),
            ("I := (Target.A());", r#"Target: procedure "A"(): Integer"#),
        ];
        for (body, want) in cases {
            let got = stubs("", body);
            let mut want = vec![want.to_string(), r#"Target: procedure "Ret"(): Decimal"#.to_string()];
            want.sort();
            assert_eq!(got, want, "{body}");
        }
    }

    #[test]
    fn a_site_no_rule_types_is_refused_with_the_reason() {
        let cases: &[(&str, &str)] = &[
            (
                "if Target.A() = Target.B() then;",
                "--tdd: cannot infer the return type of Target.A (both sides are unknown)",
            ),
            (
                "Helper.Three(1, 'x', Target.A());",
                "--tdd: cannot infer the return type of Target.A (the other Variant arguments have different types)",
            ),
            (
                "Target.A().Insert();",
                "--tdd: cannot infer the return type of Target.A (its result is used as a receiver)",
            ),
        ];
        for (body, want) in cases {
            let got = reasons("", body);
            assert!(got.iter().any(|r| r.contains(want)), "{body}: {got:?}");
        }
    }

    #[test]
    fn without_tdd_nothing_is_a_stub_and_without_a_missing_procedure_the_program_is_the_same() {
        let (_, plain) = compile("", "I := Target.A();", false);
        assert!(plain.program.tdd_stubs.is_empty());
        // The routine is refused as before.
        let (ws, l) = compile("", "I := Target.A();", false);
        let oi = ws.objects.iter().position(|o| o.id == Some(50110)).unwrap();
        assert!(
            l.unsupported[oi].iter().flatten().any(|r| r.contains("Target.A()")),
            "{:?}",
            l.unsupported[oi]
        );
        // Decision 11: with --tdd and no stub site, the program is the program without --tdd.
        let fine = |tdd| compile_with("", "I := Target.Existing();", "", tdd).1.program;
        let (with, without) = (fine(true), fine(false));
        assert!(*with == *without, "the programs differ");
    }

    #[test]
    fn call_sites_in_an_app_without_a_test_codeunit_stay_errors() {
        let (ws, l) = crate::lower::compile_apps_with(
            vec![AppSource {
                id: "a".into(),
                name: "a".into(),
                dependencies: vec![],
                files: vec![(
                    "t.al".into(),
                    format!(
                        "{TARGET}\ncodeunit 50102 \"User\" {{ procedure U() var X: Codeunit \"Target\"; begin X.Missing(); end; }}"
                    ),
                )],
            }],
            vec![AppKind::Workspace],
            &Default::default(),
            None,
            crate::check::Roots {
                tdd: true,
                ..Default::default()
            },
        );
        assert!(l.program.tdd_stubs.is_empty());
        let oi = ws.objects.iter().position(|o| o.id == Some(50102)).unwrap();
        assert!(
            !l.unsupported[oi]
                .iter()
                .flatten()
                .next()
                .unwrap_or(&String::new())
                .is_empty()
        );
    }
}
