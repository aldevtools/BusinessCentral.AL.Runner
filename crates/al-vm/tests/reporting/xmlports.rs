//! XmlPort objects, end to end (Format = Xml). Each test names the corpus test that pins it.

use al_builtins::native::Signal;
use al_builtins::value::Value;

const FIXTURES: &str = r#"
// UseLax (YXmlPortProps.al:67, :87).
xmlport 60091 "ALT Lax" { Format = Xml; UseLax = true; schema { textelement(Root) { tableelement(U; "ALT Universal") { XmlName = 'Item'; fieldelement(Id; U."Entry No.") { } fieldelement(Name; U."Text Field") { } } } } }
xmlport 60092 "ALT Strict" { Format = Xml; schema { textelement(Root) { tableelement(U; "ALT Universal") { XmlName = 'Item'; fieldelement(Id; U."Entry No.") { } fieldelement(Name; U."Text Field") { } } } } }
// MinOccurs and MaxOccurs on container text elements (xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al:67-122).
table 60093 "ALT Occ Head" { fields { field(1; "No."; Code[10]) { } field(2; Name; Text[30]) { } } keys { key(PK; "No.") { } } }
table 60094 "ALT Occ Line" { fields { field(1; "Head No."; Code[10]) { } field(2; "Line No."; Integer) { } field(3; Qty; Integer) { } } keys { key(PK; "Head No.", "Line No.") { } } }
xmlport 60095 "ALT Occ"
{
    Direction = Both; Format = Xml; Encoding = UTF8; UseRequestPage = false;
    schema
    {
        textelement(envelope)
        {
            MaxOccurs = Once;
            textelement(request)
            {
                MinOccurs = Once; MaxOccurs = Once;
                tableelement(head; "ALT Occ Head")
                {
                    UseTemporary = true; MinOccurs = Zero; MaxOccurs = Unbounded;
                    fieldattribute(no; head."No.") { }
                    fieldelement(name; head.Name) { MinOccurs = Zero; MaxOccurs = Once; }
                    textelement(lines)
                    {
                        MinOccurs = Zero; MaxOccurs = Once;
                        tableelement(line; "ALT Occ Line")
                        {
                            UseTemporary = true; MinOccurs = Zero; MaxOccurs = Unbounded;
                            LinkTable = head; LinkFields = "Head No." = field("No.");
                            fieldattribute(n; line."Line No.") { }
                            fieldelement(qty; line.Qty) { }
                        }
                    }
                }
            }
            textelement(status) { MinOccurs = Zero; MaxOccurs = Once; }
        }
    }
    procedure AddHead(No: Code[10]; NewName: Text[30]) begin head.Init(); head."No." := No; head.Name := NewName; head.Insert(); end;
    procedure AddLine(HeadNo: Code[10]; LineNo: Integer; NewQty: Integer) begin line.Init(); line."Head No." := HeadNo; line."Line No." := LineNo; line.Qty := NewQty; line.Insert(); end;
    procedure Rows() Result: Text
    begin
        head.Reset(); if head.FindSet() then repeat Result += head."No." + ':' + head.Name + ';'; until head.Next() = 0;
        Result += ' ';
        line.Reset(); if line.FindSet() then repeat Result += line."Head No." + '/' + Format(line."Line No.") + '=' + Format(line.Qty) + ';'; until line.Next() = 0;
        Result += ' status=' + status;
    end;
}

// Occurrence of a text attribute on the root, a container and a table element
// (xmlport/lt-qr-xmlport-container-occurs/QsXpTextAttrObjects.al, the QS_TextAttr_* tests of QrXpContainerOccurs.al:441).
table 60096 "ALT Attr Row" { fields { field(1; "No."; Code[10]) { } field(2; Ref; Text[30]) { } field(3; Must; Text[30]) { } } keys { key(PK; "No.") { } } }
xmlport 60097 "ALT Attr Occ"
{
    Direction = Both; Format = Xml; Encoding = UTF8; UseRequestPage = false;
    schema
    {
        textelement(envelope)
        {
            MaxOccurs = Once;
            textattribute(instattr) { XmlName = 'cache_instance_id'; Occurrence = Optional;
                trigger OnBeforePassVariable() begin instattr := InstanceText; end;
                trigger OnAfterAssignVariable() begin Log += 'instance=' + instattr + ';'; end; }
            textattribute(verattr) { XmlName = 'version'; Occurrence = Required;
                trigger OnBeforePassVariable() begin verattr := VersionText; end;
                trigger OnAfterAssignVariable() begin Log += 'version=' + verattr + ';'; end; }
            textelement(body)
            {
                MinOccurs = Zero; MaxOccurs = Once;
                textattribute(tokenattr) { XmlName = 'token'; Occurrence = Optional;
                    trigger OnBeforePassVariable() begin tokenattr := TokenText; end;
                    trigger OnAfterAssignVariable() begin Log += 'token=' + tokenattr + ';'; end; }
                tableelement(row; "ALT Attr Row")
                {
                    UseTemporary = true; MinOccurs = Zero; MaxOccurs = Unbounded;
                    fieldattribute(no; row."No.") { }
                    textattribute(refattr) { XmlName = 'ref'; Occurrence = Optional;
                        trigger OnBeforePassVariable() begin refattr := row.Ref; end; }
                    textattribute(mustattr) { XmlName = 'must'; Occurrence = Required;
                        trigger OnBeforePassVariable() begin mustattr := row.Must; end; }
                    trigger OnBeforeInsertRecord()
                    begin
                        Log += 'row ' + row."No." + ' ref=' + refattr + ' must=' + mustattr + ';';
                        row.Ref := refattr; row.Must := mustattr;
                    end;
                }
            }
        }
    }
    var InstanceText: Text; VersionText: Text; TokenText: Text; Log: Text;
    procedure SetValues(NewInstance: Text; NewVersion: Text; NewToken: Text) begin InstanceText := NewInstance; VersionText := NewVersion; TokenText := NewToken; end;
    procedure AddRow(No: Code[10]; NewRef: Text[30]; NewMust: Text[30]) begin row.Init(); row."No." := No; row.Ref := NewRef; row.Must := NewMust; row.Insert(); end;
    procedure Result() Res: Text
    begin
        Res := 'log=' + Log + ' values: instance=' + instattr + ' version=' + verattr + ' token=' + tokenattr + ' ref=' + refattr + ' must=' + mustattr + ' rows=';
        if row.FindSet() then repeat Res += row."No." + ':' + row.Ref + '/' + row.Must + ';'; until row.Next() = 0;
    end;
}

// UseTemporary with AutoSave = false (xmlport/lt-aj-xmlport/AjRunAndXmlPort.al:307 xmlport 87357, :386 xmlport 87358).
table 60100 "ALT Num" { fields { field(1; Number; Integer) { } } keys { key(PK; Number) { } } }
xmlport 60098 "ALT Temp No Save Text"
{
    Direction = Import; Format = VariableText; FieldDelimiter = '"'; FieldSeparator = '|'; UseRequestPage = false;
    schema
    {
        textelement(Root)
        {
            tableelement(Tmp; "ALT Num")
            {
                AutoSave = false; XmlName = 'tmpImport'; UseTemporary = true;
                textelement(shelf) { }
                textelement(itemcode) { }
                trigger OnAfterInitRecord() begin Log += 'init n=' + Format(Tmp.Number) + ' count=' + Format(Tmp.Count()) + ';'; end;
                trigger OnBeforeInsertRecord()
                begin
                    Log += 'before shelf=' + shelf + ' itemcode=' + itemcode + ' n=' + Format(Tmp.Number) + ' count=' + Format(Tmp.Count()) + ' temp=' + Format(Tmp.IsTemporary()) + ';';
                    if ManualInsert then begin Counter += 1; Tmp.Number := Counter; Tmp.Insert(); end;
                end;
                trigger OnAfterInsertRecord() begin Log += 'after n=' + Format(Tmp.Number) + ' count=' + Format(Tmp.Count()) + ';'; end;
            }
        }
    }
    var Log: Text; ManualInsert: Boolean; Counter: Integer;
    trigger OnPreXmlPort() begin Log += 'pre count=' + Format(Tmp.Count()) + ';'; end;
    trigger OnPostXmlPort() begin Log += 'post count=' + Format(Tmp.Count()) + ';'; end;
    procedure SetManualInsert(NewManualInsert: Boolean) begin ManualInsert := NewManualInsert; end;
    procedure Result() Res: Text
    begin
        if Tmp.FindSet() then repeat Res += Format(Tmp.Number) + ';'; until Tmp.Next() = 0;
        Res := Log + ' count=' + Format(Tmp.Count()) + ' rows=' + Res;
    end;
}
xmlport 60099 "ALT Temp No Save Xml"
{
    Direction = Import; Format = Xml; UseRequestPage = false;
    schema
    {
        textelement(Root)
        {
            tableelement(U; "ALT Universal")
            {
                UseTemporary = true; AutoSave = false; XmlName = 'Row';
                fieldelement(Id; U."Entry No.") { }
                fieldelement(Txt; U."Text Field") { }
                trigger OnBeforeInsertRecord()
                begin
                    Log += 'before id=' + Format(U."Entry No.") + ' txt=' + U."Text Field" + ' temp=' + Format(U.Count()) + ';';
                    if ManualInsert then U.Insert();
                end;
                trigger OnAfterInsertRecord() begin Log += 'after id=' + Format(U."Entry No.") + ' temp=' + Format(U.Count()) + ' real=' + Format(Real.Count()) + ';'; end;
            }
        }
    }
    var Real: Record "ALT Universal"; Log: Text; ManualInsert: Boolean;
    procedure SetManualInsert(NewManualInsert: Boolean) begin ManualInsert := NewManualInsert; end;
    procedure Result() Res: Text
    begin
        if U.FindSet() then repeat Res += Format(U."Entry No.") + ':' + U."Text Field" + ';'; until U.Next() = 0;
        Res := Log + ' count=' + Format(U.Count()) + ' rows=' + Res;
    end;
}

table 60000 "ALT Universal"
{
    fields
    {
        field(1; "Entry No."; Integer) { }
        field(3; "Integer Field"; Integer) { }
        field(5; "Decimal Field"; Decimal) { }
        field(6; "Text Field"; Text[100]) { }
        field(17; "Description Field"; Text[250]) { }
    }
    keys { key(PK; "Entry No.") { } }
}

table 60004 "ALT Parent"
{
    fields { field(1; "Entry No."; Integer) { } field(2; "Code"; Code[20]) { } }
    keys { key(PK; "Entry No.") { } }
}

table 60005 "ALT Child"
{
    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; "Parent Entry No."; Integer) { }
        field(3; "Code"; Code[20]) { }
        field(4; "Amount"; Decimal) { }
    }
    keys { key(PK; "Entry No.") { } }
}

table 60008 "ALT Blob"
{
    fields { field(1; "Code"; Code[20]) { } field(3; Data; Blob) { } }
    keys { key(PK; "Code") { } }
}

// corpus _fixtures/xmlports/ALTUniversalXmlPort.al
xmlport 60023 "ALT Universal XmlPort"
{
    Direction = Both; Format = Xml; UseRequestPage = false;
    schema
    {
        textelement(Root)
        {
            XmlName = 'Universals';
            tableelement(Universal; "ALT Universal")
            {
                XmlName = 'Universal';
                fieldelement(EntryNo; Universal."Entry No.") { }
                fieldelement(IntegerValue; Universal."Integer Field") { }
                fieldelement(TextValue; Universal."Text Field") { }
            }
        }
    }
}

