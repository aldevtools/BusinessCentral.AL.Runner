//! Test helpers shared by the tests of the workspace (ADR-037 item 5).
//!
//! A test of the interpreter wants one thing: AL source in, the value of a routine out. Before this crate every test
//! file carried its own copy of the compile-and-call sequence (39 copies, each a little different). They are here once:
//!
//! * [`compile`] and friends turn source into a [`Lowered`] program, failing the test on a diagnostic;
//! * [`run`], [`run_in`], [`run_with`] compile and call a routine, and give the `Result` of the call;
//! * [`Built`] keeps the compiled program for a test that calls several routines or wants the [`Session`] itself;
//! * [`text`], [`int`], [`boolean`], [`error_message`] read a result and fail the test with the value when it is not
//!   the kind expected.
//!
//! The crate is a dev-dependency only. It holds no behaviour of its own: it calls the compiler and the interpreter the
//! way the tests did.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_sema::lower::Lowered;
pub use al_sema::workspace::AppSource;
use al_vm::{Engine, Session};
use std::sync::Arc;

/// One app named "a" with one file "f.al".
pub fn app(src: &str) -> AppSource {
    app_named("a", src)
}

/// One app with the given name (its id is "a") and one file "f.al".
pub fn app_named(name: &str, src: &str) -> AppSource {
    AppSource {
        id: "a".into(),
        name: name.into(),
        dependencies: vec![],
        files: vec![("f.al".into(), src.into())],
    }
}

/// Compiles apps. A diagnostic of the workspace (a parse error, an unresolved reference) fails the test.
pub fn compile_apps(apps: Vec<AppSource>) -> Lowered {
    let (ws, lowered) = al_sema::lower::compile(apps, &Default::default());
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    lowered
}

/// Compiles one app of one file.
pub fn compile(src: &str) -> Lowered {
    compile_apps(vec![app(src)])
}

/// Why each routine cannot run (`None`: it can), the shape the engine takes.
pub fn blocked(l: &Lowered) -> Vec<Vec<Option<String>>> {
    l.unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect()
}

/// A compiled program and a session on it.
pub struct Built {
    pub lowered: Lowered,
    pub session: Session,
}

impl Built {
    /// A session on `lowered`.
    pub fn new(lowered: Lowered) -> Built {
        let engine = Engine::new(lowered.program.clone(), blocked(&lowered));
        Built {
            session: Session::new(engine),
            lowered,
        }
    }

    /// Compiles `src` and opens a session.
    pub fn from_source(src: &str) -> Built {
        Built::new(compile(src))
    }

    /// The object named `name` and its routine `routine`, as indexes.
    pub fn routine_in(&self, name: &str, routine: &str) -> (usize, usize) {
        let oi = self
            .lowered
            .program
            .objects
            .iter()
            .position(|o| o.name == name)
            .unwrap_or_else(|| panic!("no object named {name}"));
        (oi, self.routine_of(oi, routine))
    }

    /// The first object that has a routine named `routine`, and the routine.
    pub fn routine(&self, routine: &str) -> (usize, usize) {
        let oi = self
            .lowered
            .program
            .objects
            .iter()
            .position(|o| o.routines.iter().any(|r| r.name == routine))
            .unwrap_or_else(|| panic!("no object has a routine named {routine}"));
        (oi, self.routine_of(oi, routine))
    }

    fn routine_of(&self, oi: usize, routine: &str) -> usize {
        self.lowered.program.objects[oi]
            .routines
            .iter()
            .position(|r| r.name == routine)
            .unwrap_or_else(|| {
                panic!(
                    "object {} has no routine {routine}",
                    self.lowered.program.objects[oi].name
                )
            })
    }

    /// Calls routine `ri` of object `oi` on a fresh instance with `args`.
    pub fn call(&mut self, oi: usize, ri: usize, args: Vec<Value>) -> Result<(Value, Vec<Value>), Signal> {
        let inst = self.session.instance(oi);
        self.session.call(oi, ri, inst, args)
    }

    /// Calls the first routine named `routine`, without arguments.
    pub fn run(&mut self, routine: &str) -> Result<Value, Signal> {
        let (oi, ri) = self.routine(routine);
        self.call(oi, ri, vec![]).map(|(v, _)| v)
    }

    /// Calls routine `routine` of the object named `object`, without arguments.
    pub fn run_in(&mut self, object: &str, routine: &str) -> Result<Value, Signal> {
        let (oi, ri) = self.routine_in(object, routine);
        self.call(oi, ri, vec![]).map(|(v, _)| v)
    }
}

/// Compiles `src` and calls the first routine named `routine` (no arguments).
pub fn run(src: &str, routine: &str) -> Result<Value, Signal> {
    Built::from_source(src).run(routine)
}

