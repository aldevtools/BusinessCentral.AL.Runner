// the shape of :1042 without its subscriber: the trigger reads before the event is raised
codeunit 50010 "NBX T10 OuterAssign_TwoProcs_N"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBX Cfg";
        Setup: Codeunit "NBX Setup";
        Caller: Codeunit "NBX Caller";

    [Test]
    procedure OuterAssign_TwoProcs_NoSubscriber()
    var
        Line: Record "NBX Line";
    begin
        Setup.AfterInsert(Line);
        ValidateItem(Line, 'outer-assign');
        Setup.Check('I x=0/NEW/I1;T x=0/OUT/I1;I after x=0/OUT/I1;');
    end;

    local procedure ValidateItem(var Line: Record "NBX Line"; CfgMode: Text)
    begin
        Cfg.Set(CfgMode);
        Line.Validate("Item No.", 'I2');
    end;

    local procedure ValidateMode(var Line: Record "NBX Line"; CfgMode: Text)
    begin
        Cfg.Set(CfgMode);
        Line.Validate(Mode, Line.Mode::Production);
    end;
}

// the shape of :1056 without its subscriber
codeunit 50011 "NBX T11 InnerAssign_TwoProcs_N"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBX Cfg";
        Setup: Codeunit "NBX Setup";
        Caller: Codeunit "NBX Caller";

    [Test]
    procedure InnerAssign_TwoProcs_NoSubscriber()
    var
        Line: Record "NBX Line";
    begin
        Setup.AfterInsert(Line);
        ValidateItem(Line, 'inner-assign');
        Setup.Check('I x=0/NEW/I1;T x=0/NEW/I1;I after x=0/INN/I1;');
    end;

    local procedure ValidateItem(var Line: Record "NBX Line"; CfgMode: Text)
    begin
        Cfg.Set(CfgMode);
        Line.Validate("Item No.", 'I2');
    end;

    local procedure ValidateMode(var Line: Record "NBX Line"; CfgMode: Text)
    begin
        Cfg.Set(CfgMode);
        Line.Validate(Mode, Line.Mode::Production);
    end;
}

// no procedure between the triggers
codeunit 50012 "NBX T12 OuterAssign_DirectInTr"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBX Cfg";
        Setup: Codeunit "NBX Setup";
        Caller: Codeunit "NBX Caller";

    [Test]
    procedure OuterAssign_DirectInTrigger()
    var
        Line: Record "NBX Line";
    begin
        Setup.AfterInsert(Line);
        ValidateItem(Line, 'direct');
        Setup.Check('I x=0/NEW/I1;T x=0/OUT/I1;I after x=0/OUT/I1;');
    end;

    local procedure ValidateItem(var Line: Record "NBX Line"; CfgMode: Text)
    begin
        Cfg.Set(CfgMode);
        Line.Validate("Item No.", 'I2');
    end;

    local procedure ValidateMode(var Line: Record "NBX Line"; CfgMode: Text)
    begin
        Cfg.Set(CfgMode);
        Line.Validate(Mode, Line.Mode::Production);
    end;
}

// one procedure between
codeunit 50013 "NBX T13 OuterAssign_OneProcedu"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBX Cfg";
        Setup: Codeunit "NBX Setup";
        Caller: Codeunit "NBX Caller";

    [Test]
    procedure OuterAssign_OneProcedure()
    var
        Line: Record "NBX Line";
    begin
        Setup.AfterInsert(Line);
        ValidateItem(Line, 'one-proc');
        Setup.Check('I x=0/NEW/I1;T x=0/OUT/I1;I after x=0/OUT/I1;');
    end;

    local procedure ValidateItem(var Line: Record "NBX Line"; CfgMode: Text)
    begin
        Cfg.Set(CfgMode);
        Line.Validate("Item No.", 'I2');
    end;

    local procedure ValidateMode(var Line: Record "NBX Line"; CfgMode: Text)
    begin
        Cfg.Set(CfgMode);
        Line.Validate(Mode, Line.Mode::Production);
    end;
}

