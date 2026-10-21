//! Whether a selected test can observe the install baseline (ADR-013 decision 5, "Install
//! baseline only when needed", ADR-020 implementation note).
//!
//! The baseline is the data a session holds after the install code of the dependency apps and of the apps under test
//! ran (ADR-046). A test sees it only through rows, the counters that must not repeat (system ids, row versions,
//! AutoIncrement, time stamps of writes), isolated storage and number sequences (`al_vm::Baseline`
//! holds nothing else), and through the engine's set of tables the baseline could not seed (a
//! `Get` in dependency code) and its install facts (`al_vm::InstallFacts`, also only through a record read). Every one of those is reached through a database access, a
//! `CreateGuid`, an `IsolatedStorage` or `NumberSequence` call, or an event whose subscribers do
//! one of those.
//!
//! The rule is a whitelist over the lowered IR, so that anything it does not know is "may
//! observe" and the baseline runs. The reach starts at every routine of every selected test
//! codeunit (handlers are routines of the codeunit) and follows direct calls and interface
//! targets. It says "pure" only when all of this holds for everything reached:
//!
//! * the object is a codeunit, and every slot (global, local, parameter, return value) has a type
//!   that holds no database state: no `Record`, `RecordRef`, `Query`, page, report, `.NET` or
//!   other handle type;
//! * every statement and expression is plain computation, a call into the reach, or a builtin
//!   from the whitelist below (text, list, dictionary, JSON, XML, date and time functions, `Format`,
//!   `Evaluate`, `Error`, `Message` and friends);
//! * no routine called is an event publisher (its subscribers could do anything);
//! * the run has no custom test runner codeunit.
//!
//! Whether a test finds the answer it expects when the baseline is missing does not matter: a
//! pure reach never reads what the baseline holds.

use al_builtins::value::RtType;
use al_ir::{Arg, Const, DotNetOp, Expr, Object, Place, Routine, SetElem, Stmt};
use al_sema::lower::Lowered;
use std::collections::HashSet;

/// Builtin receivers whose methods work on values in memory only.
const PURE_RECEIVERS: &[&str] = &[
    "Text",
    "Label",
    "TextConst",
    "List",
    "Dictionary",
    "JsonArray",
    "JsonObject",
    "JsonValue",
    "JsonToken",
    "TextBuilder",
    "BigText",
    "BigInteger",
    "Date",
    "Time",
    "DateTime",
    "Enum",
    "Version",
    "ErrorInfo",
    "Variant",
    "Dialog",
    "SecretText",
    "XmlDocument",
    "XmlElement",
    "XmlNode",
    "XmlAttribute",
    "XmlText",
    "XmlCData",
    "XmlComment",
    "XmlDeclaration",
    "XmlProcessingInstruction",
    "XmlNamespaceManager",
    "XmlAttributeCollection",
    "XmlDocumentType",
];

/// The methods of `System` that touch nothing but their arguments, the host clock or the
/// session's own settings. Left out on purpose: `CreateGuid` (the counter the baseline advanced),
/// encryption, code coverage, streams, URLs and anything that loads a type.
const PURE_SYSTEM: &[&str] = &[
    "Abs",
    "ArrayLen",
    "CalcDate",
    "Clear",
    "ClearAll",
    "ClearCollectedErrors",
    "ClearLastError",
    "ClosingDate",
    "CompressArray",
    "CopyArray",
    "CreateDateTime",
    "CurrentDateTime",
    "Date2DMY",
    "Date2DWY",
    "DaTi2Variant",
    "DMY2Date",
    "DT2Date",
    "DT2Time",
    "DWY2Date",
    "Evaluate",
    "Format",
    "GetCollectedErrors",
    "GetLastErrorCallStack",
    "GetLastErrorCode",
    "GetLastErrorObject",
    "GetLastErrorText",
    "GlobalLanguage",
    "GuiAllowed",
    "HasCollectedErrors",
    "IsCollectingErrors",
    "IsNull",
    "IsNullGuid",
    "IsServiceTier",
    "NormalDate",
    "Power",
    "Random",
    "Randomize",
    "Round",
    "RoundDateTime",
    "Sleep",
    "TemporaryPath",
    "Time",
    "Today",
    "Variant2Date",
    "Variant2Time",
    "WindowsLanguage",
    "WorkDate",
];

