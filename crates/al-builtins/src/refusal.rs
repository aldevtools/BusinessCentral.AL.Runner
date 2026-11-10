//! Typed refusals (ADR-071): a refusal carries the key of the refused shape and the text for the user.
//!
//! - [`Refusal`] is the payload of `Signal::Unsupported` and of the per-routine lowering lists. Free text converts
//!   into it with the key [`LEGACY`] (`From<String>`, `From<&str>`); it reads, prints and stores exactly as the
//!   `String` did. A migrated site builds a keyed one with [`refuse!`](crate::refuse).
//! - [`refuse!`](crate::refuse) formats its arguments through [`Describe`], never through `Debug`: each argument is
//!   wrapped in [`Shown`], which has `Display` and no `Debug`, and a named placeholder (`{x}`, read from the scope
//!   past the wrapper) is rejected at compile time by [`positional_only`].
//! - Keys are registered, one file per key, in a list per crate that refuses (owner decision 2026-11-10, recorded in
//!   ADR-071's implementation notes). This crate's list is `src/refusals/<key>.rs`, collected by `build.rs` into one
//!   [`refusal_keys!`](crate::refusal_keys) call that makes [`keys`] and [`ALL`]. One test in `al-test-cli`
//!   (`tests/refusal_keys.rs`) checks all lists together and the generated table of `docs/probes-needed.md`.
//!
//! The text of a refusal is its `Display` (and `Deref<Target = str>`, a shim for readers written against the `String`
//! that goes away with `From<String>` in the last migration step).

use crate::describe::Describe;
use std::borrow::Cow;
use std::fmt;

/// What a refused shape waits for (ADR-071 decision 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Probe {
    /// No probe written yet.
    Wanted,
    /// The corpus probe spec, one sentence.
    Spec(&'static str),
    /// An issue, with its plain description.
    Issue {
        repo: Repo,
        number: u32,
        what: &'static str,
    },
}

/// The repository an issue of [`Probe::Issue`] is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Repo {
    Runner,
    Corpus,
}

impl Probe {
    /// One line for the generated table.
    pub fn text(&self) -> String {
        match self {
            Probe::Wanted => "wanted (no probe written yet)".into(),
            Probe::Spec(s) => (*s).into(),
            Probe::Issue { repo, number, what } => {
                let repo = match repo {
                    Repo::Runner => "runner",
                    Repo::Corpus => "corpus",
                };
                format!("{repo} issue {number}: {what}")
            }
        }
    }
}

/// One registry entry.
#[derive(Debug)]
pub struct KeyInfo {
    /// The name of its constant in `keys` (for the test that finds its uses).
    pub name: &'static str,
    /// The key: dotted, lower case, never reworded. The part before the first dot is its area.
    pub key: &'static str,
    pub probe: Probe,
}

/// A registered key. Only the registry macro makes one.
#[derive(Debug, Clone, Copy)]
pub struct RefusalKey(&'static KeyInfo);

impl RefusalKey {
    #[doc(hidden)]
    pub const fn new(info: &'static KeyInfo) -> RefusalKey {
        RefusalKey(info)
    }

    pub fn info(self) -> &'static KeyInfo {
        self.0
    }

    pub fn key(self) -> &'static str {
        self.0.key
    }
}

/// The key of a refusal made from free text.
pub const LEGACY: &str = "legacy";

/// The areas a key may start with: the corpus folder names in key form, plus `dotnet`, `builtin`, `compile` and
/// `runner` (plan of ADR-071 step 1, A13).
pub const AREAS: &[&str] = &[
    "record",
    "recordref",
    "page",
    "testpage",
    "report",
    "xmlport",
    "xml",
    "json",
    "http",
    "query",
    "stream",
    "format",
    "text",
    "type",
    "event",
    "permission",
    "session",
    "table",
    "codeunit",
    "collection",
    "controladdin",
    "error",
    "ui",
    "install",
    "language",
    "localization",
    "metadata",
    "profile",
    "testframework",
    "dotnet",
    "builtin",
    "compile",
    "runner",
];

/// A refusal: the key of the refused shape and the text for the user. The size of a `String`, so `Signal` does not
/// grow; free text is stored as its `String`, with no allocation more than before.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Refusal(Repr);

#[derive(Clone, PartialEq, Eq, Hash)]
enum Repr {
    /// Free text: the key [`LEGACY`].
    Legacy(String),
    Keyed(Box<Data>),
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct Data {
    key: Cow<'static, str>,
    reason: String,
}

impl Refusal {
    /// A refusal with a registered key; what `refuse!` calls.
    pub fn keyed(key: RefusalKey, reason: String) -> Refusal {
        Refusal(Repr::Keyed(Box::new(Data {
            key: Cow::Borrowed(key.key()),
            reason,
        })))
    }

    /// The key; [`LEGACY`] for free text.
    pub fn key(&self) -> &str {
        match &self.0 {
            Repr::Legacy(_) => LEGACY,
            Repr::Keyed(d) => &d.key,
        }
    }