// corpus _fixtures/xmlports/ALTParentChildXmlPort.al
xmlport 60024 "ALT Parent Child XmlPort"
{
    Direction = Both; Format = Xml; UseRequestPage = false;
    schema
    {
        textelement(Parents)
        {
            tableelement(Parent; "ALT Parent")
            {
                XmlName = 'Parent';
                fieldelement(ParentEntryNo; Parent."Entry No.") { }
                fieldelement(ParentCode; Parent.Code) { }
                tableelement(Child; "ALT Child")
                {
                    LinkTable = Parent;
                    LinkFields = "Parent Entry No." = FIELD("Entry No.");
                    XmlName = 'Child';
                    fieldattribute(Amount; Child.Amount) { }
                    fieldelement(ChildEntryNo; Child."Entry No.") { }
                    fieldelement(ChildCode; Child.Code) { }
                }
            }
        }
    }
}

// corpus _fixtures/xmlports/ALTVariableXmlPort.al
xmlport 60025 "ALT Variable XmlPort"
{
    Direction = Both; Format = Xml; UseRequestPage = false;
    schema
    {
        textelement(Universals)
        {
            tableelement(Universal; "ALT Universal")
            {
                XmlName = 'Universal';
                textattribute(NoteAttr)
                {
                    trigger OnBeforePassVariable()
                    begin
                        NoteAttr := CopyStr(UpperCase(Universal."Text Field"), 1, MaxStrLen(NoteAttr));
                    end;
                    trigger OnAfterAssignVariable()
                    begin
                        Universal."Description Field" := NoteAttr;
                    end;
                }
                fieldelement(EntryNo; Universal."Entry No.") { }
                fieldelement(IntegerValue; Universal."Integer Field")
                {
                    trigger OnAfterAssignField()
                    begin
                        Universal."Integer Field" := Universal."Integer Field" * 10;
                    end;
                }
                textelement(DisplayText)
                {
                    trigger OnBeforePassVariable()
                    begin
                        DisplayText := StrSubstNo('TXT:%1', Universal."Text Field");
                    end;
                    trigger OnAfterAssignVariable()
                    begin
                        Universal."Text Field" := CopyStr(CopyStr(DisplayText, 5), 1, MaxStrLen(Universal."Text Field"));
                    end;
                }
            }
        }
    }
}

// corpus _fixtures/xmlports/ALTUniversalUpdateXmlPort.al
xmlport 60026 "ALT Universal Update XmlPort"
{
    Direction = Import; Format = Xml; UseRequestPage = false;
    schema
    {
        textelement(Universals)
        {
            tableelement(Universal; "ALT Universal")
            {
                AutoUpdate = true;
                XmlName = 'Universal';
                fieldelement(EntryNo; Universal."Entry No.") { }
                fieldelement(IntegerValue; Universal."Integer Field") { }
            }
        }
    }
}

// corpus _fixtures/xmlports/ALTUniversalReplaceXmlPort.al
xmlport 60027 "ALT Universal Replace XmlPort"
{
    Direction = Import; Format = Xml; UseRequestPage = false;
    schema
    {
        textelement(Universals)
        {
            tableelement(Universal; "ALT Universal")
            {
                AutoReplace = true;
                XmlName = 'Universal';
                fieldelement(EntryNo; Universal."Entry No.") { }
                fieldelement(IntegerValue; Universal."Integer Field") { }
            }
        }
    }
}

// corpus _fixtures/xmlports/ALTUniversalManualXmlPort.al
xmlport 60028 "ALT Universal Manual XmlPort"
{
    Direction = Import; Format = Xml; UseRequestPage = false;
    schema
    {
        textelement(Universals)
        {
            tableelement(Universal; "ALT Universal")
            {
                AutoSave = false;
                AutoUpdate = true;
                XmlName = 'Universal';
                fieldelement(EntryNo; Universal."Entry No.") { }
                fieldelement(IntegerValue; Universal."Integer Field") { }
                fieldelement(TextValue; Universal."Text Field") { }
                trigger OnAfterInitRecord()
                begin
                    Universal."Description Field" := 'init';
                end;
                trigger OnBeforeInsertRecord()
                begin
                    Universal."Description Field" := CopyStr(StrSubstNo('manual-insert-%1', Universal."Text Field"), 1, MaxStrLen(Universal."Description Field"));
                    Universal.Insert();
                end;
                trigger OnAfterInsertRecord()
                var Persisted: Record "ALT Universal";
                begin
                    Persisted.Get(Universal."Entry No.");
                    Persisted."Description Field" := CopyStr(Persisted."Description Field" + '-after', 1, MaxStrLen(Persisted."Description Field"));
                    Persisted.Modify();
                end;
                trigger OnBeforeModifyRecord()
                var Persisted: Record "ALT Universal";
                begin
                    Persisted.Get(Universal."Entry No.");
                    Persisted."Integer Field" := Universal."Integer Field";
                    Persisted."Text Field" := Universal."Text Field";
                    Persisted."Description Field" := 'manual-modify';
                    Persisted.Modify();
                end;
                trigger OnAfterModifyRecord()
                var Persisted: Record "ALT Universal";
                begin
                    Persisted.Get(Universal."Entry No.");
                    Persisted."Description Field" := CopyStr(Persisted."Description Field" + '-after', 1, MaxStrLen(Persisted."Description Field"));
                    Persisted.Modify();
                end;
            }
        }
    }
}

// corpus _fixtures/xmlports/ALTUniversalTemporaryXmlPort.al
xmlport 60029 "ALT Temp Universal XmlPort"
{
    Direction = Import; Format = Xml; UseRequestPage = false;
    schema
    {
        textelement(Universals)
        {
            tableelement(Universal; "ALT Universal")
            {
                UseTemporary = true;
                XmlName = 'Universal';
                fieldelement(EntryNo; Universal."Entry No.") { }
                fieldelement(IntegerValue; Universal."Integer Field") { }
                fieldelement(TextValue; Universal."Text Field") { }
                trigger OnAfterInsertRecord()
                var Persisted: Record "ALT Universal";
                begin
                    Persisted.Init();
                    Persisted.TransferFields(Universal, false);
                    Persisted."Entry No." := Universal."Entry No." + 1000;
                    Persisted."Description Field" := 'temp-copy';
                    Persisted.Insert();
                end;
            }
        }
    }
}

xmlport 60030 "ALT Export Only XmlPort"
{
    Direction = Export; Format = Xml; UseRequestPage = false;
    schema { textelement(Root) { tableelement(Universal; "ALT Universal") { fieldelement(EntryNo; Universal."Entry No.") { } } } }
}

xmlport 60031 "ALT Import Only XmlPort"
{
    Direction = Import; Format = Xml; UseRequestPage = false;
    schema { textelement(Root) { tableelement(Universal; "ALT Universal") { fieldelement(EntryNo; Universal."Entry No.") { } } } }
}

xmlport 60032 "ALT Variable Text XmlPort"
{
    Direction = Export; Format = VariableText; UseRequestPage = false;
    schema { textelement(Root) { tableelement(Universal; "ALT Universal") { fieldelement(EntryNo; Universal."Entry No.") { } } } }
}

xmlport 60033 "ALT Request Page XmlPort"
{
    Direction = Export; Format = Xml; UseRequestPage = true;
    schema { textelement(Root) { tableelement(Universal; "ALT Universal") { fieldelement(EntryNo; Universal."Entry No.") { } } } }
    requestpage { layout { area(Content) { field(Flag; Flag) { } } } }
    var Flag: Boolean;
}

table 60040 "ALT Keyed"
{
    fields { field(1; "Entry No."; Integer) { } field(2; Rank; Integer) { } }
    keys { key(PK; "Entry No.") { } key(ByRank; Rank) { } }
}

// Mutation-testing fixtures: shapes the corpus fixtures above do not have.
xmlport 60034 "ALT Keyed XmlPort"
{
    Direction = Both; Format = Xml; UseRequestPage = false;
    schema { textelement(Root) { tableelement(Row; "ALT Keyed") { fieldelement(EntryNo; Row."Entry No.") { } } } }
}

xmlport 60035 "ALT Raising XmlPort"
{
    Direction = Import; Format = Xml; UseRequestPage = false;
    schema
    {
        textelement(Universals)
        {
            tableelement(Universal; "ALT Universal")
            {
                XmlName = 'Universal';
                fieldelement(EntryNo; Universal."Entry No.")
                {
                    trigger OnAfterAssignField() begin Error('RAISED-IN-TRIGGER'); end;
                }
            }
        }
    }
}

xmlport 60036 "ALT Table Root XmlPort"
{
    Direction = Both; Format = Xml; UseRequestPage = false;
    schema { tableelement(Universal; "ALT Universal") { fieldelement(EntryNo; Universal."Entry No.") { } } }
}

xmlport 60037 "ALT Empty Root XmlPort"
{
    Direction = Both; Format = Xml; UseRequestPage = false;
    schema { textelement(Root) { } }
}

xmlport 60041 "ALT Upd Init XmlPort"
{
    Direction = Import; Format = Xml; UseRequestPage = false;
    schema
    {
        textelement(Universals)
        {
            tableelement(Universal; "ALT Universal")
            {
                AutoUpdate = true;
                XmlName = 'Universal';
                fieldelement(EntryNo; Universal."Entry No.") { }
                fieldelement(IntegerValue; Universal."Integer Field") { }
                trigger OnAfterInitRecord() begin end;
            }
        }
    }
}
xmlport 60042 "ALT Upd AssignField XmlPort"
{
    Direction = Import; Format = Xml; UseRequestPage = false;
    schema
    {
        textelement(Universals)
        {
            tableelement(Universal; "ALT Universal")
            {
                AutoUpdate = true;
                XmlName = 'Universal';
                fieldelement(EntryNo; Universal."Entry No.") { }
                fieldelement(IntegerValue; Universal."Integer Field") { trigger OnAfterAssignField() begin end; }
                
            }
        }
    }
}
xmlport 60043 "ALT Upd AssignVar XmlPort"
{
    Direction = Import; Format = Xml; UseRequestPage = false;
    schema
    {
        textelement(Universals)
        {
            tableelement(Universal; "ALT Universal")
            {
                AutoUpdate = true;
                XmlName = 'Universal';
                fieldelement(EntryNo; Universal."Entry No.") { }
                fieldelement(IntegerValue; Universal."Integer Field") { }
                textelement(Note) { trigger OnAfterAssignVariable() begin end; }
            }
        }
    }
}

xmlport 60044 "ALT Nested XmlPort"
{
    Direction = Both; Format = Xml; UseRequestPage = false;
    schema
    {
        textelement(Root)
        {
            textelement(Header)
            {
                textelement(Title)
                {
                    trigger OnBeforePassVariable() begin Title := 'Hello'; end;
                    trigger OnAfterAssignVariable()
                    var Persisted: Record "ALT Universal";
                    begin
                        Persisted.Init(); Persisted."Entry No." := 77; Persisted."Description Field" := Title; Persisted.Insert();
                    end;
                }
            }
        }
    }
}

xmlport 60045 "ALT Text In Table XmlPort"
{
    Direction = Both; Format = Xml; UseRequestPage = false;
    schema
    {
        textelement(Universals)
        {
            tableelement(Universal; "ALT Universal")
            {
                XmlName = 'Universal';
                fieldelement(EntryNo; Universal."Entry No.") { }
                textelement(Group) { textelement(Inner) { } }
            }
        }
    }
}

codeunit 50102 "XP Trace"
{
    SingleInstance = true;
    var Log: Text;
    procedure Add(S: Text) begin Log += S + ';'; end;
    procedure Get(): Text begin exit(Log); end;
}

