//! The tokens of a YAML text, cut the way a libyaml-style scanner cuts them. Every behavior here is measured by the cells
//! of corpus PR 1530 (`tests/al-language/json/JsonYamlRemaining.al`, codeunit 75671), named after the group each cell
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
    Version,
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
}

type Res<T> = Result<T, Fail>;

/// All tokens of `text` (breaks are LF, CRLF or a lone CR).
pub(super) fn tokens(text: &[char]) -> Res<Vec<Tok>> {
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
    };
    loop {
        sc.next_token()?;
        if sc.toks.last() == Some(&Tok::StreamEnd) {
            return Ok(sc.toks);
        }
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

fn is_blank(c: char) -> bool {
    c == ' ' || c == '\t'
}

fn word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_'
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
                // A tab inside a line is white space. A tab that opens a line after the first is an error
                // (corpus PR 1530 BomTabsSpaces bt8, bt11); after spaces it is not measured.
                let only_spaces = self.s[self.i - self.col..self.i].iter().all(|&p| p == ' ');
                if self.flow == 0 && self.line > 0 && only_spaces {
                    return if self.col == 0 {
                        Err(Fail::Bad)
                    } else {
                        Err(Fail::Refuse(
                            "ReadFromYaml of a tab after the spaces of an indentation (not measured)",
                        ))
                    };
                }
                self.adv();
            } else if c == '#' {
                while !self.breakz() {
                    self.adv();
                }
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

    fn stale_keys(&mut self) -> Res<()> {
        for k in self.keys.iter_mut() {
            if let Some(sk) = *k {
                if sk.line < self.line || sk.pos + 1024 < self.i {
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
        let mut name = String::new();
        while word(self.at(0)) {
            name.push(self.adv());
        }
        let c = self.at(0);
        if name.is_empty() || !(self.blankz(0) || "?:,]}%@`".contains(c)) {
            return Err(Fail::Bad);
        }
        self.toks.push(if alias { Tok::Alias(name) } else { Tok::Anchor(name) });
        Ok(())
    }

    fn uri_char(&self, c: char) -> bool {
        word(c) || ";/?:@&=+$.!~*'()%".contains(c) || (self.flow == 0 && ",[]".contains(c))
    }

    fn tag(&mut self) -> Res<()> {
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
                return Err(Fail::Bad);
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
        if !(self.blankz(0) || (self.flow > 0 && ",]}".contains(c))) {
            return Err(Fail::Bad);
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
        let tok = match name.as_str() {
            "YAML" => {
                blanks(self);
                let mut ver = String::new();
                while self.at(0).is_ascii_digit() || self.at(0) == '.' {
                    ver.push(self.adv());
                }
                if ver.split('.').next() != Some("1") || !ver.contains('.') {
                    return Err(Fail::Bad);
                }
                Tok::Version
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
            while !self.blankz(0) {
                let c = self.at(0);
                if single && c == '\'' && self.at(1) == '\'' {
                    out.push('\'');
                    self.adv();
                    self.adv();
                } else if c == q {
                    break;
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
        // `\uD83D\uDE00` is one character (JsonYamlUtf8.al `u-pair` of corpus PR 1515); a lone surrogate is not measured.
        if (0xD800..0xDC00).contains(&v) && self.at(0) == '\\' && self.at(1) == 'u' {
            self.adv();
            self.adv();
            let low = self.hex(4)?;
            if (0xDC00..0xE000).contains(&low) {
                let c = 0x10000 + ((v - 0xD800) << 10) + (low - 0xDC00);
                out.push(char::from_u32(c).ok_or(Fail::Bad)?);
                return Ok(());
            }
        }
        match char::from_u32(v) {
            Some(c) => out.push(c),
            None if (0xD800..0xE000).contains(&v) => {
                return Err(Fail::Refuse(
                    "ReadFromYaml of a lone surrogate escape in double quotes (not measured)",
                ));
            }
            None => return Err(Fail::Refuse("ReadFromYaml of an escape above U+10FFFF (not measured)")),
        }
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
