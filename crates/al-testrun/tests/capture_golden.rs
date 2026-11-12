//! Value capture and loop tracking (ADR-030) on the executable form: the values, loops, iterations, statements and
//! messages a capture records for programs that use every loop kind, `break`, `continue`, `exit`, `asserterror`, calls
//! with `var` arguments and `case`.
//!
//! `capture_golden.txt` was recorded from the tree walker, the oracle at the time it was retired (ADR-034 amendment,
//! 2026-10-16); the bytecode's captured form gave the same text. A change of it is a change of what a capture reports.

use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun};
use al_vm::CaptureConfig;

const PROGRAMS: &[&str] = &[
    // Every loop kind, break and continue, nested loops.
    "codeunit 50100 X {
        trigger OnRun() var I: Integer; J: Integer; S: Integer; T: Text; L: List of [Integer]; V: Integer;
        begin
            while I < 5 do begin I += 1; if I = 2 then continue; if I = 4 then break; S += I; end;
            repeat J += 1; S += J; until J >= 3;
            for I := 1 to 3 do for J := 3 downto 2 do S += I * J;
            L.Add(4); L.Add(5); L.Add(6);
            foreach V in L do begin if V = 5 then continue; T += Format(V); end;
            for I := 1 to 4 do begin if I = 3 then break; S += 1; end;
            Message('done %1', S);
        end;
    }",
    // exit inside a loop of a callee, var arguments, case, a recursive call.
    "codeunit 50101 X {
        procedure Find(Limit: Integer): Integer var K: Integer; begin for K := 1 to 100 do begin if K * K > Limit then exit(K); end; exit(0); end;
        procedure Bump(var N: Integer; By: Integer) begin N += By; end;
        procedure Fact(N: Integer): Integer begin if N <= 1 then exit(1); exit(N * Fact(N - 1)); end;
        trigger OnRun() var A: Integer; B: Integer; C: Text;
        begin
            A := Find(50);
            Bump(A, 3); Bump(A, 4);
            case A of 10..12: C := 'low'; 13: C := 'mid'; else C := 'high'; end;
            B := Fact(5);
            while A < 20 do begin Bump(A, 5); case A mod 2 of 0: B += 1; 1: B += 2; end; end;
        end;
    }",
    // asserterror inside loops, an error that leaves a loop and a statement.
    "codeunit 50102 X {
        procedure Boom(N: Integer) begin if N >= 1 then Error('boom %1', N); end;
        trigger OnRun() var I: Integer; Hits: Integer; J: Integer;
        begin
            for I := 1 to 3 do begin
                asserterror Boom(I);
                if GetLastErrorText() <> '' then Hits += 1;
                J := 0;
                while J < 2 do begin J += 1; asserterror begin if J >= 1 then Error('x'); end; end;
            end;
            asserterror begin for I := 1 to 5 do begin Hits += 1; if I = 2 then Error('stop'); end; end;
            Hits += 10;
        end;
    }",
    // A loop whose body breaks out of nested ifs, and continue in repeat, and a loop in a callee called from a loop.
    "codeunit 50103 X {
        procedure Sum(N: Integer): Integer var I: Integer; R: Integer; begin for I := 1 to N do R += I; exit(R); end;
        trigger OnRun() var I: Integer; T: Integer; K: Integer;
        begin
            for I := 1 to 4 do begin
                if I > 1 then begin if I = 3 then begin if T >= 0 then break; end; end;
                T += Sum(I);
            end;
            repeat K += 1; if K = 2 then continue; T += K; until K >= 4;
        end;
    }",
];

fn run(code: &str) -> String {
    let apps = vec![AppSource {
        id: "a".into(),
        name: "a".into(),
        dependencies: vec![],
        files: vec![("a.al".into(), code.into())],
    }];
    let rt = CorpusRuntime::try_run_suites_with(
        apps,
        vec![al_sema::AppKind::Workspace],
        &[],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun {
            name: String::new(),
            app: Some(0),
            runner: None,
            driver: None,
        }],
        &RunOptions {
            execute: true,
            capture: CaptureConfig {
                values: true,
                loops: true,
            },
            ..Default::default()
        },
    )
    .unwrap();
    let r = &rt.results()[0];
    format!("{:?}\n{:#?}", r.outcome, r.captured.as_ref().map(|c| &c.captured))
}

#[test]
fn the_capture_of_the_programs_is_the_recorded_one() {
    let mut got = String::new();
    for p in PROGRAMS {
        got.push_str(&run(p));
        got.push('\n');
    }
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/capture_golden.txt");
    if std::env::var_os("UPDATE_CAPTURE_GOLDEN").is_some() {
        std::fs::write(&path, &got).unwrap();
    }
    let want = std::fs::read_to_string(&path).unwrap();
    assert_eq!(
        got, want,
        "the capture changed; if the change is meant: UPDATE_CAPTURE_GOLDEN=1 cargo test -p al-testrun --test capture_golden"
    );
}
