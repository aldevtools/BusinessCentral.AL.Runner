//! An event subscriber names its publisher by a name two namespaces declare (al-runner issue 8171). The subscriber's file decides
//! which object it names, as alc resolves it: the file's own namespace, then its `using`s; a qualified name names that namespace
//! only. The reporter's repro: alc 18.1 compiles `Codeunit::"Shared Name"` through `using Repro.Copy;` to codeunit 51655. Probe:
//! events/subscriber-namespace/ (SnUsingTests.al:28-55, SnOwnNamespaceTests.al:25-53, SnOwnOverUsingTests.al:26-54). The pinned
//! shapes run unmarked.

use al_builtins::assumptions;
use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_sema::workspace::AppSource;

fn app(id: &str, deps: &[&str], src: &str) -> AppSource {
    AppSource {
        id: id.into(),
        name: format!("App {id}"),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files: vec![(format!("{id}.al"), src.into())],
    }
}

fn shared(namespace: &str, id: u32) -> String {
    format!(
        "namespace {namespace};\n\
         codeunit {id} \"Shared Name\"\n\
         {{\n\
             procedure Ping() begin OnPing(); end;\n\
             [IntegrationEvent(false, false)]\n\
             local procedure OnPing() begin end;\n\
         }}\n"
    )
}

/// The reporter's three apps: "Ns Base" (51650 in Repro.Base), "Ns Copy" (51655 in Repro.Copy and 51657 "Copy Only Name"), and
/// the subscriber app with `header` (namespace and `using`s) and the publisher name `publisher`.
fn repro(header: &str, publisher: &str) -> Vec<AppSource> {
    let copy = format!(
        "{}\ncodeunit 51657 \"Copy Only Name\"\n{{\n    procedure Ping() begin OnPing(); end;\n    \
         [IntegrationEvent(false, false)]\n    local procedure OnPing() begin end;\n}}\n",
        shared("Repro.Copy", 51655)
    );
    let tests = format!(
        "{header}\n\
         codeunit 51660 \"Ns Tests\"\n\
         {{\n\
             EventSubscriberInstance = Manual;\n\
             var Ran: Boolean; RanOnly: Boolean;\n\
             procedure PingCopy(): Boolean\n\
             var Shared: Codeunit Repro.Copy.\"Shared Name\";\n\
             begin BindSubscription(this); Shared.Ping(); UnbindSubscription(this); exit(Ran); end;\n\
             procedure PingBase(): Boolean\n\
             var Shared: Codeunit Repro.Base.\"Shared Name\";\n\
             begin BindSubscription(this); Shared.Ping(); UnbindSubscription(this); exit(Ran); end;\n\
             procedure PingOnly(): Boolean\n\
             var Only: Codeunit Repro.Copy.\"Copy Only Name\";\n\
             begin BindSubscription(this); Only.Ping(); UnbindSubscription(this); exit(RanOnly); end;\n\
             [EventSubscriber(ObjectType::Codeunit, Codeunit::{publisher}, 'OnPing', '', false, false)]\n\
             local procedure HandlePing() begin Ran := true; end;\n\
             [EventSubscriber(ObjectType::Codeunit, Codeunit::Repro.Copy.\"Copy Only Name\", 'OnPing', '', false, false)]\n\
             local procedure HandleOnly() begin RanOnly := true; end;\n\
         }}\n"
    );
    vec![
        app("base", &[], &shared("Repro.Base", 51650)),
        app("copy", &["base"], &copy),
        app("tests", &["base", "copy"], &tests),
    ]
}

/// Runs `routine` of "Ns Tests" and returns its answer with the assumptions it marked.
fn run(apps: Vec<AppSource>, routine: &str) -> (Result<Value, Signal>, Vec<assumptions::Assumption>) {
    let mut b = al_testkit::Built::new(al_testkit::compile_apps(apps));
    assumptions::clear_hits();
    let r = b.run_in("Ns Tests", routine);
    (r, assumptions::take_hits())
}

