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
mod tags;
use tags::{Kind, Version};

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
#[derive(Debug, PartialEq)]
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

/// The characters corpus PR 1549 measured beyond `measured_char` (JsonYamlFinalChars.al :45-275, every one `ok` in all
/// twelve forms but U+0085, U+2028 and U+2029): Latin-1 and Latin Extended letters, other scripts, typographic
/// punctuation, characters outside the Basic Multilingual Plane, C0 and C1 controls, format characters, private use and
/// noncharacters. U+FDD0 (`:263`) is this runtime's mark of untracked call stack frames (`native::CALL_STACK_MARK`) and
/// refuses.
fn final_char(c: char) -> bool {
    matches!(
        c,
        '\u{1}'
            | '\u{2}'
            | '\u{8}'
            | '\u{b}'
            | '\u{c}'
            | '\u{e}'
            | '\u{1f}'
            | '\u{7f}'
            | '\u{81}'
            | '\u{90}'
            | '\u{9f}'
            | '\u{a1}'
            | '\u{ad}'
            | '\u{b5}'
            | '\u{c0}'
            | '\u{d7}'
            | '\u{df}'
            | '\u{e4}'
            | '\u{f1}'
            | '\u{f6}'
            | '\u{ff}'
            | '\u{100}'
            | '\u{152}'
            | '\u{17f}'
            | '\u{416}'
            | '\u{5d0}'
            | '\u{627}'
            | '\u{e01}'
            | '\u{1e9e}'
            | '\u{200b}'
            | '\u{200d}'
            | '\u{2010}'
            | '\u{2013}'
            | '\u{2014}'
            | '\u{2018}'
            | '\u{2019}'
            | '\u{201c}'
            | '\u{201d}'
            | '\u{2022}'
            | '\u{2026}'
            | '\u{202e}'
            | '\u{2060}'
            | '\u{2122}'
            | '\u{2190}'
            | '\u{2603}'
            | '\u{3042}'
            | '\u{30a2}'
            | '\u{4e2d}'
            | '\u{ac00}'
            | '\u{d7ff}'
            | '\u{e000}'
            | '\u{f8ff}'
            | '\u{fdef}'
            | '\u{ff21}'
            | '\u{fffd}'
            | '\u{fffe}'
            | '\u{ffff}'
            | '\u{10000}'
            | '\u{1f389}'
            | '\u{1f468}'
            | '\u{1f4bb}'
            | '\u{10ffff}'
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
    /// The `%TAG` handles with their prefix and whether a tag of the document used it.
    tags: HashMap<String, (String, bool)>,
    version: Version,
    depth: usize,
    /// The anchored nodes with the nodes they hold and their depth, to bound what the aliases expand to.
    expanded: usize,
}

/// A text read as a YAML tree, and whether a rule of corpus PR 1549 decided it: then the read runs through the
/// assumption `read-from-yaml-final-shapes`.
pub(crate) struct Read {
    pub(crate) doc: Res<Yaml>,
    pub(crate) marked: bool,
}

/// U+0015 is what `§` written on an MS-DOS stream reads as: measured in an InStream only (corpus PR 1549
/// JsonYamlFinalStreams.al:95 `sect-m-*`).
fn stream_char(c: char) -> bool {
    c == '\u{15}'
}

/// The text read as a YAML tree; `stream` when it came from an InStream.
pub(crate) fn document(text: &str, stream: bool) -> Read {
    let chars: Vec<char> = text.chars().filter(|&c| c != '\0').collect();
    if !chars
        .iter()
        .all(|&c| measured_char(c) || final_char(c) || (stream && stream_char(c)))
    {
        let doc = Err(Fail::Refuse("ReadFromYaml of a character that no probe measures"));
        return Read { doc, marked: false };
    }
    let scan = scan::tokens(&chars);
    let marked = scan.marked || chars.iter().any(|&c| !measured_char(c));
    let doc = scan.toks.and_then(|t| tree(&chars, t, scan.unicode_breaks));
    Read { doc, marked }
}

/// The tokens `t` of `chars` as a YAML tree; `unicode_breaks` is the number of U+0085, U+2028 and U+2029 the scanner
/// read where corpus PR 1549 measured them.
fn tree(chars: &[char], t: Vec<Tok>, unicode_breaks: usize) -> Res<Yaml> {
    let mut p = Parser {
        t,
        i: 1,
        anchors: HashMap::new(),
        tags: HashMap::new(),
        version: Version::None,
        depth: 0,
        expanded: 0,
    };
    let root = p.stream()?;
    // A tagged scalar as the document: `!!int`, `!!bool` and `!!str` with a value are measured (corpus PR 1549
    // JsonYamlFinalTags.al `Top1_1`-`Top1_3`, `Top1_8`); they read as a number, a Boolean or a text, which is no object.
    // An empty `!!null` document is the YAML error, as `!!null` is everywhere (`Top1_10`).
    if let Some(Node::Scalar { v, style, tag: Some(t) }) = &root {
        let kind = tags::kind(t);
        let null_word = *style == Style::Plain && tags::plain_null(v);
        let measured =
            (matches!(kind, Kind::Int | Kind::Bool | Kind::Str) && !null_word) || (kind == Kind::Null && v.is_empty());
        if !measured || p.version != Version::None {
            return Err(Fail::Refuse(
                "ReadFromYaml of a document that is a tagged scalar no probe measured",
            ));
        }
    }
    let doc = match root {
        None => Yaml::Null,
        Some(n) => convert(&n, p.version)?,
    };
    if chars.iter().filter(|&&c| unicode_break(c)).count() != unicode_breaks {
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
                Tok::Version(v) => {
                    directives = true;
                    self.version = if v == "1.1" { Version::V11 } else { Version::Other };
                    self.i += 1;
                }
                Tok::TagDirective(h, p) => {
                    directives = true;
                    // One directive for `!`, `!!` or a named handle, with any prefix, its escapes decoded (corpus PR
                    // 1549 JsonYamlFinalStructure.al `Directives1_13`-`Directives1_25`, JsonYamlFinalTags.al
                    // `Escapes1_9`); the same handle twice is the YAML error (`Directives1_21`). Two directives for
                    // different handles, and a handle of another form (`e!`, `foo`), are not measured.
                    if !(h == "!" || h == "!!" || named_handle(&h)) {
                        return Err(Fail::Refuse(
                            "ReadFromYaml of a %TAG directive for a handle that is not !, !! or a named one (not measured)",
                        ));
                    }
                    if self.tags.contains_key(&h) {
                        return Err(Fail::Bad);
                    }
                    if !self.tags.is_empty() {
                        return Err(Fail::Refuse(
                            "ReadFromYaml of %TAG directives for two handles (not measured)",
                        ));
                    }
                    let p = tags::unescape(&p)?;
                    self.tags.insert(h, (p, false));
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
        // A directive no tag uses is measured for a named handle with the core prefix only (`Directives1_22`); every
        // other directive was measured with a tag that uses it (`Directives1_13`-`Directives1_20`, `Directives1_25`,
        // `Escapes1_3`).
        if self
            .tags
            .iter()
            .any(|(h, (p, used))| !(*used || (named_handle(h) && p == tags::CORE)))
        {
            return Err(Fail::Refuse(
                "ReadFromYaml of a %TAG directive that no tag uses, other than a named handle with the core prefix (not measured)",
            ));
        }
        // Only one document is read (`dc2`, `dc3`, `dc15`, `dc18`, `dc22`).
        if *self.peek() != Tok::StreamEnd {
            return Err(Fail::Bad);
        }
        Ok(Some(node))
    }

    /// The tag a handle and suffix name, the suffix's escapes decoded (corpus PR 1549 JsonYamlFinalTags.al
    /// `Escapes1_*`, `Escapes2_*`). A handle a `%TAG` directive names uses its prefix (`Directives1_13`,
    /// `Directives1_17`, `Directives1_18`); an undefined named handle is the YAML error (`Directives1_23`, `Directives1_24`).
    fn resolve(&mut self, handle: &str, suffix: &str) -> Res<String> {
        let suffix = tags::unescape(suffix)?;
        if handle.is_empty() {
            return Ok(suffix);
        }
        if let Some((p, used)) = self.tags.get_mut(handle) {
            *used = true;
            return Ok(format!("{p}{suffix}"));
        }
        match handle {
            "!" => Ok(format!("!{suffix}")),
            "!!" => Ok(format!("{}{suffix}", tags::CORE)),
            _ => Err(Fail::Bad),
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

/// A named tag handle: `!`, word characters, `!` (corpus PR 1549 JsonYamlFinalStructure.al `Directives1_14`, `!e!`).
fn named_handle(h: &str) -> bool {
    h.len() > 2 && h.starts_with('!') && h.ends_with('!') && h[1..h.len() - 1].chars().all(scan::word)
}

/// The nesting a text may have (the parser refuses deeper ones).
const MAX_DEPTH: usize = 600;
/// The nodes the aliases of a text may expand to in all, above the measured sizes (PR 1530 `keys2000`: 4000 nodes; corpus
/// PR 1549 JsonYamlFinalStructure.al:355 `lol5`, 9 aliases per level: 74718 nodes).
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

/// A tagged scalar as BC hands it on to JSON (corpus PR 1549 JsonYamlFinalTags.al, `tags::scalar`).
fn tagged_scalar(v: &str, style: Style, tag: &str, version: Version) -> Res<Yaml> {
    Ok(match tags::scalar(tags::kind(tag), v, style, version)? {
        tags::Typed::Null => Yaml::Null,
        tags::Typed::Text(t) => Yaml::Text(t),
        tags::Typed::Int(i) => Yaml::Raw(i.to_string()),
        tags::Typed::Bool(b) => Yaml::Raw(b.to_string()),
        tags::Typed::Float(x) => tags::float_json(x)?,
    })
}

/// A tag on a list or a mapping: only `!!map` on a mapping reads (corpus PR 1549 JsonYamlFinalTags.al `WrongNode1_17`,
/// `WrongNode2_4`, `WrongNode2_17`, `Top1_4`-`Top1_6`); every other tag a cell puts on a collection is the YAML error
/// (`WrongNode1_1`-`WrongNode1_9`, `WrongNode1_15`-`WrongNode1_20`, `WrongNode2_*`, `Position1_8`-`Position1_10`,
/// `Position1_16`, `Position1_18`, `Top1_7`, `Top1_9`, `Top1_12`-`Top1_15`, `Top2_1`). A core tag of another name and a
/// global tag outside the core ones are not measured on a collection.
fn collection_tag(tag: Option<&str>, map: bool) -> Res<()> {
    match tag.map(tags::kind) {
        None => Ok(()),
        Some(Kind::Map) if map => Ok(()),
        Some(Kind::UnknownCore | Kind::Other) => Err(Fail::Refuse(
            "ReadFromYaml of a collection under a tag no probe measured",
        )),
        Some(_) => Err(Fail::Bad),
    }
}

/// A key: an untagged scalar, a tagged scalar in its .NET text form (`tags::key`), or `(Collection)` for a list or a
/// mapping (corpus PR 1530 `ck2`, `ck5`, `ck6`, `ck10`, `ck22`; PR 1549 JsonYamlFinalTags.al `Keys1_11`); a tagged list
/// key is the YAML error (`Keys1_10`).
fn key_text(k: &Node, version: Version) -> Res<String> {
    match k {
        Node::Scalar { v, style, tag: None } => {
            if v.is_empty() && *style == Style::Plain {
                return Err(Fail::Bad);
            }
            Ok(v.clone())
        }
        Node::Scalar { v, style, tag: Some(t) } => tags::key(tags::kind(t), v, *style, version),
        Node::Seq { tag, .. } => collection_tag(tag.as_deref(), false).map(|()| "(Collection)".into()),
        Node::Map { tag, .. } => collection_tag(tag.as_deref(), true).map(|()| "(Collection)".into()),
    }
}

fn convert(n: &Node, version: Version) -> Res<Yaml> {
    match n {
        Node::Scalar { v, style, tag: Some(t) } => tagged_scalar(v, *style, t, version),
        Node::Scalar { v, style, tag: None } => {
            let null = *style == Style::Plain && tags::plain_null(v);
            Ok(if null { Yaml::Null } else { Yaml::Text(v.clone()) })
        }
        Node::Seq { items, tag } => {
            collection_tag(tag.as_deref(), false)?;
            Ok(Yaml::Seq(
                items.iter().map(|i| convert(i, version)).collect::<Res<_>>()?,
            ))
        }
        Node::Map { pairs, tag } => {
            collection_tag(tag.as_deref(), true)?;
            let mut members: Vec<(String, Yaml)> = Vec::new();
            let mut index: HashMap<String, usize> = HashMap::new();
            for (k, v) in pairs {
                let k = key_text(k, version)?;
                let v = convert(v, version)?;
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
