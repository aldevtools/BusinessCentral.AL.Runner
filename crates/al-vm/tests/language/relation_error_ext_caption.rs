//! A table whose own Caption one tableextension overrides is named by the extension's Caption in every text that names the
//! table, while its name stays (al-runner issues 5955 and 6378; measured on real BC by corpus writer Y5, branch
//! al-runtime-rs-staging-y5, tableextension/lt-y5/Y5TestFieldExtCaption.al, codeunit 90986, on Base Application's Machine
//! Center): the TableRelation error of a Validate, TestField (equal, blank and value forms), FieldError with and without a
//! text, the Get error of a missing row, TableCaption and RecordRef.Caption. TableName and RecordRef.Name give the name.

use al_builtins::assumptions::{clear_hits, take_hits};

const SRC: &str = "
table 50501 WC { Caption = 'Work Center'; fields { field(1; No; Code[20]) { Caption = 'No.'; } field(2; Blocked; Boolean) { }
    field(3; Name; Text[50]) { } } keys { key(PK; No) { } } }
tableextension 50500 WCExt extends WC { Caption = 'Repro Work Center Extension'; }
table 50500 Ref { fields { field(1; Entry; Integer) { } field(2; WcNo; Code[20]) { Caption = 'Work Center No.'; TableRelation = WC; } }
    keys { key(PK; Entry) { } } }
codeunit 50502 C {
    local procedure Seed(var W: Record WC) begin W.No := 'REPRO'; W.Blocked := true; W.Insert(); end;
    procedure F(): Text var R: Record Ref; begin asserterror R.Validate(WcNo, 'NOWC'); exit(GetLastErrorText()); end;
    procedure G(): Text var W: Record WC; begin exit(W.TableCaption()); end;
    procedure TF(): Text var W: Record WC; begin Seed(W); asserterror W.TestField(Blocked, false); exit(GetLastErrorText() + '|' + GetLastErrorCode()); end;
    procedure TFBlank(): Text var W: Record WC; begin Seed(W); asserterror W.TestField(Name); exit(GetLastErrorText()); end;
    procedure TFValue(): Text var W: Record WC; begin Seed(W); asserterror W.TestField(No, 'OTHER'); exit(GetLastErrorText()); end;
    procedure FE(): Text var W: Record WC; begin Seed(W); asserterror W.FieldError(Blocked); exit(GetLastErrorText()); end;
    procedure FEText(): Text var W: Record WC; begin Seed(W); asserterror W.FieldError(Blocked, 'is not allowed'); exit(GetLastErrorText()); end;
    procedure GetMissing(): Text var W: Record WC; begin asserterror W.Get('NOWC'); exit(GetLastErrorText() + '|' + GetLastErrorCode()); end;
    procedure Names(): Text var W: Record WC; RR: RecordRef; begin RR.Open(Database::WC); exit(W.TableName() + '|' + RR.Caption() + '|' + RR.Name()); end; }";

fn text(f: &str) -> String {
    al_testkit::text(al_testkit::run_in(SRC, "C", f))
}

#[test]
fn every_text_that_names_the_table_uses_the_extension_caption() {
    clear_hits();
    assert_eq!(
        text("F"),
        "The field Work Center No. of table Ref contains a value (NOWC) that cannot be found in the related table (Repro Work Center Extension)."
    );
    assert_eq!(text("G"), "Repro Work Center Extension");
    assert_eq!(
        text("TF"),
        "Blocked must be equal to 'No'  in Repro Work Center Extension: No.=REPRO. Current value is 'Yes'.|TestField"
    );
    assert_eq!(
        text("TFBlank"),
        "Name must have a value in Repro Work Center Extension: No.=REPRO. It cannot be zero or empty."
    );
    assert_eq!(
        text("TFValue"),
        "No. must be equal to 'OTHER'  in Repro Work Center Extension: No.=REPRO. Current value is 'REPRO'."
    );
    assert_eq!(
        text("FE"),
        "Blocked must not be Yes in Repro Work Center Extension No.='REPRO'."
    );
    assert_eq!(
        text("FEText"),
        "Blocked is not allowed in Repro Work Center Extension No.='REPRO'."
    );
    assert_eq!(
        text("GetMissing"),
        "The Repro Work Center Extension does not exist. Identification fields and values: No.='NOWC'|DB:RecordNotFound"
    );
    assert_eq!(text("Names"), "WC|Repro Work Center Extension|WC");
    assert!(take_hits().is_empty(), "measured, not assumed");
}
