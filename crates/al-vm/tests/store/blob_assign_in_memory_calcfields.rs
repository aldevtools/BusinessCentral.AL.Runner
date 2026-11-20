//! `Src.Data := Other.Data` where `Other` holds content a stream wrote in memory, then `CalcFields` on `Src` (a database
//! record): BC reads the row back. With no row (never inserted) the Blob is empty; over a stored row it shows the row's
//! content (streams/BlobCopyNeighbors.al:3102-3150 and :3280-3330, CalcFields_AfterAssign_ByValueAndCopy). A stream write
//! into `Src` itself keeps its content (main's answer).

const SRC: &str = r#"
table 50987 "Bx Row"
{
    fields { field(1; Key1; Integer) { } field(2; Data; Blob) { } }
    keys { key(PK; Key1) { } }
}
codeunit 50988 C
{
    local procedure W(var R: Record "Bx Row"; T: Text)
    var OutS: OutStream;
    begin
        R.Data.CreateOutStream(OutS);
        OutS.WriteText(T);
    end;

    local procedure Rd(var R: Record "Bx Row"): Text
    var InS: InStream; T: Text;
    begin
        if not R.Data.HasValue() then exit('nohv:');
        R.Data.CreateInStream(InS);
        InS.ReadText(T);
        exit('hv:' + T);
    end;

    procedure NeverInserted(): Text
    var Src: Record "Bx Row"; Other: Record "Bx Row"; Before: Text;
    begin
        Src.Key1 := 10;
        W(Other, 'beta');
        Src.Data := Other.Data;
        Before := Rd(Src);
        Src.CalcFields(Data);
        exit(Before + '|' + Rd(Src));
    end;

    procedure OverStored(): Text
    var Src: Record "Bx Row"; Other: Record "Bx Row"; Before: Text;
    begin
        Src.Key1 := 7;
        W(Src, 'alpha');
        Src.Insert();
        W(Other, 'beta');
        Src.Data := Other.Data;
        Before := Rd(Src);
        Src.CalcFields(Data);
        exit(Before + '|' + Rd(Src));
    end;

    procedure OwnWrite(): Text
    var Src: Record "Bx Row";
    begin
        Src.Key1 := 11;
        W(Src, 'gamma');
        Src.CalcFields(Data);
        exit(Rd(Src));
    end;
}
"#;

#[test]
fn an_assigned_blob_of_a_record_never_inserted_reads_empty_after_calcfields() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "NeverInserted")),
        "hv:beta|nohv:"
    );
}

#[test]
fn an_assigned_blob_over_a_stored_row_reads_the_row_after_calcfields() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "OverStored")),
        "hv:beta|hv:alpha"
    );
}

#[test]
fn an_own_stream_write_keeps_its_content_after_calcfields() {
    assert_eq!(al_testkit::text(al_testkit::run_in(SRC, "C", "OwnWrite")), "hv:gamma");
}

