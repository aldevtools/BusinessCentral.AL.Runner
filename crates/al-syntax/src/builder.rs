//! Green tree construction for the parser.
//!
//! This replaces `rowan::GreenNodeBuilder` for speed. rowan hashes every token
//! and every small node to deduplicate them, which was about 40% of parse time.
//! Here tokens are interned in a direct-mapped table (a cheap hash, one compare,
//! no probing), and nodes are not deduplicated at all. The trees are the same
//! as far as any reader can tell: same kinds, same text, same shape.

use crate::SyntaxKind;
use rowan::{GreenNode, GreenToken, NodeOrToken};

/// Number of slots in the token table; a power of two.
const SLOTS: usize = 1 << 14;

/// Interning table for tree tokens. Parsing many files with one cache shares
/// the common tokens (`begin`, `;`, a space, `Rec`) across files instead of
/// allocating them again per file, which is both faster and smaller. A cache is
/// not thread-safe; use one per thread. Trees stay valid after it is dropped.
pub struct TreeCache {
    slots: Box<[Option<GreenToken>]>,
    /// The token of each kind whose text the lexer fixes (punctuation): found by kind, with no hash and no compare.
    fixed: Box<[Option<GreenToken>]>,
}

impl Default for TreeCache {
    fn default() -> Self {
        TreeCache {
            slots: vec![None; SLOTS].into_boxed_slice(),
            fixed: vec![None; 64].into_boxed_slice(),
        }
    }
}

impl TreeCache {
    fn token(&mut self, kind: SyntaxKind, text: &str) -> GreenToken {
        let raw: rowan::SyntaxKind = kind.into();
        if kind >= SyntaxKind::LParen && kind <= SyntaxKind::Amp {
            // The lexer gives each of these kinds one spelling (`:=` is Assign and nothing else is), so the text is
            // the kind's; the length is checked as a guard.
            let slot = &mut self.fixed[kind as usize - SyntaxKind::LParen as usize];
            if let Some(t) = slot {
                if t.text().len() == text.len() {
                    debug_assert_eq!(t.text(), text);
                    return t.clone();
                }
            }
            let t = GreenToken::new(raw, text);
            *slot = Some(t.clone());
            return t;
        }
        // A long text (a comment, a long string) is hardly ever seen twice: hashing and comparing it costs more than the
        // copy it could save.
        if text.len() > 40 {
            return GreenToken::new(raw, text);
        }
        let slot = &mut self.slots[hash(raw.0, text)];
        if let Some(t) = slot {
            if t.kind() == raw && same_text(t.text(), text) {
                return t.clone();
            }
        }
        let t = GreenToken::new(raw, text);
        *slot = Some(t.clone());
        t
    }
}

/// Whether two texts are equal, by inline word compares for the short texts that tokens are made of (a call to `bcmp`
/// costs more than the compare).
#[inline(always)]
fn same_text(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let n = a.len();
    if n != b.len() {
        return false;
    }
    let rd8 = |s: &[u8], i: usize| u64::from_le_bytes(s[i..i + 8].try_into().expect("8 bytes"));
    if n >= 8 {
        if n > 32 {
            return a == b;
        }
        let mut i = 0;
        while i + 8 <= n {
            if rd8(a, i) != rd8(b, i) {
                return false;
            }
            i += 8;
        }
        i == n || rd8(a, n - 8) == rd8(b, n - 8)
    } else if n >= 4 {
        let rd4 = |s: &[u8], i: usize| u32::from_le_bytes(s[i..i + 4].try_into().expect("4 bytes"));
        rd4(a, 0) == rd4(b, 0) && rd4(a, n - 4) == rd4(b, n - 4)
    } else {
        n == 0 || (a[0] == b[0] && a[n / 2] == b[n / 2] && a[n - 1] == b[n - 1])
    }
}

/// A cheap multiplicative hash of the kind and the text, reduced to a slot index. The tail of the text is read with
/// overlapping loads, not byte by byte.
fn hash(kind: u16, text: &str) -> usize {
    const K: u64 = 0x517c_c1b7_2722_0a95;
    let b = text.as_bytes();
    let n = b.len();
    let mut h = (kind as u64 + 1).wrapping_mul(K) ^ n as u64;
    let rd8 = |i: usize| u64::from_le_bytes(b[i..i + 8].try_into().expect("8 bytes"));
    if n >= 8 {
        let mut i = 0;
        while i + 8 <= n {
            h = (h.rotate_left(5) ^ rd8(i)).wrapping_mul(K);
            i += 8;
        }
        if i < n {
            h = (h.rotate_left(5) ^ rd8(n - 8)).wrapping_mul(K);
        }
    } else if n >= 4 {
        let lo = u32::from_le_bytes(b[..4].try_into().expect("4 bytes")) as u64;
        let hi = u32::from_le_bytes(b[n - 4..].try_into().expect("4 bytes")) as u64;
        h = (h.rotate_left(5) ^ (lo | hi << 32)).wrapping_mul(K);
    } else if n > 0 {
        let v = b[0] as u64 | (b[n / 2] as u64) << 8 | (b[n - 1] as u64) << 16;
        h = (h.rotate_left(5) ^ v).wrapping_mul(K);
    }
    (h >> (64 - SLOTS.trailing_zeros())) as usize
}