    pub fn is_legacy(&self) -> bool {
        matches!(self.0, Repr::Legacy(_))
    }

    /// The key up to its first dot.
    pub fn area(&self) -> &str {
        let k = self.key();
        k.split_once('.').map_or(k, |(a, _)| a)
    }

    /// The text for the user.
    pub fn reason(&self) -> &str {
        match &self.0 {
            Repr::Legacy(s) => s,
            Repr::Keyed(d) => &d.reason,
        }
    }

    pub fn into_reason(self) -> String {
        match self.0 {
            Repr::Legacy(s) => s,
            Repr::Keyed(d) => d.reason,
        }
    }

    /// The same key with a new text (a site prefix such as `OnRun: `).
    pub fn map_reason(mut self, f: impl FnOnce(String) -> String) -> Refusal {
        let reason = match &mut self.0 {
            Repr::Legacy(s) => s,
            Repr::Keyed(d) => &mut d.reason,
        };
        *reason = f(std::mem::take(reason));
        self
    }
}

/// Free text: the key `legacy` (ADR-071 decision 5). Deleted in the last migration step.
impl<S: Into<String>> From<S> for Refusal {
    fn from(text: S) -> Refusal {
        Refusal(Repr::Legacy(text.into()))
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.reason())
    }
}

/// A legacy refusal prints exactly as its `String` did (tests compare the `Debug` of `Signal` and `Outcome`); a keyed
/// one as `key: "reason"`.
impl fmt::Debug for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.is_legacy() {
            f.write_str(self.key())?;
            f.write_str(": ")?;
        }
        fmt::Debug::fmt(self.reason(), f)
    }
}

/// The text. A shim for readers written against the `String`; deleted with `From<String>`.
impl std::ops::Deref for Refusal {
    type Target = str;
    fn deref(&self) -> &str {
        self.reason()
    }
}

impl PartialEq<str> for Refusal {
    fn eq(&self, other: &str) -> bool {
        self.reason() == other
    }
}

impl PartialEq<&str> for Refusal {
    fn eq(&self, other: &&str) -> bool {
        self.reason() == *other
    }
}

impl PartialEq<String> for Refusal {
    fn eq(&self, other: &String) -> bool {
        self.reason() == other
    }
}

/// In a human-readable format (JSON) a legacy refusal is its plain text, so a dump of the compile reads as it did
/// with `String`; a keyed one is `{"key": .., "reason": ..}`. Otherwise (postcard, the compile cache) the pair
/// `(key, reason)`.
impl serde::Serialize for Refusal {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::{SerializeStruct, SerializeTuple};
        if s.is_human_readable() {
            if self.is_legacy() {
                return s.serialize_str(self.reason());
            }
            let mut st = s.serialize_struct("Refusal", 2)?;
            st.serialize_field("key", self.key())?;
            st.serialize_field("reason", self.reason())?;
            st.end()
        } else {
            let mut t = s.serialize_tuple(2)?;
            t.serialize_element(self.key())?;
            t.serialize_element(self.reason())?;
            t.end()
        }
    }
}

impl<'de> serde::Deserialize<'de> for Refusal {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Refusal, D::Error> {
        if d.is_human_readable() {
            d.deserialize_any(RefusalVisitor)
        } else {
            d.deserialize_tuple(2, RefusalVisitor)
        }
    }
}

struct RefusalVisitor;

fn stored(key: String, reason: String) -> Refusal {
    if key == LEGACY {
        return Refusal(Repr::Legacy(reason));
    }
    Refusal(Repr::Keyed(Box::new(Data {
        key: Cow::Owned(key),
        reason,
    })))
}

impl<'de> serde::de::Visitor<'de> for RefusalVisitor {
    type Value = Refusal;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a refusal: its text, or its key and text")
    }

    fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Refusal, E> {
        Ok(Refusal::from(v))
    }

    fn visit_string<E: serde::de::Error>(self, v: String) -> Result<Refusal, E> {
        Ok(Refusal::from(v))
    }

    fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<Refusal, A::Error> {
        use serde::de::Error;
        let key: String = seq.next_element()?.ok_or_else(|| A::Error::invalid_length(0, &self))?;
        let reason: String = seq.next_element()?.ok_or_else(|| A::Error::invalid_length(1, &self))?;
        Ok(stored(key, reason))
    }

    fn visit_map<A: serde::de::MapAccess<'de>>(self, mut map: A) -> Result<Refusal, A::Error> {
        use serde::de::Error;
        let (mut key, mut reason) = (None, None);
        while let Some(k) = map.next_key::<String>()? {
            match k.as_str() {
                "key" => key = Some(map.next_value::<String>()?),
                "reason" => reason = Some(map.next_value::<String>()?),
                _ => {
                    map.next_value::<serde::de::IgnoredAny>()?;
                }
            }
        }
        let key = key.ok_or_else(|| A::Error::missing_field("key"))?;
        let reason = reason.ok_or_else(|| A::Error::missing_field("reason"))?;
        Ok(stored(key, reason))
    }
}