/// The methods of `Database` that answer from the session, not from a row: `CompanyName` is the company the host gave
/// the session when it started (al-vm sessionops.rs), whatever the install code wrote.
const PURE_DATABASE: &[&str] = &["CompanyName"];

/// Methods of `Guid` that only convert a value.
const PURE_GUID: &[&str] = &["ToText"];

/// Library types (`RtType::Library`) that are plain values in memory.
const PURE_LIBRARY: &[&str] = &[
    "JsonObject",
    "JsonArray",
    "JsonToken",
    "JsonValue",
    "TextBuilder",
    "BigText",
    "ErrorInfo",
    "Version",
    "SecretText",
    "Dialog",
    "XmlDocument",
    "XmlElement",
    "XmlNode",
    "XmlAttribute",
    "XmlText",
    "XmlCData",
    "XmlComment",
    "XmlDeclaration",
    "XmlProcessingInstruction",
    "XmlNamespaceManager",
    "XmlAttributeCollection",
    "XmlDocumentType",
];

/// The attributes that make a procedure an event publisher.
const PUBLISHER_ATTRIBUTES: &[&str] = &[
    "IntegrationEvent",
    "BusinessEvent",
    "InternalEvent",
    "ExternalBusinessEvent",
];

/// Why a test of `tests` (object indices of the selected test codeunits) may observe the install
/// baseline, or `None` when none of them can. The reason names the first routine and construct
/// that could.
pub fn may_observe_baseline(lowered: &Lowered, tests: &[usize]) -> Option<String> {
    let objects = &lowered.program.objects;
    let mock = (lowered.program.dotnet_types.iter().enumerate())
        .filter(|(_, d)| {
            d.type_name
                .eq_ignore_ascii_case(al_builtins::dotnet_sigs::PERMISSION_TEST_HELPER)
        })
        .map(|(i, _)| i)
        .collect();
    let mut walk = Walk {
        whole: HashSet::new(),
        mock,
        objects,
        seen_objects: HashSet::new(),
        seen_routines: HashSet::new(),
        queue: Vec::new(),
    };
    for &t in tests {
        if walk.seen_objects.insert(t) {
            if let Err(why) = walk.check_object(t) {
                return Some(why);
            }
        }
        for r in 0..objects[t].routines.len() {
            walk.reach(t, r);
        }
    }
    // BC's test runner runs these codeunits around every test method when they exist (runtime/runner_reset.rs).
    if !tests.is_empty() {
        for id in al_sema::check::demand::TEST_RUNNER_CODEUNITS {
            let Some(o) = objects.iter().position(|o| {
                (o.is_kind(al_ir::ObjKind::Codeunit) || o.is_shell_of(al_ir::ObjKind::Codeunit)) && o.id == Some(id)
            }) else {
                continue;
            };
            let obj = &objects[o];
            if obj.uncompiled {
                return Some(format!(
                    "the test runner runs codeunit {id} {}, which was not compiled",
                    obj.name
                ));
            }
            if walk.seen_objects.insert(o) {
                if let Err(why) = walk.check_object(o) {
                    return Some(why);
                }
            }
            if let Some(r) = obj
                .routines
                .iter()
                .position(|r| r.is_trigger && r.name.eq_ignore_ascii_case("OnRun"))
            {
                walk.reach(o, r);
            }
        }
    }
    while let Some((o, r)) = walk.queue.pop() {
        let routine = &objects[o].routines[r];
        let at = |why: String| format!("{}.{}: {why}", objects[o].name, routine.name);
        if let Err(why) = walk.routine(routine) {
            return Some(at(why));
        }
    }
    None
}

/// Words that make a test codeunit unlikely to be free of data access.
const DATA_WORDS: &[&str] = &[
    "record",
    "query",
    "page",
    "report",
    "xmlport",
    "isolatedstorage",
    "numbersequence",
    "createguid",
    "database",
    "dotnet",
    "session",
    "codeunit.run",
];

/// The names in code that make a lean compile unlikely to stay small (`lean_closure_looks_pure`): [`DATA_WORDS`] as
/// whole names, with the other handle types.
const DATA_NAMES: &[&str] = &[
    "record",
    "recordref",
    "fieldref",
    "keyref",
    "recordid",
    "query",
    "page",
    "testpage",
    "testrequestpage",
    "report",
    "xmlport",
    "isolatedstorage",
    "numbersequence",
    "createguid",
    "database",
    "dotnet",
    "session",
];