// three procedures between
codeunit 50014 "NBX T14 OuterAssign_ThreeProce"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBX Cfg";
        Setup: Codeunit "NBX Setup";
        Caller: Codeunit "NBX Caller";

    [Test]
    procedure OuterAssign_ThreeProcedures()
    var
        Line: Record "NBX Line";
    begin
        Setup.AfterInsert(Line);
        ValidateItem(Line, 'three-procs');
        Setup.Check('I x=0/NEW/I1;T x=0/OUT/I1;I after x=0/OUT/I1;');
    end;

    local procedure ValidateItem(var Line: Record "NBX Line"; CfgMode: Text)
    begin
        Cfg.Set(CfgMode);
        Line.Validate("Item No.", 'I2');
    end;

    local procedure ValidateMode(var Line: Record "NBX Line"; CfgMode: Text)
    begin
        Cfg.Set(CfgMode);
        Line.Validate(Mode, Line.Mode::Production);
    end;
}

// Validate(Mode) without a value
codeunit 50015 "NBX T15 OuterAssign_OneArgumen"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBX Cfg";
        Setup: Codeunit "NBX Setup";
        Caller: Codeunit "NBX Caller";

    [Test]
    procedure OuterAssign_OneArgumentValidate()
    var
        Line: Record "NBX Line";
    begin
        Setup.AfterInsert(Line);
        ValidateItem(Line, 'one-arg');
        Setup.Check('I x=0/NEW/I1;T x=0/OUT/I1;I after x=0/OUT/I1;');
    end;

    local procedure ValidateItem(var Line: Record "NBX Line"; CfgMode: Text)
    begin
        Cfg.Set(CfgMode);
        Line.Validate("Item No.", 'I2');
    end;

    local procedure ValidateMode(var Line: Record "NBX Line"; CfgMode: Text)
    begin
        Cfg.Set(CfgMode);
        Line.Validate(Mode, Line.Mode::Production);
    end;
}

// Rec.CopyFromItem()
codeunit 50016 "NBX T16 OuterAssign_RecDotProc"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBX Cfg";
        Setup: Codeunit "NBX Setup";
        Caller: Codeunit "NBX Caller";

    [Test]
    procedure OuterAssign_RecDotProcedure()
    var
        Line: Record "NBX Line";
    begin
        Setup.AfterInsert(Line);
        ValidateItem(Line, 'rec-dot');
        Setup.Check('I x=0/NEW/I1;T x=0/OUT/I1;I after x=0/OUT/I1;');
    end;

    local procedure ValidateItem(var Line: Record "NBX Line"; CfgMode: Text)
    begin
        Cfg.Set(CfgMode);
        Line.Validate("Item No.", 'I2');
    end;

    local procedure ValidateMode(var Line: Record "NBX Line"; CfgMode: Text)
    begin
        Cfg.Set(CfgMode);
        Line.Validate(Mode, Line.Mode::Production);
    end;
}

// the trigger validates its own field again
codeunit 50017 "NBX T17 OuterAssign_SameField"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBX Cfg";
        Setup: Codeunit "NBX Setup";
        Caller: Codeunit "NBX Caller";

    [Test]
    procedure OuterAssign_SameField()
    var
        Line: Record "NBX Line";
    begin
        Setup.AfterInsert(Line);
        ValidateItem(Line, 'same-field');
        Setup.Check('I x=0/NEW/I1;J x=0/OUT/I1;I after x=0/OUT/I1;');
    end;

    local procedure ValidateItem(var Line: Record "NBX Line"; CfgMode: Text)
    begin
        Cfg.Set(CfgMode);
        Line.Validate("Item No.", 'I2');
    end;

    local procedure ValidateMode(var Line: Record "NBX Line"; CfgMode: Text)
    begin
        Cfg.Set(CfgMode);
        Line.Validate(Mode, Line.Mode::Production);
    end;
}

// a temporary record
codeunit 50018 "NBX T18 OuterAssign_TemporaryR"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBX Cfg";
        Setup: Codeunit "NBX Setup";
        Caller: Codeunit "NBX Caller";

    [Test]
    procedure OuterAssign_TemporaryRecord()
    var
        Line: Record "NBX Line" temporary;
    begin
        Setup.AfterInsert(Line);
        ValidateItem(Line, 'outer-assign');
        Setup.Check('I x=0/NEW/I1;T x=0/OUT/I1;I after x=0/OUT/I1;');
    end;

    local procedure ValidateItem(var Line: Record "NBX Line" temporary; CfgMode: Text)
    begin
        Cfg.Set(CfgMode);
        Line.Validate("Item No.", 'I2');
    end;

    local procedure ValidateMode(var Line: Record "NBX Line" temporary; CfgMode: Text)
    begin
        Cfg.Set(CfgMode);
        Line.Validate(Mode, Line.Mode::Production);
    end;
}

