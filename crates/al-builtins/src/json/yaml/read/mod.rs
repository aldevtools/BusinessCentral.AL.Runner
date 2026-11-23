//! The YAML reader of `ReadFromYaml`: a libyaml-style scanner (`scan`), a parser and the conversion to the tree BC hands
//! on to JSON. Every rule here is measured by the cells of the corpus probes of al-runner issue 8580, identical on every
//! BC major the corpus runs, read through the assumption `read-from-yaml-probed-shapes` until they are pinned:
//! - PR 1493 `tests/al-language/json/JsonYamlBlankLines.al` (codeunit 75620),
//! - PR 1515 `tests/al-language/json/JsonYamlUtf8.al` (codeunit 75621),
//! - PR 1519 `tests/al-language/json/JsonYamlPunctMatrix.al` (codeunit 75623),
//! - PR 1530 `tests/al-language/json/JsonYamlRemaining.al` (codeunit 79892).
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
    anchors: HashMap<String, (Node, usize, usize)>,
    tags: HashMap<String, String>,
    tag_used: std::cell::Cell<bool>,
    depth: usize,
    /// The anchored nodes with the nodes they hold and their depth, to bound what the aliases expand to.
    expanded: usize,
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
        tag_used: std::cell::Cell::new(false),
        depth: 0,
        expanded: 0,
    };
    let root = p.stream()?;
    if !p.tags.is_empty() && !p.tag_used.get() {
        return Err(Fail::Refuse(
            "ReadFromYaml of a %TAG directive that no tag uses (not measured)",
        ));
    }
    if let Some(Node::Scalar { tag: Some(_), .. }) = &root {
        return Err(Fail::Refuse(
            "ReadFromYaml of a document that is a tagged scalar (not measured)",
        ));
    }
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
                    // Measured: one named handle with a prefix that is not the core one (`tg17`). The handles `!`
                    // and `!!`, a second directive and a prefix of the core tags are not measured.
                    let named = h.len() > 2 && h.starts_with('!') && h.ends_with('!') && h != "!!";
                    if !named || !self.tags.is_empty() || !p.starts_with("tag:") || p.starts_with("tag:yaml.org,2002") {
                        return Err(Fail::Refuse(
                            "ReadFromYaml of a %TAG directive other than a named handle for a prefix that is not the core one (not measured)",
                        ));
                    }
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
            h => {
                self.tag_used.set(true);
                let tag = self.tags.get(h).map(|p| format!("{p}{suffix}")).ok_or(Fail::Bad)?;
                // A named handle is measured only for a tag no cell resolves (`tg17`). A prefix and suffix that
                // together give a core tag (`tag:yaml.org,200` + `2:int`) are not measured.
                if tag.starts_with("tag:yaml.org,2002") {
                    return Err(Fail::Refuse(
                        "ReadFromYaml of a %TAG handle that gives a core tag (not measured)",
                    ));
                }
                Ok(tag)
            }
        }
    }

    fn node(&mut self, block: bool, indentless: bool) -> Res<Node> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
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
            let Some((node, size, depth)) = self.anchors.get(&name) else {
                return Err(Fail::Bad);
            };
            self.expanded += size;
            if self.expanded > MAX_EXPANDED || self.depth + depth > MAX_DEPTH {
                return Err(Fail::Refuse(
                    "ReadFromYaml of aliases that expand to a larger tree than the probes measured",
                ));
            }
            return Ok(node.clone());
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
            let (size, depth) = measure(&node);
            self.anchors.insert(a, (node.clone(), size, depth));
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

/// The nesting a text may have (the parser refuses deeper ones) and the nodes its aliases may expand to in all, far
/// above the measured sizes (`keys2000`: 4000 nodes).
const MAX_DEPTH: usize = 600;
const MAX_EXPANDED: usize = 200_000;

/// The nodes in a node and its depth.
fn measure(n: &Node) -> (usize, usize) {
    match n {
        Node::Scalar { .. } => (1, 1),
        Node::Seq { items, .. } => items.iter().fold((1, 1), |(s, d), i| {
            let (is, id) = measure(i);
            (s + is, d.max(id + 1))
        }),
        Node::Map { pairs, .. } => pairs.iter().fold((1, 1), |(s, d), (k, v)| {
            let (ks, kd) = measure(k);
            let (vs, vd) = measure(v);
            (s + ks + vs, d.max(kd + 1).max(vd + 1))
        }),
    }
}