/// [`text_looks_pure`] for what a lean compile compiles (`al_sema::check::lean_named`): `false` when the code of a
/// workspace object the selected tests (`only`: app and id, in program order) name, directly or through what it names,
/// holds one of [`DATA_NAMES`]. Such a lean compile reaches the dependency apps through the demand, costs as
/// much as the complete one, and the rule then says "may observe". Comments and texts are not read. Like
/// `text_looks_pure`, a wrong answer costs time, never an outcome.
pub fn lean_closure_looks_pure(ws: &al_sema::Workspace, only: &[(usize, u32)]) -> bool {
    let named = al_sema::check::lean_named(ws, only);
    !(0..ws.objects.len()).filter(|&oi| named[oi]).any(|oi| {
        ws.object_ast(oi).syntax().descendants_with_tokens().any(|e| {
            e.as_token().is_some_and(|t| {
                matches!(
                    t.kind(),
                    al_syntax::SyntaxKind::Ident | al_syntax::SyntaxKind::QuotedIdent
                ) && DATA_NAMES.iter().any(|w| t.text().eq_ignore_ascii_case(w))
            })
        })
    })
}

/// A cheap look at the source of the selected test codeunits (`only`: their app and id; `None`:
/// every file of every workspace app) before anything is compiled: `false` when a word shows that
/// they probably use data, or when a selected codeunit cannot be found. It only decides whether to
/// try compiling without the install baseline (and without the compile cache) first; a wrong answer
/// costs time, never an outcome, because [`may_observe_baseline`] decides.
pub fn text_looks_pure(
    apps: &[al_sema::AppSource],
    kinds: &[al_sema::AppKind],
    symbols: &std::collections::BTreeSet<String>,
    only: Option<&[(usize, u32)]>,
) -> bool {
    let looks = |text: &str| {
        let lower = text.to_ascii_lowercase();
        !DATA_WORDS.iter().any(|w| lower.contains(w))
    };
    let workspace = |app: usize| kinds.get(app) == Some(&al_sema::AppKind::Workspace);
    let Some(only) = only else {
        return apps
            .iter()
            .enumerate()
            .filter(|(i, _)| workspace(*i))
            .all(|(_, a)| a.files.iter().all(|(_, text)| looks(text)));
    };
    only.iter().all(|&(app, id)| {
        if !workspace(app) {
            return false;
        }
        let needle = id.to_string();
        // The codeunit is in a file that holds its id; its text runs to the next object.
        apps[app]
            .files
            .iter()
            .filter(|(_, text)| text.contains(&needle))
            .any(|(_, text)| {
                let objects = al_syntax::scan::scan(text, symbols).objects;
                objects
                    .iter()
                    .enumerate()
                    .find(|(_, o)| o.kind == "codeunit" && o.id == Some(id))
                    .is_some_and(|(i, o)| {
                        let end = objects.get(i + 1).map_or(text.len(), |n| n.offset);
                        text.get(o.offset..end).is_some_and(looks)
                    })
            })
    })
}

struct Walk<'a> {
    /// The local slots of the routine being walked that are used as a whole (read, written, passed or
    /// given a method), as opposed to only through their fields.
    whole: HashSet<usize>,
    /// The `DotNet` declarations of `PermissionTestHelper`: its constructor, `Clear` and `Dispose` start and stop the
    /// permissions mock with no set assigned, which touches no data (corpus permissions/
    /// TestPermissionsMockEnforcement.al:74-97; al-vm dotnet/server_classes.rs).
    mock: HashSet<usize>,
    objects: &'a [Object],
    seen_objects: HashSet<usize>,
    seen_routines: HashSet<(usize, usize)>,
    queue: Vec<(usize, usize)>,
}

type Check = Result<(), String>;

