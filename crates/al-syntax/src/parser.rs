//! Parser infrastructure. The grammar lives in `grammar/`.
//!
//! Every input token, trivia included, is added to the tree exactly once, so
//! the tree's text is the source by construction. Trivia is attached to the
//! innermost node open when the next significant token arrives; nodes start
//! *after* pending trivia, so leading comments belong to the parent.
//!
//! Parser infrastructure and the parse entry points. Entry points: `parse`, `parse_with_cache`,
//! `Parse`.

use crate::builder::{Checkpoint, TreeBuilder, TreeCache};
use crate::preprocessor::preprocess;
use crate::{SyntaxKind, SyntaxNode, Token, lex};
use rowan::GreenNode;
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub offset: usize,
    pub message: String,
}

pub struct Parse {
    pub green: GreenNode,
    pub errors: Vec<ParseError>,
}

impl Parse {
    pub fn syntax(&self) -> SyntaxNode {
        SyntaxNode::new_root(self.green.clone())
    }
}

/// Lex, preprocess with `symbols`, and parse a whole `.al` file.
pub fn parse(src: &str, symbols: &BTreeSet<String>) -> Parse {
    with_thread_cache(|cache| parse_with_cache(src, symbols, cache))
}

/// Runs `f` with this thread's token cache, so repeated calls share tokens and pay for the
/// table (128 KB) once. Falls back to a private one if this is somehow re-entered.
fn with_thread_cache<T>(f: impl FnOnce(&mut TreeCache) -> T) -> T {
    thread_local! {
        static CACHE: std::cell::RefCell<TreeCache> = Default::default();
    }
    CACHE.with(|c| match c.try_borrow_mut() {
        Ok(mut cache) => f(&mut cache),
        Err(_) => f(&mut TreeCache::default()),
    })
}

/// [`parse`], interning tree nodes in `cache`. The result is identical.
pub fn parse_with_cache(src: &str, symbols: &BTreeSet<String>, cache: &mut TreeCache) -> Parse {
    parse_entry(src, symbols, cache, crate::grammar::source_file)
}

/// Parses a standalone type such as a table field's `Code[20]`; `None` if it is
/// not exactly one well-formed type.
pub fn parse_type(src: &str) -> Option<crate::ast::TypeRef> {
    // Every field of every table asks, so the cache is the thread's, not a new one each time.
    let p = with_thread_cache(|cache| parse_entry(src, &BTreeSet::new(), cache, crate::grammar::type_ref));
    if !p.errors.is_empty() {
        return None;
    }
    p.syntax().children().find_map(crate::ast::TypeRef::cast)
}

pub(crate) fn parse_entry(
    src: &str,
    symbols: &BTreeSet<String>,
    cache: &mut TreeCache,
    entry: fn(&mut Parser),
) -> Parse {
    let tokens = lex(src);
    // Without a directive the preprocessor would copy the tokens unchanged.
    let pre = if tokens.iter().any(|t| t.kind == SyntaxKind::Directive) {
        preprocess(src, &tokens, symbols)
    } else {
        crate::preprocessor::Preprocessed {
            tokens,
            diagnostics: Vec::new(),
        }
    };
    let mut p = Parser::new(src, &pre.tokens, cache);
    p.errors.extend(pre.diagnostics.into_iter().map(|d| ParseError {
        offset: d.offset,
        message: d.message,
    }));
    p.builder.start_node(SyntaxKind::SourceFile);
    entry(&mut p);
    // Anything the entry point did not consume is an error, but still part of the tree.
    if !p.at_eof() {
        p.error("unexpected input");
        p.start(SyntaxKind::ErrorNode);
        while !p.at_eof() {
            p.bump_any();
        }
        p.finish();
    }
    p.flush_trivia();
    p.builder.finish_node();
    Parse {
        green: p.builder.finish(),
        errors: p.errors,
    }
}

pub(crate) struct Parser<'a> {
    src: &'a str,
    /// Every token, trivia included: kind, start and end byte offsets.
    tokens: Vec<(SyntaxKind, usize, usize)>,
    pos: usize,
    /// The significant (non-trivia) tokens only, so look-ahead is an index
    /// instead of a scan over trivia, and `sp` is how many have been consumed.
    sig: Vec<(SyntaxKind, u32, u32, u16)>,
    // The last field of an entry is a keyword prefilter: for an identifier its length and lower-cased first letter
    // (`length << 8 | letter`), otherwise zero.
    sp: usize,
    builder: TreeBuilder<'a>,
    pub(crate) errors: Vec<ParseError>,
    /// Guards against grammar loops that do not consume input.
    fuel: std::cell::Cell<u32>,
}

