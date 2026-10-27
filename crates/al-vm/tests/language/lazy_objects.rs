//! ADR-054: an engine over a program whose objects are made on first read. Building the engine and a session reads
//! object heads only (no object is made), a call makes the objects it runs, and the engine equals the one built over
//! the finished objects (test plan items 3 and 5).

use al_ir::{Object, ObjectHead, ObjectSource, Objects};
use al_testkit::Built;
use al_vm::Engine;
use std::sync::Arc;

const SRC: &str = r#"
table 50300 "Lazy T"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Qty; Decimal) { InitValue = 3; }
    }
    keys { key(PK; Id) { Clustered = true; } }
}
codeunit 50301 Pub
{
    [IntegrationEvent(false, false)]
    procedure OnWork(var Total: Integer) begin end;
}
codeunit 50302 Sub
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::Pub, 'OnWork', '', false, false)]
    local procedure Add(var Total: Integer) begin Total += 1; end;
}
codeunit 50303 ManualSub
{
    EventSubscriberInstance = Manual;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::Pub, 'OnWork', '', false, false)]
    local procedure Add(var Total: Integer) begin Total += 10; end;
}
codeunit 50304 Unused
{
    procedure Nothing() begin end;
}
codeunit 50305 C
{
    procedure Run(): Integer
    var
        P: Codeunit Pub;
        Total: Integer;
    begin
        P.OnWork(Total);
        exit(Total);
    end;
}
"#;

/// Makes each object from a copy of the finished ones.
struct Copies(Vec<Object>);

impl ObjectSource for Copies {
    fn object(&self, i: usize) -> Object {
        self.0[i].clone()
    }
}

/// `program` with every slot empty and the heads given.
fn lazy(program: &al_ir::Program) -> al_ir::Program {
    let all: Vec<Object> = program.objects.iter().cloned().collect();
    let heads: Arc<[ObjectHead]> = all.iter().map(ObjectHead::of).collect();
    let mut p = program.clone();
    p.objects = Objects::lazy_with_heads(vec![None; all.len()], Arc::new(Copies(all)), heads);
    p
}

fn index_of(program: &al_ir::Program, name: &str) -> usize {
    program.objects.iter().position(|o| o.name == name).unwrap()
}

#[test]
fn engine_and_session_start_make_no_object_and_a_call_makes_what_it_runs() {
    let mut lowered = al_testkit::compile(SRC);
    let eager = lowered.program.clone();
    let (c, unused, table) = (
        index_of(&eager, "C"),
        index_of(&eager, "Unused"),
        index_of(&eager, "Lazy T"),
    );
    let run = eager.objects[c].routines.iter().position(|r| r.name == "Run").unwrap();
    lowered.program = Arc::new(lazy(&eager));
    let mut built = Built::new(lowered);
    let objects = &built.session.engine.program.objects;
    assert_eq!(objects.made(), 0, "building the engine and a session made objects");
    let total = built.call(c, run, vec![]).map(|(v, _)| v);
    assert_eq!(al_testkit::int(total), 1);
    let objects = &built.session.engine.program.objects;
    assert!(objects.is_made(c));
    assert!(!objects.is_made(unused), "a call made an object it does not run");
    assert!(!objects.is_made(table), "a call made an object it does not run");
    assert!(objects.made() < objects.len());
}

/// Sorted debug lines of a map, so two maps compare whatever their hash order.
fn lines<K: std::fmt::Debug, V: std::fmt::Debug>(m: &std::collections::HashMap<K, V>) -> Vec<String> {
    let mut v: Vec<String> = m.iter().map(|e| format!("{e:?}")).collect();
    v.sort();
    v
}

#[test]
fn an_engine_built_from_heads_equals_one_built_from_objects() {
    let lowered = al_testkit::compile(SRC);
    let blocked = al_testkit::blocked(&lowered);
    let eager = Engine::new(lowered.program.clone(), blocked.clone());
    let from_heads = Engine::new(lazy(&lowered.program), blocked);
    assert_eq!(from_heads.program.objects.made(), 0);
    assert_eq!(eager.tables, from_heads.tables);
    assert_eq!(lines(&eager.events.subscribers), lines(&from_heads.events.subscribers));
    assert_eq!(lines(&eager.events.publishers), lines(&from_heads.events.publishers));
    assert_eq!(lines(&eager.events.missing), lines(&from_heads.events.missing));
    assert_eq!(lines(&eager.events.unraised), lines(&from_heads.events.unraised));
    assert_eq!(eager.events.global_key, from_heads.events.global_key);
    assert_eq!(eager.events.install_subscribers, from_heads.events.install_subscribers);
    assert_eq!(lines(&eager.unevaluable_init), lines(&from_heads.unevaluable_init));
    assert_eq!(eager.platform_unseeded, from_heads.platform_unseeded);
    for id in 50300..=50305 {
        assert_eq!(eager.codeunit_object(Some(id)), from_heads.codeunit_object(Some(id)));
    }
    assert_eq!(from_heads.program.objects.made(), 0);
}
