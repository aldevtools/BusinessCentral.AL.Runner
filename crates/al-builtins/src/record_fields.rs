//! A record's field buffer (`RecordState::fields`): the field values in the table's field order. Its length never changes
//! after the record is made, so it reads and writes as a slice (`[Value]`), never as a growable vector.
//!
//! The buffer is copy-on-write (PERF-22): a fresh record of a table whose initial values are all plain values shares them
//! (`Shared`) until something takes `&mut` to them, which copies them first. Every other buffer is `Owned`.
//!
//! The one invariant: a `Shared` slice is never written and holds only plain values, never a value with identity or
//! interior state (a Blob buffer, a RecordId, a List, ...). Al-vm `Session::initial_fields` is the only maker. So a Blob's
//! holder count (`RecordState::blob_copies_of`, the `Arc::strong_count` rules) sees the same holders as with a copy.

use crate::value::Value;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

/// The field values of a record, in the table's field order.
#[derive(Clone)]
pub enum Fields {
    /// A buffer of the record's own. Its clone is a deep copy.
    Owned(Vec<Value>),
    /// Values shared with other buffers, read only; the first mutable access copies them into `Owned`. Its clone shares.
    Shared(Rc<[Value]>),
}

impl Fields {
    /// A buffer that shares `values` (a table's initial values, plain values only) until its first write.
    pub fn shared(values: Rc<[Value]>) -> Fields {
        Fields::Shared(values)
    }

    /// Whether the buffer still shares its values with another (copied on the first write).
    pub fn is_shared(&self) -> bool {
        matches!(self, Fields::Shared(_))
    }

    /// The values as a vector of their own.
    pub fn into_vec(self) -> Vec<Value> {
        match self {
            Fields::Owned(v) => v,
            Fields::Shared(rc) => rc.to_vec(),
        }
    }
}

impl Default for Fields {
    fn default() -> Fields {
        Fields::Owned(Vec::new())
    }
}

impl From<Vec<Value>> for Fields {
    fn from(v: Vec<Value>) -> Fields {
        Fields::Owned(v)
    }
}

impl From<Fields> for Vec<Value> {
    fn from(f: Fields) -> Vec<Value> {
        f.into_vec()
    }
}

impl Deref for Fields {
    type Target = [Value];
    #[inline]
    fn deref(&self) -> &[Value] {
        match self {
            Fields::Owned(v) => v,
            Fields::Shared(rc) => rc,
        }
    }
}

/// Every `&mut` to the values (indexing, `get_mut`, `iter_mut`, `swap`, `mem::replace` of an element) comes through here, so
/// a shared buffer is copied before the first write and the other holders keep their values.
impl DerefMut for Fields {
    #[inline]
    fn deref_mut(&mut self) -> &mut [Value] {
        if let Fields::Shared(rc) = self {
            *self = Fields::Owned(rc.to_vec());
        }
        match self {
            Fields::Owned(v) => v,
            Fields::Shared(_) => unreachable!("copied above"),
        }
    }
}

/// Prints the values alone, as the `Vec<Value>` this type replaced did.
impl std::fmt::Debug for Fields {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <[Value] as std::fmt::Debug>::fmt(self, f)
    }
}

impl IntoIterator for Fields {
    type Item = Value;
    type IntoIter = std::vec::IntoIter<Value>;
    fn into_iter(self) -> Self::IntoIter {
        self.into_vec().into_iter()
    }
}

impl<'a> IntoIterator for &'a Fields {
    type Item = &'a Value;
    type IntoIter = std::slice::Iter<'a, Value>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a> IntoIterator for &'a mut Fields {
    type Item = &'a mut Value;
    type IntoIter = std::slice::IterMut<'a, Value>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

#[cfg(test)]
mod tests;
