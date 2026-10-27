//! Audit of the demand-driven compile of dependency apps (ADR-013 decision 6, hard rule 4).
//!
//! Business Central runs code nobody calls by name: table triggers, event subscribers, interface
//! implementations, platform events. The demand-driven compile only compiles what reached code
//! uses, so each such path must either be reached (compiled and run) or refuse the test. A skip
//! that goes unnoticed is a wrong answer.
//!
//! Every probe is a small dependency app plus a workspace test `Go`. The dependency code raises an
//! error with a marker text; the test expects that text. So the outcome says what happened:
//!
//! * `Pass`: the implicit code ran.
//! * `NotRun`: the runtime refused. Allowed where the probe says `RanOrRefused`.
//! * `Fail`: the test ran and the code did not. That is a silent skip, and it is always wrong.
//!
//! Each probe also runs with every dependency routine compiled (`Roots::eager`) and prints both
//! outcomes (`cargo test -p al-test-cli --test demand_audit -- --nocapture`).

use al_sema::workspace::AppSource;
use al_test_cli::report::Outcome;
use al_test_cli::runtime::run_codeunit;
use al_vm::Engine;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Want {
    /// The implicit code is reached and runs.
    Ran,
    /// It runs, or the test is refused. Never a silent skip.
    RanOrRefused,
    /// The demand-driven, the complete and the cached compile give one and the same outcome
    /// (the owner's rule of 2026-10-14: zero differences), and that outcome is a pass.
    SamePass,
}

fn short(o: &Outcome) -> String {
    match o {
        Outcome::Pass => "pass".into(),
        Outcome::Fail(m) => format!("FAIL {m}"),
        Outcome::NotRun(m) => format!("not run: {}", m.chars().take(160).collect::<String>()),
    }
}

struct Probe<'a> {
    name: &'a str,
    want: Want,
    vars: &'a str,
    body: &'a str,
    /// Workspace objects besides the test codeunit.
    extra: &'a str,
    /// An attribute line for `Go` (`[HandlerFunctions('H')]`).
    attr: &'a str,
    /// Members of the test codeunit (handlers).
    members: &'a str,
    dep: &'a [(&'a str, &'a str)],
}

/// How a probe is compiled.
#[derive(Clone, Copy, PartialEq)]
enum Mode {
    /// Only what the workspace reaches in the dependency app (ADR-013).
    Demand,
    /// Every dependency routine.
    Eager,
    /// The dependency app from a compile cache (ADR-022): built by a compile of it alone.
    Cached,
}

/// The outcome of test `Go` over a dependency app `dep` (files) and workspace objects `extra`.
fn go(p: &Probe, mode: Mode) -> Outcome {
    let eager = mode == Mode::Eager;
    let Probe {
        vars: main_vars,
        body,
        extra,
        attr,
        members,
        dep,
        ..
    } = p;
    let main = format!(
        "{extra}\n\
         codeunit 50001 T {{ Subtype = Test;\n\
           local procedure Expect(Marker: Text) begin if StrPos(GetLastErrorText, Marker) = 0 then Error('expected [' + Marker + '] got [' + GetLastErrorText + ']'); end;\n\
           {members}\n\
           {attr} [Test] procedure Go() var {main_vars} begin {body} end; }}"
    );
    let app = |id: &str, deps: &[&str], files: Vec<(String, String)>| AppSource {
        id: id.into(),
        name: id.into(),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files,
    };
    let dep_app = app(
        "dep",
        &[],
        dep.iter().map(|(n, s)| (n.to_string(), s.to_string())).collect(),
    );
    let main_app = app("main", &["dep"], vec![("main.al".into(), main)]);
    let l = if mode == Mode::Cached {
        // The dependency app comes first in the program, which is how a cache is used.
        let set = al_sema::lower::lower_for_cache(vec![dep_app.clone()], &Default::default(), None);
        let Ok((_, l)) = al_sema::lower::compile_apps_cached(
            vec![dep_app, main_app],
            vec![al_sema::AppKind::Dependency, al_sema::AppKind::Workspace],
            &Default::default(),
            None,
            Default::default(),
            std::sync::Arc::new(set),
        ) else {
            panic!("the cache does not fit the apps it was built for");
        };
        l
    } else {
        al_sema::lower::compile_apps_with(
            vec![main_app, dep_app],
            vec![al_sema::AppKind::Workspace, al_sema::AppKind::Dependency],
            &Default::default(),
            None,
            al_sema::check::Roots {
                eager,
                ..Default::default()
            },
        )
        .1
    };
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let engine = Engine::new(l.program.clone(), blocked);
    let oi = l
        .program
        .objects
        .iter()
        .position(|o| o.name == "T")
        .expect("test codeunit T");
    let mut out = run_codeunit(&engine, &l, oi);
    assert_eq!(out.len(), 1);
    out.pop().expect("one test").1
}

fn check(p: Probe) {
    let (name, want) = (p.name, p.want);
    let demand = go(&p, Mode::Demand);
    let eager = go(&p, Mode::Eager);
    let cached = go(&p, Mode::Cached);
    eprintln!(
        "PROBE {name}\n  demand: {}\n  eager:  {}\n  cached: {}",
        short(&demand),
        short(&eager),
        short(&cached)
    );
    // ADR-022: a compile cache gives the outcome of the complete compile, to the letter.
    assert_eq!(
        cached, eager,
        "{name}: the cached compile differs from the complete compile"
    );
    if matches!(want, Want::SamePass) {
        assert_eq!(
            demand, eager,
            "{name}: the demand-driven compile differs from the complete one"
        );
        if want == Want::SamePass {
            assert_eq!(demand, Outcome::Pass, "{name}: the test does not pass");
        }
        return;
    }
    assert!(
        !matches!(demand, Outcome::Fail(_)),
        "{name}: the implicit code was skipped without a refusal: {}",
        short(&demand)
    );
    if want == Want::Ran {
        assert_eq!(demand, Outcome::Pass, "{name}: the implicit code did not run");
    }
    // The complete compile is the reference: what runs there may be refused here, never run
    // differently.
    if matches!(eager, Outcome::Pass) {
        assert!(
            matches!(demand, Outcome::Pass | Outcome::NotRun(_)),
            "{name}: differs from the eager compile: {}",
            short(&demand)
        );
    }
}

macro_rules! probe {
    ($name:ident, $want:expr, vars = $vars:expr, body = $body:expr,
     dep = [$(($file:expr, $src:expr)),* $(,)?]
     $(, extra = $extra:expr)? $(, attr = $attr:expr)? $(, members = $members:expr)? $(,)?) => {
        #[test]
        fn $name() {
            #[allow(unused_mut, unused_assignments)]
            let (mut extra, mut attr, mut members) = ("", "", "");
            $(extra = $extra;)?
            $(attr = $attr;)?
            $(members = $members;)?
            let dep: Vec<(&str, String)> = vec![$(($file, String::from($src))),*];
            let dep: Vec<(&str, &str)> = dep.iter().map(|(f, s)| (*f, s.as_str())).collect();
            check(Probe {
                name: stringify!($name),
                want: $want,
                vars: $vars,
                body: &$body,
                extra,
                attr,
                members,
                dep: &dep,
            });
        }
    };
}

use Want::{Ran, RanOrRefused, SamePass};

// ---------------------------------------------------------------------------------------------
// Fixtures

/// Table `DT` with `trg` in the table body and `field2` in the body of its second field.
fn table(trg: &str, field2: &str) -> String {
    "table 50100 DT { fields { field(1; K; Code[10]) { } field(2; V; Integer) { @F@ } } \
     keys { key(PK; K) { Clustered = true; } } @T@ }"
        .replace("@F@", field2)
        .replace("@T@", trg)
}