const USING_COPY: &str = "namespace Repro.Tests;\nusing Repro.Copy;";

#[test]
fn a_plain_name_binds_to_the_twin_the_files_using_names() {
    let (r, hits) = run(repro(USING_COPY, "\"Shared Name\""), "PingCopy");
    assert!(al_testkit::boolean(r), "the subscriber runs on the event of 51655");
    assert!(hits.is_empty());
}

#[test]
fn the_other_twins_event_does_not_reach_the_subscriber() {
    let (r, hits) = run(repro(USING_COPY, "\"Shared Name\""), "PingBase");
    assert!(
        !al_testkit::boolean(r),
        "51650's event does not reach a subscriber of 51655"
    );
    assert!(hits.is_empty());
}

#[test]
fn a_plain_name_binds_to_the_twin_of_the_files_own_namespace() {
    let own = "namespace Repro.Base;";
    assert!(al_testkit::boolean(run(repro(own, "\"Shared Name\""), "PingBase").0));
    assert!(!al_testkit::boolean(run(repro(own, "\"Shared Name\""), "PingCopy").0));
}

#[test]
fn the_own_namespace_comes_before_a_using_of_the_other_twin() {
    let own = "namespace Repro.Base;\nusing Repro.Copy;";
    assert!(al_testkit::boolean(run(repro(own, "\"Shared Name\""), "PingBase").0));
    assert!(!al_testkit::boolean(run(repro(own, "\"Shared Name\""), "PingCopy").0));
}

#[test]
fn a_qualified_name_binds_to_the_twin_it_names() {
    let qualified = "Repro.Copy.\"Shared Name\"";
    let (r, hits) = run(repro(USING_COPY, qualified), "PingCopy");
    assert!(al_testkit::boolean(r));
    assert!(hits.is_empty());
    assert!(!al_testkit::boolean(run(repro(USING_COPY, qualified), "PingBase").0));
    let other = "Repro.Base.\"Shared Name\"";
    assert!(al_testkit::boolean(run(repro(USING_COPY, other), "PingBase").0));
    assert!(!al_testkit::boolean(run(repro(USING_COPY, other), "PingCopy").0));
}

/// A name that one object of the program has keeps main's binding, unmarked.
#[test]
fn a_name_one_object_has_binds_as_before_without_a_mark() {
    let (r, hits) = run(repro(USING_COPY, "\"Shared Name\""), "PingOnly");
    assert!(al_testkit::boolean(r));
    assert!(hits.is_empty());
}

