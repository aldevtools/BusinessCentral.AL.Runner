//! XmlPort export with FormatEvaluate = Xml of a Time and a DateTime with milliseconds: the milliseconds without trailing zeros
//! (`12:34:56.789`, `12:34:56.5`, `12:34:56.007`, `00:00:00` without any), a DateTime with the same fraction and `Z`; a database
//! table holds the value SQL Server stored (.789 is .79, 23:59:59.999 is the next day's 00:00:00). A zero Duration is `P0DT0H0M0.0S`. Corpus writer TU's branch
//! al-runtime-rs-staging-tu, xmlport/PWXmlPortFormatEvaluate.al TU_Export_TimeMilliseconds (xmlport 90130).

use al_testkit::run;

fn src(temp: bool) -> String {
    format!(
        r#"
table 69980 "TM Blob" {{ fields {{ field(1; Code; Code[10]) {{ }} field(2; Data; Blob) {{ }} }} keys {{ key(PK; Code) {{ }} }} }}
table 69981 "TM Row" {{ fields {{ field(1; Id; Integer) {{ }} field(2; Tm; Time) {{ }} field(3; Dtm; DateTime) {{ }} field(4; Dur; Duration) {{ }} }} keys {{ key(PK; Id) {{ }} }} }}
xmlport 69982 "TM Port"
{{
    Direction = Export; Format = Xml; FormatEvaluate = Xml; UseRequestPage = false;
    schema {{ textelement(Root) {{ tableelement(T; "TM Row") {{ UseTemporary = {temp}; fieldelement(Id; T.Id) {{ }} fieldelement(Tm; T.Tm) {{ }} fieldelement(Dtm; T.Dtm) {{ }} fieldelement(Dur; T.Dur) {{ }} }} }} }}
    procedure AddRow(NewId: Integer; NewTime: Time)
    begin
        T.Id := NewId; T.Tm := NewTime;
        if NewTime <> 0T then T.Dtm := CreateDateTime(DMY2Date(5, 3, 2024), NewTime);
        T.Insert();
    end;
}}
codeunit 69983 C
{{
    procedure Export(): Text
    var B: Record "TM Blob" temporary; R: Record "TM Row"; P: XmlPort "TM Port"; OutS: OutStream; InS: InStream; Line: Text; T: Text;
    begin
        if {temp} then begin
            P.AddRow(1, 123456.789T); P.AddRow(2, 123456.5T); P.AddRow(3, 123456.007T); P.AddRow(4, 235959.999T); P.AddRow(5, 000000T);
        end else begin
            R.Id := 1; R.Tm := 123456.789T; R.Dtm := CreateDateTime(DMY2Date(5, 3, 2024), R.Tm); R.Insert();
            R.Id := 4; R.Tm := 235959.999T; R.Dtm := CreateDateTime(DMY2Date(5, 3, 2024), R.Tm); R.Insert();
        end;
        B.Data.CreateOutStream(OutS);
        P.SetDestination(OutS);
        P.Export();
        B.Data.CreateInStream(InS, TextEncoding::UTF16);
        while not InS.EOS() do begin InS.ReadText(Line); T += Line.Trim(); end;
        exit(T);
    end;
}}
"#
    )
}

#[test]
fn a_temporary_record_writes_the_exact_milliseconds() {
    let out = al_testkit::text(run(&src(true), "Export"));
    assert!(
        out.ends_with(
            "<Root><T><Id>1</Id><Tm>12:34:56.789</Tm><Dtm>2024-03-05T12:34:56.789Z</Dtm><Dur>P0DT0H0M0.0S</Dur></T>\
<T><Id>2</Id><Tm>12:34:56.5</Tm><Dtm>2024-03-05T12:34:56.5Z</Dtm><Dur>P0DT0H0M0.0S</Dur></T>\
<T><Id>3</Id><Tm>12:34:56.007</Tm><Dtm>2024-03-05T12:34:56.007Z</Dtm><Dur>P0DT0H0M0.0S</Dur></T>\
<T><Id>4</Id><Tm>23:59:59.999</Tm><Dtm>2024-03-05T23:59:59.999Z</Dtm><Dur>P0DT0H0M0.0S</Dur></T>\
<T><Id>5</Id><Tm>00:00:00</Tm><Dtm>2024-03-05T00:00:00Z</Dtm><Dur>P0DT0H0M0.0S</Dur></T></Root>"
        ),
        "{out}"
    );
}

#[test]
fn a_database_table_writes_what_sql_server_stored() {
    let out = al_testkit::text(run(&src(false), "Export"));
    assert!(
        out.ends_with(
            "<Root><T><Id>1</Id><Tm>12:34:56.79</Tm><Dtm>2024-03-05T12:34:56.79Z</Dtm><Dur>P0DT0H0M0.0S</Dur></T>\
<T><Id>4</Id><Tm>00:00:00</Tm><Dtm>2024-03-06T00:00:00Z</Dtm><Dur>P0DT0H0M0.0S</Dur></T></Root>"
        ),
        "{out}"
    );
}