impl Walk<'_> {
    fn check_object(&self, o: usize) -> Check {
        let obj = &self.objects[o];
        if !obj.is_kind(al_ir::ObjKind::Codeunit) {
            return Err(format!("{} {} is not a codeunit", obj.kind_text(), obj.name));
        }
        for g in &obj.globals {
            self.slot(&g.ty)
                .map_err(|w| format!("{} {} has a global `{}`: {w}", obj.kind_text(), obj.name, g.name))?;
        }
        Ok(())
    }

    /// [`ty`], where a `PermissionTestHelper` variable holds no data.
    fn slot(&self, t: &RtType) -> Check {
        match t {
            RtType::DotNet(i) if self.mock.contains(&(*i as usize)) => Ok(()),
            t => ty(t),
        }
    }

    fn reach(&mut self, o: usize, r: usize) {
        if self.seen_routines.insert((o, r)) {
            self.queue.push((o, r));
        }
    }

    fn enter(&mut self, o: usize, r: usize) -> Check {
        if o >= self.objects.len() || r >= self.objects[o].routines.len() {
            return Err("a call to a routine that is not in the program".into());
        }
        if self.seen_objects.insert(o) {
            self.check_object(o)?;
        }
        let callee = &self.objects[o].routines[r];
        if callee
            .attributes
            .iter()
            .any(|a| PUBLISHER_ATTRIBUTES.iter().any(|p| a.name.eq_ignore_ascii_case(p)))
        {
            return Err(format!(
                "calls the event publisher {}.{}, whose subscribers may do anything",
                self.objects[o].name, callee.name
            ));
        }
        self.reach(o, r);
        Ok(())
    }

    fn routine(&mut self, r: &Routine) -> Check {
        // A record variable is a buffer in memory until something reads or writes rows through it. A
        // routine that only assigns and reads its fields (Library Assert's TypeNameOf does) touches no
        // row, counter or storage; any other use of the variable, or handing it on, may.
        self.whole.clear();
        self.stmts(&r.body)?;
        for (i, s) in r.locals.iter().enumerate() {
            if matches!(s.ty, RtType::Record(..)) {
                if self.whole.contains(&i) {
                    return Err(format!("local `{}`: a record variable used as a whole", s.name));
                }
            } else {
                self.slot(&s.ty).map_err(|w| format!("local `{}`: {w}", s.name))?;
            }
        }
        match &r.ret {
            Some(RtType::Record(..)) => Err("return value: a record variable".into()),
            Some(t) => ty(t).map_err(|w| format!("return value: {w}")),
            None => Ok(()),
        }
    }

    fn stmts(&mut self, body: &[Stmt]) -> Check {
        for s in body {
            self.stmt(s)?;
        }
        Ok(())
    }

    fn stmt(&mut self, s: &Stmt) -> Check {
        match s {
            Stmt::Assign(p, e) => {
                self.place(p)?;
                self.expr(e)
            }
            Stmt::Expr(e) => self.expr(e),
            Stmt::If(c, a, b) => {
                self.expr(c)?;
                self.stmts(a)?;
                self.stmts(b)
            }
            Stmt::While(c, b) => {
                self.expr(c)?;
                self.stmts(b)
            }
            Stmt::Repeat(b, c) => {
                self.stmts(b)?;
                self.expr(c)
            }
            Stmt::For {
                var, from, to, body, ..
            } => {
                self.place(var)?;
                self.expr(from)?;
                self.expr(to)?;
                self.stmts(body)
            }
            Stmt::Foreach {
                var, collection, body, ..
            } => {
                self.place(var)?;
                self.expr(collection)?;
                self.stmts(body)
            }
            Stmt::Case {
                scrutinee,
                branches,
                otherwise,
            } => {
                self.expr(scrutinee)?;
                for (elems, body) in branches {
                    for e in elems {
                        self.set_elem(e)?;
                    }
                    self.stmts(body)?;
                }
                self.stmts(otherwise)
            }
            Stmt::Exit(e) => e.as_ref().map_or(Ok(()), |e| self.expr(e)),
            Stmt::AssertError(b) => self.stmts(b),
            Stmt::Break | Stmt::Continue | Stmt::Pos(_) | Stmt::Assume(_) | Stmt::TddMark(_) => Ok(()),
        }
    }

    fn set_elem(&mut self, e: &SetElem) -> Check {
        match e {
            SetElem::One(e) => self.expr(e),
            SetElem::Range(a, b) => {
                a.as_ref().map_or(Ok(()), |e| self.expr(e))?;
                b.as_ref().map_or(Ok(()), |e| self.expr(e))
            }
        }
    }

    fn place(&mut self, p: &Place) -> Check {
        match p {
            Place::Local(i) => {
                self.whole.insert(*i);
                Ok(())
            }
            Place::Global(_) => Ok(()),
            Place::Index(inner, idx) => {
                self.place(inner)?;
                idx.iter().try_for_each(|e| self.expr(e))
            }
            // A field of a local record variable: the variable stays a buffer in memory.
            Place::Field(inner, _) if matches!(**inner, Place::Local(_)) => Ok(()),
            Place::Field(..) => Err("reads or writes a record field".into()),
            Place::BaseGlobal(_) => Err("uses a variable of a base page or report".into()),
        }
    }

    fn arg(&mut self, a: &Arg) -> Check {
        match a {
            Arg::Value(e) => self.expr(e),
            Arg::Ref(p) => self.place(p),
        }
    }

    fn expr(&mut self, e: &Expr) -> Check {
        match e {
            Expr::Const(c) => constant(c),
            Expr::Load(p) => self.place(p),
            Expr::CharAt(a, b) | Expr::Binary(_, a, b) => {
                self.expr(a)?;
                self.expr(b)
            }
            Expr::Unary(_, a) => self.expr(a),
            Expr::In(a, set) => {
                self.expr(a)?;
                set.iter().try_for_each(|s| self.set_elem(s))
            }
            Expr::Ternary(a, b, c) => {
                self.expr(a)?;
                self.expr(b)?;
                self.expr(c)
            }
            Expr::Convert(t, a) => {
                self.slot(t)?;
                self.expr(a)
            }
            Expr::Call {
                object,
                routine,
                instance,
                args,
                ..
            } => {
                self.enter(*object, *routine)?;
                if let Some(i) = instance {
                    self.expr(i)?;
                }
                args.iter().try_for_each(|a| self.arg(a))
            }
            Expr::InterfaceCall {
                receiver,
                targets,
                args,
            } => {
                for &(o, r) in targets {
                    self.enter(o, r)?;
                }
                self.expr(receiver)?;
                args.iter().try_for_each(|a| self.arg(a))
            }
            Expr::InterfaceTest { value, .. } => self.expr(value),
            Expr::Builtin { id, receiver, args, .. } => {
                builtin(id.name())?;
                if let Some(r) = receiver {
                    self.arg(r)?;
                }
                args.iter().try_for_each(|a| self.arg(a))
            }
            // A stub (ADR-051) touches nothing; the value it yields is a slot of its type.
            Expr::TddFit { inner, .. } => self.expr(inner),
            Expr::TddStub { args, ty, .. } => {
                if let Some(t) = ty {
                    self.slot(t)?;
                }
                args.iter().try_for_each(|a| self.expr(a))
            }
            Expr::ThisInstance => Ok(()),
            Expr::Refused(_) => Err("refuses when it runs".into()),
            Expr::Fail { .. } => Ok(()),
            Expr::BaseInstance => Err("uses the base page or report".into()),
            Expr::EnumImpl { .. } => Err("creates the codeunit an enum value implements".into()),
            Expr::Control { .. } => Err("reads a control of a test page".into()),
            Expr::QueryColumn { .. } => Err("reads a query column".into()),
            Expr::FieldOf(base, _) => self.expr(base),
            Expr::DotNet {
                ty: Some(t),
                receiver,
                op: op @ (DotNetOp::Construct | DotNetOp::Call(_)),
                args,
            } if self.mock.contains(t)
                && match op {
                    DotNetOp::Call(m) => m.eq_ignore_ascii_case("Clear") || m.eq_ignore_ascii_case("Dispose"),
                    _ => true,
                } =>
            {
                if let Some(r) = receiver {
                    self.expr(r)?;
                }
                args.iter().try_for_each(|a| self.arg(a))
            }
            Expr::DotNet { .. } | Expr::DotNetCoerce(..) | Expr::DotNetOperand(..) => Err("calls into .NET".into()),
        }
    }
}