/// A subscriber codeunit for `event` of table `DT` (field events: `element`) that raises `marker`.
fn table_sub(event: &str, element: &str, marker: &str) -> String {
    "codeunit 50101 Subs { [EventSubscriber(ObjectType::Table, Database::DT, '@E@', '@L@', false, false)] \
     local procedure S(var Rec: Record DT) begin Error('@M@'); end; }"
        .replace("@E@", event)
        .replace("@L@", element)
        .replace("@M@", marker)
}

const ROW: &str = "R.K := 'a'; R.Insert(false);";

// ---------------------------------------------------------------------------------------------
// 1. Table triggers and events per operation, typed record

probe!(
    insert_runs_the_table_trigger,
    Ran,
    vars = "R: Record DT;",
    body = "R.K := 'a'; asserterror R.Insert(true); Expect('trg:insert');",
    dep = [("t.al", &table("trigger OnInsert() begin Error('trg:insert'); end;", ""))],
);
probe!(
    modify_runs_the_table_trigger,
    Ran,
    vars = "R: Record DT;",
    body = &format!("{ROW} R.V := 1; asserterror R.Modify(true); Expect('trg:modify');"),
    dep = [("t.al", &table("trigger OnModify() begin Error('trg:modify'); end;", ""))],
);
probe!(
    delete_runs_the_table_trigger,
    Ran,
    vars = "R: Record DT;",
    body = &format!("{ROW} asserterror R.Delete(true); Expect('trg:delete');"),
    dep = [("t.al", &table("trigger OnDelete() begin Error('trg:delete'); end;", ""))],
);
probe!(
    rename_runs_the_table_trigger,
    Ran,
    vars = "R: Record DT;",
    body = &format!("{ROW} asserterror R.Rename('b'); Expect('trg:rename');"),
    dep = [("t.al", &table("trigger OnRename() begin Error('trg:rename'); end;", ""))],
);
probe!(
    validate_runs_the_field_trigger,
    Ran,
    vars = "R: Record DT;",
    body = "asserterror R.Validate(V, 1); Expect('trg:validate');",
    dep = [(
        "t.al",
        &table("", "trigger OnValidate() begin Error('trg:validate'); end;")
    )],
);
probe!(
    insert_without_trigger_still_raises_the_before_event,
    Ran,
    vars = "R: Record DT;",
    body = "R.K := 'a'; asserterror R.Insert(false); Expect('sub:insert');",
    dep = [
        ("t.al", &table("", "")),
        ("s.al", &table_sub("OnBeforeInsertEvent", "", "sub:insert"))
    ],
);
probe!(
    modify_raises_the_after_event,
    Ran,
    vars = "R: Record DT;",
    body = &format!("{ROW} asserterror R.Modify(false); Expect('sub:modify');"),
    dep = [
        ("t.al", &table("", "")),
        ("s.al", &table_sub("OnAfterModifyEvent", "", "sub:modify"))
    ],
);
probe!(
    delete_raises_the_before_event,
    Ran,
    vars = "R: Record DT;",
    body = &format!("{ROW} asserterror R.Delete(false); Expect('sub:delete');"),
    dep = [
        ("t.al", &table("", "")),
        ("s.al", &table_sub("OnBeforeDeleteEvent", "", "sub:delete"))
    ],
);
probe!(
    rename_raises_the_after_event,
    Ran,
    vars = "R: Record DT;",
    body = &format!("{ROW} asserterror R.Rename('b'); Expect('sub:rename');"),
    dep = [
        ("t.al", &table("", "")),
        ("s.al", &table_sub("OnAfterRenameEvent", "", "sub:rename"))
    ],
);
probe!(
    validate_raises_the_field_event,
    Ran,
    vars = "R: Record DT;",
    body = "asserterror R.Validate(V, 1); Expect('sub:validate');",
    dep = [
        ("t.al", &table("", "")),
        ("s.al", &table_sub("OnAfterValidateEvent", "V", "sub:validate"))
    ],
);
probe!(
    modifyall_runs_the_modify_trigger,
    Ran,
    vars = "R: Record DT;",
    body = &format!("{ROW} asserterror R.ModifyAll(V, 1, true); Expect('trg:modify');"),
    dep = [("t.al", &table("trigger OnModify() begin Error('trg:modify'); end;", ""))],
);
probe!(
    deleteall_runs_the_delete_trigger,
    Ran,
    vars = "R: Record DT;",
    body = &format!("{ROW} asserterror R.DeleteAll(true); Expect('trg:delete');"),
    dep = [("t.al", &table("trigger OnDelete() begin Error('trg:delete'); end;", ""))],
);
probe!(
    truncate_is_refused_by_a_delete_event_subscriber,
    Ran,
    vars = "R: Record DT;",
    body = "asserterror R.Truncate(); if StrPos(GetLastErrorText, 'Truncate is not supported when') = 0 then Error('got [' + GetLastErrorText + ']');",
    dep = [
        ("t.al", &table("", "")),
        ("s.al", &table_sub("OnAfterDeleteEvent", "", "sub:delete"))
    ],
);
probe!(
    insert_with_run_trigger_false_runs_the_event_and_not_the_trigger,
    Ran,
    vars = "R: Record DT;",
    body = "R.K := 'a'; asserterror R.Insert(false); Expect('sub:insert');",
    dep = [
        ("t.al", &table("trigger OnInsert() begin Error('trg:insert'); end;", "")),
        ("s.al", &table_sub("OnAfterInsertEvent", "", "sub:insert"))
    ],
);

// ---------------------------------------------------------------------------------------------
// 2. Table extension triggers, field triggers across apps

const EXT_ON_INSERT: &str =
    "tableextension 50100 DTX extends DT { trigger OnInsert() begin Error('ext:insert'); end; }";
const EXT_FIELD: &str = "tableextension 50100 DTX extends DT { fields { field(50000; Extra; Integer) { \
     trigger OnValidate() begin Error('ext:validate'); end; } } }";
const EXT_MODIFY_FIELD: &str = "tableextension 50100 DTX extends DT { fields { modify(V) { \
     trigger OnAfterValidate() begin Error('ext:after'); end; } } }";

probe!(
    a_table_extension_trigger_runs,
    Ran,
    vars = "R: Record DT;",
    body = "R.K := 'a'; asserterror R.Insert(true); Expect('ext:insert');",
    dep = [("t.al", &table("", "")), ("x.al", EXT_ON_INSERT)],
);
probe!(
    the_before_and_after_triggers_of_a_table_extension_run,
    Ran,
    vars = "R: Record DT;",
    body = "R.K := 'a'; asserterror R.Insert(true); Expect('ext:before'); R.Insert(false); \
            asserterror R.Delete(true); Expect('ext:beforedelete');",
    dep = [
        ("t.al", &table("", "")),
        (
            "x.al",
            "tableextension 50100 DTX extends DT { trigger OnBeforeInsert() begin Error('ext:before'); end; \
             trigger OnBeforeDelete() begin Error('ext:beforedelete'); end; }"
        )
    ],
);
probe!(
    a_field_added_by_a_table_extension_runs_its_validate_trigger,
    Ran,
    vars = "R: Record DT;",
    body = "asserterror R.Validate(Extra, 1); Expect('ext:validate');",
    dep = [("t.al", &table("", "")), ("x.al", EXT_FIELD)],
);
probe!(
    a_table_extension_modify_block_brackets_the_field_validate,
    Ran,
    vars = "R: Record DT;",
    body = "asserterror R.Validate(V, 1); Expect('ext:after');",
    dep = [("t.al", &table("", "")), ("x.al", EXT_MODIFY_FIELD)],
);
probe!(
    a_workspace_table_extension_runs_its_trigger_on_a_dependency_table,
    Ran,
    vars = "R: Record DT;",
    body = "R.K := 'a'; asserterror R.Insert(true); Expect('wsext:insert');",
    dep = [("t.al", &table("", ""))],
    extra = "tableextension 50900 WsX extends DT { trigger OnInsert() begin Error('wsext:insert'); end; }",
);

