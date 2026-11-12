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

/// The six cases of XpAfterAssignTableRelation.al:552-560: existing, missing, blank (mode 0), missing then P2, existing then
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

/// XpAfterAssignTableRelation.al:578 (attribute), :591 (element), :604 (FieldValidate = No), :617 (ValidateTableRelation =
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

/// The reporter's shape (al-runner issue 7863): a temporary table element, a field with a TableRelation and no OnValidate.
#[test]
fn the_reporters_shape_imports_the_missing_value_too() {
    let got = ok_six(&temp(true, "Rel Code Plain", ""));
    assert_eq!(got[1], "Yes |  |  | after:MISS;before insert:MISS; | ");
}

/// XpAfterAssignTableRelation.al:630 (attribute) and :642 (element): the trigger, then the TableRelation check and OnValidate
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

/// XpAfterAssignTableRelation.al:655 (FieldValidate = No) and :668 (ValidateTableRelation = false) on a stored table element.
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

/// Shapes the probe did not measure keep main's refusal: a field element into the field without the check, a
/// relation with a where clause, FieldValidate = Yes, a nested table element is covered by the predicate's position fact.
#[test]
fn unmeasured_validated_shapes_keep_mains_refusal() {
    for p in [
        db(false, "Rel Code NV", ""),
        temp(false, "Rel Code NV", ""),
        db(true, "Rel Code Where", ""),
        db(true, "Rel Code", "FieldValidate = Yes;"),
        db(true, "Rel Code Plain", ""),
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

/// A field element with FieldValidate = No on a stored table element is not measured: main's answer (the trigger, no
/// validation) stays, without the assumption.
#[test]
fn an_unmeasured_unvalidated_element_keeps_mains_answer() {
    al_builtins::assumptions::take_hits();
    let got: Vec<String> = six(&db(false, "Rel Code", "FieldValidate = No;"))
        .into_iter()
        .map(|r| r.unwrap())
        .collect();
    assert_eq!(got[1], "Yes |  |  | after:MISS;before insert:MISS; | 1:MISS:,");
    assert_eq!(got[2], "Yes |  |  | after:;before insert:; | 1::,");
    assert!(
        !al_builtins::assumptions::take_hits()
            .contains(&al_builtins::assumptions::Assumption::XmlportAfterAssignBeforeValidate)
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

#[test]
fn the_reporters_repro_sees_the_imported_value() {
    assert_eq!(
        text(al_testkit::run_in(
            REPRO,
            "Repro After Assign Test",
            "AfterAssignFieldSeesImportedValue"
        )),
        "P1"
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
