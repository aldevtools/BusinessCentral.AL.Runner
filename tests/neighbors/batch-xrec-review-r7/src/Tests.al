// The Copilot shape: Insert, a guarded Insert that answers false, then HandlerLine := Line; the handler validates HandlerLine.
codeunit 50310 "NBR T1 HandlerFailedInsert"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBR Cfg";
        HandlerLine: Record "NBR Line";

    [Test]
    [HandlerFunctions('ValidateHandler')]
    procedure Handler_AssignedAfterFailedGuardedInsert()
    var
        Line: Record "NBR Line";
    begin
        Cfg.AfterInsert(Line);
        if not Line.Insert(false) then;
        HandlerLine := Line;
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

// The inherited mark through a second copy: the failed Insert on the variable that inserted, Mid := Line, HandlerLine := Mid.
codeunit 50311 "NBR T2 HandlerTwoAssigns"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBR Cfg";
        HandlerLine: Record "NBR Line";

    [Test]
    [HandlerFunctions('ValidateHandler')]
    procedure Handler_TwoAssignsAfterFailedGuardedInsert()
    var
        Line: Record "NBR Line";
        Mid: Record "NBR Line";
    begin
        Cfg.AfterInsert(Line);
        if not Line.Insert(false) then;
        Mid := Line;
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

// The validated buffer's own guarded Insert answers false after the := (the counter of the copy itself).
codeunit 50312 "NBR T3 HandlerOwnFailed"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBR Cfg";
        HandlerLine: Record "NBR Line";

    [Test]
    [HandlerFunctions('ValidateHandler')]
    procedure Handler_OwnFailedGuardedInsertAfterAssign()
    var
        Line: Record "NBR Line";
    begin
        Cfg.AfterInsert(Line);
        HandlerLine := Line;
        if not HandlerLine.Insert(false) then;
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