// ---------------------------------------------------------------------------------------------
// 3. Writes the checker cannot see as a write on a typed record

probe!(
    a_bare_modify_inside_table_code_runs_the_trigger,
    Ran,
    vars = "R: Record DT;",
    body = &format!("{ROW} asserterror R.Bump(); Expect('trg:modify');"),
    dep = [(
        "t.al",
        &table(
            "procedure Bump() begin V := 1; Modify(true); end; trigger OnModify() begin Error('trg:modify'); end;",
            ""
        )
    )],
);
probe!(
    a_bare_insert_inside_table_code_raises_the_event,
    Ran,
    vars = "R: Record DT;",
    body = "asserterror R.Add(); Expect('sub:insert');",
    dep = [
        (
            "t.al",
            &table("procedure Add() begin K := 'z'; Insert(false); end;", "")
        ),
        ("s.al", &table_sub("OnAfterInsertEvent", "", "sub:insert"))
    ],
);
probe!(
    an_event_published_by_a_table_extension_reaches_its_subscribers,
    Ran,
    vars = "R: Record DT;",
    body = "asserterror R.Fire(); Expect('sub:ext');",
    dep = [
        ("t.al", &table("", "")),
        (
            "x.al",
            "tableextension 50100 DTX extends DT { procedure Fire() begin OnThing(); end; \
             [IntegrationEvent(false, false)] local procedure OnThing() begin end; }"
        ),
        (
            "s.al",
            "codeunit 50101 Subs { [EventSubscriber(ObjectType::Table, Database::DT, 'OnThing', '', false, false)] \
             local procedure S() begin Error('sub:ext'); end; }"
        ),
    ],
);

// ---------------------------------------------------------------------------------------------
// 4. RecordRef and FieldRef: the table is known when the code runs

probe!(
    recordref_insert_on_a_table_named_by_database_runs_the_trigger_or_refuses,
    RanOrRefused,
    vars = "RR: RecordRef;",
    body = "RR.Open(Database::DT); RR.Field(1).Value := 'a'; asserterror RR.Insert(true); Expect('trg:insert');",
    dep = [("t.al", &table("trigger OnInsert() begin Error('trg:insert'); end;", ""))],
);
probe!(
    recordref_insert_on_a_run_time_table_id_refuses,
    RanOrRefused,
    vars = "RR: RecordRef; Id: Integer;",
    body = "Id := 50100; RR.Open(Id); RR.Field(1).Value := 'a'; asserterror RR.Insert(true); Expect('trg:insert');",
    dep = [("t.al", &table("trigger OnInsert() begin Error('trg:insert'); end;", ""))],
);
probe!(
    fieldref_validate_runs_the_field_trigger_or_refuses,
    RanOrRefused,
    vars = "RR: RecordRef;",
    body = "RR.Open(Database::DT); asserterror RR.Field(2).Validate(1); Expect('trg:validate');",
    dep = [(
        "t.al",
        &table("", "trigger OnValidate() begin Error('trg:validate'); end;")
    )],
);
probe!(
    recordref_from_a_typed_record_modify_runs_the_trigger_or_refuses,
    RanOrRefused,
    vars = "R: Record DT; RR: RecordRef;",
    body = &format!("{ROW} RR.GetTable(R); asserterror RR.Modify(true); Expect('trg:modify');"),
    dep = [("t.al", &table("trigger OnModify() begin Error('trg:modify'); end;", ""))],
);
probe!(
    recordref_insert_raises_the_event_or_refuses,
    RanOrRefused,
    vars = "RR: RecordRef;",
    body = "RR.Open(Database::DT); RR.Field(1).Value := 'a'; asserterror RR.Insert(false); Expect('sub:insert');",
    dep = [
        ("t.al", &table("", "")),
        ("s.al", &table_sub("OnAfterInsertEvent", "", "sub:insert"))
    ],
);
probe!(
    recordref_deleteall_runs_the_delete_trigger_or_refuses,
    RanOrRefused,
    vars = "R: Record DT; RR: RecordRef;",
    body = &format!("{ROW} RR.GetTable(R); asserterror RR.DeleteAll(true); Expect('trg:delete');"),
    dep = [("t.al", &table("trigger OnDelete() begin Error('trg:delete'); end;", ""))],
);

// ---------------------------------------------------------------------------------------------
// 5. Rename propagation, FlowFields, TableRelation

const CHILD: &str = "table 50102 DC { fields { field(1; Id; Integer) { } field(2; Ref; Code[10]) { TableRelation = DT.K; } } \
     keys { key(PK; Id) { Clustered = true; } } }";

probe!(
    rename_propagates_to_a_related_dependency_table,
    Ran,
    vars = "P: Record DT; C: Record DC;",
    body = "P.K := 'a'; P.Insert(false); C.Id := 1; C.Ref := 'a'; C.Insert(false); P.Rename('b'); C.Get(1); \
            if C.Ref <> 'B' then Error('not propagated: ' + C.Ref);",
    dep = [("t.al", &table("", "")), ("c.al", CHILD)],
);
probe!(
    a_flowfield_sums_rows_of_a_dependency_table,
    Ran,
    vars = "P: Record DT; A: Record DA;",
    body = "P.K := 'a'; P.V := 2; P.Insert(false); P.K := 'b'; P.V := 3; P.Insert(false); A.Id := 1; A.Insert(false); \
            A.CalcFields(Total); if A.Total <> 5 then Error('total ' + Format(A.Total));",
    dep = [
        ("t.al", &table("", "")),
        (
            "a.al",
            "table 50103 DA { fields { field(1; Id; Integer) { } field(2; Total; Integer) { FieldClass = FlowField; CalcFormula = sum(DT.V); } } \
             keys { key(PK; Id) { Clustered = true; } } }"
        )
    ],
);
probe!(
    validate_checks_the_table_relation,
    Ran,
    vars = "C: Record DC;",
    body = "asserterror C.Validate(Ref, 'zz'); if StrPos(GetLastErrorText, 'cannot be found in the related table') = 0 then Error('got [' + GetLastErrorText + ']');",
    dep = [("t.al", &table("", "")), ("c.al", CHILD)],
);
probe!(
    fieldref_validate_checks_the_relation_of_a_table_known_only_by_database,
    RanOrRefused,
    vars = "RR: RecordRef;",
    body = "RR.Open(Database::DC); asserterror RR.Field(2).Validate('zz'); if GetLastErrorText = '' then Error('validated without a check');",
    dep = [("t.al", &table("", "")), ("c.al", CHILD)],
);

// ---------------------------------------------------------------------------------------------
// 6. Events raised by reached publishers

const PUB: &str = "codeunit 50110 Pub { procedure Do() begin OnThing(); end; \
     [IntegrationEvent(false, false)] local procedure OnThing() begin end; }";

fn code_sub(attr: &str, params: &str, marker: &str) -> String {
    "codeunit 50111 Subs { @A@ local procedure S(@P@) begin Error('@M@'); end; }"
        .replace("@A@", attr)
        .replace("@P@", params)
        .replace("@M@", marker)
}

