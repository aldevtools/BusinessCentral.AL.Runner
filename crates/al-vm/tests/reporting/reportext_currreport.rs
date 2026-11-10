//! `CurrReport` in the code of a report extension is the report run the extension belongs to: its members
//! resolve as in the base report (Tests-ERM, Base Application's "Mfg. Document Entries" calls
//! `CurrReport.Break()` in `OnPreDataItem` of a data item it adds; issue 19). Pins for the members:
//! corpus reports/TestReportRuntimeContracts.al:80 (`Skip`, `Break`, `Quit`) and
//! streams/TestReportCurrReportLanguageReport.al (`Language`).

use al_sema::workspace::AppSource;

const SRC: &str = r#"
table 69200 "RC Tab" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
report 69201 "RC Base"
{
    UsageCategory = None;
    dataset { dataitem(Item; "RC Tab") { column(Id; Id) { } } }
}
reportextension 69202 "RC Ext" extends "RC Base"
{
    dataset
    {
        add(Item)
        {
            column(Extra; Id) { }
        }
        addlast(Item)
        {
            dataitem(Added; "RC Tab")
            {
                trigger OnPreDataItem()
                begin
                    CurrReport.Break();
                end;
                trigger OnAfterGetRecord()
                begin
                    CurrReport.Skip();
                    CurrReport.Quit();
                    Lang := CurrReport.Language;
                end;
            }
        }
    }
    trigger OnPreReport()
    begin
        CurrReport.Language := 1033;
        if CurrReport.Language <> 1033 then Error('language');
    end;
    var Lang: Integer;
}
"#;

#[test]
fn currreport_resolves_in_a_data_item_a_report_extension_adds() {
    let apps = vec![AppSource {
        id: "a".into(),
        name: "a".into(),
        dependencies: vec![],
        files: vec![("f.al".into(), SRC.into())],
    }];
    let (ws, l) = al_sema::lower::compile(apps, &Default::default());
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let mut reasons = Vec::new();
    for (oi, o) in l.unsupported.iter().enumerate() {
        for (ri, r) in o.iter().enumerate() {
            if let Some(r) = r.first() {
                reasons.push(format!(
                    "{}.{}: {r}",
                    l.program.objects[oi].name, l.program.objects[oi].routines[ri].name
                ));
            }
        }
    }
    assert!(reasons.is_empty(), "{reasons:#?}");
}

// Running it: the members act on the report run the extension belongs to
// (reports/TestReportRuntimeContracts.al:80: Skip skips the row, Break ends the data item, Quit ends the run).
mod running {
    use al_builtins::value::Value;
    use al_vm::{Engine, Session};

    fn run(mode: &str) -> String {
        let src = format!(
            r#"
table 69210 "RR Tab" {{ fields {{ field(1; Id; Integer) {{ }} }} keys {{ key(PK; Id) {{ }} }} }}
codeunit 69211 "RR Log"
{{
    SingleInstance = true;
    var Log: Text;
    procedure Add(Entry: Text) begin Log += Entry + ';'; end;
    procedure Take(): Text var R: Text; begin R := Log; Log := ''; exit(R); end;
}}
report 69212 "RR Base"
{{
    ProcessingOnly = true; UseRequestPage = false;
    dataset {{ dataitem(Base; "RR Tab") {{ trigger OnAfterGetRecord() var L: Codeunit "RR Log"; begin L.Add('base' + Format(Base.Id)); end; }} }}
}}
reportextension 69213 "RR Ext" extends "RR Base"
{{
    dataset
    {{
        addlast(Base)
        {{
            dataitem(Added; "RR Tab")
            {{
                trigger OnAfterGetRecord() var L: Codeunit "RR Log"; begin
                    L.Add('added' + Format(Added.Id));
                    if (Added.Id = 1) and ('{mode}' = 'skip') then CurrReport.Skip();
                    if (Added.Id = 2) and ('{mode}' = 'break') then CurrReport.Break();
                    if (Added.Id = 2) and ('{mode}' = 'quit') then CurrReport.Quit();
                    L.Add('after' + Format(Added.Id));
                end;
            }}
        }}
    }}
}}
codeunit 69214 C
{{
    procedure F(): Text var R: Record "RR Tab"; L: Codeunit "RR Log"; i: Integer; begin
        for i := 1 to 3 do begin R.Id := i; R.Insert(); end; L.Take();
        Report.Run(Report::"RR Base", false); exit(L.Take()); end;
}}
"#
        );
        let (ws, l) = al_sema::lower::compile(
            vec![al_sema::workspace::AppSource {
                id: "a".into(),
                name: "a".into(),
                dependencies: vec![],
                files: vec![("f.al".into(), src)],
            }],
            &Default::default(),
        );
        assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
        let blocked: Vec<Vec<Option<String>>> = l
            .unsupported
            .iter()
            .map(|o| o.iter().map(|r| r.first().map(|r| r.to_string())).collect())
            .collect();
        let oi = l.program.objects.iter().position(|o| o.name == "C").unwrap();
        let ri = l.program.objects[oi]
            .routines
            .iter()
            .position(|r| r.name == "F")
            .unwrap();
        let mut s = Session::new(Engine::new(l.program, blocked));
        let inst = s.instance(oi);
        match s.call(oi, ri, inst, vec![]).map(|(v, _)| v) {
            Ok(Value::Text(t)) => t.to_string(),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn skip_skips_the_row_of_the_added_data_item() {
        let log = run("skip");
        assert!(log.contains("added1;added2;after2;"), "{log}");
        assert!(!log.contains("after1;"), "{log}");
    }

    #[test]
    fn break_ends_the_added_data_item() {
        let log = run("break");
        assert!(log.contains("added1;after1;added2;"), "{log}");
        assert!(!log.contains("added3;"), "{log}");
    }

    #[test]
    fn quit_ends_the_run() {
        let log = run("quit");
        assert!(log.contains("added1;after1;added2;"), "{log}");
        assert!(!log.contains("after2;") && !log.contains("added3;"), "{log}");
    }
}
