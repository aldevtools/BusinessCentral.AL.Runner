//! Nested table elements without LinkTable (corpus writer TU's branch al-runtime-rs-staging-tu,
//! xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al TU_NestedTemp_*, TU_NestedReal_*, xmlports 90128/90129): each
//! pass starts without filters (a filter OnPreXmlItem set at one pass is gone at the next) and keeps the record's row;
//! `MinOccurs = Once` without rows writes the record as it stands and runs the elements below; `MaxOccurs = Once` writes the
//! first row and leaves the record on the second.

use al_builtins::native::Signal;
use al_testkit::run;

fn src(mode: i32, outers: &str, mids: &str, inners: &str) -> String {
    format!(
        r#"
table 69990 "NU Blob" {{ fields {{ field(1; Code; Code[10]) {{ }} field(2; Data; Blob) {{ }} }} keys {{ key(PK; Code) {{ }} }} }}
table 69991 "NU Row" {{ fields {{ field(1; Id; Integer) {{ }} field(2; Parent; Integer) {{ }} }} keys {{ key(PK; Id) {{ }} key(KP; Parent) {{ }} }} }}
xmlport 69992 "NU Port"
{{
    Direction = Export; Format = Xml; UseRequestPage = false;
    schema
    {{
        textelement(root)
        {{
            tableelement(outer; "NU Row")
            {{
                UseTemporary = true;
                fieldattribute(id; outer.Id) {{ }}
                tableelement(mid; "NU Row")
                {{
                    UseTemporary = true; MinOccurs = Once; MaxOccurs = Unbounded;
                    fieldattribute(id; mid.Id) {{ }}
                    tableelement(inner; "NU Row")
                    {{
                        UseTemporary = true; MinOccurs = Zero; MaxOccurs = Once;
                        fieldattribute(id; inner.Id) {{ }}
                        trigger OnPreXmlItem()
                        begin
                            Log += 'pre inner f=' + inner.GetFilters() + ' at=' + Format(inner.Id) + ';';
                            if Mode = 1 then inner.SetRange(Parent, mid.Id);
                        end;
                        trigger OnAfterGetRecord()
                        begin
                            Log += 'get inner ' + Format(inner.Id) + ';';
                        end;
                    }}
                    trigger OnPreXmlItem()
                    begin
                        Pass += 1;
                        Log += 'pre mid f=' + mid.GetFilters() + ' at=' + Format(mid.Id) + ';';
                        if (Mode = 1) or ((Mode = 2) and (Pass = 1)) then mid.SetRange(Parent, outer.Id);
                        if (Mode = 3) and (Pass = 1) then mid.SetCurrentKey(Parent);
                    end;
                    trigger OnAfterGetRecord()
                    begin
                        Log += 'get mid ' + Format(mid.Id) + ';';
                    end;
                }}
                trigger OnAfterGetRecord()
                begin
                    if Mode = 4 then mid.SetRange(Parent, outer.Id);
                end;
            }}
        }}
    }}
    var Log: Text; Mode: Integer; Pass: Integer;
    procedure Setup(M: Integer)
    begin
        Mode := M;
        {outers}
        {mids}
        {inners}
    end;
    procedure GetLog(): Text
    begin
        exit(Log);
    end;
}}
codeunit 69993 C
{{
    procedure Export(): Text
    var B: Record "NU Blob" temporary; P: XmlPort "NU Port"; OutS: OutStream; InS: InStream; Line: Text; T: Text;
    begin
        P.Setup({mode});
        B.Data.CreateOutStream(OutS);
        P.SetDestination(OutS);
        P.Export();
        B.Data.CreateInStream(InS, TextEncoding::UTF16);
        while not InS.EOS() do begin InS.ReadText(Line); T += Line.Trim(); end;
        exit(T + '#' + P.GetLog());
    end;
}}
"#
    )
}

fn rows(var: &str, list: &[(i32, i32)]) -> String {
    list.iter()
        .map(|(id, p)| format!("{var}.Id := {id}; {var}.Parent := {p}; {var}.Insert();"))
        .collect()
}

fn export(mode: i32, outers: &[(i32, i32)], mids: &[(i32, i32)], inners: &[(i32, i32)]) -> Result<String, Signal> {
    run(
        &src(mode, &rows("outer", outers), &rows("mid", mids), &rows("inner", inners)),
        "Export",
    )
    .map(|v| al_testkit::text(Ok(v)))
}

fn body(s: &str) -> &str {
    &s[s.find("<root>").expect("root")..]
}

#[test]
fn filtered_passes_start_without_the_filters_of_the_pass_before() {
    // TU_NestedTemp_TwoTwoNoneFiltered: outer 2 has no mid row; `mid` (MinOccurs = Once) writes the record as it stands.
    let out = export(1, &[(1, 0), (2, 0)], &[(11, 1), (12, 1)], &[]).unwrap();
    assert_eq!(
        body(&out),
        "<root><outer id=\"1\"><mid id=\"11\" /><mid id=\"12\" /></outer><outer id=\"2\"><mid id=\"12\" /></outer></root>#\
pre mid f= at=12;get mid 11;pre inner f= at=0;get mid 12;pre inner f= at=0;pre mid f= at=12;pre inner f= at=0;"
    );
}

#[test]
fn a_filter_of_the_first_pass_is_gone_at_the_second() {
    // TU_NestedReal_ItemFilterFirstPass: the second pass sees every row.
    let out = export(2, &[(1, 0), (2, 0)], &[(11, 1), (12, 2)], &[]).unwrap();
    assert_eq!(
        body(&out),
        "<root><outer id=\"1\"><mid id=\"11\" /></outer><outer id=\"2\"><mid id=\"11\" /><mid id=\"12\" /></outer></root>#\
pre mid f= at=12;get mid 11;pre inner f= at=0;pre mid f= at=11;get mid 11;pre inner f= at=0;get mid 12;pre inner f= at=0;"
    );
}

#[test]
fn max_occurs_once_writes_the_first_row_and_leaves_the_record_on_the_second() {
    // TU_NestedReal_SlotNoFilter (six rows: `at=102` at the next pass), TU_NestedTemp_TwoNoneTwo.
    let out = export(0, &[(1, 0), (2, 0)], &[], &[(21, 0), (22, 0), (23, 0)]).unwrap();
    assert_eq!(
        body(&out),
        "<root><outer id=\"1\"><mid id=\"0\"><inner id=\"21\" /></mid></outer><outer id=\"2\"><mid id=\"0\"><inner id=\"21\" /></mid></outer></root>#\
pre mid f= at=0;pre inner f= at=23;get inner 21;pre mid f= at=0;pre inner f= at=22;get inner 21;"
    );
}

#[test]
fn a_key_set_at_one_pass_and_a_filter_set_between_passes_are_refused() {
    for mode in [3, 4] {
        match export(mode, &[(1, 0), (2, 0)], &[(11, 1), (12, 2)], &[]) {
            Err(Signal::Unsupported(why)) => assert!(why.contains("not pinned"), "{why}"),
            other => panic!("mode {mode}: {other:?}"),
        }
    }
}
