// Review round 1 of the XmlPort import paths of al-runner issue 8877 (corpus PR 1550, XpAutoUpdateAssign.al): the shapes
// the reviewer named. Each xmlport flips one fact of a measured shape. The suite table has no OnInsert, so the record and
// the stored row agree on every field the document does not carry.
table 50200 "NR Src"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Id; Integer) { }
        field(2; Data; Blob) { }
    }

    keys { key(PK; Id) { Clustered = true; } }
}

table 50201 "NR Suite"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Code; Code[20]) { }
        field(2; Description; Text[50]) { }
        field(3; AppId; Text[30]) { }
        field(4; Runner; Integer) { }
    }

    keys { key(PK; Code) { Clustered = true; } }
}

table 50202 "NR Line"
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

table 50203 "NR Eval"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; SuiteCode; Code[20]) { }
        field(2; LineNo; Integer) { }
        field(3; Name; Text[30]) { }
        field(4; Kind; Integer) { }
    }

    keys { key(PK; SuiteCode, LineNo, Name) { Clustered = true; } }
}

table 50204 "NR Par"
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

table 50205 "NR Chi"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Id; Integer) { }
        field(2; ParentId; Integer) { }
    }

    keys { key(PK; Id) { Clustered = true; } }
}

// Again_FirstSaveSkipped_UncarriedSame: the key attribute's trigger assigns a field, OnBeforeInsertRecord skips a suite
// that exists, one linked child row. BC does not store a skipped suite again (Suite_SameAppSameHash_Skips, trace without `bm:`).
xmlport 50210 "NR SkipBefore"
{
    Direction = Import;
    Format = Xml;
    Encoding = UTF8;
    UseRequestPage = false;

    schema
    {
        textelement(Root)
        {
            tableelement(Suite; "NR Suite")
            {
                MaxOccurs = Unbounded;
                XmlName = 'Suite';

                fieldattribute(Code; Suite.Code)
                {
                    Occurrence = Required;

                    trigger OnAfterAssignField()
                    begin
                        Suite.AppId := 'app1';
                    end;
                }
                fieldattribute(Description; Suite.Description) { Occurrence = Optional; }
                tableelement(Line; "NR Line")
                {
                    LinkTable = Suite;
                    LinkFields = SuiteCode = field(Code);
                    MinOccurs = Zero;
                    XmlName = 'Line';

                    fieldattribute(Cu; Line.Cu) { Occurrence = Required; }

                    trigger OnBeforeInsertRecord()
                    var
                        Last: Record "NR Line";
                    begin
                        Last.SetRange(SuiteCode, Suite.Code);
                        if Last.FindLast() then;
                        Line.LineNo := Last.LineNo + 10000;
                    end;
                }

                trigger OnBeforeInsertRecord()
                var
                    Other: Record "NR Suite";
                begin
                    if Other.Get(Suite.Code) then
                        currXMLport.Skip();
                end;
            }
        }
    }
}

// Again_FirstSaveSkippedByAttribute: the same, the key attribute's trigger skips the suite that exists.
xmlport 50211 "NR SkipAttr"
{
    Direction = Import;
    Format = Xml;
    Encoding = UTF8;
    UseRequestPage = false;

    schema
    {
        textelement(Root)
        {
            tableelement(Suite; "NR Suite")
            {
                MaxOccurs = Unbounded;
                XmlName = 'Suite';

                fieldattribute(Code; Suite.Code)
                {
                    Occurrence = Required;

                    trigger OnAfterAssignField()
                    var
                        Other: Record "NR Suite";
                    begin
                        if Other.Get(Suite.Code) then
                            currXMLport.Skip();
                        Suite.AppId := 'app1';
                    end;
                }
                fieldattribute(Description; Suite.Description) { Occurrence = Optional; }
                tableelement(Line; "NR Line")
                {
                    LinkTable = Suite;
                    LinkFields = SuiteCode = field(Code);
                    MinOccurs = Zero;
                    XmlName = 'Line';

                    fieldattribute(Cu; Line.Cu) { Occurrence = Required; }

                    trigger OnBeforeInsertRecord()
                    var
                        Last: Record "NR Line";
                    begin
                        Last.SetRange(SuiteCode, Suite.Code);
                        if Last.FindLast() then;
                        Line.LineNo := Last.LineNo + 10000;
                    end;
                }
            }
        }
    }
}

