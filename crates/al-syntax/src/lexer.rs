//! Lossless lexer: the concatenated token texts are exactly the input.

use crate::SyntaxKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token {
    pub kind: SyntaxKind,
    /// Length in bytes.
    pub len: u32,
}

/// U+FEFF in UTF-8.
const BOM: [u8; 3] = [0xEF, 0xBB, 0xBF];

pub fn lex(src: &str) -> Vec<Token> {
    let mut lx = Lexer {
        text: src,
        src: src.as_bytes(),
        pos: 0,
        line_has_code: false,
        out: Vec::with_capacity(src.len() / 4),
    };
    if src.starts_with('\u{FEFF}') {
        lx.pos = 3;
        lx.push(SyntaxKind::Bom, 0);
    }
    while lx.pos < lx.src.len() {
        lx.next_token();
    }
    lx.out
}

struct Lexer<'a> {
    text: &'a str,
    src: &'a [u8],
    pos: usize,
    /// Whether a non-trivia token has appeared on the current line, which
    /// decides if `#` starts a directive.
    line_has_code: bool,
    out: Vec<Token>,
}

fn is_ident_start(c: char) -> bool {
    c == '_' || c.is_alphabetic()
}

fn is_ident_continue(c: char) -> bool {
    c == '_' || c.is_alphanumeric()
}

