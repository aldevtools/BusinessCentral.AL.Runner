//! `CopyArray(Target, P.Get(), Pos)`: the source is the array a procedure returns (al-runner issue 6658). Only the destination is
//! written; the copy itself is pinned by misc/lt-system-functions/Arrays.al:CopyArray_Forms.

use al_testkit::{run_in, text};

const SRC: &str = r#"
interface IGet { procedure Get(): array[3] of Integer; }
codeunit 50100 Provider implements IGet {
    var Guids: array[2] of Guid; Ints: array[3] of Integer;
    procedure Fill() begin Evaluate(Guids[1], '{11111111-1111-1111-1111-111111111111}'); Evaluate(Guids[2], '{22222222-2222-2222-2222-222222222222}'); Ints[1] := 1; Ints[2] := 2; Ints[3] := 3; end;
    procedure GetGuids(): array[2] of Guid begin exit(Guids); end;
    procedure Named() Res: array[3] of Integer begin Res := Ints; end;
    procedure Get(): array[3] of Integer begin exit(Ints); end;
    procedure Texts() Res: array[2] of Text[10] begin Res[1] := 'a'; Res[2] := 'b'; end;
}
codeunit 50101 Consumer {
    local procedure LocalArr(): array[3] of Integer var R: array[3] of Integer; begin R[1] := 7; R[2] := 8; R[3] := 9; exit(R); end;
    procedure Guids(): Text
    var P: Codeunit Provider; T: array[2] of Guid;
    begin
        P.Fill(); CopyArray(T, P.GetGuids(), 1);
        exit(Format(T[1]) + Format(T[2]));
    end;
    procedure Shapes(): Text
    var P: Codeunit Provider; I: Interface IGet; T: array[2] of Integer; T3: array[3] of Integer; X: array[2] of Text[10];
    begin
        P.Fill(); I := P;
        CopyArray(T, P.Named(), 2); CopyArray(T3, LocalArr(), 1);
        exit(Format(T[1]) + Format(T[2]) + '|' + Format(T3[1]) + Format(T3[3]) + '|' + GetIface(I) + '|' + GetTexts(P, X));
    end;
    local procedure GetIface(I: Interface IGet): Text var T: array[1] of Integer; begin CopyArray(T, I.Get(), 3); exit(Format(T[1])); end;
    local procedure GetTexts(P: Codeunit Provider; var X: array[2] of Text[10]): Text begin CopyArray(X, P.Texts(), 1); exit(X[1] + X[2]); end;
    procedure Untouched(): Text
    var P: Codeunit Provider; T: array[3] of Integer;
    begin
        P.Fill(); CopyArray(T, P.Get(), 1); T[1] := 5;
        CopyArray(T, P.Get(), 1);
        exit(Format(T[1]));
    end;
}
"#;

#[test]
fn copyarray_from_a_returned_guid_array() {
    assert_eq!(
        text(run_in(SRC, "Consumer", "Guids")),
        "{11111111-1111-1111-1111-111111111111}{22222222-2222-2222-2222-222222222222}"
    );
}

#[test]
fn copyarray_from_named_return_local_interface_and_text_arrays() {
    assert_eq!(text(run_in(SRC, "Consumer", "Shapes")), "23|79|3|ab");
}

#[test]
fn copyarray_leaves_the_source_unchanged() {
    assert_eq!(text(run_in(SRC, "Consumer", "Untouched")), "1");
}

const MORE: &str = r#"
codeunit 50100 Provider {
    procedure Get(): array[3] of Integer var R: array[3] of Integer; begin R[1] := 4; R[2] := 5; R[3] := 6; exit(R); end;
}
codeunit 50101 Consumer {
    procedure Reads(): Text
    var P: Codeunit Provider;
    begin
        exit(Format(ArrayLen(P.Get())));
    end;
    procedure IntoResult()
    var P: Codeunit Provider; T: array[3] of Integer;
    begin
        CopyArray(P.Get(), T, 1);
    end;
}
"#;

#[test]
fn arraylen_of_a_returned_array() {
    assert_eq!(text(run_in(MORE, "Consumer", "Reads")), "3");
}

#[test]
fn copyarray_into_a_returned_array_is_refused() {
    match run_in(MORE, "Consumer", "IntoResult") {
        Err(al_builtins::native::Signal::Unsupported(r)) => assert!(r.contains("not pinned"), "{r}"),
        other => panic!("{other:?}"),
    }
}
