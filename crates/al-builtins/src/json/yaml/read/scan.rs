//! The tokens of a YAML text, cut the way a libyaml-style scanner cuts them. Every behavior here is measured by the cells
//! of corpus PR 1530 (`tests/al-language/json/JsonYamlRemaining.al`, codeunit 79892), named after the group each cell
//! belongs to; an error is `YamlBadFormat` (`Fail::Bad`), a shape no cell measures is `Fail::Refuse`.

use super::Fail;

#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum Style {
    Plain,
    Single,
    Double,
    Block,
}

#[derive(Clone, PartialEq, Debug)]
pub(super) enum Tok {
    StreamStart,
    StreamEnd,
    Version(String),
    TagDirective(String, String),
    DocStart,
    DocEnd,
    BlockSeqStart,
    BlockMapStart,
    BlockEnd,
    FlowSeqStart,
    FlowSeqEnd,
    FlowMapStart,
    FlowMapEnd,
    BlockEntry,
    FlowEntry,
    Key,
    Value,
    Alias(String),
    Anchor(String),
    Tag(String, String),
    Scalar(String, Style),
}

#[derive(Clone, Copy)]
struct SimpleKey {
    required: bool,
    tok: usize,
    line: usize,
    pos: usize,
}

struct Scanner {
    s: Vec<char>,
    i: usize,
    col: usize,
    line: usize,
    toks: Vec<Tok>,
    indent: i64,
    indents: Vec<i64>,
    allowed: bool,
    flow: usize,
    keys: Vec<Option<SimpleKey>>,
    yaml_directives: usize,
    /// The open flow forms, `[` or `{`, innermost last.
    flows: Vec<char>,
    /// Whether a rule of corpus PR 1549 decided the text.
    marked: bool,
    /// The U+0085, U+2028 and U+2029 read where corpus PR 1549 measured them.
    unicode_breaks: usize,
}

/// The tokens of a text, whether a rule of corpus PR 1549 decided them, and the U+0085, U+2028 and U+2029 read where
/// PR 1549 measured them (`unicode_break_in_quotes`, `comment`).
pub(super) struct Scan {
    pub(super) toks: Res<Vec<Tok>>,
    pub(super) marked: bool,
    pub(super) unicode_breaks: usize,
}

/// The UTF-16 units an implicit key may span up to its `:` (corpus PR 1549 JsonYamlFinalStructure.al
/// `KeyLength_Plain`, `KeyLength_Forms`, `KeyLength_Layout`: 1024 read, 1025 is the YAML error, counted in UTF-16 units,
/// `emoji-key-512`, `emoji-key-513`).
const KEY_UNITS: usize = 1024;

type Res<T> = Result<T, Fail>;

/// All tokens of `text` (breaks are LF, CRLF or a lone CR).
pub(super) fn tokens(text: &[char]) -> Scan {
    let mut sc = Scanner {
        s: text.to_vec(),
        i: 0,
        col: 0,
        line: 0,
        toks: vec![Tok::StreamStart],
        indent: -1,
        indents: Vec::new(),
        allowed: true,
        flow: 0,
        keys: vec![None],
        yaml_directives: 0,
        flows: Vec::new(),
        marked: false,
        unicode_breaks: 0,
    };
    let toks = loop {
        if let Err(e) = sc.next_token() {
            break Err(e);
        }
        if sc.toks.last() == Some(&Tok::StreamEnd) {
            break Ok(std::mem::take(&mut sc.toks));
        }
    };
    Scan {
        toks,
        marked: sc.marked,
        unicode_breaks: sc.unicode_breaks,
    }
}

/// The character a one-letter escape of a double-quoted text stands for (the cells `top[..]` and `list[..]`).
fn named_escape(c: char) -> Option<char> {
    Some(match c {
        '0' => '\0',
        'a' => '\u{7}',
        'b' => '\u{8}',
        't' | '\t' => '\t',
        'n' => '\n',
        'v' => '\u{b}',
        'f' => '\u{c}',
        'r' => '\r',
        'e' => '\u{1b}',
        ' ' => ' ',
        '"' => '"',
        '/' => '/',
        '\\' => '\\',
        'N' => '\u{85}',
        '_' => '\u{a0}',
        'L' => '\u{2028}',
        'P' => '\u{2029}',
        _ => return None,
    })
}

/// LF, CR, and the line breaks of `super::unicode_break`.
fn is_break(c: char) -> bool {
    c == '\n' || c == '\r' || super::unicode_break(c)
}

