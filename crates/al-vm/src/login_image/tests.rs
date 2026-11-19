//! The session image: every test freezes a session, thaws the image into a new session of the same engine and compares
//! what the login replay check compares (`Session::state_lines`), plus direct asserts where the render cannot see
//! (design `/coord/handoff/PERF-01/step2-design.md` section 4.2).
//!
//! Conditions of the maintainer (al-runner issue 8211), kept in al-testrun `login_replay.rs`, not here: the key holds
//! content or SHA-256, never a 64-bit digest; the test codeunit stays in the key. The image holds `Session::test_object`
//! by value, so an image is only ever thawed into a session of the same test codeunit.

use super::*;
use crate::{Engine, Instance, Session};
use al_builtins::library_kind::LibraryKind;
use al_builtins::value::{LibraryValue, Value};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

const SRC: &str = r#"
table 50100 Row
{
    fields { field(1; Id; Integer) { } field(2; Name; Text[50]) { } field(3; Data; Blob) { } }
    keys { key(PK; Id) { } }
    var Gv: Integer;
}
codeunit 50101 Peer
{
    trigger OnRun() begin end;
    var Back: Codeunit Peer; N: Integer;
}
codeunit 50102 Sub
{
    EventSubscriberInstance = Manual;
    [EventSubscriber(ObjectType::Table, Database::Row, 'OnAfterInsertEvent', '', false, false)]
    local procedure H(var Rec: Record Row) begin end;
}
codeunit 50100 Keeper
{
    SingleInstance = true;
    var
        A: Codeunit Peer; B: Codeunit Peer; Sub1: Codeunit Sub;
        Rec: Record Row; T1: Record Row temporary; T2: Record Row temporary;
        Items: List of [Integer]; Dict: Dictionary of [Integer, Text]; Refs: array[3] of RecordRef;
        V1: Variant; V2: Variant; J: JsonObject; Strm: InStream;

    procedure FillTemps()
    begin
        T1.Init(); T1.Id := 1; T1.Name := 'é'; T1.Insert();
        T1.Id := 2; T1.Insert();
        T1.Id := 3; T1.Insert();
        T2.Copy(T1, true);
    end;

    procedure FilterTemps()
    begin
        T1.SetRange(Id, 1, 3);
        T1.FilterGroup(2); T1.SetFilter(Name, '<>zz'); T1.FilterGroup(0);
        T1.SetCurrentKey(Id); T1.Ascending(false);
        T1.Get(2); T1.Mark(true);
    end;

    procedure InsertViaT2() begin T2.Id := 4; T2.Insert(); end;
    procedure CountT1(): Text begin exit(Format(T1.Count())); end;
    procedure DescribeT1(): Text
    begin
        exit(Format(T1.Count()) + '|' + T1.GetFilters() + '|' + Format(T1.Ascending()) + '|' + Format(T1.MarkedOnly()) + '|' + Format(T1.Id));
    end;
    procedure FillDatabase() begin Rec.Id := 1; Rec.Name := 'é'; Rec.Insert(); Commit(); end;
    procedure Uncommitted() begin Rec.Id := 9; Rec.Insert(); end;
    procedure FillCollections() begin Items.Add(10); Items.Add(20); Dict.Add(1, 'a'); Dict.Add(2, 'é'); end;
    procedure ReadCollections(): Text
    begin
        exit(Format(Items.Count()) + Format(Items.Get(2)) + Format(Dict.Count()) + Dict.Get(2));
    end;
    procedure OpenRefs() begin Refs[1].GetTable(Rec); Refs[3].Open(50100, true); end;
    procedure Bind() begin BindSubscription(Sub1); end;
    procedure Fail() begin asserterror Error('boom'); end;
    procedure Draws(): Text
    begin
        exit(Format(Random(1000000)) + ',' + Format(Random(1000000)) + ',' + Format(Random(1000000)) + ',' + Format(CreateGuid()));
    end;
    procedure Reseed() begin Randomize(7); end;
    procedure MakeSequence() begin NumberSequence.Insert('S50', 5, 3); Commit(); end;
    procedure Schedule() begin TaskScheduler.CreateTask(50101, 0, true); end;
    procedure FillJson() begin J.Add('a', 1); end;
    procedure ReadAccent(): Text begin Rec.SetFilter(Name, '@e'); exit(Format(Rec.Count()) + '|' + Format(T1.Count())); end;
}
"#;

