//! The translation files of an app (`Translations/*.xlf`, XLIFF 1.2), ADR-053 step 2.
//!
//! Only what a lookup needs is kept: the file's `target-language` and, for each `<trans-unit>` that gives a text, its `id` and
//! its `<target>`. A unit with `translate="no"`, without a `<target>` or with an empty one gives no text: the source text
//! stays (ADR-053 decision 1). The `state` attribute of a `<target>` is kept per unit (`al_ir::UnitState`); which states
//! give the text is decided where a unit is looked up (al-vm `translations.rs`).

/// One parsed translation file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Xliff {
    /// The `target-language` attribute of `<file>`, as written (`nl-BE`).
    pub target_language: String,
    /// `(id, target)` of every unit that gives a text, sorted by id.
    pub units: Vec<(String, String)>,
    /// The state of each unit of `units`, in the same order; empty when no unit has one.
    pub states: Vec<al_ir::UnitState>,
}

/// The `target-language` of the first `<file>` of an XLIFF file whose first bytes are `start` (enough to hold the `<file>`
/// tag): the language of a package's file without reading all of it.
pub fn target_language(start: &[u8]) -> Result<String, String> {
    use quick_xml::events::Event;
    let start = start.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(start);
    let mut reader = quick_xml::Reader::from_reader(start);
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e) | Event::Empty(e)) if e.name().as_ref() == b"file" => {
                for a in e.attributes() {
                    let a = a.map_err(|err| err.to_string())?;
                    if a.key.as_ref() == b"target-language" {
                        return Ok(a.unescape_value().map_err(|err| err.to_string())?.into_owned());
                    }
                }
                return Err("a <file> without target-language".into());
            }
            Ok(Event::Eof) | Err(_) => return Err("no <file target-language> at the start of the file".into()),
            Ok(_) => {}
        }
        buf.clear();
    }
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
    let mut state = al_ir::UnitState::Absent;
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
                    b"target" if !empty && unit.is_some() => {
                        target = Some(String::new());
                        state = match attr(&e, b"state")? {
                            None => al_ir::UnitState::Absent,
                            Some(v) if v == "translated" => al_ir::UnitState::Translated,
                            Some(v) => al_ir::UnitState::Other(v),
                        };
                    }
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
                            units.push((id.clone(), s, std::mem::take(&mut state)));
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
    units.sort_by(|a, b| a.0.cmp(&b.0));
    if let Some(w) = units.windows(2).find(|w| w[0].0 == w[1].0) {
        return Err(format!("two units with id {}", w[0].0));
    }
    let stated = units.iter().any(|u| u.2 != al_ir::UnitState::Absent);
    let mut states = Vec::with_capacity(if stated { units.len() } else { 0 });
    let units = units
        .into_iter()
        .map(|(id, text, state)| {
            if stated {
                states.push(state);
            }
            (id, text)
        })
        .collect();
    Ok(Xliff {
        target_language,
        units,
        states,
    })
}

/// The file `name` (relative to its app) as the IR keeps it, from its parse: a file that could not be read keeps the reason,
/// and its language refuses every translated read.
pub fn translation_file(name: String, parsed: Result<Xliff, String>) -> al_ir::TranslationFile {
    match parsed {
        Ok(x) => al_ir::TranslationFile {
            name,
            target_language: x.target_language,
            units: x.units,
            error: None,
            states: x.states,
        },
        Err(e) => al_ir::TranslationFile {
            name,
            error: Some(e),
            ..Default::default()
        },
    }
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
    fn a_unit_keeps_the_state_of_its_target() {
        let file = r#"<xliff><file target-language="nl-NL"><body>
<trans-unit id="b"><source>B</source><target state="translated">Bt</target></trans-unit>
<trans-unit id="a"><source>A</source><target>At</target></trans-unit>
<trans-unit id="c"><source>C</source><target state="needs-translation">Ct</target></trans-unit>
</body></file></xliff>"#;
        let x = parse(file.as_bytes()).unwrap();
        let ids: Vec<&str> = x.units.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(ids, ["a", "b", "c"]);
        assert_eq!(
            x.states,
            vec![
                al_ir::UnitState::Absent,
                al_ir::UnitState::Translated,
                al_ir::UnitState::Other("needs-translation".into())
            ]
        );
        // A file whose units have no state keeps no states.
        assert!(parse(FILE.as_bytes()).unwrap().states.is_empty());
    }

    #[test]
    fn the_language_is_read_from_the_start_of_a_file() {
        let start = &FILE.as_bytes()[..FILE.find("<body>").unwrap()];
        assert_eq!(target_language(start).unwrap(), "nl-BE");
        assert_eq!(target_language(FILE.as_bytes()).unwrap(), "nl-BE");
        assert!(target_language(b"<?xml version=\"1.0\"?><xliff><fi").is_err());
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