probe!(
    an_integration_event_reaches_its_subscriber,
    Ran,
    vars = "P: Codeunit Pub;",
    body = "asserterror P.Do(); Expect('sub:event');",
    dep = [
        ("p.al", PUB),
        (
            "s.al",
            &code_sub(
                "[EventSubscriber(ObjectType::Codeunit, Codeunit::Pub, 'OnThing', '', false, false)]",
                "",
                "sub:event"
            )
        )
    ],
);
probe!(
    business_and_internal_events_reach_their_subscribers,
    Ran,
    vars = "P: Codeunit Pub;",
    body = "asserterror P.Do(); Expect('sub:business'); asserterror P.Do2(); Expect('sub:internal');",
    dep = [
        (
            "p.al",
            "codeunit 50110 Pub { procedure Do() begin OnBiz(); end; procedure Do2() begin OnInt(); end; \
             [BusinessEvent(false)] local procedure OnBiz() begin end; \
             [InternalEvent(false)] local procedure OnInt() begin end; }"
        ),
        (
            "s.al",
            "codeunit 50111 Subs { \
             [EventSubscriber(ObjectType::Codeunit, Codeunit::Pub, 'OnBiz', '', false, false)] local procedure A() begin Error('sub:business'); end; \
             [EventSubscriber(ObjectType::Codeunit, Codeunit::Pub, 'OnInt', '', false, false)] local procedure B() begin Error('sub:internal'); end; }"
        )
    ],
);
probe!(
    include_sender_passes_the_publisher_instance,
    Ran,
    vars = "P: Codeunit Pub;",
    body = "asserterror P.Do(); Expect('sub:sender');",
    dep = [
        (
            "p.al",
            "codeunit 50110 Pub { procedure Do() begin OnThing(); end; \
             [IntegrationEvent(true, false)] local procedure OnThing() begin end; }"
        ),
        (
            "s.al",
            &code_sub(
                "[EventSubscriber(ObjectType::Codeunit, Codeunit::Pub, 'OnThing', '', false, false)]",
                "Sender: Codeunit Pub",
                "sub:sender"
            )
        )
    ],
);
probe!(
    an_isolated_event_swallows_the_subscriber_error_into_the_last_error,
    Ran,
    vars = "P: Codeunit Pub;",
    body = "P.Do(); Expect('sub:iso');",
    dep = [
        (
            "p.al",
            "codeunit 50110 Pub { procedure Do() begin OnThing(); end; \
             [IntegrationEvent(false, false, true)] local procedure OnThing() begin end; }"
        ),
        (
            "s.al",
            &code_sub(
                "[EventSubscriber(ObjectType::Codeunit, Codeunit::Pub, 'OnThing', '', false, false)]",
                "",
                "sub:iso"
            )
        )
    ],
);
probe!(
    a_manual_subscriber_fires_only_while_bound,
    Ran,
    vars = "P: Codeunit Pub; S: Codeunit Subs;",
    body = "P.Do(); BindSubscription(S); asserterror P.Do(); Expect('sub:manual'); UnbindSubscription(S); P.Do();",
    dep = [
        ("p.al", PUB),
        (
            "s.al",
            "codeunit 50111 Subs { EventSubscriberInstance = Manual; \
             [EventSubscriber(ObjectType::Codeunit, Codeunit::Pub, 'OnThing', '', false, false)] \
             local procedure S() begin Error('sub:manual'); end; }"
        )
    ],
);
probe!(
    a_single_instance_subscriber_reaches_its_event,
    Ran,
    vars = "P: Codeunit Pub;",
    body = "asserterror P.Do(); Expect('sub:single');",
    dep = [
        ("p.al", PUB),
        (
            "s.al",
            "codeunit 50111 Subs { SingleInstance = true; \
             [EventSubscriber(ObjectType::Codeunit, Codeunit::Pub, 'OnThing', '', false, false)] \
             local procedure S() begin Error('sub:single'); end; }"
        )
    ],
);
probe!(
    a_subscriber_that_names_its_publisher_with_a_namespace_is_attached,
    Ran,
    vars = "P: Codeunit Dep.Ns.Pub;",
    body = "asserterror P.Do(); Expect('sub:ns');",
    dep = [
        (
            "p.al",
            "namespace Dep.Ns;\n\
             codeunit 50110 Pub { procedure Do() begin OnThing(); end; \
             [IntegrationEvent(false, false)] local procedure OnThing() begin end; }"
        ),
        (
            "s.al",
            "codeunit 50111 Subs { \
             [EventSubscriber(ObjectType::Codeunit, Codeunit::Dep.Ns.Pub, 'OnThing', '', false, false)] \
             local procedure S() begin Error('sub:ns'); end; }"
        )
    ],
    extra = "using Dep.Ns;",
);

// ---------------------------------------------------------------------------------------------
// 7. Codeunit.Run, Page.Run, Report.Run and the rest of the objects run by number

const RUN_CU: &str =
    "codeunit 50120 DepCu { trigger OnRun() begin Error('ran:onrun'); end; procedure Hello() begin end; }";

