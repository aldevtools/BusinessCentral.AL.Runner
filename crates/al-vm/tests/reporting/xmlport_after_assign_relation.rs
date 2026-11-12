//! An Xml import of a field node with OnAfterAssignField into a Code field with a TableRelation (al-runner issue 7863): the
//! trigger runs first, then the field is validated with the value the trigger left; a temporary table element validates
//! nothing; a blank field attribute runs no trigger. Assumption `xmlport-after-assign-before-validate` (corpus PR 1366,
//! xmlports/import/XpAfterAssignTableRelation.al). The shapes the probe did not measure keep main's result.

use al_builtins::native::Signal;
use al_testkit::{Built, text};

/// The port body: `{TEMP}` the UseTemporary line, `{NODE}` fieldattribute or fieldelement, `{FIELD}` the field, `{PROPS}` the
/// node's properties, `{XP}` the port's properties.
const PORT: &str = r#"
xmlport {ID} "{NAME}"
{
    Direction = Import; Encoding = UTF8; Format = Xml; FormatEvaluate = Xml; UseRequestPage = false; {XP}
    schema
    {
        textelement(Root)
        {
            tableelement(Row; "AAR Row")
            {
                {TEMP}
                fieldattribute(Id; Row.Id) { }
                {NODE}(RelCode; Row."{FIELD}")
                {
                    {PROPS}
                    trigger OnAfterAssignField()
                    begin
                        AarLog.Add('after:' + Row."{FIELD}");
                        case AarLog.GetMode() of
                            1: Row."{FIELD}" := 'P2';
                            2: Row."{FIELD}" := 'MISS';
                            3: currXMLport.Skip();
                        end;
                    end;
                }
                trigger OnBeforeInsertRecord() begin AarLog.Add('before insert:' + Row."{FIELD}"); end;
            }
        }
    }
    var AarLog: Codeunit "AAR Log";
}
"#;

const COMMON: &str = r#"
table 50700 "AAR Rel" { fields { field(1; "Code"; Code[20]) { } } keys { key(PK; "Code") { } } }
table 50701 "AAR Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; "Rel Code"; Code[20]) { TableRelation = "AAR Rel"; trigger OnValidate() begin AarLog.Add('validate:' + "Rel Code"); end; }
        field(3; "Rel Code NV"; Code[20]) { TableRelation = "AAR Rel"; ValidateTableRelation = false; trigger OnValidate() begin AarLog.Add('validate nv:' + "Rel Code NV"); end; }
        field(4; "Rel Code Plain"; Code[20]) { TableRelation = "AAR Rel"; }
        field(5; "Rel No"; Integer) { TableRelation = "AAR Int Rel"; trigger OnValidate() begin AarLog.Add('validate no:' + Format("Rel No")); end; }
        field(6; "Rel Code Where"; Code[20]) { TableRelation = "AAR Rel" where("Code" = const('P1')); trigger OnValidate() begin AarLog.Add('validate where:' + "Rel Code Where"); end; }
        field(7; "Rel Code10"; Code[10]) { TableRelation = "AAR Rel"; trigger OnValidate() begin AarLog.Add('validate c10:' + "Rel Code10"); end; }
        field(8; "Rel Code30"; Code[30]) { TableRelation = "AAR Rel"; trigger OnValidate() begin AarLog.Add('validate c30:' + "Rel Code30"); end; }
        field(9; "Rel Code Named"; Code[20]) { TableRelation = "AAR Rel"."Code"; trigger OnValidate() begin AarLog.Add('validate named:' + "Rel Code Named"); end; }
    }
    keys { key(PK; Id) { } }
    var AarLog: Codeunit "AAR Log";
}
table 50702 "AAR Int Rel" { fields { field(1; No; Integer) { } } keys { key(PK; No) { } } }
table 50703 "AAR Buf" { fields { field(1; Id; Integer) { } field(2; Data; Blob) { } } keys { key(PK; Id) { } } }
codeunit 50704 "AAR Log"
{
    SingleInstance = true;
    var Entries: Text; Mode: Integer;
    procedure Reset(NewMode: Integer) begin Entries := ''; Mode := NewMode; end;
    procedure GetMode(): Integer begin exit(Mode); end;
    procedure Add(Entry: Text) begin Entries += Entry + ';'; end;
    procedure Take(): Text var R: Text; begin R := Entries; Entries := ''; exit(R); end;
}
codeunit 50705 "AAR Run"
{
    [TryFunction]
    local procedure TryImport(XpId: Integer; Xml: Text)
    var B: Record "AAR Buf" temporary; OutS: OutStream; InS: InStream;
    begin
        B.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        OutS.WriteText(Xml);
        B.Data.CreateInStream(InS, TextEncoding::UTF8);
        Xmlport.Import(XpId, InS);
    end;
    local procedure DbRows(): Text
    var Row: Record "AAR Row"; R: Text;
    begin
        if Row.FindSet() then
            repeat
                R += Format(Row.Id) + ':' + Row."Rel Code" + ':' + Row."Rel Code NV" + ',';
            until Row.Next() = 0;
        exit(R);
    end;
    procedure Case(XpId: Integer; IsAttribute: Boolean; Value: Text; Mode: Integer): Text
    var Row: Record "AAR Row"; Rel: Record "AAR Rel"; AarLog: Codeunit "AAR Log"; Xml: Text; Ok: Boolean; R: Text;
    begin
        if IsAttribute then
            Xml := '<?xml version="1.0" encoding="UTF-8"?><Root><Row Id="1" RelCode="' + Value + '" /></Root>'
        else
            Xml := '<?xml version="1.0" encoding="UTF-8"?><Root><Row Id="1"><RelCode>' + Value + '</RelCode></Row></Root>';
        Row.DeleteAll();
        Rel.DeleteAll();
        Rel."Code" := 'P1';
        Rel.Insert();
        Rel."Code" := 'P2';
        Rel.Insert();
        AarLog.Reset(Mode);
        ClearLastError();
        Ok := TryImport(XpId, Xml);
        R := Format(Ok) + ' | ' + GetLastErrorText() + ' | ' + GetLastErrorCode() + ' | ' + AarLog.Take() + ' | ' + DbRows();
        ClearLastError();
        exit(R);
    end;
    procedure CaseAll(XpId: Integer; IsAttribute: Boolean; Value: Text; Mode: Integer): Text
    var Row: Record "AAR Row"; Rel: Record "AAR Rel"; IntRel: Record "AAR Int Rel"; AarLog: Codeunit "AAR Log"; Xml: Text; Ok: Boolean; R: Text;
    begin
        if IsAttribute then
            Xml := '<?xml version="1.0" encoding="UTF-8"?><Root><Row Id="1" RelCode="' + Value + '" /></Root>'
        else
            Xml := '<?xml version="1.0" encoding="UTF-8"?><Root><Row Id="1"><RelCode>' + Value + '</RelCode></Row></Root>';
        Row.DeleteAll();
        Rel.DeleteAll();
        Rel."Code" := 'P1';
        Rel.Insert();
        Rel."Code" := 'P2';
        Rel.Insert();
        IntRel.DeleteAll();
        IntRel.No := 1;
        IntRel.Insert();
        IntRel.No := 2;
        IntRel.Insert();
        AarLog.Reset(Mode);
        ClearLastError();
        Ok := TryImport(XpId, Xml);
        R := Format(Ok) + ' | ' + GetLastErrorText() + ' | ' + GetLastErrorCode() + ' | ' + AarLog.Take() + ' | ';
        if Row.FindSet() then
            repeat
                R += Format(Row.Id) + ':' + Row."Rel Code Plain" + ':' + Format(Row."Rel No") + ':' + Row."Rel Code10" + ',';
            until Row.Next() = 0;
        ClearLastError();
        exit(R);
    end;
    procedure Doc(XpId: Integer; Xml: Text): Text
    var Row: Record "AAR Row"; Rel: Record "AAR Rel"; AarLog: Codeunit "AAR Log"; Ok: Boolean; R: Text;
    begin
        Row.DeleteAll();
        Rel.DeleteAll();
        Rel."Code" := 'P1';
        Rel.Insert();
        AarLog.Reset(0);
        ClearLastError();
        Ok := TryImport(XpId, Xml);
        R := Format(Ok) + ' | ' + GetLastErrorText() + ' | ' + AarLog.Take() + ' | ' + DbRows();
        ClearLastError();
        exit(R);
    end;
    procedure TwoRows(): Text
    var AarLog: Codeunit "AAR Log";
    begin
        AarLog.Reset(0);
        TryImport(50711, '<Root><Row Id="1" RelCode="" /><Row Id="2" RelCode="" /></Root>');
        exit(AarLog.Take());
    end;
}
"#;

