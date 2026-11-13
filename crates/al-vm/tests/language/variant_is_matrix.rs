//! Every Variant.IsXxx method asked of a Variant that holds a value of each kind (al-runner issue 7963; corpus PR 1382,
//! types/variant/VariantIsMatrix.al:135-493, the probe's codeunit with its Observer calls turned into a returned text). The
//! methods the registry did not have (IsTransactionType, IsDataClassificationType, IsSecurityFiltering) and IsOption of a
//! TransactionType mark `variant-is-matrix`; kinds outside the probe refuse in the three new methods. The probe's Label
//! and TextConst cells (:186, both IsTextConstant only) are left out: a TextConst into a Variant stays refused, and a Label in a
//! Variant is still a Text (not changed here).

use al_builtins::assumptions::{Assumption, take_hits};

const SRC: &str = r#"
enum 69971 "LT VaM Color"
{
    Extensible = false;
    value(0; Red) { }
    value(1; Green) { }
}

table 69972 "LT VaM Row"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Id; Integer) { }
        field(2; Name; Text[30]) { }
        field(3; Data; Blob) { }
    }

    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

codeunit 69973 "LT VaM Is Matrix"
{

    var
        SomeLabel: Label 'label text';
        SomeTextConst: TextConst ENU = 'const text';

    local procedure AllFlags(V: Variant): Text
    begin
        exit(FlagsA(V) + FlagsB(V) + FlagsC(V));
    end;

    local procedure FlagsA(V: Variant) R: Text
    begin
        if V.IsAction() then R += 'Action,';
        if V.IsAutomation() then R += 'Automation,';
        if V.IsBigInteger() then R += 'BigInteger,';
        if V.IsBinary() then R += 'Binary,';
        if V.IsBoolean() then R += 'Boolean,';
        if V.IsByte() then R += 'Byte,';
        if V.IsChar() then R += 'Char,';
        if V.IsClientType() then R += 'ClientType,';
        if V.IsCode() then R += 'Code,';
        if V.IsCodeunit() then R += 'Codeunit,';
        if V.IsDataClassification() then R += 'DataClassification,';
        if V.IsDataClassificationType() then R += 'DataClassificationType,';
        if V.IsDate() then R += 'Date,';
        if V.IsDateFormula() then R += 'DateFormula,';
        if V.IsDateTime() then R += 'DateTime,';
        if V.IsDecimal() then R += 'Decimal,';
        if V.IsDefaultLayout() then R += 'DefaultLayout,';
        if V.IsDictionary() then R += 'Dictionary,';
        if V.IsDotNet() then R += 'DotNet,';
        if V.IsDuration() then R += 'Duration,';
        if V.IsExecutionMode() then R += 'ExecutionMode,';
    end;

    local procedure FlagsB(V: Variant) R: Text
    begin
        if V.IsFieldRef() then R += 'FieldRef,';
        if V.IsFile() then R += 'File,';
        if V.IsFilterPageBuilder() then R += 'FilterPageBuilder,';
        if V.IsGuid() then R += 'Guid,';
        if V.IsInStream() then R += 'InStream,';
        if V.IsInteger() then R += 'Integer,';
        if V.IsJsonArray() then R += 'JsonArray,';
        if V.IsJsonObject() then R += 'JsonObject,';
        if V.IsJsonToken() then R += 'JsonToken,';
        if V.IsJsonValue() then R += 'JsonValue,';
        if V.IsList() then R += 'List,';
        if V.IsNotification() then R += 'Notification,';
        if V.IsObjectType() then R += 'ObjectType,';
        if V.IsOption() then R += 'Option,';
        if V.IsOutStream() then R += 'OutStream,';
        if V.IsPromptMode() then R += 'PromptMode,';
        if V.IsRecord() then R += 'Record,';
        if V.IsRecordId() then R += 'RecordId,';
        if V.IsRecordRef() then R += 'RecordRef,';
        if V.IsReportFormat() then R += 'ReportFormat,';
        if V.IsSecurityFiltering() then R += 'SecurityFiltering,';
        if V.IsTableConnectionType() then R += 'TableConnectionType,';
        if V.IsTestPermissions() then R += 'TestPermissions,';
        if V.IsText() then R += 'Text,';
        if V.IsTextBuilder() then R += 'TextBuilder,';
        if V.IsTextConstant() then R += 'TextConstant,';
        if V.IsTextEncoding() then R += 'TextEncoding,';
        if V.IsTime() then R += 'Time,';
        if V.IsTransactionType() then R += 'TransactionType,';
        if V.IsWideChar() then R += 'WideChar,';
    end;

    local procedure FlagsC(V: Variant) R: Text
    begin
        if V.IsXmlAttribute() then R += 'XmlAttribute,';
        if V.IsXmlAttributeCollection() then R += 'XmlAttributeCollection,';
        if V.IsXmlCData() then R += 'XmlCData,';
        if V.IsXmlComment() then R += 'XmlComment,';
        if V.IsXmlDeclaration() then R += 'XmlDeclaration,';
        if V.IsXmlDocument() then R += 'XmlDocument,';
        if V.IsXmlDocumentType() then R += 'XmlDocumentType,';
        if V.IsXmlElement() then R += 'XmlElement,';
        if V.IsXmlNamespaceManager() then R += 'XmlNamespaceManager,';
        if V.IsXmlNameTable() then R += 'XmlNameTable,';
        if V.IsXmlNode() then R += 'XmlNode,';
        if V.IsXmlNodeList() then R += 'XmlNodeList,';
        if V.IsXmlProcessingInstruction() then R += 'XmlProcessingInstruction,';
        if V.IsXmlReadOptions() then R += 'XmlReadOptions,';
        if V.IsXmlText() then R += 'XmlText,';
        if V.IsXmlWriteOptions() then R += 'XmlWriteOptions,';
    end;

    procedure TransactionType_AllValues() Out: Text
    var
        TT: TransactionType;
        V: Variant;
    begin
        V := TT;
        Out += 'default=' + AllFlags(V) + ';';
        TT := TransactionType::Browse;
        V := TT;
        Out += 'browse=' + AllFlags(V) + ';';
        TT := TransactionType::Snapshot;
        V := TT;
        Out += 'snapshot=' + AllFlags(V) + ';';
        TT := TransactionType::Update;
        V := TT;
        Out += 'update=' + AllFlags(V) + ';';
        TT := TransactionType::UpdateNoLocks;
        V := TT;
        Out += 'updatenolocks=' + AllFlags(V) + ';';
        TT := TransactionType::Report;
        V := TT;
        Out += 'report=' + AllFlags(V) + ';';
        V := TransactionType::Update;
        Out += 'literal=' + AllFlags(V) + ';';
        V := CurrentTransactionType();
        Out += 'current=' + AllFlags(V) + ';';
        V := Format(TransactionType::Update);
        Out += 'formatted=' + AllFlags(V) + ';';
    end;

    procedure Unassigned_Variant() Out: Text
    var
        V: Variant;
    begin
        Out += 'unassigned=' + AllFlags(V) + ';';
    end;

    procedure Simple_Kinds() Out: Text
    var
        Bo: Boolean;
        I: Integer;
        Bi: BigInteger;
        D: Decimal;
        By: Byte;
        Ch: Char;
        T: Text;
        C: Code[10];
        O: Option A,B;
        E: Enum "LT VaM Color";
        V: Variant;
    begin
        Bo := true;
        V := Bo;
        Out += 'boolean=' + AllFlags(V) + ';';
        I := 7;
        V := I;
        Out += 'integer=' + AllFlags(V) + ';';
        Bi := 7;
        V := Bi;
        Out += 'biginteger=' + AllFlags(V) + ';';
        D := 1.5;
        V := D;
        Out += 'decimal=' + AllFlags(V) + ';';
        By := 7;
        V := By;
        Out += 'byte=' + AllFlags(V) + ';';
        Ch := 'x';
        V := Ch;
        Out += 'char=' + AllFlags(V) + ';';
        T := 'abc';
        V := T;
        Out += 'text=' + AllFlags(V) + ';';
        C := 'ABC';
        V := C;
        Out += 'code=' + AllFlags(V) + ';';
        O := O::B;
        V := O;
        Out += 'option=' + AllFlags(V) + ';';
        E := E::Green;
        V := E;
        Out += 'enum=' + AllFlags(V) + ';';
    end;

    procedure Date_Time_Kinds() Out: Text
    var
        Dt: Date;
        Tm: Time;
        DtTm: DateTime;
        Du: Duration;
        DF: DateFormula;
        G: Guid;
        V: Variant;
    begin
        Dt := 20240115D;
        V := Dt;
        Out += 'date=' + AllFlags(V) + ';';
        Tm := 101500T;
        V := Tm;
        Out += 'time=' + AllFlags(V) + ';';
        DtTm := CreateDateTime(20240115D, 101500T);
        V := DtTm;
        Out += 'datetime=' + AllFlags(V) + ';';
        Du := 1000;
        V := Du;
        Out += 'duration=' + AllFlags(V) + ';';
        Evaluate(DF, '<1D>');
        V := DF;
        Out += 'dateformula=' + AllFlags(V) + ';';
        G := CreateGuid();
        V := G;
        Out += 'guid=' + AllFlags(V) + ';';
    end;

    procedure Record_Kinds() Out: Text
    var
        Row: Record "LT VaM Row";
        RecRef: RecordRef;
        Fld: FieldRef;
        Rid: RecordId;
        Cu: Codeunit "LT VaM Is Matrix";
        V: Variant;
    begin
        Row.DeleteAll();
        Row.Id := 1;
        Row.Insert();
        V := Row;
        Out += 'record=' + AllFlags(V) + ';';
        Rid := Row.RecordId();
        V := Rid;
        Out += 'recordid=' + AllFlags(V) + ';';
        RecRef.GetTable(Row);
        V := RecRef;
        Out += 'recordref=' + AllFlags(V) + ';';
        Fld := RecRef.Field(2);
        V := Fld;
        Out += 'fieldref=' + AllFlags(V) + ';';
        V := Cu;
        Out += 'codeunit=' + AllFlags(V) + ';';
    end;

    procedure Stream_Builder_Collection_Kinds() Out: Text
    var
        Row: Record "LT VaM Row" temporary;
        InS: InStream;
        OutS: OutStream;
        TB: TextBuilder;
        L: List of [Text];
        Dict: Dictionary of [Text, Text];
        N: Notification;
        FPB: FilterPageBuilder;
        V: Variant;
    begin
        Row.Data.CreateOutStream(OutS);
        V := OutS;
        Out += 'outstream=' + AllFlags(V) + ';';
        Row.Data.CreateInStream(InS);
        V := InS;
        Out += 'instream=' + AllFlags(V) + ';';
        TB.Append('x');
        V := TB;
        Out += 'textbuilder=' + AllFlags(V) + ';';
        L.Add('x');
        V := L;
        Out += 'list=' + AllFlags(V) + ';';
        Dict.Add('k', 'v');
        V := Dict;
        Out += 'dictionary=' + AllFlags(V) + ';';
        N.Message('m');
        V := N;
        Out += 'notification=' + AllFlags(V) + ';';
        V := FPB;
        Out += 'filterpagebuilder=' + AllFlags(V) + ';';
    end;

    procedure Json_Kinds() Out: Text
    var
        JO: JsonObject;
        JA: JsonArray;
        JV: JsonValue;
        JT: JsonToken;
        V: Variant;
    begin
        JO.Add('a', 1);
        V := JO;
        Out += 'jsonobject=' + AllFlags(V) + ';';
        JA.Add(1);
        V := JA;
        Out += 'jsonarray=' + AllFlags(V) + ';';
        JV.SetValue(1);
        V := JV;
        Out += 'jsonvalue=' + AllFlags(V) + ';';
        JO.Get('a', JT);
        V := JT;
        Out += 'jsontoken=' + AllFlags(V) + ';';
    end;

    procedure Platform_Option_Kinds() Out: Text
    var
        Ac: Action;
        Ct: ClientType;
        Dc: DataClassification;
        Dl: DefaultLayout;
        Em: ExecutionMode;
        Ot: ObjectType;
        Rf: ReportFormat;
        Tc: TableConnectionType;
        Tp: TestPermissions;
        Te: TextEncoding;
        Pm: PromptMode;
        V: Variant;
    begin
        V := Ac;
        Out += 'action=' + AllFlags(V) + ';';
        V := Ct;
        Out += 'clienttype=' + AllFlags(V) + ';';
        V := Dc;
        Out += 'dataclassification=' + AllFlags(V) + ';';
        V := Dl;
        Out += 'defaultlayout=' + AllFlags(V) + ';';
        V := Em;
        Out += 'executionmode=' + AllFlags(V) + ';';
        V := Ot;
        Out += 'objecttype=' + AllFlags(V) + ';';
        V := Rf;
        Out += 'reportformat=' + AllFlags(V) + ';';
        V := Tc;
        Out += 'tableconnectiontype=' + AllFlags(V) + ';';
        V := Tp;
        Out += 'testpermissions=' + AllFlags(V) + ';';
        V := Te;
        Out += 'textencoding=' + AllFlags(V) + ';';
        V := Pm;
        Out += 'promptmode=' + AllFlags(V) + ';';
    end;

    procedure Xml_Kinds_A() Out: Text
    var
        Doc: XmlDocument;
        El: XmlElement;
        At: XmlAttribute;
        Tx: XmlText;
        Cd: XmlCData;
        Cm: XmlComment;
        Dc: XmlDeclaration;
        Pi: XmlProcessingInstruction;
        V: Variant;
    begin
        Doc := XmlDocument.Create();
        V := Doc;
        Out += 'xmldocument=' + AllFlags(V) + ';';
        El := XmlElement.Create('a');
        V := El;
        Out += 'xmlelement=' + AllFlags(V) + ';';
        At := XmlAttribute.Create('k', 'v');
        V := At;
        Out += 'xmlattribute=' + AllFlags(V) + ';';
        Tx := XmlText.Create('t');
        V := Tx;
        Out += 'xmltext=' + AllFlags(V) + ';';
        Cd := XmlCData.Create('c');
        V := Cd;
        Out += 'xmlcdata=' + AllFlags(V) + ';';
        Cm := XmlComment.Create('c');
        V := Cm;
        Out += 'xmlcomment=' + AllFlags(V) + ';';
        Dc := XmlDeclaration.Create('1.0', 'utf-8', 'yes');
        V := Dc;
        Out += 'xmldeclaration=' + AllFlags(V) + ';';
        Pi := XmlProcessingInstruction.Create('pi', 'd');
        V := Pi;
        Out += 'xmlprocessinginstruction=' + AllFlags(V) + ';';
    end;

    procedure Xml_Kinds_B() Out: Text
    var
        El: XmlElement;
        Dt: XmlDocumentType;
        Nd: XmlNode;
        Nl: XmlNodeList;
        Ac: XmlAttributeCollection;
        Nm: XmlNamespaceManager;
        Nt: XmlNameTable;
        Ro: XmlReadOptions;
        Wo: XmlWriteOptions;
        V: Variant;
    begin
        El := XmlElement.Create('a');
        El.Add(XmlElement.Create('b'));
        El.SetAttribute('k', 'v');
        Dt := XmlDocumentType.Create('a');
        V := Dt;
        Out += 'xmldocumenttype=' + AllFlags(V) + ';';
        Nd := El.AsXmlNode();
        V := Nd;
        Out += 'xmlnode=' + AllFlags(V) + ';';
        Nl := El.GetChildNodes();
        V := Nl;
        Out += 'xmlnodelist=' + AllFlags(V) + ';';
        Ac := El.Attributes();
        V := Ac;
        Out += 'xmlattributecollection=' + AllFlags(V) + ';';
        V := Nm;
        Out += 'xmlnamespacemanager=' + AllFlags(V) + ';';
        V := Nt;
        Out += 'xmlnametable=' + AllFlags(V) + ';';
        V := Ro;
        Out += 'xmlreadoptions=' + AllFlags(V) + ';';
        V := Wo;
        Out += 'xmlwriteoptions=' + AllFlags(V) + ';';
    end;
}

