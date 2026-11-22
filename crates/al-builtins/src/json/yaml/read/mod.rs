//! The YAML reader of `ReadFromYaml`: a libyaml-style scanner (`scan`), a parser and the conversion to the tree BC hands
//! on to JSON. Every rule here is measured by the cells of the corpus probes of al-runner issue 8580, identical on BC 27.5,
//! 28.5 and 29.0, read through the assumption `read-from-yaml-probed-shapes` until they are pinned:
//! - PR 1493 `tests/al-language/json/JsonYamlBlankLines.al` (codeunit 75620),
//! - PR 1515 `tests/al-language/json/JsonYamlUtf8.al` (codeunit 75621),
//! - PR 1519 `tests/al-language/json/JsonYamlPunctMatrix.al` (codeunit 75623),
//! - PR 1530 `tests/al-language/json/JsonYamlRemaining.al` (codeunit 75671).
//!
//! Cell names below (`tg4`, `ck22`, `ml13`...) are the cells of PR 1530 unless another file is named. A shape no cell
//! measures refuses (`Fail::Refuse`).

use crate::json::{error, write_string};
use scan::{Style, Tok};
use std::collections::HashMap;

mod scan;

/// A YAML node as BC hands it on to JSON: every untagged scalar but the null forms is a text (P88:230-236, PnA:159 of
/// `corpus/tests/al-language/json/`).
#[derive(Debug, PartialEq)]
pub(crate) enum Yaml {
    Null,
    Text(String),
    /// A JSON number or Boolean written as is (a tagged scalar, `tg2`-`tg4`).
    Raw(String),
    Map(Vec<(String, Yaml)>),
    Seq(Vec<Yaml>),
}

impl Yaml {
    pub(crate) fn write_json(&self, out: &mut String) {
        match self {
            Yaml::Null => out.push_str("null"),
            Yaml::Text(t) => write_string(out, t),
            Yaml::Raw(t) => out.push_str(t),
            Yaml::Map(members) => {
                out.push('{');
                for (i, (k, v)) in members.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    write_string(out, k);
                    out.push(':');
                    v.write_json(out);
                }
                out.push('}');
            }
            Yaml::Seq(items) => {
                out.push('[');
                for (i, v) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    v.write_json(out);
                }
                out.push(']');
            }
        }
    }

    /// The containers on the deepest path, this one counted.
    pub(crate) fn depth(&self) -> usize {
        match self {
            Yaml::Map(m) => 1 + m.iter().map(|(_, v)| v.depth()).max().unwrap_or(0),
            Yaml::Seq(s) => 1 + s.iter().map(Yaml::depth).max().unwrap_or(0),
            _ => 0,
        }
    }
}

/// The YAML error (P88:238, PnA:177-178).
pub(crate) fn bad_yaml() -> crate::native::Signal {
    error("The data does not represent valid YAML.", "YamlBadFormat")
}

/// Why a text does not read.
pub(crate) enum Fail {
    /// `YamlBadFormat`: BC's YAML error.
    Bad,
    /// A shape no cell measures.
    Refuse(&'static str),
}

type Res<T> = Result<T, Fail>;

/// The characters a text may hold: printable ASCII, tab, line breaks, U+00E9 (`literal-e-acute`), U+FEFF (`bt1`-`bt4`)
/// and the characters of corpus PR 1515 (JsonYamlUtf8.al :25-34: U+00A7, U+00E9, U+00FC, U+00F8, U+20AC, U+1F600, U+0301,
/// U+00A0, U+0085, U+2028, U+2029, U+FEFF, U+0080). Anything else is not measured. U+0000 is dropped (`bt22`, `bt23`).
fn measured_char(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\r' | ' '
            ..='~'
                | '\u{80}'
                | '\u{85}'
                | '\u{a0}'
                | '\u{a7}'
                | '\u{e9}'
                | '\u{f8}'
                | '\u{fc}'
                | '\u{301}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{20ac}'
                | '\u{feff}'
                | '\u{1F600}'
    )
}