/// A session on `SRC` and the indexes of its objects.
struct Fx {
    session: Session,
    keeper: usize,
    peer: usize,
    row: usize,
}

fn fx() -> Fx {
    let l = al_testkit::compile(SRC);
    let engine = Engine::new(l.program.clone(), al_testkit::blocked(&l));
    let at = |name: &str| engine.program.objects.iter().position(|o| o.name == name).expect(name);
    let (keeper, peer, row) = (at("Keeper"), at("Peer"), at("Row"));
    Fx {
        session: Session::new(engine),
        keeper,
        peer,
        row,
    }
}

impl Fx {
    /// Runs `routine` of Keeper on the session's single instance.
    fn run(&mut self, routine: &str) -> String {
        answer(&mut self.session, self.keeper, routine)
    }

    fn keeper_instance(&mut self) -> Rc<RefCell<Instance>> {
        self.session.codeunit_instance(self.keeper)
    }

    fn set(&mut self, global: &str, v: Value) {
        let at = slot(&self.session, self.keeper, global);
        self.keeper_instance().borrow_mut().globals[at] = v;
    }

    fn get(&mut self, global: &str) -> Value {
        get(&mut self.session, self.keeper, global)
    }

    /// A new Peer instance, held by nothing else.
    fn peer(&mut self) -> Rc<RefCell<Instance>> {
        self.session.instance(self.peer)
    }
}

fn slot(s: &Session, object: usize, name: &str) -> usize {
    s.engine.program.objects[object]
        .globals
        .iter()
        .position(|g| g.name == name)
        .unwrap_or_else(|| panic!("no global {name}"))
}

fn get(s: &mut Session, object: usize, name: &str) -> Value {
    let at = slot(s, object, name);
    s.codeunit_instance(object).borrow().globals[at].clone()
}

/// What `routine` answers, or what it refuses with (a read the notes refuse is an outcome to compare).
fn outcome(s: &mut Session, object: usize, routine: &str) -> String {
    match try_call(s, object, routine) {
        Ok(v) => format!("{v:?}"),
        Err(e) => format!("{e:?}"),
    }
}

/// What `routine` answers; a set-up routine that fails is a broken test.
fn answer(s: &mut Session, object: usize, routine: &str) -> String {
    match try_call(s, object, routine) {
        Ok(v) => format!("{v:?}"),
        Err(e) => panic!("{routine} failed: {e:?}"),
    }
}

fn try_call(s: &mut Session, object: usize, routine: &str) -> Result<Value, al_builtins::native::Signal> {
    let ri = s.engine.program.objects[object]
        .routines
        .iter()
        .position(|r| r.name == routine)
        .unwrap_or_else(|| panic!("no routine {routine}"));
    let inst = s.codeunit_instance(object);
    s.call(object, ri, inst, vec![]).map(|(v, _)| v)
}

/// A new session of the engine: what a thaw is made into (design 0.2).
fn new_session(s: &Session) -> Session {
    Session::new(s.engine.clone())
}

fn image(s: &Session) -> LoginImage {
    s.login_image(0)
        .unwrap_or_else(|why| panic!("no image: {}", why.reason()))
}

fn thaw(s: &Session, image: &LoginImage) -> Session {
    let mut twin = new_session(s);
    twin.thaw_login_image(image);
    twin
}

/// Freezes `s`, thaws into a twin and asserts the twin's state lines equal `s`'s.
fn round_trip(s: &mut Session) -> Session {
    let image = image(s);
    let mut twin = thaw(s, &image);
    assert_eq!(s.state_lines(), twin.state_lines());
    twin
}

