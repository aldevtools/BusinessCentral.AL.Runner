// Copilot thread 4726130568 (train 9390): the outer trigger writes the whole record with RecordRef.SetTable before it nests.
codeunit 50310 "NBR T1 TriggerSetTable"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBR Cfg";

    [Test]
    procedure OuterTrigger_SetTableIntoRec()
    var
        Line: Record "NBR Line";
    begin
        Cfg.AfterInsert(Line);
        Cfg.Set('settable');
        Line.Validate("Item No.", 'I2');
        Cfg.Check('I x=0/NEW/I1;T x=0/NEW/I1;I after x=0/OUT/I1;');
    end;
}

// Copilot thread 4726130587 (train 9390): the outer Validate on a buffer from Get, whose trigger moves the key and inserts
// before it nests.
codeunit 50311 "NBR T2 TriggerReinsert"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBR Cfg";

    [Test]
    procedure OuterTrigger_InsertOnGetBuffer()
    var
        Line: Record "NBR Line";
        Line2: Record "NBR Line";
    begin
        Cfg.AfterInsert(Line);
        Line2.Get(1);
        Line2.Series := 'NEW';
        Cfg.Set('reinsert');
        Line2.Validate("Item No.", 'I2');
        Cfg.Check('I x=0/NEW/I1;T x=0/NEW/I1;I after x=0/OUT/I1;');
    end;
}

// Issue 9380 item 1: RecordRef.SetTable writes another record's fields into the buffer before the outer Validate.
codeunit 50312 "NBR T3 TestSetTable"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBR Cfg";

    [Test]
    procedure SetTableBeforeValidate()
    var
        Line: Record "NBR Line";
        Other: Record "NBR Line";
        RecRef: RecordRef;
    begin
        Cfg.AfterInsert(Line);
        Other := Line;
        Other.Series := 'SET';
        RecRef.GetTable(Other);
        RecRef.SetTable(Line);
        Cfg.Set('plain');
        Line.Validate("Item No.", 'I2');
        Cfg.Check('I x=0/SET/I1;T x=0/SET/I1;I after x=0/OUT/I1;');
    end;
}

// Issue 9380 item 1: Page.GetRecord writes another record's fields into the buffer before the outer Validate.
codeunit 50313 "NBR T4 TestGetRecord"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBR Cfg";

    [Test]
    procedure GetRecordBeforeValidate()
    var
        Line: Record "NBR Line";
        Other: Record "NBR Line";
        Card: Page "NBR Card";
    begin
        Cfg.AfterInsert(Line);
        Other := Line;
        Other.Series := 'PAG';
        Card.SetRecord(Other);
        Card.GetRecord(Line);
        Cfg.Set('plain');
        Line.Validate("Item No.", 'I2');
        Cfg.Check('I x=0/PAG/I1;T x=0/PAG/I1;I after x=0/OUT/I1;');
    end;
}

// Issue 9380 item 2: the [MessageHandler]'s buffer assigned from a Record.Copy of the variable that inserted.
codeunit 50314 "NBR T5 HandlerFromCopy"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBR Cfg";
        HandlerLine: Record "NBR Line";

    [Test]
    [HandlerFunctions('ValidateHandler')]
    procedure Handler_AssignedFromCopy()
    var
        Line: Record "NBR Line";
        Mid: Record "NBR Line";
    begin
        Cfg.AfterInsert(Line);
        Mid.Copy(Line);
        HandlerLine := Mid;
        Cfg.Set('plain');
        Message('validate');
        Cfg.Check('I x=0/NEW/I1;T x=0/NEW/I1;I after x=0/OUT/I1;');
    end;

    [MessageHandler]
    procedure ValidateHandler(Msg: Text[1024])
    begin
        HandlerLine.Validate("Item No.", 'I2');
    end;
}

// Issue 9380 item 2: the [MessageHandler]'s buffer assigned from a by-value parameter that took the variable that inserted.
codeunit 50315 "NBR T6 HandlerFromByValue"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBR Cfg";
        HandlerLine: Record "NBR Line";

    [Test]
    [HandlerFunctions('ValidateHandler')]
    procedure Handler_AssignedFromByValue()
    var
        Line: Record "NBR Line";
    begin
        Cfg.AfterInsert(Line);
        Take(Line);
        Cfg.Set('plain');
        Message('validate');
        Cfg.Check('I x=0/NEW/I1;T x=0/NEW/I1;I after x=0/OUT/I1;');
    end;

    local procedure Take(L: Record "NBR Line")
    begin
        HandlerLine := L;
    end;

    [MessageHandler]
    procedure ValidateHandler(Msg: Text[1024])
    begin
        HandlerLine.Validate("Item No.", 'I2');
    end;
}

// Issue 9380 item 2: the [MessageHandler]'s buffer assigned in an earlier test of the codeunit.
codeunit 50316 "NBR T7 HandlerFromEarlierTest"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBR Cfg";
        HandlerLine: Record "NBR Line";

    [Test]
    procedure A_AssignInFirstTest()
    var
        Line: Record "NBR Line";
    begin
        Cfg.AfterInsert(Line);
        HandlerLine := Line;
    end;

    [Test]
    [HandlerFunctions('ValidateHandler')]
    procedure B_Handler_AssignedInEarlierTest()
    begin
        Cfg.Set('plain');
        Message('validate');
        Cfg.Check('I x=0/NEW/I1;T x=0/NEW/I1;I after x=0/OUT/I1;');
    end;

    [MessageHandler]
    procedure ValidateHandler(Msg: Text[1024])
    begin
        HandlerLine.Validate("Item No.", 'I2');
    end;
}

// The gate's same-test fact on the Own arm: the buffer of a global variable whose Insert ran in an earlier test of the codeunit.
codeunit 50317 "NBR T8 OwnFromEarlierTest"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBR Cfg";
        Line: Record "NBR Line";

    [Test]
    procedure A_InsertInFirstTest()
    begin
        Cfg.AfterInsert(Line);
    end;

    [Test]
    procedure B_Own_InsertedInEarlierTest()
    begin
        Cfg.Set('plain');
        Line.Validate("Item No.", 'I2');
        Cfg.Check('I x=0/NEW/I1;T x=0/NEW/I1;I after x=0/OUT/I1;');
    end;
}
