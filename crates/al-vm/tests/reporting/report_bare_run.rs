//! The bare `RunModal()` / `Run()` in a report's own procedure runs the instance whose procedure runs: the triggers run once and
//! the globals they set read back through the variable; a second call, and a bare `RunModal()` inside OnPreReport of the running
//! instance, raise ObjectHasAlreadyBeenRun (the run goes on). The bare `RequestOptionsPage.Caption` reads the request page's
//! Caption, and back what was set, also inside a later run (corpus master ac3499d6,
//! report/lt-y40-bare-report-run/Y40BareReportRun.al:214-377, BC 27.5/28.5/29.0).

use al_testkit::{Built, text};

const SRC: &str = r#"
table 50240 "BR Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
codeunit 50241 "BR Log" { SingleInstance = true;
    var Entries: Text;
    procedure Add(E: Text) begin Entries += E + ';'; end;
    procedure Take(): Text var R: Text; begin R := Entries; Entries := ''; exit(R); end; }
report 50242 "BR Finder" { UsageCategory = None; ProcessingOnly = true;
    dataset { dataitem(Row; "BR Row") {
        trigger OnPreDataItem() begin Log.Add('pre-item count=' + Format(Row.Count())); end;
        trigger OnAfterGetRecord() begin Found := true; Visited += 1; end; } }
    requestpage { Caption = 'BR Options'; layout { area(Content) { field(NoteCtl; Note) { } } } }
    trigger OnInitReport() begin Log.Add('init'); end;
    trigger OnPreReport()
    begin
        Log.Add('pre caption=' + OptionsCaption());
        if Inner then begin
            Inner := false;
            if TryBare() then Log.Add('inner-ok') else Log.Add('inner-error:' + GetLastErrorText() + '|' + GetLastErrorCode());
        end;
    end;
    trigger OnPostReport() begin Log.Add('post visited=' + Format(Visited)); end;
    var Log: Codeunit "BR Log"; Found: Boolean; Visited: Integer; Note: Text[30]; Inner: Boolean;
    procedure FindModal(): Boolean begin UseRequestPage(false); RunModal(); exit(Found); end;
    procedure FindRun(): Boolean begin UseRequestPage(false); Run(); exit(Found); end;
    procedure InsidePreReport() begin Inner := true; UseRequestPage(false); RunModal(); end;
    procedure State(): Text begin exit('found=' + Format(Found) + ' visited=' + Format(Visited) + ' userp=' + Format(UseRequestPage())); end;
    procedure OptionsCaption(): Text begin exit(RequestOptionsPage.Caption); end;
    procedure SetOptionsCaption(C: Text) begin RequestOptionsPage.Caption := C; end;
    [TryFunction] local procedure TryBare() begin RunModal(); end;
}
codeunit 50243 C {
    var Log: Codeunit "BR Log";
    local procedure Seed(N: Integer) var R: Record "BR Row"; I: Integer;
    begin R.DeleteAll(); for I := 1 to N do begin R.Id := I; R.Insert(); end; Log.Take(); end;
    [TryFunction] local procedure TryFind(var Rep: Report "BR Finder"; var F: Boolean) begin F := Rep.FindModal(); end;
    local procedure Find(var Rep: Report "BR Finder"): Text var F: Boolean;
    begin
        ClearLastError();
        if TryFind(Rep, F) then exit('ok found=' + Format(F));
        exit('error:' + GetLastErrorText() + '|' + GetLastErrorCode());
    end;
    procedure Modal(): Text var Rep: Report "BR Finder"; Out: Text;
    begin Seed(3); Out := Find(Rep) + '#' + Log.Take() + '#' + Rep.State(); exit(Out); end;
    procedure Twice(): Text var Rep: Report "BR Finder"; Out: Text;
    begin Seed(2); Out := Find(Rep) + '#' + Log.Take() + '#' + Find(Rep) + '#' + Log.Take() + '#' + Rep.State(); exit(Out); end;
    procedure RunIt(): Text var Rep: Report "BR Finder"; Out: Text;
    begin Seed(2); Out := Format(Rep.FindRun()) + '#' + Log.Take() + '#' + Rep.State(); exit(Out); end;
    procedure Inside(): Text var Rep: Report "BR Finder"; Out: Text;
    begin Seed(1); Rep.InsidePreReport(); Out := Log.Take() + '#' + Rep.State(); exit(Out); end;
    procedure Caption(): Text var Rep: Report "BR Finder"; Out: Text;
    begin Seed(1); Out := Rep.OptionsCaption(); Rep.SetOptionsCaption('Changed'); Out += '#' + Rep.OptionsCaption() + '#' + Find(Rep) + '#' + Log.Take(); exit(Out); end;
}
"#;

fn run(routine: &str) -> String {
    text(Built::new(al_testkit::compile(SRC)).run_in("C", routine))
}

#[test]
fn bare_run_modal_in_a_procedure_runs_that_instance() {
    assert_eq!(
        run("Modal"),
        "ok found=Yes#init;pre caption=BR Options;pre-item count=3;post visited=3;#found=Yes visited=3 userp=No"
    );
}

#[test]
fn a_second_bare_run_modal_raises_and_leaves_the_instance_as_it_is() {
    assert_eq!(
        run("Twice"),
        "ok found=Yes#init;pre caption=BR Options;pre-item count=2;post visited=2;#\
         error:The object has been run. Use the function CLEAR(Report).|ObjectHasAlreadyBeenRun##found=Yes visited=2 userp=No"
    );
}

#[test]
fn bare_run_in_a_procedure_runs_that_instance() {
    assert_eq!(
        run("RunIt"),
        "Yes#init;pre caption=BR Options;pre-item count=2;post visited=2;#found=Yes visited=2 userp=No"
    );
}

#[test]
fn bare_run_modal_inside_the_running_instance_raises_and_the_run_goes_on() {
    assert_eq!(
        run("Inside"),
        "init;pre caption=BR Options;inner-error:The object has been run. Use the function CLEAR(Report).|ObjectHasAlreadyBeenRun;\
         pre-item count=1;post visited=1;#found=Yes visited=1 userp=No"
    );
}

#[test]
fn bare_request_options_page_caption_reads_back_what_was_set() {
    assert_eq!(
        run("Caption"),
        "BR Options#Changed#ok found=Yes#init;pre caption=Changed;pre-item count=1;post visited=1;"
    );
}