#[test]
fn a_new_session_round_trips_with_no_difference() {
    let mut f = fx();
    f.keeper_instance();
    round_trip(&mut f.session);
}

#[test]
fn every_plain_field_the_login_may_change_comes_back() {
    let mut f = fx();
    f.run("Fail");
    f.run("MakeSequence");
    let s = &mut f.session;
    s.global_language = 1031;
    s.work_date = Some(738_000);
    s.isolated_storage
        .insert(("a".into(), "k".into()), ("v".into(), true, false));
    s.isolated_committed
        .insert(("a".into(), "k".into()), ("v".into(), true, false));
    s.autoinc.insert(("t".into(), 3), 17);
    s.caught_errors = 3;
    s.next_guid = 5;
    s.commits += 7;
    s.application_areas = "#All".into();
    s.next_row_version += 3;
    s.calls_made = 9;
    s.waited_ms = 1234;
    s.encrypted.insert("plain".into(), "cipher".into());
    s.tdd_reached.insert("stub".into());
    s.demo_written.insert(18);
    let mut twin = round_trip(s);
    assert_eq!(twin.global_language, 1031);
    assert_eq!(twin.work_date, Some(738_000));
    assert_eq!(twin.isolated_storage, s.isolated_storage);
    assert_eq!(twin.isolated_committed, s.isolated_committed);
    assert_eq!(twin.autoinc, s.autoinc);
    assert_eq!(twin.caught_errors, 3);
    assert_eq!(twin.next_guid, 5);
    assert_eq!(twin.commits, s.commits);
    assert_eq!(twin.application_areas, "#All");
    assert_eq!(twin.next_row_version, s.next_row_version);
    assert_eq!(twin.calls_made, 9);
    assert_eq!(twin.waited_ms, 1234);
    assert_eq!(twin.encrypted, s.encrypted);
    assert_eq!(twin.tdd_reached, s.tdd_reached);
    assert_eq!(twin.demo_written, s.demo_written);
    assert_eq!(twin.sequences.len(), s.sequences.len());
    assert!(!s.sequences.is_empty());
    assert_eq!(format!("{:?}", twin.last_error), format!("{:?}", s.last_error));
    assert!(twin.last_error.message.contains("boom"));
    let _ = twin.state_lines();
}

#[test]
fn the_clock_and_the_random_sequence_go_on_as_in_the_original() {
    let mut f = fx();
    f.run("Reseed");
    f.run("Draws");
    let mut twin = round_trip(&mut f.session);
    let keeper = f.keeper;
    assert_eq!(
        (twin.clock_draws, twin.last_stamp, twin.clock_key),
        (f.session.clock_draws, f.session.last_stamp, f.session.clock_key)
    );
    let ours = outcome(&mut f.session, keeper, "Draws");
    let theirs = outcome(&mut twin, keeper, "Draws");
    assert_eq!(ours, theirs, "the next three Random draws and the next Guid");
    // `Randomize(n)` without a draw is kept too.
    let mut g = fx();
    g.run("Reseed");
    let mut twin = round_trip(&mut g.session);
    let ours = outcome(&mut g.session, g.keeper, "Draws");
    assert_eq!(ours, outcome(&mut twin, g.keeper, "Draws"));
}

#[test]
fn two_globals_holding_one_instance_thaw_as_one_instance() {
    let mut f = fx();
    let p = f.peer();
    f.set("A", Value::Codeunit(p.clone()));
    f.set("B", Value::Codeunit(p));
    let mut twin = round_trip(&mut f.session);
    let (Value::Codeunit(a), Value::Codeunit(b)) = (get(&mut twin, f.keeper, "A"), get(&mut twin, f.keeper, "B"))
    else {
        panic!("globals hold instances")
    };
    assert!(Rc::ptr_eq(&a, &b));
    let n = slot(&twin, f.peer, "N");
    a.borrow_mut().globals[n] = Value::Integer(9);
    assert!(matches!(b.borrow().globals[n], Value::Integer(9)));
}