/// U+0085, U+2028 and U+2029 break a line, as LF does: every place JsonYamlUtf8.al of corpus PR 1515 put one (a plain
/// value `sp-N`, a `>-` line `sf-N`, double quotes `sd-N`, :735-798) answers the YAML error. A text with one that reads
/// is not measured.
fn unicode_break(c: char) -> bool {
    matches!(c, '\u{85}' | '\u{2028}' | '\u{2029}')
}

#[derive(Clone, Debug)]
enum Node {
    Scalar {
        v: String,
        style: Style,
        tag: Option<String>,
    },
    Seq {
        items: Vec<Node>,
        tag: Option<String>,
    },
    Map {
        pairs: Vec<(Node, Node)>,
        tag: Option<String>,
    },
}

struct Parser {
    t: Vec<Tok>,
    i: usize,
    anchors: HashMap<String, Node>,
    tags: HashMap<String, String>,
    depth: usize,
}

/// The text read as a YAML tree.
pub(crate) fn document(text: &str) -> Res<Yaml> {
    let chars: Vec<char> = text.chars().filter(|&c| c != '\0').collect();
    if !chars.iter().all(|&c| measured_char(c)) {
        return Err(Fail::Refuse("ReadFromYaml of a character that no probe measures"));
    }
    let mut p = Parser {
        t: scan::tokens(&chars)?,
        i: 1,
        anchors: HashMap::new(),
        tags: HashMap::new(),
        depth: 0,
    };
    let root = p.stream()?;
    let doc = match root {
        None => Yaml::Null,
        Some(n) => convert(&n)?,
    };
    if chars.iter().any(|&c| unicode_break(c)) {
        return Err(Fail::Refuse(
            "ReadFromYaml of a text with U+0085, U+2028 or U+2029 that reads (not measured)",
        ));
    }
    Ok(doc)
}

fn empty() -> Node {
    Node::Scalar {
        v: String::new(),
        style: Style::Plain,
        tag: None,
    }
}

impl Parser {
    fn peek(&self) -> &Tok {
        &self.t[self.i.min(self.t.len() - 1)]
    }

    fn next(&mut self) -> Tok {
        let t = self.peek().clone();
        self.i += 1;
        t
    }

    fn stream(&mut self) -> Res<Option<Node>> {
        while *self.peek() == Tok::DocEnd {
            self.i += 1;
        }
        if *self.peek() == Tok::StreamEnd {
            return Ok(None);
        }
        let mut directives = false;
        loop {
            match self.peek().clone() {
                Tok::Version => {
                    directives = true;
                    self.i += 1;
                }
                Tok::TagDirective(h, p) => {
                    directives = true;
                    self.tags.insert(h, p);
                    self.i += 1;
                }
                _ => break,
            }
        }
        if *self.peek() == Tok::DocStart {
            self.i += 1;
        } else if directives {
            return Err(Fail::Bad);
        }
        let node = if matches!(self.peek(), Tok::DocStart | Tok::DocEnd | Tok::StreamEnd) {
            empty()
        } else {
            self.node(true, false)?
        };
        if *self.peek() == Tok::DocEnd {
            self.i += 1;
        }
        // Only one document is read (`dc2`, `dc3`, `dc15`, `dc18`, `dc22`).
        if *self.peek() != Tok::StreamEnd {
            return Err(Fail::Bad);
        }
        Ok(Some(node))
    }

    fn resolve(&self, handle: &str, suffix: &str) -> Res<String> {
        match handle {
            "" => Ok(suffix.to_string()),
            "!" => Ok(format!("!{suffix}")),
            "!!" => Ok(format!("tag:yaml.org,2002:{suffix}")),
            h => self.tags.get(h).map(|p| format!("{p}{suffix}")).ok_or(Fail::Bad),
        }
    }

