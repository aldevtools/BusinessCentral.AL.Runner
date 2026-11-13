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
//!
//! A text that holds half of a surrogate pair (ADR-068): AL texts are UTF-16 and keep a lone half (`T[2] := 55296`,
//! corpus types/text-code-char/TextCharIndexAssignEdges.al:100-118), which a Rust `String` cannot hold. Such a text keeps
//! U+FFFD at each lone position in its string (one UTF-16 unit, as the half is, so lengths and positions stay right) and the
//! real halves in a side list. Halves that meet merge into the real character (`from_units`). Only the readers that are
//! known to match BC on such a text use [`Txt::audited_str`] or [`Txt::units`]; every other read (`Deref`, `as_str`, a
//! compare, `Display`) records its caller in a per-thread slot that the VM turns into a refusal ([`take_unaudited_read`]).

use std::borrow::Borrow;
use std::cell::Cell;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::panic::Location;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering::Relaxed};

struct Inner {
    s: String,
    /// 0 not known yet, 1 ASCII only, 2 not. (Atomic: rows of the test data are shared between threads, and
    /// the flags are only hints that any thread computes the same way.)
    ascii: AtomicU8,
    /// 0 not known yet, 1 holds a marker (`native::TEXT_MARKS`), 2 does not.
    marked: AtomicU8,
    /// The real units at the lone positions (UTF-16 index, unit), sorted; `None` for every text without a lone half.
    lone: Option<Box<[(u32, u16)]>>,
}

impl Clone for Inner {
    fn clone(&self) -> Inner {
        Inner {
            s: self.s.clone(),
            ascii: AtomicU8::new(self.ascii.load(Relaxed)),
            marked: AtomicU8::new(self.marked.load(Relaxed)),
            lone: self.lone.clone(),
        }
    }
}

