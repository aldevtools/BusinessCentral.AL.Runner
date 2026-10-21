//! The debugger seam of ADR-048: a session with a `DebugHook` sees each statement before it runs, with the call
//! depth and the locals of every frame, and computes what it computes without one.

use std::sync::{Arc, Mutex};

use al_testkit::Built;
use al_vm::{DebugHook, DebugView, Rendered, Resume};

const SRC: &str = r#"
codeunit 50100 Stepping
{
    procedure Outer(): Integer
    var
        Result: Integer;
    begin
        Result := 1;
        Result := Twice(Result);
        Result := Result + 10;
        exit(Result);
    end;

    local procedure Twice(X: Integer): Integer
    var
        Y: Integer;
    begin
        Y := X * 2;
        exit(Y);
    end;
}
"#;

/// One statement the hook saw: its line, the call depth, and per frame (innermost first) the routine and its locals.
type Seen = (u32, usize, Vec<(String, Vec<(String, Rendered)>)>);

struct Recorder(Arc<Mutex<Vec<Seen>>>);

impl DebugHook for Recorder {
    fn statement(&mut self, view: &DebugView<'_>) -> Resume {
        let program = view.program();
        let line = program.pos(view.pos()).map_or(0, |p| p.line);
        let frames = view
            .frames()
            .into_iter()
            .map(|f| (program.objects[f.object].routines[f.routine].name.clone(), f.locals))
            .collect();
        self.0.lock().unwrap().push((line, view.depth(), frames));
        Resume::Continue
    }
}

fn local<'a>(frame: &'a (String, Vec<(String, Rendered)>), name: &str) -> &'a Rendered {
    &frame
        .1
        .iter()
        .find(|(n, _)| n == name)
        .unwrap_or_else(|| panic!("no {name} in {frame:?}"))
        .1
}

#[test]
fn the_hook_sees_every_statement_with_its_depth_and_the_locals_of_each_frame() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let mut b = Built::from_source(SRC);
    b.session.attach_debugger(Box::new(Recorder(seen.clone())));
    assert_eq!(al_testkit::int(b.run_in("Stepping", "Outer")), 12);
    let seen = seen.lock().unwrap();
    let lines: Vec<(u32, usize)> = seen.iter().map(|(l, d, _)| (*l, *d)).collect();
    // Lines of SRC (1-based, the first line is empty): Outer's four statements, Twice's two at depth 2 in between.
    assert_eq!(lines, vec![(8, 1), (9, 1), (18, 2), (19, 2), (10, 1), (11, 1)]);
    // Before `Result := Twice(Result)` runs, Result is 1.
    assert_eq!(local(&seen[1].2[0], "Result"), &Rendered::Int(1));
    // Inside Twice: X is 1 and Y is still 0; the caller's frame below shows Result as 1.
    let (_, _, frames) = &seen[2];
    assert_eq!(frames.len(), 2);
    assert_eq!(frames[0].0, "Twice");
    assert_eq!(local(&frames[0], "X"), &Rendered::Int(1));
    assert_eq!(local(&frames[0], "Y"), &Rendered::Int(0));
    assert_eq!(frames[1].0, "Outer");
    assert_eq!(local(&frames[1], "Result"), &Rendered::Int(1));
    // Before `exit(Y)`, Y is 2; back in Outer, Result is 2.
    assert_eq!(local(&seen[3].2[0], "Y"), &Rendered::Int(2));
    assert_eq!(local(&seen[4].2[0], "Result"), &Rendered::Int(2));
}

#[test]
fn a_detached_debugger_sees_nothing_and_the_result_is_the_same() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let mut b = Built::from_source(SRC);
    b.session.attach_debugger(Box::new(Recorder(seen.clone())));
    b.session.detach_debugger();
    assert_eq!(al_testkit::int(b.run_in("Stepping", "Outer")), 12);
    assert!(seen.lock().unwrap().is_empty());
}