impl Lexer<'_> {
    fn push(&mut self, kind: SyntaxKind, start: usize) {
        self.out.push(Token {
            kind,
            len: (self.pos - start) as u32,
        });
        if !kind.is_trivia() {
            self.line_has_code = true;
        }
    }

    fn peek(&self, off: usize) -> u8 {
        self.src.get(self.pos + off).copied().unwrap_or(0)
    }

    fn char_at(&self, pos: usize) -> Option<char> {
        // `pos` is always on a char boundary. Only the first char is decoded;
        // validating the rest of the file here made lexing quadratic.
        match *self.src.get(pos)? {
            b if b < 0x80 => Some(b as char),
            _ => self.text[pos..].chars().next(),
        }
    }

    fn eat_while(&mut self, f: impl Fn(u8) -> bool) {
        while self.pos < self.src.len() && f(self.src[self.pos]) {
            self.pos += 1;
        }
    }

    fn eat_ident_rest(&mut self) {
        let src = self.src;
        let mut i = self.pos;
        // The ASCII run, with no bounds checks beyond the slice's own.
        while i < src.len() && (src[i] == b'_' || src[i].is_ascii_alphanumeric()) {
            i += 1;
        }
        self.pos = i;
        while let Some(&b) = self.src.get(self.pos) {
            if b < 0x80 {
                if !(b == b'_' || b.is_ascii_alphanumeric()) {
                    break;
                }
                self.pos += 1;
            } else {
                let Some(c) = self.char_at(self.pos) else { break };
                if !is_ident_continue(c) {
                    break;
                }
                self.pos += c.len_utf8();
            }
        }
    }

    fn next_token(&mut self) {
        use SyntaxKind::*;
        let start = self.pos;
        let c = self.src[self.pos];
        match c {
            b'\n' => {
                self.pos += 1;
                self.push(Newline, start);
                self.line_has_code = false;
            }
            b'\r' if self.peek(1) == b'\n' => {
                self.pos += 2;
                self.push(Newline, start);
                self.line_has_code = false;
            }
            // A byte order mark (U+FEFF, bytes EF BB BF) between tokens is white space, as in BC's compiler.
            b' ' | b'\t' | b'\r' | 0x0c | 0xEF if c != 0xEF || self.src[self.pos..].starts_with(&BOM) => {
                loop {
                    self.eat_while(|b| matches!(b, b' ' | b'\t' | 0x0c));
                    if self.src[self.pos..].starts_with(&BOM) {
                        self.pos += 3;
                    } else {
                        break;
                    }
                }
                if self.pos == start {
                    self.pos += 1; // lone \r
                }
                self.push(Whitespace, start);
            }
            b'/' if self.peek(1) == b'/' => {
                self.eat_while(|b| b != b'\n' && b != b'\r');
                self.push(LineComment, start);
            }
            b'/' if self.peek(1) == b'*' => {
                self.pos += 2;
                loop {
                    if self.pos >= self.src.len() {
                        self.push(Error, start);
                        return;
                    }
                    if self.src[self.pos] == b'*' && self.peek(1) == b'/' {
                        self.pos += 2;
                        break;
                    }
                    self.pos += 1;
                }
                self.push(BlockComment, start);
            }
            b'#' if !self.line_has_code => {
                self.eat_while(|b| b != b'\n' && b != b'\r');
                self.push(Directive, start);
            }
            b'\'' => self.string(start, false),
            // `@'...'`: a multiline text literal (BC 26 and later; see `multiline_text_literal_is_one_string_token`).
            b'@' if self.peek(1) == b'\'' => {
                self.pos += 1;
                self.string(start, true);
            }
            b'"' => {
                self.pos += 1;
                // A doubled quote inside the name is one escaped quote (see `ast::unquote_ident`).
                loop {
                    self.eat_while(|b| b != b'"' && b != b'\n');
                    if self.peek(0) == b'"' && self.peek(1) == b'"' {
                        self.pos += 2;
                    } else {
                        break;
                    }
                }
                if self.peek(0) == b'"' {
                    self.pos += 1;
                    self.push(QuotedIdent, start);
                } else {
                    self.push(Error, start);
                }
            }
            b'0'..=b'9' => self.number(start),
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => {
                self.pos += 1;
                self.eat_ident_rest();
                self.push(Ident, start);
            }
            _ => {
                let ch = self.char_at(self.pos).expect("valid utf-8");
                if is_ident_start(ch) {
                    self.pos += ch.len_utf8();
                    self.eat_ident_rest();
                    self.push(Ident, start);
                } else {
                    self.punct(start, ch);
                }
            }
        }
    }

    fn string(&mut self, start: usize, multiline: bool) {
        let src = self.src;
        let mut i = self.pos + 1;
        // Up to the closing quote; a doubled quote is one quote of the text; a line end (unless `multiline`) or the
        // end of the file makes an error token.
        while i < src.len() {
            match src[i] {
                b'\'' if src.get(i + 1) == Some(&b'\'') => i += 2,
                b'\'' => {
                    self.pos = i + 1;
                    self.push(SyntaxKind::StringLit, start);
                    return;
                }
                b'\n' | b'\r' if !multiline => break,
                _ => i += 1,
            }
        }
        self.pos = i;
        self.push(SyntaxKind::Error, start);
    }

    fn number(&mut self, start: usize) {
        use SyntaxKind::*;
        self.eat_while(|b| b.is_ascii_digit());
        let mut kind = Integer;
        if self.peek(0) == b'.' && self.peek(1).is_ascii_digit() {
            self.pos += 1;
            self.eat_while(|b| b.is_ascii_digit());
            kind = Decimal;
        }
        let suffix_start = self.pos;
        self.eat_ident_rest();
        let suffix = &self.text[suffix_start..self.pos];
        // An exponent (`e3`) is not a suffix: the number ends before it and the `e3` is an identifier
        // (corpus must-not-compile/cases/0582-decimal-literal-scientific.al pins AL0111 "Semicolon expected").
        if suffix.len() > 1
            && suffix.as_bytes()[0].eq_ignore_ascii_case(&b'e')
            && suffix[1..].bytes().all(|b| b.is_ascii_digit())
        {
            self.pos = suffix_start;
            self.push(kind, start);
            return;
        }
        kind = if suffix.is_empty() {
            kind
        } else if suffix.eq_ignore_ascii_case("D") && kind == Integer {
            DateLit
        } else if suffix.eq_ignore_ascii_case("DT") && kind == Integer {
            DateTimeLit
        } else if suffix.eq_ignore_ascii_case("T") {
            TimeLit
        } else if suffix.eq_ignore_ascii_case("L") && kind == Integer {
            BigIntLit
        } else {
            Error
        };
        self.push(kind, start);
    }

    fn punct(&mut self, start: usize, ch: char) {
        use SyntaxKind::*;
        let two = match (self.peek(0), self.peek(1)) {
            (b':', b'=') => Some(Assign),
            (b'+', b'=') => Some(PlusAssign),
            (b'-', b'=') => Some(MinusAssign),
            (b'*', b'=') => Some(StarAssign),
            (b'/', b'=') => Some(SlashAssign),
            (b'.', b'.') => Some(DotDot),
            (b':', b':') => Some(ColonColon),
            (b'<', b'>') => Some(Neq),
            (b'<', b'=') => Some(Le),
            (b'>', b'=') => Some(Ge),
            _ => None,
        };
        if let Some(k) = two {
            self.pos += 2;
            return self.push(k, start);
        }
        let one = match ch {
            '(' => LParen,
            ')' => RParen,
            '[' => LBrack,
            ']' => RBrack,
            '{' => LBrace,
            '}' => RBrace,
            ',' => Comma,
            ';' => Semicolon,
            ':' => Colon,
            '.' => Dot,
            '+' => Plus,
            '-' => Minus,
            '*' => Star,
            '/' => Slash,
            '=' => Eq,
            '<' => Lt,
            '>' => Gt,
            '?' => Question,
            '|' => Pipe,
            '&' => Amp,
            _ => Error,
        };
        self.pos += ch.len_utf8();
        self.push(one, start);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use SyntaxKind::*;

    fn kinds(src: &str) -> Vec<(SyntaxKind, &str)> {
        let mut out = Vec::new();
        let mut pos = 0;
        for t in lex(src) {
            let end = pos + t.len as usize;
            out.push((t.kind, &src[pos..end]));
            pos = end;
        }
        assert_eq!(pos, src.len(), "tokens must cover the input");
        out
    }

    fn significant(src: &str) -> Vec<(SyntaxKind, &str)> {
        kinds(src).into_iter().filter(|(k, _)| !k.is_trivia()).collect()
    }

    #[test]
    fn empty_input_has_no_tokens() {
        assert!(lex("").is_empty());
    }

    // A doubled quote inside a quoted identifier is one escaped quote. Microsoft's page "Sales Order Statistics"
    // declares `field("TotalAdjCostLCY[2] - TotalSalesLineLCY[2].""Unit Cost (LCY)"""; ...)`, which real BC
    // compiles (no corpus test uses it; issue 30).
    #[test]
    fn a_doubled_quote_inside_a_quoted_identifier_is_one_escaped_quote() {
        assert_eq!(
            significant(r#"X."A.""B (C)""".Y "" Z"#),
            vec![
                (Ident, "X"),
                (Dot, "."),
                (QuotedIdent, r#""A.""B (C)""""#),
                (Dot, "."),
                (Ident, "Y"),
                (QuotedIdent, r#""""#),
                (Ident, "Z"),
            ]
        );
    }

    #[test]
    fn identifiers_and_quoted_identifiers() {
        assert_eq!(
            significant(r#"Rec "ALT Universal" _x1"#),
            vec![(Ident, "Rec"), (QuotedIdent, "\"ALT Universal\""), (Ident, "_x1")]
        );
    }

    #[test]
    fn strings_with_doubled_quote_escape() {
        assert_eq!(
            significant("'it''s' ''"),
            vec![(StringLit, "'it''s'"), (StringLit, "''")]
        );
    }

    // A multiline text literal `@'...'` (BC 26 and later) is one string token that may span lines, with the
    // line ends of the source kept (al-language/misc/lt-expressions-statements/PLiteralExpressions.al:152-158,
    // al-language/text/lt-bu-multiline-crlf/BuMultilineCrlf.al:23-27).
    #[test]
    fn multiline_text_literal_is_one_string_token() {
        assert_eq!(significant("@'a\nb' x"), vec![(StringLit, "@'a\nb'"), (Ident, "x")]);
        assert_eq!(significant("@'a\r\nb'"), vec![(StringLit, "@'a\r\nb'")]);
        assert_eq!(significant("@'it''s\nq'"), vec![(StringLit, "@'it''s\nq'")]);
        assert_eq!(significant("@'single'"), vec![(StringLit, "@'single'")]);
        // A plain literal still ends at the line end.
        assert_eq!(significant("'a\nb'")[0].0, Error);
    }

    #[test]
    fn string_does_not_end_a_comment_marker_inside_it() {
        assert_eq!(significant("'a // b' x"), vec![(StringLit, "'a // b'"), (Ident, "x")]);
    }

    #[test]
    fn unterminated_string_stops_at_end_of_line_as_error() {
        assert_eq!(kinds("'abc\nx"), vec![(Error, "'abc"), (Newline, "\n"), (Ident, "x")]);
    }

    #[test]
    fn numbers_and_typed_literals() {
        assert_eq!(
            significant("42 1.5 0D 20240101d 120000T 0DT 1L 120000.5T"),
            vec![
                (Integer, "42"),
                (Decimal, "1.5"),
                (DateLit, "0D"),
                (DateLit, "20240101d"),
                (TimeLit, "120000T"),
                (DateTimeLit, "0DT"),
                (BigIntLit, "1L"),
                (TimeLit, "120000.5T"),
            ]
        );
    }

    #[test]
    fn an_exponent_is_not_part_of_a_number() {
        // AL has no scientific notation: `1.5e3` is the decimal `1.5` followed by the identifier `e3`, which the parser
        // then reports as "Semicolon expected" (corpus must-not-compile/cases/0582-decimal-literal-scientific.al, AL0111).
        assert_eq!(
            significant("1.5e3 2E10"),
            vec![(Decimal, "1.5"), (Ident, "e3"), (Integer, "2"), (Ident, "E10")]
        );
    }

    #[test]
    fn a_range_is_not_a_decimal() {
        assert_eq!(
            significant("1..5"),
            vec![(Integer, "1"), (DotDot, ".."), (Integer, "5")]
        );
    }

    #[test]
    fn a_letter_after_digits_that_is_not_a_suffix_is_not_swallowed() {
        // `1Day` is not a date literal; the lexer must not split it as `1D` + `ay`.
        assert_eq!(significant("1Day"), vec![(Error, "1Day")]);
    }

    #[test]
    fn compound_operators_take_the_longest_match() {
        assert_eq!(
            significant(":= += -= *= /= .. :: <> <= >= = < > + - * / : . ( ) [ ] { } , ;")
                .into_iter()
                .map(|(k, _)| k)
                .collect::<Vec<_>>(),
            vec![
                Assign,
                PlusAssign,
                MinusAssign,
                StarAssign,
                SlashAssign,
                DotDot,
                ColonColon,
                Neq,
                Le,
                Ge,
                Eq,
                Lt,
                Gt,
                Plus,
                Minus,
                Star,
                Slash,
                Colon,
                Dot,
                LParen,
                RParen,
                LBrack,
                RBrack,
                LBrace,
                RBrace,
                Comma,
                Semicolon
            ]
        );
    }

    #[test]
    fn comments_are_trivia() {
        assert_eq!(
            kinds("a // c\n/* b\n */b"),
            vec![
                (Ident, "a"),
                (Whitespace, " "),
                (LineComment, "// c"),
                (Newline, "\n"),
                (BlockComment, "/* b\n */"),
                (Ident, "b")
            ]
        );
    }

    #[test]
    fn unterminated_block_comment_is_an_error_to_end_of_input() {
        assert_eq!(kinds("/* x"), vec![(Error, "/* x")]);
    }

    #[test]
    fn crlf_is_one_newline_token() {
        assert_eq!(kinds("a\r\nb"), vec![(Ident, "a"), (Newline, "\r\n"), (Ident, "b")]);
    }

    #[test]
    fn directive_at_line_start_spans_the_line() {
        assert_eq!(
            kinds("  #if BC29PLUS // why\nx"),
            vec![
                (Whitespace, "  "),
                (Directive, "#if BC29PLUS // why"),
                (Newline, "\n"),
                (Ident, "x")
            ]
        );
    }

    #[test]
    fn hash_after_code_on_the_same_line_is_an_error() {
        assert_eq!(significant("x #if"), vec![(Ident, "x"), (Error, "#"), (Ident, "if")]);
    }

    #[test]
    fn bom_is_its_own_trivia_token() {
        assert_eq!(kinds("\u{FEFF}x"), vec![(Bom, "\u{FEFF}"), (Ident, "x")]);
    }

    // Microsoft's BC 29 Base Application has pages (Sales/Reminder/ReminderLevels.Page.al, ReminderTerms.Page.al,
    // ReminderTermsTranslation.Page.al, ReminderText.Page.al) with a byte order mark at the start of line 2; BC compiles
    // them, so a U+FEFF between tokens is white space.
    #[test]
    fn a_byte_order_mark_between_tokens_is_white_space() {
        assert_eq!(
            kinds("page 1 P\n\u{FEFF}{ x }\n \u{FEFF} \u{FEFF}y"),
            vec![
                (Ident, "page"),
                (Whitespace, " "),
                (Integer, "1"),
                (Whitespace, " "),
                (Ident, "P"),
                (Newline, "\n"),
                (Whitespace, "\u{FEFF}"),
                (LBrace, "{"),
                (Whitespace, " "),
                (Ident, "x"),
                (Whitespace, " "),
                (RBrace, "}"),
                (Newline, "\n"),
                (Whitespace, " \u{FEFF} \u{FEFF}"),
                (Ident, "y"),
            ]
        );
        // Inside a string or a comment it is part of the text.
        assert_eq!(significant("'a\u{FEFF}b'"), vec![(StringLit, "'a\u{FEFF}b'")]);
        assert_eq!(kinds("// a\u{FEFF}b"), vec![(LineComment, "// a\u{FEFF}b")]);
    }

    #[test]
    fn non_ascii_in_comments_and_strings_keeps_char_boundaries() {
        assert_eq!(
            significant("'ä — ü' x // ✓"),
            vec![(StringLit, "'ä — ü'"), (Ident, "x")]
        );
    }

    /// `&` joins terms in a TableRelation filter (record/lt-table-relation-quotes/TableRelationQuotes.al:65,
    /// `filter(<> A & <> "B C")`).
    #[test]
    fn filter_ampersand() {
        assert_eq!(
            significant("filter(<> A & <> \"B C\")"),
            vec![
                (Ident, "filter"),
                (LParen, "("),
                (Neq, "<>"),
                (Ident, "A"),
                (Amp, "&"),
                (Neq, "<>"),
                (QuotedIdent, "\"B C\""),
                (RParen, ")"),
            ]
        );
    }

    #[test]
    fn ternary_question_mark_and_filter_pipe() {
        assert_eq!(
            significant("c ? filter(1 | 2) : x"),
            vec![
                (Ident, "c"),
                (Question, "?"),
                (Ident, "filter"),
                (LParen, "("),
                (Integer, "1"),
                (Pipe, "|"),
                (Integer, "2"),
                (RParen, ")"),
                (Colon, ":"),
                (Ident, "x")
            ]
        );
    }

    #[test]
    fn non_ascii_letters_continue_and_start_identifiers() {
        assert_eq!(
            significant("Größe1 Äpfel x_ü2 ü"),
            vec![(Ident, "Größe1"), (Ident, "Äpfel"), (Ident, "x_ü2"), (Ident, "ü")]
        );
    }

    #[test]
    fn a_non_ascii_letter_after_digits_is_not_a_literal_suffix() {
        assert_eq!(significant("1ä 2D"), vec![(Error, "1ä"), (DateLit, "2D")]);
    }

    #[test]
    fn a_multibyte_symbol_is_one_error_token_and_lexing_continues() {
        assert_eq!(significant("a €x"), vec![(Ident, "a"), (Error, "€"), (Ident, "x")]);
    }

    #[test]
    fn lexing_a_long_file_of_non_ascii_text_is_linear() {
        // The lexer once re-validated the rest of the input for every
        // character, which made this quadratic: 600k characters took minutes.
        let src = "ä ".repeat(300_000);
        let started = std::time::Instant::now();
        assert_eq!(lex(&src).len(), 600_000);
        assert!(
            started.elapsed() < std::time::Duration::from_secs(10),
            "lexing took {:?}",
            started.elapsed()
        );
    }

    #[test]
    fn unknown_character_is_a_one_char_error() {
        assert_eq!(significant("a ~ b"), vec![(Ident, "a"), (Error, "~"), (Ident, "b")]);
    }
}