/// The UTF-16 units of `s`; only counted past `KEY_UNITS / 2` characters.
fn units(s: &[char]) -> usize {
    if s.len() <= KEY_UNITS / 2 {
        s.len()
    } else {
        s.iter().map(|c| c.len_utf16()).sum()
    }
}

fn is_blank(c: char) -> bool {
    c == ' ' || c == '\t'
}

pub(super) fn word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_'
}

/// The characters an anchor or alias name was measured with (corpus PR 1549 JsonYamlFinalStructure.al
/// `AnchorNames1_*`, `AnchorNames2_*`): letters, digits and the listed punctuation, U+00E4, U+00E9, U+00DF, U+4E2D and
/// U+1F600. A `:` at the end of a name (`*a: b`) and `,`, `[`, `]`, `{`, `}`, `` ` ``, `\` are not measured.
fn anchor_char(c: char) -> bool {
    word(c) || ".-/:#'\"%@!$*&=+~|>?^();<".contains(c) || matches!(c, 'ä' | 'é' | 'ß' | '中' | '\u{1F600}')
}

impl Scanner {
    fn at(&self, k: usize) -> char {
        self.s.get(self.i + k).copied().unwrap_or('\0')
    }

    fn eof(&self) -> bool {
        self.i >= self.s.len()
    }

    fn blankz(&self, k: usize) -> bool {
        let c = self.at(k);
        is_blank(c) || is_break(c) || self.i + k >= self.s.len()
    }

    fn breakz(&self) -> bool {
        is_break(self.at(0)) || self.eof()
    }

    fn adv(&mut self) -> char {
        let c = self.at(0);
        self.i += 1;
        self.col += 1;
        c
    }

    /// Skips one break and answers `\n`.
    fn read_break(&mut self) -> Option<char> {
        if self.at(0) == '\r' && self.at(1) == '\n' {
            self.i += 2;
        } else if is_break(self.at(0)) {
            self.i += 1;
        } else {
            return None;
        }
        self.col = 0;
        self.line += 1;
        Some('\n')
    }

    fn doc_indicator(&self, c: char) -> bool {
        self.col == 0 && self.at(0) == c && self.at(1) == c && self.at(2) == c && self.blankz(3)
    }

    fn next_token(&mut self) -> Res<()> {
        self.skip_to_next_token()?;
        self.stale_keys()?;
        self.unroll(self.col as i64);
        if self.eof() {
            self.unroll(-1);
            self.remove_key()?;
            self.allowed = false;
            self.toks.push(Tok::StreamEnd);
            return Ok(());
        }
        let c = self.at(0);
        if self.col == 0 && c == '%' {
            return self.directive();
        }
        if self.doc_indicator('-') || self.doc_indicator('.') {
            self.unroll(-1);
            self.remove_key()?;
            self.allowed = false;
            let start = c == '-';
            self.i += 3;
            self.col += 3;
            self.toks.push(if start { Tok::DocStart } else { Tok::DocEnd });
            return Ok(());
        }
        match c {
            '[' | '{' => self.flow_start(c),
            ']' | '}' => self.flow_end(c),
            ',' => {
                self.remove_key()?;
                self.allowed = true;
                self.adv();
                self.toks.push(Tok::FlowEntry);
                Ok(())
            }
            '-' if self.blankz(1) => self.block_entry(),
            '?' if self.flow > 0 || self.blankz(1) => self.key(),
            ':' if self.flow > 0 || self.blankz(1) => self.value(),
            '*' | '&' => self.anchor(c == '*'),
            '!' => self.tag(),
            '|' | '>' if self.flow == 0 => self.block_scalar(c == '>'),
            '\'' | '"' => self.flow_scalar(c),
            _ => {
                let starts = !(self.blankz(0) || "-?:,[]{}#&*!|>'\"%@`".contains(c))
                    || (c == '-' && !is_blank(self.at(1)))
                    || (self.flow == 0 && (c == '?' || c == ':') && !self.blankz(1));
                if !starts {
                    return Err(Fail::Bad);
                }
                self.plain_scalar()
            }
        }
    }

    fn skip_to_next_token(&mut self) -> Res<()> {
        loop {
            let c = self.at(0);
            if c == ' ' {
                self.adv();
            } else if c == '\t' {
                self.indent_tab()?;
                self.adv();
            } else if c == '#' {
                self.comment();
            } else if is_break(c) {
                self.read_break();
                if self.flow == 0 {
                    self.allowed = true;
                }
            } else {
                return Ok(());
            }
        }
    }

