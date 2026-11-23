//! Each resource kind and each "touches everything" kind an install session records (ADR-079 decision 1).

use super::{InstallFootprint, Lasting};
use crate::{Engine, Session};
use al_data::memory::Resource;

const SRC: &str = r#"
table 50100 "Auto Row" { fields { field(1; Id; Integer) { AutoIncrement = true; } field(2; Name; Text[20]) { } } keys { key(PK; Id) { } } }
codeunit 50100 Probe
{
    procedure Sequence()
    var N: BigInteger;
    begin
        if not NumberSequence.Exists('S') then NumberSequence.Insert('S');
        N := NumberSequence.Next('S');
    end;
    procedure StorageGet(): Boolean
    var V: Text;
    begin
        exit(IsolatedStorage.Get('K', V));
    end;
    procedure StorageSet()
    begin
        IsolatedStorage.Set('K', 'v');
    end;
    procedure AutoInsert()
    var R: Record "Auto Row";
    begin
        R.Name := 'x'; R.Insert();
    end;
    procedure RowVersion(): BigInteger
    begin
        exit(Database.LastUsedRowVersion());
    end;
    procedure ModuleInfo(): Boolean
    var M: ModuleInfo;
    begin
        exit(NavApp.GetModuleInfo('{00000000-0000-0000-0000-000000000001}', M));
    end;
}
"#;

fn run(routine: &str) -> InstallFootprint {
    let l = al_testkit::compile(SRC);
    let engine = Engine::new(l.program.clone(), al_testkit::blocked(&l));
    let probe = (engine.program.objects.iter())
        .position(|o| o.name == "Probe")
        .expect("Probe");
    let mut s = Session::new(engine);
    let ri = (s.engine.program.objects[probe].routines.iter())
        .position(|r| r.name == routine)
        .expect("routine");
    s.install_footprint_start();
    let inst = s.codeunit_instance(probe);
    s.call(probe, ri, inst, vec![])
        .unwrap_or_else(|e| panic!("{routine}: {e:?}"));
    s.install_footprint_take().expect("recorded")
}

#[test]
fn a_number_sequence_is_read_and_written() {
    let fp = run("Sequence");
    let seq = |l: &Lasting| matches!(l, Lasting::Sequence(_, n) if n == "S");
    assert!(fp.reads.iter().any(seq) && fp.writes.iter().any(seq), "{fp:?}");
}

#[test]
fn isolated_storage_get_reads_and_set_writes() {
    let get = run("StorageGet");
    let key = |l: &Lasting| matches!(l, Lasting::IsolatedStorage(_, k) if k == "K");
    assert!(get.reads.iter().any(key) && !get.writes.iter().any(key), "{get:?}");
    let set = run("StorageSet");
    assert!(set.writes.iter().any(key), "{set:?}");
    assert!(get.reads_from(&set, u32::MAX));
}

#[test]
fn an_insert_that_takes_an_auto_increment_number_reads_and_writes_the_counter() {
    let fp = run("AutoInsert");
    let counter = |l: &Lasting| matches!(l, Lasting::AutoIncrement(..));
    assert!(fp.reads.iter().any(counter) && fp.writes.iter().any(counter), "{fp:?}");
    assert!(
        fp.collides(&fp.clone()),
        "two sessions that take numbers of one counter collide"
    );
}

#[test]
fn last_used_row_version_reads_the_row_version_which_every_row_write_writes() {
    let fp = run("RowVersion");
    assert!(fp.reads.contains(&Lasting::RowVersion), "{fp:?}");
    let writer = run("AutoInsert");
    assert!(fp.reads_from(&writer, u32::MAX));
}

#[test]
fn the_apps_whose_code_ran_are_met() {
    let fp = run("StorageGet");
    assert!(!fp.apps_met.is_empty(), "the probe's own app: {fp:?}");
    assert!(fp.everything.is_empty(), "{fp:?}");
}

#[test]
fn touching_everything_reads_from_and_collides_with_every_session() {
    let mut all = InstallFootprint::default();
    all.everything.insert("a page".into());
    let none = InstallFootprint::default();
    assert!(all.reads_from(&none, 1) && none.reads_from(&all, 1) && all.collides(&none));
    assert!(!none.reads_from(&none, 1) && !none.collides(&none));
}

#[test]
fn store_resources_keep_the_store_rules() {
    let row = Lasting::Store(Resource::Row("C".into(), 7, vec![1]));
    let extent = Lasting::Store(Resource::Extent("C".into(), 7));
    assert!(extent.read_sees_write(&row) && row.writes_collide(&row) && !extent.writes_collide(&extent));
}
