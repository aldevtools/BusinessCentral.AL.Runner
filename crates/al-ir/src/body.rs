//! The statements of a routine, decoded on first use (ADR-022, "Measured 2026-10-21").
//!
//! A compact form (postcard, `Serializer::is_human_readable` false) stores a body as a byte string of its own. Reading
//! such a form keeps the bytes and decodes them the first time the body is read. What link would do to the statements
//! (rewrite the references, shift the source positions) is kept as a list of [`BodyFixup`]s and applied, in order, right
//! after the decode. The statements a reader sees are the same as with an eager decode; only when the work is done
//! differs. A human-readable form (JSON) is the plain statement list.

use crate::Stmt;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::sync::{Arc, OnceLock};

/// Work on the statements of a body that is deferred until the body is decoded.
pub trait BodyFixup: Send + Sync {
    /// Applies the work. An error means the stored body does not fit the tables it was linked with: a damaged entry.
    fn apply(&self, body: &mut Vec<Stmt>) -> Result<(), String>;
}

struct Pending {
    bytes: Box<[u8]>,
    fixups: Vec<Arc<dyn BodyFixup>>,
}

/// The statements of a routine. Reads through `Deref` and `DerefMut` as the `Vec<Stmt>` it holds.
pub struct Body {
    done: OnceLock<Vec<Stmt>>,
    /// The bytes and the deferred work while the body is not decoded. Kept after a decode through `&self` (it cannot be
    /// dropped there); dropped by every `&mut self` access.
    pending: Option<Box<Pending>>,
}

impl Body {
    pub fn new(stmts: Vec<Stmt>) -> Body {
        let done = OnceLock::new();
        let _ = done.set(stmts);
        Body { done, pending: None }
    }

    /// Whether the statements are decoded (always, except for a body read from a compact form and not yet used).
    pub fn is_loaded(&self) -> bool {
        self.done.get().is_some()
    }

    /// The statements, decoded now if they are not yet.
    pub fn stmts(&self) -> &Vec<Stmt> {
        self.done.get_or_init(|| {
            let p = self.pending.as_ref().expect("a body is either decoded or pending");
            decode(p)
        })
    }

    /// The statements, for one read that keeps nothing: a body not decoded yet is decoded for this call and the
    /// statements are dropped after it, so the body stays compact. For a static walk over many routines that runs none
    /// of them; a reader that comes back to the body uses [`Body::stmts`].
    pub fn read<R>(&self, f: impl FnOnce(&[Stmt]) -> R) -> R {
        match self.done.get() {
            Some(stmts) => f(stmts),
            None => f(&decode(
                self.pending.as_ref().expect("a body is either decoded or pending"),
            )),
        }
    }

    /// The statements for change, decoded now if they are not yet.
    pub fn stmts_mut(&mut self) -> &mut Vec<Stmt> {
        if self.done.get().is_none() {
            let p = self.pending.take().expect("a body is either decoded or pending");
            let _ = self.done.set(decode(&p));
        } else {
            self.pending = None;
        }
        self.done.get_mut().expect("decoded above")
    }

    /// Keeps the statements as their compact form (the bytes a compile cache stores) and drops the decoded list: the next
    /// read decodes them again. Most lowered bodies are never read by a run; compact, a body takes about a sixth of the
    /// memory. A body with work pending, or not decoded, is left as it is.
    pub fn compact(&mut self) {
        if self.pending.is_some() {
            // Decoded through `&self`: the bytes and the work still give the same statements.
            self.done.take();
            return;
        }
        let Some(stmts) = self.done.take() else {
            return;
        };
        let bytes = postcard::to_stdvec(&stmts)
            .unwrap_or_else(|e| panic!("internal error: a routine body does not encode: {e}"));
        self.pending = Some(Box::new(Pending {
            bytes: bytes.into_boxed_slice(),
            fixups: Vec::new(),
        }));
    }

    pub fn into_stmts(mut self) -> Vec<Stmt> {
        std::mem::take(self.stmts_mut())
    }

