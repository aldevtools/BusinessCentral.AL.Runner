//! The text of a `Text` or `Code` value: shared, copy on write.
//!
//! A variable read, an argument, a return value and a `text_arg` clone the value; with a `String` inside, each
//! of those copied the whole text, and a loop over a long text (`CopyStr(Big, i, 1)`, `Big[i]`, `StrPos`) went
//! quadratic. [`Txt`] holds the string behind a reference count, so a clone is a counter bump; a write
//! (`+=`, an assignment to `T[i]`) copies only when the string is shared (`make_mut`) and appends in place
//! when it is not. It also remembers whether the text is all ASCII (then a UTF-16 position is a byte position),
//! computed once per string and dropped on a write.
//!
//! Internals are free (ADR-009): nothing AL code can see changes.

use std::borrow::Borrow;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering::Relaxed};

struct Inner {
    s: String,
    /// 0 not known yet, 1 ASCII only, 2 not. (Atomic: rows of the test data are shared between threads, and
    /// the flags are only hints that any thread computes the same way.)
    ascii: AtomicU8,
    /// 0 not known yet, 1 holds the call-stack marker (`CALL_STACK_MARK`), 2 does not.
    marked: AtomicU8,
}

impl Clone for Inner {
    fn clone(&self) -> Inner {
        Inner {
            s: self.s.clone(),
            ascii: AtomicU8::new(self.ascii.load(Relaxed)),
            marked: AtomicU8::new(self.marked.load(Relaxed)),
        }
    }
}

// How many times a text was scanned for the call-stack mark (the first `has_marker` of a text the flag is not known
// of). A loop that builds a text must not scan the whole text each round; the test of the append counts this. Counted
// per thread, so tests that run in parallel in one binary do not add to each other's count.
thread_local! {
    static MARKER_SCANS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// The number of scans so far on this thread (see `MARKER_SCANS`).
pub fn marker_scans() -> u64 {
    MARKER_SCANS.with(|c| c.get())
}

/// A shared, copy-on-write text.
#[derive(Clone)]
pub struct Txt(Arc<Inner>);

impl Txt {
    pub fn new(s: String) -> Txt {
        Txt(Arc::new(Inner {
            s,
            ascii: AtomicU8::new(0),
            marked: AtomicU8::new(0),
        }))
    }

    pub fn as_str(&self) -> &str {
        &self.0.s
    }

    /// Whether every character is ASCII; computed once for this string.
    pub fn is_ascii(&self) -> bool {
        match self.0.ascii.load(Relaxed) {
            1 => true,
            2 => false,
            _ => {
                let a = self.0.s.is_ascii();
                self.0.ascii.store(if a { 1 } else { 2 }, Relaxed);
                a
            }
        }
    }

    /// Whether the text holds `CALL_STACK_MARK`, the marker of a call stack with an untracked frame (no text function reads
    /// such a text). Computed once for this string.
    pub fn has_marker(&self) -> bool {
        match self.0.marked.load(Relaxed) {
            1 => true,
            2 => false,
            _ => {
                MARKER_SCANS.with(|c| c.set(c.get() + 1));
                let m = self.0.s.contains(crate::native::CALL_STACK_MARK);
                self.0.marked.store(if m { 1 } else { 2 }, Relaxed);
                m
            }
        }
    }

    /// The length in UTF-16 units (what AL counts).
    pub fn utf16_len(&self) -> usize {
        if self.is_ascii() {
            self.0.s.len()
        } else {
            self.0.s.encode_utf16().count()
        }
    }

    /// The string for writing: copied first when another value shares it. The remembered ASCII flag is dropped.
    pub fn make_mut(&mut self) -> &mut String {
        let inner = Arc::make_mut(&mut self.0);
        inner.ascii.store(0, Relaxed);
        inner.marked.store(0, Relaxed);
        &mut inner.s
    }

    /// Overwrites the ASCII byte at `i` with another ASCII byte, in place when nothing else holds the text, and keeps what
    /// is known of the text (still ASCII, marker unchanged). The caller checks `is_ascii`, `i < len` and `c < 128`.
    pub fn set_ascii_byte(&mut self, i: usize, c: u8) {
        debug_assert!(self.is_ascii() && i < self.0.s.len() && c < 128);
        let inner = Arc::make_mut(&mut self.0);
        let mut buf = [0u8; 4];
        inner.s.replace_range(i..=i, char::from(c).encode_utf8(&mut buf));
        // The mark is not ASCII, so the marker flag cannot change.
    }

    pub fn push(&mut self, c: char) {
        let mut b = [0u8; 4];
        self.push_str(c.encode_utf8(&mut b));
    }

    pub fn push_str(&mut self, more: &str) {
        let was_ascii = self.0.ascii.load(Relaxed);
        let was_marked = self.0.marked.load(Relaxed);
        let more_ascii = more.is_ascii();
        let more_marked = more.contains(crate::native::CALL_STACK_MARK);
        let s = self.make_mut();
        s.push_str(more);
        // What is known of the parts is known of the whole.
        if was_ascii == 1 && more_ascii {
            self.0.ascii.store(1, Relaxed);
        } else if was_ascii == 2 || !more_ascii {
            self.0.ascii.store(2, Relaxed);
        }
        if was_marked == 2 && !more_marked {
            self.0.marked.store(2, Relaxed);
        } else if was_marked == 1 || more_marked {
            self.0.marked.store(1, Relaxed);
        }
    }

    /// An owned string: the buffer itself when this is the only holder, else a copy.
    pub fn into_string(self) -> String {
        match Arc::try_unwrap(self.0) {
            Ok(inner) => inner.s,
            Err(shared) => shared.s.clone(),
        }
    }

    /// Whether both are the same buffer (a cheap equality check).
    pub fn ptr_eq(a: &Txt, b: &Txt) -> bool {
        Arc::ptr_eq(&a.0, &b.0)
    }
}

impl Deref for Txt {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0.s
    }
}