#[test]
fn two_instances_that_hold_each_other_thaw_as_the_same_cycle() {
    let mut f = fx();
    let (p, q) = (f.peer(), f.peer());
    let back = slot(&f.session, f.peer, "Back");
    p.borrow_mut().globals[back] = Value::Codeunit(q.clone());
    q.borrow_mut().globals[back] = Value::Codeunit(p.clone());
    f.set("A", Value::Codeunit(p));
    drop(q);
    let mut twin = round_trip(&mut f.session);
    let Value::Codeunit(a) = get(&mut twin, f.keeper, "A") else {
        panic!("A holds an instance")
    };
    let Value::Codeunit(q) = a.borrow().globals[back].clone() else {
        panic!("Back holds an instance")
    };
    let Value::Codeunit(again) = q.borrow().globals[back].clone() else {
        panic!("Back holds an instance")
    };
    assert!(Rc::ptr_eq(&a, &again), "the cycle closes on the same instance");
}

fn record_of(v: Value) -> Rc<RefCell<al_builtins::value::RecordState>> {
    match v {
        Value::Record(r) => r,
        other => panic!("not a record: {other:?}"),
    }
}

#[test]
fn records_sharing_one_temporary_table_thaw_sharing_one_table() {
    let mut f = fx();
    f.run("FillTemps");
    let mut twin = round_trip(&mut f.session);
    assert_eq!(f.run("CountT1"), outcome(&mut twin, f.keeper, "CountT1"));
    answer(&mut twin, f.keeper, "InsertViaT2");
    let theirs = outcome(&mut twin, f.keeper, "CountT1");
    assert_ne!(
        theirs,
        f.run("CountT1"),
        "a row inserted through T2 of the twin is found through T1 of the twin"
    );
    assert_eq!(
        f.run("CountT1"),
        outcome(&mut new_twin_of_original(&mut f), f.keeper, "CountT1")
    );
}

/// A second thaw of a fresh image of `f`'s session (the original is not changed by what the twin did).
fn new_twin_of_original(f: &mut Fx) -> Session {
    let image = image(&f.session);
    thaw(&f.session, &image)
}

#[test]
fn a_temporary_table_keeps_its_rows_and_a_record_its_view() {
    let mut f = fx();
    f.run("FillTemps");
    f.run("FilterTemps");
    let mut twin = round_trip(&mut f.session);
    assert_eq!(f.run("DescribeT1"), outcome(&mut twin, f.keeper, "DescribeT1"));
    let (ours, theirs) = (record_of(f.get("T1")), record_of(get(&mut twin, f.keeper, "T1")));
    let (ours, theirs) = (ours.borrow(), theirs.borrow());
    assert_eq!(ours.filter_group, theirs.filter_group);
    assert_eq!(format!("{:?}", ours.filters), format!("{:?}", theirs.filters));
    assert_eq!(format!("{:?}", ours.marks), format!("{:?}", theirs.marks));
    assert_eq!(format!("{:?}", ours.position), format!("{:?}", theirs.position));
    assert_eq!(ours.key, theirs.key);
    assert_eq!(ours.ascending, theirs.ascending);
}

fn ref_array(v: Value) -> Rc<RefCell<Vec<Value>>> {
    match v {
        Value::Array(a) => a,
        other => panic!("not an array: {other:?}"),
    }
}

fn library_cell(v: &Value) -> &Rc<RefCell<LibraryValue>> {
    match v {
        Value::Library(_, c) => c,
        other => panic!("not a library value: {other:?}"),
    }
}

