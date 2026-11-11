//! ADR-073 Phase 2: an app an engine leaves uninstalled (`Engine::uninstalled_apps`, read through `Engine::app_installed`)
//! is not on that engine's tenant. The corpus job runs the try-write suite's app in the main pass, and BC's main job does
//! not have that app: its subscribers do not run, `NavApp.GetModuleInfo` does not find it, and a lookup by an id known at
//! run time does not find its objects. The listings: al-test-cli tests/suite/uninstalled_app_listings.rs.

const MAIN: &str = "
    table 50100 MainRow { fields { field(1; K; Integer) { } } }
    codeunit 50101 MainPub { [IntegrationEvent(false, false)] procedure OnThing() begin end;
        procedure Raise() begin OnThing(); end; }
    codeunit 50102 C {
        procedure Fired(): Integer var P: Codeunit MainPub; R: Record MainRow; begin P.Raise(); exit(R.Count()); end;
        procedure Module(): Boolean var M: ModuleInfo; begin exit(NavApp.GetModuleInfo('bb000000-0000-0000-0000-000000000002', M)); end;
        procedure RunById(Id: Integer) begin Codeunit.Run(Id); end;
        procedure OpenById(Id: Integer) var R: RecordRef; begin R.Open(Id); end;
    }";

const TW: &str = "
    table 81702 TwRow { fields { field(1; K; Integer) { } } }
    codeunit 81701 TwSub { [EventSubscriber(ObjectType::Codeunit, Codeunit::MainPub, 'OnThing', '', false, false)]
        local procedure H() var R: Record MainRow; begin R.K := 1; R.Insert(); end; }
    codeunit 81703 TwRun { trigger OnRun() begin end; }";

/// The two apps on an engine that leaves the try-write app uninstalled when `uninstalled`.
fn built(uninstalled: bool) -> al_testkit::Built {
    let app = |id: &str, name: &str, deps: Vec<String>, src: &str| al_sema::workspace::AppSource {
        id: id.into(),
        name: name.into(),
        dependencies: deps,
        files: vec![(format!("{name}.al"), src.into())],
    };
    let main_id = "aa000000-0000-0000-0000-000000000001";
    let l = al_testkit::compile_apps(vec![
        app(main_id, "main", vec![], MAIN),
        app("bb000000-0000-0000-0000-000000000002", "tw", vec![main_id.into()], TW),
    ]);
    let tw = l.program.apps.iter().position(|a| a.name == "tw").unwrap() as u32;
    let mut engine = al_testkit::engine(&l);
    if uninstalled {
        std::sync::Arc::get_mut(&mut engine).unwrap().uninstalled_apps = [tw].into();
    }
    al_testkit::Built {
        session: al_vm::Session::new(engine),
        lowered: l,
    }
}

#[test]
fn an_installed_app_is_seen_everywhere() {
    let mut b = built(false);
    assert_eq!(al_testkit::int(b.run_in("C", "Fired")), 1);
    assert!(al_testkit::boolean(b.run_in("C", "Module")));
}

#[test]
fn an_uninstalled_app_is_not_seen() {
    let mut b = built(true);
    assert_eq!(
        al_testkit::int(b.run_in("C", "Fired")),
        0,
        "its subscriber does not run"
    );
    assert!(
        !al_testkit::boolean(b.run_in("C", "Module")),
        "NavApp.GetModuleInfo does not find it"
    );
}

#[test]
fn a_lookup_by_a_run_time_id_does_not_find_an_object_of_an_uninstalled_app() {
    let call = |uninstalled: bool, routine: &str, id: i32| {
        let mut b = built(uninstalled);
        let (oi, ri) = b.routine_in("C", routine);
        b.call(oi, ri, vec![al_builtins::value::Value::Integer(id)]).map(|_| ())
    };
    assert!(call(false, "RunById", 81703).is_ok());
    assert!(call(true, "RunById", 81703).is_err());
    assert!(call(false, "OpenById", 81702).is_ok());
    assert!(call(true, "OpenById", 81702).is_err());
}