    fn node(&mut self, block: bool, indentless: bool) -> Res<Node> {
        self.depth += 1;
        if self.depth > 600 {
            return Err(Fail::Refuse(
                "ReadFromYaml of a text nested deeper than 600 levels (not measured)",
            ));
        }
        let r = self.node_inner(block, indentless);
        self.depth -= 1;
        r
    }

    fn node_inner(&mut self, block: bool, indentless: bool) -> Res<Node> {
        if let Tok::Alias(name) = self.peek().clone() {
            self.i += 1;
            // An alias to an anchor that is not defined (`an4`, `an8`, `an17`) is an error.
            return self.anchors.get(&name).cloned().ok_or(Fail::Bad);
        }
        let (mut anchor, mut tag) = (None, None);
        loop {
            match self.peek().clone() {
                Tok::Anchor(a) if anchor.is_none() => anchor = Some(a),
                Tok::Tag(h, s) if tag.is_none() => tag = Some(self.resolve(&h, &s)?),
                _ => break,
            }
            self.i += 1;
        }
        let node = match self.peek().clone() {
            Tok::BlockEntry if indentless => Node::Seq {
                items: self.indentless_seq()?,
                tag,
            },
            Tok::Scalar(v, style) => {
                self.i += 1;
                Node::Scalar { v, style, tag }
            }
            Tok::FlowSeqStart => Node::Seq {
                items: self.flow_seq()?,
                tag,
            },
            Tok::FlowMapStart => Node::Map {
                pairs: self.flow_map()?,
                tag,
            },
            Tok::BlockSeqStart if block => Node::Seq {
                items: self.block_seq()?,
                tag,
            },
            Tok::BlockMapStart if block => Node::Map {
                pairs: self.block_map()?,
                tag,
            },
            _ if anchor.is_some() || tag.is_some() => Node::Scalar {
                v: String::new(),
                style: Style::Plain,
                tag,
            },
            _ => return Err(Fail::Bad),
        };
        if let Some(a) = anchor {
            self.anchors.insert(a, node.clone());
        }
        Ok(node)
    }

    fn block_seq(&mut self) -> Res<Vec<Node>> {
        self.i += 1;
        let mut items = Vec::new();
        loop {
            match self.next() {
                Tok::BlockEntry => {
                    if matches!(self.peek(), Tok::BlockEntry | Tok::BlockEnd) {
                        items.push(empty());
                    } else {
                        items.push(self.node(true, false)?);
                    }
                }
                Tok::BlockEnd => return Ok(items),
                _ => return Err(Fail::Bad),
            }
        }
    }

    fn indentless_seq(&mut self) -> Res<Vec<Node>> {
        let mut items = Vec::new();
        while *self.peek() == Tok::BlockEntry {
            self.i += 1;
            if matches!(self.peek(), Tok::BlockEntry | Tok::Key | Tok::Value | Tok::BlockEnd) {
                items.push(empty());
            } else {
                items.push(self.node(true, false)?);
            }
        }
        Ok(items)
    }

    fn block_map(&mut self) -> Res<Vec<(Node, Node)>> {
        self.i += 1;
        let mut pairs = Vec::new();
        loop {
            match self.next() {
                Tok::Key => {
                    let k = if matches!(self.peek(), Tok::Key | Tok::Value | Tok::BlockEnd) {
                        empty()
                    } else {
                        self.node(true, true)?
                    };
                    let v = if *self.peek() == Tok::Value {
                        self.i += 1;
                        if matches!(self.peek(), Tok::Key | Tok::Value | Tok::BlockEnd) {
                            empty()
                        } else {
                            self.node(true, true)?
                        }
                    } else {
                        empty()
                    };
                    pairs.push((k, v));
                }
                Tok::BlockEnd => return Ok(pairs),
                _ => return Err(Fail::Bad),
            }
        }
    }