struct Port {
    id: u32,
    temp: bool,
    attr: bool,
    field: &'static str,
    props: &'static str,
    xp: &'static str,
}

fn port(p: &Port) -> String {
    PORT.replace("{ID}", &p.id.to_string())
        .replace("{NAME}", &format!("AAR {}", p.id))
        .replace("{TEMP}", if p.temp { "UseTemporary = true;" } else { "" })
        .replace("{NODE}", if p.attr { "fieldattribute" } else { "fieldelement" })
        .replace("{FIELD}", p.field)
        .replace("{PROPS}", p.props)
        .replace("{XP}", p.xp)
}

fn build(p: &Port) -> Built {
    Built::from_source(&format!("{COMMON}{}", port(p)))
}

/// The six cases of XpAfterAssignTableRelation.al:1426-1434: existing, missing, blank (mode 0), missing then P2, existing then
/// MISS, existing then P2.
fn six(p: &Port) -> Vec<Result<String, Signal>> {
    let cases: [(&str, i32); 6] = [("P1", 0), ("MISS", 0), ("", 0), ("MISS", 1), ("P1", 2), ("P1", 1)];
    let mut b = build(p);
    let (oi, ri) = b.routine_in("AAR Run", "Case");
    cases
        .iter()
        .map(|(v, mode)| {
            b.call(
                oi,
                ri,
                vec![
                    al_builtins::value::Value::Integer(p.id as i32),
                    al_builtins::value::Value::Boolean(p.attr),
                    al_builtins::value::Value::Text((*v).into()),
                    al_builtins::value::Value::Integer(*mode),
                ],
            )
            .map(|(v, _)| text(Ok(v)))
        })
        .collect()
}

fn ok_six(p: &Port) -> Vec<String> {
    al_builtins::assumptions::take_hits();
    let got: Vec<String> = six(p)
        .into_iter()
        .map(|r| r.unwrap_or_else(|e| panic!("port {}: {e:?}", p.id)))
        .collect();
    let hits = al_builtins::assumptions::take_hits();
    assert!(
        hits.contains(&al_builtins::assumptions::Assumption::XmlportAfterAssignBeforeValidate),
        "port {}: {hits:?}",
        p.id
    );
    got
}

const MISSING: &str = "No | The field Rel Code of table AAR Row contains a value (MISS) that cannot be found in the related table (AAR Rel). | DB:PrimRecordNotFound";

fn temp(attr: bool, field: &'static str, props: &'static str) -> Port {
    Port {
        id: 50710,
        temp: true,
        attr,
        field,
        props,
        xp: "",
    }
}

fn db(attr: bool, field: &'static str, props: &'static str) -> Port {
    Port {
        id: 50711,
        temp: false,
        attr,
        field,
        props,
        xp: "",
    }
}