    /// Applies `fixup` now when the body is decoded, or keeps it for the decode when it is not.
    pub fn fix(&mut self, fixup: Arc<dyn BodyFixup>) -> Result<(), String> {
        match (self.done.get_mut(), self.pending.as_mut()) {
            (None, Some(p)) => {
                p.fixups.push(fixup);
                Ok(())
            }
            (Some(stmts), _) => {
                self.pending = None;
                fixup.apply(stmts)
            }
            (None, None) => unreachable!("a body is either decoded or pending"),
        }
    }
}

fn decode(p: &Pending) -> Vec<Stmt> {
    let mut stmts: Vec<Stmt> = postcard::from_bytes(&p.bytes)
        .unwrap_or_else(|e| panic!("internal error: a stored routine body does not decode: {e}"));
    for f in &p.fixups {
        f.apply(&mut stmts)
            .unwrap_or_else(|e| panic!("internal error: a stored routine body does not link: {e}"));
    }
    stmts
}

impl Default for Body {
    fn default() -> Body {
        Body::new(Vec::new())
    }
}

impl From<Vec<Stmt>> for Body {
    fn from(stmts: Vec<Stmt>) -> Body {
        Body::new(stmts)
    }
}

impl std::ops::Deref for Body {
    type Target = Vec<Stmt>;
    fn deref(&self) -> &Vec<Stmt> {
        self.stmts()
    }
}

impl std::ops::DerefMut for Body {
    fn deref_mut(&mut self) -> &mut Vec<Stmt> {
        self.stmts_mut()
    }
}

impl<'a> IntoIterator for &'a Body {
    type Item = &'a Stmt;
    type IntoIter = std::slice::Iter<'a, Stmt>;
    fn into_iter(self) -> Self::IntoIter {
        self.stmts().iter()
    }
}

impl<'a> IntoIterator for &'a mut Body {
    type Item = &'a mut Stmt;
    type IntoIter = std::slice::IterMut<'a, Stmt>;
    fn into_iter(self) -> Self::IntoIter {
        self.stmts_mut().iter_mut()
    }
}

impl Clone for Body {
    fn clone(&self) -> Body {
        match (self.done.get(), self.pending.as_ref()) {
            (Some(stmts), _) => Body::new(stmts.clone()),
            (None, Some(p)) => Body {
                done: OnceLock::new(),
                pending: Some(Box::new(Pending {
                    bytes: p.bytes.clone(),
                    fixups: p.fixups.clone(),
                })),
            },
            (None, None) => unreachable!("a body is either decoded or pending"),
        }
    }
}

impl PartialEq for Body {
    fn eq(&self, other: &Body) -> bool {
        self.stmts() == other.stmts()
    }
}

impl std::fmt::Debug for Body {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.stmts().fmt(f)
    }
}

impl Serialize for Body {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        if s.is_human_readable() {
            return self.stmts().serialize(s);
        }
        match (self.done.get(), self.pending.as_ref()) {
            (None, Some(p)) if p.fixups.is_empty() => s.serialize_bytes(&p.bytes),
            _ => {
                let bytes = postcard::to_stdvec(self.stmts()).map_err(serde::ser::Error::custom)?;
                s.serialize_bytes(&bytes)
            }
        }
    }
}

