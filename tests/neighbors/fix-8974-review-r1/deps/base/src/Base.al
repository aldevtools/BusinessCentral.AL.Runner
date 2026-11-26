table 50400 "PX Line"
{
    DataClassification = CustomerContent;
    InherentPermissions = RIMD;
    InherentEntitlements = RIMD;

    fields
    {
        field(1; "Entry No."; Integer) { }
        // Has a table extension OnBeforeValidate (Ext.al): the one unmeasured fact of Chain_ExtOnBeforeValidateAssigns.
        field(2; Mode; Integer)
        {
            trigger OnValidate()
            var
                Cfg: Codeunit "PX Cfg";
                Worker: Codeunit "PX Worker";
            begin
                if Cfg.Get() = 'ext-before' then
                    Worker.CallTableProc(Rec);
            end;
        }
        field(3; Series; Code[20]) { }
        field(4; Note; Code[20]) { }
        field(5; Seen; Code[20]) { }
        field(6; Kind; Integer)
        {
            trigger OnValidate()
            var
                Cfg: Codeunit "PX Cfg";
            begin
                if Cfg.Get() = 'two-trg' then
                    Series := 'TRG';
            end;
        }
        field(7; "Line Count"; Integer)
        {
            FieldClass = FlowField;
            CalcFormula = count("PX Line");
        }
        // No table extension trigger and no subscriber: each test on it has one unmeasured writer.
        field(8; Plain; Integer)
        {
            trigger OnValidate()
            var
                Cfg: Codeunit "PX Cfg";
                Worker: Codeunit "PX Worker";
                Other: Record "PX Line";
                Card: Page "PX Card";
                RecRef: RecordRef;
                V: Variant;
                OS: OutStream;
                InS: InStream;
            begin
                case Cfg.Get() of
                    'p-ctrl':
                        Series := 'TRG';
                    'p-assign':
                        begin
                            Other := Rec;
                            Other.Series := 'RA';
                            Rec := Other;
                        end;
                    'p-variant':
                        begin
                            Other := Rec;
                            Other.Series := 'VA';
                            V := Other;
                            Rec := V;
                        end;
                    'p-calc':
                        CalcFields("Line Count");
                    'p-settable':
                        begin
                            RecRef.GetTable(Rec);
                            RecRef.Field(3).Value := 'ST';
                            RecRef.SetTable(Rec);
                        end;
                    'p-getrecord':
                        begin
                            Other := Rec;
                            Other.Series := 'GR';
                            Card.SetRecord(Other);
                            Card.GetRecord(Rec);
                        end;
                    'p-selfilter':
                        Card.SetSelectionFilter(Rec);
                    'p-calcsums':
                        begin
                            Other.Init();
                            Other."Entry No." := 2;
                            Other.Amount := 7;
                            Other.Insert();
                            CalcSums(Amount);
                        end;
                    'p-blob':
                        begin
                            Pic.CreateOutStream(OS);
                            OS.WriteText('X');
                        end;
                    'p-calcblob':
                        CalcFields(Pic);
                    'p-media':
                        begin
                            Other.Pic.CreateOutStream(OS);
                            OS.WriteText('X');
                            Other.Pic.CreateInStream(InS);
                            Img.ImportStream(InS, 'x');
                        end;
                    'p-mediaset':
                        begin
                            Other.Pic.CreateOutStream(OS);
                            OS.WriteText('X');
                            Other.Pic.CreateInStream(InS);
                            Imgs.ImportStream(InS, 'x');
                        end;
                    'p-evaluate':
                        Evaluate(Series, 'EV');
                end;
                Worker.CallTableProc(Rec);
            end;
        }
        // One OnAfterValidateEvent subscriber ("PX Sub C") and no table extension trigger.
        field(9; Single; Integer)
        {
            trigger OnValidate()
            var
                Cfg: Codeunit "PX Cfg";
            begin
                if Cfg.Get() in ['s-ctrl', 's-fieldref', 's-testpage', 's-action'] then
                    Series := 'TRG';
            end;
        }
        // Written in the trigger of Plain: by CalcSums (Plain_CalcSums) and by an OutStream (Plain_BlobStream).
        field(10; Amount; Decimal) { }
        field(11; Pic; Blob) { }
        // Written in the trigger of Plain by ImportStream (Plain_MediaImportStream, Plain_MediaSetImportStream).
        field(12; Img; Media) { }
        field(13; Imgs; MediaSet) { }
    }

    keys
    {
        key(PK; "Entry No.") { Clustered = true; }
    }

    procedure TableProcRead()
    begin
        Seen := xRec.Series;
    end;
}

codeunit 50401 "PX Cfg"
{
    SingleInstance = true;

    var
        CfgMode: Text;

    procedure Set(NewMode: Text)
    begin
        CfgMode := NewMode;
    end;

    procedure Get(): Text
    begin
        exit(CfgMode);
    end;
}

codeunit 50402 "PX Worker"
{
    procedure CallTableProc(var Line: Record "PX Line")
    begin
        Line.TableProcRead();
    end;
}

page 50403 "PX Card"
{
    PageType = Card;
    SourceTable = "PX Line";

    layout
    {
        area(Content)
        {
            field(Single; Rec.Single) { }
            field(Seen; Rec.Seen) { }
        }
    }

    actions
    {
        area(Processing)
        {
            action(ValidateSingle)
            {
                trigger OnAction()
                begin
                    Rec.Validate(Single, 1);
                end;
            }
        }
    }
}