/// An argument of [`refuse!`](crate::refuse): `Display` through [`Describe`]. It has no `Debug`, so `{:?}` on an
/// argument does not compile.
pub struct Shown<'a, T: ?Sized>(pub &'a T);

impl<T: Describe + ?Sized> fmt::Display for Shown<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad(&self.0.describe())
    }
}

/// Whether every placeholder of `fmt` is positional (`{}`, `{0}`, `{:>5}`); `{{` and `}}` are escapes. A named
/// placeholder (`{x}`, `{x:?}`) would be read from the scope by `format!` and skip [`Shown`].
pub const fn positional_only(fmt: &str) -> bool {
    let b = fmt.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'{' {
            if i + 1 < b.len() && b[i + 1] == b'{' {
                i += 2;
                continue;
            }
            // The argument part: up to `:` or `}`; it must be empty or digits.
            i += 1;
            while i < b.len() && b[i] != b':' && b[i] != b'}' {
                if !b[i].is_ascii_digit() {
                    return false;
                }
                i += 1;
            }
            while i < b.len() && b[i] != b'}' {
                i += 1;
            }
        }
        i += 1;
    }
    true
}

/// A keyed refusal: `refuse!(keys::NAME, "text {} and {}", a, b)`. Each argument is formatted through `Describe`.
///
/// ```
/// # use al_builtins::refusal::{KeyInfo, Probe, RefusalKey};
/// # const KEY: RefusalKey = RefusalKey::new(&KeyInfo { name: "KEY", key: "xml.doc.example", probe: Probe::Wanted });
/// let v = al_builtins::value::Value::Integer(5);
/// let r = al_builtins::refuse!(KEY, "assigning {} to {}", v, "a Code[10]");
/// assert_eq!(r.reason(), "assigning Integer 5 to a Code[10]");
/// ```
///
/// A debug format of an argument does not compile:
///
/// ```compile_fail
/// # use al_builtins::refusal::{KeyInfo, Probe, RefusalKey};
/// # const KEY: RefusalKey = RefusalKey::new(&KeyInfo { name: "KEY", key: "xml.doc.example", probe: Probe::Wanted });
/// let v = al_builtins::value::Value::Integer(5);
/// let r = al_builtins::refuse!(KEY, "assigning {:?} to {}", v, "a Code[10]");
/// ```
///
/// Nor does an argument that has only `Debug`:
///
/// ```compile_fail
/// # use al_builtins::refusal::{KeyInfo, Probe, RefusalKey};
/// # const KEY: RefusalKey = RefusalKey::new(&KeyInfo { name: "KEY", key: "xml.doc.example", probe: Probe::Wanted });
/// let v = std::time::Duration::from_secs(5);
/// let r = al_builtins::refuse!(KEY, "assigning {} to {}", v, "a Code[10]");
/// ```
///
/// Nor a named placeholder, which `format!` would read from the scope past `Describe`:
///
/// ```compile_fail
/// # use al_builtins::refusal::{KeyInfo, Probe, RefusalKey};
/// # const KEY: RefusalKey = RefusalKey::new(&KeyInfo { name: "KEY", key: "xml.doc.example", probe: Probe::Wanted });
/// let v = 5; // `i32` has `Display`: only the check of the placeholders stops this
/// let r = al_builtins::refuse!(KEY, "assigning {v} to {}", "a Code[10]");
/// ```
#[macro_export]
macro_rules! refuse {
    ($key:expr, $fmt:literal $(, $arg:expr)* $(,)?) => {{
        const _: () = assert!(
            $crate::refusal::positional_only($fmt),
            "refuse!: a named placeholder is read from the scope and skips Describe; pass the value as an argument"
        );
        $crate::refusal::Refusal::keyed($key, format!($fmt $(, $crate::refusal::Shown(&$arg))*))
    }};
}

/// One crate's list of refusal keys: `pub mod keys` with a constant per entry, and `ALL`. Each crate that refuses
/// calls it once with the entries its `build.rs` collected from `src/refusals/<key>.rs`.
#[macro_export]
macro_rules! refusal_keys {
    ($( $(#[$m:meta])* $name:ident => { key: $key:literal, probe: $probe:expr $(,)? } ),* $(,)?) => {
        /// The registered refusal keys of this crate, one constant per file in `src/refusals/`.
        pub mod keys {
            #[allow(unused_imports)]
            use $crate::refusal::{KeyInfo, Probe::*, RefusalKey, Repo::*};
            $(
                $(#[$m])*
                pub const $name: RefusalKey = RefusalKey::new(&KeyInfo { name: stringify!($name), key: $key, probe: $probe });
            )*
        }
        /// Every key of this crate's list, in the order of the file names.
        pub const ALL: &[$crate::refusal::RefusalKey] = &[ $( keys::$name ),* ];
    };
}

include!(concat!(env!("OUT_DIR"), "/refusal_registry.rs"));

#[cfg(test)]
mod tests;