/// The shapes around the pin that no cell measures (al-runner PR 8149 review, probe p1 N05-N08, N10, N13, N14, N17, N18, N22):
/// CalcFields of
/// the record never inserted refuses. The pin assigns another record's in-memory write to the fresh Blob of a record with no
/// write, and calls CalcFields on that record, a by-value copy or a `Record.Copy` (BlobCopyNeighbors.al:3102-3134).
const NEIGHBORS: &str = r#"
table 50989 "By Row"
{
    fields { field(1; Key1; Integer) { } field(2; Data; Blob) { } field(3; Data2; Blob) { } }
    keys { key(PK; Key1) { } }
}
codeunit 50990 C
{
    local procedure W(var R: Record "By Row"; T: Text)
    var OutS: OutStream;
    begin
        R.Data.CreateOutStream(OutS);
        OutS.WriteText(T);
    end;

    local procedure W2(var R: Record "By Row"; T: Text)
    var OutS: OutStream;
    begin
        R.Data2.CreateOutStream(OutS);
        OutS.WriteText(T);
    end;

    procedure Chained()
    var Src: Record "By Row"; Other: Record "By Row"; Third: Record "By Row";
    begin
        Src.Key1 := 10;
        Third.Key1 := 11;
        W(Other, 'beta');
        Src.Data := Other.Data;
        Third.Data := Src.Data;
        Third.CalcFields(Data);
    end;

    procedure TargetHadOwnWrite()
    var Src: Record "By Row"; Other: Record "By Row";
    begin
        Src.Key1 := 10;
        W(Src, 'alpha');
        W(Other, 'beta');
        Src.Data := Other.Data;
        Src.CalcFields(Data);
    end;

    procedure SameRecordOtherField()
    var Src: Record "By Row";
    begin
        Src.Key1 := 10;
        W2(Src, 'beta');
        Src.Data := Src.Data2;
        Src.CalcFields(Data);
    end;

    procedure TransferFieldsThenCalc()
    var Src: Record "By Row"; Other: Record "By Row"; Dst: Record "By Row";
    begin
        Src.Key1 := 10;
        W(Other, 'beta');
        Src.Data := Other.Data;
        Dst.TransferFields(Src);
        Dst.CalcFields(Data);
    end;

    procedure ThroughRecordRef()
    var Src: Record "By Row"; Other: Record "By Row"; RR: RecordRef; FR: FieldRef;
    begin
        Src.Key1 := 10;
        W(Other, 'beta');
        Src.Data := Other.Data;
        RR.GetTable(Src);
        FR := RR.Field(2);
        FR.CalcField();
        RR.SetTable(Src);
    end;

    procedure FailedModifyBetween()
    var Src: Record "By Row"; Other: Record "By Row";
    begin
        Src.Key1 := 10;
        W(Other, 'beta');
        Src.Data := Other.Data;
        if Src.Modify() then;
        Src.CalcFields(Data);
    end;

    local procedure CalcIn(R: Record "By Row")
    begin
        R.CalcFields(Data);
    end;

    procedure TempTargetByValue()
    var Src: Record "By Row" temporary; Other: Record "By Row";
    begin
        Src.Key1 := 10;
        W(Other, 'beta');
        Src.Data := Other.Data;
        CalcIn(Src);
    end;

    procedure SourceFilledByCopyStream()
    var Src: Record "By Row"; Other: Record "By Row"; Feed: Record "By Row"; InS: InStream; OutS: OutStream;
    begin
        Src.Key1 := 10;
        W(Feed, 'beta');
        Feed.Data.CreateInStream(InS);
        Other.Data.CreateOutStream(OutS);
        CopyStream(OutS, InS);
        Src.Data := Other.Data;
        Src.CalcFields(Data);
    end;

    procedure FilterOnTarget()
    var Src: Record "By Row"; Other: Record "By Row";
    begin
        Src.Key1 := 10;
        Src.SetRange(Key1, 5);
        W(Other, 'beta');
        Src.Data := Other.Data;
        Src.CalcFields(Data);
    end;

    procedure AssignedTwiceSecondEmpty()
    var Src: Record "By Row"; Other: Record "By Row"; Empty: Record "By Row";
    begin
        Src.Key1 := 10;
        W(Other, 'beta');
        Src.Data := Other.Data;
        Src.Data := Empty.Data;
        Src.Data := Other.Data;
        Src.CalcFields(Data);
    end;
}
"#;

#[test]
fn calcfields_after_an_assignment_outside_the_pin_is_refused() {
    for routine in [
        "Chained",
        "TargetHadOwnWrite",
        "SameRecordOtherField",
        "TransferFieldsThenCalc",
        "ThroughRecordRef",
        "FailedModifyBetween",
        "AssignedTwiceSecondEmpty",
        "TempTargetByValue",
        "SourceFilledByCopyStream",
        "FilterOnTarget",
    ] {
        match al_testkit::run_in(NEIGHBORS, "C", routine) {
            Err(al_builtins::native::Signal::Unsupported(why)) => {
                assert!(why.contains("assigned"), "{routine}: {why}")
            }
            other => panic!("{routine}: expected a refusal, got {other:?}"),
        }
    }
}
