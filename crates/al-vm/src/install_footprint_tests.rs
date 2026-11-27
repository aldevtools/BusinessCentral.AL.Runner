//! Each resource kind and each "touches everything" kind an install session records (ADR-079 decision 1).

use super::{InstallFootprint, Lasting};
use crate::{Engine, Session};
use al_data::memory::Resource;

const SRC: &str = r#"
table 50100 "Auto Row" { fields { field(1; Id; Integer) { AutoIncrement = true; } field(2; Name; Text[20]) { } } keys { key(PK; Id) { } } }
codeunit 50100 Probe
{
    procedure DeleteKey()
    begin
        DeleteEncryptionKey();
    end;
    procedure KeyExists(): Boolean
    begin
        exit(EncryptionKeyExists());
    end;
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
    all.touch_everything("a page");
    let none = InstallFootprint::default();
    assert!(all.reads_from(&none, 1) && none.reads_from(&all, 1) && all.collides(&none));
    assert!(!none.reads_from(&none, 1) && !none.collides(&none));
}

#[test]
fn store_resources_keep_the_store_rules() {
    let t = al_data::contract::table();
    let key = al_data::memory::RowKey::of(&t, vec![al_data::contract::row("B", 1, 1)[0].clone()]);
    let row = Lasting::Store(Resource::Row("C".into(), t.object, key));
    let extent = Lasting::Store(Resource::Extent("C".into(), t.object));
    assert!(extent.read_sees_write(&row) && row.writes_collide(&row) && !extent.writes_collide(&extent));
}

#[test]
fn builtins_outside_the_kept_receivers_touch_everything() {
    let all = |name: &str| {
        let (r, m) = name.split_once('.').unwrap_or((name, ""));
        super::builtin_touches_everything(r, m.split('#').next().unwrap_or(m))
    };
    for kept in [
        "Record.Insert#2",
        "Text.StrSubstNo",
        "NavApp.GetModuleInfo",
        "Session.CurrentSessionId",
    ] {
        assert!(!all(kept), "{kept}");
    }
    for every in [
        "HttpClient.Send",
        "File.Create",
        "TaskScheduler.CreateTask#1",
        "Session.StartSession#1",
        "DataTransfer.CopyRows",
        "NavApp.GetArchiveRecordRef",
        "Media.ImportStream",
        "A.Receiver.Nobody.Named",
    ] {
        assert!(all(every), "{every}");
    }
    // The real names have the `Receiver.Method` shape the classifier reads.
    let names: Vec<&str> = al_builtins::builtin_id::BuiltinId::all().map(|id| id.name()).collect();
    assert!(names.iter().any(|&n| n.starts_with("HttpClient.") && all(n)));
    assert!(names.iter().any(|&n| n.starts_with("Record.") && !all(n)));
}

/// The receivers kept by method (`System`, `Database`, `Blob`, `Query`): a method that reads or writes state the footprint
/// does not hold touches everything, the encryption key's eight among them (the review of PR 8865, finding M2).
#[test]
fn methods_of_receivers_kept_by_method_that_reach_lasting_state_touch_everything() {
    let all = |name: &str| {
        let (r, m) = name.split_once('.').unwrap_or((name, ""));
        super::builtin_touches_everything(r, m)
    };
    for kept in [
        "System.Format",
        "System.CalcDate",
        "Database.CompanyName",
        "Blob.CreateInStream",
        "Query.Read",
    ] {
        assert!(!all(kept), "{kept}");
    }
    for every in [
        "System.DeleteEncryptionKey",
        "System.EncryptionKeyExists",
        "System.CreateEncryptionKey",
        "System.ImportEncryptionKey",
        "System.ExportEncryptionKey",
        "System.EncryptionEnabled",
        "System.Encrypt",
        "System.Decrypt",
        "System.ImportObjects",
        "System.CodeCoverageLog",
        "System.SomethingNew",
        "Database.AlterKey",
        "Database.ImportData",
        "Database.RegisterTableConnection",
        "Database.SomethingNew",
        "Blob.Import",
        "Blob.Export",
        "Query.SaveAsCsv",
    ] {
        assert!(all(every), "{every}");
    }
}