// How many times a text was scanned for a marker (the first `has_marker` of a text the flag is not known
// of). A loop that builds a text must not scan the whole text each round; the test of the append counts this. Counted
// per thread, so tests that run in parallel in one binary do not add to each other's count.
thread_local! {
    static MARKER_SCANS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// The number of scans so far on this thread (see `MARKER_SCANS`).
pub fn marker_scans() -> u64 {
    MARKER_SCANS.with(|c| c.get())
}

// The first read of a text with a lone half by a reader that is not audited (ADR-068 item 3), since the VM last took it.
thread_local! {
    static UNAUDITED: Cell<Option<&'static Location<'static>>> = const { Cell::new(None) };
}

#[cold]
#[inline(never)]
fn note_unaudited(at: &'static Location<'static>) {
    UNAUDITED.with(|c| {
        if c.get().is_none() {
            c.set(Some(at));
        }
    });
}

/// The place of the first unaudited read of a text with a lone half on this thread since the last call, and clears it.
pub fn take_unaudited_read() -> Option<&'static Location<'static>> {
    UNAUDITED.with(|c| c.take())
}

/// Whether an unaudited read of a text with a lone half is waiting (see [`take_unaudited_read`]).
#[inline]
pub fn unaudited_read_pending() -> bool {
    UNAUDITED.with(|c| c.get().is_some())
}

/// `Err` with the refusal when an unaudited read of a text with a lone half is waiting (ADR-068 item 3): the VM calls it
/// when a builtin returns, at each statement and when a call from outside returns, so the read refuses instead of running
/// on U+FFFD.
#[inline]
pub fn check_unaudited() -> Result<(), crate::native::Signal> {
    if unaudited_read_pending() {
        return Err(unaudited_signal());
    }
    Ok(())
}

#[cold]
#[inline(never)]
fn unaudited_signal() -> crate::native::Signal {
    let at = take_unaudited_read().expect("pending");
    crate::native::Signal::unsupported(unaudited_reason(at))
}

/// The refusal for an unaudited read at `at` (ADR-068 item 3).
pub fn unaudited_reason(at: &Location<'_>) -> String {
    let file = at.file().rsplit_once("src/").map_or(at.file(), |(_, f)| f);
    format!(
        "a text with half of a surrogate pair read by {file}:{} (not pinned)",
        at.line()
    )
}

const REPLACEMENT: u16 = 0xFFFD;

fn is_surrogate(u: u16) -> bool {
    (0xD800..0xE000).contains(&u)
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
            lone: None,
        }))
    }

    /// The text of these UTF-16 units, exactly: a high and a low half next to each other are one character, a half that
    /// stands alone is kept (ADR-068).
    pub fn from_units(units: &[u16]) -> Txt {
        if let Ok(s) = String::from_utf16(units) {
            return Txt::new(s);
        }
        let mut s = String::with_capacity(units.len());
        let mut lone = Vec::new();
        let mut i = 0usize;
        while i < units.len() {
            let u = units[i];
            if (0xD800..0xDC00).contains(&u) && units.get(i + 1).is_some_and(|l| (0xDC00..0xE000).contains(l)) {
                let c = 0x10000 + ((u32::from(u) - 0xD800) << 10) + (u32::from(units[i + 1]) - 0xDC00);
                s.push(char::from_u32(c).expect("a valid pair"));
                i += 2;
                continue;
            }
            if is_surrogate(u) {
                s.push('\u{FFFD}');
                lone.push((i as u32, u));
            } else {
                s.push(char::from_u32(u32::from(u)).expect("not a surrogate"));
            }
            i += 1;
        }
        Txt(Arc::new(Inner {
            s,
            ascii: AtomicU8::new(2),
            marked: AtomicU8::new(0),
            lone: Some(lone.into_boxed_slice()),
        }))
    }

    /// A text of one UTF-16 unit, which may be half of a surrogate pair.
    pub fn from_unit(u: u16) -> Txt {
        Txt::from_units(&[u])
    }

    /// The exact UTF-16 units, the lone halves included.
    pub fn units(&self) -> Vec<u16> {
        let mut v: Vec<u16> = self.0.s.encode_utf16().collect();
        if let Some(lone) = &self.0.lone {
            for &(i, u) in lone.iter() {
                if let Some(slot) = v.get_mut(i as usize) {
                    debug_assert_eq!(*slot, REPLACEMENT);
                    *slot = u;
                }
            }
        }
        v
    }

    /// Whether the text holds half of a surrogate pair.
    #[inline]
    pub fn has_lone(&self) -> bool {
        self.0.lone.is_some()
    }

    /// The lone halves (UTF-16 position, unit), empty for a text without one.
    pub fn lone_units(&self) -> &[(u32, u16)] {
        self.0.lone.as_deref().unwrap_or(&[])
    }

    /// The string with U+FFFD at each lone position, for a reader that is known to match BC on such a text (its pin in a
    /// comment at the call). Every other reader goes through `Deref`, which refuses on a lone half (ADR-068 item 3).
    #[inline]
    pub fn audited_str(&self) -> &str {
        &self.0.s
    }

    #[inline]
    #[track_caller]
    pub fn as_str(&self) -> &str {
        if self.0.lone.is_some() {
            note_unaudited(Location::caller());
        }
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

    /// Whether the text holds a marker (one of `native::TEXT_MARKS`): no text function reads such a text. Computed once for this string.
    pub fn has_marker(&self) -> bool {
        match self.0.marked.load(Relaxed) {
            1 => true,
            2 => false,
            _ => {
                MARKER_SCANS.with(|c| c.set(c.get() + 1));
                let m = self.0.s.contains(crate::native::TEXT_MARKS);
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

    /// The string for writing: copied first when another value shares it. The remembered ASCII flag is dropped. A writer
    /// of a text with a lone half is not audited: it is recorded (ADR-068 item 3) and the halves are dropped.
    #[track_caller]
    pub fn make_mut(&mut self) -> &mut String {
        if self.0.lone.is_some() {
            note_unaudited(Location::caller());
        }
        let inner = Arc::make_mut(&mut self.0);
        inner.ascii.store(0, Relaxed);
        inner.marked.store(0, Relaxed);
        inner.lone = None;
        &mut inner.s
    }

    /// Appends `more`, exactly: a lone high half at the end of this text and a lone low half at the start of `more` become
    /// one character (corpus text/text-methods/ForeachText.al, Foreach_Text_SurrogatePair).
    pub fn push_txt(&mut self, more: &Txt) {
        if self.0.lone.is_none() && more.0.lone.is_none() {
            self.push_str(&more.0.s);
            return;
        }
        let mut u = self.units();
        u.extend(more.units());
        *self = Txt::from_units(&u);
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
        let more_marked = more.contains(crate::native::TEXT_MARKS);
        // Valid text after the lone halves leaves their positions as they are.
        let inner = Arc::make_mut(&mut self.0);
        inner.ascii.store(0, Relaxed);
        inner.marked.store(0, Relaxed);
        inner.s.push_str(more);
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
    #[track_caller]
    pub fn into_string(self) -> String {
        if self.0.lone.is_some() {
            note_unaudited(Location::caller());
        }
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
    #[inline]
    #[track_caller]
    fn deref(&self) -> &str {
        if self.0.lone.is_some() {
            note_unaudited(Location::caller());
        }
        &self.0.s
    }
}

impl AsRef<str> for Txt {
    #[track_caller]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Borrow<str> for Txt {
    #[track_caller]
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for Txt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.as_str(), f)
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

// A text with a lone half equals no Rust string (none can hold the half): `false` without a refusal is exact.
impl PartialEq<Txt> for String {
    fn eq(&self, other: &Txt) -> bool {
        other.0.lone.is_none() && self.as_str() == other.0.s
    }
}

impl PartialEq<Txt> for str {
    fn eq(&self, other: &Txt) -> bool {
        other.0.lone.is_none() && self == other.0.s
    }
}

impl PartialEq<Txt> for &str {
    fn eq(&self, other: &Txt) -> bool {
        other.0.lone.is_none() && *self == other.0.s
    }
}

/// Equal by the exact units: the strings and the lone halves.
impl PartialEq for Txt {
    fn eq(&self, other: &Txt) -> bool {
        Arc::ptr_eq(&self.0, &other.0) || (self.0.s == other.0.s && self.0.lone == other.0.lone)
    }
}
impl Eq for Txt {}

impl PartialEq<str> for Txt {
    fn eq(&self, other: &str) -> bool {
        self.0.lone.is_none() && self.0.s == other
    }
}
impl PartialEq<&str> for Txt {
    fn eq(&self, other: &&str) -> bool {
        self.0.lone.is_none() && self.0.s == *other
    }
}
impl PartialEq<String> for Txt {
    fn eq(&self, other: &String) -> bool {
        self.0.lone.is_none() && &self.0.s == other
    }
}

impl PartialOrd for Txt {
    fn partial_cmp(&self, other: &Txt) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Txt {
    #[track_caller]
    fn cmp(&self, other: &Txt) -> std::cmp::Ordering {
        if self.0.lone.is_some() || other.0.lone.is_some() {
            note_unaudited(Location::caller());
        }
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

    // ADR-068: a lone half keeps its unit and its position (corpus types/text-code-char/TextCharIndexAssignEdges.al:116).
    #[test]
    fn a_lone_half_keeps_its_unit_and_position() {
        let t = Txt::from_units(&[0x61, 0xD800, 0x63]);
        assert!(t.has_lone());
        assert_eq!(t.utf16_len(), 3);
        assert_eq!(t.units(), vec![0x61, 0xD800, 0x63]);
        assert_eq!(t.audited_str(), "a\u{FFFD}c");
        assert_eq!(t.lone_units(), &[(1, 0xD800)]);
        // A pair and a half: the half after the pair is at unit 2.
        let u = Txt::from_units(&[0xD83D, 0xDE00, 0xDC00]);
        assert_eq!(u.lone_units(), &[(2, 0xDC00)]);
        assert_eq!(u.units(), vec![0xD83D, 0xDE00, 0xDC00]);
    }

    // Halves that meet merge into the character (corpus text/text-methods/ForeachText.al, Foreach_Text_SurrogatePair).
    #[test]
    fn halves_that_meet_merge() {
        let mut t = Txt::from_unit(0xD83D);
        t.push_txt(&Txt::from_unit(0xDE00));
        assert!(!t.has_lone());
        assert_eq!(t.audited_str(), "\u{1F600}");
        assert_eq!(Txt::from_units(&[0x61]), Txt::from("a"));
    }

    // ADR-068 item 3: a read that is not audited is recorded, an audited one is not; valid text records nothing.
    #[test]
    fn an_unaudited_read_of_a_lone_half_is_recorded() {
        let _ = take_unaudited_read();
        let ok = Txt::from("abc");
        let _ = ok.len();
        assert!(take_unaudited_read().is_none());
        let t = Txt::from_unit(0xD800);
        let _ = t.audited_str();
        let _ = t.units();
        assert!(take_unaudited_read().is_none());
        let _ = t.len();
        let at = take_unaudited_read().expect("recorded");
        assert!(at.file().ends_with("text.rs"));
        assert!(take_unaudited_read().is_none());
        // Equality is exact, and no Rust string equals a text with a lone half.
        assert_ne!(t, Txt::from("\u{FFFD}"));
        assert!(t != *"\u{FFFD}");
        assert!(take_unaudited_read().is_none());
    }
}
