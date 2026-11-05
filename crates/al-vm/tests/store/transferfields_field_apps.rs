//! TransferFields between two tables of one app whose fields of one number come from different apps (al-runner issue 6982:
//! Base Application's `Contact.TransferFields(Cust)` with a Boolean of one app and a Code[20] of another under one number).
//! Real BC's values: corpus branch al-runtime-rs-staging-r6792 0394701c, record/transferfields-cross-app/
//! TransferFieldsFieldApps.al:131-206 with tests/al-language-xapp-a|b|c|d/Tfx*.al (BC 27.5, 28.5 and 29.0).

use al_builtins::value::Value;
use al_testkit::AppSource;

fn app(id: &str, deps: &[&str], src: &str) -> AppSource {
    AppSource {
        id: id.into(),
        name: format!("App {id}"),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files: vec![(format!("{id}.al"), src.into())],
    }
}

const A: &str = r#"
table 95830 "Src" { fields { field(1; "No."; Code[20]) { } } keys { key(PK; "No.") { } } }
table 95831 "Tgt" { fields { field(1; "No."; Code[20]) { } field(95835; "Owner Code"; Code[20]) { } } keys { key(PK; "No.") { } } }
table 95832 "Src 2" { fields { field(1; "No."; Code[20]) { } } keys { key(PK; "No.") { } } }
table 95833 "Tgt 2" { fields { field(1; "No."; Code[20]) { } } keys { key(PK; "No.") { } } }
"#;

const B: &str = r#"
tableextension 95834 "B Src" extends "Src"
{
    fields
    {
        field(95830; "XB Flag D"; Boolean) { }
        field(95831; "XB Flag C"; Boolean) { }
        field(95832; "XB Qty D"; Integer) { }
        field(95833; "XB Code D"; Code[20]) { }
        field(95834; "XB Text D"; Text[30]) { }
        field(95835; "XB Flag A"; Boolean) { }
    }
}
tableextension 95835 "B Src 2" extends "Src 2" { fields { field(95830; "XB Same Flag"; Boolean) { } } }
tableextension 95836 "B Tgt 2" extends "Tgt 2" { fields { field(95830; "XB Same Code"; Code[20]) { } } }
"#;

const C: &str = r#"tableextension 95837 "C Tgt" extends "Tgt" { fields { field(95831; "XC Code"; Code[20]) { } } }"#;

const D: &str = r#"
tableextension 95838 "D Tgt" extends "Tgt"
{
    fields
    {
        field(95830; "XD Code"; Code[20]) { }
        field(95832; "XD Qty"; Integer) { }
        field(95833; "XD Flag"; Boolean) { }
        field(95834; "XD Code 2"; Code[20]) { }
    }
}
"#;

const M: &str = r#"
codeunit 95811 T
{
    local procedure SourceRef(var SrcRef: RecordRef; LongText: Boolean)
    begin
        SrcRef.Open(95830);
        SrcRef.Init();
        SrcRef.Field(1).Value := 'S1';
        SrcRef.Field(95830).Value := true;
        SrcRef.Field(95831).Value := true;
        SrcRef.Field(95832).Value := 7;
        SrcRef.Field(95833).Value := 'SRCCODE';
        if LongText then
            SrcRef.Field(95834).Value := 'source text longer than twenty'
        else
            SrcRef.Field(95834).Value := 'src text';
        SrcRef.Field(95835).Value := true;
    end;

    local procedure TargetRef(var TgtRef: RecordRef)
    begin
        TgtRef.Open(95831);
        TgtRef.Init();
        TgtRef.Field(1).Value := 'T1';
        TgtRef.Field(95830).Value := 'KEEPD';
        TgtRef.Field(95831).Value := 'KEEPC';
        TgtRef.Field(95832).Value := 5;
        TgtRef.Field(95833).Value := false;
        TgtRef.Field(95834).Value := 'KEEPD2';
        TgtRef.Field(95835).Value := 'KEEPA';
    end;

    local procedure Dump(RecVariant: Variant): Text
    var RecRef: RecordRef; i: Integer; No: Integer; Result: Text;
    begin
        RecRef.GetTable(RecVariant);
        for i := 95829 to 95835 do begin
            No := i;
            if i = 95829 then No := 1;
            if RecRef.FieldExist(No) then
                Result += Format(No) + '=' + Format(RecRef.Field(No).Value(), 0, 9) + ';';
        end;
        exit(Result);
    end;

    [TryFunction]
    local procedure TryForward(var Tgt: Record "Tgt"; var Src: Record "Src"; Mode: Integer)
    begin
        case Mode of
            1: Tgt.TransferFields(Src);
            2: Tgt.TransferFields(Src, false);
            3: Tgt.TransferFields(Src, true, true);
        end;
    end;

    [TryFunction]
    local procedure TryBackward(var Src: Record "Src"; var Tgt: Record "Tgt"; Mode: Integer)
    begin
        case Mode of
            1: Src.TransferFields(Tgt);
            2: Src.TransferFields(Tgt, false);
            3: Src.TransferFields(Tgt, true, true);
        end;
    end;

    [TryFunction]
    local procedure TrySameApp(var Tgt: Record "Tgt 2"; var Src: Record "Src 2"; Mode: Integer)
    begin
        case Mode of
            1: Tgt.TransferFields(Src);
            2: Tgt.TransferFields(Src, false);
            3: Tgt.TransferFields(Src, true, true);
        end;
    end;

    procedure Forward(Mode: Integer; LongText: Boolean): Text
    var Src: Record "Src"; Tgt: Record "Tgt"; SrcRef: RecordRef; TgtRef: RecordRef;
    begin
        SourceRef(SrcRef, LongText); SrcRef.SetTable(Src);
        TargetRef(TgtRef); TgtRef.SetTable(Tgt);
        if TryForward(Tgt, Src, Mode) then exit('ok: ' + Dump(Tgt));
        exit('error: ' + GetLastErrorText() + ' | ' + GetLastErrorCode() + ' | ' + Dump(Tgt));
    end;

    procedure Backward(Mode: Integer): Text
    var Src: Record "Src"; Tgt: Record "Tgt"; SrcRef: RecordRef; TgtRef: RecordRef;
    begin
        SourceRef(SrcRef, false); SrcRef.SetTable(Src);
        TargetRef(TgtRef); TgtRef.SetTable(Tgt);
        if TryBackward(Src, Tgt, Mode) then exit('ok: ' + Dump(Src));
        exit('error: ' + GetLastErrorText() + ' | ' + GetLastErrorCode() + ' | ' + Dump(Src));
    end;

    procedure SameApp(Mode: Integer): Text
    var Src: Record "Src 2"; Tgt: Record "Tgt 2";
    begin
        Src."No." := 'S2'; Src."XB Same Flag" := true;
        Tgt."No." := 'T2'; Tgt."XB Same Code" := 'KEEPB';
        if TrySameApp(Tgt, Src, Mode) then exit('ok: ' + Dump(Tgt));
        exit('error: ' + GetLastErrorText() + ' | ' + GetLastErrorCode() + ' | ' + Dump(Tgt));
    end;
}
"#;