// a buffer from Get
codeunit 50019 "NBX T19 OuterAssign_AfterGet"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBX Cfg";
        Setup: Codeunit "NBX Setup";
        Caller: Codeunit "NBX Caller";

    [Test]
    procedure OuterAssign_AfterGet()
    var
        Line: Record "NBX Line";
    begin
        Setup.AfterInsert(Line);
        Line.Get(1);
        Line.Series := 'NEW';
        ValidateItem(Line, 'outer-assign');
        Setup.Check('I x=0/NEW/I1;T x=0/OUT/I1;I after x=0/OUT/I1;');
    end;

    local procedure ValidateItem(var Line: Record "NBX Line"; CfgMode: Text)
    begin
        Cfg.Set(CfgMode);
        Line.Validate("Item No.", 'I2');
    end;

    local procedure ValidateMode(var Line: Record "NBX Line"; CfgMode: Text)
    begin
        Cfg.Set(CfgMode);
        Line.Validate(Mode, Line.Mode::Production);
    end;
}

// a record never inserted
codeunit 50020 "NBX T20 OuterAssign_FreshRecor"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBX Cfg";
        Setup: Codeunit "NBX Setup";
        Caller: Codeunit "NBX Caller";

    [Test]
    procedure OuterAssign_FreshRecord()
    var
        Line: Record "NBX Line";
    begin
        Line.DeleteAll();
        Line.Init();
        Line."Line No." := 1;
        Line.Series := 'NEW';
        Line."Item No." := 'I1';
        ValidateItem(Line, 'outer-assign');
        Setup.Check('I x=0/NEW/I1;T x=0/OUT/I1;I after x=0/OUT/I1;');
    end;

    local procedure ValidateItem(var Line: Record "NBX Line"; CfgMode: Text)
    begin
        Cfg.Set(CfgMode);
        Line.Validate("Item No.", 'I2');
    end;

    local procedure ValidateMode(var Line: Record "NBX Line"; CfgMode: Text)
    begin
        Cfg.Set(CfgMode);
        Line.Validate(Mode, Line.Mode::Production);
    end;
}

// a filter on the record
codeunit 50021 "NBX T21 OuterAssign_FilteredRe"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBX Cfg";
        Setup: Codeunit "NBX Setup";
        Caller: Codeunit "NBX Caller";

    [Test]
    procedure OuterAssign_FilteredRecord()
    var
        Line: Record "NBX Line";
    begin
        Setup.AfterInsert(Line);
        Line.SetRange("Line No.", 1);
        ValidateItem(Line, 'outer-assign');
        Setup.Check('I x=0/NEW/I1;T x=0/OUT/I1;I after x=0/OUT/I1;');
    end;

    local procedure ValidateItem(var Line: Record "NBX Line"; CfgMode: Text)
    begin
        Cfg.Set(CfgMode);
        Line.Validate("Item No.", 'I2');
    end;

    local procedure ValidateMode(var Line: Record "NBX Line"; CfgMode: Text)
    begin
        Cfg.Set(CfgMode);
        Line.Validate(Mode, Line.Mode::Production);
    end;
}

// the outer trigger filters xRec before it nests
codeunit 50022 "NBX T22 OuterFilter_BeforeNest"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBX Cfg";
        Setup: Codeunit "NBX Setup";
        Caller: Codeunit "NBX Caller";

    [Test]
    procedure OuterFilter_BeforeNesting()
    var
        Line: Record "NBX Line";
    begin
        Setup.AfterInsert(Line);
        ValidateItem(Line, 'outer-filter');
        Setup.Check('I x=0/NEW/I1;T x=0/NEW/I1;T filters=Series: F;I after x=0/NEW/I1;');
    end;

    local procedure ValidateItem(var Line: Record "NBX Line"; CfgMode: Text)
    begin
        Cfg.Set(CfgMode);
        Line.Validate("Item No.", 'I2');
    end;

    local procedure ValidateMode(var Line: Record "NBX Line"; CfgMode: Text)
    begin
        Cfg.Set(CfgMode);
        Line.Validate(Mode, Line.Mode::Production);
    end;
}

