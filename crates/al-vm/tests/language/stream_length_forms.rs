//! `OutStream.Write(Value, Length)` and `InStream.Read(var Value, Length)` of fixed-size types, `Write` of an Option, and
//! `Write` of a Record or a RecordId (streams/instream-outstream/StreamLengthIntegers.al, StreamLengthOthers.al,
//! streams/media/P102WriteRecord.al).

const TABLE: &str =
    "table 50100 T { fields { field(1; Id; Integer) { } field(2; Data; Blob) { } } keys { key(PK; Id) { } } }";

fn run(body: &str) -> String {
    let src = format!(
        "{TABLE}
        codeunit 50100 C {{
        procedure F(): Text var R: Record T; O: OutStream; I: InStream; N: Integer; S: Text; Opt: Option A,B,C; Big: BigInteger; begin
        {body} end; }}"
    );
    al_testkit::text(al_testkit::run_in(&src, "C", "F"))
}

#[test]
fn the_length_of_the_type_writes_and_reads_as_without_one() {
    let out = run("R.Data.CreateOutStream(O); S := Format(O.Write(258, 4)); Big := 7; S += Format(O.Write(Big, 8));
        Opt := Opt::B; S += Format(O.Write(Opt, 4));
        R.Data.CreateInStream(I); S += Format(I.Read(N, 4)) + Format(N); Big := 0; S += Format(I.Read(Big, 8)) + Format(Big);
        Opt := Opt::A; S += Format(I.Read(Opt, 4)) + Format(Opt) + Format(I.EOS());
        S += Format(I.Read(Opt, 4)) + Format(Opt);
        exit(S);");
    assert_eq!(out, "4844258874BYes0A");
}

#[test]
fn another_length_raises_invalid_stream_length() {
    let src = format!(
        "{TABLE}
        codeunit 50100 C {{
        procedure W(): Text var R: Record T; O: OutStream; N: Integer; begin R.Data.CreateOutStream(O); N := O.Write(258, 2); end;
        procedure Rd(): Text var R: Record T; O: OutStream; I: InStream; N: Integer; D: Decimal; begin
            R.Data.CreateOutStream(O); O.Write(1.5); R.Data.CreateInStream(I); N := I.Read(D, 8); end; }}"
    );
    let w = al_testkit::error_message(al_testkit::run_in(&src, "C", "W"));
    assert_eq!(
        w,
        "An invalid length 2 was specified in stream function. 4 was expected."
    );
    let r = al_testkit::error_message(al_testkit::run_in(&src, "C", "Rd"));
    assert_eq!(
        r,
        "An invalid length 8 was specified in stream function. 12 was expected."
    );
}

#[test]
fn a_record_or_a_record_id_cannot_be_written() {
    let src = format!(
        "{TABLE}
        codeunit 50100 C {{
        procedure W(): Text var R: Record T; O: OutStream; begin R.Data.CreateOutStream(O); O.Write(R); end;
        procedure Id(): Text var R: Record T; O: OutStream; N: Integer; begin R.Data.CreateOutStream(O); N := O.Write(R.RecordId(), 4); end; }}"
    );
    for routine in ["W", "Id"] {
        let m = al_testkit::error_message(al_testkit::run_in(&src, "C", routine));
        assert_eq!(m, "ALStream.ALWrite, invalid stream mode: Stream", "{routine}");
    }
}
