//! `Evaluate` into a TableFilter field of a field named on a table whose field has a CaptionClass resolved by a bound
//! OnResolveCaptionClass subscriber (al-runner issue 7164), and of the quoted blank `''` on a Code[10] field, alone and in a compound
//! condition (al-runner issue 7165). Pinned by corpus pull request 1157, on corpus master
//! format-evaluate/tablefilter-neutral/TableFilterEnumForms.al:539 TableFilter_Evaluate_CaptionClassFieldName_SelectsPricedRow,
//! :551 TableFilter_Evaluate_ResolvedCaptionClass_RejectsUnknownField, :567 TableFilter_Evaluate_CodeBesideCaptionClass_SelectsNamedRow,
//! :579 TableFilter_Evaluate_QuotedBlankCode10_SelectsBlankService and :587 TableFilter_Evaluate_CompoundQuotedBlankCode10_SelectsBlankService
//! (fixtures CaptionCodeFixtures.al). The stored text (`Static Price=>=40`, `Service Type=''`) is an assumption; only the
//! selection is pinned. The tests read the stored text back as the Base Application page "Table Filter" does: each
//! field by its "Field Caption" (see codeunit Apply), its filter handed to FieldRef.SetFilter as written.

const OBJECTS: &str = r#"
codeunit 42 "Caption Class" { [IntegrationEvent(false, false)] procedure OnResolveCaptionClass(CaptionArea: Text; CaptionExpr: Text; Language: Integer; var Caption: Text; var Resolved: Boolean) begin end; }
table 69071 "FMT Caption Filter Row"
{
    fields
    {
        field(1; Code; Code[20]) { }
        field(2; "Nominal Price"; Decimal) { Caption = 'Static Price'; CaptionClass = 'NFMTFILTER,PRICE'; }
    }
    keys { key(PK; Code) { Clustered = true; } }
}
codeunit 69072 "FMT Filter Caption Resolver"
{
    EventSubscriberInstance = Manual;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Caption Class", 'OnResolveCaptionClass', '', false, false)]
    local procedure Resolve(CaptionArea: Text; CaptionExpr: Text; Language: Integer; var Caption: Text; var Resolved: Boolean)
    begin
        if (CaptionArea = 'NFMTFILTER') and (CaptionExpr = 'PRICE') then begin
            Caption := 'Displayed Price';
            Resolved := true;
        end;
    end;
}
table 69073 "FMT Blank Service Row"
{
    fields
    {
        field(1; Code; Code[20]) { }
        field(2; "Service Type"; Code[10]) { }
        field(3; Batch; Code[10]) { }
    }
    keys { key(PK; Code) { Clustered = true; } }
}
table 95453 "LT R6951 Holder" { fields { field(1; Id; Integer) { } field(2; Flt; TableFilter) { } } keys { key(PK; Id) { Clustered = true; } } }
codeunit 95461 Apply
{
    // As page "Table Filter" ParseTableFilter: `<table>: <caption>=<filter>,...`, each field found by its "Field Caption" in the
    // virtual Field table, which al-testkit does not hold: there the Field Caption of a CaptionClass field is its Caption property
    // (crates/al-vm/src/platform/fields.rs, "Field Caption come from the declaration"), so `Static Price` names field 2 here; the other
    // fields have no CaptionClass and are found by FieldRef.Caption.
    procedure Selects(Value: Text; TableNo: Integer): Text
    var
        Ref: RecordRef;
        FRef: FieldRef;
        Rest: Text;
        Part: Text;
        Cap: Text;
        P: Integer;
        I: Integer;
        No: Integer;
    begin
        Rest := CopyStr(Value, StrPos(Value, ': ') + 2);
        Ref.Open(TableNo);
        while Rest <> '' do begin
            P := StrPos(Rest, ',');
            if P = 0 then begin Part := Rest; Rest := ''; end else begin Part := CopyStr(Rest, 1, P - 1); Rest := CopyStr(Rest, P + 1); end;
            Cap := CopyStr(Part, 1, StrPos(Part, '=') - 1);
            No := 0;
            if Cap = 'Static Price' then No := 2;
            for I := 1 to Ref.FieldCount() do
                if (No = 0) and (Ref.FieldIndex(I).Caption() = Cap) then No := Ref.FieldIndex(I).Number();
            if No = 0 then Error('no field %1', Cap);
            FRef := Ref.Field(No);
            FRef.SetFilter(CopyStr(Part, StrPos(Part, '=') + 1));
        end;
        Ref.FindFirst();
        FRef := Ref.Field(1);
        exit(Value + '|' + Format(Ref.Count()) + '|' + Format(FRef.Value()));
    end;
}
"#;