    /// A tab between tokens. Inside a line, and in a flow form also in the indentation, it is white space (corpus PR 1530
    /// BomTabsSpaces cm26; PR 1549 JsonYamlFinalStructure.al `Tabs1_7`-`Tabs1_10`, `Tabs2_11`-`Tabs2_15`, `Tabs2_19`).
    /// In the indentation of a block line it is white space before the first node of the document (`Tabs1_1`-`Tabs1_4`,
    /// `Tabs1_19`, `Tabs1_20`, `Tabs2_1`, `Tabs2_2`, `Tabs2_5`, `Tabs2_16`-`Tabs2_18`) and after spaces on the line after
    /// a `:` before a plain key (`Tabs1_5`); at column 0 after a node it is the YAML error (PR 1530 bt8, bt11). Other
    /// places are not measured.
    fn indent_tab(&mut self) -> Res<()> {
        let only_spaces = self.s[self.i - self.col..self.i].iter().all(|&p| p == ' ');
        let first_line_start = self.line == 0 && self.col == 0;
        if !only_spaces || first_line_start {
            return Ok(());
        }
        let first_node = self.toks.iter().all(|t| {
            matches!(
                t,
                Tok::StreamStart | Tok::Version(_) | Tok::TagDirective(..) | Tok::DocStart
            )
        });
        let after_value = self.col > 0 && self.toks.last() == Some(&Tok::Value) && self.plain_key_follows();
        if self.flow > 0 || first_node || after_value {
            self.marked = true;
            return Ok(());
        }
        if self.col == 0 {
            return Err(Fail::Bad);
        }
        Err(Fail::Refuse(
            "ReadFromYaml of a tab in the indentation of a block line at a place no probe measured",
        ))
    }

    /// Whether the blanks at the read position are followed by a plain key of word characters and `: ` (the measured
    /// line after a `:`, `Tabs1_5`: `o:~  {T}q: a`).
    fn plain_key_follows(&self) -> bool {
        let mut k = 0;
        while is_blank(self.at(k)) {
            k += 1;
        }
        let start = k;
        while word(self.at(k)) {
            k += 1;
        }
        k > start && self.at(k) == ':' && self.blankz(k + 1)
    }

    fn stale_keys(&mut self) -> Res<()> {
        let colon_next = self.at(0) == ':';
        for k in self.keys.iter_mut() {
            if let Some(sk) = *k {
                // In block context a key longer than `KEY_UNITS` goes stale: a required one is the YAML error, a `:`
                // after another one finds no key (`plain-1025`, `nested-1025`, `listitem-1025`, `two-keys-1025`,
                // `spaces-before-colon-1100`). A key in a flow mapping has no limit (`flow-1025`, `flow-3000`).
                let long = sk.line == self.line && self.flow == 0 && units(&self.s[sk.pos..self.i]) > KEY_UNITS;
                if long {
                    self.marked = true;
                    // The long keys measured before their `:` start with a letter, a non-ASCII character or a quote
                    // (`plain-*`, `single-*`, `double-*`, `emoji-key-*`, `auml-key-*`); a key that starts with any other
                    // character (a digit, `-`, a flow collection, an anchor, a tag, an alias) is not measured that
                    // long. A long flow mapping with no `:` after it reads (`flow-1025`).
                    let c = self.s[sk.pos];
                    let measured = c.is_ascii_alphabetic() || u32::from(c) > 0x7F || matches!(c, '\'' | '"');
                    if colon_next && !measured {
                        return Err(Fail::Refuse(
                            "ReadFromYaml of a key longer than 1024 UTF-16 units that starts with a character other than a letter, a non-ASCII character or a quote (not measured)",
                        ));
                    }
                }
                if sk.line < self.line || long {
                    if sk.required {
                        return Err(Fail::Bad);
                    }
                    *k = None;
                }
            }
        }
        Ok(())
    }

    fn save_key(&mut self) -> Res<()> {
        let required = self.flow == 0 && self.indent == self.col as i64;
        if self.allowed {
            self.remove_key()?;
            *self.keys.last_mut().expect("level") = Some(SimpleKey {
                required,
                tok: self.toks.len(),
                line: self.line,
                pos: self.i,
            });
        }
        Ok(())
    }

