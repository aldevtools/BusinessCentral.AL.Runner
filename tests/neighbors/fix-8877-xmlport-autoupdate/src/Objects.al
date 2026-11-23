// Neighbors of al-runner issue 8877 (corpus PR probe-xmlport-autoupdate-8877, XpAutoUpdateAssign.al): an XmlPort import
// whose parent table element is stored before its linked child rows and stored again after them (no AutoUpdate or
// AutoReplace), where triggers of the parent's attributes assign fields of the parent. The suite xmlport is a stand-in
// for the shape of an AI test suite import; the other xmlports flip one fact each.
table 50100 "NB XAU Par"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Id; Integer) { }
        field(2; Name; Text[30]) { }
        field(3; Stamp; Text[30]) { }
        field(4; Extra; Text[30]) { }
    }

    keys { key(PK; Id) { Clustered = true; } }
}

table 50101 "NB XAU Chi"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Id; Integer) { }
        field(2; ParentId; Integer) { }
        field(3; Note; Text[30]) { }
    }

    keys { key(PK; Id) { Clustered = true; } }
}

table 50102 "NB XAU ChiW"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Id; Integer) { }
        field(2; ParentId; Integer) { }
    }

    keys { key(PK; Id) { Clustered = true; } }

    trigger OnInsert()
    var
        P: Record "NB XAU Par";
    begin
        if P.Get(ParentId) then begin
            P.Stamp := 'db';
            P.Modify();
        end;
    end;
}

table 50103 "NB XAU Suite"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Code; Code[20]) { }
        field(2; Description; Text[50]) { }
        field(3; AppId; Text[30]) { }
        field(4; Hash; Text[30]) { }
        field(5; Validation; Boolean) { }
        field(6; Runner; Integer) { }
    }

    keys { key(PK; Code) { Clustered = true; } }

    trigger OnInsert()
    begin
        if Runner = 0 then
            Runner := 7;
    end;
}

table 50104 "NB XAU Lang"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; SuiteCode; Code[20]) { }
        field(2; Tag; Text[10]) { }
        field(3; Validation; Boolean) { }
    }

    keys { key(PK; SuiteCode, Tag) { Clustered = true; } }
}

table 50105 "NB XAU Line"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; SuiteCode; Code[20]) { }
        field(2; LineNo; Integer) { }
        field(3; Cu; Integer) { }
    }

    keys { key(PK; SuiteCode, LineNo) { Clustered = true; } }
}

table 50106 "NB XAU Eval"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; SuiteCode; Code[20]) { }
        field(2; LineNo; Integer) { }
        field(3; Name; Text[30]) { }
    }

    keys { key(PK; SuiteCode, LineNo, Name) { Clustered = true; } }
}

table 50107 "NB XAU Src"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Id; Integer) { }
        field(2; Data; Blob) { }
    }

    keys { key(PK; Id) { Clustered = true; } }
}

