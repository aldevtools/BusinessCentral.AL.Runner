//! The checks the field table makes (al-runner issue 8211): each `rest` field's predicate reports that field by its name.

use crate::{Engine, Session};

fn session() -> Session {
    let l = al_testkit::compile("codeunit 50100 Idle { procedure Nothing() begin end; }");
    Session::new(Engine::new(l.program.clone(), al_testkit::blocked(&l)))
}

#[test]
fn a_new_session_is_at_rest_and_holds_what_a_new_session_holds() {
    let s = session();
    assert_eq!(s.at_rest_why(), None);
    assert_eq!(s.install_refused_field(), None);
}

// An interface call that runs: `run_iface` notes the frame it entered (exec/run.rs), and `install_left` must not carry
// a session in which one runs.
#[test]
fn a_running_interface_call_is_not_at_rest() {
    let mut s = session();
    s.iface_frames.push(0);
    assert_eq!(s.at_rest_why(), Some("iface_frames"));
}

#[test]
fn each_rest_field_is_named_by_its_own_predicate() {
    let mut s = session();
    s.depth = 1;
    assert_eq!(s.at_rest_why(), Some("depth"));
    s.depth = 0;
    s.next_interface_call = true;
    assert_eq!(s.at_rest_why(), Some("next_interface_call"));
    s.next_interface_call = false;
    s.keyref_receiver_local = true;
    assert_eq!(s.at_rest_why(), Some("keyref_receiver_local"));
}

#[test]
fn a_refused_field_is_named_and_an_image_only_one_is_not_held_of_install_code() {
    let mut s = session();
    s.sessions_started = 1;
    assert_eq!(s.install_refused_field(), Some("sessions_started"));
    s.sessions_started = 0;
    s.fault_trace = true;
    assert_eq!(s.install_refused_field(), None);
    assert_eq!(s.image_refused_field(), Some("fault_trace"));
}

// A group 1 entry a record took back (filter group 1, the global group) leaves no filter, so install code is held to
// main's rule and passes; the image carries none of the group 1 state, so it refuses the session: the entry still
// refuses the records that hold a filter in group 1 (`group1_sessions.rs`).
#[test]
fn a_taken_back_group_1_entry_refuses_the_image_and_not_install_code() {
    let l = al_testkit::compile(
        "table 50100 G1Row { fields { field(1; Id; Integer) { } field(2; Qty; Integer) { } } keys { key(PK; Id) { } } }
        codeunit 50100 G1 {
            procedure SetAndTakeBack() var R: Record G1Row; R2: Record G1Row;
            begin R.FilterGroup(1); R.SetRange(Qty, 5); R2.FilterGroup(1); R2.SetRange(Qty); end; }",
    );
    let mut s = Session::new(Engine::new(l.program.clone(), al_testkit::blocked(&l)));
    let object = s.engine.program.objects.iter().position(|o| o.name == "G1").unwrap();
    let inst = s.codeunit_instance(object);
    s.call(object, 0, inst, vec![]).expect("the routine runs");
    assert!(s.global_group1.is_empty(), "no table holds a group 1 filter");
    assert_eq!(s.install_refused_field(), None);
    assert_eq!(s.image_refused_field(), Some("global_group1"));
}
