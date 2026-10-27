//! `Engine::sibling`: a run's second engine shares what the first worked out from the program (ADR-054 follow-up: the
//! install baseline's engine and the tests' engine were each built from the attribute text of every routine).

use std::sync::Arc;

const SRC: &str = r#"
table 50100 T { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
codeunit 50101 Pub { [IntegrationEvent(false, false)] procedure OnThing() begin end; }
codeunit 50102 Sub { [EventSubscriber(ObjectType::Codeunit, Codeunit::Pub, 'OnThing', '', false, false)] local procedure Handle() begin end; }
"#;

#[test]
fn a_sibling_shares_the_program_tables_and_has_its_own_run_state() {
    let l = al_testkit::compile(SRC);
    let engine = al_vm::Engine::new(l.program.clone(), al_testkit::blocked(&l));
    let sibling = engine.sibling();
    assert!(
        Arc::ptr_eq(&engine.events, &sibling.events),
        "the event registry is shared"
    );
    assert!(
        Arc::ptr_eq(&engine.tables, &sibling.tables),
        "the table definitions are shared"
    );
    assert!(Arc::ptr_eq(&engine.unevaluable_init, &sibling.unevaluable_init));
    assert!(Arc::ptr_eq(&engine.program, &sibling.program));
    assert_eq!(engine.blocked, sibling.blocked);
    assert_ne!(
        engine.uid, sibling.uid,
        "state shared between sessions is keyed by the engine"
    );
    assert!(sibling.test_data.is_none() && sibling.unseeded.is_empty() && sibling.company_refusal.is_none());
    // What a new engine works out is what the sibling holds.
    let fresh = al_vm::Engine::new(l.program.clone(), al_testkit::blocked(&l));
    assert_eq!(format!("{:?}", fresh.events), format!("{:?}", sibling.events));
}
