//! `<Second dec>` and `<Second dec.>` (with the period, as the test libraries' unique-code builders write it), their widths and
//! case, on a Time and a DateTime (al-runner issue 6650). Every value below is real BC's: corpus writer Y64, branch
//! al-runtime-rs-staging-y64 85d86e7b, types/lt-datetime/Format.al:1066-1300 (Y64_Time_Format_SecondDecPeriod, _More, _Edges,
//! Y64_DateTime_Format_SecondDecPeriod, Y64_SecondDecPeriod_ReporterCode).

use al_testkit::text;

const SRC: &str = r#"
codeunit 69803 C
{
    [TryFunction] local procedure TryT(V: Time; S: Text; var R: Text) begin R := Format(V, 0, S); end;
    [TryFunction] local procedure TryDT(V: DateTime; S: Text; var R: Text) begin R := Format(V, 0, S); end;

    local procedure Strings(): List of [Text]
    var L: List of [Text];
    begin
        L.Add('<Hours24,2><Filler Character,0><Minutes,2><Seconds,2><Second dec.>');
        L.Add('<Second dec.>');
        L.Add('<Second dec.,3>');
        L.Add('<Second dec.,4>');
        L.Add('<Second Dec.>');
        L.Add('<SECOND DEC.>');
        L.Add('<Second dec,1>');
        L.Add('<Second dec,2>');
        L.Add('<Second dec,4>');
        L.Add('<Second dec,5>');
        L.Add('<Seconds,2><Second dec.>');
        L.Add('<Second dec.><Filler Character,0>');
        L.Add('<Filler Character,0><Second dec,4>');
        L.Add('<Second dec..>');
        L.Add('<Second dec>.');
        exit(L);
    end;

    procedure OfTime(V: Time): Text
    var S: Text; R: Text; T: Text;
    begin
        foreach S in Strings() do
            if TryT(V, S, R) then T += '[' + R + ']' else T += 'error:' + GetLastErrorCode();
        exit(T);
    end;

    procedure OfDateTime(V: DateTime): Text
    var S: Text; R: Text; T: Text;
    begin
        foreach S in Strings() do
            if TryDT(V, S, R) then T += '[' + R + ']' else T += 'error:' + GetLastErrorCode();
        exit(T);
    end;

    procedure Times(): Text
    begin
        exit(OfTime(134506.078T) + '|' + OfTime(090507.5T) + '|' + OfTime(090507.05T) + '|' + OfTime(090507T) + '|' + OfTime(235959.999T) + '|' + OfTime(0T));
    end;

    procedure DateTimes(): Text
    begin
        exit(OfDateTime(CreateDateTime(20240305D, 134506.078T)) + '|' + OfDateTime(CreateDateTime(20240305D, 090507T)));
    end;

    procedure Reporter(): Text
    var CurrTime: Text; T: Text;
    begin
        CurrTime := Format(134506.078T, 0, '<Hours24,2><Filler Character,0><Minutes,2><Seconds,2><Second dec.>');
        T := 'X-' + DelChr(CurrTime, '=', '.,:');
        CurrTime := Format(090507T, 0, '<Hours24,2><Filler Character,0><Minutes,2><Seconds,2><Second dec.>');
        exit(T + ';X-' + DelChr(CurrTime, '=', '.,:'));
    end;

    procedure ErrorText(): Text
    var R: Text;
    begin
        if TryT(134506.078T, '<Second dec..>', R) then exit('no error');
        exit(GetLastErrorText());
    end;
}
"#;

const E: &str = "error:InvalidFormatString";

fn row(cells: [&str; 15]) -> String {
    cells
        .iter()
        .map(|c| if *c == E { E.to_string() } else { format!("[{c}]") })
        .collect()
}

#[test]
fn second_dec_on_a_time() {
    let rows = [
        row([
            "134506.078",
            ".078",
            "***",
            ".078",
            ".078",
            ".078",
            "*",
            "**",
            ".078",
            ".0780",
            "06.078",
            ".078",
            ".078",
            E,
            ".078.",
        ]),
        row([
            "090507.5", ".5", ".50", ".500", ".5", ".5", "*", ".5", ".500", ".5000", "07.5", ".5", ".500", E, ".5.",
        ]),
        row([
            "090507.05",
            ".05",
            ".05",
            ".050",
            ".05",
            ".05",
            "*",
            "**",
            ".050",
            ".0500",
            "07.05",
            ".05",
            ".050",
            E,
            ".05.",
        ]),
        row([
            "090507", "", "000", "0000", "", "", "0", "00", "0000", "00000", "07", "", "0000", E, ".",
        ]),
        row([
            "235959.999",
            ".999",
            "***",
            ".999",
            ".999",
            ".999",
            "*",
            "**",
            ".999",
            ".9990",
            "59.999",
            ".999",
            ".999",
            E,
            ".999.",
        ]),
        row([
            "000000", "", "000", "0000", "", "", "0", "00", "0000", "00000", "00", "", "0000", E, ".",
        ]),
    ];
    assert_eq!(text(al_testkit::run_in(SRC, "C", "Times")), rows.join("|"));
}

#[test]
fn second_dec_on_a_datetime() {
    let rows = [
        row([
            "134506.078",
            ".078",
            "***",
            ".078",
            ".078",
            ".078",
            "*",
            "**",
            ".078",
            ".0780",
            "06.078",
            ".078",
            ".078",
            E,
            ".078.",
        ]),
        row([
            "090507", "", ".00", ".000", "", "", ".", ".0", ".000", ".0000", "07", "", ".000", E, ".",
        ]),
    ];
    assert_eq!(text(al_testkit::run_in(SRC, "C", "DateTimes")), rows.join("|"));
}

#[test]
fn the_reporters_unique_code_and_the_error_text() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "Reporter")), "X-134506078;X-090507");
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "ErrorText")),
        "An invalid field or attribute has been specified for the 'Format' property.\\\\<Second dec..>"
    );
}