#[test]
fn a_recordref_array_round_trips_node_by_node() {
    let mut f = fx();
    f.run("FillDatabase");
    f.run("OpenRefs");
    // Two of the array's RecordRefs on one record.
    let arr = ref_array(f.get("Refs"));
    let first = arr.borrow()[0].clone();
    arr.borrow_mut()[1] = first;
    drop(arr);
    let mut twin = round_trip(&mut f.session);
    let arr = ref_array(get(&mut twin, f.keeper, "Refs"));
    let arr = arr.borrow();
    assert!(Rc::ptr_eq(library_cell(&arr[0]), library_cell(&arr[1])));
    assert!(!Rc::ptr_eq(library_cell(&arr[0]), library_cell(&arr[2])));
}

#[test]
fn a_binding_holds_the_thawn_instance_a_global_holds() {
    let mut f = fx();
    f.run("Bind");
    let mut twin = round_trip(&mut f.session);
    let Value::Codeunit(held) = get(&mut twin, f.keeper, "Sub1") else {
        panic!("Sub1 holds an instance")
    };
    let bound = twin
        .events
        .bindings
        .iter()
        .find_map(|b| b.instance.upgrade())
        .expect("a live binding");
    assert!(Rc::ptr_eq(&bound, &held));
}

#[test]
fn a_table_globals_set_two_records_share_stays_one_set() {
    let mut f = fx();
    let one = record_of(f.get("Rec"));
    let two = Rc::new(RefCell::new(one.borrow().clone()));
    let set = al_builtins::value::TableGlobals::new_shared();
    set.set_instance(f.row as u32, f.session.instance(f.row));
    one.borrow_mut().globals = set.share();
    two.borrow_mut().globals = set.share();
    drop(set);
    f.set("V1", Value::Record(one.clone()));
    f.set("V2", Value::Record(two));
    drop(one);
    let mut twin = round_trip(&mut f.session);
    let (a, b) = (
        record_of(get(&mut twin, f.keeper, "V1")),
        record_of(get(&mut twin, f.keeper, "V2")),
    );
    let (x, y) = (
        a.borrow().globals.instance(f.row as u32),
        b.borrow().globals.instance(f.row as u32),
    );
    assert!(Rc::ptr_eq(
        &x.expect("the set holds the table's instance"),
        &y.expect("and so does the other")
    ));
}

#[test]
fn a_blob_two_fields_share_stays_one_buffer() {
    let mut f = fx();
    let blob = al_builtins::stream::new_blob();
    blob.borrow_mut().bytes.push(7);
    f.set("V1", Value::Blob(blob.clone()));
    f.set("V2", Value::Blob(blob.clone()));
    let mut twin = round_trip(&mut f.session);
    let (Value::Blob(a), Value::Blob(b)) = (get(&mut twin, f.keeper, "V1"), get(&mut twin, f.keeper, "V2")) else {
        panic!("globals hold blobs")
    };
    assert!(Arc::ptr_eq(&a, &b));
    a.borrow_mut().bytes.push(8);
    assert_eq!(b.borrow().bytes, vec![7, 8]);
    assert_eq!(blob.borrow().bytes, vec![7], "the original's buffer is not the twin's");
}

fn media_object() -> al_builtins::media::MediaObject {
    al_builtins::media::MediaObject {
        bytes: vec![1, 2, 3],
        mime: "image/png".into(),
        description: "d".into(),
        file_name: String::new(),
        file_column: None,
        width: 0,
        height: 0,
        pending: None,
    }
}

#[test]
fn media_and_event_counts_are_written_into_the_sessions_own_cells() {
    let mut f = fx();
    f.session.media.objects.borrow_mut().insert([1; 16], media_object());
    f.session.events.event_stats.calls.borrow_mut().insert((0, 0), 3);
    f.session.events.event_stats.active.borrow_mut().insert(1, 2);
    let image = image(&f.session);
    let mut twin = new_session(&f.session);
    let (objects, stats) = (Rc::as_ptr(&twin.media.objects), Rc::as_ptr(&twin.events.event_stats));
    twin.thaw_login_image(&image);
    assert_eq!(Rc::as_ptr(&twin.media.objects), objects, "the session's own cell");
    assert_eq!(Rc::as_ptr(&twin.events.event_stats), stats, "the session's own cell");
    assert_eq!(
        twin.media.objects.borrow().get(&[1; 16]).map(|m| m.bytes.clone()),
        Some(vec![1, 2, 3])
    );
    assert_eq!(twin.events.event_stats.calls.borrow().get(&(0, 0)), Some(&3));
    assert_eq!(twin.events.event_stats.active.borrow().get(&1), Some(&2));
    assert_eq!(f.session.state_lines(), twin.state_lines());
}