probe!(
    codeunit_run_with_a_constant_id_runs_on_run,
    Ran,
    vars = "",
    body = "asserterror Codeunit.Run(Codeunit::DepCu); Expect('ran:onrun');",
    dep = [("c.al", RUN_CU)],
);
probe!(
    run_on_a_codeunit_variable_runs_on_run,
    Ran,
    vars = "C: Codeunit DepCu;",
    body = "asserterror C.Run(); Expect('ran:onrun');",
    dep = [("c.al", RUN_CU)],
);
probe!(
    codeunit_run_with_a_run_time_id_refuses,
    RanOrRefused,
    vars = "Id: Integer;",
    body = "Id := 50120; asserterror Codeunit.Run(Id); Expect('ran:onrun');",
    dep = [("c.al", RUN_CU)],
);
probe!(
    codeunit_run_with_a_run_time_id_of_a_codeunit_used_otherwise_refuses,
    RanOrRefused,
    vars = "Id: Integer; C: Codeunit DepCu;",
    body = "C.Hello(); Id := 50120; asserterror Codeunit.Run(Id); Expect('ran:onrun');",
    dep = [("c.al", RUN_CU)],
);
probe!(
    page_run_with_a_run_time_id_refuses,
    RanOrRefused,
    vars = "Id: Integer;",
    body = "Id := 50121; asserterror Page.Run(Id); Expect('page:open');",
    dep = [(
        "p.al",
        "page 50121 DP { PageType = Card; trigger OnOpenPage() begin Error('page:open'); end; }"
    )],
);
probe!(
    page_run_with_a_page_constant_runs_the_page_or_refuses,
    RanOrRefused,
    vars = "",
    body = "asserterror Page.Run(Page::DP); Expect('page:open');",
    dep = [(
        "p.al",
        "page 50121 DP { PageType = Card; trigger OnOpenPage() begin Error('page:open'); end; }"
    )],
    attr = "[HandlerFunctions('H')]",
    members = "[PageHandler] procedure H(var P: TestPage DP) begin end;",
);
probe!(
    run_modal_without_an_id_opens_the_lookup_page_of_the_table_or_refuses,
    RanOrRefused,
    vars = "R: Record DT;",
    // BC opens the table's lookup page, a dependency page nothing here names: the runtime must not
    // answer that no page has id 0 (the error text of a page that does not exist).
    body = "asserterror Page.RunModal(0, R); if StrPos(GetLastErrorText, 'invoke the Page object with the ID') > 0 then Error('wrong error: ' + GetLastErrorText);",
    dep = [
        (
            "t.al",
            &table("", "").replace("fields {", "LookupPageId = DP; fields {")
        ),
        ("p.al", DEP_PAGE_PLAIN)
    ],
);
probe!(
    report_run_with_a_run_time_id_refuses,
    RanOrRefused,
    vars = "Id: Integer;",
    body = "Id := 50122; asserterror Report.Run(Id); Expect('rep:pre');",
    dep = [(
        "r.al",
        "report 50122 DR { ProcessingOnly = true; UseRequestPage = false; trigger OnPreReport() begin Error('rep:pre'); end; }"
    )],
);
probe!(
    a_report_variable_run_executes_the_report_triggers_or_refuses,
    RanOrRefused,
    vars = "Rep: Report DR;",
    body = "asserterror Rep.Run(); Expect('rep:pre');",
    dep = [(
        "r.al",
        "report 50122 DR { ProcessingOnly = true; UseRequestPage = false; trigger OnPreReport() begin Error('rep:pre'); end; }"
    )],
);
probe!(
    report_run_with_a_report_constant_executes_the_triggers_or_refuses,
    RanOrRefused,
    vars = "",
    body = "asserterror Report.Run(Report::DR); Expect('rep:pre');",
    dep = [(
        "r.al",
        "report 50122 DR { ProcessingOnly = true; UseRequestPage = false; trigger OnPreReport() begin Error('rep:pre'); end; }"
    )],
);
probe!(
    xmlport_run_with_a_run_time_id_refuses,
    RanOrRefused,
    vars = "Id: Integer;",
    body = "Id := 50123; asserterror Xmlport.Run(Id); Expect('xp:pre');",
    dep = [(
        "x.al",
        "xmlport 50123 DX { Direction = Export; Format = Xml; UseRequestPage = false; schema { textelement(Root) { } } \
         trigger OnPreXmlPort() begin Error('xp:pre'); end; }"
    )],
);
probe!(
    an_xmlport_variable_export_executes_the_triggers_or_refuses,
    RanOrRefused,
    vars = "Xp: XmlPort DX; S: OutStream; T: Codeunit \"Temp Blob Probe\";",
    body = "Xp.SetDestination(S); asserterror Xp.Export(); Expect('xp:pre');",
    dep = [
        (
            "x.al",
            "xmlport 50123 DX { Direction = Export; Format = Xml; UseRequestPage = false; schema { textelement(Root) { } } \
             trigger OnPreXmlPort() begin Error('xp:pre'); end; }"
        ),
        ("t.al", "codeunit 50124 \"Temp Blob Probe\" { }")
    ],
);

// ---------------------------------------------------------------------------------------------
// 8. Pages: triggers and the platform's page events

const DEP_PAGE: &str = "page 50130 DP { PageType = Card; SourceTable = DT; \
     layout { area(Content) { field(KCtl; Rec.K) { } } } \
     actions { area(Processing) { action(Act) { trigger OnAction() begin end; } } } \
     trigger OnOpenPage() begin Error('page:open'); end; }";
const DEP_PAGE_PLAIN: &str = "page 50130 DP { PageType = Card; SourceTable = DT; \
     layout { area(Content) { field(KCtl; Rec.K) { } } } }";

fn page_sub(event: &str, element: &str, marker: &str) -> String {
    "codeunit 50131 Subs { [EventSubscriber(ObjectType::Page, Page::DP, '@E@', '@L@', false, false)] \
     local procedure S() begin Error('@M@'); end; }"
        .replace("@E@", event)
        .replace("@L@", element)
        .replace("@M@", marker)
}

probe!(
    a_test_page_runs_the_triggers_of_a_dependency_page,
    Ran,
    vars = "TP: TestPage DP;",
    body = "asserterror TP.OpenNew(); Expect('page:open');",
    dep = [("t.al", &table("", "")), ("p.al", DEP_PAGE)],
);
probe!(
    the_open_page_event_of_a_dependency_page_reaches_its_subscriber,
    Ran,
    vars = "TP: TestPage DP;",
    body = "asserterror TP.OpenNew(); Expect('sub:pageopen');",
    dep = [
        ("t.al", &table("", "")),
        ("p.al", DEP_PAGE_PLAIN),
        ("s.al", &page_sub("OnOpenPageEvent", "", "sub:pageopen"))
    ],
);
probe!(
    the_after_get_record_event_of_a_dependency_page_runs_or_refuses,
    RanOrRefused,
    vars = "TP: TestPage DP; R: Record DT;",
    body = &format!("{ROW} asserterror TP.OpenView(); Expect('sub:agr');"),
    dep = [
        ("t.al", &table("", "")),
        ("p.al", DEP_PAGE_PLAIN),
        ("s.al", &page_sub("OnAfterGetRecordEvent", "", "sub:agr"))
    ],
);
probe!(
    the_field_validate_event_of_a_dependency_page_runs_or_refuses,
    RanOrRefused,
    vars = "TP: TestPage DP;",
    body = "TP.OpenNew(); asserterror TP.KCtl.SetValue('x'); Expect('sub:pagevalidate');",
    dep = [
        ("t.al", &table("", "")),
        ("p.al", DEP_PAGE_PLAIN),
        ("s.al", &page_sub("OnAfterValidateEvent", "K", "sub:pagevalidate"))
    ],
);
probe!(
    the_action_event_of_a_dependency_page_runs_or_refuses,
    RanOrRefused,
    vars = "TP: TestPage DP;",
    body = "TP.OpenNew(); asserterror TP.Act.Invoke(); Expect('sub:action');",
    dep = [
        ("t.al", &table("", "")),
        (
            "p.al",
            DEP_PAGE_PLAIN
                .replace("layout", "actions { area(Processing) { action(Act) { } } } layout")
                .as_str()
        ),
        ("s.al", &page_sub("OnBeforeActionEvent", "Act", "sub:action"))
    ],
);
probe!(
    a_lookup_on_a_workspace_page_runs_the_table_field_on_lookup,
    Ran,
    vars = "TP: TestPage WsDt;",
    body = "TP.OpenNew(); asserterror TP.VCtl.Lookup(); Expect('trg:lookup');",
    dep = [("t.al", &table("", "trigger OnLookup() begin Error('trg:lookup'); end;"))],
    extra = WS_DT_PAGE,
);
probe!(
    a_lookup_on_a_dependency_page_runs_the_table_field_on_lookup,
    Ran,
    vars = "TP: TestPage DP;",
    body = "TP.OpenNew(); asserterror TP.KCtl.Lookup(); Expect('trg:lookup');",
    dep = [
        (
            "t.al",
            &table("", "").replace(
                "field(1; K; Code[10]) { }",
                "field(1; K; Code[10]) { trigger OnLookup() begin Error('trg:lookup'); end; }"
            )
        ),
        ("p.al", DEP_PAGE_PLAIN)
    ],
);
probe!(
    a_caption_class_resolver_in_a_dependency_app_runs_or_refuses,
    RanOrRefused,
    vars = "TP: TestPage WsCard;",
    body = "asserterror TP.OpenView(); Expect('sub:caption');",
    dep = [(
        "s.al",
        "codeunit 50140 Resolver { [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Caption Class\", 'OnResolveCaptionClass', '', false, false)] \
         local procedure OnResolve(CaptionArea: Text; CaptionExpr: Text; Language: Integer; var Caption: Text; var Resolved: Boolean) \
         begin if CaptionExpr.Contains('BOOM') then Error('sub:caption'); end; }"
    )],
    extra = "table 50910 WsRow { fields { field(1; K; Code[10]) { } field(2; A; Decimal) { } } keys { key(PK; K) { Clustered = true; } } }\n\
             page 50910 WsCard { PageType = Card; SourceTable = WsRow; layout { area(Content) { \
             field(KCtl; Rec.K) { } field(BoomCtl; Rec.A) { CaptionClass = 'ZZ,BOOM'; } } } }\n\
             codeunit 50911 Seed { }",
);

