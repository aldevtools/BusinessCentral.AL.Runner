// Finding 2: a guarded Insert in the outer trigger answers false (the key exists) before it nests.
codeunit 50310 "NBR T1 TriggerFailedInsert"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBR Cfg";

    [Test]
    procedure OuterTrigger_FailedGuardedInsert()
    var
        Line: Record "NBR Line";
    begin
        Cfg.AfterInsert(Line);
        Cfg.Set('failinsert');
        Line.Validate("Item No.", 'I2');
        Cfg.Check('I x=0/NEW/I1;T x=0/NEW/I1;I after x=0/OUT/I1;');
    end;
}

// Finding 2: a guarded Insert answers false (the key exists) between the Insert and the outer Validate.
codeunit 50311 "NBR T2 TestFailedInsert"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBR Cfg";

    [Test]
    procedure FailedGuardedInsertBeforeValidate()
    var
        Line: Record "NBR Line";
    begin
        Cfg.AfterInsert(Line);
        if not Line.Insert(false) then;
        Cfg.Set('plain');
        Line.Validate("Item No.", 'I2');
        Cfg.Check('I x=0/NEW/I1;T x=0/NEW/I1;I after x=0/OUT/I1;');
    end;
}

// Finding 3: Page.SetSelectionFilter writes the selected row's fields into the buffer before the outer Validate.
codeunit 50312 "NBR T3 TestSelectionFilter"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBR Cfg";
        Card: Page "NBR Card";
        Line: Record "NBR Line";

    [Test]
    [HandlerFunctions('PickHandler')]
    procedure SetSelectionFilterBeforeValidate()
    begin
        Cfg.AfterInsert(Line);
        Card.RunModal();
        Cfg.Set('plain');
        Line.Validate("Item No.", 'I2');
        Cfg.Check('I x=0/OLD/I1;T x=0/OLD/I1;I after x=0/OUT/I1;');
    end;

    [ModalPageHandler]
    procedure PickHandler(var Page: TestPage "NBR Card")
    begin
        Card.Pick(Line);
    end;
}