    fn remove_key(&mut self) -> Res<()> {
        let k = self.keys.last_mut().expect("level");
        if let Some(sk) = *k {
            if sk.required {
                return Err(Fail::Bad);
            }
        }
        *k = None;
        Ok(())
    }

    fn roll(&mut self, col: usize, at: Option<usize>, t: Tok) {
        if self.flow > 0 || self.indent >= col as i64 {
            return;
        }
        self.indents.push(self.indent);
        self.indent = col as i64;
        match at {
            Some(n) => self.toks.insert(n, t),
            None => self.toks.push(t),
        }
    }

    fn unroll(&mut self, col: i64) {
        if self.flow > 0 {
            return;
        }
        while self.indent > col {
            self.toks.push(Tok::BlockEnd);
            self.indent = self.indents.pop().unwrap_or(-1);
        }
    }

    fn flow_start(&mut self, c: char) -> Res<()> {
        self.save_key()?;
        self.flow += 1;
        self.flows.push(c);
        self.keys.push(None);
        self.allowed = true;
        self.adv();
        self.toks
            .push(if c == '[' { Tok::FlowSeqStart } else { Tok::FlowMapStart });
        Ok(())
    }

    fn flow_end(&mut self, c: char) -> Res<()> {
        self.remove_key()?;
        if self.flow > 0 {
            self.flow -= 1;
            self.flows.pop();
            self.keys.pop();
        }
        self.allowed = false;
        self.adv();
        self.toks.push(if c == ']' { Tok::FlowSeqEnd } else { Tok::FlowMapEnd });
        Ok(())
    }

    fn block_entry(&mut self) -> Res<()> {
        if self.flow == 0 {
            if !self.allowed {
                return Err(Fail::Bad);
            }
            self.roll(self.col, None, Tok::BlockSeqStart);
        }
        self.remove_key()?;
        self.allowed = true;
        self.adv();
        self.toks.push(Tok::BlockEntry);
        Ok(())
    }

    fn key(&mut self) -> Res<()> {
        if self.flow == 0 {
            if !self.allowed {
                return Err(Fail::Bad);
            }
            self.roll(self.col, None, Tok::BlockMapStart);
        }
        self.remove_key()?;
        self.allowed = self.flow == 0;
        self.adv();
        self.toks.push(Tok::Key);
        Ok(())
    }

    fn value(&mut self) -> Res<()> {
        let sk = self.keys.last().copied().flatten();
        if let Some(sk) = sk {
            // A long key in a flow mapping reads (`flow-1025`); in a flow sequence (`[k..k: 1]`) it is not measured.
            let long = units(&self.s[sk.pos..self.i]) > KEY_UNITS;
            self.marked |= long;
            if self.flows.last() == Some(&'[') && long {
                return Err(Fail::Refuse(
                    "ReadFromYaml of a key longer than 1024 UTF-16 units in a flow sequence (not measured)",
                ));
            }
            self.toks.insert(sk.tok, Tok::Key);
            let col = self.i_col_of(sk);
            self.roll(col, Some(sk.tok), Tok::BlockMapStart);
            *self.keys.last_mut().expect("level") = None;
            self.allowed = false;
        } else {
            if self.flow == 0 {
                if !self.allowed {
                    return Err(Fail::Bad);
                }
                self.roll(self.col, None, Tok::BlockMapStart);
            }
            self.allowed = self.flow == 0;
        }
        self.adv();
        self.toks.push(Tok::Value);
        Ok(())
    }

    /// The column where the saved simple key started: its line is the current line.
    fn i_col_of(&self, sk: SimpleKey) -> usize {
        let line_start = self.i - self.col;
        sk.pos.saturating_sub(line_start)
    }

    fn anchor(&mut self, alias: bool) -> Res<()> {
        self.save_key()?;
        self.allowed = false;
        self.adv();
        // A name runs up to the next blank, line break or end, in a flow form also up to `,`, `]` or `}` (corpus PR 1549
        // JsonYamlFinalStructure.al `Aliases1_11`, `Aliases1_18`, `Aliases1_19`). An empty name is the YAML error
        // (`Aliases1_24`, `Aliases1_25`, `Aliases2_1`, `Aliases2_2`).
        let mut name = String::new();
        while !self.blankz(0) && !(self.flow > 0 && ",]}".contains(self.at(0))) {
            name.push(self.adv());
        }
        if name.is_empty() || !name.chars().all(word) {
            self.marked = true;
        }
        if name.is_empty() {
            return Err(Fail::Bad);
        }
        if !name.chars().all(anchor_char) || name.ends_with(':') {
            return Err(Fail::Refuse(
                "ReadFromYaml of an anchor or alias name with a character no probe measured",
            ));
        }
        self.toks.push(if alias { Tok::Alias(name) } else { Tok::Anchor(name) });
        Ok(())
    }

