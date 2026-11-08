//! RecordRef.LoadFields with a FlowField or a FlowFilter field, and AreFieldsLoaded of a FlowField on a record that fetched
//! nothing (Record and RecordRef). Every literal is what real BC answered in the corpus:
//! record/load-fields/Y87RecordRefLoadFields.al (codeunits 94655 and 94656, writer Y87).

use al_testkit::text;

const SRC: &str = r#"
table 50210 LF
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Name; Text[30]) { }
        field(3; Qty; Decimal) { }
        field(4; Cnt; Integer) { FieldClass = FlowField; CalcFormula = count(LL where("Row Id" = field(Id))); }
        field(6; Flt; Integer) { FieldClass = FlowFilter; }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

table 50211 LL
{
    fields
    {
        field(1; "Row Id"; Integer) { }
        field(2; "Line No."; Integer) { }
    }
    keys { key(PK; "Row Id", "Line No.") { Clustered = true; } }
}

codeunit 50210 C
{
    local procedure Seed()
    var
        Row: Record LF;
        Line: Record LL;
    begin
        Row.Id := 1;
        Row.Name := 'one';
        Row.Insert();
        Line."Row Id" := 1;
        Line."Line No." := 1;
        Line.Insert();
        Line."Line No." := 2;
        Line.Insert();
    end;

    local procedure Flags(var R: RecordRef): Text
    begin
        exit(Format(R.AreFieldsLoaded(1)) + Format(R.AreFieldsLoaded(2)) + Format(R.AreFieldsLoaded(4)) + Format(R.AreFieldsLoaded(6)) + Format(R.AreFieldsLoaded(4, 6)));
    end;

    [TryFunction]
    local procedure Statement(var R: RecordRef; Which: Integer)
    begin
        case Which of
            1:
                R.LoadFields(2, 4);
            2:
                R.LoadFields(6, 2);
        end;
    end;

    local procedure Cell(State: Integer): Text
    var
        R: RecordRef;
        Txt: Text;
    begin
        R.Open(50210);
        case State of
            1:
                ;
            2:
                R.FindFirst();
            3:
                R.Field(1).Value := 1;
        end;
        Txt := Format(R.LoadFields(4)) + Format(R.LoadFields(6, 2)) + ' ';
        if Statement(R, 1) then
            Txt += 'ok '
        else
            Txt += GetLastErrorCode() + ':' + GetLastErrorText() + ' ';
        if Statement(R, 2) then
            Txt += 'ok '
        else
            Txt += GetLastErrorCode() + ':' + GetLastErrorText() + ' ';
        exit(Txt + Flags(R) + ' ' + Format(R.Field(4).Value));
    end;

    procedure RefMatrix(): Text
    begin
        Seed();
        exit(Cell(1) + '|' + Cell(2) + '|' + Cell(3));
    end;

    procedure RefGetUnderSetLoadFields(): Text
    var
        Row: Record LF;
        R: RecordRef;
        Txt: Text;
    begin
        Seed();
        Row.Get(1);
        R.Open(50210);
        R.SetLoadFields(2);
        R.Get(Row.RecordId());
        Txt := Flags(R) + Format(R.AreFieldsLoaded(3)) + ' ';
        R.Close();
        R.Open(50210);
        R.SetLoadFields(4);
        R.Get(Row.RecordId());
        exit(Txt + Flags(R) + Format(R.AreFieldsLoaded(3)));
    end;

    procedure RecClearOfFetched(): Text
    var
        Row: Record LF;
    begin
        Seed();
        Row.Get(1);
        Clear(Row);
        exit(Format(Row.AreFieldsLoaded(Row.Id)) + Format(Row.AreFieldsLoaded(Row.Name)) + Format(Row.AreFieldsLoaded(Row.Cnt)) + Format(Row.AreFieldsLoaded(Row.Id, Row.Flt)));
    end;

    procedure RecUnfetched(): Text
    var
        Row: Record LF;
        Txt: Text;
    begin
        Txt := Format(Row.AreFieldsLoaded(Row.Cnt)) + Format(Row.AreFieldsLoaded(Row.Flt)) + Format(Row.AreFieldsLoaded(Row.Id, Row.Cnt));
        Row.Init();
        Txt += Format(Row.AreFieldsLoaded(Row.Cnt));
        Row.Id := 1;
        exit(Txt + Format(Row.AreFieldsLoaded(Row.Cnt)) + Format(Row.AreFieldsLoaded(Row.Cnt, Row.Flt)));
    end;
}
"#;

/// Codeunit 94655: never fetched (`LoadFields_*_NeverFetched_*`) no error, the function form No, loaded=No for every field;
/// after FindFirst (`*_AfterFindFirst_*`) the function form No with a FlowField or FlowFilter in the list, the statement raises
/// DB:Ok naming the first such field of the list, every field loaded; key assigned (`*_KeyAssigned_*`) no error, function No,
/// every field loaded. The FlowField is never calculated (`values` Cnt=0).
#[test]
fn recordref_load_fields_with_flow_fields() {
    let msg = |f: &str| {
        format!(
            "DB:Ok:The field '{f}' in the 'LF' table is not valid for auto loading and cannot be passed to calls to AddLoadFields, SetLoadFields, or LoadFields. "
        )
    };
    assert_eq!(
        text(al_testkit::run(SRC, "RefMatrix")),
        format!(
            "NoNo ok ok NoNoNoNoNo 0|NoNo {}{}YesYesYesYesYes 0|NoNo ok ok YesYesYesYesYes 0",
            msg("Cnt"),
            msg("Flt")
        )
    );
}

/// Codeunit 94656 Rec_Cleared, Rec_Init `NoNoNoNoNoNo` / `NoNoNoNo`; Rec_KeyAssigned `YesYesYesYesYesYes` / `YesYesYesYes`.
#[test]
fn are_fields_loaded_of_a_flow_field_on_an_unfetched_record() {
    assert_eq!(text(al_testkit::run(SRC, "RecUnfetched")), "NoNoNoNoYesYes");
}

/// RecordRef.Get of a RecordId fetches the whole row whatever SetLoadFields asked for: codeunit 94655
/// `LoadFields_*_AfterGetUnderSetLoadFieldsName_*` loaded=YesYesYesYesYesYes, codeunit 94656 Ref_SetLoadFieldsNameThenGet and
/// Ref_SetLoadFieldsCntThenGet `YesYesYesYesYesYes` (the Record form keeps Qty unloaded: Rec_SetLoadFieldsNameThenGet
/// `YesYesNoYesYesYes`); record/W4PlatformAndMeta.al:497-516 PartialRecords_RecordRefLoadFields `recref_loaded_txt` Yes.
#[test]
fn recordref_get_fetches_the_whole_row_under_set_load_fields() {
    assert_eq!(
        text(al_testkit::run(SRC, "RefGetUnderSetLoadFields")),
        "YesYesYesYesYesYes YesYesYesYesYesYes"
    );
}

/// Codeunit 94656 Rec_AfterClearOfFetched: after Get and Clear every field reads as not loaded (`NoNoNoNoNoNo`, `NoNoNoNo`).
#[test]
fn are_fields_loaded_after_clear_of_a_fetched_record() {
    assert_eq!(text(al_testkit::run(SRC, "RecClearOfFetched")), "NoNoNoNo");
}