// ViewConst_OtherTriggerReadsRecord: the measured linked const element; OnPostXmlPort reads the element's record.
xmlport 50212 "NR ViewOtherRead"
{
    Direction = Import;
    Format = Xml;
    Encoding = UTF8;
    UseRequestPage = false;

    schema
    {
        textelement(Root)
        {
            tableelement(Suite; "NR Suite")
            {
                XmlName = 'Suite';

                fieldattribute(Code; Suite.Code) { Occurrence = Required; }
                tableelement(SuiteEval; "NR Eval")
                {
                    LinkTable = Suite;
                    LinkFields = SuiteCode = field(Code);
                    MinOccurs = Zero;
                    XmlName = 'Evaluator';
                    SourceTableView = where(LineNo = const(0));
                    fieldattribute(Name; SuiteEval.Name) { Occurrence = Required; }
                }
            }
        }
    }

    trigger OnPostXmlPort()
    begin
        Seen := SuiteEval.GetFilters();
    end;

    var
        Seen: Text;
}

// ViewConst_NoLinkFields: LinkTable without LinkFields.
xmlport 50213 "NR ViewNoLinkFields"
{
    Direction = Import;
    Format = Xml;
    Encoding = UTF8;
    UseRequestPage = false;

    schema
    {
        textelement(Root)
        {
            tableelement(Suite; "NR Suite")
            {
                XmlName = 'Suite';

                fieldattribute(Code; Suite.Code) { Occurrence = Required; }
                tableelement(SuiteEval; "NR Eval")
                {
                    LinkTable = Suite;
                    MinOccurs = Zero;
                    XmlName = 'Evaluator';
                    SourceTableView = where(LineNo = const(0));
                    fieldattribute(Name; SuiteEval.Name) { Occurrence = Required; }
                }
            }
        }
    }
}

// ViewConst_FilterEq: `filter(0)` instead of `const(0)`.
xmlport 50214 "NR ViewFilterEq"
{
    Direction = Import;
    Format = Xml;
    Encoding = UTF8;
    UseRequestPage = false;

    schema
    {
        textelement(Root)
        {
            tableelement(Suite; "NR Suite")
            {
                XmlName = 'Suite';

                fieldattribute(Code; Suite.Code) { Occurrence = Required; }
                tableelement(SuiteEval; "NR Eval")
                {
                    LinkTable = Suite;
                    LinkFields = SuiteCode = field(Code);
                    MinOccurs = Zero;
                    XmlName = 'Evaluator';
                    SourceTableView = where(LineNo = filter(0));
                    fieldattribute(Name; SuiteEval.Name) { Occurrence = Required; }
                }
            }
        }
    }
}

// ViewConst_TwoRows: the measured element; the document holds two of its rows under one suite.
xmlport 50215 "NR View"
{
    Direction = Import;
    Format = Xml;
    Encoding = UTF8;
    UseRequestPage = false;

    schema
    {
        textelement(Root)
        {
            tableelement(Suite; "NR Suite")
            {
                XmlName = 'Suite';

                fieldattribute(Code; Suite.Code) { Occurrence = Required; }
                tableelement(SuiteEval; "NR Eval")
                {
                    LinkTable = Suite;
                    LinkFields = SuiteCode = field(Code);
                    MinOccurs = Zero;
                    XmlName = 'Evaluator';
                    SourceTableView = where(LineNo = const(0));
                    fieldattribute(Name; SuiteEval.Name) { Occurrence = Required; }
                }
            }
        }
    }
}