// a third Validate nested
codeunit 50023 "NBX T23 ThreeLevel_DeepAssign"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBX Cfg";
        Setup: Codeunit "NBX Setup";
        Caller: Codeunit "NBX Caller";

    [Test]
    procedure ThreeLevel_DeepAssign()
    var
        Line: Record "NBX Line";
    begin
        Setup.AfterInsert(Line);
        ValidateItem(Line, 'three-level');
        Setup.Check('I x=0/NEW/I1;T x=0/NEW/I1;O x=0/NEW/I1;T after x=0/DEEP/I1;I after x=0/DEEP/I1;');
    end;

    local procedure ValidateItem(var Line: Record "NBX Line"; CfgMode: Text)
    begin
        Cfg.Set(CfgMode);
        Line.Validate("Item No.", 'I2');
    end;

    local procedure ValidateMode(var Line: Record "NBX Line"; CfgMode: Text)
    begin
        Cfg.Set(CfgMode);
        Line.Validate(Mode, Line.Mode::Production);
    end;
}

// the Validate in the test procedure
codeunit 50024 "NBX T24 OuterAssign_FromTestPr"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBX Cfg";
        Setup: Codeunit "NBX Setup";
        Caller: Codeunit "NBX Caller";

    [Test]
    procedure OuterAssign_FromTestProcedure()
    var
        Line: Record "NBX Line";
    begin
        Setup.AfterInsert(Line);
        Cfg.Set('outer-assign');
        Line.Validate("Item No.", 'I2');
        Setup.Check('I x=0/NEW/I1;T x=0/OUT/I1;I after x=0/OUT/I1;');
    end;

    local procedure ValidateItem(var Line: Record "NBX Line"; CfgMode: Text)
    begin
        Cfg.Set(CfgMode);
        Line.Validate("Item No.", 'I2');
    end;

    local procedure ValidateMode(var Line: Record "NBX Line"; CfgMode: Text)
    begin
        Cfg.Set(CfgMode);
        Line.Validate(Mode, Line.Mode::Production);
    end;
}

// the Validate in a codeunit that is no test codeunit
codeunit 50025 "NBX T25 OuterAssign_FromCodeun"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBX Cfg";
        Setup: Codeunit "NBX Setup";
        Caller: Codeunit "NBX Caller";

    [Test]
    procedure OuterAssign_FromCodeunit()
    var
        Line: Record "NBX Line";
    begin
        Setup.AfterInsert(Line);
        Caller.ValidateItem(Line, 'outer-assign');
        Setup.Check('I x=0/NEW/I1;T x=0/OUT/I1;I after x=0/OUT/I1;');
    end;

    local procedure ValidateItem(var Line: Record "NBX Line"; CfgMode: Text)
    begin
        Cfg.Set(CfgMode);
        Line.Validate("Item No.", 'I2');
    end;

    local procedure ValidateMode(var Line: Record "NBX Line"; CfgMode: Text)
    begin
        Cfg.Set(CfgMode);
        Line.Validate(Mode, Line.Mode::Production);
    end;
}

// sole subscriber
codeunit 50026 "NBX T26 ScopeOnPrem_Subscriber"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBX Cfg";
        Setup: Codeunit "NBX Setup";
        Caller: Codeunit "NBX Caller";

    [Test]
    [Scope('OnPrem')]
    procedure ScopeOnPrem_Subscriber()
    var
        Line: Record "NBX Line";
    begin
        Setup.AfterInsert(Line);
        ValidateMode(Line, 'event');
        Setup.Check('T x=0/NEW/I1;P x=0/NEW/I1;');
    end;

    local procedure ValidateItem(var Line: Record "NBX Line"; CfgMode: Text)
    begin
        Cfg.Set(CfgMode);
        Line.Validate("Item No.", 'I2');
    end;

    local procedure ValidateMode(var Line: Record "NBX Line"; CfgMode: Text)
    begin
        Cfg.Set(CfgMode);
        Line.Validate(Mode, Line.Mode::Production);
    end;
}

