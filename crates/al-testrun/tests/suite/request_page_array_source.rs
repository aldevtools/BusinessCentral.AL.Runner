//! A request page control bound to an element of an array variable of the report (`field("Text[1]"; Text[1])`), as Base
//! Application's "Contact - Cover Sheet" declares its text lines, is a control of the request page. The report's dataset columns put
//! routines in before the request page, which moved the offsets the control routines are named by (the control was refused as "bound
//! to a name that is no variable of its base object"; Tests-Marketing, 17 tests, and `Enabled = Var` of LogInteraction, 3 more). The expression control pinned by corpus
//! misc/lt-probes-needed-7/PnGTempControl.al and pages/W1Controls.al:TempRecordControl_SetValue; that an array element behaves
//! the same is the assumption of an expression control.

use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun};
use al_testrun::{CaseId, Outcome};

const BASE: &str = r#"
table 50519 "RPA Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
codeunit 50520 "RPA Log" { SingleInstance = true; var Seen: Text; procedure Put(T: Text) begin Seen := T; end; procedure Get(): Text begin exit(Seen); end; }
report 50520 "RPA Report" { ProcessingOnly = true; UsageCategory = None;
    dataset { dataitem(Row; "RPA Row") { column(Text_1_; Text[1]) { } } }
    requestpage { SaveValues = false; AboutTitle = 'About'; AboutText = 'Get cover sheets.'; layout { area(Content) { group(Options) { Caption = 'Options'; group(Text) { Caption = 'Text';
        field("Text[1]"; Text[1]) { ApplicationArea = All; ShowCaption = false; ToolTip = 'Specifies a placeholder.'; }
        field("Text[2]"; Text[2]) { ApplicationArea = All; Importance = Additional; ShowCaption = false; ToolTip = 'x'; } }
        group(Remarks) { Caption = 'Remarks'; field("Marks[1]"; Marks[1]) { ApplicationArea = All; Caption = 'As agreed upon'; ToolTip = 'y'; } } } } } }
    trigger OnPreReport() var L: Codeunit "RPA Log"; begin L.Put(Text[1] + Format(Marks[1])); end;
    var Text: array[3] of Text[30]; Marks: array[2] of Boolean; }
"#;

const MAIN: &str = r#"
codeunit 50521 "T"
{
    Subtype = Test;
    [Test]
    [HandlerFunctions('Fill')]
    procedure SetTheElement()
    var L: Codeunit "RPA Log";
    begin
        Report.Run(Report::"RPA Report");
        if L.Get() <> 'helloYes' then Error('seen %1', L.Get());
    end;
    [RequestPageHandler]
    procedure Fill(var P: TestRequestPage "RPA Report") begin P."Text[1]".SetValue('hello'); P."Marks[1]".SetValue(true); P.OK().Invoke(); end;
}
"#;

fn run_variant(base: &str) -> Outcome {
    let rt = CorpusRuntime::run_suites_with(
        vec![AppSource {
            id: "main".into(),
            name: "main".into(),
            dependencies: vec![],
            files: vec![("main.al".into(), format!("{base}{MAIN}"))],
        }],
        vec![al_sema::AppKind::Workspace],
        &[],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions::default(),
    );
    rt.first_outcome(&CaseId {
        codeunit: 50521,
        name: "SetTheElement".into(),
    })
}

#[test]
fn a_request_page_control_bound_to_an_array_element_takes_a_value() {
    assert_eq!(run_variant(BASE), Outcome::Pass);
}
