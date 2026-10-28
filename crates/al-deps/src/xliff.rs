//! The translation files of an app (`Translations/*.xlf`, XLIFF 1.2), ADR-053 step 2.
//!
//! Only what a lookup needs is kept: the file's `target-language` and, for each `<trans-unit>` that gives a text, its `id` and
//! its `<target>`. A unit with `translate="no"`, without a `<target>` or with an empty one gives no text: the source text
//! stays (ADR-053 decision 1). The `state` attribute is not read.

/// One parsed translation file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Xliff {
    /// The `target-language` attribute of `<file>`, as written (`nl-BE`).
    pub target_language: String,
    /// `(id, target)` of every unit that gives a text, sorted by id.
    pub units: Vec<(String, String)>,
}

/// Parses an XLIFF 1.2 file. A file with no `<file target-language>`, two `<file>` elements with different languages,
/// two units with one id, or markup inside a `<target>` is an error: which text BC shows then is not pinned.
pub fn parse(bytes: &[u8]) -> Result<Xliff, String> {
    use quick_xml::events::Event;
    let text = std::str::from_utf8(bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes))
        .map_err(|e| format!("not UTF-8: {e}"))?;
    let mut reader = quick_xml::Reader::from_str(text);
    let attr = |e: &quick_xml::events::BytesStart, k: &[u8]| -> Result<Option<String>, String> {
        for a in e.attributes() {
            let a = a.map_err(|err| err.to_string())?;
            if a.key.as_ref() == k {
                return Ok(Some(a.unescape_value().map_err(|err| err.to_string())?.into_owned()));
            }
        }
        Ok(None)
    };
    let mut language: Option<String> = None;
    let mut units = Vec::new();
    // The unit being read: its id and whether it is translated.
    let mut unit: Option<(String, bool)> = None;
    let mut target: Option<String> = None;
    loop {
        match reader.read_event().map_err(|e| e.to_string())? {
            Event::Eof => break,
            Event::Start(e) | Event::Empty(e) if target.is_some() => {
                return Err(format!(
                    "markup <{}> inside a <target>",
                    String::from_utf8_lossy(e.name().as_ref())
                ));
            }
            ev @ (Event::Start(_) | Event::Empty(_)) => {
                let empty = matches!(ev, Event::Empty(_));
                let (Event::Start(e) | Event::Empty(e)) = ev else {
                    unreachable!()
                };
                match e.name().as_ref() {
                    b"file" => {
                        let l = attr(&e, b"target-language")?.ok_or("a <file> without target-language")?;
                        match &language {
                            Some(prev) if !prev.eq_ignore_ascii_case(&l) => {
                                return Err(format!("two <file> elements for {prev} and {l}"));
                            }
                            _ => language = Some(l),
                        }
                    }
                    b"trans-unit" if !empty => {
                        let id = attr(&e, b"id")?.ok_or("a <trans-unit> without id")?;
                        let translate = attr(&e, b"translate")?.is_none_or(|v| !v.eq_ignore_ascii_case("no"));
                        unit = Some((id, translate));
                    }
                    b"target" if !empty && unit.is_some() => target = Some(String::new()),
                    _ => {}
                }
            }
            Event::Text(t) => {
                if let Some(s) = target.as_mut() {
                    s.push_str(&t.unescape().map_err(|e| e.to_string())?);
                }
            }
            Event::CData(t) => {
                if let Some(s) = target.as_mut() {
                    s.push_str(&String::from_utf8_lossy(&t));
                }
            }
            Event::End(e) => match e.name().as_ref() {
                b"target" => {
                    if let (Some(s), Some((id, true))) = (target.take(), &unit) {
                        if !s.is_empty() {
                            units.push((id.clone(), s));
                        }
                    }
                }
                b"trans-unit" => unit = None,
                _ => {}
            },
            _ => {}
        }
    }
    let target_language = language.ok_or("no <file target-language>")?;
    units.sort();
    if let Some(w) = units.windows(2).find(|w| w[0].0 == w[1].0) {
        return Err(format!("two units with id {}", w[0].0));
    }
    Ok(Xliff { target_language, units })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The file of al-runner issue 5772, cut down, with units that give no text.
    const FILE: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<xliff version="1.2" xmlns="urn:oasis:names:tc:xliff:document:1.2">
  <file datatype="xml" source-language="en-US" target-language="nl-BE" original="AL Runner Repros">
    <body>
      <group id="body">
        <trans-unit id="Codeunit 414859830 - NamedType 868134460" size-unit="char" translate="yes" xml:space="preserve">
          <source>Hello world</source>
          <target>Hallo &amp; wereld </target>
          <note from="Xliff Generator" annotates="general" priority="3">Codeunit ALR Labels - NamedType HelloLbl</note>
        </trans-unit>
        <trans-unit id="Enum 1 - EnumValue 2 - Property 2879900210" translate="no">
          <source>Released</source>
          <target>Vrijgegeven</target>
        </trans-unit>
        <trans-unit id="Enum 1 - EnumValue 3 - Property 2879900210">
          <source>Open</source>
          <target></target>
        </trans-unit>
        <trans-unit id="Enum 1 - EnumValue 4 - Property 2879900210">
          <source>Done</source>
          <target/>
        </trans-unit>
        <trans-unit id="Enum 1 - EnumValue 5 - Property 2879900210">
          <source>None</source>
        </trans-unit>
      </group>
    </body>
  </file>
</xliff>"#;

    #[test]
    fn a_file_gives_its_language_and_the_units_that_give_a_text() {
        let x = parse(FILE.as_bytes()).unwrap();
        assert_eq!(x.target_language, "nl-BE");
        assert_eq!(
            x.units,
            vec![(
                "Codeunit 414859830 - NamedType 868134460".to_string(),
                "Hallo & wereld ".to_string()
            )]
        );
    }

    #[test]
    fn a_file_without_a_language_or_with_markup_in_a_target_is_an_error() {
        assert!(parse(b"<xliff><file></file></xliff>").is_err());
        let markup = r#"<xliff><file target-language="nl-BE"><trans-unit id="a"><target>x<g id="1">y</g></target></trans-unit></file></xliff>"#;
        assert!(parse(markup.as_bytes()).unwrap_err().contains("markup"));
        let twice = r#"<xliff><file target-language="nl-BE"><trans-unit id="a"><target>x</target></trans-unit><trans-unit id="a"><target>y</target></trans-unit></file></xliff>"#;
        assert!(parse(twice.as_bytes()).unwrap_err().contains("two units"));
    }
}