/// XpAfterAssignTableRelation.al:1437 (attribute), :1450 (element), :1463 (FieldValidate = No), :1476 (ValidateTableRelation =
/// false): a temporary table element validates nothing; a blank attribute runs no trigger, a blank element does.
#[test]
fn a_temporary_table_element_runs_the_trigger_and_validates_nothing() {
    for (p, blank) in [
        (temp(true, "Rel Code", ""), "Yes |  |  | before insert:; | "),
        (temp(false, "Rel Code", ""), "Yes |  |  | after:;before insert:; | "),
        (
            temp(true, "Rel Code", "FieldValidate = No;"),
            "Yes |  |  | before insert:; | ",
        ),
        (temp(true, "Rel Code NV", ""), "Yes |  |  | before insert:; | "),
    ] {
        assert_eq!(
            ok_six(&p),
            [
                "Yes |  |  | after:P1;before insert:P1; | ",
                "Yes |  |  | after:MISS;before insert:MISS; | ",
                blank,
                "Yes |  |  | after:MISS;before insert:P2; | ",
                "Yes |  |  | after:P1;before insert:MISS; | ",
                "Yes |  |  | after:P1;before insert:P2; | ",
            ],
            "{}",
            p.props
        );
    }
}

/// XpAfterAssignTableRelation.al:1489 (attribute) and :1501 (element): the trigger, then the TableRelation check and OnValidate
/// with the value it left.
#[test]
fn a_stored_table_element_validates_the_value_the_trigger_left() {
    for (attr, blank) in [
        (true, "Yes |  |  | before insert:; | 1::,"),
        (false, "Yes |  |  | after:;validate:;before insert:; | 1::,"),
    ] {
        assert_eq!(
            ok_six(&db(attr, "Rel Code", "")),
            [
                "Yes |  |  | after:P1;validate:P1;before insert:P1; | 1:P1:,".to_string(),
                format!("{MISSING} | after:MISS; | "),
                blank.to_string(),
                "Yes |  |  | after:MISS;validate:P2;before insert:P2; | 1:P2:,".to_string(),
                format!("{MISSING} | after:P1; | "),
                "Yes |  |  | after:P1;validate:P2;before insert:P2; | 1:P2:,".to_string(),
            ],
            "attr {attr}"
        );
    }
}

/// XpAfterAssignTableRelation.al:1514 (FieldValidate = No) and :1527 (ValidateTableRelation = false) on a stored table element.
#[test]
fn a_stored_table_element_without_the_check() {
    assert_eq!(
        ok_six(&db(true, "Rel Code", "FieldValidate = No;")),
        [
            "Yes |  |  | after:P1;before insert:P1; | 1:P1:,",
            "Yes |  |  | after:MISS;before insert:MISS; | 1:MISS:,",
            "Yes |  |  | before insert:; | 1::,",
            "Yes |  |  | after:MISS;before insert:P2; | 1:P2:,",
            "Yes |  |  | after:P1;before insert:MISS; | 1:MISS:,",
            "Yes |  |  | after:P1;before insert:P2; | 1:P2:,",
        ]
    );
    assert_eq!(
        ok_six(&db(true, "Rel Code NV", "")),
        [
            "Yes |  |  | after:P1;validate nv:P1;before insert:P1; | 1::P1,",
            "Yes |  |  | after:MISS;validate nv:MISS;before insert:MISS; | 1::MISS,",
            "Yes |  |  | before insert:; | 1::,",
            "Yes |  |  | after:MISS;validate nv:P2;before insert:P2; | 1::P2,",
            "Yes |  |  | after:P1;validate nv:MISS;before insert:MISS; | 1::MISS,",
            "Yes |  |  | after:P1;validate nv:P2;before insert:P2; | 1::P2,",
        ]
    );
}

fn refused(p: &Port) -> String {
    match six(p).remove(0) {
        Err(Signal::Unsupported(why)) => why.to_string(),
        other => panic!("port {}: {other:?}", p.id),
    }
}

/// Shapes the probe did not measure keep main's refusal: a field element into the field without the check, a relation with
/// a where clause; a nested table element is covered by the predicate's position fact.
#[test]
fn unmeasured_validated_shapes_keep_mains_refusal() {
    for p in [
        db(false, "Rel Code NV", ""),
        temp(false, "Rel Code NV", ""),
        db(true, "Rel Code Where", ""),
    ] {
        let why = refused(&p);
        assert!(
            why.contains("which runs first is not pinned"),
            "{} {}: {why}",
            p.field,
            p.props
        );
    }
}

/// DefaultFieldsValidation = false is not measured: the port refuses where the probe would change main's answer (a blank
/// attribute: main runs the trigger, the measured shapes do not), and keeps main's answer elsewhere.
#[test]
fn default_fields_validation_false_keeps_mains_answer_or_refuses_the_blank_attribute() {
    let p = Port {
        id: 50712,
        temp: false,
        attr: true,
        field: "Rel Code",
        props: "",
        xp: "DefaultFieldsValidation = false;",
    };
    let got = six(&p);
    assert_eq!(
        got[0].as_ref().unwrap(),
        "Yes |  |  | after:P1;before insert:P1; | 1:P1:,"
    );
    assert!(
        matches!(&got[2], Err(Signal::Unsupported(why)) if why.contains("blank field attribute")),
        "{:?}",
        got[2]
    );
}

/// Two rows in one document are not measured: the second row refuses.
#[test]
fn a_second_row_refuses() {
    let mut b = build(&db(true, "Rel Code", ""));
    match b.run_in("AAR Run", "TwoRows") {
        Err(Signal::Unsupported(why)) => assert!(why.contains("second row"), "{why}"),
        other => panic!("{other:?}"),
    }
}