#[test]
fn two_thaws_of_one_image_share_nothing() {
    let mut f = fx();
    let p = f.peer();
    f.set("A", Value::Codeunit(p));
    f.run("FillTemps");
    let image = image(&f.session);
    let mut one = thaw(&f.session, &image);
    let mut two = thaw(&f.session, &image);
    let before = two.state_lines();
    // A global, a temporary row and a media object change in the first twin only.
    let Value::Codeunit(a) = get(&mut one, f.keeper, "A") else {
        panic!("A holds an instance")
    };
    let n = slot(&one, f.peer, "N");
    a.borrow_mut().globals[n] = Value::Integer(5);
    answer(&mut one, f.keeper, "InsertViaT2");
    one.media.objects.borrow_mut().insert([2; 16], media_object());
    assert_ne!(one.state_lines(), before);
    assert_eq!(two.state_lines(), before);
    assert_eq!(f.session.state_lines(), before);
}

#[test]
fn the_image_is_send_and_sync_and_thaws_on_another_thread() {
    let mut f = fx();
    let p = f.peer();
    f.set("A", Value::Codeunit(p));
    f.run("FillTemps");
    let image = Arc::new(image(&f.session));
    let (engine, expected) = (f.session.engine.clone(), f.session.state_lines().lines);
    let (got, noted) = std::thread::spawn(move || {
        let mut s = Session::new(engine);
        let before = al_builtins::instance_cycles::noted();
        s.thaw_login_image(&image);
        (s.state_lines().lines, al_builtins::instance_cycles::noted() - before)
    })
    .join()
    .unwrap();
    assert_eq!(got, expected);
    assert!(noted > 0, "the thawn instances are noted on the thread that made them");
}

#[test]
fn a_text_outside_ascii_in_a_row_reads_the_same_after_the_thaw() {
    let mut f = fx();
    f.run("FillDatabase");
    f.run("FillTemps");
    let mut twin = round_trip(&mut f.session);
    // The same result or the same refusal, for a table and for a temporary table that hold `é`.
    assert_eq!(f.run("ReadAccent"), outcome(&mut twin, f.keeper, "ReadAccent"));
}

#[test]
fn a_shared_instance_thawn_as_two_is_caught() {
    let mut f = fx();
    let p = f.peer();
    f.set("A", Value::Codeunit(p.clone()));
    f.set("B", Value::Codeunit(p));
    let mut image = image(&f.session);
    assert!(image.split_first_shared_instance(), "an instance with two references");
    let (mut ours, mut theirs) = (f.session.state_lines(), thaw(&f.session, &image).state_lines());
    assert_ne!(ours, theirs);
    let at = ours
        .lines
        .iter()
        .zip(&theirs.lines)
        .position(|(a, b)| a != b)
        .expect("a differing line");
    assert!(
        ours.lines[at].contains("instance") || theirs.lines[at].contains("instance"),
        "{}",
        ours.lines[at]
    );
    ours.lines.clear();
    theirs.lines.clear();
}

#[test]
fn a_field_the_image_forgets_is_caught() {
    let mut f = fx();
    f.session.clock_draws = 4;
    let mut image = image(&f.session);
    image.forget_clock_draws();
    let (ours, theirs) = (f.session.state_lines(), thaw(&f.session, &image).state_lines());
    assert_ne!(ours, theirs);
    let at = ours
        .lines
        .iter()
        .zip(&theirs.lines)
        .position(|(a, b)| a != b)
        .expect("a differing line");
    assert!(ours.lines[at].starts_with("counters"), "{}", ours.lines[at]);
}

