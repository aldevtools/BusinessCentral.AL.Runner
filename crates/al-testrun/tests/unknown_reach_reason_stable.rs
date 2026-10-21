//! The reason "part of its reach is not known (X)" names the same X on every run. The reach walk visits the
//! subscribers of an event in a fixed order, so the first unknown part it meets does not depend on the order of a
//! hash map.

use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun};
use al_testrun::{CaseId, Outcome};
use std::collections::BTreeSet;

fn source() -> String {
    let events = [
        "OnBeforeInsertEvent",
        "OnAfterInsertEvent",
        "OnBeforeModifyEvent",
        "OnAfterModifyEvent",
        "OnBeforeDeleteEvent",
        "OnAfterDeleteEvent",
        "OnBeforeRenameEvent",
        "OnAfterRenameEvent",
    ];
    let mut subs = String::new();
    for (i, e) in events.iter().enumerate() {
        subs.push_str(&format!(
            "    [EventSubscriber(ObjectType::Table, Database::\"Watched\", '{e}', '', false, false)]\n    local procedure S{i}()\n    begin\n        Codeunit.Run({});\n    end;\n",
            50990 + i
        ));
    }
    format!(
        r#"
table 50980 "Watched" {{ fields {{ field(1; "No."; Integer) {{ }} }} keys {{ key(PK; "No.") {{ }} }} }}
codeunit 50981 "Subs"
{{
{subs}}}
codeunit 50982 "Unknown Reach"
{{
    Subtype = Test;
    [Test]
    procedure A_RefusedMidRunInserts()
    var
        W: Record "Watched";
    begin
        Sleep(-1);
        W.Insert(true);
    end;
    [Test]
    procedure B_Fails()
    begin
        Error('boom');
    end;
}}
"#
    )
}

fn reason() -> String {
    let rt = CorpusRuntime::run_suites_with(
        vec![AppSource {
            id: "main".into(),
            name: "main".into(),
            dependencies: vec![],
            files: vec![("main.al".into(), source())],
        }],
        vec![al_sema::AppKind::Workspace],
        &[],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions::default(),
    );
    match rt.first_outcome(&CaseId {
        codeunit: 50982,
        name: "B_Fails".into(),
    }) {
        Outcome::NotRun(m) => {
            assert!(m.contains("not known"), "{m}");
            m
        }
        o => panic!("{o:?}"),
    }
}

#[test]
fn the_unknown_part_named_is_the_same_on_every_run() {
    let reasons: BTreeSet<String> = (0..8).map(|_| reason()).collect();
    assert_eq!(reasons.len(), 1, "{reasons:#?}");
}
