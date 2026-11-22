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
}
