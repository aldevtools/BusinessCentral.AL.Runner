//! `Engine::compiled_codeunit` and `Engine::codeunit_object`: the lookups of a codeunit by id that `Codeunit.Run(Id)` and
//! the test runner's steps around each test method make, answered from an index built with the engine instead of a scan
//! of every object of the program on each call. They give what the scan gave: the first object in program order.

use al_ir::{ObjKind, Object, Program};
use al_vm::Engine;

fn object(kind: ObjKind, id: Option<u32>, uncompiled: bool) -> Object {
    Object {
        kind,
        id,
        uncompiled,
        ..Default::default()
    }
}

#[test]
fn a_codeunit_is_found_by_id_as_the_first_object_in_program_order() {
    let objects = vec![
        object(ObjKind::Codeunit, Some(7), true),
        object(ObjKind::Codeunit, Some(7), false),
        object(ObjKind::Codeunit, Some(7), false),
        object(ObjKind::Table, Some(8), false),
        object(ObjKind::Codeunit, Some(9), true),
        object(ObjKind::Page, Some(10), false),
        object(ObjKind::Codeunit, Some(10), false),
        object(ObjKind::Codeunit, None, false),
    ];
    let scan = |objects: &[Object], id: Option<u32>, shells: bool| {
        objects
            .iter()
            .position(|o| (o.is_kind(ObjKind::Codeunit) || shells && o.is_shell_of(ObjKind::Codeunit)) && o.id == id)
    };
    let n = objects.len();
    let engine = Engine::new(
        Program {
            objects: objects.clone().into(),
            ..Default::default()
        },
        vec![Vec::new(); n],
    );
    for id in [Some(7), Some(8), Some(9), Some(10), Some(11), None] {
        assert_eq!(
            engine.compiled_codeunit(id),
            scan(&objects, id, false),
            "compiled {id:?}"
        );
        assert_eq!(engine.codeunit_object(id), scan(&objects, id, true), "any {id:?}");
    }
    // A shell first: the compiled codeunit after it is the one that runs.
    assert_eq!(engine.compiled_codeunit(Some(7)), Some(1));
    assert_eq!(engine.codeunit_object(Some(7)), Some(0));
    // Only a shell: no compiled codeunit, but the object exists.
    assert_eq!(engine.compiled_codeunit(Some(9)), None);
    assert_eq!(engine.codeunit_object(Some(9)), Some(4));
    // Another kind with the id is not a codeunit.
    assert_eq!(engine.compiled_codeunit(Some(8)), None);
    assert_eq!(engine.compiled_codeunit(Some(10)), Some(6));
}

/// `Engine::derived`: data a crate above the VM works out from the engine's program once, shared by every session and
/// worker thread of the engine.
#[test]
fn derived_data_is_made_once_per_engine_and_type() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    struct Count(usize);
    struct Other;
    let made = AtomicUsize::new(0);
    let make = || Count(made.fetch_add(1, Ordering::Relaxed) + 1);
    let engine = Engine::new(Program::default(), Vec::new());
    let a = engine.derived(make);
    let b = engine.derived(make);
    assert!(Arc::ptr_eq(&a, &b));
    assert_eq!((a.0, made.load(Ordering::Relaxed)), (1, 1));
    // Another type has a slot of its own.
    let _ = engine.derived(|| Other);
    assert_eq!(engine.derived(make).0, 1);
    // Another engine makes its own.
    let other = Engine::new(Program::default(), Vec::new());
    assert_eq!(other.derived(make).0, 2);
    // Threads share the one value.
    let engine2 = engine.clone();
    let from_thread = std::thread::spawn(move || engine2.derived(|| Count(99)).0)
        .join()
        .unwrap();
    assert_eq!(from_thread, 1);
}

/// `Engine::reset_run_state`: a warm server runs its next request on the engine of the last one (al-runner issue 5856).
/// Every field a run sets goes back to what a new engine has; what comes from the program stays.
#[test]
fn a_kept_engine_is_reset_to_what_a_new_engine_has() {
    use al_host::testdata::{TableError, TableProvider, TableRequest, TableRows, TestData};
    use std::sync::Arc;
    struct Empty;
    impl TableProvider for Empty {
        fn company(&self) -> &str {
            "C"
        }
        fn source(&self) -> String {
            "empty".into()
        }
        fn read(&self, _: &TableRequest) -> Result<Arc<TableRows>, TableError> {
            Err(TableError::Absent)
        }
    }
    let program = Program {
        objects: vec![object(ObjKind::Codeunit, Some(50100), false)].into(),
        ..Program::default()
    };
    let fresh = Engine::new(program.clone(), Vec::new());
    let mut engine = Engine::new(program, Vec::new());
    let uid = engine.uid;
    {
        let e = Arc::get_mut(&mut engine).expect("not shared");
        e.unseeded.insert(0);
        e.demo_tables.insert(0);
        e.company_refusal = Some("no company".into());
        e.skipped_apps.push("`App` by Publisher".into());
        e.resource_overrides
            .write()
            .unwrap()
            .insert(0, Arc::new(vec![("a.txt".into(), b"x".to_vec())]));
        e.resources_read.store(true, std::sync::atomic::Ordering::Relaxed);
        e.test_data = Some(TestData::new(Arc::new(Empty)));
        e.reset_run_state();
    }
    assert_eq!(engine.unseeded, fresh.unseeded);
    assert_eq!(engine.demo_tables, fresh.demo_tables);
    assert_eq!(engine.company_refusal, fresh.company_refusal);
    assert_eq!(engine.skipped_apps, fresh.skipped_apps);
    assert!(engine.resource_overrides.read().unwrap().is_empty());
    assert!(!engine.resources_read.load(std::sync::atomic::Ordering::Relaxed));
    assert!(engine.test_data.is_none());
    // What comes from the program stays.
    assert_eq!(engine.uid, uid);
    assert_eq!(
        engine.compiled_codeunit(Some(50100)),
        fresh.compiled_codeunit(Some(50100))
    );
    assert_eq!(engine.platform_unseeded, fresh.platform_unseeded);
}
