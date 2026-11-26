// Neighbors of al-runner issue 8952: a parent stored before its linked child rows whose first save was left out.
table 50300 "N Src" { DataClassification = SystemMetadata; fields { field(1; Id; Integer) { } field(2; Data; Blob) { } } keys { key(PK; Id) { Clustered = true; } } }
table 50301 "N Suite" { DataClassification = SystemMetadata; fields { field(1; Code; Code[20]) { } field(2; Description; Text[50]) { } field(3; AppId; Text[30]) { } } keys { key(PK; Code) { Clustered = true; } } }
table 50302 "N Line" { DataClassification = SystemMetadata; fields { field(1; SuiteCode; Code[20]) { } field(2; Cu; Integer) { } } keys { key(PK; SuiteCode, Cu) { Clustered = true; } } }
xmlport 50310 "N SkipBeforePlain"
{
    Direction = Import;
    Format = Xml;
    Encoding = UTF8;
    UseRequestPage = false;
    schema
    {
        textelement(Root)
        {
            tableelement(Suite; "N Suite")
            {
                MaxOccurs = Unbounded;
                XmlName = 'Suite';
                fieldattribute(Code; Suite.Code)
                {
                    Occurrence = Required;
                }
                fieldattribute(Description; Suite.Description) { Occurrence = Optional; }
                tableelement(Line; "N Line")
                {
                    LinkTable = Suite;
                    LinkFields = SuiteCode = field(Code);
                    MinOccurs = Zero;
                    XmlName = 'Line';
                    fieldattribute(Cu; Line.Cu) { Occurrence = Required; }

                    trigger OnAfterInitRecord()
                    var
                        Flag: Codeunit "N Flag";
                    begin
                        if Flag.SkipLines() then
                            currXMLport.Skip();
                    end;
                }
                trigger OnBeforeInsertRecord()
                var
                    Other: Record "N Suite";
                begin
                    if Other.Get(Suite.Code) then currXMLport.Skip();
                end;
            }
        }
    }

}
xmlport 50311 "N SkipBeforeAssign"
{
    Direction = Import;
    Format = Xml;
    Encoding = UTF8;
    UseRequestPage = false;
    schema
    {
        textelement(Root)
        {
            tableelement(Suite; "N Suite")
            {
                MaxOccurs = Unbounded;
                XmlName = 'Suite';
                fieldattribute(Code; Suite.Code)
                {
                    Occurrence = Required;
                    trigger OnAfterAssignField()
                    var
                        Other: Record "N Suite";
                    begin
                        Suite.AppId := 'app1';
                    end;
                }
                fieldattribute(Description; Suite.Description) { Occurrence = Optional; }
                tableelement(Line; "N Line")
                {
                    LinkTable = Suite;
                    LinkFields = SuiteCode = field(Code);
                    MinOccurs = Zero;
                    XmlName = 'Line';
                    fieldattribute(Cu; Line.Cu) { Occurrence = Required; }

                    trigger OnAfterInitRecord()
                    var
                        Flag: Codeunit "N Flag";
                    begin
                        if Flag.SkipLines() then
                            currXMLport.Skip();
                    end;
                }
                trigger OnBeforeInsertRecord()
                var
                    Other: Record "N Suite";
                begin
                    if Other.Get(Suite.Code) then currXMLport.Skip();
                end;
            }
        }
    }

}
xmlport 50312 "N UnboundBefore"
{
    Direction = Import;
    Format = Xml;
    Encoding = UTF8;
    UseRequestPage = false;
    schema
    {
        textelement(Root)
        {
            tableelement(Suite; "N Suite")
            {
                MaxOccurs = Unbounded;
                XmlName = 'Suite';
                fieldattribute(Code; Suite.Code)
                {
                    Occurrence = Required;
                }
                fieldattribute(Description; Suite.Description) { Occurrence = Optional; }
                tableelement(Line; "N Line")
                {
                    LinkTable = Suite;
                    LinkFields = SuiteCode = field(Code);
                    MinOccurs = Zero;
                    XmlName = 'Line';
                    fieldattribute(Cu; Line.Cu) { Occurrence = Required; }

                    trigger OnAfterInitRecord()
                    var
                        Flag: Codeunit "N Flag";
                    begin
                        if Flag.SkipLines() then
                            currXMLport.Skip();
                    end;
                }
                trigger OnBeforeInsertRecord()
                var
                    Other: Record "N Suite";
                begin
                    if Other.Get(Suite.Code) then currXMLport.BreakUnbound();
                end;
            }
        }
    }

}
xmlport 50313 "N SkipBeforeNew"
{
    Direction = Import;
    Format = Xml;
    Encoding = UTF8;
    UseRequestPage = false;
    schema
    {
        textelement(Root)
        {
            tableelement(Suite; "N Suite")
            {
                MaxOccurs = Unbounded;
                XmlName = 'Suite';
                fieldattribute(Code; Suite.Code)
                {
                    Occurrence = Required;
                }
                fieldattribute(Description; Suite.Description) { Occurrence = Optional; }
                tableelement(Line; "N Line")
                {
                    LinkTable = Suite;
                    LinkFields = SuiteCode = field(Code);
                    MinOccurs = Zero;
                    XmlName = 'Line';
                    fieldattribute(Cu; Line.Cu) { Occurrence = Required; }

                    trigger OnAfterInitRecord()
                    var
                        Flag: Codeunit "N Flag";
                    begin
                        if Flag.SkipLines() then
                            currXMLport.Skip();
                    end;
                }
                trigger OnBeforeInsertRecord()
                var
                    Other: Record "N Suite";
                begin
                    currXMLport.Skip();
                end;
            }
        }
    }

}
xmlport 50314 "N AttrSkipChild"
{
    Direction = Import;
    Format = Xml;
    Encoding = UTF8;
    UseRequestPage = false;
    schema
    {
        textelement(Root)
        {
            tableelement(Suite; "N Suite")
            {
                MaxOccurs = Unbounded;
                XmlName = 'Suite';
                fieldattribute(Code; Suite.Code)
                {
                    Occurrence = Required;
                    trigger OnAfterAssignField()
                    var
                        Other: Record "N Suite";
                    begin
                        currXMLport.Skip();
                    end;
                }
                fieldattribute(Description; Suite.Description) { Occurrence = Optional; }
                tableelement(Line; "N Line")
                {
                    LinkTable = Suite;
                    LinkFields = SuiteCode = field(Code);
                    MinOccurs = Zero;
                    XmlName = 'Line';
                    fieldattribute(Cu; Line.Cu) { Occurrence = Required; }

                    trigger OnAfterInitRecord()
                    var
                        Flag: Codeunit "N Flag";
                    begin
                        if Flag.SkipLines() then
                            currXMLport.Skip();
                    end;
                }
            }
        }
    }

}
xmlport 50315 "N SkipBothAssign"
{
    Direction = Import;
    Format = Xml;
    Encoding = UTF8;
    UseRequestPage = false;
    schema
    {
        textelement(Root)
        {
            tableelement(Suite; "N Suite")
            {
                MaxOccurs = Unbounded;
                XmlName = 'Suite';
                fieldattribute(Code; Suite.Code)
                {
                    Occurrence = Required;
                    trigger OnAfterAssignField()
                    var
                        Other: Record "N Suite";
                    begin
                        if Other.Get(Suite.Code) then currXMLport.Skip(); Suite.AppId := 'app1';
                    end;
                }
                fieldattribute(Description; Suite.Description) { Occurrence = Optional; }
                tableelement(Line; "N Line")
                {
                    LinkTable = Suite;
                    LinkFields = SuiteCode = field(Code);
                    MinOccurs = Zero;
                    XmlName = 'Line';
                    fieldattribute(Cu; Line.Cu) { Occurrence = Required; }

                    trigger OnAfterInitRecord()
                    var
                        Flag: Codeunit "N Flag";
                    begin
                        if Flag.SkipLines() then
                            currXMLport.Skip();
                    end;
                }
                trigger OnBeforeInsertRecord()
                var
                    Other: Record "N Suite";
                begin
                    if Other.Get(Suite.Code) then currXMLport.Skip();
                end;
            }
        }
    }

}