    fn uri_char(&self, c: char) -> bool {
        word(c) || ";/?:@&=+$.!~*'()%".contains(c) || (self.flow == 0 && ",[]".contains(c))
    }

    fn tag(&mut self) -> Res<()> {
        // Every tag is read by the rules of corpus PR 1549 (`super::tags`).
        self.marked = true;
        self.save_key()?;
        self.allowed = false;
        self.adv();
        let (handle, suffix);
        if self.at(0) == '<' {
            self.adv();
            let mut uri = String::new();
            while self.uri_char(self.at(0)) {
                uri.push(self.adv());
            }
            if self.at(0) != '>' {
                return Err(Fail::Refuse(
                    "ReadFromYaml of a verbatim tag with other characters (not measured)",
                ));
            }
            self.adv();
            handle = String::new();
            suffix = uri;
        } else {
            let mut h = String::from("!");
            let mut look = 0;
            while word(self.at(look)) {
                look += 1;
            }
            if look > 0 && self.at(look) == '!' {
                for _ in 0..=look {
                    h.push(self.adv());
                }
                let mut uri = String::new();
                while self.uri_char(self.at(0)) {
                    uri.push(self.adv());
                }
                handle = h;
                suffix = uri;
            } else {
                let mut uri = String::new();
                while self.uri_char(self.at(0)) {
                    uri.push(self.adv());
                }
                if uri.is_empty() {
                    handle = String::new();
                    suffix = "!".into();
                } else if let Some(rest) = uri.strip_prefix('!') {
                    handle = "!!".into();
                    suffix = rest.to_string();
                } else {
                    handle = "!".into();
                    suffix = uri;
                }
            }
        }
        let c = self.at(0);
        // A tag right before the end of a flow form is the YAML error (corpus PR 1549 JsonYamlFinalTags.al
        // `Position1_11`, `Position1_12`, `Position1_14`); before a `,` it is not measured.
        if self.flow > 0 && "]}".contains(c) {
            return Err(Fail::Bad);
        }
        if self.flow > 0 && c == ',' {
            return Err(Fail::Refuse(
                "ReadFromYaml of a tag right before `,` in a flow form (not measured)",
            ));
        }
        if !self.blankz(0) {
            return Err(Fail::Refuse(
                "ReadFromYaml of a tag with other characters than a URI (not measured)",
            ));
        }
        self.toks.push(Tok::Tag(handle, suffix));
        Ok(())
    }

    fn directive(&mut self) -> Res<()> {
        self.unroll(-1);
        self.remove_key()?;
        self.allowed = false;
        self.adv();
        let mut name = String::new();
        while word(self.at(0)) {
            name.push(self.adv());
        }
        let blanks = |s: &mut Scanner| {
            while is_blank(s.at(0)) {
                s.adv();
            }
        };
        // `%YAML` and `%TAG` are rules of corpus PR 1549.
        self.marked |= matches!(name.as_str(), "YAML" | "TAG");
        let tok = match name.as_str() {
            "YAML" => {
                blanks(self);
                let mut ver = String::new();
                while !self.blankz(0) {
                    ver.push(self.adv());
                }
                // Versions 1.0 to 1.3 read; a second directive, `2.0`, `1`, `1.10` and none are the YAML error
                // (corpus PR 1549 JsonYamlFinalStructure.al `Directives1_1`-`Directives1_11`, `Directives2_15`). Other
                // versions are not measured.
                self.yaml_directives += 1;
                if self.yaml_directives > 1 || matches!(ver.as_str(), "2.0" | "1" | "1.10" | "") {
                    return Err(Fail::Bad);
                }
                if !matches!(ver.as_str(), "1.0" | "1.1" | "1.2" | "1.3") {
                    return Err(Fail::Refuse("ReadFromYaml of a %YAML version no probe measured"));
                }
                Tok::Version(ver)
            }
            "TAG" => {
                blanks(self);
                let mut handle = String::new();
                while !self.blankz(0) {
                    handle.push(self.adv());
                }
                blanks(self);
                let mut prefix = String::new();
                while !self.blankz(0) {
                    prefix.push(self.adv());
                }
                Tok::TagDirective(handle, prefix)
            }
            _ => {
                while !self.breakz() {
                    self.adv();
                }
                return Ok(());
            }
        };
        while is_blank(self.at(0)) {
            self.adv();
        }
        if self.at(0) == '#' {
            while !self.breakz() {
                self.adv();
            }
        }
        if !self.breakz() {
            return Err(Fail::Bad);
        }
        self.toks.push(tok);
        Ok(())
    }