#[test]
fn a_builtin_that_touches_everything_marks_the_session() {
    let l =
        al_testkit::compile("codeunit 50100 Probe { procedure Run() var C: HttpClient; begin C.Timeout(1000); end; }");
    let engine = Engine::new(l.program.clone(), al_testkit::blocked(&l));
    let mut s = Session::new(engine);
    s.install_footprint_start();
    let inst = s.codeunit_instance(0);
    let _ = s.call(0, 0, inst, vec![]);
    let fp = s.install_footprint_take().unwrap();
    assert!(fp.everything.keys().any(|w| w.contains("HttpClient")), "{fp:?}");
}

/// A builtin whose operands are all values runs on the plain path (`run_plain_builtin`), not through `run_builtin`: it
/// marks the session all the same (found by the neighbor app `install-order-footprint-everything4`).
#[test]
fn a_plain_builtin_that_touches_everything_marks_the_session() {
    let fp = run_src(
        "codeunit 50100 Probe { procedure Run() begin if TaskScheduler.CanCreateTask() then; end; }",
        "Run",
    );
    assert!(fp.everything.keys().any(|w| w.contains("TaskScheduler")), "{fp:?}");
}

const ROW_VERSION_SRC: &str = r#"
table 50100 "Versioned" { fields { field(1; Id; Integer) { } field(2; Name; Text[20]) { } field(3; MaxVersion; Integer) { FieldClass = FlowField; CalcFormula = count("Versioned" where(SystemRowVersion = filter(>0))); } } keys { key(PK; Id) { } } }
codeunit 50100 Probe
{
    procedure FieldRead(): BigInteger
    var R: Record "Versioned";
    begin
        if R.FindFirst() then;
        exit(R.SystemRowVersion);
    end;
    procedure FilterOn()
    var R: Record "Versioned";
    begin
        R.SetFilter(SystemRowVersion, '>%1', 5);
        if R.IsEmpty() then;
    end;
    procedure Dynamic()
    var R: RecordRef; F: FieldRef;
    begin
        R.Open(50100);
        F := R.Field(1);
    end;
    procedure FlowField()
    var R: Record "Versioned";
    begin
        R.CalcFields(MaxVersion);
    end;
    procedure OtherField(): Text
    var R: Record "Versioned";
    begin
        if R.FindFirst() then;
        exit(R.Name);
    end;
}
"#;