xmlport 50110 "NB XAU Suite"
{
    Direction = Import;
    Format = Xml;
    Encoding = UTF8;
    UseRequestPage = false;

    schema
    {
        textelement(Root)
        {
            tableelement(Suite; "NB XAU Suite")
            {
                MaxOccurs = Unbounded;
                XmlName = 'Suite';

                fieldattribute(Code; Suite.Code)
                {
                    Occurrence = Required;

                    trigger OnAfterAssignField()
                    var
                        Other: Record "NB XAU Suite";
                    begin
                        Other.SetRange(Code, Suite.Code);
                        if Other.FindFirst() then
                            if Other.AppId = CallerApp then
                                if Other.Hash = CallerHash then begin
                                    Skipped.Add(Suite.Code);
                                    currXMLport.Skip();
                                end else
                                    Other.Delete()
                            else
                                Error('Suite %1 belongs to another app.', Suite.Code);
                        Suite.AppId := CallerApp;
                        Suite.Hash := CallerHash;
                    end;
                }
                fieldattribute(Description; Suite.Description) { Occurrence = Optional; }
                textattribute(SuiteValidation)
                {
                    XmlName = 'Validation';
                    Occurrence = Optional;

                    trigger OnAfterAssignVariable()
                    begin
                        Suite.Validation := UpperCase(SuiteValidation) = 'TRUE';
                    end;
                }
                tableelement(Lang; "NB XAU Lang")
                {
                    LinkTable = Suite;
                    LinkFields = SuiteCode = field(Code);
                    MinOccurs = Zero;
                    XmlName = 'Language';

                    textattribute(LangTag)
                    {
                        XmlName = 'Tag';
                        Occurrence = Required;

                        trigger OnAfterAssignVariable()
                        begin
                            Lang.Tag := LangTag;
                        end;
                    }

                    trigger OnAfterInitRecord()
                    begin
                        if Skipped.Contains(Suite.Code) then
                            currXMLport.Skip();
                    end;
                }
                tableelement(SuiteEval; "NB XAU Eval")
                {
                    LinkTable = Suite;
                    LinkFields = SuiteCode = field(Code);
                    MinOccurs = Zero;
                    XmlName = 'Evaluator';
                    SourceTableView = where(LineNo = const(0));

                    fieldattribute(Name; SuiteEval.Name) { Occurrence = Required; }
                }
                tableelement(Line; "NB XAU Line")
                {
                    LinkTable = Suite;
                    LinkFields = SuiteCode = field(Code);
                    MinOccurs = Zero;
                    XmlName = 'Line';

                    fieldattribute(Cu; Line.Cu) { Occurrence = Required; }
                    tableelement(LineEval; "NB XAU Eval")
                    {
                        LinkTable = Line;
                        LinkFields = SuiteCode = field(SuiteCode), LineNo = field(LineNo);
                        MinOccurs = Zero;
                        XmlName = 'Evaluator';

                        fieldattribute(Name; LineEval.Name) { Occurrence = Required; }

                        trigger OnAfterInitRecord()
                        begin
                            if Skipped.Contains(Suite.Code) then
                                currXMLport.Skip();
                        end;

                        trigger OnBeforeInsertRecord()
                        begin
                            LineEval.LineNo := Line.LineNo;
                        end;
                    }

                    trigger OnAfterInitRecord()
                    begin
                        if Skipped.Contains(Suite.Code) then
                            currXMLport.Skip();
                    end;

                    trigger OnBeforeInsertRecord()
                    var
                        Last: Record "NB XAU Line";
                    begin
                        Last.SetRange(SuiteCode, Suite.Code);
                        if Last.FindLast() then;
                        Line.LineNo := Last.LineNo + 10000;
                    end;
                }

                trigger OnBeforeInsertRecord()
                begin
                    if Skipped.Contains(Suite.Code) then
                        currXMLport.Skip();
                end;
            }
        }
    }

    procedure SetCaller(App: Text[30]; HashValue: Text[30])
    begin
        CallerApp := App;
        CallerHash := HashValue;
    end;

    var
        Skipped: List of [Code[20]];
        CallerApp: Text[30];
        CallerHash: Text[30];
}

// No AutoUpdate: a child trigger changes the parent's record variable between its two saves.
xmlport 50111 "NB XAU Stale"
{
    Direction = Import;
    Format = Xml;
    Encoding = UTF8;
    UseRequestPage = false;

    schema
    {
        textelement(Root)
        {
            tableelement(Par; "NB XAU Par")
            {
                XmlName = 'P';

                fieldelement(Id; Par.Id)
                {
                    trigger OnAfterAssignField()
                    begin
                        Par.Stamp := 'trig';
                    end;
                }
                fieldelement(Name; Par.Name) { }
                tableelement(Chi; "NB XAU Chi")
                {
                    XmlName = 'C';
                    LinkTable = Par;
                    LinkFields = ParentId = field(Id);
                    MinOccurs = Zero;

                    fieldelement(CId; Chi.Id) { }

                    trigger OnAfterInitRecord()
                    begin
                        Par.Extra := 'child';
                    end;
                }
            }
        }
    }
}

// No AutoUpdate: the child table's OnInsert changes the stored parent row between its two saves.
xmlport 50112 "NB XAU StaleDb"
{
    Direction = Import;
    Format = Xml;
    Encoding = UTF8;
    UseRequestPage = false;

    schema
    {
        textelement(Root)
        {
            tableelement(Par; "NB XAU Par")
            {
                XmlName = 'P';

                fieldelement(Id; Par.Id)
                {
                    trigger OnAfterAssignField()
                    begin
                        Par.Extra := 'trig';
                    end;
                }
                fieldelement(Name; Par.Name) { }
                tableelement(Chi; "NB XAU ChiW")
                {
                    XmlName = 'C';
                    LinkTable = Par;
                    LinkFields = ParentId = field(Id);
                    MinOccurs = Zero;

                    fieldelement(CId; Chi.Id) { }
                }
            }
        }
    }
}

