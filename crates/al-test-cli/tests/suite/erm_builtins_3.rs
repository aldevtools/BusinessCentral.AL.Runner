//! Builtins that kept about 1000 Tests-ERM tests from running (issue 22): `SecretText.Unwrap`,
//! `XmlDocument.ReadFrom(Text, XmlReadOptions, var XmlDocument)`, `BigText.Read` and
//! `Media.ImportStream` with a file name. What no corpus test pins is refused with a reason.

use al_test_cli::report::Outcome;
use al_test_cli::runtime::run_codeunit;
use al_vm::Engine;

fn compile(tests: &str) -> (al_sema::lower::Lowered, usize) {
    let src = format!(
        "table 62800 \"B3 Row\" {{ fields {{ field(1; \"No.\"; Code[20]) {{ }} field(2; Pic; Media) {{ }} }} keys {{ key(PK; \"No.\") {{ }} }} }}
         codeunit 62801 T {{ Subtype = Test; {tests} }}"
    );
    let l = al_testkit::compile(&src);
    let oi = l.program.objects.iter().position(|o| o.name == "T").unwrap();
    (l, oi)
}

fn outcomes(tests: &str) -> Vec<(String, Outcome)> {
    let (l, oi) = compile(tests);
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let engine = Engine::new(l.program.clone(), blocked);
    run_codeunit(&engine, &l, oi)
}

// types/TestSecretText.al:43: Unwrap is "the one API that returns the plain text"; what a secret
// carries is pinned there (SecretStrSubstNo, IsEmpty).
#[test]
fn unwrap_returns_the_text_a_secret_carries() {
    let o = outcomes(
        "[Test] procedure Unwrap() var S: SecretText; T: Text; begin
             S := SecretStrSubstNo('a-%1', SecretStrSubstNo('b'));
             T := S.Unwrap(); if T <> 'a-b' then Error('got %1', T); end;
         [Test] procedure UnwrapEmpty() var S: SecretText; begin if S.Unwrap() <> '' then Error('not empty'); end;",
    );
    for (n, r) in &o {
        assert_eq!(r, &Outcome::Pass, "{n}");
    }
}

// misc/W4XmlJsonHttp.al (W4 Xml Options Tests.ReadFrom_*): the default XmlReadOptions and PreserveWhitespace true or
// false read white space between tags and around the root as the form without options does.
#[test]
fn read_from_text_with_xml_read_options_reads_white_space_as_the_form_without_options_does() {
    let o = outcomes(
        "[Test] procedure Compact() var D: XmlDocument; O: XmlReadOptions; R: Text; begin
             if not XmlDocument.ReadFrom('<root><a>x y</a></root>', O, D) then Error('read'); D.WriteTo(R);
             if not R.Contains('<a>x y</a>') then Error('got %1', R); end;
         [Test] procedure Indented() var D: XmlDocument; O: XmlReadOptions; R: Text; begin
             O.PreserveWhitespace(false);
             if not XmlDocument.ReadFrom('<root>  <a/></root>', O, D) then Error('read'); D.WriteTo(R);
             if not R.Contains('<root>  <a />') then Error('got %1', R); end;
         [Test] procedure Outside() var D: XmlDocument; O: XmlReadOptions; begin
             if not XmlDocument.ReadFrom(' <root/>', O, D) then Error('read'); end;",
    );
    for (n, r) in &o {
        assert_eq!(r, &Outcome::Pass, "{n}");
    }
}

#[test]
fn media_import_stream_with_a_file_name_is_lowered() {
    // Base Application's Document Attachment calls it (ImportStream(Stream, '', '', FileName)); what the file
    // name changes is visible only in the Tenant Media row, which is not read from the media store (ADR-017).
    let (l, oi) = compile(
        "procedure Import(var IS: InStream) var R: Record \"B3 Row\"; begin R.Pic.ImportStream(IS, 'd', 'image/png', 'f.png'); end;",
    );
    let ri = l.program.objects[oi]
        .routines
        .iter()
        .position(|r| r.name == "Import")
        .unwrap();
    assert!(l.unsupported[oi][ri].is_empty(), "{:?}", l.unsupported[oi][ri]);
}