// ViewConst_ParentAutoUpdate: AutoUpdate on the suite element.
xmlport 50216 "NR ViewParentUpd"
{
    Direction = Import;
    Format = Xml;
    Encoding = UTF8;
    UseRequestPage = false;

    schema
    {
        textelement(Root)
        {
            tableelement(Suite; "NR Suite")
            {
                XmlName = 'Suite';
                AutoUpdate = true;

                fieldattribute(Code; Suite.Code) { Occurrence = Required; }
                tableelement(SuiteEval; "NR Eval")
                {
                    LinkTable = Suite;
                    LinkFields = SuiteCode = field(Code);
                    MinOccurs = Zero;
                    XmlName = 'Evaluator';
                    SourceTableView = where(LineNo = const(0));
                    fieldattribute(Name; SuiteEval.Name) { Occurrence = Required; }
                }
            }
        }
    }
}

// ViewConst_TwoLinkFields: a second pair in LinkFields.
xmlport 50217 "NR ViewTwoLinkFields"
{
    Direction = Import;
    Format = Xml;
    Encoding = UTF8;
    UseRequestPage = false;

    schema
    {
        textelement(Root)
        {
            tableelement(Suite; "NR Suite")
            {
                XmlName = 'Suite';

                fieldattribute(Code; Suite.Code) { Occurrence = Required; }
                tableelement(SuiteEval; "NR Eval")
                {
                    LinkTable = Suite;
                    LinkFields = SuiteCode = field(Code), Kind = field(Runner);
                    MinOccurs = Zero;
                    XmlName = 'Evaluator';
                    SourceTableView = where(LineNo = const(0));
                    fieldattribute(Name; SuiteEval.Name) { Occurrence = Required; }
                }
            }
        }
    }
}