impl<'de> Deserialize<'de> for Body {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Body, D::Error> {
        if d.is_human_readable() {
            return Vec::<Stmt>::deserialize(d).map(Body::new);
        }
        struct Bytes;
        impl<'de> serde::de::Visitor<'de> for Bytes {
            type Value = Box<[u8]>;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("the bytes of a routine body")
            }
            fn visit_bytes<E: serde::de::Error>(self, v: &[u8]) -> Result<Box<[u8]>, E> {
                Ok(v.into())
            }
            fn visit_byte_buf<E: serde::de::Error>(self, v: Vec<u8>) -> Result<Box<[u8]>, E> {
                Ok(v.into_boxed_slice())
            }
        }
        let bytes = d.deserialize_bytes(Bytes)?;
        Ok(Body {
            done: OnceLock::new(),
            pending: Some(Box::new(Pending {
                bytes,
                fixups: Vec::new(),
            })),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Expr, Stmt};

    fn stmts() -> Vec<Stmt> {
        vec![
            Stmt::Pos(3),
            Stmt::If(
                Expr::Const(crate::Const::Boolean(true)),
                vec![Stmt::Pos(4), Stmt::Break],
                vec![Stmt::Pos(9)],
            ),
            Stmt::Exit(None),
        ]
    }

    struct Shift(u32);
    impl BodyFixup for Shift {
        fn apply(&self, body: &mut Vec<Stmt>) -> Result<(), String> {
            crate::shift_stmts(body, self.0);
            Ok(())
        }
    }

    #[test]
    fn a_body_read_from_the_compact_form_is_decoded_on_first_read_with_the_same_statements() {
        let bytes = postcard::to_stdvec(&Body::new(stmts())).unwrap();
        let read: Body = postcard::from_bytes(&bytes).unwrap();
        assert!(!read.is_loaded(), "reading the compact form decodes nothing");
        assert_eq!(*read, stmts());
        assert!(read.is_loaded());
    }

    #[test]
    fn deferred_work_is_applied_in_order_at_the_decode_and_a_decoded_body_takes_it_at_once() {
        let bytes = postcard::to_stdvec(&Body::new(stmts())).unwrap();
        let mut lazy: Body = postcard::from_bytes(&bytes).unwrap();
        lazy.fix(Arc::new(Shift(10))).unwrap();
        lazy.fix(Arc::new(Shift(5))).unwrap();
        assert!(!lazy.is_loaded(), "the work waits for the decode");
        let mut eager = Body::new(stmts());
        crate::shift_stmts(&mut eager, 15);
        assert_eq!(lazy, eager);
        // Serialized again, the pending work is in the bytes.
        let again: Body = postcard::from_bytes(&postcard::to_stdvec(&lazy.clone()).unwrap()).unwrap();
        assert_eq!(again, eager);
        let mut decoded = Body::new(stmts());
        decoded.fix(Arc::new(Shift(15))).unwrap();
        assert_eq!(decoded, eager);
    }

    /// A lowered body is kept compact until something reads it (`al_sema` `lower_core`): most bodies of a run are
    /// never read again after the lowering.
    #[test]
    fn a_compacted_body_holds_bytes_and_reads_the_same_statements() {
        let mut body = Body::new(stmts());
        body.compact();
        assert!(!body.is_loaded(), "a compacted body holds its bytes only");
        assert_eq!(
            postcard::to_stdvec(&body).unwrap(),
            postcard::to_stdvec(&Body::new(stmts())).unwrap()
        );
        body.fix(Arc::new(Shift(2))).unwrap();
        let mut shifted = stmts();
        crate::shift_stmts(&mut shifted, 2);
        assert_eq!(*body, shifted);
    }

    /// A static walk over many routines that runs none of them (al-testrun `written.rs`) reads a compact body without
    /// keeping it decoded.
    #[test]
    fn a_read_that_keeps_nothing_leaves_a_compact_body_compact() {
        let mut body = Body::new(stmts());
        body.compact();
        assert_eq!(body.read(|s| s.to_vec()), stmts());
        assert!(!body.is_loaded(), "the read kept nothing");
        let decoded = Body::new(stmts());
        assert_eq!(decoded.read(<[Stmt]>::len), 3);
    }

    #[test]
    fn the_human_readable_form_is_the_plain_statement_list() {
        let json = serde_json::to_string(&Body::new(stmts())).unwrap();
        assert_eq!(json, serde_json::to_string(&stmts()).unwrap());
        let back: Body = serde_json::from_str(&json).unwrap();
        assert!(back.is_loaded());
        assert_eq!(*back, stmts());
    }
}