fn run_src(src: &str, routine: &str) -> InstallFootprint {
    let l = al_testkit::compile(src);
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
fn every_read_of_the_system_row_version_field_reads_the_row_version() {
    for routine in ["FieldRead", "FilterOn", "Dynamic", "FlowField"] {
        let fp = run_src(ROW_VERSION_SRC, routine);
        assert!(fp.reads.contains(&Lasting::RowVersion), "{routine}: {fp:?}");
    }
}

#[test]
fn a_read_of_another_field_does_not_read_the_row_version() {
    let fp = run_src(ROW_VERSION_SRC, "OtherField");
    assert!(!fp.reads.contains(&Lasting::RowVersion), "{fp:?}");
}

/// Fixture tables with the platform's ids (as al-vm tests/store/published_application_tenant_id.rs).
const SERVED_SRC: &str = r#"
table 2000000153 "NAV App Installed App" { fields { field(1; "App ID"; Guid) { } field(3; Name; Text[250]) { } } keys { key(PK; "App ID") { } } }
table 2000000038 AllObj { fields { field(1; "Object Type"; Option) { OptionMembers = TableData,Table; } field(3; "Object ID"; Integer) { } field(4; "Object Name"; Text[30]) { } field(60; "App Package ID"; Guid) { } field(61; "App Runtime Package ID"; Guid) { } field(62; "Object Subtype"; Text[30]) { } } keys { key(PK; "Object Type", "Object ID") { } } }
codeunit 50100 Probe
{
    procedure AppTable()
    var R: Record "NAV App Installed App";
    begin
        if R.FindFirst() then;
    end;
    procedure OtherServed()
    var R: Record AllObj;
    begin
        if R.FindFirst() then;
    end;
}
"#;

#[test]
fn a_read_of_an_app_table_reads_the_presence_of_every_app() {
    let fp = run_src(SERVED_SRC, "AppTable");
    assert!(fp.apps_all && fp.everything.is_empty(), "{fp:?}");
    let other = InstallFootprint::default();
    assert!(fp.reads_from(&other, 7), "every app's presence is read");
}

#[test]
fn a_read_of_another_served_table_touches_everything() {
    let fp = run_src(SERVED_SRC, "OtherServed");
    assert!(fp.touches_everything() && !fp.apps_all, "{fp:?}");
}

#[test]
fn a_change_the_recorder_did_not_note_touches_everything() {
    let l = al_testkit::compile(SRC);
    let engine = Engine::new(l.program.clone(), al_testkit::blocked(&l));
    let mut s = Session::new(engine);
    // Unrecorded: changes made behind the recorder's back stand in for a site that forgot its note.
    s.install_footprint_start();
    s.isolated_storage
        .insert(("scope".into(), "K".into()), ("v".into(), false, false));
    s.autoinc.insert((String::new(), 7), 3);
    let fp = s.install_footprint_take().unwrap();
    assert!(fp.touches_everything(), "{fp:?}");
    // A recorded change passes the net.
    let probe = (s.engine.program.objects.iter())
        .position(|o| o.name == "Probe")
        .unwrap();
    let routine = (s.engine.program.objects[probe].routines.iter()).position(|r| r.name == "StorageSet");
    s.install_footprint_start();
    let inst = s.codeunit_instance(probe);
    s.call(probe, routine.unwrap_or(0), inst, vec![]).unwrap();
    let fp = s.install_footprint_take().unwrap();
    assert!(!fp.touches_everything(), "{fp:?}");
}

#[test]
fn an_impact_trace_that_was_on_keeps_running_and_the_session_is_recorded_all_the_same() {
    // The baseline of `al-runner impact-index build` runs the install code with the impact trace on (al-testrun
    // `baseline::run_traced`): the footprint takes the session's own part of it, and the outer trace keeps every note.
    let l = al_testkit::compile(SRC);
    let engine = Engine::new(l.program.clone(), al_testkit::blocked(&l));
    let probe = (engine.program.objects.iter())
        .position(|o| o.name == "Probe")
        .expect("Probe");
    let mut s = Session::new(engine);
    let ri = |s: &Session, n: &str| {
        (s.engine.program.objects[probe].routines.iter())
            .position(|r| r.name == n)
            .unwrap()
    };
    s.enable_impact_trace();
    let (before, during) = (ri(&s, "StorageGet"), ri(&s, "AutoInsert"));
    let inst = s.codeunit_instance(probe);
    s.call(probe, before, inst.clone(), vec![]).unwrap();
    s.install_footprint_start();
    s.call(probe, during, inst, vec![]).unwrap();
    s.commit_transaction();
    let fp = s.install_footprint_take().expect("recorded");
    assert!(fp.everything.is_empty(), "{fp:?}");
    assert!(!fp.apps_met.is_empty(), "{fp:?}");
    let outer = s.take_impact_trace();
    let routines: Vec<usize> = outer.routines.iter().map(|r| r.1).collect();
    assert!(routines.contains(&before) && routines.contains(&during), "{outer:?}");
    assert!(
        !outer.writes.is_empty(),
        "the insert of the recorded session: {outer:?}"
    );
}

/// A store that lost its recorder while the install ran (a store put in place of the recorded one) leaves no record of
/// its rows: that counts as touching everything, never as an empty footprint.
#[test]
fn a_store_whose_record_was_lost_touches_everything() {
    let l = al_testkit::compile(SRC);
    let engine = Engine::new(l.program.clone(), al_testkit::blocked(&l));
    let mut s = Session::new(engine);
    s.install_footprint_start();
    let _ = s.store.footprint_take();
    let fp = s.install_footprint_take().unwrap();
    assert!(fp.touches_everything(), "{fp:?}");
}

/// The encryption key is session state the baseline capture does not hold: a subset run that deleted it must not leave it
/// deleted for the next run (`Session::set_install_session_state`, the review of PR 8865, finding M2).
#[test]
fn the_install_session_state_puts_the_encryption_key_back() {
    let l = al_testkit::compile(SRC);
    let engine = Engine::new(l.program.clone(), al_testkit::blocked(&l));
    let probe = (engine.program.objects.iter())
        .position(|o| o.name == "Probe")
        .expect("Probe");
    let mut s = Session::new(engine);
    let ri = |s: &Session, n: &str| {
        (s.engine.program.objects[probe].routines.iter())
            .position(|r| r.name == n)
            .unwrap()
    };
    let state = s.install_session_state();
    let inst = s.codeunit_instance(probe);
    let (delete, exists) = (ri(&s, "DeleteKey"), ri(&s, "KeyExists"));
    s.call(probe, delete, inst.clone(), vec![]).unwrap();
    s.set_install_session_state(&state);
    let (v, _) = s.call(probe, exists, inst, vec![]).unwrap();
    assert_eq!(format!("{v:?}"), "Boolean(true)");
}

/// The indexed writes answer what [`Lasting::read_sees_write`] and [`Lasting::writes_collide`] answer against every write
/// (the review of PR 8865, finding L7).
#[test]
fn indexed_writes_answer_what_the_rules_answer() {
    let all = [
        Lasting::Store(Resource::Extent("C".into(), 1)),
        Lasting::Store(Resource::Table(2)),
        Lasting::AutoIncrement("C".into(), 1),
        Lasting::Sequence("C".into(), "S".into()),
        Lasting::IsolatedStorage("M".into(), "K".into()),
        Lasting::RowVersion,
    ];
    let mut sets: Vec<std::collections::BTreeSet<Lasting>> = vec![Default::default()];
    for (i, a) in all.iter().enumerate() {
        sets.push([a.clone()].into());
        for b in &all[i + 1..] {
            sets.push([a.clone(), b.clone()].into());
        }
    }
    for set in &sets {
        let fp = InstallFootprint {
            writes: set.clone(),
            ..Default::default()
        };
        let x = fp.written();
        for r in &all {
            let one: std::collections::BTreeSet<Lasting> = [r.clone()].into();
            let seen = set.iter().any(|w| r.read_sees_write(w));
            assert_eq!(x.any_seen_by(&one), seen, "read {r:?} of {set:?}");
            let collide = set.iter().any(|w| r.writes_collide(w));
            assert_eq!(x.any_collides_with(&one), collide, "write {r:?} of {set:?}");
        }
    }
}

/// A pure framework member served in process (al-host `inproc.txt`) and a server class member that keeps nothing touch
/// no lasting resource; every other .NET member, and a framework member that is not listed, touches everything
/// (al-runner issue 8954).
#[test]
fn the_dotnet_members_that_keep_nothing() {
    let kept = |t: &str, m: Option<&str>| super::dotnet_member_kept(t, m, 0);
    assert!(kept("System.Text.StringBuilder", None), "constructor");
    assert!(kept("System.Text.StringBuilder", Some("Append")));
    assert!(kept("System.Text.StringBuilder", Some("toString")), "case");
    assert!(kept(
        "Microsoft.Dynamics.Nav.Types.ALConfigSettings",
        Some("ApiServicesEnabled")
    ));
    assert!(kept(
        al_builtins::dotnet_sigs::TENANT_SETTINGS_HELPER,
        Some("TryGetStringTenantSetting")
    ));
    assert!(!kept("System.Text.StringBuilder", Some("NoSuchMember")));
    assert!(
        !kept("System.Text.StringBuilderX", Some("Append")),
        "the type matches exactly"
    );
    assert!(!kept(
        al_builtins::dotnet_sigs::TENANT_SETTINGS_HELPER,
        Some("SetTestTenantEnvironmentType")
    ));
    assert!(!kept("System.IO.File", Some("WriteAllText")));
}

/// A member of a CLR array type (`System.Byte[]`, any `T[]`) is the member of `System.Array` that `inproc.txt` lists
/// (`Byte[].Length` and `Byte[].GetValue` in Cryptography Management Impl.); a member it does not list, a multi-
/// dimensional array type and a type that only contains `[]` still touch everything (al-runner issue 8954).
#[test]
fn an_array_member_is_the_system_array_member() {
    let kept = |t: &str, m: Option<&str>| super::dotnet_member_kept(t, m, 0);
    assert!(kept("System.Byte[]", Some("Length")));
    assert!(kept("System.Byte[]", Some("GetValue")));
    assert!(kept("System.Byte[]", Some("SetValue")));
    assert!(kept("System.String[]", Some("GetValue")));
    assert!(!kept("System.Byte[]", Some("Clone")));
    assert!(!kept("System.Byte[]", None), "constructor");
    assert!(!kept("System.Byte[,]", Some("Length")));
    assert!(!kept("[]", Some("Length")));
    assert!(!kept(
        "System.Collections.Generic.List`1[System.Byte[]]",
        Some("Length")
    ));
}

/// An encoding `Encoding.UTF8`, `ASCII` or `Unicode` answers and an algorithm `HashAlgorithm.Create` answers have a
/// runtime type of their own (`System.Text.UTF8Encoding+UTF8EncodingSealed`, `System.Security.Cryptography.SHA256Managed`);
/// their members are the `System.Text.Encoding` and `HashAlgorithm` members `inproc.txt` lists. A member it does not list
/// and another subclass still touch everything (al-runner issue 8954, neighbor app install-order-footprint-dotnet-array).
#[test]
fn an_encoding_or_hash_instance_member_is_the_listed_base_member() {
    let kept = |t: &str, m: Option<&str>| super::dotnet_member_kept(t, m, 0);
    assert!(kept("System.Text.UTF8Encoding+UTF8EncodingSealed", Some("GetBytes")));
    assert!(kept("System.Text.ASCIIEncoding+ASCIIEncodingSealed", Some("GetString")));
    assert!(kept("System.Text.UnicodeEncoding", Some("GetByteCount")));
    assert!(kept("System.Security.Cryptography.SHA256Managed", Some("ComputeHash")));
    assert!(kept(
        "System.Security.Cryptography.MD5CryptoServiceProvider",
        Some("Dispose")
    ));
    assert!(!kept(
        "System.Text.UTF8Encoding+UTF8EncodingSealed",
        Some("GetPreamble")
    ));
    assert!(!kept("System.Text.UTF32Encoding", Some("GetBytes")));
    assert!(!kept("System.Security.Cryptography.HMACSHA256", Some("ComputeHash")));
}

/// The `MemoryStream` that Cryptography Management Impl. `GenerateHash(InStream, ...)` makes, copies an InStream into and
/// reads with `ToArray` keeps nothing: its no-argument constructor and `ToArray` are served in process (al-host
/// `inproc.txt`). Every other constructor and member of the type still touches everything (al-runner issue 9046).
#[test]
fn the_memory_stream_of_generate_hash_keeps_nothing() {
    let kept = super::dotnet_member_kept;
    let ms = "System.IO.MemoryStream";
    assert!(kept(ms, None, 0), "MemoryStream()");
    assert!(kept(ms, Some("ToArray"), 0));
    assert!(!kept(ms, None, 1), "MemoryStream(Byte[]) and MemoryStream(Int32)");
    assert!(!kept(ms, None, 2), "MemoryStream(Byte[], Boolean)");
    for member in [
        "WriteTo",
        "CopyTo",
        "GetBuffer",
        "Write",
        "Position",
        "Length",
        "Dispose",
    ] {
        assert!(!kept(ms, Some(member), 1), "{member}");
        assert!(!kept(ms, Some(member), 0), "{member}");
    }
    assert!(!kept("System.IO.FileStream", None, 0));
}

/// Every method kept by name is the method of a real builtin: an entry no builtin has keeps nothing, and the builtin
/// it meant still touches everything (issue 9046: `DT2Date` was listed as `DT`).
#[test]
fn every_kept_method_names_a_builtin() {
    let names: Vec<&str> = al_builtins::builtin_id::BuiltinId::all().map(|id| id.name()).collect();
    for (receiver, methods) in super::KEPT_METHODS {
        for method in *methods {
            let prefix = format!("{receiver}.{method}#");
            assert!(names.iter().any(|n| n.starts_with(&prefix)), "{receiver}.{method}");
        }
    }
    for receiver in super::KEPT_RECEIVERS {
        let prefix = format!("{receiver}.");
        assert!(names.iter().any(|n| n.starts_with(&prefix)), "{receiver}");
    }
    for method in super::NAVAPP_KEPT_METHODS {
        let prefix = format!("NavApp.{method}#");
        assert!(names.iter().any(|n| n.starts_with(&prefix)), "NavApp.{method}");
    }
    for method in super::SESSION_TOUCHING_METHODS {
        let prefix = format!("Session.{method}#");
        assert!(names.iter().any(|n| n.starts_with(&prefix)), "Session.{method}");
    }
}

/// Issue 9046: the date conversions a pinned corpus case measures as functions of their arguments keep nothing. Their
/// neighbor without a pin (`DaTi2Variant`) and every `TaskScheduler` method still touch everything.
#[test]
fn date_conversions_keep_nothing() {
    let all = |name: &str| {
        let (r, m) = name.split_once('.').unwrap_or((name, ""));
        super::builtin_touches_everything(r, m)
    };
    for kept in [
        "System.DT2Date",
        "System.DT2Time",
        "System.DMY2Date",
        "System.DWY2Date",
        "System.Date2DMY",
        "System.Date2DWY",
        "System.Variant2Date",
        "System.Variant2Time",
    ] {
        assert!(!all(kept), "{kept}");
    }
    for every in [
        "System.DaTi2Variant",
        "TaskScheduler.CanCreateTask",
        "TaskScheduler.CreateTask",
        "TaskScheduler.TaskExists",
        "TaskScheduler.SetTaskReady",
        "TaskScheduler.CancelTask",
        "TaskScheduler.SomethingNew",
    ] {
        assert!(all(every), "{every}");
    }
}

/// Issue 9260: `CompanyProperty.DisplayName` and `CompanyProperty.ID` read the session company's row of the Company
/// table (corpus `tests/al-language/session-system/company/CompanyRecord.al:15-44`), a read the store records;
/// `NavApp.GetCallerCallstackModuleInfos` reads the module info of the apps whose code is on the call stack
/// (`tests/al-language/session-system/TestNavAppCallerCallstackModuleInfos.al:5-7`), apps the session already meets.
/// The other `CompanyProperty` and `NavApp` methods keep touching everything.
#[test]
fn company_property_reads_and_the_caller_callstack_keep_nothing() {
    let all = |name: &str| {
        let (r, m) = name.split_once('.').unwrap_or((name, ""));
        super::builtin_touches_everything(r, m)
    };
    for kept in [
        "CompanyProperty.DisplayName",
        "CompanyProperty.ID",
        "NavApp.GetCallerCallstackModuleInfos",
    ] {
        assert!(!all(kept), "{kept}");
    }
    for every in [
        "CompanyProperty.UrlName",
        "CompanyProperty.SomethingNew",
        "NavApp.IsInstalling",
        "NavApp.GetArchiveRecordRef",
    ] {
        assert!(all(every), "{every}");
    }
}

/// The platform table Company as the symbols declare it, and install code that reads the two company properties.
const COMPANY_SRC: &str = r#"
table 2000000006 Company
{
    DataPerCompany = false;
    fields
    {
        field(1; Name; Text[30]) { }
        field(2; "Evaluation Company"; Boolean) { }
        field(3; "Display Name"; Text[250]) { }
        field(8000; Id; Guid) { }
        field(8005; "Business Profile Id"; Text[250]) { }
    }
    keys { key(PK; Name) { } }
}
codeunit 50100 Probe
{
    procedure DisplayName(): Text
    begin
        exit(CompanyProperty.DisplayName());
    end;
    procedure Id(): Guid
    begin
        exit(CompanyProperty.ID());
    end;
    procedure Callers(): Integer
    begin
        exit(Ask().Count());
    end;
    local procedure Ask(): List of [ModuleInfo]
    begin
        exit(NavApp.GetCallerCallstackModuleInfos());
    end;
}
"#;

/// Issue 9260: each company property reads the session company's row of the Company table, recorded by the store, so an
/// install that writes that row is read from; the caller call stack reads no lasting resource.
#[test]
fn company_property_reads_read_the_company_row() {
    for routine in ["DisplayName", "Id"] {
        let fp = run_src(COMPANY_SRC, routine);
        assert!(!fp.touches_everything() && fp.writes.is_empty(), "{routine}: {fp:?}");
        // Object 0 of the program: the Company table.
        let company_row = |r: &Lasting| matches!(r, Lasting::Store(Resource::Row(c, 0, _)) if c.is_empty());
        assert!(fp.reads.iter().any(company_row), "{routine}: {fp:?}");
        let writer = InstallFootprint {
            writes: fp.reads.clone(),
            ..Default::default()
        };
        assert!(
            fp.reads_from(&writer, 7),
            "{routine}: a write of the Company row is read"
        );
    }
    let fp = run_src(COMPANY_SRC, "Callers");
    assert!(
        !fp.touches_everything() && fp.reads.is_empty() && fp.writes.is_empty(),
        "{fp:?}"
    );
}
