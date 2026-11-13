//! AL Runner issue 6924 (comment of 2026-11-13): a procedure of a codeunit with TableNo has no implicit Rec (corpus
//! must-not-compile 3140, a bare field there is AL0118), so a bare `Message`, `UserId` or `Today` there is the system
//! function even when the TableNo table has a field of that name. It runs as the system function; the field is not read.

use super::user_reports::{all_pass, run_with_deps};

#[test]
fn issue_6924_system_functions_named_like_a_field_run_outside_on_run() {
    let objects = "table 50200 \"Repro Ctx\"
{
    fields
    {
        field(1; \"Entry No.\"; Integer) { }
        field(2; Message; Text[50]) { }
        field(3; UserId; Code[50]) { }
        field(4; Today; Date) { }
    }
    keys { key(PK; \"Entry No.\") { Clustered = true; } }
}
codeunit 50201 \"Repro Handler\"
{
    TableNo = \"Repro Ctx\";
    trigger OnRun() begin end;
    procedure Say() begin Message('x'); end;
    procedure User(): Text begin exit(UserId); end;
    procedure UserCall(): Text begin exit(UserId()); end;
    procedure Day(): Date begin exit(Today); end;
    procedure DayCall(): Date begin exit(Today()); end;
}
codeunit 50202 T
{
    Subtype = Test;
    var Said: Text;
    [Test]
    [HandlerFunctions('MsgHandler')]
    procedure SystemFunctions()
    var
        H: Codeunit \"Repro Handler\";
    begin
        H.Say();
        if Said <> 'x' then Error('message %1', Said);
        if H.User() <> UserId() then Error('UserId %1', H.User());
        if H.UserCall() <> UserId() then Error('UserId() %1', H.UserCall());
        if H.Day() <> Today() then Error('Today %1', H.Day());
        if H.DayCall() <> Today() then Error('Today() %1', H.DayCall());
    end;
    [MessageHandler]
    procedure MsgHandler(Msg: Text[1024]) begin Said := Msg; end;
}";
    all_pass(run_with_deps(&[], objects));
}
