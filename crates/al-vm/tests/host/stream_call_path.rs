//! `InStream.Read` and `OutStream.Write` on a local stream variable run on the stream path (`Session::run_stream_builtin`,
//! the native on the local in place); on a global they run on `run_builtin`. Both give the same results, the raise of a read
//! after the end included, and the local stays usable after it (a `var` parameter too).

use al_testkit::text;

fn src(local: bool) -> String {
    let (decl, globals) = if local {
        ("Rec: Record B temporary; OutS: OutStream; InS: InStream;", "")
    } else {
        ("", "var Rec: Record B temporary; OutS: OutStream; InS: InStream;")
    };
    format!(
        r#"
table 50196 B {{ TableType = Temporary; fields {{ field(1; Id; Integer) {{ }} field(2; Data; Blob) {{ }} }} keys {{ key(PK; Id) {{ }} }} }}
codeunit 50196 C
{{
    {globals}

    procedure F(): Text
    var
        {decl} By: Byte; I: Integer; N: Integer; D: Decimal; R: Text;
    begin
        Rec.Data.CreateOutStream(OutS);
        for I := 1 to 5 do begin By := 64 + I; N := OutS.Write(By); end;
        R += Format(N) + ';';
        D := 1.5;
        OutS.Write(D);
        Rec.Data.CreateInStream(InS);
        for I := 1 to 5 do begin N := InS.Read(By); R += Format(N) + ':' + Format(By) + ';'; end;
        N := InS.Read(D);
        R += Format(N) + ':' + Format(D) + ';';
        N := InS.Read(By);
        R += Format(N) + ':' + Format(By) + ';';
        asserterror InS.Read(By);
        R += GetLastErrorCode() + ';' + Format(InS.EOS()) + ';';
        Rec.Data.CreateInStream(InS);
        ReadOne(InS, R);
        ReadOne(InS, R);
        exit(R);
    end;

    local procedure ReadOne(var S: InStream; var R: Text)
    var
        By: Byte; N: Integer;
    begin
        N := S.Read(By);
        R += Format(N) + ':' + Format(By) + ';';
    end;
}}
"#
    )
}

fn run(local: bool) -> String {
    text(al_testkit::run_in(&src(local), "C", "F"))
}

#[test]
fn a_local_stream_reads_and_writes_as_a_global_one() {
    let global = run(false);
    assert_eq!(run(true), global);
    // What main's path gives (a global receiver).
    assert_eq!(
        global,
        "1;1:A;1:B;1:C;1:D;1:E;12:1.5;0:\0;InvalidStreamData;Yes;1:A;1:B;"
    );
}