    fn flow_seq(&mut self) -> Res<Vec<Node>> {
        self.i += 1;
        let mut items = Vec::new();
        let mut first = true;
        loop {
            if *self.peek() == Tok::FlowSeqEnd {
                self.i += 1;
                return Ok(items);
            }
            if !first {
                if *self.peek() != Tok::FlowEntry {
                    return Err(Fail::Bad);
                }
                self.i += 1;
                if *self.peek() == Tok::FlowSeqEnd {
                    self.i += 1;
                    return Ok(items);
                }
            }
            first = false;
            if *self.peek() == Tok::Key {
                // `[a: 1]` is a list of one mapping (`fs6`).
                self.i += 1;
                let k = if matches!(self.peek(), Tok::Value | Tok::FlowEntry | Tok::FlowSeqEnd) {
                    empty()
                } else {
                    self.node(false, false)?
                };
                let v = if *self.peek() == Tok::Value {
                    self.i += 1;
                    if matches!(self.peek(), Tok::FlowEntry | Tok::FlowSeqEnd) {
                        empty()
                    } else {
                        self.node(false, false)?
                    }
                } else {
                    empty()
                };
                items.push(Node::Map {
                    pairs: vec![(k, v)],
                    tag: None,
                });
            } else {
                items.push(self.node(false, false)?);
            }
        }
    }

    fn flow_map(&mut self) -> Res<Vec<(Node, Node)>> {
        self.i += 1;
        let mut pairs = Vec::new();
        let mut first = true;
        loop {
            if *self.peek() == Tok::FlowMapEnd {
                self.i += 1;
                return Ok(pairs);
            }
            if !first {
                if *self.peek() != Tok::FlowEntry {
                    return Err(Fail::Bad);
                }
                self.i += 1;
                if *self.peek() == Tok::FlowMapEnd {
                    self.i += 1;
                    return Ok(pairs);
                }
            }
            first = false;
            if *self.peek() == Tok::Key {
                self.i += 1;
                let k = if matches!(self.peek(), Tok::Value | Tok::FlowEntry | Tok::FlowMapEnd) {
                    empty()
                } else {
                    self.node(false, false)?
                };
                let v = if *self.peek() == Tok::Value {
                    self.i += 1;
                    if matches!(self.peek(), Tok::FlowEntry | Tok::FlowMapEnd) {
                        empty()
                    } else {
                        self.node(false, false)?
                    }
                } else {
                    empty()
                };
                pairs.push((k, v));
            } else {
                // `{a, b}`: keys with no value (`fm4`).
                let k = self.node(false, false)?;
                let v = if *self.peek() == Tok::Value {
                    self.i += 1;
                    if matches!(self.peek(), Tok::FlowEntry | Tok::FlowMapEnd) {
                        empty()
                    } else {
                        self.node(false, false)?
                    }
                } else {
                    empty()
                };
                pairs.push((k, v));
            }
        }
    }
}

const CORE: &str = "tag:yaml.org,2002:";

/// A tagged scalar (`tg1`-`tg24`). A tag no cell resolves is an error (`tg8`, `tg16`, `tg17`) or, for the core forms
/// whose text no cell measures, a refusal.
fn tagged_scalar(v: &str, tag: &str) -> Res<Yaml> {
    let Some(kind) = tag.strip_prefix(CORE) else {
        return Err(Fail::Bad);
    };
    // An empty value under `!!str` or `!!map` is null (`tg12`).
    match kind {
        "str" => Ok(if v.is_empty() {
            Yaml::Null
        } else {
            Yaml::Text(v.to_string())
        }),
        "int" => match v.parse::<i64>() {
            Ok(n) if v.chars().all(|c| c.is_ascii_digit() || c == '-') => Ok(Yaml::Raw(n.to_string())),
            _ if v.chars().all(|c| c.is_ascii_alphabetic()) && !v.is_empty() => Err(Fail::Bad),
            _ => Err(Fail::Refuse(
                "ReadFromYaml of a !!int text other than a whole number (not measured)",
            )),
        },
        "float" => match v.parse::<f64>() {
            Ok(n) if v.chars().all(|c| c.is_ascii_digit() || c == '.' || c == '-') => Ok(Yaml::Raw(format!("{n:?}"))),
            _ => Err(Fail::Refuse(
                "ReadFromYaml of a !!float text other than a plain decimal (not measured)",
            )),
        },
        "bool" => match v {
            "yes" | "true" => Ok(Yaml::Raw("true".into())),
            "no" | "false" => Ok(Yaml::Raw("false".into())),
            _ => Err(Fail::Refuse(
                "ReadFromYaml of a !!bool text other than yes, no, true and false (not measured)",
            )),
        },
        "timestamp" => date(v).map(|d| Yaml::Text(format!("{d}T00:00:00"))),
        "null" | "seq" | "binary" | "set" | "omap" | "pairs" => Err(Fail::Bad),
        _ => Err(Fail::Bad),
    }
}