fn refused(r: Result<Value, Signal>) -> String {
    match r {
        Err(Signal::Unsupported(why)) => why.to_string(),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

/// Files whose plain name the AL compilers of BC 28 and 29 reject (corpus PR 1423, must-not-compile 3171 to 3173: AL0275 and
/// AL0118), and a file without a namespace whose `using` alc ignores (AL0789): the subscriber is bound to no twin, and raising
/// the event of either twin refuses.
#[test]
fn a_plain_name_the_file_does_not_resolve_to_one_twin_refuses_the_event_of_each_twin() {
    for header in [
        "",
        "namespace Repro.Tests;\nusing Repro.Base;\nusing Repro.Copy;",
        "namespace Repro.Tests;",
        "using Repro.Copy;",
    ] {
        for routine in ["PingCopy", "PingBase"] {
            let why = refused(run(repro(header, "\"Shared Name\""), routine).0);
            assert!(why.contains("Shared Name"), "{header:?} {routine}: {why}");
            assert!(!why.contains("was not compiled"), "{header:?} {routine}: {why}");
        }
        // The other publisher of the codeunit is not touched.
        assert!(al_testkit::boolean(run(repro(header, "\"Shared Name\""), "PingOnly").0));
    }
}

/// A subscriber binds only to an object its own app sees: a dependency app's subscriber to "Twin Pub" is not bound to the codeunit
/// of that name a dependent app declares in its own namespace (Microsoft's "Config. Validate Management" subscribes to table 8613
/// "Config. Package Table"; the corpus app declares a table of that name too, _fixtures/ALTRelationNameCollisionTables.al:28).
#[test]
fn a_subscriber_binds_only_to_an_object_its_app_sees() {
    let dep = "codeunit 51670 \"Twin Pub\"\n{\n    procedure Ping() begin OnPing(); end;\n    \
               [IntegrationEvent(false, false)]\n    local procedure OnPing() begin end;\n}\n\
               codeunit 51671 Seen\n{\n    SingleInstance = true;\n    var Ran: Boolean;\n    \
               procedure Mark() begin Ran := true; end;\n    procedure WasRun(): Boolean begin exit(Ran); end;\n}\n\
               codeunit 51673 \"Dep Subs\"\n{\n    \
               [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Twin Pub\", 'OnPing', '', false, false)]\n    \
               local procedure H() var S: Codeunit Seen; begin S.Mark(); end;\n}\n";
    let tests = "codeunit 51660 \"Ns Tests\"\n{\n    procedure PingMain(): Boolean\n    var P: Codeunit M.\"Twin Pub\"; S: Codeunit Seen;\n    \
         begin P.Ping(); exit(S.WasRun()); end;\n    procedure PingDep(): Boolean\n    var P: Codeunit \"Twin Pub\"; S: Codeunit Seen;\n    \
         begin P.Ping(); exit(S.WasRun()); end;\n}\n";
    let twin = shared("M", 51672).replace("Shared Name", "Twin Pub");
    let apps = || {
        let mut main = app("main", &["dep"], &twin);
        main.files.push(("tests.al".into(), tests.into()));
        vec![main, app("dep", &[], dep)]
    };
    assert!(!al_testkit::boolean(run(apps(), "PingMain").0));
    assert!(al_testkit::boolean(run(apps(), "PingDep").0));
}

/// One codeunit and one table, no namespace, and subscribers that name each by its bare id
/// (`[EventSubscriber(ObjectType::Codeunit, 51650, ...)]`, the form Microsoft's ERM Change Global Dimensions writes as
/// `ObjectType::Table, 483`): found by id, as main found it, unmarked.
fn single_app_by_id(publisher_arg: &str) -> Vec<AppSource> {
    let src = format!(
        "codeunit 51650 \"Only Pub\"\n{{\n    procedure Ping() begin OnPing(); end;\n    \
         [IntegrationEvent(false, false)]\n    local procedure OnPing() begin end;\n}}\n\
         table 51651 \"Only Tab\" {{ fields {{ field(1; K; Integer) {{ }} }} keys {{ key(PK; K) {{ }} }} }}\n\
         codeunit 51660 \"Ns Tests\"\n{{\n    EventSubscriberInstance = Manual;\n    var Ran: Boolean; Inserted: Boolean;\n    \
         procedure Ping(): Boolean var P: Codeunit \"Only Pub\"; begin BindSubscription(this); P.Ping(); UnbindSubscription(this); exit(Ran); end;\n    \
         procedure Insert(): Boolean var T: Record \"Only Tab\"; begin BindSubscription(this); T.K := 1; T.Insert(); UnbindSubscription(this); exit(Inserted); end;\n    \
         [EventSubscriber(ObjectType::Codeunit, {publisher_arg}, 'OnPing', '', false, false)]\n    \
         local procedure HandlePing() begin Ran := true; end;\n    \
         [EventSubscriber(ObjectType::Table, 51651, 'OnAfterInsertEvent', '', false, false)]\n    \
         local procedure HandleInsert(var Rec: Record \"Only Tab\") begin Inserted := true; end;\n}}\n"
    );
    vec![app("tests", &[], &src)]
}

#[test]
fn a_bare_id_names_the_object_of_that_id() {
    // The bare-id form runs through its own assumption, `subscriber-publisher-by-id`, and through no other.
    let by_id = vec![assumptions::Assumption::SubscriberPublisherById];
    let (r, hits) = run(single_app_by_id("51650"), "Ping");
    assert!(al_testkit::boolean(r), "the codeunit subscriber by id runs");
    assert_eq!(hits, by_id);
    let (r, hits) = run(single_app_by_id("51650"), "Insert");
    assert!(al_testkit::boolean(r), "the table subscriber by id runs");
    assert_eq!(hits, by_id);
}

/// A second argument that is neither `<Kind>::<name>` nor a bare id is not read: raising the event refuses instead of skipping
/// the subscriber.
#[test]
fn a_publisher_argument_of_another_form_refuses_the_event() {
    let why = refused(run(single_app_by_id("'Only Pub'"), "Ping").0);
    assert!(why.contains("'Only Pub'"), "{why}");
    // The table subscriber of the same codeunit is read and runs.
    assert!(al_testkit::boolean(run(single_app_by_id("'Only Pub'"), "Insert").0));
}

/// A twin without a namespace against a namespaced one is not measured: the subscriber is bound to neither and each event refuses
/// (also through a `using` of the namespaced twin).
#[test]
fn a_twin_without_a_namespace_is_not_picked() {
    let mut apps = repro(USING_COPY, "\"Shared Name\"");
    apps[0].files[0].1 = shared("Repro.Base", 51650).replace("namespace Repro.Base;\n", "");
    apps[2].files[0].1 = apps[2].files[0]
        .1
        .replace("Codeunit Repro.Base.\"Shared Name\"", "Codeunit 51650");
    let why = refused(run(apps, "PingCopy").0);
    assert!(why.contains("Shared Name"), "{why}");
}

/// Table and page twins are not measured: a subscriber to a table or page name two namespaces have is bound to neither, also
/// through a `using` of one of them, and the event of each twin refuses.
#[test]
fn table_and_page_twins_are_not_picked() {
    let object = |kind: &str, ns: &str, id: u32| match kind {
        "table" => format!(
            "namespace {ns};\ntable {id} \"Shared Name\" {{ fields {{ field(1; K; Integer) {{ }} }} keys {{ key(PK; K) {{ }} }} }}\n"
        ),
        _ => format!("namespace {ns};\npage {id} \"Shared Name\" {{ layout {{ area(Content) {{ }} }} }}\n"),
    };
    let tests = |kind: &str| {
        let (sub, raise, var) = match kind {
            "table" => (
                "ObjectType::Table, Database::\"Shared Name\", 'OnAfterInsertEvent'",
                "T.K := 1; T.Insert();",
                "T: Record Repro.Copy.\"Shared Name\"",
            ),
            _ => (
                "ObjectType::Page, Page::\"Shared Name\", 'OnOpenPageEvent'",
                "T.OpenView(); T.Close();",
                "T: TestPage Repro.Copy.\"Shared Name\"",
            ),
        };
        format!(
            "{USING_COPY}\ncodeunit 51660 \"Ns Tests\"\n{{\n    EventSubscriberInstance = Manual;\n    var Ran: Boolean;\n    \
             procedure Raise(): Boolean var {var}; begin BindSubscription(this); {raise} UnbindSubscription(this); exit(Ran); end;\n    \
             [EventSubscriber({sub}, '', false, false)]\n    local procedure H() begin Ran := true; end;\n}}\n"
        )
    };
    for kind in ["table", "page"] {
        let apps = vec![
            app("base", &[], &object(kind, "Repro.Base", 51650)),
            app("copy", &["base"], &object(kind, "Repro.Copy", 51655)),
            app("tests", &["base", "copy"], &tests(kind)),
        ];
        let why = refused(run(apps, "Raise").0);
        assert!(why.contains("Shared Name"), "{kind}: {why}");
    }
}
