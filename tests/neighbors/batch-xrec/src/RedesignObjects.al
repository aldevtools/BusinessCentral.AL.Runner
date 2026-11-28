// Neighbors of the redesign (review round 3 of al-runner PR 9328): one xRec handle for a whole nest of Validates of one record
// variable (assumption xrec-one-across-nested-validate, corpus probe record/validate/XRecNestedValidateShapes.al, al-runner
// issue 9331). The trigger of "Item No." nests Validate(Mode) through two procedures of the table, Mode's may nest
// Validate(Grade), Grade's Validate(Deep). Each trigger logs its xRec as Mode ordinal/Series/"Item No.".
table 50150 "NBX Shape Line"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Line No."; Integer) { }
        field(2; Mode; Option)
        {
            OptionMembers = Purchase,Production;

            trigger OnValidate()
            var
                Cfg: Codeunit "NBX Cfg";
            begin
                if Cfg.Get() = '' then
                    exit;
                Cfg.Add('T x=' + Shape.Fmt(xRec));
                if Cfg.Get().EndsWith('inner-assign') then
                    xRec.Series := 'INN';
                if Cfg.Get().EndsWith('inner-filter') then
                    xRec.SetRange(Series, 'F');
                if Cfg.Get().EndsWith('inner-try') then
                    if not TryAssignXRec() then
                        Cfg.Add('caught');
                if Cfg.Get().StartsWith('two-level') or Cfg.Get().StartsWith('four-level') then begin
                    Validate(Grade, Grade::High);
                    Cfg.Add('T after x=' + Shape.Fmt(xRec));
                end;
            end;
        }
        field(3; Series; Code[20]) { }
        field(4; "Item No."; Code[20])
        {
            trigger OnValidate()
            var
                Cfg: Codeunit "NBX Cfg";
                Other: Record "NBX Shape Line";
                V: Variant;
            begin
                if Cfg.Get() = '' then
                    exit;
                Cfg.Add('I x=' + Shape.Fmt(xRec));
                Series := 'MID';
                if Cfg.Get().EndsWith('outer-assign') then
                    xRec.Series := 'OUT';
                case true of
                    Cfg.Get().StartsWith('two'), Cfg.Get().StartsWith('four-level'):
                        CopyFromItem();
                    Cfg.Get().StartsWith('ext'):
                        ExtUpdateMode();
                    Cfg.Get().StartsWith('variant'):
                        begin
                            V := Mode::Production;
                            Validate(Mode, V);
                        end;
                    Cfg.Get().StartsWith('other-copy'):
                        begin
                            Other := Rec;
                            Other.Validate(Mode, Other.Mode::Production);
                        end;
                end;
                Cfg.Add('I after x=' + Shape.Fmt(xRec));
                if Cfg.Get().EndsWith('filter') then
                    Cfg.Add('I filters=' + xRec.GetFilters());
            end;
        }
        field(5; Grade; Option)
        {
            OptionMembers = Low,High;

            trigger OnValidate()
            var
                Cfg: Codeunit "NBX Cfg";
            begin
                if Cfg.Get() = '' then
                    exit;
                Cfg.Add('G x=' + Shape.Fmt(xRec));
                if Cfg.Get().EndsWith('deep-assign') then
                    xRec.Series := 'GRD';
                if Cfg.Get().StartsWith('four-level') then
                    Validate(Deep, 1);
            end;
        }
        field(6; Deep; Integer)
        {
            trigger OnValidate()
            var
                Cfg: Codeunit "NBX Cfg";
            begin
                if Cfg.Get() <> '' then
                    Cfg.Add('D x=' + Shape.Fmt(xRec));
            end;
        }
    }

    keys
    {
        key(PK; "Line No.") { Clustered = true; }
    }

    var
        Shape: Codeunit "NBX Shape Setup";

    local procedure CopyFromItem()
    begin
        UpdateMode();
    end;

    local procedure UpdateMode()
    begin
        Validate(Mode, Mode::Production);
    end;

    [TryFunction]
    local procedure TryAssignXRec()
    begin
        xRec.Series := 'TRY';
        Error('try failed');
    end;
}

// The nested Validate in a procedure of a table extension: not measured.
tableextension 50151 "NBX Shape Ext" extends "NBX Shape Line"
{
    procedure ExtUpdateMode()
    begin
        Validate(Mode, Mode::Production);
    end;
}

codeunit 50152 "NBX Shape Setup"
{
    // The buffer of the probe: inserted by the record itself, Series changed after.
    procedure AfterInsert(var Line: Record "NBX Shape Line")
    begin
        Line.DeleteAll();
        Line.Init();
        Line."Line No." := 1;
        Line.Mode := Line.Mode::Purchase;
        Line.Series := 'OLD';
        Line."Item No." := 'I1';
        Line.Insert(false);
        Line.Series := 'NEW';
    end;

    procedure Fmt(Line: Record "NBX Shape Line"): Text
    begin
        exit(Format(Line.Mode, 0, 2) + '/' + Line.Series + '/' + Line."Item No.");
    end;
}