/// `2001-12-14` (`tg15`); other timestamp forms are not measured.
fn date(v: &str) -> Res<&str> {
    let b = v.as_bytes();
    let ok = b.len() == 10
        && b.iter().enumerate().all(|(i, c)| {
            if i == 4 || i == 7 {
                *c == b'-'
            } else {
                c.is_ascii_digit()
            }
        });
    if ok {
        Ok(v)
    } else {
        Err(Fail::Refuse(
            "ReadFromYaml of a !!timestamp other than a date (not measured)",
        ))
    }
}

fn key_text(k: &Node) -> Res<String> {
    match k {
        Node::Scalar { v, style, tag } => {
            if v.is_empty() && *style == Style::Plain && tag.is_none() {
                return Err(Fail::Bad);
            }
            Ok(v.clone())
        }
        // A key that is a list or a mapping (`ck2`, `ck5`, `ck6`, `ck10`, `ck22`).
        _ => Ok("(Collection)".into()),
    }
}

fn convert(n: &Node) -> Res<Yaml> {
    match n {
        Node::Scalar { v, style, tag: Some(t) } => {
            if v.is_empty() && *style == Style::Plain && t.as_str() == format!("{CORE}str") {
                return Ok(Yaml::Null);
            }
            tagged_scalar(v, t)
        }
        Node::Scalar { v, style, tag: None } => {
            let null = *style == Style::Plain && matches!(v.as_str(), "" | "~" | "null" | "Null" | "NULL");
            Ok(if null { Yaml::Null } else { Yaml::Text(v.clone()) })
        }
        Node::Seq { items, tag } => {
            match tag.as_deref() {
                None => {}
                Some("!") => {}
                Some(t) if t.starts_with(CORE) && t != format!("{CORE}map") && t != format!("{CORE}str") => {
                    return Err(Fail::Bad);
                }
                Some(t) if t.starts_with(CORE) => {
                    return Err(Fail::Refuse(
                        "ReadFromYaml of a list tagged !!map or !!str (not measured)",
                    ));
                }
                Some(_) => return Err(Fail::Bad),
            }
            Ok(Yaml::Seq(items.iter().map(convert).collect::<Res<_>>()?))
        }
        Node::Map { pairs, tag } => {
            match tag.as_deref() {
                None => {}
                Some(t) if t == format!("{CORE}map") => {}
                Some(t) if t.starts_with(CORE) && t != format!("{CORE}str") => return Err(Fail::Bad),
                Some(t) if t.starts_with(CORE) => {
                    return Err(Fail::Refuse("ReadFromYaml of a mapping tagged !!str (not measured)"));
                }
                Some(_) => return Err(Fail::Bad),
            }
            let mut members: Vec<(String, Yaml)> = Vec::new();
            for (k, v) in pairs {
                let k = key_text(k)?;
                let v = convert(v)?;
                // The last of equal keys wins and keeps the place of the first (`dk1`-`dk14`).
                match members.iter_mut().find(|(m, _)| *m == k) {
                    Some(slot) => slot.1 = v,
                    None => members.push((k, v)),
                }
            }
            Ok(Yaml::Map(members))
        }
    }
}