/// The reporter's repro (al-runner issue 7863), with a Blob field for Temp Blob and the assertion as the routine's result.
const REPRO: &str = r#"
table 50100 "Repro Aa Partner" { fields { field(1; "Code"; Code[20]) { } } keys { key(PK; "Code") { Clustered = true; } } }
table 50101 "Repro Aa Trans"
{
    fields
    {
        field(1; "Transaction No."; Integer) { }
        field(2; "Partner Code"; Code[20]) { TableRelation = "Repro Aa Partner"; }
    }
    keys { key(PK; "Transaction No.") { Clustered = true; } }
}
table 50102 "Repro Buf" { fields { field(1; Id; Integer) { } field(2; Data; Blob) { } } keys { key(PK; Id) { } } }
xmlport 50100 "Repro Aa Import"
{
    Direction = Import;
    Encoding = UTF8;
    FormatEvaluate = Xml;
    schema
    {
        textelement(Root)
        {
            tableelement(Trans; "Repro Aa Trans")
            {
                UseTemporary = true;
                fieldattribute(TransNo; Trans."Transaction No.") { }
                fieldattribute(ToPartnerCode; Trans."Partner Code")
                {
                    trigger OnAfterAssignField()
                    begin
                        ToPartnerCode2 := Trans."Partner Code";
                    end;
                }
            }
        }
    }
    var
        ToPartnerCode2: Code[20];
    procedure GetToPartnerCode(): Code[20]
    begin
        exit(ToPartnerCode2);
    end;
}
codeunit 50150 "Repro After Assign Test"
{
    procedure AfterAssignFieldSeesImportedValue(): Code[20]
    var
        Partner: Record "Repro Aa Partner";
        TempBlob: Record "Repro Buf" temporary;
        AaImport: XmlPort "Repro Aa Import";
        OutStr: OutStream;
        InStr: InStream;
    begin
        Partner."Code" := 'P1';
        Partner.Insert();
        TempBlob.Data.CreateOutStream(OutStr);
        OutStr.WriteText('<?xml version="1.0" encoding="UTF-8"?><Root><Trans TransNo="1" ToPartnerCode="P1" /></Root>');
        TempBlob.Data.CreateInStream(InStr);
        AaImport.SetSource(InStr);
        AaImport.Import();
        exit(AaImport.GetToPartnerCode());
    end;
}
"#;

/// The reporter's repro (al-runner issue 7863, XpAfterAssignTableRelation.al:1642): the trigger sees the imported partner
/// code; the pass is flagged.
#[test]
fn the_reporters_repro_returns_the_imported_partner_code() {
    al_builtins::assumptions::take_hits();
    let got = al_testkit::run_in(REPRO, "Repro After Assign Test", "AfterAssignFieldSeesImportedValue");
    assert_eq!(text(got), "P1");
    assert!(
        al_builtins::assumptions::take_hits()
            .contains(&al_builtins::assumptions::Assumption::XmlportAfterAssignBeforeValidate)
    );
}

/// A subscriber to the field's validate event is not measured: main's refusal stays.
#[test]
fn a_validate_subscriber_keeps_mains_refusal() {
    let sub = r#"
codeunit 50706 "AAR Sub"
{
    [EventSubscriber(ObjectType::Table, Database::"AAR Row", 'OnBeforeValidateEvent', 'Rel Code', false, false)]
    local procedure OnBefore(var Rec: Record "AAR Row") begin end;
}
"#;
    let p = temp(true, "Rel Code", "");
    let mut b = Built::from_source(&format!("{COMMON}{}{sub}", port(&p)));
    let (oi, ri) = b.routine_in("AAR Run", "Case");
    let args = vec![
        al_builtins::value::Value::Integer(p.id as i32),
        al_builtins::value::Value::Boolean(true),
        al_builtins::value::Value::Text("P1".into()),
        al_builtins::value::Value::Integer(0),
    ];
    match b.call(oi, ri, args) {
        Err(Signal::Unsupported(why)) => assert!(why.to_string().contains("which runs first is not pinned"), "{why}"),
        other => panic!("{other:?}"),
    }
}

fn doc(src: &str, id: u32, xml: &str) -> Result<String, Signal> {
    let mut b = Built::from_source(&format!("{COMMON}{src}"));
    let (oi, ri) = b.routine_in("AAR Run", "Doc");
    b.call(
        oi,
        ri,
        vec![
            al_builtins::value::Value::Integer(id as i32),
            al_builtins::value::Value::Text(xml.into()),
        ],
    )
    .map(|(v, _)| text(Ok(v)))
}

/// Each fact of the probe's ports that the shape requires, flipped one at a time on the stored attribute shape: main's
/// refusal stays (XpAfterAssignTableRelation.al:141-1324 declares Direction = Import, Encoding = UTF8, FormatEvaluate = Xml, a
/// table element with nothing but UseTemporary, one node with OnAfterAssignField and no other property).
#[test]
fn unmeasured_port_and_node_facts_keep_mains_refusal() {
    let base = port(&db(true, "Rel Code", ""));
    let occurrence = port(&db(true, "Rel Code", "Occurrence = Optional;"));
    let id_attr = "fieldattribute(Id; Row.Id) { }";
    let second = format!(
        "{id_attr} fieldattribute(RelCode2; Row.\"Rel Code NV\") {{ trigger OnAfterAssignField() begin AarLog.Add('after2'); end; }}"
    );
    let one = r#"<Root><Row Id="1" RelCode="P1" /></Root>"#;
    for (what, src, xml) in [
        (
            "Direction = Both",
            base.replace("Direction = Import;", "Direction = Both;"),
            one,
        ),
        (
            "FormatEvaluate = Legacy",
            base.replace("FormatEvaluate = Xml;", "FormatEvaluate = Legacy;"),
            one,
        ),
        ("no FormatEvaluate", base.replace("FormatEvaluate = Xml;", ""), one),
        ("no Encoding", base.replace("Encoding = UTF8;", ""), one),
        (
            "AutoSave = false",
            base.replace(id_attr, &format!("AutoSave = false; {id_attr}")),
            one,
        ),
        (
            "AutoUpdate = true",
            base.replace(id_attr, &format!("AutoUpdate = true; {id_attr}")),
            one,
        ),
        (
            "MinOccurs = Zero",
            base.replace(id_attr, &format!("MinOccurs = Zero; {id_attr}")),
            one,
        ),
        ("Occurrence", occurrence, one),
        (
            "two nodes with the trigger",
            base.replace(id_attr, &second),
            r#"<Root><Row Id="1" RelCode="P1" RelCode2="P1" /></Root>"#,
        ),
    ] {
        al_builtins::assumptions::take_hits();
        match doc(&src, 50711, xml) {
            Err(Signal::Unsupported(why)) => assert!(why.contains("which runs first is not pinned"), "{what}: {why}"),
            other => panic!("{what}: {other:?}"),
        }
        assert!(
            !al_builtins::assumptions::take_hits()
                .contains(&al_builtins::assumptions::Assumption::XmlportAfterAssignBeforeValidate),
            "{what}"
        );
    }
}

