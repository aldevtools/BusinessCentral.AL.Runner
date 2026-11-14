//! AL Runner issue 6924 (comment of 2026-11-13): a procedure of a codeunit with TableNo has no implicit Rec (corpus
//! must-not-compile 3140, corpus PR 1386, not pinned yet), so a bare `Message`, `UserId`, `Today`, `WorkDate` or
//! `CompanyName` there is the system function even when the TableNo table has a field of that name. It runs as the
//! system function under the assumption `tableno-procedure-bare-system-function`; the field is not read.

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
        field(5; WorkDate; Date) { }
        field(6; CompanyName; Text[30]) { }
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
    procedure Work(): Date begin exit(WorkDate); end;
    procedure Company(): Text begin exit(CompanyName); end;
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
        if H.Work() <> WorkDate() then Error('WorkDate %1', H.Work());
        if H.Company() <> CompanyName() then Error('CompanyName %1', H.Company());
    end;
    [MessageHandler]
    procedure MsgHandler(Msg: Text[1024]) begin Said := Msg; end;
}";
    all_pass(run_with_deps(&[], objects));
}

/// The quoted names in a procedure, and a codeunit that also declares a global `Rec` (corpus PR 1393,
/// codeunits/TableNoBareSystemFunction.al:405-417 and :459-482, under the same assumption): in a procedure the bare and
/// quoted names call the system function and `Rec.UserId` reads the global variable; in OnRun the bare name and
/// `Rec.UserId` read the run record.
#[test]
fn quoted_names_and_a_global_rec_run_the_system_function_in_a_procedure() {
    let objects = "table 50200 \"Repro Ctx\"
{
    fields
    {
        field(1; \"Entry No.\"; Integer) { }
        field(3; UserId; Code[50]) { }
        field(4; Today; Date) { }
    }
    keys { key(PK; \"Entry No.\") { Clustered = true; } }
}
codeunit 50201 \"Repro Global Rec\"
{
    TableNo = \"Repro Ctx\";
    trigger OnRun()
    begin
        RunBare := UserId;
        RunQuoted := \"UserId\";
        RunRecUser := Rec.UserId;
    end;
    var
        Rec: Record \"Repro Ctx\";
        RunBare: Text;
        RunQuoted: Text;
        RunRecUser: Text;
    procedure SetGlobalRec(NewRec: Record \"Repro Ctx\") begin Rec := NewRec; end;
    procedure Got(): Text begin exit(RunBare + '|' + RunQuoted + '|' + RunRecUser); end;
    procedure User(): Text begin exit(UserId); end;
    procedure QuotedUser(): Text begin exit(\"UserId\"); end;
    procedure QuotedDay(): Date begin exit(\"Today\"); end;
    procedure RecUser(): Text begin exit(Rec.UserId); end;
}
codeunit 50202 T
{
    Subtype = Test;
    [Test]
    procedure GlobalRec()
    var
        H: Codeunit \"Repro Global Rec\";
        RunRec: Record \"Repro Ctx\";
        GlobalRec: Record \"Repro Ctx\";
    begin
        RunRec.\"Entry No.\" := 1;
        RunRec.UserId := 'RUN-USER';
        GlobalRec.\"Entry No.\" := 2;
        GlobalRec.UserId := 'GLOBAL-USER';
        H.SetGlobalRec(GlobalRec);
        H.Run(RunRec);
        if H.Got() <> 'RUN-USER|RUN-USER|RUN-USER' then Error('OnRun %1', H.Got());
        if H.User() <> UserId() then Error('UserId %1', H.User());
        if H.QuotedUser() <> UserId() then Error('quoted UserId %1', H.QuotedUser());
        if H.QuotedDay() <> Today() then Error('quoted Today %1', H.QuotedDay());
        if H.RecUser() <> 'GLOBAL-USER' then Error('Rec.UserId %1', H.RecUser());
    end;
}";
    all_pass(run_with_deps(&[], objects));
}
