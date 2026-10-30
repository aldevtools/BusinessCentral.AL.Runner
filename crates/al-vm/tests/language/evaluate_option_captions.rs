//! `Evaluate` into an Option field with captions, and into the Option key field of a RecordId
//! (types/lt-option-evaluate-captions/OptionEvaluateCaptions.al, codeunit 90822).

fn run(src: &str) -> String {
    al_testkit::text(al_testkit::run_with(src, "C", "F", vec![]).map(|(v, _)| v))
}

const TABLES: &str = r#"
table 50100 R
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Swap; Option) { OptionMembers = A,B,C; OptionCaption = 'B,C,A'; }
        field(3; ML; Option) { OptionMembers = Red,Green,Blue; OptionCaptionML = ENU = 'Red one,Green one,Blue one', DEU = 'Rot,Gruen,Blau'; }
        field(4; Sp; Option) { OptionMembers = " ",Open,"Closed Now"; OptionCaption = ' ,Open cap,Closed Now cap'; }
        field(5; Lead; Option) { OptionMembers = P,Q; OptionCaption = ' P cap,Q cap '; }
    }
    keys { key(PK; Id) { Clustered = true; } }
}
table 50101 K
{
    fields
    {
        field(1; "Doc Type"; Option) { OptionMembers = " ",Invoice,"Credit Memo"; OptionCaption = ' ,Inv cap,Credit Memo cap'; }
        field(2; "No."; Code[20]) { }
    }
    keys { key(PK; "Doc Type", "No.") { Clustered = true; } }
}
table 50102 P
{
    fields
    {
        field(1; "Doc Type"; Option) { OptionMembers = " ",Invoice,"Credit Memo"; }
        field(2; "No."; Code[20]) { }
    }
    keys { key(PK; "Doc Type", "No.") { Clustered = true; } }
}
"#;

// :162 CaptionIsNameOfAnotherMember, :192 BlanksAndPrefixes, :270 FormatNumber.
#[test]
fn captions_win_blanks_are_trimmed_and_format_numbers_two_and_nine_read_as_zero() {
    let src = format!(
        r#"{TABLES}
codeunit 50100 C
{{
    local procedure N(O: Integer): Text
    begin
        exit(Format(O));
    end;

    procedure F(): Text
    var Row: Record R; M: Text; Tab: Text; Ch: Char;
    begin
        Ch := 9; Tab[1] := Ch;
        if Evaluate(Row.Swap, 'A') then M += N(Row.Swap) + ';';
        if Evaluate(Row.Lead, 'P cap') then M += N(Row.Lead) + ';';
        if Evaluate(Row.Lead, 'Q cap') then M += N(Row.Lead) + ';';
        if Evaluate(Row.Sp, Tab + 'Open') then M += N(Row.Sp) + ';';
        if Evaluate(Row.Sp, Tab) then M += N(Row.Sp) + ';';
        if not Evaluate(Row.Sp, 'Closed  Now') then M += 'no;';
        if Evaluate(Row.Sp, 'Open cap', 9) then M += N(Row.Sp) + ';';
        if Evaluate(Row.Sp, '2', 2) then M += N(Row.Sp) + ';';
        exit(M);
    end;
}}"#
    );
    assert_eq!(run(&src), "2;0;1;1;0;no;1;2;");
}

// :236 CaptionML_ByLanguage.
#[test]
fn caption_ml_takes_the_captions_of_the_session_language() {
    let src = format!(
        r#"{TABLES}
codeunit 50100 C
{{
    local procedure N(O: Integer): Text
    begin
        exit(Format(O));
    end;

    procedure F(): Text
    var Row: Record R; M: Text;
    begin
        if Evaluate(Row.ML, 'Green one') then M += N(Row.ML) + ';';
        GlobalLanguage(1031);
        if Evaluate(Row.ML, 'gruen') then M += N(Row.ML) + ';';
        Row.ML := Row.ML::Red;
        if not Evaluate(Row.ML, 'Blue one') then M += 'no;';
        GlobalLanguage(1036);
        if Evaluate(Row.ML, 'Blue one') then M += N(Row.ML) + ';';
        GlobalLanguage(1033);
        exit(M);
    end;
}}"#
    );
    assert_eq!(run(&src), "1;1;no;2;");
}

// :330 ResultUnused_Errors.
#[test]
fn a_rejected_text_with_the_result_unused_raises_the_option_error() {
    let src = format!(
        r#"{TABLES}
codeunit 50100 C
{{
    procedure F(): Text
    var Row: Record R; V: Option Alpha,Beta,Gamma; M: Text; Tab: Text; Ch: Char;
    begin
        Ch := 9; Tab[1] := Ch;
        if Evaluate(V, 'gamma', 2) then M += Format(V) + ';';
        asserterror Evaluate(Row.Sp, 'Nope', 9);
        M += GetLastErrorText() + '|' + GetLastErrorCode() + ';';
        asserterror Evaluate(V, Tab);
        M += GetLastErrorText() + ';';
        exit(M);
    end;
}}"#
    );
    assert_eq!(
        run(&src),
        "Gamma;'Nope' is not an option. The existing options are:  ,Open cap,Closed Now cap|InvalidOptionString;\
         '\\t' is not an option. The existing options are: Alpha,Beta,Gamma;"
    );
}

// :392 RecordId_OptionKeyWithCaption, :414 RecordId_OptionKeyShapes.
#[test]
fn a_record_id_option_key_reads_captions_case_blanks_quotes_and_integers() {
    let src = format!(
        r#"{TABLES}
codeunit 50100 C
{{
    local procedure E(T: Text): Text
    var Id: RecordId;
    begin
        if not Evaluate(Id, T) then exit('false:' + GetLastErrorText());
        exit(Format(Id));
    end;

    procedure F(): Text
    begin
        exit(E('K: Credit Memo cap,D-1') + ';' + E('K: Inv cap,D-1') + ';' + E('K: 2,D-1') + ';' + E('K: Quote,D-1') + ';' +
            E('P: invoice,D-1') + ';' + E('P:  Invoice ,D-1') + ';' + E('P: "",D-1') + ';' + E('P: "Credit Memo",D-1') + ';' +
            E('P: 2147483647,D-1') + ';' + E('P: 2147483648,D-1'));
    end;
}}"#
    );
    assert_eq!(
        run(&src),
        "K: Credit Memo cap,D-1;K: Inv cap,D-1;K: Credit Memo cap,D-1;\
         false:'Quote' is not an option. The existing options are:  ,Inv cap,Credit Memo cap;\
         P: Invoice,D-1;P: Invoice,D-1;P: \" \",D-1;P: Credit Memo,D-1;P: 2147483647,D-1;\
         false:'2147483648' is not an option. The existing options are:  ,Invoice,Credit Memo"
    );
}
