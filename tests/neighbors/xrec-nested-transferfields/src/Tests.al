// TransferFields from a record of the same table, without its key, before the outer Validate: the buffer keeps its version.
codeunit 50310 "NBR T1 TestTransferSameTable"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBR Cfg";

    [Test]
    procedure TransferFieldsSameTableBeforeValidate()
    var
        Line: Record "NBR Line";
        Other: Record "NBR Line";
    begin
        Cfg.AfterInsert(Line);
        Other := Line;
        Other.Series := 'TRF';
        Line.TransferFields(Other, false);
        Cfg.Set('plain');
        Line.Validate("Item No.", 'I2');
        Cfg.Check('I x=0/TRF/I1;T x=0/TRF/I1;I after x=0/OUT/I1;');
    end;
}

// TransferFields into Rec in the outer trigger, before it nests.
codeunit 50311 "NBR T2 TriggerTransfer"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBR Cfg";

    [Test]
    procedure OuterTrigger_TransferFieldsIntoRec()
    var
        Line: Record "NBR Line";
    begin
        Cfg.AfterInsert(Line);
        Cfg.Set('transfer');
        Line.Validate("Item No.", 'I2');
        Cfg.Check('I x=0/NEW/I1;T x=0/NEW/I1;I after x=0/OUT/I1;');
    end;
}

// TransferFields from a record of another table before the outer Validate: the buffer keeps its version.
codeunit 50312 "NBR T3 TestTransferOtherTable"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBR Cfg";

    [Test]
    procedure TransferFieldsOtherTableBeforeValidate()
    var
        Line: Record "NBR Line";
        Src: Record "NBR Src";
    begin
        Cfg.AfterInsert(Line);
        Src."Line No." := 9;
        Src.Series := 'SRC';
        Line.TransferFields(Src, false);
        Cfg.Set('plain');
        Line.Validate("Item No.", 'I2');
        Cfg.Check('I x=0/SRC/I1;T x=0/SRC/I1;I after x=0/OUT/I1;');
    end;
}

// TransferFields from a record of the same table with its key before the outer Validate: the buffer drops its version.
codeunit 50313 "NBR T4 TestTransferWithKey"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBR Cfg";

    [Test]
    procedure TransferFieldsSameTableWithKeyBeforeValidate()
    var
        Line: Record "NBR Line";
        Other: Record "NBR Line";
    begin
        Cfg.AfterInsert(Line);
        Other := Line;
        Other.Series := 'TPK';
        Line.TransferFields(Other);
        Cfg.Set('plain');
        Line.Validate("Item No.", 'I2');
        Cfg.Check('I x=0/TPK/I1;T x=0/TPK/I1;I after x=0/OUT/I1;');
    end;
}