/// Two rows into a field attribute with FieldValidate = No (stored and temporary): main's answer, no validation and then the
/// trigger, which the measured order (the trigger, then no validation; XpAfterAssignTableRelation.al:1463, :1514) does not change
/// for a value that is not blank. A blank second row is not measured and refuses (the first row's blank runs no trigger).
#[test]
fn two_rows_with_field_validate_no_keep_mains_answer() {
    for (p, rows) in [
        (db(true, "Rel Code", "FieldValidate = No;"), "1:P1:,2:MISS:,"),
        (temp(true, "Rel Code", "FieldValidate = No;"), ""),
    ] {
        let src = port(&p);
        let got = doc(
            &src,
            p.id,
            r#"<Root><Row Id="1" RelCode="P1" /><Row Id="2" RelCode="MISS" /></Root>"#,
        );
        assert_eq!(
            got.unwrap(),
            format!("Yes |  | after:P1;before insert:P1;after:MISS;before insert:MISS; | {rows}"),
            "temp {}",
            p.temp
        );
        match doc(
            &src,
            p.id,
            r#"<Root><Row Id="1" RelCode="P1" /><Row Id="2" RelCode="" /></Root>"#,
        ) {
            Err(Signal::Unsupported(why)) => assert!(why.contains("second row"), "{why}"),
            other => panic!("temp {}: {other:?}", p.temp),
        }
    }
}

/// Skip or Break in OnAfterAssignField before the field is validated is not measured: it refuses.
#[test]
fn skip_in_the_trigger_before_the_validation_refuses() {
    let p = db(true, "Rel Code", "");
    let mut b = build(&p);
    let (oi, ri) = b.routine_in("AAR Run", "Case");
    let args = vec![
        al_builtins::value::Value::Integer(p.id as i32),
        al_builtins::value::Value::Boolean(true),
        al_builtins::value::Value::Text("P1".into()),
        al_builtins::value::Value::Integer(3),
    ];
    match b.call(oi, ri, args) {
        Err(Signal::Unsupported(why)) => assert!(why.contains("Skip or Break in OnAfterAssignField"), "{why}"),
        other => panic!("{other:?}"),
    }
}

/// `port` for the Integer field "Rel No": the trigger sets 2 (mode 1) or 99 (mode 2), as XpAfterAssignTableRelation.al
/// ports 77737-77738 do.
fn int_port(p: &Port) -> String {
    int_text(port(p))
}

/// The replacements of `int_port`, on any port text.
fn int_text(src: String) -> String {
    src.replace("'after:' + Row.\"Rel No\"", "'after:' + Format(Row.\"Rel No\")")
        .replace(
            "'before insert:' + Row.\"Rel No\"",
            "'before insert:' + Format(Row.\"Rel No\")",
        )
        .replace(":= 'P2'", ":= 2")
        .replace(":= 'MISS'", ":= 99")
}

/// `port` without OnAfterAssignField (XpAfterAssignTableRelation.al ports 77714, 77716-77719, 77729, 77732-77733, 77739).
fn plain_port(p: &Port) -> String {
    let src = port(p);
    let start = src.find("trigger OnAfterAssignField()").unwrap();
    let end = start + src[start..].find("end;\n                }").unwrap() + "end;\n".len();
    src[..start].to_string() + &src[end..]
}

fn cases(src: &str, id: u32, attr: bool, cases: &[(&str, i32)]) -> Vec<Result<String, Signal>> {
    let mut b = Built::from_source(&format!("{COMMON}{src}"));
    let (oi, ri) = b.routine_in("AAR Run", "CaseAll");
    cases
        .iter()
        .map(|(v, mode)| {
            b.call(
                oi,
                ri,
                vec![
                    al_builtins::value::Value::Integer(id as i32),
                    al_builtins::value::Value::Boolean(attr),
                    al_builtins::value::Value::Text((*v).into()),
                    al_builtins::value::Value::Integer(*mode),
                ],
            )
            .map(|(v, _)| text(Ok(v)))
        })
        .collect()
}

const SIX: [(&str, i32); 6] = [("P1", 0), ("MISS", 0), ("", 0), ("MISS", 1), ("P1", 2), ("P1", 1)];
const INT_SEVEN: [(&str, i32); 7] = [("1", 0), ("99", 0), ("", 0), ("0", 0), ("99", 1), ("1", 2), ("1", 1)];
const PLAIN_THREE: [(&str, i32); 3] = [("P1", 0), ("MISS", 0), ("", 0)];

/// Runs `cases` and checks that every case passes and that `flag` was marked (or, with `None`, that neither assumption of
/// this area was).
fn ok_cases(
    src: &str,
    p: &Port,
    list: &[(&str, i32)],
    flag: Option<al_builtins::assumptions::Assumption>,
) -> Vec<String> {
    use al_builtins::assumptions::{Assumption, take_hits};
    take_hits();
    let got: Vec<String> = cases(src, p.id, p.attr, list)
        .into_iter()
        .map(|r| r.unwrap_or_else(|e| panic!("port {} {}: {e:?}", p.id, p.field)))
        .collect();
    let hits = take_hits();
    match flag {
        Some(a) => assert!(hits.contains(&a), "port {} {}: {hits:?}", p.id, p.field),
        None => assert!(
            !hits.contains(&Assumption::XmlportAfterAssignBeforeValidate)
                && !hits.contains(&Assumption::XmlportTemporaryImportNotValidated),
            "port {} {}: {hits:?}",
            p.id,
            p.field
        ),
    }
    got
}

