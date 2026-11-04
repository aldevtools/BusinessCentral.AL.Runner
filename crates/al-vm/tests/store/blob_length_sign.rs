//! `Blob.Length` before CalcFields of a stored blob whose stored size is not pinned (al-runner issue 6896): a comparison
//! with the literal 0 is answered from the stored content being non-empty; every other use of the number stays refused.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::{run_in, text};

const SRC: &str = r#"
table 69972 "BLS Plain" { fields { field(1; Id; Integer) { } field(2; Content; Blob) { Compressed = false; } } keys { key(PK; Id) { } } }
table 69970 "BLS Row" { fields { field(1; Id; Integer) { } field(2; Content; Blob) { } } keys { key(PK; Id) { } } }
codeunit 69971 C
{
    local procedure Store(Id: Integer; N: Integer)
    var Row: Record "BLS Row"; OutS: OutStream;
    begin
        Row.Id := Id;
        if N > 0 then begin
            Row.Content.CreateOutStream(OutS);
            OutS.WriteText(PadStr('', N, 'x'));
        end;
        Row.Insert();
    end;

    local procedure B(Value: Boolean): Text
    begin
        if Value then exit('1');
        exit('0');
    end;

    procedure PlainSize(N: Integer): Integer
    var Row: Record "BLS Plain"; Got: Record "BLS Plain"; OutS: OutStream;
    begin
        Row.Id := 1;
        Row.Content.CreateOutStream(OutS);
        OutS.WriteText(PadStr('', N, 'x'));
        Row.Insert();
        Got.Get(1);
        exit(Got.Content.Length);
    end;

    procedure Comparisons(N: Integer): Text
    var Row: Record "BLS Row"; R: Text;
    begin
        Store(1, N);
        Row.Get(1);
        R := B(Row.Content.Length = 0) + B(Row.Content.Length <> 0) + B(Row.Content.Length > 0) + B(0 < Row.Content.Length);
        R += B(0 = Row.Content.Length) + B(0 > Row.Content.Length) + B(Row.Content.Length <= 0) + B(0 <= Row.Content.Length);
        R += B(Row.Content.Length >= 0) + B(0 <> Row.Content.Length) + B(Row.Content.Length < 0) + B(0 >= Row.Content.Length);
        if Row.Content.Length = 0 then R += 'z' else R += 'n';
        exit(R);
    end;

    procedure ComparisonsFindSet(N: Integer): Text
    var Row: Record "BLS Row";
    begin
        Store(1, N);
        Row.FindSet();
        exit(B(Row.Content.Length = 0) + B(Row.Content.Length > 0));
    end;

    procedure Shown(): Text
    var Row: Record "BLS Row";
    begin
        Store(1, 600);
        Row.Get(1);
        exit(Format(Row.Content.Length));
    end;

    procedure Assigned(): Integer
    var Row: Record "BLS Row"; I: Integer;
    begin
        Store(1, 600);
        Row.Get(1);
        I := Row.Content.Length;
        exit(I);
    end;

    procedure ComparedWithOne(): Boolean
    var Row: Record "BLS Row";
    begin
        Store(1, 600);
        Row.Get(1);
        exit(Row.Content.Length = 1);
    end;

    procedure Arithmetic(): Boolean
    var Row: Record "BLS Row";
    begin
        Store(1, 600);
        Row.Get(1);
        exit(Row.Content.Length - 0 = 0);
    end;

    procedure ComparedWithVariable(): Boolean
    var Row: Record "BLS Row"; Zero: Integer;
    begin
        Store(1, 600);
        Row.Get(1);
        exit(Row.Content.Length = Zero);
    end;
}
"#;

fn run_n(routine: &str, n: i32) -> Result<Value, Signal> {
    al_testkit::run_with(SRC, "C", routine, vec![Value::Integer(n)]).map(|(v, _)| v)
}

fn assert_refused(r: Result<Value, Signal>) {
    match r {
        Err(Signal::Unsupported(reason)) => assert!(reason.contains("stored size is not pinned"), "{reason}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn comparisons_with_zero_of_an_unpinned_stored_size_are_answered() {
    // 600 bytes: not among the pinned stored sizes. Non-empty: = 0 No, <> 0 Yes, > 0 Yes, 0 < Yes, 0 = No, 0 > No, <= 0 No,
    // 0 <= Yes, >= 0 Yes, 0 <> Yes, < 0 No, 0 >= No, then the `if` form.
    assert_eq!(text(run_n("Comparisons", 600)), "011100011100n");
    assert_eq!(text(run_n("Comparisons", 1000)), "011100011100n");
    assert_eq!(text(run_n("ComparisonsFindSet", 4000)), "01");
}

#[test]
fn comparisons_with_zero_of_a_pinned_or_empty_blob_are_unchanged() {
    assert_eq!(text(run_n("Comparisons", 0)), "100010111001z");
    assert_eq!(text(run_n("Comparisons", 5)), "011100011100n");
}

#[test]
fn every_other_use_of_an_unpinned_stored_size_stays_refused() {
    assert_refused(run_in(SRC, "C", "Shown"));
    assert_refused(run_in(SRC, "C", "Assigned"));
    assert_refused(run_in(SRC, "C", "ComparedWithOne"));
    assert_refused(run_in(SRC, "C", "Arithmetic"));
    assert_refused(run_in(SRC, "C", "ComparedWithVariable"));
}

#[test]
fn a_field_without_compression_stores_the_content_size() {
    // record/flowfields/BlobStoredSizeSign.al:224-356, the `U-` observations: the content size, for every size.
    for n in [1, 2, 5, 600, 1000, 4000, 100_000] {
        assert_eq!(al_testkit::int(run_n("PlainSize", n)), n, "{n}");
    }
}