// ---------------------------------------------------------------------------------------------
// 9. Global Triggers (the platform's database triggers)

const WS_DT_PAGE: &str = "page 50970 WsDt { PageType = Card; SourceTable = DT; \
     layout { area(Content) { field(KCtl; Rec.K) { } field(VCtl; Rec.V) { } } } }";

const GT: &str = "codeunit 50150 Gt {\n\
     [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Global Triggers\", 'GetDatabaseTableTriggerSetup', '', false, false)]\n\
     local procedure Setup(TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean; var OnDatabaseDelete: Boolean; var OnDatabaseRename: Boolean) \
     begin OnDatabaseInsert := true; OnDatabaseModify := true; OnDatabaseDelete := true; OnDatabaseRename := true; end;\n\
     [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Global Triggers\", 'OnDatabaseInsert', '', false, false)]\n\
     local procedure Ins(RecRef: RecordRef) begin Error('gt:insert'); end;\n\
     [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Global Triggers\", 'OnDatabaseModify', '', false, false)]\n\
     local procedure Upd(RecRef: RecordRef) begin Error('gt:modify'); end;\n\
     [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Global Triggers\", 'OnDatabaseDelete', '', false, false)]\n\
     local procedure Del(RecRef: RecordRef) begin Error('gt:delete'); end; }";
const WS_TABLE: &str = "table 50920 WsRow { fields { field(1; K; Code[10]) { } field(2; V; Integer) { } } \
     keys { key(PK; K) { Clustered = true; } } \
     procedure Bump() begin V := 1; Modify(); end; }";

probe!(
    a_typed_insert_raises_the_global_insert_event,
    Ran,
    vars = "R: Record WsRow;",
    body = "R.K := 'a'; asserterror R.Insert(); Expect('gt:insert');",
    dep = [("g.al", GT)],
    extra = WS_TABLE,
);
/// `Global Triggers` subscribers that enable and observe `OnDatabaseModify` only.
const GT_MODIFY: &str = "codeunit 50151 GtModify {\n\
     [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Global Triggers\", 'GetDatabaseTableTriggerSetup', '', false, false)]\n\
     local procedure Setup(TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean; var OnDatabaseDelete: Boolean; var OnDatabaseRename: Boolean) \
     begin OnDatabaseModify := true; end;\n\
     [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Global Triggers\", 'OnDatabaseModify', '', false, false)]\n\
     local procedure Upd(RecRef: RecordRef) begin Error('gt:modify'); end; }";

probe!(
    a_recordref_modify_raises_the_global_modify_event,
    Ran,
    vars = "R: Record WsRow; RR: RecordRef;",
    body =
        "R.K := 'a'; R.Insert(); RR.GetTable(R); RR.Field(2).Value := 5; asserterror RR.Modify(); Expect('gt:modify');",
    dep = [("g.al", GT_MODIFY)],
    extra = WS_TABLE,
);
probe!(
    a_bare_modify_in_workspace_table_code_raises_the_global_modify_event,
    Ran,
    vars = "R: Record WsRow;",
    body = "R.K := 'a'; R.Insert(); asserterror R.Bump(); Expect('gt:modify');",
    dep = [("g.al", GT_MODIFY)],
    extra = WS_TABLE,
);
probe!(
    a_workspace_page_over_a_dependency_table_runs_its_insert_trigger,
    Ran,
    vars = "TP: TestPage WsDt;",
    body = "TP.OpenNew(); TP.KCtl.SetValue('a'); asserterror TP.Close(); Expect('trg:insert');",
    dep = [("t.al", &table("trigger OnInsert() begin Error('trg:insert'); end;", ""))],
    extra = WS_DT_PAGE,
);
probe!(
    a_workspace_page_over_a_dependency_table_runs_the_field_trigger_and_events,
    Ran,
    vars = "TP: TestPage WsDt;",
    body = "TP.OpenNew(); asserterror TP.VCtl.SetValue(1); Expect('sub:validate');",
    dep = [
        ("t.al", &table("", "")),
        ("s.al", &table_sub("OnBeforeValidateEvent", "V", "sub:validate"))
    ],
    extra = WS_DT_PAGE,
);
probe!(
    a_workspace_page_over_a_dependency_table_raises_the_global_insert_event,
    Ran,
    vars = "TP: TestPage WsDt;",
    body = "TP.OpenNew(); TP.KCtl.SetValue('a'); asserterror TP.Close(); Expect('gt:insert');",
    dep = [("t.al", &table("", "")), ("g.al", GT)],
    extra = WS_DT_PAGE,
);

// ---------------------------------------------------------------------------------------------
// 10. Interface and enum dispatch

const IFACE: &str = "interface IThing { procedure Run(); }\n\
     codeunit 50160 ImplA implements IThing { procedure Run() begin Error('impl:A'); end; }\n\
     codeunit 50161 ImplB implements IThing { procedure Run() begin Error('impl:B'); end; }\n\
     codeunit 50162 ImplU implements IThing { procedure Run() begin Error('impl:U'); end; }";

probe!(
    an_enum_value_without_its_own_implementation_takes_the_default,
    Ran,
    vars = "E: Enum DE; I: Interface IThing;",
    body = "E := E::One; I := E; asserterror I.Run(); Expect('impl:A');",
    dep = [
        ("i.al", IFACE),
        (
            "e.al",
            "enum 50160 DE implements IThing { DefaultImplementation = IThing = ImplA; Extensible = true; \
             value(0; Zero) { } value(1; One) { } }"
        )
    ],
);
probe!(
    an_enum_value_with_its_own_implementation_takes_it,
    Ran,
    vars = "E: Enum DE; I: Interface IThing;",
    body = "E := E::Two; I := E; asserterror I.Run(); Expect('impl:B');",
    dep = [
        ("i.al", IFACE),
        (
            "e.al",
            "enum 50160 DE implements IThing { DefaultImplementation = IThing = ImplA; \
             value(0; Zero) { } value(2; Two) { Implementation = IThing = ImplB; } }"
        )
    ],
);
probe!(
    an_enum_extension_value_brings_its_own_implementation,
    Ran,
    vars = "E: Enum DE; I: Interface IThing;",
    body = "E := E::Three; I := E; asserterror I.Run(); Expect('impl:B');",
    dep = [
        ("i.al", IFACE),
        (
            "e.al",
            "enum 50160 DE implements IThing { Extensible = true; DefaultImplementation = IThing = ImplA; value(0; Zero) { } }"
        ),
        (
            "x.al",
            "enumextension 50160 DEX extends DE { value(3; Three) { Implementation = IThing = ImplB; } }"
        )
    ],
);
probe!(
    an_unknown_ordinal_takes_the_unknown_value_implementation,
    Ran,
    vars = "E: Enum DE; I: Interface IThing;",
    body = "E := Enum::DE.FromInteger(77); I := E; asserterror I.Run(); Expect('impl:U');",
    dep = [
        ("i.al", IFACE),
        (
            "e.al",
            "enum 50160 DE implements IThing { Extensible = true; DefaultImplementation = IThing = ImplA; \
             UnknownValueImplementation = IThing = ImplU; value(0; Zero) { } }"
        )
    ],
);
probe!(
    a_codeunit_assigned_to_an_interface_runs_its_procedure,
    Ran,
    vars = "C: Codeunit ImplA; I: Interface IThing;",
    body = "I := C; asserterror I.Run(); Expect('impl:A');",
    dep = [("i.al", IFACE)],
);