fn run(body: &str) -> String {
    let src = format!("{OBJECTS}\ncodeunit 95460 T\n{{\n{body}\n}}\n");
    al_testkit::text(al_testkit::run_in(&src, "T", "F"))
}

const SEED_PRICES: &str = "if R.FieldCaption(\"Nominal Price\") <> 'Displayed Price' then Error('caption');
        R.Code := 'PRICE42'; R.\"Nominal Price\" := 42.5; R.Insert(false);
        R.Code := 'PRICE17'; R.\"Nominal Price\" := 17.5; R.Insert(false);";

fn caption_case(condition: &str) -> String {
    run(&format!(
        "procedure F(): Text var R: Record \"FMT Caption Filter Row\"; H: Record \"LT R6951 Holder\"; Res: Codeunit \"FMT Filter Caption Resolver\"; A: Codeunit Apply;
        begin BindSubscription(Res); {SEED_PRICES}
        Evaluate(H.Flt, 'FMT Caption Filter Row: {}');
        exit(A.Selects(Format(H.Flt), Database::\"FMT Caption Filter Row\")); end;",
        condition.replace('\'', "''")
    ))
}

/// :539 TableFilter_Evaluate_CaptionClassFieldName_SelectsPricedRow: the field by its NAME selects the priced row.
#[test]
fn a_field_with_a_resolved_caption_class_is_found_by_its_name() {
    assert_eq!(
        caption_case("Nominal Price=>=40"),
        "FMT Caption Filter Row: Static Price=>=40|1|PRICE42"
    );
}

/// :567 TableFilter_Evaluate_CodeBesideCaptionClass_SelectsNamedRow.
#[test]
fn another_field_beside_a_caption_class_field_selects_its_row() {
    assert_eq!(
        caption_case("Code=PRICE42"),
        "FMT Caption Filter Row: Code=PRICE42|1|PRICE42"
    );
}

/// :551 TableFilter_Evaluate_ResolvedCaptionClass_RejectsUnknownField: the resolved caption is an unknown field.
#[test]
fn the_resolved_caption_is_an_unknown_field() {
    let got = run(
        "procedure F(): Text var R: Record \"FMT Caption Filter Row\"; H: Record \"LT R6951 Holder\"; Res: Codeunit \"FMT Filter Caption Resolver\";
        begin BindSubscription(Res); if R.FieldCaption(\"Nominal Price\") <> 'Displayed Price' then Error('caption');
        asserterror Evaluate(H.Flt, 'FMT Caption Filter Row: Displayed Price=>=40');
        exit(GetLastErrorText()); end;",
    );
    assert!(
        got.contains("The supplied field number 'Displayed Price' cannot be found"),
        "{got}"
    );
}

fn service_case(condition: &str) -> String {
    run(&format!(
        "procedure F(): Text var R: Record \"FMT Blank Service Row\"; H: Record \"LT R6951 Holder\"; A: Codeunit Apply;
        begin
        R.Code := 'BLANK42'; R.Batch := 'KEEP'; R.Insert(false);
        R.Code := 'FILLED42'; R.\"Service Type\" := 'REPAIR'; R.Insert(false);
        R.Code := 'OTHER42'; R.\"Service Type\" := ''; R.Batch := 'DROP'; R.Insert(false);
        Evaluate(H.Flt, 'FMT Blank Service Row: {}');
        exit(A.Selects(Format(H.Flt), Database::\"FMT Blank Service Row\")); end;",
        condition.replace('\'', "''")
    ))
}

/// :579 TableFilter_Evaluate_QuotedBlankCode10_SelectsBlankService: both blank-service rows, BLANK42 first.
#[test]
fn a_quoted_blank_on_a_code10_field_selects_the_blank_rows() {
    assert_eq!(
        service_case("Service Type=''"),
        "FMT Blank Service Row: Service Type=''|2|BLANK42"
    );
}

/// :587 TableFilter_Evaluate_CompoundQuotedBlankCode10_SelectsBlankService: only BLANK42.
#[test]
fn a_quoted_blank_in_a_compound_condition_selects_one_row() {
    assert_eq!(
        service_case("Batch=KEEP,Service Type=''"),
        "FMT Blank Service Row: Service Type='',Batch=KEEP|1|BLANK42"
    );
}