fn built() -> al_testkit::Built {
    al_testkit::Built::new(al_testkit::compile_apps(vec![
        app("a", &[], A),
        app("b", &["a"], B),
        app("d", &["a"], D),
        app("c", &["a", "b", "d"], C),
        app("m", &["a", "b", "c"], M),
    ]))
}

fn call(b: &mut al_testkit::Built, routine: &str, args: Vec<Value>) -> String {
    let (oi, ri) = b.routine_in("T", routine);
    let (v, _) = b.call(oi, ri, args).unwrap_or_else(|e| panic!("{routine}: {e:?}"));
    al_testkit::text(Ok(v))
}

/// TransferFieldsFieldApps.al:131-149 (TransferFields_FieldsOfOtherApps_Forward): a field of another kind from another app is
/// skipped in every mode (the target keeps its value, no error), whether the two apps depend on each other (C on B) or not (B
/// and D), and against the target table's own field (A); an Integer moves into an Integer and a Text into a Code (upper case)
/// across apps; a Text too long for the Code field is skipped.
#[test]
fn fields_of_other_apps_forward() {
    let mut b = built();
    let f = |b: &mut al_testkit::Built, mode: i32, long: bool| {
        call(b, "Forward", vec![Value::Integer(mode), Value::Boolean(long)])
    };
    assert_eq!(
        f(&mut b, 0, false),
        "ok: 1=T1;95830=KEEPD;95831=KEEPC;95832=5;95833=false;95834=KEEPD2;95835=KEEPA;"
    );
    assert_eq!(
        f(&mut b, 1, false),
        "ok: 1=S1;95830=KEEPD;95831=KEEPC;95832=7;95833=false;95834=SRC TEXT;95835=KEEPA;"
    );
    assert_eq!(
        f(&mut b, 2, false),
        "ok: 1=T1;95830=KEEPD;95831=KEEPC;95832=7;95833=false;95834=SRC TEXT;95835=KEEPA;"
    );
    assert_eq!(
        f(&mut b, 3, false),
        "ok: 1=S1;95830=KEEPD;95831=KEEPC;95832=7;95833=false;95834=SRC TEXT;95835=KEEPA;"
    );
    assert_eq!(
        f(&mut b, 1, true),
        "ok: 1=S1;95830=KEEPD;95831=KEEPC;95832=7;95833=false;95834=KEEPD2;95835=KEEPA;"
    );
    assert_eq!(
        f(&mut b, 3, true),
        "ok: 1=S1;95830=KEEPD;95831=KEEPC;95832=7;95833=false;95834=KEEPD2;95835=KEEPA;"
    );
}

/// TransferFieldsFieldApps.al:151-163 (TransferFields_FieldsOfOtherApps_Backward): the same the other way round; a Code moves
/// into a Text across apps.
#[test]
fn fields_of_other_apps_backward() {
    let mut b = built();
    let want = |pk: &str| format!("ok: 1={pk};95830=true;95831=true;95832=5;95833=SRCCODE;95834=KEEPD2;95835=true;");
    assert_eq!(call(&mut b, "Backward", vec![Value::Integer(1)]), want("T1"));
    assert_eq!(call(&mut b, "Backward", vec![Value::Integer(2)]), want("S1"));
    assert_eq!(call(&mut b, "Backward", vec![Value::Integer(3)]), want("T1"));
}

/// TransferFieldsFieldApps.al:198-207 (TransferFields_TwoFieldsOfOneExtensionApp): two fields of one app (B) on two tables of
/// another app (A) fail with DB:TypeMisMatch, the primary key already copied; SkipFieldsWithoutCheck skips the field.
#[test]
fn two_fields_of_one_extension_app() {
    let mut b = built();
    let err = |pk: &str| {
        format!(
            "error: The following fields must have the same type:\\\\Field: XB Same Code <-- XB Same Flag\\Table: Tgt 2 <-- Src 2\\Type: Code <-- Boolean | DB:TypeMisMatch | 1={pk};95830=KEEPB;"
        )
    };
    assert_eq!(call(&mut b, "SameApp", vec![Value::Integer(1)]), err("S2"));
    assert_eq!(call(&mut b, "SameApp", vec![Value::Integer(2)]), err("T2"));
    assert_eq!(
        call(&mut b, "SameApp", vec![Value::Integer(3)]),
        "ok: 1=S2;95830=KEEPB;"
    );
}