// ---------------------------------------------------------------------------------------------
// 11. SingleInstance, RunObject, background tasks, task scheduler

probe!(
    a_single_instance_codeunit_keeps_its_state_across_variables,
    Ran,
    vars = "A: Codeunit Counter; B: Codeunit Counter;",
    body = "A.Bump(); B.Bump(); if B.Get() <> 2 then Error('not shared');",
    dep = [(
        "c.al",
        "codeunit 50170 Counter { SingleInstance = true; var N: Integer; procedure Bump() begin N += 1; end; procedure Get(): Integer begin exit(N); end; }"
    )],
);
probe!(
    a_codeunit_run_object_action_runs_on_run,
    Ran,
    vars = "TP: TestPage WsHost;",
    body = "TP.OpenView(); asserterror TP.RunIt.Invoke(); Expect('ran:onrun');",
    dep = [("c.al", RUN_CU)],
    extra = "page 50930 WsHost { PageType = Card; layout { area(Content) { } } \
             actions { area(Processing) { action(RunIt) { RunObject = Codeunit DepCu; } } } }",
);
probe!(
    a_page_run_object_action_opens_a_dependency_page_or_refuses,
    RanOrRefused,
    vars = "TP: TestPage WsHost;",
    body = "TP.OpenView(); asserterror TP.RunIt.Invoke(); Expect('page:open');",
    dep = [("t.al", &table("", "")), ("p.al", DEP_PAGE)],
    extra = "page 50930 WsHost { PageType = Card; layout { area(Content) { } } \
             actions { area(Processing) { action(RunIt) { RunObject = Page DP; } } } }",
);
probe!(
    a_page_background_task_of_a_dependency_codeunit_runs_or_refuses,
    RanOrRefused,
    vars = "TP: TestPage WsBg;",
    body = "asserterror TP.OpenView(); Expect('ran:onrun');",
    dep = [("c.al", RUN_CU)],
    extra = "page 50940 WsBg { PageType = Card; layout { area(Content) { } } \
             trigger OnOpenPage() var TaskId: Integer; begin CurrPage.EnqueueBackgroundTask(TaskId, 50120); end; }",
);
probe!(
    a_task_scheduler_task_for_a_dependency_codeunit_is_created_and_not_run_in_either_compile,
    Ran,
    vars = "",
    // A task starts in a session of its own after the creating transaction commits, which a test does
    // not wait for (docs/probes-needed.md, `TaskScheduler.CreateTask`). So nothing runs, in the
    // eager compile too: this is a decision about tasks, not a gap of the demand-driven compile.
    body = "TaskScheduler.CreateTask(50120, 0, true);",
    dep = [("c.al", RUN_CU)],
);
probe!(
    start_session_for_a_dependency_codeunit_runs_or_refuses,
    RanOrRefused,
    vars = "SessionId: Integer;",
    // The platform refuses to start a session in a test run under test isolation, before it looks at
    // the codeunit (session/TestStartSessionRecord.al:StartSession_FromATestCodeunit_IsRefusedUnderCodeunitIsolation).
    body = "asserterror StartSession(SessionId, 50120); if StrPos(GetLastErrorText, 'Sessions can only be started') = 0 then Error('got [' + GetLastErrorText + ']');",
    dep = [("c.al", RUN_CU)],
);

// ---------------------------------------------------------------------------------------------
// A dependency query the runtime refuses is refused the same way in both compiles

/// A query of a dependency app with a property the runtime refuses (`ReadState`; Base Application's
/// query 777, which corpus query/TestQueryRoleCenterFromPlans.al uses, once had `InherentEntitlements`,
/// which the runtime now accepts as harmless). The
/// eager compile must refuse the test that opens it, as the demand-driven compile does, and must
/// not lower the query without a layout (a runtime panic).
#[test]
fn a_dependency_query_with_an_unsupported_property_is_refused_in_both_compiles() {
    let p = Probe {
        name: "refused_dependency_query",
        want: Want::RanOrRefused,
        vars: "Q: Query \"DepQ\";",
        body: "Q.Open(); if Q.Read() then;",
        extra: "",
        attr: "",
        members: "",
        dep: &[(
            "q.al",
            "table 50500 DepRow { fields { field(1; K; Code[10]) { } } keys { key(PK; K) { Clustered = true; } } }\n\
             query 50501 DepQ { QueryType = Normal; ReadState = ReadCommitted;\n\
               elements { dataitem(Row; DepRow) { column(K; K) { } } } }",
        )],
    };
    let (demand, eager) = (short(&go(&p, Mode::Demand)), short(&go(&p, Mode::Eager)));
    eprintln!("demand: {demand}\neager:  {eager}");
    assert!(demand.contains("ReadState"), "{demand}");
    assert!(!eager.contains("panic"), "{eager}");
    assert_eq!(demand, eager);
}

// ---------------------------------------------------------------------------------------------
// The guard at dispatch time, independent of what the checker reached

#[test]
fn raising_an_event_with_a_declared_but_uncompiled_subscriber_is_refused() {
    let (_, mut l) = al_sema::lower::compile_apps_with(
        vec![AppSource {
            id: "main".into(),
            name: "main".into(),
            dependencies: vec![],
            files: vec![(
                "main.al".into(),
                "table 50100 WsRow { fields { field(1; K; Code[10]) { } } keys { key(PK; K) { Clustered = true; } } }\n\
                 codeunit 50001 T { Subtype = Test; [Test] procedure Go() var R: Record WsRow; begin R.K := 'a'; R.Insert(); end;\n\
                 [Test] procedure Read() var R: Record WsRow; begin if R.FindFirst() then; end; }"
                    .into(),
            )],
        }],
        vec![al_sema::AppKind::Workspace],
        &Default::default(),
        None,
        Default::default(),
    );
    let table = l.program.objects.iter().position(|o| o.name == "WsRow").unwrap();
    std::sync::Arc::make_mut(&mut l.program)
        .uncompiled_subscribers
        .push(al_ir::UncompiledSubscriber {
            publisher: Some(table as u32),
            platform: al_ir::PlatformPublisher::None,
            event: "onafterinsertevent".into(),
            element: String::new(),
            subscriber: "codeunit \"Elsewhere\"".into(),
        });
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let engine = Engine::new(l.program.clone(), blocked);
    let oi = l.program.objects.iter().position(|o| o.name == "T").unwrap();
    let out = run_codeunit(&engine, &l, oi);
    let get = |n: &str| out.iter().find(|(m, _)| m == n).unwrap().1.clone();
    assert!(
        matches!(get("Go"), Outcome::NotRun(why) if why.contains("onafterinsertevent") && why.contains("Elsewhere")),
        "{out:?}"
    );
    // Reading raises no event.
    assert_eq!(get("Read"), Outcome::Pass);
}

// ---------------------------------------------------------------------------------------------
// 12. Zero differences (owner decision 2026-10-14): the static call graph of a page, report or
//     XML port variable does not depend on how much was compiled.

/// An unsupported builtin: a routine that holds it cannot run.
const RARE: &str = "var T: Text; begin T := NoSuchBuiltin(1); end;";