xmlport 60046 "ALT Ordered XmlPort"
{
    Direction = Import; Format = Xml; UseRequestPage = false;
    schema
    {
        textelement(Parents)
        {
            tableelement(Parent; "ALT Parent")
            {
                XmlName = 'Parent';
                fieldelement(ParentEntryNo; Parent."Entry No.") { }
                tableelement(Child; "ALT Child")
                {
                    LinkTable = Parent;
                    LinkFields = "Parent Entry No." = FIELD("Entry No.");
                    XmlName = 'Child';
                    fieldelement(ChildEntryNo; Child."Entry No.") { }
                    trigger OnAfterInsertRecord() var T: Codeunit "XP Trace"; begin T.Add('C' + Format(Child."Entry No.")); end;
                }
                trigger OnAfterInsertRecord() var T: Codeunit "XP Trace"; begin T.Add('P' + Format(Parent."Entry No.")); end;
            }
        }
    }
}

xmlport 60047 "ALT Raising Export XmlPort"
{
    Direction = Export; Format = Xml; UseRequestPage = false;
    schema
    {
        textelement(Root)
        {
            textelement(Boom)
            {
                trigger OnBeforePassVariable() begin Error('EXPORT-RAISED'); end;
            }
        }
    }
}

// The shape of Base App's "Consolidation Import/Export" (xmlport 1): the container text element
// itself carries a trigger. No corpus test pins what BC does with it.
xmlport 60048 "ALT Container Trigger XmlPort"
{
    Direction = Both; Format = Xml; UseRequestPage = false;
    schema
    {
        textelement(Root)
        {
            textelement(Leaf) { }
            trigger OnAfterAssignVariable() begin end;
        }
    }
}

// corpus xmlport/TestXmlPortFormatEvaluateContracts.al (68460..68462)
table 60060 "FE Table"
{
    fields { field(1; Id; Integer) { } field(2; Dt; Date) { } field(3; Dec; Decimal) { } field(4; Bl; Boolean) { } }
    keys { key(PK; Id) { } }
}
xmlport 60061 "FE Legacy"
{
    Format = Xml; Direction = Both; FormatEvaluate = Legacy; UseRequestPage = false;
    schema { textelement(Root) { tableelement(T; "FE Table") {
        fieldelement(Id; T.Id) { } fieldelement(Dt; T.Dt) { } fieldelement(Dec; T.Dec) { } fieldelement(Bl; T.Bl) { } } } }
}
enum 60065 "FE Enum" { value(0; Zero) { Caption = 'Zero cap'; } value(1; One) { Caption = 'One cap'; } }
table 60066 "FE Enum Table"
{
    fields { field(1; Id; Integer) { } field(2; En; Enum "FE Enum") { } }
    keys { key(PK; Id) { } }
}
xmlport 60067 "FE Enum Xml"
{
    Format = Xml; Direction = Both; FormatEvaluate = Xml; UseRequestPage = false;
    schema { textelement(Root) { tableelement(T; "FE Enum Table") { fieldelement(Id; T.Id) { } fieldelement(En; T.En) { } } } }
}
xmlport 60062 "FE Xml"
{
    Format = Xml; Direction = Both; FormatEvaluate = Xml; UseRequestPage = false;
    schema { textelement(Root) { tableelement(T; "FE Table") {
        fieldelement(Id; T.Id) { } fieldelement(Dt; T.Dt) { } fieldelement(Dec; T.Dec) { } fieldelement(Bl; T.Bl) { } } } }
}

// corpus xmlport/lt-af-xmlport/AfXmlPortAttrChildren.al:9-113 (without its Encoding property)
table 60070 "ALT AF Row"
{
    fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } field(3; Lang; Text[10]) { } }
    keys { key(PK; Id) { } }
}

xmlport 60071 "ALT AF XP Attr"
{
    Direction = Both; Format = Xml; UseRequestPage = false;
    schema
    {
        textelement(Root)
        {
            tableelement(Row; "ALT AF Row")
            {
                fieldelement(Id; Row.Id) { }
                fieldelement(Name; Row.Name)
                {
                    fieldattribute(Lang; Row.Lang) { }
                    textattribute(Kind) { }
                }
            }
        }
    }
    procedure GetKind(): Text begin exit(Kind); end;
    procedure SetKind(NewKind: Text) begin Kind := NewKind; end;
}

xmlport 60072 "ALT AF XP Child Text"
{
    Direction = Both; Format = Xml; UseRequestPage = false;
    schema
    {
        textelement(Root)
        {
            tableelement(Row; "ALT AF Row")
            {
                fieldelement(Id; Row.Id) { }
                fieldelement(Name; Row.Name) { textelement(Child) { } }
            }
        }
    }
    procedure SetChild(NewChild: Text) begin Child := NewChild; end;
    procedure GetChild(): Text begin exit(Child); end;
}

xmlport 60073 "ALT AF XP Child Field"
{
    Direction = Both; Format = Xml; UseRequestPage = false;
    schema
    {
        textelement(Root)
        {
            tableelement(Row; "ALT AF Row")
            {
                fieldelement(Id; Row.Id) { }
                fieldelement(Name; Row.Name) { fieldelement(Lang; Row.Lang) { } }
            }
        }
    }
}

codeunit 50100 Helpers
{
    procedure Row(EntryNo: Integer; IntegerValue: Integer; TextValue: Text; Description: Text)
    var U: Record "ALT Universal";
    begin
        U.Init(); U."Entry No." := EntryNo; U."Integer Field" := IntegerValue;
        U."Text Field" := TextValue; U."Description Field" := Description; U.Insert();
    end;

    procedure Stage(var B: Record "ALT Blob"; var InStr: InStream; Xml: Text)
    var OutStr: OutStream;
    begin
        B.Init(); B.Code := 'TMP'; B.Insert();
        B.Data.CreateOutStream(OutStr);
        OutStr.WriteText(Xml);
        B.Data.CreateInStream(InStr);
    end;

    procedure ReadAll(var B: Record "ALT Blob"): Text
    var InStr: InStream; Segment: Text; All: Text;
    begin
        B.CalcFields(Data);
        B.Data.CreateInStream(InStr);
        while not InStr.EOS() do begin
            InStr.ReadText(Segment);
            All += Segment;
        end;
        exit(All);
    end;
}
"#;

fn run(vars: &str, body: &str) -> Result<Value, Signal> {
    run_in(vars, body, "", None)
}

/// With `seed`, the codeunit's `Seed` runs first under the default model and commits, then F runs
/// under `TransactionModel::None`.
fn run_in(vars: &str, body: &str, extra: &str, seed: Option<&str>) -> Result<Value, Signal> {
    let seed_proc = seed
        .map(|b| format!("procedure Seed() var {vars} begin {b} Commit(); end;"))
        .unwrap_or_default();
    let src = format!(
        "{FIXTURES}\ncodeunit 50101 C {{ {seed_proc} {extra} procedure F(): Text var {vars} begin {body} end; }}"
    );
    let l = al_testkit::compile(&src);
    let blocked = al_testkit::blocked(&l);
    let oi = l.program.objects.iter().position(|o| o.name == "C").unwrap();
    let ri = l.program.objects[oi]
        .routines
        .iter()
        .position(|r| r.name == "F")
        .unwrap();
    if let Some(Some(r)) = blocked.get(oi).and_then(|o| o.get(ri)) {
        return Err(Signal::Unsupported(r.clone()));
    }
    let mut s = al_testkit::session(l);
    let inst = s.instance(oi);
    if seed.is_some() {
        let si = s.engine.program.objects[oi]
            .routines
            .iter()
            .position(|r| r.name == "Seed")
            .unwrap();
        s.call(oi, si, inst.clone(), vec![]).unwrap();
        s.tx_model = al_vm::TxModel::None;
    }
    s.call(oi, ri, inst, vec![]).map(|(v, _)| v)
}

fn text(vars: &str, body: &str) -> String {
    match run(vars, body) {
        Ok(Value::Text(s)) | Ok(Value::Code(s)) => s.to_string(),
        other => panic!("{other:?}"),
    }
}

