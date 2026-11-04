//! `Clear` of an ErrorInfo variable gives a fresh one: the message, title and detailed message are empty
//! again (errors/error-texts/ErrorMessage.al:184-206, `Error` of a cleared ErrorInfo raises ''), its SystemId
//! is the empty Guid (errors/errorinfo/BcErrorInfoSystemId.al:55-66), and a TestField given it
//! raises the plain text (record/testfield/RecordTestFieldGroupA.al:59-72). A variable assigned from it shares it
//! and is cleared too (errorinfo Copy_Semantics, codeunit 73026: `b kept` is '').

use al_testkit::text;

const SRC: &str = r#"
codeunit 69710 C
{
    procedure F(): Text
    var Info: ErrorInfo; Copy: ErrorInfo; T: Text;
    begin
        Info.Message := 'm';
        Info.Title := 't';
        Info.DetailedMessage := 'd';
        Copy := Info;
        Clear(Info);
        T := '[' + Info.Message + '|' + Info.Title + '|' + Info.DetailedMessage + '|' + Copy.Message + ']';
        exit(T);
    end;
}
"#;

#[test]
fn clearing_an_errorinfo_clears_every_variable_that_shares_it() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "F")), "[|||]");
}
