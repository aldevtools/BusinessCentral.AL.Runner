//! A page whose OnInsertRecord inserts the row itself and answers false keeps editing that row as a stored one: the next
//! edit goes through OnModifyRecord and lands in the table (Base Application's Item Tracking Lines does this on a temporary
//! source; assumption page-oninsertrecord-false-after-code-insert; Tests-Manufacturing PostProductionJournalUsingLotItemTracking).
//! A row nobody inserted stays vetoed (corpus handlers/TestPageRecordTriggers.al:OnInsertRecord_ReturningFalse_SuppressesTheInsert).

use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun};
use al_testrun::{CaseId, Outcome};

const SRC: &str = r#"
table 50410 "PIC Row" { fields { field(1; Id; Integer) { } field(2; Qty; Integer) { } } keys { key(PK; Id) { Clustered = true; } } }
page 50410 "PIC Card" { PageType = Card; SourceTable = "PIC Row";
    layout { area(Content) { field(Id; Rec.Id) { } field(Qty; Rec.Qty) { } } }
    trigger OnInsertRecord(BelowxRec: Boolean): Boolean
    begin
        Rec.Insert();
        exit(false);
    end;
    trigger OnModifyRecord(): Boolean
    begin
        Rec.Modify();
        exit(false);
    end;
}
codeunit 50410 "T"
{
    Subtype = Test;
    [Test]
    procedure EditAfterTheCodeInsertedTheRow()
    var Card: TestPage "PIC Card"; Row: Record "PIC Row";
    begin
        Card.OpenNew();
        Card.Id.SetValue(1);
        Card.Qty.SetValue(5);
        Card.OK().Invoke();
        Row.Get(1);
        if Row.Qty <> 5 then Error('Qty was %1', Row.Qty);
    end;
}
"#;

#[test]
fn an_edit_after_the_code_inserted_the_row_modifies_it() {
    let rt = CorpusRuntime::run_suites_with(
        vec![AppSource {
            id: "main".into(),
            name: "main".into(),
            dependencies: vec![],
            files: vec![("main.al".into(), SRC.into())],
        }],
        vec![al_sema::AppKind::Workspace],
        &[],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions::default(),
    );
    let o = rt.first_outcome(&CaseId {
        codeunit: 50410,
        name: "EditAfterTheCodeInsertedTheRow".into(),
    });
    assert_eq!(o, Outcome::Pass, "{o:?}");
}