/// A tagged scalar, by equality with the cells that measure it (`tg1`-`tg24`, `nl9` of corpus PR 1530); every other
/// text under a core tag refuses.
fn tagged_scalar(v: &str, style: Style, tag: &str) -> Res<Yaml> {
    let Some(kind) = tag.strip_prefix(CORE) else {
        // `!` on an empty value (`tg14`) and a tag no cell resolves (`tg8`, `tg16`, `tg17`) are errors.
        return if tag == "!" && !(v.is_empty() && style == Style::Plain) {
            Err(Fail::Refuse("ReadFromYaml of a scalar under the tag ! (not measured)"))
        } else {
            Err(Fail::Bad)
        };
    };
    match (kind, v, style) {
        // `tg1`, `tg9`, `tg18`, `tg21`, `tg23`, `tg24`; an empty plain value is null (`tg12`, handled by `convert`).
        ("str", v, _) if !v.is_empty() => Ok(Yaml::Text(v.to_string())),
        ("int", "1", Style::Double) => Ok(Yaml::Raw("1".into())),
        ("float", "1", Style::Plain) => Ok(Yaml::Raw("1.0".into())),
        ("bool", "yes", Style::Plain) => Ok(Yaml::Raw("true".into())),
        ("timestamp", "2001-12-14", Style::Plain) => Ok(Yaml::Text("2001-12-14T00:00:00".into())),
        // `tg22`, `tg5`, `nl9`, `tg10`.
        ("int", "abc", Style::Plain)
        | ("null", "", Style::Double | Style::Plain)
        | ("binary", "aGk=", Style::Plain) => Err(Fail::Bad),
        _ => Err(Fail::Refuse("ReadFromYaml of a core tag with a text no probe measured")),
    }
}

/// A key: an untagged scalar or collection, or a `!!str` scalar with a text (`ck28`, `tg13`).
fn key_text(k: &Node) -> Res<String> {
    match k {
        Node::Scalar { v, style, tag } => {
            match tag.as_deref() {
                None => {
                    if v.is_empty() && *style == Style::Plain {
                        return Err(Fail::Bad);
                    }
                }
                Some(t) if t == format!("{CORE}str") && !v.is_empty() => {}
                Some(_) => {
                    return Err(Fail::Refuse(
                        "ReadFromYaml of a key under a tag other than !!str (not measured)",
                    ));
                }
            }
            Ok(v.clone())
        }
        // A key that is a list or a mapping (`ck2`, `ck5`, `ck6`, `ck10`, `ck22`).
        Node::Seq { tag: None, .. } | Node::Map { tag: None, .. } => Ok("(Collection)".into()),
        _ => Err(Fail::Refuse(
            "ReadFromYaml of a collection key under a tag (not measured)",
        )),
    }
}

fn convert(n: &Node) -> Res<Yaml> {
    match n {
        Node::Scalar { v, style, tag: Some(t) } => {
            if v.is_empty() && *style == Style::Plain && t.as_str() == format!("{CORE}str") {
                return Ok(Yaml::Null);
            }
            tagged_scalar(v, *style, t)
        }
        Node::Scalar { v, style, tag: None } => {
            let null = *style == Style::Plain && matches!(v.as_str(), "" | "~" | "null" | "Null" | "NULL");
            Ok(if null { Yaml::Null } else { Yaml::Text(v.clone()) })
        }
        Node::Seq { items, tag } => {
            match tag.as_deref() {
                None => {}
                // `tg6`, `tg20`.
                Some(t) if t == format!("{CORE}seq") => return Err(Fail::Bad),
                Some(_) => return Err(Fail::Refuse("ReadFromYaml of a list under a tag (not measured)")),
            }
            Ok(Yaml::Seq(items.iter().map(convert).collect::<Res<_>>()?))
        }
        Node::Map { pairs, tag } => {
            match tag.as_deref() {
                // `tg7`, `tg19`, `dc17`.
                None => {}
                Some(t) if t == format!("{CORE}map") => {}
                // `tg11`.
                Some(t) if t == format!("{CORE}set") => return Err(Fail::Bad),
                Some(_) => return Err(Fail::Refuse("ReadFromYaml of a mapping under a tag (not measured)")),
            }
            let mut members: Vec<(String, Yaml)> = Vec::new();
            let mut index: HashMap<String, usize> = HashMap::new();
            for (k, v) in pairs {
                let k = key_text(k)?;
                let v = convert(v)?;
                // The last of equal keys wins and keeps the place of the first (`dk1`-`dk14`).
                match index.get(&k) {
                    Some(&at) => members[at].1 = v,
                    None => {
                        index.insert(k.clone(), members.len());
                        members.push((k, v));
                    }
                }
            }
            Ok(Yaml::Map(members))
        }
    }
}