"#;

const EXPECTED: &[(&str, &str)] = &[
    (
        "TransactionType_AllValues",
        "default=TransactionType,;browse=TransactionType,;snapshot=TransactionType,;update=TransactionType,;updatenolocks=TransactionType,;report=TransactionType,;literal=TransactionType,;current=TransactionType,;formatted=Text,;",
    ),
    ("Unassigned_Variant", "unassigned=;"),
    (
        "Simple_Kinds",
        "boolean=Boolean,;integer=Integer,;biginteger=BigInteger,;decimal=Decimal,;byte=Byte,;char=Char,WideChar,;text=Text,;code=Code,;option=Option,;enum=Option,;",
    ),
    (
        "Date_Time_Kinds",
        "date=Date,;time=Time,;datetime=DateTime,;duration=Duration,;dateformula=DateFormula,;guid=Guid,;",
    ),
    (
        "Record_Kinds",
        "record=Record,;recordid=RecordId,;recordref=RecordRef,;fieldref=FieldRef,;codeunit=Codeunit,;",
    ),
    (
        "Stream_Builder_Collection_Kinds",
        "outstream=OutStream,;instream=InStream,;textbuilder=TextBuilder,;list=List,;dictionary=Dictionary,;notification=Notification,;filterpagebuilder=FilterPageBuilder,;",
    ),
    (
        "Json_Kinds",
        "jsonobject=JsonObject,;jsonarray=JsonArray,;jsonvalue=JsonValue,;jsontoken=JsonToken,;",
    ),
    (
        "Platform_Option_Kinds",
        "action=Action,;clienttype=ClientType,;dataclassification=DataClassification,DataClassificationType,;defaultlayout=DefaultLayout,;executionmode=ExecutionMode,;objecttype=ObjectType,;reportformat=ReportFormat,;tableconnectiontype=TableConnectionType,;testpermissions=TestPermissions,;textencoding=TextEncoding,;promptmode=PromptMode,;",
    ),
    (
        "Xml_Kinds_A",
        "xmldocument=XmlDocument,;xmlelement=XmlElement,;xmlattribute=XmlAttribute,;xmltext=XmlText,;xmlcdata=XmlCData,;xmlcomment=XmlComment,;xmldeclaration=XmlDeclaration,;xmlprocessinginstruction=XmlProcessingInstruction,;",
    ),
    (
        "Xml_Kinds_B",
        "xmldocumenttype=XmlDocumentType,;xmlnode=XmlElement,;xmlnodelist=XmlNodeList,;xmlattributecollection=XmlAttributeCollection,;xmlnamespacemanager=XmlNamespaceManager,;xmlnametable=XmlNameTable,;xmlreadoptions=XmlReadOptions,;xmlwriteoptions=XmlWriteOptions,;",
    ),
];

