// The scans of production code stop at the first `#[cfg(test)]` of a file; a test module in a file of its own starts with one.
#![cfg(test)]
//! The reasons a refused test gives: each names the cause a reader has to fix, not a later step that the cause made fail.

use super::tests::{outcomes, with_baseline};
use super::*;

/// Two request pages of two reports, each extended by a report extension whose request page has a trigger (refused while
/// lowering, so the handler's parameter type is not lowered either), and one handler per report (al-runner issue 8100).
const TWO_REQUEST_PAGES: &str = r#"
report 60200 "Inv" { ProcessingOnly = true; dataset { dataitem(Row; Integer) { DataItemTableView = where(Number = const(1)); } }
  requestpage { layout { area(Content) { group(Options) { field(A; A) { } } } } } var A: Boolean; }
report 60201 "Cr" { ProcessingOnly = true; dataset { dataitem(Row; Integer) { DataItemTableView = where(Number = const(1)); } }
  requestpage { layout { area(Content) { group(Options) { field(B; B) { } } } } } var B: Boolean; }
reportextension 60210 "Inv Ext" extends "Inv" { requestpage { layout { addlast(Options) { field(X; X) { } } }
  trigger OnOpenPage() begin X := true; end; } trigger OnPostReport() begin if X then; end; var X: Boolean; }
reportextension 60211 "Cr Ext" extends "Cr" { requestpage { layout { addlast(Options) { field(X; X) { } } }
  trigger OnOpenPage() begin X := true; end; } trigger OnPostReport() begin if X then; end; var X: Boolean; }
"#;

#[test]
fn two_refused_request_page_handlers_of_two_reports_give_the_first_handlers_own_reason() {
    let src = format!(
        "{TWO_REQUEST_PAGES}
        codeunit 60220 T {{ Subtype = Test;
            [Test] [HandlerFunctions('InvHandler,CrHandler')] procedure Both() begin Report.RunModal(Report::\"Inv\"); end;
            [RequestPageHandler] procedure InvHandler(var P: TestRequestPage \"Inv\") begin P.OK().Invoke(); end;
            [RequestPageHandler] procedure CrHandler(var P: TestRequestPage \"Cr\") begin P.OK().Invoke(); end; }}"
    );
    let o = outcomes(&src);
    let Outcome::NotRun(why) = &o[0].1 else {
        panic!("{o:?}")
    };
    assert!(why.starts_with("handler InvHandler: "), "{why}");
    assert!(why.contains("TestRequestPage \"Inv\""), "{why}");
    assert!(!why.contains("which one BC would bind"), "{why}");
}

#[test]
fn two_request_page_handlers_of_one_report_keep_the_binding_refusal_unless_the_page_is_refused() {
    // Main's outcome: two handlers whose parameters name the same report are refused as a binding BC's choice of is not
    // pinned. When the request page itself is refused, the handler's own reason comes first, as with one handler.
    let src = format!(
        "{TWO_REQUEST_PAGES}
        report 60202 \"Plain\" {{ ProcessingOnly = true; dataset {{ dataitem(Row; Integer) {{ }} }} }}
        codeunit 60220 T {{ Subtype = Test;
            [Test] [HandlerFunctions('H1,H2')] procedure Plain() begin end;
            [Test] [HandlerFunctions('R1,R2')] procedure Refused() begin end;
            [Test] [HandlerFunctions('M1,M2')] procedure Messages() begin end;
            [RequestPageHandler] procedure H1(var P: TestRequestPage \"Plain\") begin end;
            [RequestPageHandler] procedure H2(var P: TestRequestPage \"Plain\") begin end;
            [RequestPageHandler] procedure R1(var P: TestRequestPage \"Inv\") begin end;
            [RequestPageHandler] procedure R2(var P: TestRequestPage \"Inv\") begin end;
            [MessageHandler] procedure M1(Msg: Text) begin end;
            [MessageHandler] procedure M2(Msg: Text) begin end; }}"
    );
    let o = outcomes(&src);
    let reason = |name: &str| match &o.iter().find(|(n, _)| n == name).unwrap().1 {
        Outcome::NotRun(r) => r.clone(),
        other => panic!("{name}: {other:?}"),
    };
    assert!(reason("Plain").contains("H1 and H2 are both RequestPage handlers; which one BC would bind is not pinned"));
    assert!(reason("Messages").contains("M1 and M2 are both Message handlers; which one BC would bind is not pinned"));
    let refused = reason("Refused");
    assert!(refused.starts_with("handler R1: ") && refused.contains("TestRequestPage \"Inv\""), "{refused}");
}

#[test]
fn a_read_of_a_table_a_stopped_install_routine_writes_names_the_routine_and_its_refusal() {
    // al-runner issue 8104: the dependency's install trigger stops at a refused call before it writes its rows.
    let dep = r#"
table 50100 "Rule" { fields { field(1; "Code"; Code[20]) { } field(2; Active; Boolean) { } } keys { key(PK; "Code") { } } }
codeunit 50110 "Installer" { Subtype = Install;
  trigger OnInstallAppPerCompany() begin Check(); Create(); end;
  local procedure Check() begin Debugger.Deactivate(); end;
  local procedure Create() var R: Record "Rule"; begin R.Code := 'DEFAULT'; R.Active := true; if R.Insert() then; end; }
"#;
    let main = r#"codeunit 50001 "T" { Subtype = Test;
  [Test] procedure Scan() var R: Record "Rule"; begin R.SetRange(Active, true); if R.FindSet() then; end;
  [Test] procedure Lookup() var R: Record "Rule"; begin if R.Get('NONE') then; end; }"#;
    let outcome = with_baseline(main, dep);
    for name in ["Scan", "Lookup"] {
        let Outcome::NotRun(why) = outcome(name) else {
            panic!("{name}: {:?}", outcome(name))
        };
        assert!(
            why.contains("because Installer.OnInstallAppPerCompany stopped: "),
            "{name}: {why}"
        );
        assert!(why.contains("Debugger"), "{name}: {why}");
        // The site of the stop (object and position) is in the baseline note, not in the reason, so that tests stopped by
        // the same refused call group together in the ranking of refusals.
        assert!(!why.contains(" at "), "{name}: {why}");
    }
}
