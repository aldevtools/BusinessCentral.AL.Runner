//! `Format(RecordId, 0, 10)` is the bookmark GetUrl gives, unescaped, and `Evaluate(RecordId, Text, 10)` reads it back; a text
//! without `;` is no bookmark (types/lt-y49-recordid-format10/Y49RecordIdFormat10.al:117-335, corpus master).

const SRC: &str = r#"
table 92943 "LT Y49 Bm Code Row" { fields { field(1; "No."; Code[20]) { } field(2; Name; Text[30]) { } } keys { key(PK; "No.") { } } }
table 92944 "LT Y49 Bm Line Row" { fields { field(1; "Doc Type"; Option) { OptionMembers = Quote,Order,Invoice; } field(2; "No."; Code[20]) { } field(3; "Line No."; Integer) { } }
    keys { key(PK; "Doc Type", "No.", "Line No.") { } } }
codeunit 50773 C {
    procedure Shapes(): Text
    var A: Record "LT Y49 Bm Code Row"; B: Record "LT Y49 Bm Line Row"; Blank: Record "LT Y49 Bm Code Row";
    begin
        A."No." := 'A-1'; A.Insert(); A."No." := '1'; A.Insert();
        B."Doc Type" := B."Doc Type"::Order; B."No." := 'X'; B."Line No." := 10000; B.Insert();
        A.Get('A-1');
        exit(Format(A.RecordId, 0, 10) + '|' + Format(B.RecordId, 0, 10) + '|' + Format(Blank.RecordId, 0, 10));
    end;
    procedure RoundTrip(): Text
    var A: Record "LT Y49 Bm Code Row"; B: Record "LT Y49 Bm Line Row"; RecId: RecordId; RecRef: RecordRef; Res: Text;
    begin
        A."No." := 'A-1'; A.Name := 'Name A-1'; A.Insert();
        B."Doc Type" := B."Doc Type"::Order; B."No." := 'X'; B."Line No." := 10000; B.Insert();
        Res := Format(Evaluate(RecId, Format(A.RecordId, 0, 10), 10)) + ' ' + Format(RecId) + ' ' + Format(RecId = A.RecordId);
        RecRef.Get(RecId); Res += ' ' + Format(RecRef.Field(2).Value);
        Clear(RecId);
        Res += '|' + Format(Evaluate(RecId, Format(B.RecordId, 0, 10), 10)) + ' ' + Format(RecId);
        Clear(RecId);
        Res += '|' + Format(Evaluate(RecId, '17;D2sBAAJ7', 10)) + ' ' + Format(RecId);
        Clear(RecId);
        Res += '|' + Format(Evaluate(RecId, 'garbage', 10)) + ' ' + Format(RecId);
        Res += '|' + Format(Evaluate(RecId, '', 10)) + ' ' + Format(RecId);
        exit(Res);
    end;
    procedure GarbageStatement()
    var RecId: RecordId;
    begin
        Evaluate(RecId, 'garbage', 10);
    end;
    procedure OtherTargets(): Text
    var T: Text; I: Integer; G: Guid;
    begin
        exit(Format(Evaluate(T, 'x', 10)) + T + Format(Evaluate(I, '12', 10)) + Format(I)
            + Format(Evaluate(G, '{3F2504E0-4F89-11D3-9A0C-0305E82C3301}', 10)) + Format(G));
    end;
}
"#;

fn run(routine: &str) -> String {
    al_testkit::text(al_testkit::run_in(SRC, "C", routine))
}

#[test]
fn format_10_of_a_record_id_is_its_bookmark() {
    assert_eq!(
        run("Shapes"),
        "17;D2sBAAJ7/0EALQAx|25;EGsBAACLAQAAAAJ7/1gAAAAAhxAn|11;D2sBAAJ7"
    );
}

#[test]
fn evaluate_10_reads_a_bookmark_back_and_a_text_without_semicolon_is_none() {
    assert_eq!(
        run("RoundTrip"),
        "Yes LT Y49 Bm Code Row: A-1 Yes Name A-1|Yes LT Y49 Bm Line Row: Order,X,10000|Yes LT Y49 Bm Code Row: \"\"|No |Yes "
    );
    assert_eq!(
        al_testkit::error_message(al_testkit::run_in(SRC, "C", "GarbageStatement")),
        "The value \"garbage\" can't be evaluated into type RecordID."
    );
}

#[test]
fn evaluate_10_into_text_integer_and_guid_reads_as_without_a_number() {
    assert_eq!(
        run("OtherTargets"),
        "YesxYes12Yes{3F2504E0-4F89-11D3-9A0C-0305E82C3301}"
    );
}