impl<'a> Parser<'a> {
    fn new(src: &'a str, toks: &[Token], cache: &'a mut TreeCache) -> Self {
        let mut tokens = Vec::with_capacity(toks.len());
        let mut sig = Vec::with_capacity(toks.len() / 2);
        let mut off = 0;
        for t in toks {
            let end = off + t.len as usize;
            tokens.push((t.kind, off, end));
            if !t.kind.is_trivia() {
                let sigil = if t.kind == SyntaxKind::Ident && t.len < 256 {
                    u16::from(t.len as u8) << 8 | u16::from(src.as_bytes()[off].to_ascii_lowercase())
                } else {
                    0
                };
                sig.push((t.kind, off as u32, end as u32, sigil));
            }
            off = end;
        }
        Parser {
            src,
            tokens,
            pos: 0,
            sig,
            sp: 0,
            builder: TreeBuilder::new(cache),
            errors: Vec::new(),
            fuel: 256.into(),
        }
    }

    /// The n-th significant token ahead (0 = current).
    #[inline]
    fn sig(&self, n: usize) -> Option<(SyntaxKind, usize, usize)> {
        self.sig
            .get(self.sp + n)
            .map(|&(k, s, e, _)| (k, s as usize, e as usize))
    }

    #[inline]
    fn tick(&self) {
        if self.fuel.get() == 0 {
            panic!("parser made no progress at offset {}", self.offset());
        }
        self.fuel.set(self.fuel.get() - 1);
    }

    /// Kind of the n-th significant token ahead (0 = current).
    pub(crate) fn nth(&self, n: usize) -> SyntaxKind {
        self.tick();
        self.sig.get(self.sp + n).map_or(SyntaxKind::__Last, |t| t.0)
    }

    pub(crate) fn current(&self) -> SyntaxKind {
        self.nth(0)
    }

    pub(crate) fn nth_text(&self, n: usize) -> &'a str {
        self.sig(n).map_or("", |(_, s, e)| &self.src[s..e])
    }

    pub(crate) fn at(&self, kind: SyntaxKind) -> bool {
        self.current() == kind
    }

    pub(crate) fn at_eof(&self) -> bool {
        self.sp >= self.sig.len()
    }

    /// Current token is the identifier `kw`, case-insensitively.
    pub(crate) fn at_kw(&self, kw: &str) -> bool {
        self.nth_kw(0, kw)
    }

    pub(crate) fn nth_kw(&self, n: usize, kw: &str) -> bool {
        self.tick();
        let Some(&(kind, start, end, sigil)) = self.sig.get(self.sp + n) else {
            return false;
        };
        // The length and the first letter turn most checks down (the keywords are lower case and under 256 bytes).
        let kb = kw.as_bytes();
        if kind != SyntaxKind::Ident || sigil != (kb.len() as u16) << 8 | u16::from(kb[0]) {
            return false;
        }
        self.src.as_bytes()[start as usize..end as usize].eq_ignore_ascii_case(kb)
    }

    /// Current token is the identifier `kw` for one of `kws`, case-insensitively (one look-ahead for all of them).
    pub(crate) fn at_any_kw(&self, kws: &[&str]) -> bool {
        self.tick();
        let Some(&(kind, start, end, sigil)) = self.sig.get(self.sp) else {
            return false;
        };
        if kind != SyntaxKind::Ident {
            return false;
        }
        kws.iter().any(|kw| {
            let kb = kw.as_bytes();
            sigil == (kb.len() as u16) << 8 | u16::from(kb[0])
                && self.src.as_bytes()[start as usize..end as usize].eq_ignore_ascii_case(kb)
        })
    }

    /// Which of `kws` the current token is (an identifier, case-insensitively), by one look-ahead.
    pub(crate) fn which_kw(&self, kws: &[&str]) -> Option<usize> {
        self.tick();
        let &(kind, start, end, sigil) = self.sig.get(self.sp)?;
        if kind != SyntaxKind::Ident {
            return None;
        }
        kws.iter().position(|kw| {
            let kb = kw.as_bytes();
            sigil == (kb.len() as u16) << 8 | u16::from(kb[0])
                && self.src.as_bytes()[start as usize..end as usize].eq_ignore_ascii_case(kb)
        })
    }

    pub(crate) fn offset(&self) -> usize {
        self.sig(0).map_or(self.src.len(), |(_, s, _)| s)
    }

    fn flush_trivia(&mut self) {
        while self.pos < self.tokens.len() && self.tokens[self.pos].0.is_trivia() {
            self.push_raw();
        }
    }

    fn push_raw(&mut self) {
        let (k, s, e) = self.tokens[self.pos];
        self.builder.token(k, &self.src[s..e]);
        self.pos += 1;
        if !k.is_trivia() {
            self.sp += 1;
        }
    }

    pub(crate) fn start(&mut self, kind: SyntaxKind) {
        self.flush_trivia();
        self.builder.start_node(kind);
    }

    pub(crate) fn finish(&mut self) {
        self.builder.finish_node();
    }

    pub(crate) fn checkpoint(&mut self) -> Checkpoint {
        self.flush_trivia();
        self.builder.checkpoint()
    }

    pub(crate) fn start_at(&mut self, cp: Checkpoint, kind: SyntaxKind) {
        self.builder.start_node_at(cp, kind);
    }

    pub(crate) fn bump_any(&mut self) {
        self.flush_trivia();
        if self.pos < self.tokens.len() {
            self.push_raw();
            self.fuel.set(256);
        }
    }

    pub(crate) fn eat(&mut self, kind: SyntaxKind) -> bool {
        if self.at(kind) {
            self.bump_any();
            true
        } else {
            false
        }
    }

    pub(crate) fn eat_kw(&mut self, kw: &str) -> bool {
        if self.at_kw(kw) {
            self.bump_any();
            true
        } else {
            false
        }
    }

    pub(crate) fn expect(&mut self, kind: SyntaxKind) -> bool {
        if self.eat(kind) {
            return true;
        }
        self.error(&format!("expected {kind:?}"));
        false
    }

    pub(crate) fn expect_kw(&mut self, kw: &str) -> bool {
        if self.eat_kw(kw) {
            return true;
        }
        self.error(&format!("expected `{kw}`"));
        false
    }

    pub(crate) fn error(&mut self, msg: &str) {
        let found = self.nth_text(0);
        self.errors.push(ParseError {
            offset: self.offset(),
            message: format!("{msg}, found `{found}`"),
        });
    }

    /// Records an error and wraps the current token in an `ErrorNode`, so the
    /// parser always makes progress.
    pub(crate) fn err_and_bump(&mut self, msg: &str) {
        self.error(msg);
        if !self.at_eof() {
            self.start(SyntaxKind::ErrorNode);
            self.bump_any();
            self.finish();
        }
    }
}