/// A card page over `DT` with the given body.
fn card(id: u32, name: &str, body: &str) -> String {
    format!(
        "page {id} {name} {{ PageType = Card; SourceTable = DT; \
         layout {{ area(Content) {{ field(KCtl; Rec.K) {{ }} }} }} {body} }}"
    )
}

probe!(
    the_run_object_of_a_dependency_page_is_not_followed_by_the_page_variable_that_opens_the_host,
    SamePass,
    vars = "TP: TestPage DH;",
    body = "TP.OpenView(); TP.Close();",
    dep = [
        ("t.al", &table("", "")),
        (
            "h.al",
            &card(
                50140,
                "DH",
                "actions { area(Processing) { action(Next) { RunObject = Page DT2; } } }"
            )
        ),
        ("t2.al", &card(50141, "DT2", &format!("trigger OnOpenPage() {RARE}"))),
    ],
);
probe!(
    the_run_object_codeunit_of_a_workspace_page_runs_on_run_only,
    SamePass,
    vars = "TP: TestPage WsHost;",
    body = "TP.OpenView(); TP.Close();",
    dep = [(
        "c.al",
        &format!("codeunit 50142 DepCu {{ trigger OnRun() begin end; procedure Other() {RARE} }}")
    )],
    extra = "page 50930 WsHost { PageType = Card; layout { area(Content) { } } \
             actions { area(Processing) { action(RunIt) { RunObject = Codeunit DepCu; } } } }",
);
probe!(
    a_page_variable_of_a_routine_the_test_page_reaches_does_not_pull_in_the_pages_code,
    SamePass,
    vars = "TP: TestPage DH;",
    body = "TP.OpenView(); TP.Close();",
    dep = [
        ("t.al", &table("", "")),
        (
            "h.al",
            &card(
                50143,
                "DH",
                "trigger OnOpenPage() var C: Codeunit Shower; begin C.Show(); end;"
            )
        ),
        (
            "c.al",
            "codeunit 50144 Shower { procedure Show() var P: Page DP2; begin end; }"
        ),
        ("t2.al", &card(50145, "DP2", &format!("trigger OnOpenPage() {RARE}"))),
    ],
);
probe!(
    a_report_variable_of_a_dependency_codeunit_reaches_the_report_triggers_only,
    SamePass,
    vars = "C: Codeunit Holder;",
    body = "C.Hold();",
    dep = [
        ("t.al", &table("", "")),
        (
            "c.al",
            "codeunit 50146 Holder { procedure Hold() var R: Report Rp; begin end; }"
        ),
        (
            "r.al",
            &format!(
                "report 50147 Rp {{ ProcessingOnly = true; UseRequestPage = false; \
                 procedure Helper() {RARE} }}"
            )
        ),
    ],
);
probe!(
    a_procedure_called_on_a_page_variable_of_a_dependency_page_is_compiled,
    SamePass,
    vars = "P: Page DP2; R: Record DT;",
    body = "P.SetRecord(R); asserterror P.Proc(); Expect('page:proc');",
    dep = [
        ("t.al", &table("", "")),
        (
            "p.al",
            &card(50150, "DP2", "procedure Proc() begin Error('page:proc'); end;")
        ),
    ],
);

// ---------------------------------------------------------------------------------------------
// 13. Issue 11: a test's outcome does not depend on the other test codeunits of the run.

/// The outcome of the tests of codeunit `T1` when the run selects `selected` (ids of the test
/// codeunits of the workspace; `None`: all), in the demand-driven compile or the complete one.
fn outcomes_with_selection(
    main: &str,
    dep: &[(&str, &str)],
    selected: Option<Vec<u32>>,
    eager: bool,
) -> Vec<(String, Outcome)> {
    let app = |id: &str, deps: &[&str], files: Vec<(String, String)>| AppSource {
        id: id.into(),
        name: id.into(),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files,
    };
    let dep_app = app(
        "dep",
        &[],
        dep.iter().map(|(n, s)| (n.to_string(), s.to_string())).collect(),
    );
    let main_app = app("main", &["dep"], vec![("main.al".into(), main.to_string())]);
    let (_, l) = al_sema::lower::compile_apps_with(
        vec![main_app, dep_app],
        vec![al_sema::AppKind::Workspace, al_sema::AppKind::Dependency],
        &Default::default(),
        None,
        al_sema::check::Roots {
            eager,
            only_tests: selected.map(|ids| ids.into_iter().map(|id| (0usize, id)).collect()),
            ..Default::default()
        },
    );
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let engine = Engine::new(l.program.clone(), blocked);
    let oi = l
        .program
        .objects
        .iter()
        .position(|o| o.name == "T1")
        .expect("test codeunit T1");
    run_codeunit(&engine, &l, oi)
}

#[test]
fn a_tests_outcome_does_not_change_when_another_test_codeunit_activates_extensions() {
    // Dependency: a report with two reportextensions that change a run, a page with a page
    // extension that declares a procedure.
    let dep = [
        ("t.al", table("", "")),
        (
            "r.al",
            "report 50200 Rp { ProcessingOnly = true; UseRequestPage = false; }".to_string(),
        ),
        (
            "e1.al",
            "reportextension 50201 RpE1 extends Rp { trigger OnPreReport() begin end; }".to_string(),
        ),
        (
            "e2.al",
            "reportextension 50202 RpE2 extends Rp { trigger OnPostReport() begin end; }".to_string(),
        ),
        ("p.al", card(50203, "DP", "")),
        (
            "pe.al",
            "pageextension 50204 DPE extends DP { layout { } procedure Extra() begin end; }".to_string(),
        ),
    ];
    let dep: Vec<(&str, &str)> = dep.iter().map(|(n, s)| (*n, s.as_str())).collect();
    // T1 holds the report variable (the reportextension rule decides it); T2 drives the page
    // with its extension; T3 runs the report.
    let main = "codeunit 50001 T1 { Subtype = Test; \
                  [Test] procedure RunsTheReport() var R: Report Rp; begin R.Run(); end; \
                  [Test] procedure OpensThePage() var TP: TestPage DP; begin TP.OpenView(); TP.Close(); end; }\n\
                codeunit 50002 T2 { Subtype = Test; \
                  [Test] procedure Other() var TP: TestPage DP; R: Report Rp; begin TP.OpenView(); R.Run(); end; }";
    for eager in [false, true] {
        let alone = outcomes_with_selection(main, &dep, Some(vec![50001]), eager);
        let together = outcomes_with_selection(main, &dep, Some(vec![50001, 50002]), eager);
        let everything = outcomes_with_selection(main, &dep, None, eager);
        assert_eq!(alone, together, "eager={eager}: T1 differs alone and with T2");
        assert_eq!(
            alone, everything,
            "eager={eager}: T1 differs alone and in the whole run"
        );
    }
    let demand = outcomes_with_selection(main, &dep, Some(vec![50001]), false);
    let eager = outcomes_with_selection(main, &dep, Some(vec![50001]), true);
    assert_eq!(demand, eager, "demand and eager differ");
}
probe!(
    an_interface_that_only_a_reached_routine_returns_is_compiled,
    SamePass,
    vars = "C: Codeunit Maker; I: Interface IThing;",
    body = "I := C.Make(); I.Hello();",
    dep = [
        ("i.al", "interface IThing { procedure Hello(); }"),
        (
            "impl.al",
            "codeunit 50210 Impl implements IThing { procedure Hello() begin end; }"
        ),
        (
            "m.al",
            "codeunit 50211 Maker { procedure Make(): Interface IThing var I: Codeunit Impl; begin exit(I); end; }"
        ),
    ],
);