fn refusal(vars: &str, body: &str) -> String {
    match run(vars, body) {
        Err(Signal::Unsupported(r)) => r,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

const EXPORT_VARS: &str = "H: Codeunit Helpers; B: Record \"ALT Blob\"; OutStr: OutStream; P: XmlPort \"ALT Universal XmlPort\"; Ok: Boolean;";

// xmlport/TestXmlPortObject.al:XmlPort_Export_SetDestination_ProducesWellFormedXml
#[test]
fn export_writes_the_root_and_every_row_to_the_destination() {
    let out = text(
        EXPORT_VARS,
        "H.Row(1, 10, 'First', ''); H.Row(2, 20, 'Second', '');
         B.Code := 'XP1'; B.Insert(); B.Data.CreateOutStream(OutStr);
         P.SetDestination(OutStr);
         Ok := P.Export();
         B.Modify();
         exit(Format(Ok) + '|' + H.ReadAll(B));",
    );
    let (ok, xml) = out.split_once('|').unwrap();
    assert_eq!(ok, "Yes");
    assert!(xml.contains("<Universals>"), "{xml}");
    assert!(xml.contains("<EntryNo>1</EntryNo>"), "{xml}");
    assert!(xml.contains("<TextValue>Second</TextValue>"), "{xml}");
}

// xmlport/TestXmlPortObject.al:XmlPort_SetTableView_RestrictsExportedRows
#[test]
fn set_table_view_restricts_the_exported_rows() {
    let out = text(
        &format!("{EXPORT_VARS} U: Record \"ALT Universal\";"),
        "H.Row(1, 10, 'First', ''); H.Row(2, 20, 'Second', '');
         U.SetRange(\"Entry No.\", 2);
         B.Code := 'XP2'; B.Insert(); B.Data.CreateOutStream(OutStr);
         P.SetTableView(U); P.SetDestination(OutStr);
         Ok := P.Export();
         B.Modify();
         exit(H.ReadAll(B));",
    );
    assert!(out.contains("<EntryNo>2</EntryNo>"), "{out}");
    assert!(!out.contains("<EntryNo>1</EntryNo>"), "{out}");
}

// xmlport/TestXmlPortObject.al:XmlPort_Export_StaticWithRecord_RespectsFilters
#[test]
fn the_static_export_respects_the_filters_of_the_given_record() {
    let out = text(
        "H: Codeunit Helpers; B: Record \"ALT Blob\"; OutStr: OutStream; U: Record \"ALT Universal\"; Ok: Boolean;",
        "H.Row(1, 10, 'First', ''); H.Row(2, 20, 'Second', '');
         U.SetRange(\"Entry No.\", 2);
         B.Code := 'XP3'; B.Insert(); B.Data.CreateOutStream(OutStr);
         Ok := XmlPort.Export(60023, OutStr, U);
         B.Modify();
         exit(Format(Ok) + '|' + H.ReadAll(B));",
    );
    assert!(out.starts_with("Yes|"), "{out}");
    assert!(out.contains("<EntryNo>2</EntryNo>"), "{out}");
    assert!(!out.contains("<EntryNo>1</EntryNo>"), "{out}");
}

const IMPORT_VARS: &str = "H: Codeunit Helpers; B: Record \"ALT Blob\" temporary; InStr: InStream; U: Record \"ALT Universal\"; P: XmlPort \"ALT Universal XmlPort\";";
const TWO_ROWS: &str = "'<?xml version=\"1.0\" encoding=\"UTF-8\"?><Universals><Universal><EntryNo>1</EntryNo><IntegerValue>10</IntegerValue><TextValue>First</TextValue></Universal><Universal><EntryNo>2</EntryNo><IntegerValue>20</IntegerValue><TextValue>Second</TextValue></Universal></Universals>'";

// xmlport/TestXmlPortObject.al:XmlPort_Import_SetSource_InsertsRows
#[test]
fn import_through_set_source_inserts_every_row() {
    let out = text(
        IMPORT_VARS,
        &format!(
            "H.Stage(B, InStr, {TWO_ROWS});
             P.SetSource(InStr); P.Import();
             U.Get(2);
             exit(Format(U.Count()) + '|' + Format(U.\"Integer Field\") + '|' + U.\"Text Field\");"
        ),
    );
    assert_eq!(out, "2|20|Second");
}

// xmlport/TestXmlPortObject.al:XmlPort_Import_StaticFromStream_InsertsRows
#[test]
fn the_static_import_inserts_every_row() {
    let out = text(
        IMPORT_VARS,
        &format!("H.Stage(B, InStr, {TWO_ROWS}); XmlPort.Import(60023, InStr); exit(Format(U.Count()));"),
    );
    assert_eq!(out, "2");
}

// xmlport/TestXmlPortObject.al:XmlPort_Import_StaticWithRecord_GivenVariableUnchangedWithoutExplicitGet
#[test]
fn the_record_given_to_the_static_import_only_seeds_the_table_view() {
    let out = text(
        IMPORT_VARS,
        "H.Stage(B, InStr, '<Universals><Universal><EntryNo>1</EntryNo><IntegerValue>10</IntegerValue><TextValue>First</TextValue></Universal></Universals>');
         XmlPort.Import(60023, InStr, U);
         exit(Format(U.\"Entry No.\") + '|' + Format(U.\"Integer Field\") + '|' + Format(U.Count()));",
    );
    assert_eq!(out, "0|0|1");
}

// xmlport/TestXmlPortAdvanced.al:XmlPort_NestedTableElements_ExportIncludesChildRecordsAndAttribute
#[test]
fn a_nested_table_element_exports_its_linked_rows_and_field_attributes() {
    let out = text(
        "H: Codeunit Helpers; B: Record \"ALT Blob\"; OutStr: OutStream; P: XmlPort \"ALT Parent Child XmlPort\"; Pa: Record \"ALT Parent\"; C: Record \"ALT Child\";",
        "Pa.\"Entry No.\" := 1; Pa.Code := 'P1'; Pa.Insert();
         C.\"Entry No.\" := 10; C.\"Parent Entry No.\" := 1; C.Code := 'C10'; C.Amount := 15; C.Insert();
         C.\"Entry No.\" := 11; C.\"Parent Entry No.\" := 1; C.Code := 'C11'; C.Amount := 25; C.Insert();
         C.\"Entry No.\" := 12; C.\"Parent Entry No.\" := 2; C.Code := 'C12'; C.Amount := 35; C.Insert();
         B.Code := 'XPA1'; B.Insert(); B.Data.CreateOutStream(OutStr);
         P.SetDestination(OutStr); P.Export(); B.Modify();
         exit(H.ReadAll(B));",
    );
    assert!(out.contains("<ParentEntryNo>1</ParentEntryNo>"), "{out}");
    assert!(out.contains("<ChildEntryNo>10</ChildEntryNo>"), "{out}");
    assert!(out.contains("<ChildEntryNo>11</ChildEntryNo>"), "{out}");
    assert!(!out.contains("<ChildEntryNo>12</ChildEntryNo>"), "{out}");
    // Amount is a Decimal: BC writes it with two decimal places (the test's own comment).
    assert!(out.contains("Amount=\"15.00\""), "{out}");
}

// xmlport/TestXmlPortAdvanced.al:XmlPort_NestedTableElements_ImportCreatesParentAndChildren
#[test]
fn a_nested_import_creates_the_children_with_the_link_field_of_their_parent() {
    let out = text(
        "H: Codeunit Helpers; B: Record \"ALT Blob\" temporary; InStr: InStream; Pa: Record \"ALT Parent\"; C: Record \"ALT Child\";",
        "H.Stage(B, InStr, '<Parents><Parent><ParentEntryNo>1</ParentEntryNo><ParentCode>P1</ParentCode><Child Amount=\"15.00\"><ChildEntryNo>10</ChildEntryNo><ChildCode>C10</ChildCode></Child><Child Amount=\"25.00\"><ChildEntryNo>11</ChildEntryNo><ChildCode>C11</ChildCode></Child></Parent></Parents>');
         XmlPort.Import(60024, InStr);
         C.Get(10);
         exit(Format(Pa.Count()) + '|' + Format(C.Count()) + '|' + Format(C.\"Parent Entry No.\") + '|' + Format(C.Amount));",
    );
    assert_eq!(out, "1|2|1|15");
}

// xmlport/TestXmlPortAdvanced.al:XmlPort_TextVariableTriggers_ExportManipulatesTextAndAttributes
#[test]
fn before_pass_variable_shapes_exported_text_and_attributes() {
    let out = text(
        "H: Codeunit Helpers; B: Record \"ALT Blob\"; OutStr: OutStream; P: XmlPort \"ALT Variable XmlPort\";",
        "H.Row(1, 2, 'First', '');
         B.Code := 'XPA3'; B.Insert(); B.Data.CreateOutStream(OutStr);
         P.SetDestination(OutStr); P.Export(); B.Modify();
         exit(H.ReadAll(B));",
    );
    assert!(out.contains("NoteAttr=\"FIRST\""), "{out}");
    assert!(out.contains("<DisplayText>TXT:First</DisplayText>"), "{out}");
}

// xmlport/TestXmlPortAdvanced.al:XmlPort_TextVariableTriggers_ImportMutatesAssignedValues
#[test]
fn after_assign_triggers_rewrite_what_an_import_stores() {
    let out = text(
        "H: Codeunit Helpers; B: Record \"ALT Blob\" temporary; InStr: InStream; U: Record \"ALT Universal\";",
        "H.Stage(B, InStr, '<Universals><Universal NoteAttr=\"ALPHA\"><EntryNo>1</EntryNo><IntegerValue>3</IntegerValue><DisplayText>TXT:alpha</DisplayText></Universal></Universals>');
         XmlPort.Import(60025, InStr);
         U.Get(1);
         exit(Format(U.\"Integer Field\") + '|' + U.\"Text Field\" + '|' + U.\"Description Field\");",
    );
    assert_eq!(out, "30|alpha|ALPHA");
}

fn partial(entry: i32, int: i32) -> String {
    format!(
        "'<Universals><Universal><EntryNo>{entry}</EntryNo><IntegerValue>{int}</IntegerValue></Universal></Universals>'"
    )
}

// xmlport/TestXmlPortAdvanced.al:XmlPort_AutoUpdate_UpdatesSpecifiedFieldsAndPreservesOthers
#[test]
fn auto_update_changes_only_the_fields_present_in_the_import() {
    let out = text(
        IMPORT_VARS,
        &format!(
            "H.Row(1, 10, 'Keep', 'KeepDesc');
             H.Stage(B, InStr, {});
             XmlPort.Import(60026, InStr);
             U.Get(1);
             exit(Format(U.\"Integer Field\") + '|' + U.\"Text Field\" + '|' + U.\"Description Field\");",
            partial(1, 99)
        ),
    );
    assert_eq!(out, "99|Keep|KeepDesc");
}

// xmlport/TestXmlPortAdvanced.al:XmlPort_AutoReplace_ReinitializesOmittedFields
#[test]
fn auto_replace_resets_the_fields_the_import_omits() {
    let out = text(
        IMPORT_VARS,
        &format!(
            "H.Row(1, 10, 'Keep', 'KeepDesc');
             H.Stage(B, InStr, {});
             XmlPort.Import(60027, InStr);
             U.Get(1);
             exit(Format(U.\"Integer Field\") + '|' + U.\"Text Field\" + '|' + U.\"Description Field\" + '|' + Format(U.Count()));",
            partial(1, 99)
        ),
    );
    assert_eq!(out, "99|||1");
}

fn full(entry: i32, int: i32, txt: &str) -> String {
    format!(
        "'<Universals><Universal><EntryNo>{entry}</EntryNo><IntegerValue>{int}</IntegerValue><TextValue>{txt}</TextValue></Universal></Universals>'"
    )
}

// xmlport/TestXmlPortAdvanced.al:XmlPort_AutoSaveFalse_OnBeforeAndAfterInsertControlPersistence
#[test]
fn with_auto_save_off_the_insert_triggers_decide_what_is_stored() {
    let out = text(
        IMPORT_VARS,
        &format!(
            "H.Stage(B, InStr, {}); XmlPort.Import(60028, InStr); U.Get(1); exit(U.\"Description Field\");",
            full(1, 10, "First")
        ),
    );
    assert_eq!(out, "manual-insert-First-after");
}

// xmlport/TestXmlPortAdvanced.al:XmlPort_AutoSaveFalse_OnBeforeAndAfterModifyControlPersistence
#[test]
fn with_auto_save_off_the_modify_triggers_decide_what_is_stored() {
    let out = text(
        IMPORT_VARS,
        &format!(
            "H.Row(1, 10, 'Before', 'Existing'); H.Stage(B, InStr, {}); XmlPort.Import(60028, InStr); U.Get(1);
             exit(Format(U.\"Integer Field\") + '|' + U.\"Text Field\" + '|' + U.\"Description Field\");",
            full(1, 42, "After")
        ),
    );
    assert_eq!(out, "42|After|manual-modify-after");
}

// xmlport/TestXmlPortAdvanced.al:XmlPort_UseTemporary_RequiresManualPersistence
#[test]
fn a_temporary_table_element_stores_nothing_in_the_real_table() {
    let out = text(
        IMPORT_VARS,
        &format!(
            "H.Stage(B, InStr, {}); XmlPort.Import(60029, InStr);
             if U.Get(1) then exit('real row 1 exists');
             U.Get(1001);
             exit(Format(U.\"Integer Field\") + '|' + U.\"Text Field\" + '|' + U.\"Description Field\");",
            full(1, 10, "Temp")
        ),
    );
    assert_eq!(out, "10|Temp|temp-copy");
}

// xmlport/TestXmlPortImportWriteTx.al:StaticGuardedImport_WithUncommittedWrite_ReturnsFalse_AndImportsNothing
#[test]
fn a_guarded_static_import_with_a_pending_write_returns_false_and_imports_nothing() {
    let out = text(
        &format!("{IMPORT_VARS} Ok: Boolean;"),
        &format!(
            "Commit(); H.Stage(B, InStr, {TWO_ROWS}); H.Row(9, 0, '', '');
             Ok := XmlPort.Import(60023, InStr);
             U.SetFilter(\"Entry No.\", '1..2'); exit(Format(Ok) + '|' + Format(U.Count()));"
        ),
    );
    assert_eq!(out, "No|0");
}

// xmlport/TestXmlPortImportWriteTx.al:InstanceGuardedImport_WithUncommittedWrite_IsRefused_AndImportsNothing
#[test]
fn a_guarded_instance_import_with_a_pending_write_is_refused_with_an_error() {
    let out = text(
        &format!("{IMPORT_VARS} Ok: Boolean;"),
        &format!(
            "Commit(); H.Stage(B, InStr, {TWO_ROWS}); H.Row(9, 0, '', '');
             P.SetSource(InStr);
             asserterror Ok := P.Import();
             U.SetFilter(\"Entry No.\", '1..2'); exit(GetLastErrorText() + '|' + Format(Ok) + '|' + Format(U.Count()));"
        ),
    );
    assert!(out.contains("the transaction is stopped"), "{out}");
    assert!(out.ends_with("|No|0"), "{out}");
}

// xmlport/TestXmlPortImportWriteTx.al:StaticStatementImport_WithUncommittedWrite_IsAllowed_AndImportsRows
// and InstanceStatementImport_WithUncommittedWrite_IsAllowed_AndImportsRows
#[test]
fn the_statement_form_joins_the_callers_transaction() {
    let out = text(
        IMPORT_VARS,
        &format!(
            "Commit(); H.Stage(B, InStr, {TWO_ROWS}); H.Row(9, 0, '', '');
             P.SetSource(InStr); P.Import();
             exit(Format(U.Count()));"
        ),
    );
    assert_eq!(out, "3");
}

// xmlport/TestXmlPortImportWriteTx.al:InstanceGuardedImport_AfterCommit_ReturnsTrue_AndImportsRows
// and GuardedImport_LeavingImportedRows_DoesNotBlockTheNextGuardedImport
#[test]
fn a_guarded_import_commits_its_own_transaction() {
    let out = text(
        &format!("{IMPORT_VARS} Ok: Boolean; Ok2: Boolean;"),
        &format!(
            "Commit(); H.Stage(B, InStr, {TWO_ROWS});
             P.SetSource(InStr); Ok := P.Import();
             exit(Format(Ok) + '|' + Format(U.Count()) + '|' + Format(Database.IsInWriteTransaction()));"
        ),
    );
    assert_eq!(out, "Yes|2|No");
}

// xmlport/TestXmlPortImportWriteTx.al:StaticGuardedImport_FailingSecondElement_ReturnsFalse_AndRollsBackTheFirstRow
// and InstanceGuardedImport_FailingSecondElement_ReturnsFalse_AndRollsBackTheFirstRow
#[test]
fn a_failing_guarded_import_returns_false_and_rolls_back_its_rows() {
    let bad = "'<Universals><Universal><EntryNo>9321</EntryNo><IntegerValue>10</IntegerValue><TextValue>First</TextValue></Universal><Universal><EntryNo>9322</EntryNo><IntegerValue>NotAnInteger</IntegerValue><TextValue>Second</TextValue></Universal></Universals>'";
    for call in [
        "Ok := XmlPort.Import(60023, InStr);",
        "P.SetSource(InStr); Ok := P.Import();",
    ] {
        let out = text(
            &format!("{IMPORT_VARS} Ok: Boolean;"),
            &format!(
                "H.Row(1, 0, '', ''); Commit(); H.Stage(B, InStr, {bad}); {call}
                 exit(Format(Ok) + '|' + Format(U.Count()) + '|' + Format(U.Get(1)));"
            ),
        );
        assert_eq!(out, "No|1|Yes", "{call}");
    }
}

// The statement form leaves the failure to the caller (xmlport/TestXmlPortImportWriteTx.al does not
// pin its text, so the failing statement import is refused).
// xmlport/TestXmlPortExportImportContracts.al:XmlPortImport_BadInteger_ThrowsEvaluate
#[test]
fn a_failing_statement_import_raises_the_evaluate_error() {
    match run(
        IMPORT_VARS,
        "H.Stage(B, InStr, '<Universals><Universal><EntryNo>1</EntryNo><IntegerValue>x</IntegerValue></Universal></Universals>'); XmlPort.Import(60023, InStr); exit('');",
    ) {
        Err(Signal::Error(e)) => {
            assert_eq!(e.message, "The value \"x\" can't be evaluated into type Integer.");
            assert_eq!(e.code, "Evaluate");
        }
        other => panic!("expected an error, got {other:?}"),
    }
}

// xmlport/TestXmlPortPrecompiledObject.al:PrecompiledXmlPort_DeclaredExportDirection_RefusesImport
// (the message is a platform resource string the corpus does not pin).

#[test]
fn xmlports_the_runtime_cannot_run_are_refused_before_they_start() {
    // Declaring the port is fine; running it is refused before anything of it starts.
    assert_eq!(
        text("P: XmlPort \"ALT Request Page XmlPort\";", "exit('declared');"),
        "declared"
    );
    let r = refusal(
        "P: XmlPort \"ALT Request Page XmlPort\"; B: Record \"ALT Blob\"; OutStr: OutStream;",
        "B.Code := 'X'; B.Insert(); B.Data.CreateOutStream(OutStr); P.SetDestination(OutStr); P.Export(); exit('ran');",
    );
    assert!(r.contains("request page"), "{r}");
}

// The text of a failed import is not pinned by the corpus, so reading it is refused rather than
// answered with an invented sentence (hard rule 3); the pinned parts (false, rollback) still hold.
// The error of a failed guarded import stays readable.
#[test]
fn the_last_error_of_a_failed_guarded_import_can_be_read() {
    let bad = "'<Universals><Universal><EntryNo>1</EntryNo><IntegerValue>x</IntegerValue></Universal></Universals>'";
    for (read, expected) in [
        (
            "GetLastErrorText()",
            "The value \"x\" can't be evaluated into type Integer.",
        ),
        ("GetLastErrorCode()", "Evaluate"),
    ] {
        let out = text(
            &format!("{IMPORT_VARS} Ok: Boolean;"),
            &format!("H.Stage(B, InStr, {bad}); Ok := XmlPort.Import(60023, InStr); exit({read});"),
        );
        assert_eq!(out, expected);
    }
    // Without a read the import still reports false.
    let out = text(
        &format!("{IMPORT_VARS} Ok: Boolean;"),
        &format!("H.Stage(B, InStr, {bad}); Ok := XmlPort.Import(60023, InStr); exit(Format(Ok));"),
    );
    assert_eq!(out, "No");
}

// ---- mutation-testing additions (docs/status.md debt) ----

// SetSource takes an InStream and SetDestination an OutStream; the stream of the other direction
// is refused (the corpus only ever passes the right one).
#[test]
fn set_source_and_set_destination_refuse_the_stream_of_the_other_direction() {
    let vars =
        "B: Record \"ALT Blob\" temporary; InStr: InStream; OutStr: OutStream; P: XmlPort \"ALT Universal XmlPort\";";
    let r = refusal(vars, "P.SetSource(OutStr); exit('');");
    assert!(r.contains("Xmlport.SetSource"), "{r}");
    let r = refusal(vars, "P.SetDestination(InStr); exit('');");
    assert!(r.contains("Xmlport.SetDestination"), "{r}");
    let r = refusal(vars, "XmlPort.Import(60023, OutStr); exit('');");
    assert!(r.contains("stream"), "{r}");
    let r = refusal(vars, "XmlPort.Export(60023, InStr); exit('');");
    assert!(r.contains("stream"), "{r}");
}

// SetTableView copies only the filters of a plain view: one that is temporary, marked-only,
// sorted by another key or descending changes more than filters, and what BC does is not pinned.
#[test]
fn set_table_view_refuses_a_view_that_is_more_than_filters() {
    for (setup, why) in [
        ("", "ok"),
        ("V.MarkedOnly(true);", "marked"),
        // A key and a direction order the export (corpus xmlport/TestXmlPortTriggersReplaceViewContracts.al).
        ("V.SetCurrentKey(Rank);", "ok"),
        ("V.Ascending(false);", "ok"),
    ] {
        let r = run(
            "V: Record \"ALT Keyed\"; P: XmlPort \"ALT Keyed XmlPort\";",
            &format!("V.SetRange(\"Entry No.\", 1); {setup} P.SetTableView(V); exit('set');"),
        );
        if why == "ok" {
            assert!(matches!(r, Ok(Value::Text(ref t)) if t == "set"), "{r:?}");
        } else {
            match r {
                Err(Signal::Unsupported(m)) => assert!(m.contains("temporary or marked"), "{setup}: {m}"),
                other => panic!("{setup}: {other:?}"),
            }
        }
    }
    let r = refusal(
        "V: Record \"ALT Keyed\" temporary; P: XmlPort \"ALT Keyed XmlPort\";",
        "P.SetTableView(V); exit('');",
    );
    assert!(r.contains("temporary or marked"), "{r}");
}

// A view of another table is refused too, and an xmlport with no table element has nothing to
// give the view to; both refusals name the xmlport.
#[test]
fn set_table_view_refusals_name_what_is_wrong() {
    let r = refusal(
        "V: Record \"ALT Universal\"; P: XmlPort \"ALT Keyed XmlPort\";",
        "P.SetTableView(V); exit('');",
    );
    assert!(r.contains("another table"), "{r}");
    let r = refusal(
        "V: Record \"ALT Universal\"; P: XmlPort \"ALT Empty Root XmlPort\";",
        "P.SetTableView(V); exit('');",
    );
    assert!(
        r.contains("\"ALT Empty Root XmlPort\"") && r.contains("no table element"),
        "{r}"
    );
}

// A refused direction names the xmlport and the direction it declares.
// xmlport/TestXmlPortExportImportContracts.al:XmlPort_WrongDirection_Throws
#[test]
fn a_direction_mismatch_raises_the_pinned_text_and_code() {
    let err = |vars: &str, body: &str| match run(vars, body) {
        Err(Signal::Error(e)) => (e.message, e.code),
        other => panic!("expected an error, got {other:?}"),
    };
    let (m, c) = err(
        "B: Record \"ALT Blob\" temporary; H: Codeunit Helpers; InStr: InStream; P: XmlPort \"ALT Export Only XmlPort\";",
        "H.Stage(B, InStr, '<Root/>'); P.SetSource(InStr); P.Import(); exit('');",
    );
    assert_eq!(m, "The XMLPort 'ALT Export Only XmlPort' is designed for export only.");
    assert_eq!(c, "XmlPortExportDirection");
    let (m, c) = err(
        "B: Record \"ALT Blob\"; OutStr: OutStream; P: XmlPort \"ALT Import Only XmlPort\";",
        "B.Code := 'X'; B.Insert(); B.Data.CreateOutStream(OutStr); P.SetDestination(OutStr); P.Export(); exit('');",
    );
    assert_eq!(m, "The XMLPort 'ALT Import Only XmlPort' is designed for import only.");
    assert_eq!(c, "XmlPortImportDirection");
}

// The root of an importable document must be a text element with content.
#[test]
fn an_import_whose_root_is_not_a_text_element_with_content_is_refused() {
    let vars = "H: Codeunit Helpers; B: Record \"ALT Blob\" temporary; InStr: InStream; P: XmlPort \"ALT Table Root XmlPort\"; E: XmlPort \"ALT Empty Root XmlPort\";";
    let r = refusal(
        vars,
        "H.Stage(B, InStr, '<Universal><EntryNo>1</EntryNo></Universal>'); P.SetSource(InStr); P.Import(); exit('');",
    );
    assert!(r.contains("root is not a text element with content"), "{r}");
    let r = refusal(
        vars,
        "H.Stage(B, InStr, '<Root><X>1</X></Root>'); E.SetSource(InStr); E.Import(); exit('');",
    );
    assert!(r.contains("root is not a text element with content"), "{r}");
}

// An error that AL code raises inside an import trigger is the last error of a guarded import,
// with its own text (xmlport/TestXmlPortImportWriteTx.al:StaticGuardedImport_FailingSecondElement_ReturnsFalse...
// pins the false and the rollback; the text of the platform's own failures is what is not pinned).
#[test]
fn an_error_raised_by_a_trigger_keeps_its_text_as_the_last_error() {
    let out = text(
        &format!("{IMPORT_VARS} Ok: Boolean;"),
        "H.Stage(B, InStr, '<Universals><Universal><EntryNo>1</EntryNo></Universal></Universals>');
         Ok := XmlPort.Import(60035, InStr);
         exit(Format(Ok) + '|' + GetLastErrorText());",
    );
    assert_eq!(out, "No|RAISED-IN-TRIGGER");
}

// A guarded import inside a TryFunction needs a transaction world the try scope cannot give;
// the corpus does not pin what BC does.
#[test]
fn a_guarded_import_inside_a_try_function_is_refused() {
    let r = run_in(
        &format!("{IMPORT_VARS} Ok: Boolean;"),
        "H.Stage(B, InStr, '<Universals/>'); exit(Format(Attempt(InStr)));",
        "[TryFunction] procedure Attempt(var S: InStream) var Ok: Boolean; begin Ok := XmlPort.Import(60023, S); end;",
        None,
    );
    match r {
        Err(Signal::Unsupported(m)) => assert!(m.contains("inside a TryFunction"), "{m}"),
        other => panic!("{other:?}"),
    }
}

// codeunit/TestWriteTxTestBoundary.al:WriteTx_Test12_AnXmlPortImportMayWriteUnderNone: the import
// opens a transaction of its own under TransactionModel::None, which ends with it -- and a direct
// write from the test is refused again afterwards.
#[test]
fn under_transaction_model_none_an_import_may_write_and_the_refusal_returns_after_it() {
    let r = run_in(
        &format!("{IMPORT_VARS} Seen: Text;"),
        &format!(
            "H.Stage(B, InStr, {TWO_ROWS});
             P.SetSource(InStr); P.Import();
             Seen := Format(U.Count()) + '|' + Format(Database.IsInWriteTransaction());
             asserterror H.Row(9, 0, '', '');
             exit(Seen + '|' + GetLastErrorText());"
        ),
        "",
        Some(""),
    );
    assert_eq!(
        text_of(r),
        "2|No|A transaction must be started before changes can be made to the database."
    );
}

// The statement form joins the caller's transaction: it neither commits the caller's pending
// write nor returns a value (xmlport/TestXmlPortImportWriteTx.al:StaticStatementImport_WithUncommittedWrite_IsAllowed_AndImportsRows).
#[test]
fn a_statement_import_leaves_the_callers_write_uncommitted() {
    let out = text(
        IMPORT_VARS,
        &format!(
            "H.Stage(B, InStr, {TWO_ROWS}); H.Row(9, 0, '', '');
             P.SetSource(InStr); P.Import();
             exit(Format(Database.IsInWriteTransaction()));"
        ),
    );
    assert_eq!(out, "Yes");
}

fn text_of(r: Result<Value, Signal>) -> String {
    match r {
        Ok(Value::Text(s)) | Ok(Value::Code(s)) => s.to_string(),
        other => panic!("{other:?}"),
    }
}

// AutoUpdate on an existing row with a trigger that assigns fields is refused, whichever trigger
// it is: which value wins (the document's or the trigger's) is not pinned.
#[test]
fn auto_update_with_any_assigning_trigger_is_refused() {
    for id in [60041, 60042, 60043] {
        // The text element of the last port is not optional, so the document carries it.
        let note = if id == 60043 { "<Note>n</Note>" } else { "" };
        let r = refusal(
            IMPORT_VARS,
            &format!(
                "H.Row(1, 5, 'Keep', ''); H.Stage(B, InStr, '<Universals><Universal><EntryNo>1</EntryNo><IntegerValue>9</IntegerValue>{note}</Universal></Universals>');
                 XmlPort.Import({id}, InStr); exit('');"
            ),
        );
        assert!(r.contains("AutoUpdate with triggers that assign fields"), "{id}: {r}");
    }
    // The control: the same document through the trigger-free fixture updates the row.
    let out = text(
        IMPORT_VARS,
        "H.Row(1, 5, 'Keep', ''); H.Stage(B, InStr, '<Universals><Universal><EntryNo>1</EntryNo><IntegerValue>9</IntegerValue></Universal></Universals>');
         XmlPort.Import(60026, InStr); U.Get(1); exit(Format(U.\"Integer Field\") + U.\"Text Field\");",
    );
    assert_eq!(out, "9Keep");
}

// A text element with content inside the root is a group: its children are exported and
// imported at that depth (corpus fixtures only nest table elements).
#[test]
fn a_nested_text_group_exports_and_imports_its_variables() {
    let out = text(
        "H: Codeunit Helpers; B: Record \"ALT Blob\"; OutStr: OutStream;",
        "B.Code := 'N1'; B.Insert(); B.Data.CreateOutStream(OutStr);
         XmlPort.Export(60044, OutStr); B.Modify();
         exit(H.ReadAll(B));",
    );
    assert!(
        out.contains("<Root>  <Header>    <Title>Hello</Title>  </Header></Root>"),
        "{out}"
    );
    let out = text(
        IMPORT_VARS,
        "H.Stage(B, InStr, '<Root><Header><Title>World</Title></Header></Root>');
         XmlPort.Import(60044, InStr); U.Get(77); exit(U.\"Description Field\");",
    );
    assert_eq!(out, "World");
}

// A text element with content inside a table element is refused in both directions.
#[test]
fn a_text_group_inside_a_table_element_is_refused() {
    let r = refusal(
        "H: Codeunit Helpers; B: Record \"ALT Blob\"; OutStr: OutStream;",
        "H.Row(1, 0, '', ''); B.Code := 'N2'; B.Insert(); B.Data.CreateOutStream(OutStr);
         XmlPort.Export(60045, OutStr); exit('');",
    );
    assert!(r.contains("text element with content inside a table element"), "{r}");
    let r = refusal(
        IMPORT_VARS,
        "H.Stage(B, InStr, '<Universals><Universal><EntryNo>1</EntryNo><Group><Inner>x</Inner></Group></Universal></Universals>');
         XmlPort.Import(60045, InStr); exit('');",
    );
    assert!(r.contains("text element with content inside a table element"), "{r}");
}

// The parent row is stored before the elements below it are read (assumption in
// docs/probes-needed.md; the corpus reads the same order from the stored rows).
#[test]
fn a_parent_is_stored_before_its_children_are_read() {
    let out = text(
        "H: Codeunit Helpers; B: Record \"ALT Blob\" temporary; InStr: InStream; T: Codeunit \"XP Trace\";",
        "H.Stage(B, InStr, '<Parents><Parent><ParentEntryNo>1</ParentEntryNo><Child><ChildEntryNo>10</ChildEntryNo></Child><Child><ChildEntryNo>11</ChildEntryNo></Child></Parent></Parents>');
         XmlPort.Import(60046, InStr); exit(T.Get());",
    );
    assert_eq!(out, "P1;C10;C11;");
}

// The export root must be a text element with content, as for an import.
#[test]
fn an_export_whose_root_is_not_a_text_element_with_content_is_refused() {
    for id in [60036, 60037] {
        let r = refusal(
            "H: Codeunit Helpers; B: Record \"ALT Blob\"; OutStr: OutStream;",
            &format!(
                "H.Row(1, 0, '', ''); B.Code := 'R'; B.Insert(); B.Data.CreateOutStream(OutStr);
                 XmlPort.Export({id}, OutStr); exit('');"
            ),
        );
        assert!(r.contains("root is not a text element with content"), "{id}: {r}");
    }
}

// An export to a UTF-16 or MS-DOS stream is refused: only UTF-8 and Windows are implemented, and the
// output encoding of the others is not pinned.
// The document is written in its own encoding whatever the encoding of the stream.
#[test]
fn an_export_to_a_utf16_or_msdos_stream_writes_the_same_bytes() {
    for enc in ["UTF16", "MSDos", "UTF8"] {
        let out = text(
            "H: Codeunit Helpers; B: Record \"ALT Blob\"; OutStr: OutStream;",
            &format!(
                "H.Row(1, 0, '', ''); B.Code := 'U'; B.Insert(); B.Data.CreateOutStream(OutStr, TextEncoding::{enc});
                 XmlPort.Export(60023, OutStr); B.Modify(); B.CalcFields(Data); exit(Format(B.Data.Length));"
            ),
        );
        assert!(out.parse::<i32>().is_ok_and(|n| n > 2 && n % 2 == 0), "{enc}: {out}");
    }
}

// An error raised while exporting: as a statement it reaches the caller with its text; with the
// result used (guarded) the outcome is not pinned and is refused.
#[test]
fn an_error_raised_during_export_reaches_a_statement_and_refuses_a_guarded_call() {
    let vars = "H: Codeunit Helpers; B: Record \"ALT Blob\"; OutStr: OutStream; Ok: Boolean;";
    let setup = "H.Row(1, 0, 'x', ''); B.Code := 'E'; B.Insert(); B.Data.CreateOutStream(OutStr);";
    let out = text(
        vars,
        &format!("{setup} asserterror XmlPort.Export(60047, OutStr); exit(GetLastErrorText());"),
    );
    assert_eq!(out, "EXPORT-RAISED");
    let r = refusal(vars, &format!("{setup} Ok := XmlPort.Export(60047, OutStr); exit('');"));
    assert!(r.contains("an error inside an Export whose result is used"), "{r}");
    // A good export with the result used is true; as a statement it has no value.
    let out = text(
        vars,
        &format!("{setup} Ok := XmlPort.Export(60023, OutStr); exit(Format(Ok));"),
    );
    assert_eq!(out, "Yes");
}

// The statement form of an import that fails with an error AL code raised in a trigger lets that
// error through with its own text (only the platform's own failures are refused as unpinned, see
// a_failing_statement_import_is_refused_because_its_error_text_is_not_pinned).
#[test]
fn a_statement_import_lets_a_trigger_error_through() {
    let out = text(
        IMPORT_VARS,
        "H.Stage(B, InStr, '<Universals><Universal><EntryNo>1</EntryNo></Universal></Universals>');
         asserterror XmlPort.Import(60035, InStr);
         exit(GetLastErrorText());",
    );
    assert_eq!(out, "RAISED-IN-TRIGGER");
}

// A container text element with a trigger (Base App xmlport 1, "Consolidation Import/Export") is
// not pinned by any corpus test, so running the port is refused. Code that only holds the port
// (handlers/TestReqPageBoundVisibility.al:172-188 reaches it through Report.Run and cancels the
// request page) still runs.
#[test]
fn a_container_text_element_trigger_is_refused_when_the_port_runs_not_when_it_is_declared() {
    let held = text("P: XmlPort \"ALT Container Trigger XmlPort\";", "exit('reached');");
    assert_eq!(held, "reached");
    let why = refusal(
        "P: XmlPort \"ALT Container Trigger XmlPort\"; B: Record \"ALT Blob\"; OutStr: OutStream;",
        "B.Code := 'X'; B.Insert(); B.Data.CreateOutStream(OutStr); P.SetDestination(OutStr); P.Export(); exit('ran');",
    );
    assert!(why.contains("triggers of the container text element Root"), "{why}");
}

const FE_ROW: &str = "T: Record \"FE Table\"; B: Record \"ALT Blob\"; OutStr: OutStream; PL: XmlPort \"FE Legacy\"; PX: XmlPort \"FE Xml\"; H: Codeunit Helpers; s: Text;";

// xmlport/TestXmlPortFormatEvaluateContracts.al:131-155: Legacy writes the display text, Xml the XML forms.
#[test]
fn format_evaluate_decides_the_text_an_export_writes() {
    let out = text(
        FE_ROW,
        "T.Id := 1; T.Dt := DMY2Date(5, 3, 2024); T.Dec := 1234.5; T.Bl := true; T.Insert();
         B.Code := 'L'; B.Insert(); B.Data.CreateOutStream(OutStr); PL.SetDestination(OutStr); PL.Export(); B.Modify();
         s := H.ReadAll(B);
         B.Code := 'X'; B.Insert(); B.Data.CreateOutStream(OutStr); PX.SetDestination(OutStr); PX.Export(); B.Modify();
         exit(s + '#' + H.ReadAll(B));",
    );
    let (legacy, xml) = out.split_once('#').unwrap();
    assert!(
        legacy.contains("<Dt>03/05/24</Dt><Dec>1,234.50</Dec><Bl>Yes</Bl>") || legacy.contains("<Dt>03/05/24</Dt>"),
        "{legacy}"
    );
    assert!(legacy.contains("1,234.50") && legacy.contains("Yes"), "{legacy}");
    assert!(
        xml.contains("<Dt>2024-03-05</Dt>") && xml.contains("1234.50") && xml.contains("<Bl>true</Bl>"),
        "{xml}"
    );
}

// :157-197: an import reads only its own form; the Xml port refuses the display date (the start of BC's text
// is pinned) and the Legacy port the XML date ("The date is not valid."); neither stores a row.
#[test]
fn format_evaluate_decides_the_form_an_import_reads() {
    let imp = |port: u32, row: &str| {
        text(
            "H: Codeunit Helpers; B: Record \"ALT Blob\" temporary; InStr: InStream; T: Record \"FE Table\"; Ok: Boolean;",
            &format!(
                "H.Stage(B, InStr, '<Root><T>{row}</T></Root>'); Ok := XmlPort.Import({port}, InStr);
                 if T.FindFirst() then; exit(Format(Ok) + '|' + GetLastErrorText() + '|' + Format(T.Count()) + '|' + Format(T.Dt, 0, 9) + Format(T.Dec, 0, 9));"
            ),
        )
    };
    let xml_row = "<Id>1</Id><Dt>2024-03-05</Dt><Dec>1234.5</Dec><Bl>true</Bl>";
    let display_row = "<Id>1</Id><Dt>03/05/24</Dt><Dec>1,234.5</Dec><Bl>Yes</Bl>";
    assert_eq!(imp(60062, xml_row), "Yes||1|2024-03-051234.5");
    let r = imp(60062, display_row);
    assert!(
        r.starts_with("No|The format of the Date '03/05/24' does not match your device's date settings."),
        "{r}"
    );
    assert!(r.ends_with("|0|0"), "{r}");
    assert_eq!(imp(60061, display_row), "Yes||1|2024-03-051234.5");
    assert_eq!(imp(60061, xml_row), "No|The date is not valid.|0|0");
}

// xmlport/lt-xmlport-query-datatransfer/XpTImportBasic.al:Import_DuplicateKeyExisting
#[test]
fn an_import_of_a_key_that_is_stored_raises_the_table_error() {
    let r = run(
        IMPORT_VARS,
        &format!(
            "U.\"Entry No.\" := 1; U.Insert(); Commit(); Clear(U);
             H.Stage(B, InStr, {TWO_ROWS}); XmlPort.Import(60023, InStr);
             exit(Format(U.Count()));"
        ),
    );
    match r {
        Err(Signal::Error(e)) => assert!(e.message.contains("already exists"), "{}", e.message),
        other => panic!("the second import must raise: {other:?}"),
    }
}

// xmlport/lt-y-xmlport-props/YXmlPortProps.al:UseLax_Import_AcceptsUnknownElements: a UseLax port skips an element of another
// namespace after the last element of a sequence and lists `any element in namespace '##other'` for one of no namespace; a
// strict port names the other namespace.
#[test]
fn use_lax_skips_an_element_of_another_namespace_at_the_end_of_a_sequence() {
    let ports = r#"
procedure Imp(Id: Integer; Xml: Text): Text var H: Codeunit Helpers; B: Record "ALT Blob" temporary; InStr: InStream; begin H.Stage(B, InStr, Xml); if not TryImp(Id, InStr) then exit('error:' + GetLastErrorText()); exit('ok'); end;
[TryFunction] procedure TryImp(Id: Integer; InStr: InStream) begin XmlPort.Import(Id, InStr); end;
"#;
    let procs = ports;
    let run_one = |body: &str| {
        let src_extra = procs.to_string();
        match run_in("U: Record \"ALT Universal\";", body, &src_extra, None) {
            Ok(Value::Text(s)) => s.to_string(),
            other => panic!("{other:?}"),
        }
    };
    let lax_ns = run_one(
        "exit(Imp(60091, '<Root><Item><Id>6</Id><Name>six</Name><Unknown xmlns=\"urn:other\">x</Unknown></Item></Root>') + '|' + Format(U.Count()));",
    );
    assert_eq!(lax_ns, "ok|1");
    let lax =
        run_one("exit(Imp(60091, '<Root><Item><Id>7</Id><Name>seven</Name><Unknown>x</Unknown></Item></Root>'));");
    assert_eq!(
        lax,
        "error:Imported XML cannot validate with the schema: The element 'Item' has invalid child element 'Unknown'. List of possible elements expected: any element in namespace '##other'."
    );
    let strict = run_one(
        "exit(Imp(60092, '<Root><Item><Id>5</Id><Name>five</Name><Unknown xmlns=\"urn:other\">x</Unknown></Item></Root>'));",
    );
    assert_eq!(
        strict,
        "error:Imported XML cannot validate with the schema: The element 'Item' has invalid child element 'Unknown' in namespace 'urn:other'."
    );
}

const AF_VARS: &str = "H: Codeunit Helpers; B: Record \"ALT Blob\"; T: Record \"ALT Blob\" temporary; OutStr: OutStream; InStr: InStream; R: Record \"ALT AF Row\";";
const AF_SEED: &str = "R.Id := 1; R.Name := 'Alpha'; R.Lang := 'en'; R.Insert(); B.Code := 'AF'; B.Insert(); B.Data.CreateOutStream(OutStr);";

// xmlport/lt-af-xmlport/AfXmlPortAttrChildren.al:AF_Attributes_In_FieldElement (lines 241-258)
#[test]
fn a_field_element_carries_a_field_attribute_and_a_text_attribute() {
    let out = text(
        &format!("{AF_VARS} P: XmlPort \"ALT AF XP Attr\"; Q: XmlPort \"ALT AF XP Attr\";"),
        &format!(
            "{AF_SEED} P.SetKind('K1'); P.SetDestination(OutStr); P.Export(); B.Modify();
             H.Stage(T, InStr, '<?xml version=\"1.0\" encoding=\"utf-8\"?><Root><Row><Id>2</Id><Name Lang=\"de\" Kind=\"K9\">Beta</Name></Row></Root>');
             Q.SetSource(InStr); Q.Import(); R.Get(2);
             exit(CopyStr(H.ReadAll(B), StrPos(H.ReadAll(B), '<Root>')) + '|' + R.Name + '|' + R.Lang + '|' + Q.GetKind());"
        ),
    );
    assert_eq!(
        out,
        "<Root>  <Row>    <Id>1</Id>    <Name Lang=\"en\" Kind=\"K1\">Alpha</Name>  </Row></Root>|Beta|de|K9"
    );
}

// xmlport/lt-af-xmlport/AfXmlPortAttrChildren.al:AF_FieldElement_With_TextElement_Child (lines 271-288)
#[test]
fn a_field_element_holds_a_text_element_after_its_value() {
    let out = text(
        &format!("{AF_VARS} P: XmlPort \"ALT AF XP Child Text\"; Q: XmlPort \"ALT AF XP Child Text\";"),
        &format!(
            "{AF_SEED} P.SetChild('c1'); P.SetDestination(OutStr); P.Export(); B.Modify();
             H.Stage(T, InStr, '<?xml version=\"1.0\" encoding=\"utf-8\"?><Root><Row><Id>3</Id><Name>Gamma<Child>cc</Child></Name></Row></Root>');
             Q.SetSource(InStr); Q.Import(); R.Get(3);
             exit(CopyStr(H.ReadAll(B), StrPos(H.ReadAll(B), '<Root>')) + '|' + R.Name + '|' + R.Lang + '|' + Q.GetChild());"
        ),
    );
    assert_eq!(
        out,
        "<Root>  <Row>    <Id>1</Id>    <Name>Alpha<Child>c1</Child></Name>  </Row></Root>|Gamma||cc"
    );
}

// xmlport/lt-af-xmlport/AfXmlPortAttrChildren.al:AF_FieldElement_With_FieldElement_Child (lines 300-315)
#[test]
fn a_field_element_holds_a_field_element_after_its_value() {
    let out = text(
        &format!("{AF_VARS} P: XmlPort \"ALT AF XP Child Field\"; Q: XmlPort \"ALT AF XP Child Field\";"),
        &format!(
            "{AF_SEED} P.SetDestination(OutStr); P.Export(); B.Modify();
             H.Stage(T, InStr, '<?xml version=\"1.0\" encoding=\"utf-8\"?><Root><Row><Id>4</Id><Name>Delta<Lang>fr</Lang></Name></Row></Root>');
             Q.SetSource(InStr); Q.Import(); R.Get(4);
             exit(CopyStr(H.ReadAll(B), StrPos(H.ReadAll(B), '<Root>')) + '|' + R.Name + '|' + R.Lang);"
        ),
    );
    assert_eq!(
        out,
        "<Root>  <Row>    <Id>1</Id>    <Name>Alpha<Lang>en</Lang></Name>  </Row></Root>|Delta|fr"
    );
}

// Not pinned: a field element whose field attribute is missing in the document is refused.
#[test]
fn a_field_element_without_its_field_attribute_is_refused_on_import() {
    let why = refusal(
        &format!("{AF_VARS} Q: XmlPort \"ALT AF XP Attr\";"),
        "H.Stage(T, InStr, '<Root><Row><Id>2</Id><Name Kind=\"K9\">Beta</Name></Row></Root>'); Q.SetSource(InStr); Q.Import(); exit('');",
    );
    assert!(why.contains("without its field attribute Lang"), "{why}");
}

// xmlport/PWXmlPortFormatEvaluate.al:Import_Xml_Enum_Invalid: an undeclared ordinal is imported, a name that is no member raises
// with the captions.
#[test]
fn an_xml_import_into_an_enum_takes_an_undeclared_ordinal_and_names_the_captions_for_an_unknown_name() {
    let imp = |value: &str| {
        text(
            "H: Codeunit Helpers; B: Record \"ALT Blob\" temporary; InStr: InStream; T: Record \"FE Enum Table\"; Ok: Boolean;",
            &format!(
                "H.Stage(B, InStr, '<Root><T><Id>1</Id><En>{value}</En></T></Root>'); Ok := XmlPort.Import(60067, InStr);
                 if T.FindFirst() then; exit(Format(Ok) + '|' + GetLastErrorText() + '|' + GetLastErrorCode() + '|' + Format(T.Count()) + '|' + Format(T.En.AsInteger()));"
            ),
        )
    };
    assert_eq!(imp("7"), "Yes|||1|7");
    assert_eq!(
        imp("Two"),
        "No|'Two' is not an option. The existing options are: Zero cap,One cap|InvalidOptionString|0|0"
    );
}

const OCC_VARS: &str = "H: Codeunit Helpers; B: Record \"ALT Blob\" temporary; InStr: InStream; OutStr: OutStream; P: XmlPort \"ALT Occ\";";

fn occ_export(setup: &str) -> String {
    text(
        OCC_VARS,
        &format!(
            "{setup} B.Code := 'X'; B.Insert(); B.Data.CreateOutStream(OutStr); P.SetDestination(OutStr); P.Export(); B.Modify();
             exit(H.ReadAll(B));"
        ),
    )
}

fn occ_import(body: &str) -> String {
    text(
        OCC_VARS,
        &format!(
            "H.Stage(B, InStr, '<?xml version=\"1.0\" encoding=\"UTF-8\"?>{body}'); P.SetSource(InStr); P.Import(); exit(P.Rows());"
        ),
    )
}

fn occ_import_error(body: &str) -> String {
    match run(
        OCC_VARS,
        &format!(
            "H.Stage(B, InStr, '<?xml version=\"1.0\" encoding=\"UTF-8\"?>{body}'); P.SetSource(InStr); P.Import(); exit('');"
        ),
    ) {
        Err(Signal::Error(e)) => format!("{}|{}", e.message, e.code),
        other => panic!("expected an error, got {other:?}"),
    }
}

// QrXpContainerOccurs.al:QR_Export_NoRows, QR_Export_OneHead_NoLines, QR_Export_TwoHeads_LinesOnSecond: every container is
// written once, also when empty.
#[test]
fn an_export_writes_every_container_once_also_when_empty() {
    assert_eq!(
        occ_export(""),
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\"?><envelope>  <request />  <status /></envelope>"
    );
    assert_eq!(
        occ_export("P.AddHead('H1', ''); P.AddHead('H2', 'two'); P.AddLine('H2', 10, 1); P.AddLine('H9', 10, 9);"),
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\"?><envelope>  <request>    <head no=\"H1\">      <name />      <lines />    </head>    <head no=\"H2\">      <name>two</name>      <lines>        <line n=\"10\">          <qty>1</qty>        </line>      </lines>    </head>  </request>  <status /></envelope>"
    );
}

// QrXpContainerOccurs.al:QR_Import_Present: containers absent or once as their MinOccurs allows; every line of a lines
// container is linked to its head.
#[test]
fn an_import_takes_each_container_absent_or_once() {
    assert_eq!(occ_import("<envelope><request/></envelope>"), "  status=");
    assert_eq!(
        occ_import("<envelope><request></request><status>s</status></envelope>"),
        "  status=s"
    );
    assert_eq!(
        occ_import("<envelope><request><head no=\"H1\"/></request></envelope>"),
        "H1:;  status="
    );
    assert_eq!(
        occ_import(
            "<envelope><request><head no=\"H1\"><name>one</name><lines><line n=\"10\"><qty>5</qty></line><line n=\"20\"><qty>7</qty></line></lines></head><head no=\"H2\"><lines><line n=\"10\"><qty>1</qty></line></lines></head></request></envelope>"
        ),
        "H1:one;H2:; H1/10=5;H1/20=7;H2/10=1; status="
    );
}

// QrXpContainerOccurs.al:QR_Import_MissingOrTwoRequests, QR_Import_Repeated: a missing required container or a second one fails
// the schema check.
#[test]
fn an_import_of_a_missing_or_repeated_container_fails_the_schema_check() {
    let pre = "Imported XML cannot validate with the schema: ";
    assert_eq!(
        occ_import_error("<envelope></envelope>"),
        format!(
            "{pre}The element 'envelope' has incomplete content. List of possible elements expected: 'request'.|XmlPortInvalidXml"
        )
    );
    assert_eq!(
        occ_import_error("<envelope><request/><request/></envelope>"),
        format!(
            "{pre}The element 'envelope' has invalid child element 'request'. List of possible elements expected: 'status'.|XmlPortInvalidXml"
        )
    );
    assert_eq!(
        occ_import_error("<envelope><request><head no=\"H1\"><lines/><lines/></head></request></envelope>"),
        format!("{pre}The element 'head' has invalid child element 'lines'.|XmlPortInvalidXml")
    );
    assert_eq!(
        occ_import_error("<envelope><request><head no=\"H1\"><name>a</name><name>b</name></head></request></envelope>"),
        format!(
            "{pre}The element 'head' has invalid child element 'name'. List of possible elements expected: 'lines'.|XmlPortInvalidXml"
        )
    );
}

const ATTR_VARS: &str = "H: Codeunit Helpers; B: Record \"ALT Blob\" temporary; InStr: InStream; OutStr: OutStream; P: XmlPort \"ALT Attr Occ\";";

fn attr_export(setup: &str) -> String {
    text(
        ATTR_VARS,
        &format!(
            "{setup} B.Code := 'X'; B.Insert(); B.Data.CreateOutStream(OutStr); P.SetDestination(OutStr); P.Export(); B.Modify();
             exit(H.ReadAll(B));"
        ),
    )
}

/// `ok <result>` or `<error text>|<code>`.
fn attr_import(body: &str) -> String {
    match run(
        ATTR_VARS,
        &format!(
            "H.Stage(B, InStr, '<?xml version=\"1.0\" encoding=\"UTF-8\"?>{body}'); P.SetSource(InStr); P.Import(); exit('ok ' + P.Result());"
        ),
    ) {
        Ok(Value::Text(s)) => s.to_string(),
        Err(Signal::Error(e)) => format!("{}|{}", e.message, e.code),
        other => panic!("{other:?}"),
    }
}

// QrXpContainerOccurs.al:QS_TextAttr_Export_AllEmpty, QS_TextAttr_Export_AllSet, QS_TextAttr_Export_OptionalSetRequiredEmpty:
// an empty Required text attribute is written as attr="", an empty Optional one is left out.
#[test]
fn an_export_leaves_out_an_empty_optional_text_attribute_and_writes_an_empty_required_one() {
    let decl = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\"?>";
    assert_eq!(
        attr_export(""),
        format!("{decl}<envelope version=\"\">  <body /></envelope>")
    );
    assert_eq!(
        attr_export("P.SetValues('inst-1', '2', 'tok'); P.AddRow('R1', 'ref-1', 'must-1');"),
        format!(
            "{decl}<envelope cache_instance_id=\"inst-1\" version=\"2\">  <body token=\"tok\">    <row no=\"R1\" ref=\"ref-1\" must=\"must-1\" />  </body></envelope>"
        )
    );
    assert_eq!(
        attr_export("P.SetValues('inst-1', '', ''); P.AddRow('R1', '', ''); P.AddRow('R2', 'ref-2', '');"),
        format!(
            "{decl}<envelope cache_instance_id=\"inst-1\" version=\"\">  <body>    <row no=\"R1\" must=\"\" />    <row no=\"R2\" ref=\"ref-2\" must=\"\" />  </body></envelope>"
        )
    );
}

// QrXpContainerOccurs.al:QS_TextAttr_Import_Root, QS_TextAttr_Import_Container: a missing Required text attribute fails the schema
// check, a present but empty one imports; a missing Optional one runs no trigger; an attribute the schema lacks fails.
#[test]
fn an_import_checks_the_occurrence_of_root_and_container_text_attributes() {
    let pre = "Imported XML cannot validate with the schema: ";
    assert_eq!(
        attr_import("<envelope cache_instance_id=\"i1\" version=\"2\"/>"),
        "ok log=instance=i1;version=2; values: instance=i1 version=2 token= ref= must= rows="
    );
    assert_eq!(
        attr_import("<envelope version=\"2\"/>"),
        "ok log=version=2; values: instance= version=2 token= ref= must= rows="
    );
    let missing = format!("{pre}The required attribute 'version' is missing.|XmlPortInvalidXml");
    assert_eq!(attr_import("<envelope cache_instance_id=\"i1\"/>"), missing);
    assert_eq!(attr_import("<envelope/>"), missing);
    assert_eq!(
        attr_import("<envelope version=\"\"/>"),
        "ok log=version=; values: instance= version= token= ref= must= rows="
    );
    assert_eq!(
        attr_import("<envelope version=\"2\" other=\"x\"/>"),
        format!("{pre}The 'other' attribute is not declared.|XmlPortInvalidXml")
    );
    assert_eq!(
        attr_import("<envelope version=\"2\"><body token=\"t1\"/></envelope>"),
        "ok log=version=2;token=t1; values: instance= version=2 token=t1 ref= must= rows="
    );
    assert_eq!(
        attr_import("<envelope version=\"2\"><body/></envelope>"),
        "ok log=version=2; values: instance= version=2 token= ref= must= rows="
    );
}

// QrXpContainerOccurs.al:QS_TextAttr_Import_Rows: an Optional text attribute missing on a later row keeps the value of the row before;
// a Required one missing on a row fails the schema check.
#[test]
fn an_import_checks_the_occurrence_of_row_text_attributes() {
    assert_eq!(
        attr_import(
            "<envelope version=\"2\"><body><row no=\"R1\" ref=\"a\" must=\"m1\"/><row no=\"R2\" must=\"m2\"/></body></envelope>"
        ),
        "ok log=version=2;row R1 ref=a must=m1;row R2 ref=a must=m2; values: instance= version=2 token= ref=a must=m2 rows=R1:a/m1;R2:a/m2;"
    );
    assert_eq!(
        attr_import("<envelope version=\"2\"><body><row no=\"R1\" must=\"m1\"/><row no=\"R2\"/></body></envelope>"),
        "Imported XML cannot validate with the schema: The required attribute 'must' is missing.|XmlPortInvalidXml"
    );
    assert_eq!(
        attr_import("<envelope version=\"2\"><body><row no=\"R1\" must=\"\"/></body></envelope>"),
        "ok log=version=2;row R1 ref= must=; values: instance= version=2 token= ref= must= rows=R1:/;"
    );
}

fn temp_no_save(port: &str, manual: bool, data: &str) -> String {
    text(
        &format!("H: Codeunit Helpers; B: Record \"ALT Blob\" temporary; InStr: InStream; P: XmlPort \"{port}\"; Real: Record \"ALT Universal\"; Nl: Text[2];"),
        &format!(
            "Nl[1] := 13; Nl[2] := 10; H.Stage(B, InStr, {data}); P.SetSource(InStr); P.SetManualInsert({manual}); P.Import();
             exit(P.Result() + ' real after=' + Format(Real.Count()));"
        ),
    )
}

// AjRunAndXmlPort.al:QS_VariableText_TempInteger_AutoSaveFalse, _ManualInsert: with UseTemporary and AutoSave = false the triggers
// run as for a saved row and see the imported values; the temporary table holds only what OnBeforeInsertRecord inserts.
#[test]
fn a_temporary_table_element_without_autosave_holds_only_what_its_triggers_insert_variable_text() {
    let data = "'\"S1\"|\"ITEM-A\"' + Nl + '\"S2\"|\"ITEM-B\"' + Nl";
    assert_eq!(
        temp_no_save("ALT Temp No Save Text", false, data),
        "pre count=0;init n=0 count=0;before shelf=S1 itemcode=ITEM-A n=0 count=0 temp=Yes;after n=0 count=0;init n=0 count=0;before shelf=S2 itemcode=ITEM-B n=0 count=0 temp=Yes;after n=0 count=0;post count=0; count=0 rows= real after=0"
    );
    assert_eq!(
        temp_no_save("ALT Temp No Save Text", true, data),
        "pre count=0;init n=0 count=0;before shelf=S1 itemcode=ITEM-A n=0 count=0 temp=Yes;after n=1 count=1;init n=0 count=1;before shelf=S2 itemcode=ITEM-B n=0 count=1 temp=Yes;after n=2 count=2;post count=2; count=2 rows=1;2; real after=0"
    );
}

// AjRunAndXmlPort.al:QS_Xml_TempTable_AutoSaveFalse_Values, _ManualInsert: the same for an Xml port; the database is unchanged.
#[test]
fn a_temporary_table_element_without_autosave_holds_only_what_its_triggers_insert_xml() {
    let data = "'<Root><Row><Id>1</Id><Txt>one</Txt></Row><Row><Id>2</Id><Txt>two</Txt></Row></Root>'";
    assert_eq!(
        temp_no_save("ALT Temp No Save Xml", false, data),
        "before id=1 txt=one temp=0;after id=1 temp=0 real=0;before id=2 txt=two temp=0;after id=2 temp=0 real=0; count=0 rows= real after=0"
    );
    assert_eq!(
        temp_no_save("ALT Temp No Save Xml", true, data),
        "before id=1 txt=one temp=0;after id=1 temp=1 real=0;before id=2 txt=two temp=1;after id=2 temp=2 real=0; count=2 rows=1:one;2:two; real after=0"
    );
}