/// Compact S-expression view of a tree for tests: nodes in parentheses,
/// significant tokens by their text, trivia omitted.
pub fn debug_tree(node: &SyntaxNode) -> String {
    let mut s = String::new();
    write_node(node, &mut s);
    s
}

fn write_node(node: &SyntaxNode, s: &mut String) {
    s.push('(');
    s.push_str(&format!("{:?}", node.kind()));
    for child in node.children_with_tokens() {
        match child {
            rowan::NodeOrToken::Node(n) => {
                s.push(' ');
                write_node(&n, s);
            }
            rowan::NodeOrToken::Token(t) if !t.kind().is_trivia() => {
                s.push(' ');
                s.push_str(t.text());
            }
            rowan::NodeOrToken::Token(_) => {}
        }
    }
    s.push(')');
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: &str = "codeunit 1 A { procedure P() begin if true then exit; end; }";
    const B: &str = "table 2 B { fields { field(1; X; Integer) { } } }\n#if BC27PLUS\ncodeunit 3 C { }\n#endif\n";

    #[test]
    fn a_shared_cache_gives_the_same_trees_as_a_fresh_one() {
        let syms: BTreeSet<String> = ["BC27PLUS".to_string()].into();
        let mut cache = TreeCache::default();
        for src in [A, B, A, B, ""] {
            let shared = parse_with_cache(src, &syms, &mut cache);
            let fresh = parse_with_cache(src, &syms, &mut TreeCache::default());
            assert_eq!(debug_tree(&shared.syntax()), debug_tree(&fresh.syntax()));
            assert_eq!(shared.syntax().text(), src);
            assert_eq!(shared.errors, fresh.errors);
            assert_eq!(shared.green, fresh.green, "structurally equal trees");
        }
    }

    #[test]
    fn parse_uses_the_directive_free_fast_path_and_the_preprocessor_alike() {
        // Same code with and without a directive line: only the disabled text differs.
        let plain = parse(A, &BTreeSet::new());
        let with = parse(&format!("#if NOPE\nx\n#endif\n{A}"), &BTreeSet::new());
        assert!(plain.errors.is_empty() && with.errors.is_empty());
        assert_eq!(debug_tree(&plain.syntax()), debug_tree(&with.syntax()));
    }
}
