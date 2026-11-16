//! A stream write to the Blob of `xRec` in OnModify while `Rec` holds the same buffer. On BC the write does not show in
//! `Rec` (streams/BlobCopyStates.al:2419-2453, OnModify_Mode2_*: `Rec` reads `alpha` after `xx` went into `xRec`); the
//! runtime shares the buffer, so the write is refused. A write to `Rec` after a read of `xRec` keeps main's answer
//! (OnModify_Mode1_*, :2380-2417).

const SRC: &str = r#"
table 50985 "Bx Row"
{
    fields { field(1; "Entry No."; Integer) { } field(2; Mode; Integer) { } field(10; Content; Blob) { } }
    keys { key(PK; "Entry No.") { } }
    trigger OnModify()
    var OutS: OutStream; InS: InStream; Txt: Text;
    begin
        case Mode of
            1:
                begin
                    xRec.Content.CreateInStream(InS);
                    InS.ReadText(Txt);
                    Rec.Content.CreateOutStream(OutS);
                    OutS.WriteText('trig');
                end;
            2:
                begin
                    xRec.Content.CreateOutStream(OutS);
                    OutS.WriteText('xx');
                    Rec.Content.CreateInStream(InS);
                    InS.ReadText(Txt);
                end;
        end;
        Seen := Txt;
    end;
    procedure GetSeen(): Text begin exit(Seen); end;
    var Seen: Text;
}
codeunit 50986 C
{
    local procedure Run(M: Integer): Text
    var R: Record "Bx Row"; OutS: OutStream;
    begin
        R.Init(); R."Entry No." := 1;
        R.Content.CreateOutStream(OutS);
        OutS.WriteText('alpha');
        R.Insert();
        R.Mode := M;
        R.Modify(true);
        exit(R.GetSeen());
    end;

    procedure WriteXRec(): Text begin exit(Run(2)); end;
    procedure WriteRec(): Text begin exit(Run(1)); end;
}
"#;

#[test]
fn a_stream_write_to_the_xrec_blob_in_on_modify_is_refused() {
    match al_testkit::run_in(SRC, "C", "WriteXRec") {
        Err(al_builtins::native::Signal::Unsupported(why)) => assert!(why.contains("xRec"), "{why}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn a_read_of_xrec_then_a_write_to_rec_keeps_mains_answer() {
    assert_eq!(al_testkit::text(al_testkit::run_in(SRC, "C", "WriteRec")), "alpha");
}