const AFTER: Option<al_builtins::assumptions::Assumption> =
    Some(al_builtins::assumptions::Assumption::XmlportAfterAssignBeforeValidate);
const NOT_VALIDATED: Option<al_builtins::assumptions::Assumption> =
    Some(al_builtins::assumptions::Assumption::XmlportTemporaryImportNotValidated);

fn missing(field: &str, value: &str, table: &str) -> String {
    format!(
        "No | The field {field} of table AAR Row contains a value ({value}) that cannot be found in the related table ({table}). | DB:PrimRecordNotFound"
    )
}

/// The reporter's shape (al-runner issue 7863), XpAfterAssignTableRelation.al:1654: a temporary table element, a field with a
/// TableRelation and no OnValidate. The trigger sees the imported value; nothing is checked.
#[test]
fn the_reporters_shape_runs_the_trigger_and_validates_nothing() {
    let p = temp(true, "Rel Code Plain", "");
    assert_eq!(
        ok_cases(&port(&p), &p, &SIX, AFTER),
        [
            "Yes |  |  | after:P1;before insert:P1; | ",
            "Yes |  |  | after:MISS;before insert:MISS; | ",
            "Yes |  |  | before insert:; | ",
            "Yes |  |  | after:MISS;before insert:P2; | ",
            "Yes |  |  | after:P1;before insert:MISS; | ",
            "Yes |  |  | after:P1;before insert:P2; | ",
        ]
    );
}

/// XpAfterAssignTableRelation.al:1677: the same field on a stored table element is checked after the trigger, with the value
/// it left.
#[test]
fn a_field_without_on_validate_on_a_stored_table_element_is_checked_after_the_trigger() {
    let p = db(true, "Rel Code Plain", "");
    let miss = missing("Rel Code Plain", "MISS", "AAR Rel");
    assert_eq!(
        ok_cases(&port(&p), &p, &SIX, AFTER),
        [
            "Yes |  |  | after:P1;before insert:P1; | 1:P1:0:,".to_string(),
            format!("{miss} | after:MISS; | "),
            "Yes |  |  | before insert:; | 1::0:,".to_string(),
            "Yes |  |  | after:MISS;before insert:P2; | 1:P2:0:,".to_string(),
            format!("{miss} | after:P1; | "),
            "Yes |  |  | after:P1;before insert:P2; | 1:P2:0:,".to_string(),
        ]
    );
}

/// XpAfterAssignTableRelation.al:1740 (temporary) and :1753 (stored): FieldValidate = Yes on the attribute is the default.
#[test]
fn field_validate_yes_with_the_trigger_is_the_default() {
    let p = temp(true, "Rel Code", "FieldValidate = Yes;");
    assert_eq!(
        ok_six(&p),
        [
            "Yes |  |  | after:P1;before insert:P1; | ",
            "Yes |  |  | after:MISS;before insert:MISS; | ",
            "Yes |  |  | before insert:; | ",
            "Yes |  |  | after:MISS;before insert:P2; | ",
            "Yes |  |  | after:P1;before insert:MISS; | ",
            "Yes |  |  | after:P1;before insert:P2; | ",
        ]
    );
    assert_eq!(
        ok_six(&db(true, "Rel Code", "FieldValidate = Yes;")),
        [
            "Yes |  |  | after:P1;validate:P1;before insert:P1; | 1:P1:,".to_string(),
            format!("{MISSING} | after:MISS; | "),
            "Yes |  |  | before insert:; | 1::,".to_string(),
            "Yes |  |  | after:MISS;validate:P2;before insert:P2; | 1:P2:,".to_string(),
            format!("{MISSING} | after:P1; | "),
            "Yes |  |  | after:P1;validate:P2;before insert:P2; | 1:P2:,".to_string(),
        ]
    );
}

/// XpAfterAssignTableRelation.al:1786 (temporary) and :1799 (stored): a field element with FieldValidate = No runs the
/// trigger, also on a blank element, and validates nothing.
#[test]
fn a_field_element_with_field_validate_no_runs_the_trigger_and_validates_nothing() {
    for (p, rows) in [
        (temp(false, "Rel Code", "FieldValidate = No;"), ["", "", "", "", "", ""]),
        (
            db(false, "Rel Code", "FieldValidate = No;"),
            ["1:P1:,", "1:MISS:,", "1::,", "1:P2:,", "1:MISS:,", "1:P2:,"],
        ),
    ] {
        let logs = [
            "after:P1;before insert:P1;",
            "after:MISS;before insert:MISS;",
            "after:;before insert:;",
            "after:MISS;before insert:P2;",
            "after:P1;before insert:MISS;",
            "after:P1;before insert:P2;",
        ];
        let want: Vec<String> = logs
            .iter()
            .zip(rows)
            .map(|(log, row)| format!("Yes |  |  | {log} | {row}"))
            .collect();
        assert_eq!(ok_six(&p), want, "temp {}", p.temp);
    }
}

/// XpAfterAssignTableRelation.al:1822 (temporary) and :1836 (stored): an Integer field related to an Integer key. A blank
/// attribute runs nothing and leaves 0; the value 0 runs the trigger and OnValidate and passes the check.
#[test]
fn an_integer_field_follows_the_same_order() {
    let p = temp(true, "Rel No", "");
    assert_eq!(
        ok_cases(&int_port(&p), &p, &INT_SEVEN, AFTER),
        [
            "Yes |  |  | after:1;before insert:1; | ",
            "Yes |  |  | after:99;before insert:99; | ",
            "Yes |  |  | before insert:0; | ",
            "Yes |  |  | after:0;before insert:0; | ",
            "Yes |  |  | after:99;before insert:2; | ",
            "Yes |  |  | after:1;before insert:99; | ",
            "Yes |  |  | after:1;before insert:2; | ",
        ]
    );
    let p = db(true, "Rel No", "");
    let miss = missing("Rel No", "99", "AAR Int Rel");
    assert_eq!(
        ok_cases(&int_port(&p), &p, &INT_SEVEN, AFTER),
        [
            "Yes |  |  | after:1;validate no:1;before insert:1; | 1::1:,".to_string(),
            format!("{miss} | after:99; | "),
            "Yes |  |  | before insert:0; | 1::0:,".to_string(),
            "Yes |  |  | after:0;validate no:0;before insert:0; | 1::0:,".to_string(),
            "Yes |  |  | after:99;validate no:2;before insert:2; | 1::2:,".to_string(),
            format!("{miss} | after:1; | "),
            "Yes |  |  | after:1;validate no:2;before insert:2; | 1::2:,".to_string(),
        ]
    );
}