    /// The lines joined the way plain and quoted scalars fold them.
    fn fold(out: &mut String, lead: &mut String, trail: &mut String) {
        if lead.starts_with('\n') {
            if trail.is_empty() {
                out.push(' ');
            } else {
                out.push_str(trail);
            }
        } else {
            out.push_str(lead);
            out.push_str(trail);
        }
        lead.clear();
        trail.clear();
    }

    fn plain_scalar(&mut self) -> Res<()> {
        self.save_key()?;
        self.allowed = false;
        let indent = self.indent + 1;
        let (mut out, mut ws, mut lead, mut trail) = (String::new(), String::new(), String::new(), String::new());
        let mut leading = false;
        loop {
            if self.doc_indicator('-') || self.doc_indicator('.') || self.at(0) == '#' {
                break;
            }
            while !self.blankz(0) {
                let c = self.at(0);
                if (c == ':' && self.blankz(1)) || (self.flow > 0 && ",?[]{}".contains(c)) {
                    break;
                }
                if leading || !ws.is_empty() {
                    if leading {
                        Self::fold(&mut out, &mut lead, &mut trail);
                        leading = false;
                    } else {
                        out.push_str(&ws);
                        ws.clear();
                    }
                }
                out.push(self.adv());
            }
            if !(is_blank(self.at(0)) || is_break(self.at(0))) {
                break;
            }
            while is_blank(self.at(0)) || is_break(self.at(0)) {
                if is_blank(self.at(0)) {
                    if leading && (self.col as i64) < indent && self.at(0) == '\t' {
                        return Err(Fail::Bad);
                    }
                    let c = self.adv();
                    if !leading {
                        ws.push(c);
                    }
                } else {
                    if self.flow > 0 && super::unicode_break(self.at(0)) {
                        self.unicode_break_in_flow(leading || !ws.is_empty())?;
                    }
                    self.read_break();
                    if leading {
                        trail.push('\n');
                    } else {
                        ws.clear();
                        lead = "\n".into();
                        leading = true;
                    }
                }
            }
            if self.flow == 0 && (self.col as i64) < indent {
                break;
            }
        }
        if leading {
            self.allowed = true;
        }
        self.toks.push(Tok::Scalar(out, Style::Plain));
        Ok(())
    }

    fn flow_scalar(&mut self, q: char) -> Res<()> {
        self.save_key()?;
        self.allowed = false;
        self.adv();
        let single = q == '\'';
        let (mut out, mut ws, mut lead, mut trail) = (String::new(), String::new(), String::new(), String::new());
        loop {
            if self.doc_indicator('-') || self.doc_indicator('.') || self.eof() {
                return Err(Fail::Bad);
            }
            let mut leading = false;
            while !self.blankz(0) || (single && super::unicode_break(self.at(0))) {
                let c = self.at(0);
                if single && c == '\'' && self.at(1) == '\'' {
                    out.push('\'');
                    self.adv();
                    self.adv();
                } else if c == q {
                    break;
                } else if single && super::unicode_break(c) {
                    self.unicode_break_in_quotes(&mut out)?;
                } else if !single && c == '\\' && is_break(self.at(1)) {
                    self.adv();
                    self.read_break();
                    leading = true;
                    break;
                } else if !single && c == '\\' {
                    self.escape(&mut out)?;
                } else {
                    out.push(self.adv());
                }
            }
            if self.at(0) == q {
                break;
            }
            while is_blank(self.at(0)) || is_break(self.at(0)) {
                if is_blank(self.at(0)) {
                    let c = self.adv();
                    if !leading {
                        ws.push(c);
                    }
                } else if leading {
                    self.read_break();
                    trail.push('\n');
                } else {
                    ws.clear();
                    self.read_break();
                    lead = "\n".into();
                    leading = true;
                }
            }
            if leading {
                // A line after the first of a quoted text in a block mapping must be indented past it (cell ml13).
                if self.flow == 0 && self.indent >= 0 && (self.col as i64) <= self.indent && !self.eof() {
                    return Err(Fail::Bad);
                }
                Self::fold(&mut out, &mut lead, &mut trail);
            } else {
                out.push_str(&ws);
                ws.clear();
            }
        }
        self.adv();
        self.toks
            .push(Tok::Scalar(out, if single { Style::Single } else { Style::Double }));
        Ok(())
    }

