//! A record's field buffer (`RecordState::fields`): the field values in the table's field order. Its length never changes
//! after the record is made, so it reads and writes as a slice (`[Value]`), never as a growable vector.
//!
//! PERF-22 (`/coord/handoff/PERF-22/design.md`) makes this type copy-on-write: a fresh record of a table whose initial values
//! are all plain values will share them until its first write. This step is the inert type change only: every buffer is
//! `Owned`, exactly the `Vec<Value>` it was before, so no value and no Blob holder count changes. The shared arm comes with
//! the step that turns sharing on (design section 3, commit 3).

use crate::value::Value;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

/// The field values of a record, in the table's field order.
#[derive(Clone)]
pub enum Fields {
    /// A buffer of the record's own.
    Owned(Vec<Value>),
}

impl Fields {
    /// A buffer that starts out as `values` (a table's initial values). In this step it is a copy of its own
    /// (`is_shared()` is false); the step that turns sharing on keeps `values` itself until the first write.
    pub fn shared(values: Rc<[Value]>) -> Fields {
        Fields::Owned(values.to_vec())
    }

    /// Whether the buffer still shares its values with another (copied on the first write). Always false in this step.
    pub fn is_shared(&self) -> bool {
        false
    }

    /// The values as a vector of their own.
    pub fn into_vec(self) -> Vec<Value> {
        match self {
            Fields::Owned(v) => v,
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
        }
    }
}

impl DerefMut for Fields {
    #[inline]
    fn deref_mut(&mut self) -> &mut [Value] {
        match self {
            Fields::Owned(v) => v,
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
