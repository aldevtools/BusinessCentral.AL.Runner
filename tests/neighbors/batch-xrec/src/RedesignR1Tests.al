// Fix round 1 of the redesign. Each shape keeps main's two copies: the gate of assumption xrec-one-across-nested-validate
// does not admit it (no probe cell measures it).

// the outer Validate without a value: "Item No." assigned, then Validate("Item No.") (every probe cell hands a value)
codeunit 50172 "NBX T72 OuterOneArgValidate"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBX Cfg";
        Setup: Codeunit "NBX Setup";

    [Test]
    procedure OuterOneArgValidate_TwoProcs()
    var
        Line: Record "NBX Line";
    begin
        Setup.AfterInsert(Line);
        ValidateItem(Line, 'outer-assign');
        Setup.Check('I x=0/NEW/I2;T x=0/NEW/I2;I after x=0/OUT/I2;');
    end;

    local procedure ValidateItem(var Line: Record "NBX Line"; CfgMode: Text)
    begin
        Cfg.Set(CfgMode);
        Line."Item No." := 'I2';
        Line.Validate("Item No.");
    end;
}

// the buffer's own Insert, then its own Rename (the probe's buffer has no write after its Insert)
codeunit 50173 "NBX T73 RenameAfterInsert"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBX Cfg";
        Setup: Codeunit "NBX Setup";

    [Test]
    procedure OuterAssign_RenameAfterInsert()
    var
        Line: Record "NBX Line";
    begin
        Setup.AfterInsert(Line);
        Line.Rename(2);
        ValidateItem(Line, 'outer-assign');
        Setup.Check('I x=0/NEW/I1;T x=0/NEW/I1;I after x=0/OUT/I1;');
    end;

    local procedure ValidateItem(var Line: Record "NBX Line"; CfgMode: Text)
    begin
        Cfg.Set(CfgMode);
        Line.Validate("Item No.", 'I2');
    end;
}

// a buffer assigned from another variable that inserted (the probe validates the variable that inserted)
codeunit 50174 "NBX T74 AssignedFromInserted"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBX Cfg";
        Setup: Codeunit "NBX Setup";

    [Test]
    procedure OuterAssign_AssignedFromInserted()
    var
        Line: Record "NBX Line";
        Line2: Record "NBX Line";
    begin
        Setup.AfterInsert(Line);
        Line2 := Line;
        ValidateItem(Line2, 'outer-assign');
        Setup.Check('I x=0/NEW/I1;T x=0/NEW/I1;I after x=0/OUT/I1;');
    end;

    local procedure ValidateItem(var Line: Record "NBX Line"; CfgMode: Text)
    begin
        Cfg.Set(CfgMode);
        Line.Validate("Item No.", 'I2');
    end;
}

// the outer Validate from a [MessageHandler] on a buffer assigned from the variable that inserted (probe
// Validate_FromMessageHandler_OneXRec), the outer trigger assigning Rec := Other before it nests (no cell)
codeunit 50175 "NBX T75 HandlerRecAssigned"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBX Cfg";
        Setup: Codeunit "NBX Setup";
        Shape: Codeunit "NBX Shape Setup";
        HandlerLine: Record "NBX Shape Line";

    [Test]
    [HandlerFunctions('ValidateHandler')]
    procedure FromMessageHandler_TriggerAssignsRec()
    var
        Line: Record "NBX Shape Line";
    begin
        Shape.AfterInsert(Line);
        Cfg.Set('two-self-assign-outer-assign');
        HandlerLine := Line;
        Message('validate');
        Setup.Check('I x=0/NEW/I1;T x=0/NEW/I1;I after x=0/OUT/I1;');
    end;

    [MessageHandler]
    procedure ValidateHandler(Msg: Text[1024])
    begin
        HandlerLine.Validate("Item No.", 'I2');
    end;
}

// the outer Validate from the test on the variable that inserted, the outer trigger assigning Rec := Other before it nests
// (no cell)
codeunit 50176 "NBX T76 TestRecAssigned"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBX Cfg";
        Setup: Codeunit "NBX Setup";
        Shape: Codeunit "NBX Shape Setup";

    [Test]
    procedure FromTest_TriggerAssignsRec()
    var
        Line: Record "NBX Shape Line";
    begin
        Shape.AfterInsert(Line);
        Cfg.Set('two-self-assign-outer-assign');
        Line.Validate("Item No.", 'I2');
        Setup.Check('I x=0/NEW/I1;T x=0/NEW/I1;I after x=0/OUT/I1;');
    end;
}