// Again_TextElementTrigger: stored again after its child rows; the assign trigger sits on a leaf text element.
xmlport 50218 "NR AgainText"
{
    Direction = Import;
    Format = Xml;
    Encoding = UTF8;
    UseRequestPage = false;

    schema
    {
        textelement(Root)
        {
            tableelement(Par; "NR Par")
            {
                XmlName = 'P';

                fieldelement(Id; Par.Id) { }
                textelement(Tag)
                {
                    trigger OnAfterAssignVariable()
                    begin
                        Par.Stamp := Tag;
                    end;
                }
                tableelement(Chi; "NR Chi")
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

// UpdInit_Attributes: AutoUpdate, an OnAfterInitRecord that assigns an uncarried field, the fields as attributes.
xmlport 50219 "NR UpdInitAttr"
{
    Direction = Import;
    Format = Xml;
    Encoding = UTF8;
    UseRequestPage = false;

    schema
    {
        textelement(Root)
        {
            tableelement(Par; "NR Par")
            {
                XmlName = 'P';
                AutoUpdate = true;

                fieldattribute(Id; Par.Id) { }
                fieldattribute(Name; Par.Name) { Occurrence = Optional; }

                trigger OnAfterInitRecord()
                begin
                    Par.Extra := 'init';
                end;
            }
        }
    }
}

// UpdText_WithAttribute: AutoUpdate, the key's assign trigger, a text element with an attribute below it.
xmlport 50220 "NR UpdTextAttr"
{
    Direction = Import;
    Format = Xml;
    Encoding = UTF8;
    UseRequestPage = false;

    schema
    {
        textelement(Root)
        {
            tableelement(Par; "NR Par")
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
                textelement(Info)
                {
                    MinOccurs = Zero;

                    textattribute(Kind) { Occurrence = Optional; }

                    trigger OnAfterAssignVariable()
                    begin
                        Par.Name := Info;
                    end;
                }
            }
        }
    }
}

// UpdKey_PlainAttribute: AutoUpdate, the key field element's assign trigger, a field attribute without a trigger beside it.
xmlport 50221 "NR UpdPlainAttr"
{
    Direction = Import;
    Format = Xml;
    Encoding = UTF8;
    UseRequestPage = false;

    schema
    {
        textelement(Root)
        {
            tableelement(Par; "NR Par")
            {
                XmlName = 'P';
                AutoUpdate = true;

                fieldattribute(Name; Par.Name) { Occurrence = Optional; }
                fieldelement(Id; Par.Id)
                {
                    trigger OnAfterAssignField()
                    begin
                        Par.Stamp := 'trig';
                    end;
                }
            }
        }
    }
}

// UpdInit_BeforeModifyChangesCarried: AutoUpdate, an OnAfterInitRecord whose values equal the stored row, an
// OnBeforeModifyRecord that changes a carried field. Main's path.
xmlport 50222 "NR UpdInitBm"
{
    Direction = Import;
    Format = Xml;
    Encoding = UTF8;
    UseRequestPage = false;

    schema
    {
        textelement(Root)
        {
            tableelement(Par; "NR Par")
            {
                XmlName = 'P';
                AutoUpdate = true;

                fieldelement(Id; Par.Id) { }
                fieldelement(Name; Par.Name) { MinOccurs = Zero; }

                trigger OnAfterInitRecord()
                begin
                    Par.Stamp := 's0';
                    Par.Extra := 'e0';
                end;

                trigger OnBeforeModifyRecord()
                begin
                    Par.Name := 'bm';
                end;
            }
        }
    }
}

// UpdInit_BeforeModifyRestoresUncarried: the OnAfterInitRecord value of an uncarried field differs from the stored row;
// OnBeforeModifyRecord writes the stored value back and changes a carried field.
xmlport 50223 "NR UpdInitBmRestore"
{
    Direction = Import;
    Format = Xml;
    Encoding = UTF8;
    UseRequestPage = false;

    schema
    {
        textelement(Root)
        {
            tableelement(Par; "NR Par")
            {
                XmlName = 'P';
                AutoUpdate = true;

                fieldelement(Id; Par.Id) { }
                fieldelement(Name; Par.Name) { MinOccurs = Zero; }

                trigger OnAfterInitRecord()
                begin
                    Par.Stamp := 's0';
                    Par.Extra := 'init';
                end;

                trigger OnBeforeModifyRecord()
                begin
                    Par.Extra := 'e0';
                    Par.Name := 'bm';
                end;
            }
        }
    }
}

codeunit 50250 "NR Lib"
{
    procedure Run(PortId: Integer; Xml: Text)
    var
        Src: Record "NR Src";
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

    procedure Wipe()
    var
        Suite: Record "NR Suite";
        Line: Record "NR Line";
        Eval: Record "NR Eval";
        Par: Record "NR Par";
        Chi: Record "NR Chi";
    begin
        Suite.DeleteAll();
        Line.DeleteAll();
        Eval.DeleteAll();
        Par.DeleteAll();
        Chi.DeleteAll();
    end;

    procedure SeedPar()
    var
        Par: Record "NR Par";
    begin
        Wipe();
        Par.Id := 1;
        Par.Name := 'old';
        Par.Stamp := 's0';
        Par.Extra := 'e0';
        Par.Insert();
    end;

    procedure SuiteRow(Code: Code[20]): Text
    var
        Suite: Record "NR Suite";
    begin
        Suite.Get(Code);
        exit(Suite.Code + '/' + Suite.Description + '/' + Suite.AppId);
    end;

    procedure EvalRows(): Text
    var
        Eval: Record "NR Eval";
        Txt: Text;
    begin
        if Eval.FindSet() then
            repeat
                Txt += Eval.SuiteCode + '/' + Format(Eval.LineNo) + '/' + Eval.Name + ';';
            until Eval.Next() = 0;
        exit(Txt);
    end;

    procedure ParRow(Id: Integer): Text
    var
        Par: Record "NR Par";
    begin
        Par.Get(Id);
        exit(Format(Par.Id) + '/' + Par.Name + '/' + Par.Stamp + '/' + Par.Extra);
    end;

    procedure AreEqual(Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('expected <%1>, got <%2>', Expected, Actual);
    end;
}