    /// U+0085, U+2028 or U+2029 between two characters of a single-quoted text: U+0085 reads as a space, the others as
    /// themselves (corpus PR 1549 JsonYamlFinalChars.al :220, :224, :236, the `s` form). Another place is not measured.
    fn unicode_break_in_quotes(&mut self, out: &mut String) -> Res<()> {
        let prev = out.chars().last().filter(|&p| !is_blank(p));
        let next = self.at(1);
        if prev.is_none() || is_blank(next) || is_break(next) || next == '\'' {
            return Err(Fail::Refuse(
                "ReadFromYaml of U+0085, U+2028 or U+2029 at a place in single quotes no probe measured",
            ));
        }
        let c = self.adv();
        out.push(if c == '\u{85}' { ' ' } else { c });
        self.marked = true;
        self.unicode_breaks += 1;
        Ok(())
    }

    /// U+0085, U+2028 or U+2029 right after a character of a plain text in a flow form, before another one: the YAML
    /// error (corpus PR 1549 JsonYamlFinalChars.al :220, :224, :236, the `fs` and `fm` forms). After a blank or before a
    /// blank or a line break it is not measured.
    fn unicode_break_in_flow(&mut self, after_blank: bool) -> Res<()> {
        let next = self.at(1);
        if after_blank || is_blank(next) || is_break(next) || self.eof() {
            return Err(Fail::Refuse(
                "ReadFromYaml of U+0085, U+2028 or U+2029 at a place in a flow form no probe measured",
            ));
        }
        self.marked = true;
        Err(Fail::Bad)
    }

    /// A comment up to its line break. U+0085, U+2028 or U+2029 right before LF or CR ends it like a line break
    /// (corpus PR 1549 JsonYamlFinalChars.al :220, :224, :236, the `c` form).
    fn comment(&mut self) {
        while !self.breakz() {
            self.adv();
        }
        if super::unicode_break(self.at(0)) && matches!(self.at(1), '\n' | '\r') {
            self.marked = true;
            self.unicode_breaks += 1;
        }
    }

    /// The escapes of a double-quoted text: the cells `\b \t \/ \x41 \0 \e \N \_ \L \P \u`, `\a`, `\ ` and `\q`
    /// (an error) of group `DoubleQuotedEscape`.
    fn escape(&mut self, out: &mut String) -> Res<()> {
        self.adv();
        let c = self.adv();
        if let Some(e) = named_escape(c) {
            out.push(e);
            return Ok(());
        }
        let hex_len = match c {
            'x' => 2,
            'u' => 4,
            'U' => 8,
            _ => return Err(Fail::Bad),
        };
        let v = self.hex(hex_len)?;
        // `\uD83D\uDE00` is one character (JsonYamlUtf8.al `u-pair` of corpus PR 1515).
        if (0xD800..0xDC00).contains(&v) && self.at(0) == '\\' && self.at(1) == 'u' {
            self.adv();
            self.adv();
            let low = self.hex(4)?;
            if (0xDC00..0xE000).contains(&low) {
                let c = 0x10000 + ((v - 0xD800) << 10) + (low - 0xDC00);
                out.push(char::from_u32(c).ok_or(Fail::Bad)?);
                return Ok(());
            }
            self.marked = true;
            return Err(Fail::Refuse(
                "ReadFromYaml of a high surrogate escape before another \\u escape (not measured)",
            ));
        }
        // A lone surrogate (`\uD800`-`\uDFFF`, `\uD83Dx`, a low before a high one, `\U0000D800`) and a code above
        // U+10FFFF (`\U00110000`, `\U00200000`, `\U7FFFFFFF`, `\UFFFFFFFF`) are the YAML error (corpus PR 1549
        // JsonYamlFinalStructure.al `EscapeSurrogates1_1`-`EscapeSurrogates1_4`, `EscapeSurrogates1_6`-`EscapeSurrogates1_8`,
        // `EscapeSurrogates1_10`, `EscapeSurrogates1_12`, `EscapeSurrogates2_1`-`EscapeSurrogates2_3`).
        // The surrogate cells end the text after it (`"`), go on with `x` after a high one or put a low one before a high
        // one (`\uDE00\uD83D`); a surrogate before any other character or escape is not measured.
        let high = (0xD800..0xDC00).contains(&v);
        let low = (0xDC00..0xE000).contains(&v);
        let next = (self.at(0), self.at(1));
        let high_next = next == ('\\', 'u')
            && (2..6)
                .try_fold(0u32, |a, k| self.at(k).to_digit(16).map(|d| a * 16 + d))
                .is_some_and(|h| (0xD800..0xDC00).contains(&h));
        let measured_next = next.0 == '"' || (high && next.0 == 'x') || (low && high_next);
        if (high || low) && !measured_next {
            self.marked = true;
            return Err(Fail::Refuse(
                "ReadFromYaml of a lone surrogate escape before a character no probe measured",
            ));
        }
        let c = char::from_u32(v);
        self.marked |= c.is_none();
        out.push(c.ok_or(Fail::Bad)?);
        Ok(())
    }