// No AutoUpdate: OnBeforeModifyRecord of the second save changes a field the document does not carry.
xmlport 50113 "NB XAU BeforeModify"
{
    Direction = Import;
    Format = Xml;
    Encoding = UTF8;
    UseRequestPage = false;

    schema
    {
        textelement(Root)
        {
            tableelement(Par; "NB XAU Par")
            {
                XmlName = 'P';

                fieldelement(Id; Par.Id)
                {
                    trigger OnAfterAssignField()
                    begin
                        Par.Stamp := 'trig';
                    end;
                }
                fieldelement(Name; Par.Name) { }
                tableelement(Chi; "NB XAU Chi")
                {
                    XmlName = 'C';
                    LinkTable = Par;
                    LinkFields = ParentId = field(Id);
                    MinOccurs = Zero;

                    fieldelement(CId; Chi.Id) { }
                }

                trigger OnBeforeModifyRecord()
                begin
                    Par.Extra := 'bm';
                end;
            }
        }
    }
}

// AutoUpdate (not the shape the fix runs): an OnAfterAssignField of the key, an existing row, no child rows.
xmlport 50114 "NB XAU KeyTrig"
{
    Direction = Import;
    Format = Xml;
    Encoding = UTF8;
    UseRequestPage = false;

    schema
    {
        textelement(Root)
        {
            tableelement(Par; "NB XAU Par")
            {
                XmlName = 'P';
                AutoUpdate = true;

                fieldelement(Id; Par.Id)
                {
                    trigger OnAfterAssignField()
                    begin
                        Par.Stamp := 'trig';
                    end;
                }
                fieldelement(Name; Par.Name) { MinOccurs = Zero; }
            }
        }
    }
}

// AutoUpdate on a parent with a linked child table element.
xmlport 50115 "NB XAU NestUpd"
{
    Direction = Import;
    Format = Xml;
    Encoding = UTF8;
    UseRequestPage = false;

    schema
    {
        textelement(Root)
        {
            tableelement(Par; "NB XAU Par")
            {
                XmlName = 'P';
                AutoUpdate = true;

                fieldelement(Id; Par.Id)
                {
                    trigger OnAfterAssignField()
                    begin
                        Par.Stamp := 'trig';
                    end;
                }
                fieldelement(Name; Par.Name) { MinOccurs = Zero; }
                tableelement(Chi; "NB XAU Chi")
                {
                    XmlName = 'C';
                    LinkTable = Par;
                    LinkFields = ParentId = field(Id);
                    MinOccurs = Zero;

                    fieldelement(CId; Chi.Id) { }
                    fieldelement(Note; Chi.Note) { }
                }
            }
        }
    }
}

// No AutoUpdate: the parent with an assign trigger and a linked child, in a temporary table element.
xmlport 50116 "NB XAU TempPar"
{
    Direction = Import;
    Format = Xml;
    Encoding = UTF8;
    UseRequestPage = false;

    schema
    {
        textelement(Root)
        {
            tableelement(Par; "NB XAU Par")
            {
                XmlName = 'P';
                UseTemporary = true;

                fieldelement(Id; Par.Id)
                {
                    trigger OnAfterAssignField()
                    begin
                        Par.Stamp := 'trig';
                    end;
                }
                fieldelement(Name; Par.Name) { }
                tableelement(Chi; "NB XAU Chi")
                {
                    XmlName = 'C';
                    LinkTable = Par;
                    LinkFields = ParentId = field(Id);
                    MinOccurs = Zero;

                    fieldelement(CId; Chi.Id) { }
                }
            }
        }
    }

    procedure Rows(): Text
    var
        Txt: Text;
    begin
        if Par.FindSet() then
            repeat
                Txt += Format(Par.Id) + '/' + Par.Name + '/' + Par.Stamp + ';';
            until Par.Next() = 0;
        exit(Txt);
    end;
}

// No AutoUpdate: OnAfterAssignField of the key field element assigns a field; the parent is stored again after its child rows.
xmlport 50117 "NB XAU Again"
{
    Direction = Import;
    Format = Xml;
    Encoding = UTF8;
    UseRequestPage = false;

    schema
    {
        textelement(Root)
        {
            tableelement(Par; "NB XAU Par")
            {
                XmlName = 'P';

                fieldelement(Id; Par.Id)
                {
                    trigger OnAfterAssignField()
                    begin
                        Par.Stamp := 'trig';
                    end;
                }
                fieldelement(Name; Par.Name) { }
                tableelement(Chi; "NB XAU Chi")
                {
                    XmlName = 'C';
                    LinkTable = Par;
                    LinkFields = ParentId = field(Id);
                    MinOccurs = Zero;

                    fieldelement(CId; Chi.Id) { }
                }
            }
        }
    }
}
