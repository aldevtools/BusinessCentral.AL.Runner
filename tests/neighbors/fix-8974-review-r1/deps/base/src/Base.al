table 50400 "PX Line"
{
    DataClassification = CustomerContent;
    InherentPermissions = RIMD;
    InherentEntitlements = RIMD;

    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; Mode; Integer)
        {
            trigger OnValidate()
            var
                Cfg: Codeunit "PX Cfg";
                Worker: Codeunit "PX Worker";
                Other: Record "PX Line";
            begin
                case Cfg.Get() of
                    'cu-direct':
                        begin
                            Series := 'TRG';
                            Worker.CallTableProc(Rec);
                        end;
                    'ext-before':
                        Worker.CallTableProc(Rec);
                    'assign-rec':
                        begin
                            Other := Rec;
                            Other.Series := 'RA';
                            Rec := Other;
                            Worker.CallTableProc(Rec);
                        end;
                    'calc':
                        begin
                            CalcFields("Line Count");
                            Worker.CallTableProc(Rec);
                        end;
                end;
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
                if Cfg.Get() in ['two-trg', 'fieldref', 'direct'] then
                    Series := 'TRG';
            end;
        }
        field(7; "Line Count"; Integer)
        {
            FieldClass = FlowField;
            CalcFormula = count("PX Line");
        }
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
