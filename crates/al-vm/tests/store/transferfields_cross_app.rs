//! Gap C3-16: TransferFields between fields of one number and different types, from a table of this app into a table of
//! another app and into one of this app (record/lt-y38-transfer-xapp/Y38TransferXApp.al, corpus writer Y38, measured on BC 27.5,
//! 28.5, 29.0).

use al_testkit::AppSource;

fn app(id: &str, deps: &[&str], src: &str) -> AppSource {
    AppSource {
        id: id.into(),
        name: format!("App {id}"),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files: vec![(format!("{id}.al"), src.into())],
    }
}

const FIELDS: &str = r#"
        field(1; "No."; Code[20]) { }
        field(2; CustNo; Code[20]) { }
        field(5; OtherName; Text[100]) { }
        field(33; Factor; Decimal) { }
        field(40; Qty; Decimal) { }
        field(41; Cod; Text[5]) { }
        field(42; Opt; Integer) { }
        field(43; Txt; Code[30]) { }
        field(44; Dt; DateTime) { }
        field(45; Amt; Boolean) { }"#;

fn other_app() -> String {
    format!(r#"table 61105 "Xa Target" {{ fields {{ {FIELDS} }} keys {{ key(PK; "No.") {{ }} }} }}"#)
}

fn own_app() -> String {
    format!(
        r#"
table 92570 "Tf Source"
{{
    fields
    {{
        field(1; "No."; Code[20]) {{ }}
        field(2; CustNo; Code[20]) {{ }}
        field(5; Name; Text[100]) {{ }}
        field(33; Flag; Boolean) {{ }}
        field(40; Qty; Integer) {{ }}
        field(41; Cod; Code[10]) {{ }}
        field(42; Opt; Option) {{ OptionMembers = A,B,C; }}
        field(43; Txt; Text[30]) {{ }}
        field(44; Dt; Date) {{ }}
        field(45; Amt; Decimal) {{ }}
    }}
    keys {{ key(PK; "No.") {{ }} }}
}}
table 92571 "Same Target" {{ fields {{ {FIELDS} }} keys {{ key(PK; "No.") {{ }} }} }}
table 92574 "Pair 41" {{ fields {{ field(1; "No."; Code[20]) {{ }} field(2; CustNo; Code[20]) {{ }} field(41; Cod; Text[5]) {{ }} }} keys {{ key(PK; "No.") {{ }} }} }}
table 92575 "Pair 42" {{ fields {{ field(1; "No."; Code[20]) {{ }} field(2; CustNo; Code[20]) {{ }} field(42; Opt; Integer) {{ }} }} keys {{ key(PK; "No.") {{ }} }} }}
codeunit 92579 C
{{
    local procedure Source(var Src: Record "Tf Source"; Flag: Boolean; CodValue: Code[10])
    begin
        Src."No." := 'S1'; Src.CustNo := 'C1'; Src.Name := 'Src name'; Src.Flag := Flag; Src.Qty := 7; Src.Cod := CodValue;
        Src.Opt := Src.Opt::C; Src.Txt := 'abc'; Src.Dt := 20260115D; Src.Amt := 3.25;
    end;

    [TryFunction]
    local procedure TryXa(var Dst: Record "Xa Target"; var Src: Record "Tf Source"; Mode: Integer)
    begin
        case Mode of
            1: Dst.TransferFields(Src, false);
            2: Dst.TransferFields(Src);
            3: Dst.TransferFields(Src, false, true);
        end;
    end;

    [TryFunction]
    local procedure TrySame(var Dst: Record "Same Target"; var Src: Record "Tf Source"; Mode: Integer)
    begin
        case Mode of
            1: Dst.TransferFields(Src, false);
            3: Dst.TransferFields(Src, false, true);
        end;
    end;

    [TryFunction]
    local procedure TryP41(var Dst: Record "Pair 41"; var Src: Record "Tf Source"; Mode: Integer)
    begin
        case Mode of
            1: Dst.TransferFields(Src, false);
            3: Dst.TransferFields(Src, false, true);
        end;
    end;

    [TryFunction]
    local procedure TryP42(var Dst: Record "Pair 42"; var Src: Record "Tf Source")
    begin
        Dst.TransferFields(Src, false);
    end;

    local procedure Outcome(Ok: Boolean): Text
    begin
        if Ok then exit('ok ');
        exit('error:' + GetLastErrorText() + '|' + GetLastErrorCode() + ' ');
    end;

    procedure Xa(Mode: Integer; Flag: Boolean; CodValue: Code[10]): Text
    var Src: Record "Tf Source"; Dst: Record "Xa Target"; R: Text;
    begin
        Source(Src, Flag, CodValue);
        if Mode <> 2 then begin
            Dst."No." := 'T1'; Dst.Factor := 1.5; Dst.Qty := 2.5; Dst.CustNo := 'OLD'; Dst.Cod := 'KEEP'; Dst.Insert();
        end;
        R := Outcome(TryXa(Dst, Src, Mode));
        exit(R + Dst."No." + ';' + Dst.CustNo + ';' + Dst.OtherName + ';' + Format(Dst.Factor, 0, 9) + ';' + Format(Dst.Qty, 0, 9) + ';' +
            Dst.Cod + ';' + Format(Dst.Opt) + ';' + Dst.Txt + ';' + Format(Dst.Dt) + ';' + Format(Dst.Amt));
    end;

    procedure Same(Mode: Integer): Text
    var Src: Record "Tf Source"; Dst: Record "Same Target"; R: Text;
    begin
        Source(Src, true, 'ABC');
        Dst."No." := 'T1'; Dst.Factor := 1.5; Dst.Qty := 2.5; Dst.CustNo := 'OLD'; Dst.Cod := 'KEEP'; Dst.Insert();
        R := Outcome(TrySame(Dst, Src, Mode));
        exit(R + Dst."No." + ';' + Dst.CustNo + ';' + Dst.OtherName + ';' + Format(Dst.Factor, 0, 9) + ';' + Format(Dst.Qty, 0, 9) + ';' +
            Dst.Cod + ';' + Format(Dst.Opt) + ';' + Dst.Txt + ';' + Format(Dst.Dt) + ';' + Format(Dst.Amt));
    end;

    procedure P41(Mode: Integer; CodValue: Code[10]): Text
    var Src: Record "Tf Source"; Dst: Record "Pair 41"; R: Text;
    begin
        Source(Src, true, CodValue);
        Dst."No." := 'T1'; Dst.CustNo := 'OLD'; Dst.Cod := 'KEEP'; Dst.Insert();
        R := Outcome(TryP41(Dst, Src, Mode));
        exit(R + Dst."No." + ';' + Dst.CustNo + ';' + Dst.Cod);
    end;

    procedure P42(): Text
    var Src: Record "Tf Source"; Dst: Record "Pair 42"; R: Text;
    begin
        Source(Src, true, 'ABC');
        Dst."No." := 'T1'; Dst.CustNo := 'OLD'; Dst.Insert();
        R := Outcome(TryP42(Dst, Src));
        exit(R + Dst."No." + ';' + Dst.CustNo + ';' + Format(Dst.Opt));
    end;
}}
"#
    )
}

fn built() -> al_testkit::Built {
    al_testkit::Built::new(al_testkit::compile_apps(vec![
        app("a", &[], &other_app()),
        app("m", &["a"], &own_app()),
    ]))
}

fn call(b: &mut al_testkit::Built, routine: &str, args: Vec<al_builtins::value::Value>) -> String {
    let (oi, ri) = b.routine_in("C", routine);
    let (v, _) = b.call(oi, ri, args).unwrap_or_else(|e| panic!("{routine}: {e:?}"));
    al_testkit::text(Ok(v))
}

fn args(mode: i32, flag: bool, cod: &str) -> Vec<al_builtins::value::Value> {
    use al_builtins::value::Value;
    vec![Value::Integer(mode), Value::Boolean(flag), Value::Code(cod.into())]
}

// Y38TransferXApp.al:449-512, X1-X4 and XA_TF_LongCode: across apps every field of another kind is skipped, whatever
// SkipFieldsWithoutCheck says (the target keeps its value); Code moves into Text and Text into Code; a Code value too long for the
// Text field is skipped too.
#[test]
fn across_apps_fields_of_another_kind_are_skipped() {
    let mut b = built();
    let moved = "ok T1;C1;Src name;1.5;2.5;ABC;0;ABC;;No";
    assert_eq!(call(&mut b, "Xa", args(1, true, "ABC")), moved);
    assert_eq!(call(&mut b, "Xa", args(1, false, "ABC")), moved);
    assert_eq!(
        call(&mut b, "Xa", args(2, true, "ABC")),
        "ok S1;C1;Src name;0;0;ABC;0;ABC;;No"
    );
    assert_eq!(call(&mut b, "Xa", args(3, true, "ABC")), moved);
    let long = "ok T1;C1;Src name;1.5;2.5;KEEP;0;ABC;;No";
    assert_eq!(call(&mut b, "Xa", args(1, true, "ABCDEFG")), long);
    assert_eq!(call(&mut b, "Xa", args(3, true, "ABCDEFG")), long);
}

// Y38TransferXApp.al:517-534 (S1, S2) and :539-595 (p41, p41_long, p42): within one app a field of another kind fails with
// DB:TypeMisMatch and the fields before it stay copied; a Code too long for the Text field fails with the size error.
#[test]
fn within_one_app_fields_of_another_kind_fail() {
    use al_builtins::value::Value;
    let mut b = built();
    assert_eq!(
        call(&mut b, "Same", vec![Value::Integer(1)]),
        "error:The following fields must have the same type:\\\\Field: Factor <-- Flag\\Table: Same Target <-- Tf Source\\Type: Decimal <-- Boolean|DB:TypeMisMatch T1;C1;Src name;1.5;2.5;KEEP;0;;;No"
    );
    assert_eq!(
        call(&mut b, "Same", vec![Value::Integer(3)]),
        "ok T1;C1;Src name;1.5;2.5;ABC;0;ABC;;No"
    );
    assert_eq!(
        call(&mut b, "P41", vec![Value::Integer(1), Value::Code("ABC".into())]),
        "ok T1;C1;ABC"
    );
    assert_eq!(
        call(&mut b, "P41", vec![Value::Integer(1), Value::Code("ABCDEFG".into())]),
        "error:The contents of the Cod field cannot be copied to the Cod field because the size of the Cod field is too small.\\\\Type: Text5 <-- Code10\\Field: Cod <-- Cod\\Table: Pair 41 <-- Tf Source|DB:TypeMisMatch T1;C1;KEEP"
    );
    assert_eq!(
        call(&mut b, "P41", vec![Value::Integer(3), Value::Code("ABCDEFG".into())]),
        "ok T1;C1;KEEP"
    );
    assert_eq!(
        call(&mut b, "P42", vec![]),
        "error:The following fields must have the same type:\\\\Field: Opt <-- Opt\\Table: Pair 42 <-- Tf Source\\Type: Integer <-- Option|DB:TypeMisMatch T1;C1;0"
    );
}