/// Compiles `src` and calls `routine` of the object named `object` (no arguments).
pub fn run_in(src: &str, object: &str, routine: &str) -> Result<Value, Signal> {
    Built::from_source(src).run_in(object, routine)
}

/// Compiles `src` and calls `routine` of `object` with `args`; the result and the final values of the parameters.
pub fn run_with(src: &str, object: &str, routine: &str, args: Vec<Value>) -> Result<(Value, Vec<Value>), Signal> {
    let mut b = Built::from_source(src);
    let (oi, ri) = b.routine_in(object, routine);
    b.call(oi, ri, args)
}

/// With the `runner` feature: compiles `src` and runs the tests of the codeunit named `codeunit` the way `al-runner test` does
/// (one session per codeunit, the install code first), giving each test's name and outcome.
#[cfg(feature = "runner")]
pub fn outcomes_in(src: &str, codeunit: &str) -> Vec<(String, al_testrun::Outcome)> {
    outcomes_in_app(app(src), codeunit)
}

/// [`outcomes_in`] for an app the caller describes (for example one with a GUID id, whose module is readable).
#[cfg(feature = "runner")]
pub fn outcomes_in_app(app: AppSource, codeunit: &str) -> Vec<(String, al_testrun::Outcome)> {
    let l = compile_apps(vec![app]);
    let engine = Engine::new(l.program.clone(), blocked(&l));
    let oi = l
        .program
        .objects
        .iter()
        .position(|o| o.name == codeunit)
        .unwrap_or_else(|| panic!("no object named {codeunit}"));
    al_testrun::runtime::run_codeunit(&engine, &l, oi)
}

/// A session on a compiled program (the program moves into its engine).
pub fn session(l: Lowered) -> Session {
    Session::new(Engine::new(l.program.clone(), blocked(&l)))
}

/// The engine of a compiled program, for a test that builds its own sessions.
pub fn engine(l: &Lowered) -> Arc<Engine> {
    Engine::new(l.program.clone(), blocked(l))
}

/// The text of a result, or the test fails with what it was.
pub fn text(r: Result<Value, Signal>) -> String {
    match r {
        Ok(Value::Text(s)) | Ok(Value::Code(s)) => s.to_string(),
        other => panic!("{other:?}"),
    }
}

/// The Integer of a result, or the test fails with what it was.
pub fn int(r: Result<Value, Signal>) -> i32 {
    match r {
        Ok(Value::Integer(i)) => i,
        other => panic!("{other:?}"),
    }
}

/// The Boolean of a result, or the test fails with what it was.
pub fn boolean(r: Result<Value, Signal>) -> bool {
    match r {
        Ok(Value::Boolean(b)) => b,
        other => panic!("{other:?}"),
    }
}

/// The message of the AL error a call raised, or the test fails with what it was.
pub fn error_message(r: Result<Value, Signal>) -> String {
    match r {
        Err(Signal::Error(e)) => e.message.clone(),
        other => panic!("expected an error, got {other:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &str = "codeunit 50100 C { procedure F(): Integer begin exit(42); end;
        procedure G(a: Integer): Integer begin exit(a + 1); end;
        procedure T(): Text begin exit('hi'); end;
        procedure B(): Boolean begin exit(true); end;
        procedure E() begin Error('boom %1', 3); end; }
        codeunit 50101 D { procedure F(): Integer begin exit(7); end; }";

    #[test]
    fn a_routine_is_found_by_name_in_the_first_object_that_has_it_or_in_the_named_object() {
        assert_eq!(int(run(SRC, "F")), 42);
        assert_eq!(int(run_in(SRC, "D", "F")), 7);
        assert_eq!(int(run_in(SRC, "C", "F")), 42);
        assert_eq!(text(run(SRC, "T")), "hi");
        assert!(boolean(run(SRC, "B")));
        assert_eq!(error_message(run(SRC, "E")), "boom 3");
    }

    #[test]
    fn arguments_go_in_and_the_final_values_come_back() {
        let (v, finals) = run_with(SRC, "C", "G", vec![Value::Integer(9)]).unwrap();
        assert!(matches!(v, Value::Integer(10)));
        assert_eq!(finals.len(), 1);
    }

    #[test]
    fn a_built_program_runs_several_routines_on_one_session() {
        let mut b = Built::from_source(SRC);
        assert_eq!(int(b.run("F")), 42);
        assert_eq!(text(b.run("T")), "hi");
    }

    #[test]
    #[should_panic(expected = "no object named Nope")]
    fn an_object_that_is_not_there_fails_the_test_with_its_name() {
        run_in(SRC, "Nope", "F").ok();
    }
}