/// XpAfterAssignTableRelation.al:1861: a Code[10] field related to a Code[20] key, stored; a value too long fails at the
/// assignment, before the trigger.
#[test]
fn a_code10_field_follows_the_same_order_and_a_long_value_fails_first() {
    let p = db(true, "Rel Code10", "");
    let miss = missing("Rel Code10", "MISS", "AAR Rel");
    let mut list = SIX.to_vec();
    list.push(("ABCDEFGHIJK", 0));
    assert_eq!(
        ok_cases(&port(&p), &p, &list, AFTER),
        [
            "Yes |  |  | after:P1;validate c10:P1;before insert:P1; | 1::0:P1,".to_string(),
            format!("{miss} | after:MISS; | "),
            "Yes |  |  | before insert:; | 1::0:,".to_string(),
            "Yes |  |  | after:MISS;validate c10:P2;before insert:P2; | 1::0:P2,".to_string(),
            format!("{miss} | after:P1; | "),
            "Yes |  |  | after:P1;validate c10:P2;before insert:P2; | 1::0:P2,".to_string(),
            "No | The length of the string is 11, but it must be less than or equal to 10 characters. Value: ABCDEFGHIJK | StringLengthExceeded |  | ".to_string(),
        ]
    );
}

/// XpAfterAssignTableRelation.al:1700 (attribute), :1720 (element), :1766 (attribute with FieldValidate = Yes), :1667
/// (attribute into the field without OnValidate): without triggers, a temporary table element validates nothing.
#[test]
fn a_temporary_table_element_without_triggers_validates_nothing() {
    for p in [
        temp(true, "Rel Code", ""),
        temp(false, "Rel Code", ""),
        temp(true, "Rel Code", "FieldValidate = Yes;"),
        temp(true, "Rel Code Plain", ""),
    ] {
        assert_eq!(
            ok_cases(&plain_port(&p), &p, &PLAIN_THREE, NOT_VALIDATED),
            [
                "Yes |  |  | before insert:P1; | ",
                "Yes |  |  | before insert:MISS; | ",
                "Yes |  |  | before insert:; | ",
            ],
            "attr {} {} {}",
            p.attr,
            p.field,
            p.props
        );
    }
}

/// Main's answer stays, without either assumption, for the stored layouts without triggers (XpAfterAssignTableRelation.al:1710,
/// :1730, :1850 measure the same values) and for temporary ones the probe did not measure (an Integer field, an element with
/// FieldValidate = Yes, ValidateTableRelation = false): main validates them.
#[test]
fn layouts_without_triggers_outside_the_measured_temporary_ones_keep_mains_answer() {
    let p = db(true, "Rel Code", "");
    assert_eq!(
        ok_cases(&plain_port(&p), &p, &PLAIN_THREE, None),
        [
            "Yes |  |  | validate:P1;before insert:P1; | 1::0:,".to_string(),
            format!("{MISSING} |  | "),
            "Yes |  |  | before insert:; | 1::0:,".to_string(),
        ]
    );
    let p = db(false, "Rel Code", "");
    assert_eq!(
        ok_cases(&plain_port(&p), &p, &PLAIN_THREE, None)[2],
        "Yes |  |  | validate:;before insert:; | 1::0:,"
    );
    let p = db(true, "Rel No", "");
    assert_eq!(
        ok_cases(
            &int_text(plain_port(&p)),
            &p,
            &[("1", 0), ("99", 0), ("", 0), ("0", 0)],
            None
        ),
        [
            "Yes |  |  | validate no:1;before insert:1; | 1::1:,".to_string(),
            format!("{} |  | ", missing("Rel No", "99", "AAR Int Rel")),
            "Yes |  |  | before insert:0; | 1::0:,".to_string(),
            "Yes |  |  | validate no:0;before insert:0; | 1::0:,".to_string(),
        ]
    );
    let p = temp(true, "Rel No", "");
    assert_eq!(
        ok_cases(&int_text(plain_port(&p)), &p, &[("99", 0)], None),
        [format!("{} |  | ", missing("Rel No", "99", "AAR Int Rel"))]
    );
    let p = temp(false, "Rel Code", "FieldValidate = Yes;");
    assert_eq!(
        ok_cases(&plain_port(&p), &p, &[("MISS", 0)], None),
        [format!("{MISSING} |  | ")]
    );
    let p = temp(true, "Rel Code NV", "");
    assert_eq!(
        ok_cases(&plain_port(&p), &p, &[("MISS", 0)], None),
        ["Yes |  |  | validate nv:MISS;before insert:MISS; | "]
    );
    // The temporary layout with one fact the four measured ports (XpAfterAssignTableRelation.al:576, :694, :765, :935) do not
    // have, flipped one at a time: main validates, without the assumption.
    let base = plain_port(&temp(true, "Rel Code", ""));
    let doc_miss = r#"<Root><Row Id="1" RelCode="MISS" /></Root>"#;
    for (shape, src, xml) in [
        (
            "another field attribute with FieldValidate = No",
            base.replace(
                "fieldattribute(Id; Row.Id) { }",
                "fieldattribute(Id; Row.Id) { }\n                fieldattribute(Other; Row.\"Rel Code Plain\") { FieldValidate = No; }",
            ),
            r#"<Root><Row Id="1" Other="P1" RelCode="MISS" /></Root>"#.to_string(),
        ),
        (
            "two measured fields in one row",
            base.replace(
                "fieldattribute(Id; Row.Id) { }",
                "fieldattribute(Id; Row.Id) { }\n                fieldattribute(Other; Row.\"Rel Code Plain\") { }",
            ),
            r#"<Root><Row Id="1" Other="P1" RelCode="MISS" /></Root>"#.to_string(),
        ),
        (
            "the Id attribute with a property",
            base.replace("fieldattribute(Id; Row.Id) { }", "fieldattribute(Id; Row.Id) { Occurrence = Required; }"),
            doc_miss.to_string(),
        ),
        (
            "another field element with MinOccurs = Zero",
            base.replace(
                "fieldattribute(Id; Row.Id) { }",
                "fieldattribute(Id; Row.Id) { }\n                fieldelement(Other; Row.\"Rel Code Plain\") { MinOccurs = Zero; }",
            ),
            doc_miss.to_string(),
        ),
        (
            "a second table element below the root",
            base.replace(
                "        textelement(Root)\n        {\n",
                "        textelement(Root)\n        {\n            tableelement(Rel2; \"AAR Rel\") { MinOccurs = Zero; fieldattribute(C; Rel2.\"Code\") { } }\n",
            ),
            doc_miss.to_string(),
        ),
        (
            "a text element beside the table element",
            base.replace(
                "        textelement(Root)\n        {\n",
                "        textelement(Root)\n        {\n            textelement(Extra) { MinOccurs = Zero; }\n",
            ),
            doc_miss.to_string(),
        ),
        (
            "the root with XmlName = 'Other'",
            base.replace("        textelement(Root)\n        {\n", "        textelement(Root)\n        {\n            XmlName = 'Other';\n"),
            r#"<Other><Row Id="1" RelCode="MISS" /></Other>"#.to_string(),
        ),
        (
            "OnPreXmlPort and OnPostXmlPort",
            base.replace(
                "    var AarLog: Codeunit \"AAR Log\";\n}",
                "    trigger OnPreXmlPort() begin end;\n    trigger OnPostXmlPort() begin end;\n    var AarLog: Codeunit \"AAR Log\";\n}",
            ),
            doc_miss.to_string(),
        ),
    ] {
        assert_ne!(src, base, "{shape}: the template did not change");
        al_builtins::assumptions::take_hits();
        let got = doc(&src, 50710, &xml).unwrap_or_else(|e| panic!("{shape}: {e:?}"));
        assert!(
            got.starts_with("No | The field Rel Code of table AAR Row contains a value (MISS) that cannot be found"),
            "{shape}: {got}"
        );
        let hits = al_builtins::assumptions::take_hits();
        assert!(!hits.contains(&NOT_VALIDATED.unwrap()), "{shape}: {hits:?}");
    }
}