    /// `n` hex digits; a short or bad one is the YAML error (`top[x]`, `top[u12]`).
    fn hex(&mut self, n: usize) -> Res<u32> {
        let mut v = 0u32;
        for _ in 0..n {
            let d = self.at(0).to_digit(16).ok_or(Fail::Bad)?;
            self.adv();
            v = v * 16 + d;
        }
        Ok(v)
    }

    fn block_scalar(&mut self, folded: bool) -> Res<()> {
        self.remove_key()?;
        self.allowed = true;
        self.adv();
        let (mut chomp, mut inc) = (0i32, 0usize);
        let digit = |s: &mut Scanner, inc: &mut usize| -> Res<()> {
            if let Some(d) = s.at(0).to_digit(10) {
                if d == 0 {
                    return Err(Fail::Bad);
                }
                *inc = d as usize;
                s.adv();
            }
            Ok(())
        };
        if self.at(0) == '+' || self.at(0) == '-' {
            chomp = if self.adv() == '+' { 1 } else { -1 };
            digit(self, &mut inc)?;
        } else if self.at(0).is_ascii_digit() {
            digit(self, &mut inc)?;
            if self.at(0) == '+' || self.at(0) == '-' {
                chomp = if self.adv() == '+' { 1 } else { -1 };
            }
        }
        while is_blank(self.at(0)) {
            self.adv();
        }
        if self.at(0) == '#' {
            while !self.breakz() {
                self.adv();
            }
        }
        if !self.breakz() {
            return Err(Fail::Bad);
        }
        self.read_break();
        let mut indent = if inc > 0 {
            (self.indent.max(0) as usize) + inc
        } else {
            0
        };
        if inc > 0 && self.indent < 0 {
            indent = inc;
        }
        let (mut out, mut lead, mut trail) = (String::new(), String::new(), String::new());
        self.block_breaks(&mut indent, &mut trail)?;
        let mut lead_blank = false;
        while self.col == indent && !self.eof() {
            let trail_blank = is_blank(self.at(0));
            if folded && lead.starts_with('\n') && !lead_blank && !trail_blank {
                if trail.is_empty() {
                    out.push(' ');
                }
            } else {
                out.push_str(&lead);
            }
            lead.clear();
            out.push_str(&trail);
            trail.clear();
            lead_blank = is_blank(self.at(0));
            while !self.breakz() {
                out.push(self.adv());
            }
            if let Some(b) = self.read_break() {
                lead.push(b);
            }
            self.block_breaks(&mut indent, &mut trail)?;
        }
        if chomp != -1 {
            out.push_str(&lead);
        }
        if chomp == 1 {
            out.push_str(&trail);
        }
        self.toks.push(Tok::Scalar(out, Style::Block));
        Ok(())
    }

    fn block_breaks(&mut self, indent: &mut usize, trail: &mut String) -> Res<()> {
        let mut max = 0;
        loop {
            while (*indent == 0 || self.col < *indent) && self.at(0) == ' ' {
                self.adv();
            }
            max = max.max(self.col);
            if (*indent == 0 || self.col < *indent) && self.at(0) == '\t' {
                return Err(Fail::Bad);
            }
            if !is_break(self.at(0)) {
                break;
            }
            self.read_break();
            trail.push('\n');
        }
        if *indent == 0 {
            *indent = max.max((self.indent + 1).max(0) as usize).max(1);
        }
        Ok(())
    }
}