fn refused(f: &mut Fx) -> NotImaged {
    match f.session.login_image(0) {
        Ok(_) => panic!("an image was made"),
        Err(why) => why,
    }
}

#[test]
fn a_value_with_no_exact_form_refuses_with_its_path() {
    let mut f = fx();
    let stream = Value::Library(
        LibraryKind::InStream,
        Rc::new(RefCell::new(LibraryValue::InStream(
            al_builtins::stream::InState::snapshot(b"zq9".to_vec(), Default::default()),
        ))),
    );
    f.set("Strm", stream);
    match refused(&mut f) {
        NotImaged::Value { path, what } => {
            assert!(path.starts_with("codeunit 50100 global "), "{path}");
            assert!(!path.contains("zq9") && !what.contains("zq9"), "no value in the reason");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_deferred_shape_refuses_with_its_own_reason() {
    let mut f = fx();
    f.run("FillJson");
    match refused(&mut f) {
        NotImaged::Value { what, .. } => assert!(what.contains("(deferred)"), "{what}"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_handle_held_outside_the_globals_refuses() {
    let mut f = fx();
    let held = f.get("Rec");
    assert!(matches!(refused(&mut f), NotImaged::HeldOutside { .. }));
    drop(held);
    f.session
        .login_image(0)
        .unwrap_or_else(|why| panic!("{}", why.reason()));
}

#[test]
fn a_session_that_is_not_at_rest_or_holds_refused_state_says_which() {
    let mut f = fx();
    f.keeper_instance();
    f.session.tx.try_depth = 1;
    assert!(matches!(
        refused(&mut f),
        NotImaged::NotAtRest | NotImaged::Field("tx.try_depth")
    ));
    f.session.tx.try_depth = 0;
    f.run("Schedule");
    assert_eq!(refused(&mut f), NotImaged::Field("tasks"));
    let mut g = fx();
    g.run("Uncommitted");
    assert!(matches!(refused(&mut g), NotImaged::Rows(_)));
}

#[test]
fn the_render_takes_every_value_the_image_takes() {
    let mut f = fx();
    let (p, q) = (f.peer(), f.peer());
    let back = slot(&f.session, f.peer, "Back");
    p.borrow_mut().globals[back] = Value::Codeunit(q.clone());
    q.borrow_mut().globals[back] = Value::Codeunit(p.clone());
    f.set("A", Value::Codeunit(p));
    drop(q);
    f.run("FillDatabase");
    f.run("FillTemps");
    f.run("FilterTemps");
    f.run("FillCollections");
    f.run("OpenRefs");
    f.run("Bind");
    let blob = al_builtins::stream::new_blob();
    f.set("V1", Value::Blob(blob.clone()));
    f.set("V2", Value::Blob(blob));
    let image = image(&f.session);
    let lines = f.session.state_lines();
    assert!(
        lines.opaque.is_empty(),
        "the check compares what the image restores: {:?}",
        lines.opaque
    );
    let mut twin = thaw(&f.session, &image);
    assert_eq!(twin.state_lines(), lines);
    assert_eq!(
        f.run("ReadCollections"),
        outcome(&mut twin, f.keeper, "ReadCollections")
    );
}

#[test]
fn the_size_counts_what_the_image_holds() {
    let mut f = fx();
    let p = f.peer();
    f.set("A", Value::Codeunit(p));
    f.run("FillTemps");
    let size = image(&f.session).size();
    assert_eq!(size.instances, 2, "Keeper and one Peer");
    assert_eq!(size.temp_tables, 1);
    assert_eq!(size.temp_rows, 3);
    assert!(size.rows_own_bytes <= size.rows_bytes);
    assert!(size.nodes >= size.instances + size.temp_tables);
}