#[test]
fn each_held_kind_answers_the_measured_is_methods() {
    let mut b = al_testkit::Built::from_source(SRC);
    for (routine, expected) in EXPECTED {
        take_hits();
        let got = al_testkit::text(b.run_in("LT VaM Is Matrix", routine));
        assert_eq!(&got, expected, "{routine}");
        assert!(
            take_hits().contains(&Assumption::VariantIsMatrix),
            "{routine} must mark its assumption"
        );
    }
}

const NEIGHBORS: &str = r#"
codeunit 69974 "LT VaM Neighbors"
{
    procedure Reporter(): Boolean
    var
        Value: Variant;
        Tt: TransactionType;
    begin
        Tt := TransactionType::Update;
        Value := Tt;
        exit(Value.IsTransactionType());
    end;

    procedure HttpClientIsTransactionType(): Boolean
    var
        Client: HttpClient;
        V: Variant;
    begin
        V := Client;
        exit(V.IsTransactionType());
    end;

    procedure AttributeNodeIsDataClassificationType(): Boolean
    var
        Nd: XmlNode;
        V: Variant;
    begin
        Nd := XmlAttribute.Create('k', 'v').AsXmlNode();
        V := Nd;
        exit(V.IsDataClassificationType());
    end;

    procedure AppOptionIsOption(): Boolean
    var
        O: Option A,B;
        V: Variant;
    begin
        V := O;
        exit(V.IsOption());
    end;

    procedure AppOptionWithTheMembersIsOption(): Boolean
    var
        O: Option UpdateNoLocks,Update,Snapshot,Browse,Report;
        V: Variant;
    begin
        O := O::Update;
        V := O;
        exit(V.IsOption());
    end;

    procedure AppOptionWithTheMembersIsTransactionType(): Boolean
    var
        O: Option UpdateNoLocks,Update,Snapshot,Browse,Report;
        V: Variant;
    begin
        O := O::Update;
        V := O;
        exit(V.IsTransactionType());
    end;

    procedure FieldOptionWithTheMembersIsOption(): Boolean
    var
        Row: Record "LT VaM Tt Row";
        V: Variant;
    begin
        Row.Tt := Row.Tt::Browse;
        V := Row.Tt;
        exit(V.IsOption());
    end;

    procedure FieldOptionDefaultIsTransactionType(): Boolean
    var
        Row: Record "LT VaM Tt Row";
        V: Variant;
    begin
        V := Row.Tt;
        exit(V.IsTransactionType());
    end;

    procedure TransactionTypeLiteralIsOption(): Boolean
    var
        V: Variant;
    begin
        V := TransactionType::Update;
        exit(V.IsOption());
    end;
}

