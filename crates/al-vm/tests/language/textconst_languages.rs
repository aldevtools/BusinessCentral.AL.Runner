//! A TextConst reads the text of the language in force, else its ENU text, else (with no ENU text) the text of its only
//! language, or of DEU then FRA in 1033 (text/P140TextConstNoEnu.al, P140TextConstLang.al). Other
//! languages than 1033, 1031 and 1036 are refused.

use al_builtins::native::Signal;

fn code(body: &str) -> String {
    format!("codeunit 50100 C {{ {body} }}")
}

fn read(src: &str) -> String {
    al_testkit::text(al_testkit::run_with(src, "C", "F", vec![]).map(|(v, _)| v))
}

const DECLS: &str = "var DeuFra: TextConst DEU = 'Hallo', FRA = 'Bonjour'; OnlyFra: TextConst FRA = 'Seulement'; \
                     EnuEmptyDeu: TextConst ENU = '', DEU = 'Leer'; EnuDeu: TextConst ENU = 'test', DEU = 'Pruefung';";

fn refused(src: &str) {
    match al_testkit::run_with(src, "C", "F", vec![]) {
        Err(Signal::Unsupported(why)) => assert!(why.contains("TextConst"), "{why}"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_textconst_without_enu_reads_its_only_or_its_deu_first_text_in_the_default_language() {
    // P140TextConstNoEnu.al:23-71 (P140_ReadAsText_DefaultLanguage): `deufra_text` 'Hallo', `onlyfra_text` 'Seulement'.
    let src = code(&format!(
        "{DECLS} procedure F(): Text begin exit(DeuFra + '|' + OnlyFra + '|' + Format(StrLen(DeuFra))); end;"
    ));
    assert_eq!(read(&src), "Hallo|Seulement|5");
    let local = code("procedure F(): Text var L: TextConst DEU = 'Nur Deutsch'; begin exit(L); end;");
    assert_eq!(read(&local), "Nur Deutsch");
}

#[test]
fn a_textconst_reads_the_text_of_the_language_in_force() {
    // P140TextConstLang.al:20-56 (P140_ReadAsText_Deu), :58-91 (P140_ReadAsText_Fra), :163-191 (BM_TextConst_Methods).
    let src = code(&format!(
        "{DECLS} procedure F(): Text var R: Text; begin GlobalLanguage(1031); \
         R := DeuFra + '|' + OnlyFra + '|[' + EnuEmptyDeu + ']|' + EnuDeu; GlobalLanguage(1036); \
         R += '|' + DeuFra + '|[' + EnuEmptyDeu + ']|' + EnuDeu; GlobalLanguage(1033); R += '|' + DeuFra + '|' + EnuDeu; \
         exit(R); end;"
    ));
    assert_eq!(read(&src), "Hallo|Seulement|[Leer]|Pruefung|Bonjour|[]|test|Hallo|test");
}

#[test]
fn a_textconst_whose_fallback_is_not_pinned_is_refused() {
    // Without ENU and without the language in force, only one language or DEU then FRA in 1033 is pinned.
    refused(&code(
        "var G: TextConst FRA = 'Bonjour', DEU = 'Hallo'; procedure F(): Text begin exit(G); end;",
    ));
    // A language code the runtime cannot map to a language id, read in a language it could stand for.
    refused(&code(
        "var G: TextConst ENU = 'Hello', NLD = 'Hallo'; procedure F(): Text begin GlobalLanguage(1043); exit(G); end;",
    ));
}

#[test]
fn a_textconst_read_in_a_language_no_test_pins_is_refused() {
    // Every pinned read runs in 1033, 1031 or 1036 (P140TextConstLang.al, P140TextConstNoEnu.al): another language may
    // fall back through its parent culture, so even an ENU text or a single text is not pinned there.
    refused(&code(
        "var G: TextConst ENU = 'Hello', DEU = 'Hallo'; procedure F(): Text begin GlobalLanguage(1030); exit(G); end;",
    ));
    refused(&code(
        "var G: TextConst DEU = 'Hallo'; procedure F(): Text begin GlobalLanguage(2055); exit(G); end;",
    ));
}

#[test]
fn a_textconst_with_deu_first_and_a_language_other_than_fra_is_refused() {
    // Only `DEU = .., FRA = ..` read where neither matches is pinned (P140TextConstNoEnu.al:23-71, `DeuFra` in 1033).
    refused(&code(
        "var G: TextConst DEU = 'Hallo', NLD = 'Hallo'; procedure F(): Text begin exit(G); end;",
    ));
    refused(&code(
        "var G: TextConst DEU = 'Hallo', FRA = 'Bonjour', ITA = 'Ciao'; procedure F(): Text begin exit(G); end;",
    ));
    refused(&code(
        "var G: TextConst DEU = 'Hallo', NLD = 'Hallo'; procedure F(): Text begin GlobalLanguage(1036); exit(G); end;",
    ));
}