/// Unmeasured fields and nodes with the trigger keep main's refusal: an Integer or Code[10] field on a field element, a
/// Code[10] field on a temporary table element, a Code[30] field, a relation that names the related field.
#[test]
fn unmeasured_fields_with_the_trigger_keep_mains_refusal() {
    for (src, p) in [
        (int_port(&temp(false, "Rel No", "")), temp(false, "Rel No", "")),
        (int_port(&db(false, "Rel No", "")), db(false, "Rel No", "")),
        (port(&db(false, "Rel Code10", "")), db(false, "Rel Code10", "")),
        (port(&temp(true, "Rel Code10", "")), temp(true, "Rel Code10", "")),
        (port(&db(true, "Rel Code30", "")), db(true, "Rel Code30", "")),
        (port(&db(true, "Rel Code Named", "")), db(true, "Rel Code Named", "")),
    ] {
        let value = if p.field == "Rel No" { "1" } else { "P1" };
        match cases(&src, p.id, p.attr, &[(value, 0)]).remove(0) {
            Err(Signal::Unsupported(why)) => {
                assert!(
                    why.contains("which runs first is not pinned"),
                    "{} attr {}: {why}",
                    p.field,
                    p.attr
                )
            }
            other => panic!("{} attr {} temp {}: {other:?}", p.field, p.attr, p.temp),
        }
    }
}

/// An Integer attribute with FieldValidate = No is not measured: main's answer (no validation, then the trigger) stays,
/// without the assumption. A value that does not evaluate fails before the trigger, as on main.
#[test]
fn an_unmeasured_integer_attribute_keeps_mains_answer() {
    let p = db(true, "Rel No", "FieldValidate = No;");
    let got = ok_cases(&int_port(&p), &p, &[("99", 0)], None);
    assert_eq!(got[0], "Yes |  |  | after:99;before insert:99; | 1::99:,");
    let p = db(true, "Rel No", "");
    let got = ok_cases(&int_port(&p), &p, &[("abc", 0)], None);
    assert!(got[0].starts_with("No | ") && got[0].ends_with(" |  | "), "{got:?}");
}

/// Two rows into a temporary table element without triggers are not measured: the second row refuses.
#[test]
fn a_second_row_without_triggers_on_a_temporary_table_element_refuses() {
    let p = temp(true, "Rel Code", "");
    match doc(
        &plain_port(&p),
        p.id,
        r#"<Root><Row Id="1" RelCode="P1" /><Row Id="2" RelCode="P1" /></Root>"#,
    ) {
        Err(Signal::Unsupported(why)) => assert!(why.contains("second row of temporary table element"), "{why}"),
        other => panic!("{other:?}"),
    }
}

/// Two rows into a field element with FieldValidate = No: the measured order gives main's answer for any value, also blank.
#[test]
fn two_rows_into_a_field_element_with_field_validate_no_keep_mains_answer() {
    let p = db(false, "Rel Code", "FieldValidate = No;");
    let got = doc(
        &port(&p),
        p.id,
        "<Root><Row Id=\"1\"><RelCode>P1</RelCode></Row><Row Id=\"2\"><RelCode></RelCode></Row></Root>",
    );
    assert_eq!(
        got.unwrap(),
        "Yes |  | after:P1;before insert:P1;after:;before insert:; | 1:P1:,2::,"
    );
}