table 69975 "LT VaM Tt Row"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Id; Integer) { }
        field(2; Tt; Option) { OptionMembers = UpdateNoLocks,Update,Snapshot,Browse,Report; }
    }

    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}
"#;

#[test]
fn the_reporters_repro_answers_true_flagged() {
    take_hits();
    assert!(al_testkit::boolean(al_testkit::run_in(
        NEIGHBORS,
        "LT VaM Neighbors",
        "Reporter"
    )));
    assert!(take_hits().contains(&Assumption::VariantIsMatrix));
}

#[test]
fn a_kind_the_probe_did_not_hold_refuses_in_the_new_methods() {
    for routine in ["HttpClientIsTransactionType", "AttributeNodeIsDataClassificationType"] {
        let r = al_testkit::run_in(NEIGHBORS, "LT VaM Neighbors", routine);
        assert!(
            matches!(r, Err(al_builtins::native::Signal::Unsupported(_))),
            "{routine}: {r:?}"
        );
    }
}

#[test]
fn is_option_of_an_app_option_keeps_mains_answer_unflagged() {
    take_hits();
    assert!(al_testkit::boolean(al_testkit::run_in(
        NEIGHBORS,
        "LT VaM Neighbors",
        "AppOptionIsOption"
    )));
    assert!(!take_hits().contains(&Assumption::VariantIsMatrix));
}