impl AsRef<str> for Txt {
    fn as_ref(&self) -> &str {
        &self.0.s
    }
}

impl Borrow<str> for Txt {
    fn borrow(&self) -> &str {
        &self.0.s
    }
}

impl fmt::Display for Txt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0.s, f)
    }
}

impl fmt::Debug for Txt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0.s, f)
    }
}

impl Default for Txt {
    fn default() -> Txt {
        Txt::new(String::new())
    }
}

impl From<String> for Txt {
    fn from(s: String) -> Txt {
        Txt::new(s)
    }
}

impl From<&str> for Txt {
    fn from(s: &str) -> Txt {
        Txt::new(s.to_string())
    }
}

impl From<std::sync::Arc<str>> for Txt {
    fn from(s: std::sync::Arc<str>) -> Txt {
        Txt::new(s.to_string())
    }
}

impl From<&std::sync::Arc<str>> for Txt {
    fn from(s: &std::sync::Arc<str>) -> Txt {
        Txt::new(s.to_string())
    }
}

impl From<&String> for Txt {
    fn from(s: &String) -> Txt {
        Txt::new(s.clone())
    }
}

impl From<Txt> for String {
    fn from(t: Txt) -> String {
        t.into_string()
    }
}

impl std::iter::FromIterator<char> for Txt {
    fn from_iter<I: IntoIterator<Item = char>>(iter: I) -> Txt {
        Txt::new(iter.into_iter().collect())
    }
}

impl PartialEq<Txt> for String {
    fn eq(&self, other: &Txt) -> bool {
        self.as_str() == other.as_str()
    }
}

impl PartialEq<Txt> for str {
    fn eq(&self, other: &Txt) -> bool {
        self == other.as_str()
    }
}

impl PartialEq<Txt> for &str {
    fn eq(&self, other: &Txt) -> bool {
        *self == other.as_str()
    }
}

impl PartialEq for Txt {
    fn eq(&self, other: &Txt) -> bool {
        Arc::ptr_eq(&self.0, &other.0) || self.0.s == other.0.s
    }
}
impl Eq for Txt {}

impl PartialEq<str> for Txt {
    fn eq(&self, other: &str) -> bool {
        self.0.s == other
    }
}
impl PartialEq<&str> for Txt {
    fn eq(&self, other: &&str) -> bool {
        self.0.s == *other
    }
}
impl PartialEq<String> for Txt {
    fn eq(&self, other: &String) -> bool {
        &self.0.s == other
    }
}

impl PartialOrd for Txt {
    fn partial_cmp(&self, other: &Txt) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Txt {
    fn cmp(&self, other: &Txt) -> std::cmp::Ordering {
        self.0.s.cmp(&other.0.s)
    }
}

impl Hash for Txt {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.s.hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Rows of the test data hold texts and are read from several threads (`SharedMat`).
    #[test]
    fn a_text_can_be_shared_between_threads() {
        fn needs<T: Send + Sync>() {}
        needs::<Txt>();
    }

    #[test]
    fn a_clone_shares_and_a_write_copies_only_when_shared() {
        let a = Txt::from("abc");
        let mut b = a.clone();
        assert!(Txt::ptr_eq(&a, &b));
        b.push_str("d");
        assert!(!Txt::ptr_eq(&a, &b));
        assert_eq!((a.as_str(), b.as_str()), ("abc", "abcd"));
        let p = b.as_str().as_ptr();
        b.push_str("e");
        // Not shared: the write went to the same string (it may move on growth, so only the content is checked).
        assert_eq!(b.as_str(), "abcde");
        let _ = p;
    }

    #[test]
    fn ascii_is_remembered_and_dropped_on_a_write() {
        let mut t = Txt::from("abc");
        assert!(t.is_ascii());
        t.push_str("ä");
        assert!(!t.is_ascii());
        assert_eq!(t.utf16_len(), 4);
        let mut u = Txt::from("abc");
        assert!(u.is_ascii());
        u.push_str("de");
        assert!(u.is_ascii());
    }
}