#[derive(Clone, Copy)]
pub(crate) struct Checkpoint(usize);

pub(crate) struct TreeBuilder<'a> {
    cache: &'a mut TreeCache,
    /// Open nodes: kind and index in `children` of the node's first child.
    parents: Vec<(SyntaxKind, usize)>,
    children: Vec<NodeOrToken<GreenNode, GreenToken>>,
}

impl<'a> TreeBuilder<'a> {
    pub(crate) fn new(cache: &'a mut TreeCache) -> Self {
        TreeBuilder {
            cache,
            parents: Vec::new(),
            children: Vec::new(),
        }
    }

    pub(crate) fn token(&mut self, kind: SyntaxKind, text: &str) {
        let t = self.cache.token(kind, text);
        self.children.push(NodeOrToken::Token(t));
    }

    pub(crate) fn start_node(&mut self, kind: SyntaxKind) {
        self.parents.push((kind, self.children.len()));
    }

    pub(crate) fn finish_node(&mut self) {
        let (kind, first) = self.parents.pop().expect("finish_node without start_node");
        let node = GreenNode::new(kind.into(), self.children.drain(first..));
        self.children.push(NodeOrToken::Node(node));
    }

    pub(crate) fn checkpoint(&self) -> Checkpoint {
        Checkpoint(self.children.len())
    }

    /// Opens a node that starts at `cp`, wrapping what was added since.
    pub(crate) fn start_node_at(&mut self, cp: Checkpoint, kind: SyntaxKind) {
        assert!(cp.0 <= self.children.len(), "checkpoint is past the end");
        if let Some(&(_, first)) = self.parents.last() {
            assert!(cp.0 >= first, "checkpoint is before the open node's first child");
        }
        self.parents.push((kind, cp.0));
    }

    pub(crate) fn finish(mut self) -> GreenNode {
        assert!(self.parents.is_empty(), "unfinished nodes");
        assert_eq!(self.children.len(), 1, "a tree has exactly one root");
        match self.children.pop() {
            Some(NodeOrToken::Node(n)) => n,
            _ => panic!("the root is a node"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use SyntaxKind::*;

    fn tree(f: impl FnOnce(&mut TreeBuilder)) -> crate::SyntaxNode {
        let mut cache = TreeCache::default();
        let mut b = TreeBuilder::new(&mut cache);
        b.start_node(SourceFile);
        f(&mut b);
        b.finish_node();
        crate::SyntaxNode::new_root(b.finish())
    }

    #[test]
    fn equal_tokens_share_one_allocation_and_different_ones_do_not() {
        let mut cache = TreeCache::default();
        let a = cache.token(Ident, "begin");
        let b = cache.token(Ident, "begin");
        assert!(std::ptr::eq(a.text().as_ptr(), b.text().as_ptr()));
        let c = cache.token(Ident, "end");
        let d = cache.token(Whitespace, "end");
        assert_eq!((c.text(), d.text()), ("end", "end"));
        assert_ne!(c.kind(), d.kind(), "the kind is part of a token's identity");
        assert_eq!(cache.token(Ident, "begin").text(), "begin");
    }

    #[test]
    fn colliding_tokens_evict_each_other_but_never_return_the_wrong_text() {
        let mut cache = TreeCache::default();
        // Far more distinct tokens than slots: every lookup must still be exact.
        for i in 0..(SLOTS * 3) {
            let text = format!("ident{i}");
            assert_eq!(cache.token(Ident, &text).text(), text);
        }
    }

    #[test]
    fn nodes_nest_and_the_text_is_the_concatenated_tokens() {
        let t = tree(|b| {
            b.token(Ident, "a");
            b.start_node(Block);
            b.token(Whitespace, " ");
            b.token(Ident, "b");
            b.finish_node();
        });
        assert_eq!(t.text(), "a b");
        assert_eq!(t.children().map(|n| n.kind()).collect::<Vec<_>>(), vec![Block]);
    }

    #[test]
    fn a_checkpoint_wraps_what_was_added_since_and_wraps_again_at_the_same_place() {
        let t = tree(|b| {
            let cp = b.checkpoint();
            b.token(Ident, "a");
            b.start_node_at(cp, BinExpr);
            b.token(Plus, "+");
            b.token(Ident, "b");
            b.finish_node();
            b.start_node_at(cp, BinExpr);
            b.token(Plus, "+");
            b.token(Ident, "c");
            b.finish_node();
        });
        assert_eq!(t.text(), "a+b+c");
        // ((a + b) + c): the second wrap contains the first.
        let outer = t.children().next().unwrap();
        assert_eq!(outer.kind(), BinExpr);
        assert_eq!(outer.text(), "a+b+c");
        let inner = outer.children().next().unwrap();
        assert_eq!((inner.kind(), inner.text().to_string()), (BinExpr, "a+b".to_string()));
    }

    #[test]
    #[should_panic(expected = "before the open node's first child")]
    fn a_checkpoint_from_outside_the_open_node_is_refused() {
        tree(|b| {
            b.token(Ident, "a");
            b.start_node(Block);
            let cp = Checkpoint(0);
            b.start_node_at(cp, BinExpr);
        });
    }
}