/// An Option whose members are the five TransactionType members is an Option, not a TransactionType: an app Option and an Option
/// field keep main's IsOption = true, unflagged (round-1 review of al-runner PR 7985, cells B200 and B204; the TransactionType
/// field of the platform table "XmlPort Metadata" is the same kind of value, corpus tests 67450 and 68474).
#[test]
fn an_option_with_the_transaction_type_members_keeps_is_option_unflagged() {
    for routine in ["AppOptionWithTheMembersIsOption", "FieldOptionWithTheMembersIsOption"] {
        take_hits();
        assert!(
            al_testkit::boolean(al_testkit::run_in(NEIGHBORS, "LT VaM Neighbors", routine)),
            "{routine}"
        );
        assert!(!take_hits().contains(&Assumption::VariantIsMatrix), "{routine}");
    }
}

/// IsTransactionType of an Option with the five members keeps main's refusal, also at ordinal 0 (cell X04): only a value whose
/// declared type is TransactionType answers.
#[test]
fn is_transaction_type_of_an_option_with_the_members_refuses() {
    for routine in [
        "AppOptionWithTheMembersIsTransactionType",
        "FieldOptionDefaultIsTransactionType",
    ] {
        let r = al_testkit::run_in(NEIGHBORS, "LT VaM Neighbors", routine);
        assert!(
            matches!(r, Err(al_builtins::native::Signal::Unsupported(_))),
            "{routine}: {r:?}"
        );
    }
}

/// A TransactionType literal in a Variant is no Option (VariantIsMatrix.al:148, `literal=TransactionType,`).
#[test]
fn a_transaction_type_literal_is_no_option() {
    take_hits();
    assert!(!al_testkit::boolean(al_testkit::run_in(
        NEIGHBORS,
        "LT VaM Neighbors",
        "TransactionTypeLiteralIsOption"
    )));
    assert!(take_hits().contains(&Assumption::VariantIsMatrix));
}