codeunit 50351 "N Flag"
{
    SingleInstance = true;

    var
        Skip: Boolean;

    procedure SetSkipLines(Value: Boolean)
    begin
        Skip := Value;
    end;

    procedure SkipLines(): Boolean
    begin
        exit(Skip);
    end;
}

codeunit 50350 "N Lib"
{
    procedure Run(PortId: Integer; Xml: Text)
    var
        Src: Record "N Src";
        InS: InStream;
        OutS: OutStream;
    begin
        Src.DeleteAll();
        Src.Id := 1;
        Src.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        OutS.WriteText(Xml);
        Src.Insert();
        Src.CalcFields(Data);
        Src.Data.CreateInStream(InS, TextEncoding::UTF8);
        Xmlport.Import(PortId, InS);
    end;

    // Seeds suite A with description da, then imports A with description db and one line (Cu 6).
    procedure Again(PortId: Integer; SkipLines: Boolean): Text
    var
        Suite: Record "N Suite";
        Line: Record "N Line";
        Flag: Codeunit "N Flag";
    begin
        Suite.DeleteAll();
        Line.DeleteAll();
        Suite.Code := 'A';
        Suite.Description := 'da';
        Suite.Insert();
        Flag.SetSkipLines(SkipLines);
        Run(PortId, '<Root><Suite Code="A" Description="db"><Line Cu="6"/></Suite></Root>');
        exit(Rows());
    end;

    procedure Rows(): Text
    var
        Suite: Record "N Suite";
        Line: Record "N Line";
        R: Text;
    begin
        if Suite.FindSet() then
            repeat
                R += Suite.Code + '/' + Suite.Description + '/' + Suite.AppId + ';';
            until Suite.Next() = 0;
        if Line.FindSet() then
            repeat
                R += 'L' + Line.SuiteCode + '/' + Format(Line.Cu) + ';';
            until Line.Next() = 0;
        exit(R);
    end;
}