// nested
codeunit 50027 "NBX T27 ScopeOnPremWithHandler"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBX Cfg";
        Setup: Codeunit "NBX Setup";
        Caller: Codeunit "NBX Caller";

    [Test]
    [HandlerFunctions('MessageHandler')]
    [Scope('OnPrem')]
    procedure ScopeOnPremWithHandler_Nested()
    var
        Line: Record "NBX Line";
    begin
        Setup.AfterInsert(Line);
        Cfg.Set('event-nested');
        Line.Validate("Item No.", 'I2');
        Message('done');
        Setup.Check('I x=0/NEW/I1;T x=0/NEW/I1;P x=0/NEW/I1;I after x=0/NEW/I1;');
    end;

    local procedure ValidateItem(var Line: Record "NBX Line"; CfgMode: Text)
    begin
        Cfg.Set(CfgMode);
        Line.Validate("Item No.", 'I2');
    end;

    local procedure ValidateMode(var Line: Record "NBX Line"; CfgMode: Text)
    begin
        Cfg.Set(CfgMode);
        Line.Validate(Mode, Line.Mode::Production);
    end;

    [MessageHandler]
    procedure MessageHandler(Msg: Text[1024])
    begin
    end;
}

// Scope Cloud
codeunit 50028 "NBX T28 ScopeCloud_Subscriber"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBX Cfg";
        Setup: Codeunit "NBX Setup";
        Caller: Codeunit "NBX Caller";

    [Test]
    [Scope('Cloud')]
    procedure ScopeCloud_Subscriber()
    var
        Line: Record "NBX Line";
    begin
        Setup.AfterInsert(Line);
        ValidateMode(Line, 'event');
        Setup.Check('T x=0/NEW/I1;P x=0/NEW/I1;');
    end;

    local procedure ValidateItem(var Line: Record "NBX Line"; CfgMode: Text)
    begin
        Cfg.Set(CfgMode);
        Line.Validate("Item No.", 'I2');
    end;

    local procedure ValidateMode(var Line: Record "NBX Line"; CfgMode: Text)
    begin
        Cfg.Set(CfgMode);
        Line.Validate(Mode, Line.Mode::Production);
    end;
}

// another attribute too
codeunit 50029 "NBX T29 ScopeOnPremAndOther_Su"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBX Cfg";
        Setup: Codeunit "NBX Setup";
        Caller: Codeunit "NBX Caller";

    [Test]
    [Scope('OnPrem')]
    [TransactionModel(TransactionModel::AutoRollback)]
    procedure ScopeOnPremAndOther_Subscriber()
    var
        Line: Record "NBX Line";
    begin
        Setup.AfterInsert(Line);
        ValidateMode(Line, 'event');
        Setup.Check('T x=0/NEW/I1;P x=0/NEW/I1;');
    end;

    local procedure ValidateItem(var Line: Record "NBX Line"; CfgMode: Text)
    begin
        Cfg.Set(CfgMode);
        Line.Validate("Item No.", 'I2');
    end;

    local procedure ValidateMode(var Line: Record "NBX Line"; CfgMode: Text)
    begin
        Cfg.Set(CfgMode);
        Line.Validate(Mode, Line.Mode::Production);
    end;
}

// the measured caller without Scope
codeunit 50030 "NBX T30 NoScope_Subscriber"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBX Cfg";
        Setup: Codeunit "NBX Setup";
        Caller: Codeunit "NBX Caller";

    [Test]
    procedure NoScope_Subscriber()
    var
        Line: Record "NBX Line";
    begin
        Setup.AfterInsert(Line);
        ValidateMode(Line, 'event');
        Setup.Check('T x=0/NEW/I1;P x=0/NEW/I1;');
    end;

    local procedure ValidateItem(var Line: Record "NBX Line"; CfgMode: Text)
    begin
        Cfg.Set(CfgMode);
        Line.Validate("Item No.", 'I2');
    end;

    local procedure ValidateMode(var Line: Record "NBX Line"; CfgMode: Text)
    begin
        Cfg.Set(CfgMode);
        Line.Validate(Mode, Line.Mode::Production);
    end;
}

// Issue 8311: the trigger's xRec is blank (a record no field was set on), the subscriber's table procedure assigns xRec. Not
// measured (handoffs: probe cell), so both binaries give main's blank copy.
codeunit 50031 "NBX T31 BlankXRec_ProcAssigns"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBX Cfg";
        Setup: Codeunit "NBX Setup";

    [Test]
    procedure BlankXRec_ProcAssigns()
    var
        Line: Record "NBX Line";
    begin
        Line.DeleteAll();
        Line.Init();
        ValidateMode(Line, 'blank-assign');
        Setup.Check('T x=0//;T after x=0/SET/;');
    end;

    local procedure ValidateMode(var Line: Record "NBX Line"; CfgMode: Text)
    begin
        Cfg.Set(CfgMode);
        Line.Validate(Mode, Line.Mode::Production);
    end;
}
