//! The error of a rejected Date text follows the language in force
//! (format-evaluate/EvalLastError.al:463-481, BT_Evaluate_DottedDate_Language1031).

use al_builtins::native::Signal;

fn run(src: &str) -> Result<String, Signal> {
    al_testkit::run_with(src, "C", "F", vec![]).map(|(v, _)| al_testkit::text(Ok(v)))
}

const PROC: &str = "[TryFunction] local procedure TryEval(var D: Date; T: Text) begin Evaluate(D, T); end;";

#[test]
fn the_date_error_is_german_in_language_1031() {
    let src = format!(
        "codeunit 50100 C {{ {PROC} procedure F(): Text var D: Date; begin GlobalLanguage(1031); \
         if TryEval(D, '29.05.2018') then exit('ok'); exit(GetLastErrorText()); end; }}"
    );
    assert_eq!(
        run(&src).unwrap(),
        "Das Format Date „29.05.2018“ stimmt nicht mit den Datumseinstellungen Ihres Geräts überein.\\\\Stellen Sie sicher, dass Tag, Monat und Jahr die richtige Reihenfolge aufweisen und das richtige Trennzeichen verwendet wird (z. b. - oder /). Alternativ können Sie das Format „jjjj-mm-tt“ verwenden."
    );
}

#[test]
fn the_json_date_conversion_error_is_german_in_language_1031() {
    // format-evaluate/EvalLastError.al:463-481 (`1031 json`).
    let src = "codeunit 50100 C { [TryFunction] local procedure TryJ(T: Text; var D: Date) var J: JsonValue; begin \
               J.SetValue(T); D := J.AsDate(); end; procedure F(): Text var D: Date; begin GlobalLanguage(1031); \
               if TryJ('29.05.2018', D) then exit('ok'); exit(GetLastErrorText()); end; }";
    assert_eq!(
        run(src).unwrap(),
        "Die Konvertierung von Microsoft.Dynamics.Nav.Runtime.NavJsonValue in Microsoft.Dynamics.Nav.Runtime.NavDate ist nicht möglich."
    );
}

#[test]
fn the_date_error_in_another_language_is_refused() {
    let src = format!(
        "codeunit 50100 C {{ {PROC} procedure F(): Text var D: Date; begin GlobalLanguage(1036); \
         if TryEval(D, '29.05.2018') then exit('ok'); exit(GetLastErrorText()); end; }}"
    );
    match run(&src) {
        Err(Signal::Unsupported(_)) => {}
        other => panic!("{other:?}"),
    }
}
