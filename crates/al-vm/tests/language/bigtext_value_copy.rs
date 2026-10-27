//! BigText is a value: `B := A` copies it, a by-value parameter gets a copy, and Clear of a copy leaves the original
//! (al-runner issue 5940, the reporter's repro on real BC; registered assumption `bigtext-value-copy`).

use al_builtins::assumptions::{Assumption, clear_hits, take_hits};

#[test]
fn assignment_and_by_value_parameter_copy_a_bigtext() {
    let src = "codeunit 50100 M { procedure AddTo(Value: BigText) begin Value.AddText('c'); end;
                   procedure AddToVar(var Value: BigText) begin Value.AddText('d'); end; }
        codeunit 50101 C { procedure F(): Text var M: Codeunit M; A: BigText; B: BigText; C: BigText; D: BigText; begin
            A.AddText('a'); B := A; A.AddText('b'); M.AddTo(A); C := A; Clear(C);
            D := A; M.AddToVar(D);
            exit(StrSubstNo('A=%1;B=%2;C=%3;D=%4', A.Length(), B.Length(), C.Length(), D.Length())); end; }";
    clear_hits();
    assert_eq!(al_testkit::text(al_testkit::run_in(src, "C", "F")), "A=2;B=1;C=0;D=3");
    assert!(take_hits().contains(&Assumption::BigTextValueCopy));
}