fn constant(_: &Const) -> Check {
    Ok(())
}

/// Whether a builtin (a catalog overload id such as `Text.StrLen#1`) works on values in memory.
fn builtin(id: &str) -> Check {
    let method = id.split('#').next().unwrap_or(id);
    let Some((receiver, name)) = method.split_once('.') else {
        return Err(format!("calls the builtin `{id}`"));
    };
    let ok = match receiver {
        "System" => PURE_SYSTEM.iter().any(|m| m.eq_ignore_ascii_case(name)),
        "Guid" => PURE_GUID.iter().any(|m| m.eq_ignore_ascii_case(name)),
        "Database" => PURE_DATABASE.iter().any(|m| m.eq_ignore_ascii_case(name)),
        r => PURE_RECEIVERS.iter().any(|p| p.eq_ignore_ascii_case(r)),
    };
    if ok {
        Ok(())
    } else {
        Err(format!("calls the builtin `{id}`"))
    }
}

/// Whether a slot of this type holds only values in memory.
fn ty(t: &RtType) -> Check {
    match t {
        RtType::Boolean
        | RtType::Integer
        | RtType::BigInteger
        | RtType::Decimal
        | RtType::Byte
        | RtType::Char
        | RtType::Text(_)
        | RtType::Code(_)
        | RtType::Date
        | RtType::Time
        | RtType::DateTime
        | RtType::Duration
        | RtType::Guid
        | RtType::DateFormula
        | RtType::Option(_)
        | RtType::Enum(_)
        | RtType::Variant
        | RtType::Codeunit(_)
        | RtType::Interface(_) => Ok(()),
        RtType::List(t) | RtType::Array(_, t) => ty(t),
        RtType::Dictionary(k, v) => {
            ty(k)?;
            ty(v)
        }
        RtType::Library(n) if PURE_LIBRARY.iter().any(|p| p.eq_ignore_ascii_case(n)) => Ok(()),
        RtType::Library(n) => Err(format!("a variable of type {n}")),
        RtType::Record(..) => Err("a record variable".into()),
        RtType::TestPage(_) | RtType::Page(_) => Err("a page variable".into()),
        RtType::Query(_) => Err("a query variable".into()),
        RtType::DotNet(_) => Err("a DotNet variable".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use al_sema::workspace::AppSource;

    fn verdict(src: &str, tests: &[&str]) -> Option<String> {
        let (ws, l) = al_sema::lower::compile(
            vec![AppSource {
                id: "a".into(),
                name: "a".into(),
                dependencies: vec![],
                files: vec![("f.al".into(), src.into())],
            }],
            &Default::default(),
        );
        assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
        let t: Vec<usize> = l
            .program
            .objects
            .iter()
            .enumerate()
            .filter(|(_, o)| tests.contains(&o.name.as_str()))
            .map(|(i, _)| i)
            .collect();
        assert_eq!(t.len(), tests.len());
        may_observe_baseline(&l, &t)
    }

    const TABLE: &str = "table 60900 S { fields { field(1; K; Code[10]) { } } keys { key(PK; K) { } } }\n";

    /// The pre-check of a lean selection reads the code of every workspace object the selected tests name, not only
    /// the tests' own: a helper with a record makes the lean compile not worth trying (its demand reaches the
    /// dependency apps and the rule says "may observe" after it). Comments and texts do not count, and neither does
    /// code no selected test names.
    #[test]
    fn the_lean_pre_check_reads_the_code_the_tests_name() {
        let looks = |helper: &str| {
            let ws = al_sema::Workspace::load(
                vec![AppSource {
                    id: "a".into(),
                    name: "a".into(),
                    dependencies: vec![],
                    files: vec![(
                        "f.al".into(),
                        format!(
                            "{TABLE}{helper}
                            codeunit 60002 T {{ Subtype = Test; var H: Codeunit H; [Test] procedure P() begin H.Go(); end; }}"
                        ),
                    )],
                }],
                &Default::default(),
            );
            lean_closure_looks_pure(&ws, &[(0, 60002)])
        };
        assert!(looks("codeunit 60001 H { procedure Go() begin Message('x'); end; }"));
        assert!(!looks(
            "codeunit 60001 H { procedure Go() var R: Record S; begin R.DeleteAll(); end; }"
        ));
        assert!(looks(
            "codeunit 60001 H { // the record of it
                procedure Go() begin Message('a report'); end; }"
        ));
        assert!(looks(
            "codeunit 60001 H { procedure Go() begin end; }
            codeunit 60003 Other { procedure X() var R: Record S; begin R.DeleteAll(); end; }"
        ));
    }

    /// BC's test runner runs codeunits 131006, 130301, 130302 and 132553 around every test method when they exist
    /// (runtime/runner_reset.rs): what they reach counts as the test's.
    #[test]
    fn the_test_runners_reset_codeunits_count_as_the_tests_reach() {
        let src = format!(
            "{TABLE}codeunit 130301 \"Reset State Before Test Run\" {{ trigger OnRun() var R: Record S; begin if R.Get('X') then; end; }}
            codeunit 60002 T {{ Subtype = Test; [Test] procedure P() begin end; }}"
        );
        assert!(verdict(&src, &["T"]).is_some_and(|w| w.contains("Reset State Before Test Run")));
    }

    /// Starting, clearing and stopping the permissions mock touch no data (corpus permissions/
    /// TestPermissionsMockEnforcement.al:74-97); assigning a set is refused when it runs.
    #[test]
    fn the_permissions_mock_without_a_set_cannot_observe_the_baseline() {
        let dotnet = "dotnet { assembly(\"Microsoft.Dynamics.Nav.PermissionTestHelper\") { type(\"Microsoft.Dynamics.Nav.Runtime.PermissionTestHelper\"; \"PermissionTestHelper\") { } } }\n";
        let mock = |extra: &str| {
            format!(
                "{dotnet}codeunit 131006 \"Permissions Mock\" {{ SingleInstance = true;
                    trigger OnRun() begin if Started then Stop() else Start(); end;
                    var H: DotNet PermissionTestHelper; Started: Boolean;
                    procedure Start() begin H := H.PermissionTestHelper(); Started := true; end;
                    procedure Stop() begin H.Clear(); H.Dispose(); Started := false; {extra} end; }}
                codeunit 60002 T {{ Subtype = Test; [Test] procedure P() begin end; }}"
            )
        };
        assert_eq!(verdict(&mock(""), &["T"]), None);
        assert!(verdict(&mock("H.AddEffectivePermissionSet('X');"), &["T"]).is_some_and(|w| w.contains(".NET")));
    }

    /// `CompanyName` answers the company the session works in, which the host gives it when it starts (al-vm
    /// sessionops.rs `cx.company`), never a row the install code wrote. The corpus's Probe Observer scrubs it from
    /// every observed value (_fixtures/ProbeObserver.Codeunit.al:81-83), so most of the corpus's tests call it.
    #[test]
    fn the_company_name_of_the_session_cannot_observe_the_baseline() {
        let src = "codeunit 60001 Scrubber { procedure Scrub(V: Text): Text begin
                if CompanyName() <> '' then V := V.Replace(CompanyName(), '<Company>'); exit(V); end; }
            codeunit 60002 T { Subtype = Test; var S: Codeunit Scrubber;
                [Test] procedure P() begin if S.Scrub('x') <> 'x' then Error('x'); end; }";
        assert_eq!(verdict(src, &["T"]), None);
    }

    #[test]
    fn plain_computation_cannot_observe_the_baseline() {
        let src = "enum 60000 Tier { value(0; Low) { } value(1; High) { } }
            codeunit 60001 Logic { procedure Pick(A: Decimal): Enum Tier begin if A > 10 then exit(Tier::High); exit(Tier::Low); end; }
            codeunit 60002 T { Subtype = Test; var L: Codeunit Logic;
                [Test] procedure P() var Items: List of [Text]; B: TextBuilder; begin
                    Items.Add(Format(L.Pick(20))); B.Append(Items.Get(1));
                    if StrLen(B.ToText()) <> 4 then Error('x'); end; }";
        assert_eq!(verdict(src, &["T"]), None);
    }

    #[test]
    fn a_record_variable_used_as_a_whole_may_observe_it() {
        let src = format!(
            "{TABLE}codeunit 60002 T {{ Subtype = Test; [Test] procedure P() var R: Record S; begin R.DeleteAll(); end; }}"
        );
        assert!(verdict(&src, &["T"]).unwrap().contains("Record.DeleteAll"));
        // handing the record to a routine lets that routine do anything with it
        let src = format!(
            "{TABLE}codeunit 60001 H {{ procedure Fill(var R: Record S) begin R.K := 'A'; end; }}
            codeunit 60002 T {{ Subtype = Test; var Hp: Codeunit H; [Test] procedure P() var R: Record S; begin Hp.Fill(R); end; }}"
        );
        assert!(verdict(&src, &["T"]).unwrap().contains("record variable"));
    }

    #[test]
    fn a_record_variable_whose_fields_are_set_and_read_is_a_buffer_in_memory() {
        // Library Assert's TypeNameOf does this: a local `Field` record whose `Type` is assigned and formatted.
        // No row is read or written, so the baseline cannot show.
        let src = format!(
            "{TABLE}codeunit 60001 H {{ procedure Name(): Text var R: Record S; begin R.K := 'A'; exit(Format(R.K)); end; }}
            codeunit 60002 T {{ Subtype = Test; var Hp: Codeunit H;
              [Test] procedure P() var R: Record S; begin if Hp.Name() <> 'A' then Error('x'); end; }}"
        );
        assert_eq!(verdict(&src, &["T"]), None);
    }

    #[test]
    fn a_helper_that_reads_a_table_may_observe_it() {
        let src = format!(
            "{TABLE}codeunit 60001 H {{ procedure N(): Integer var R: Record S; begin exit(R.Count()); end; }}
            codeunit 60002 T {{ Subtype = Test; var Hp: Codeunit H; [Test] procedure P() begin if Hp.N() <> 0 then Error('x'); end; }}"
        );
        let why = verdict(&src, &["T"]).unwrap();
        assert!(why.starts_with("H.N"), "{why}");
    }

    #[test]
    fn a_routine_that_is_never_called_does_not_count() {
        let src = format!(
            "{TABLE}codeunit 60001 H {{ procedure Unused(): Integer var R: Record S; begin exit(R.Count()); end; procedure Used(): Integer begin exit(1); end; }}
            codeunit 60002 T {{ Subtype = Test; var Hp: Codeunit H; [Test] procedure P() begin if Hp.Used() <> 1 then Error('x'); end; }}"
        );
        // The codeunit H is reached through Used; Unused has a record variable, but only
        // the routines called are entered.
        assert_eq!(verdict(&src, &["T"]), None);
    }

    #[test]
    fn create_guid_may_observe_it() {
        let src =
            "codeunit 60002 T { Subtype = Test; [Test] procedure P() var G: Guid; begin G := CreateGuid(); end; }";
        assert!(verdict(src, &["T"]).unwrap().contains("CreateGuid"));
    }

    #[test]
    fn isolated_storage_and_number_sequences_may_observe_it() {
        let a = "codeunit 60002 T { Subtype = Test; [Test] procedure P() var V: Text; begin IsolatedStorage.Get('k', V); end; }";
        assert!(verdict(a, &["T"]).unwrap().contains("IsolatedStorage"));
        let b = "codeunit 60002 T { Subtype = Test; [Test] procedure P() begin NumberSequence.Insert('s'); end; }";
        assert!(verdict(b, &["T"]).unwrap().contains("NumberSequence"));
    }

    #[test]
    fn an_event_publisher_may_observe_it() {
        let src = "codeunit 60001 H { [IntegrationEvent(false, false)] procedure Raise() begin end; }
            codeunit 60002 T { Subtype = Test; var Hp: Codeunit H; [Test] procedure P() begin Hp.Raise(); end; }";
        assert!(verdict(src, &["T"]).unwrap().contains("event publisher"));
    }

    #[test]
    fn running_a_codeunit_by_id_may_observe_it() {
        let src = "codeunit 60001 H { trigger OnRun() begin end; }
            codeunit 60002 T { Subtype = Test; [Test] procedure P() begin Codeunit.Run(60001); end; }";
        assert!(verdict(src, &["T"]).unwrap().contains("Codeunit.Run"));
    }

    #[test]
    fn a_call_through_an_interface_enters_every_implementation() {
        let src = format!(
            "{TABLE}interface IFoo {{ procedure N(): Integer; }}
            codeunit 60001 Pure implements IFoo {{ procedure N(): Integer begin exit(1); end; }}
            codeunit 60003 Dirty implements IFoo {{ procedure N(): Integer var R: Record S; begin exit(R.Count()); end; }}
            codeunit 60002 T {{ Subtype = Test; [Test] procedure P() var F: Interface IFoo; begin F := Pure; if F.N() <> 1 then Error('x'); end; }}"
        );
        let why = verdict(&src, &["T"]).unwrap_or_default();
        assert!(why.contains("Dirty") && why.contains("Record.Count"), "{why}");
    }

    #[test]
    fn only_the_selected_test_codeunits_count() {
        let src = format!(
            "{TABLE}codeunit 60002 T {{ Subtype = Test; [Test] procedure P() begin end; }}
            codeunit 60003 U {{ Subtype = Test; [Test] procedure Q() var R: Record S; begin R.DeleteAll(); end; }}"
        );
        assert_eq!(verdict(&src, &["T"]), None);
        assert!(verdict(&src, &["T", "U"]).is_some());
    }
}
