//! The objects of a program as a list of slots (ADR-054 decision 3).
//!
//! A slot holds a finished object, or nothing yet and a source that makes the object the first time the slot is read.
//! Reading goes through one accessor (`Index`, [`Objects::get`], [`Objects::iter`]); the first read of an empty slot
//! asks the source and keeps the answer (`OnceLock`, so test codeunits on several threads that touch the same object
//! make it once and see the same object). A slot's content does not depend on when it is made, so the order of the
//! reads is not observable.
//!
//! [`Objects::iter`] reads every slot. A pass over all objects that only needs what an object's head holds must not
//! use it on a lazy list: [`Objects::made`] counts the objects a source made, so a test can fail when a start-up pass
//! makes every object.

use crate::{Object, ObjectHead};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};

/// Makes the object of a slot that was left empty. Called at most once per slot of an [`Objects`] list.
pub trait ObjectSource: Send + Sync {
    /// The object of slot `i`. A source that cannot make it (a damaged cache block whose start check passed) panics:
    /// an internal error of the test that touched it (ADR-054 owner decision 2).
    fn object(&self, i: usize) -> Object;
}

/// The objects of a program, in program order.
#[derive(Default)]
pub struct Objects {
    /// Boxed: an object is about 1.5 KB, and a list over a compile cache has about ten thousand slots, most never made.
    slots: Vec<OnceLock<Box<Object>>>,
    /// Makes the objects of the empty slots.
    source: Option<Arc<dyn ObjectSource>>,
    /// How many objects the source made.
    made: Arc<AtomicUsize>,
    /// The heads of the objects, given with a lazy list or computed from the objects on first read. Every change of
    /// the list through `&mut self` drops them, so they never describe objects that changed.
    heads: OnceLock<Arc<[ObjectHead]>>,
}

impl Objects {
    pub fn new() -> Objects {
        Objects::default()
    }

    /// A list whose slot `i` holds `finished[i]` when that is given, and otherwise is made by `source` on first read.
    pub fn lazy(finished: Vec<Option<Object>>, source: Arc<dyn ObjectSource>) -> Objects {
        let slots = finished
            .into_iter()
            .map(|o| {
                let slot = OnceLock::new();
                if let Some(o) = o {
                    let _ = slot.set(Box::new(o));
                }
                slot
            })
            .collect();
        Objects {
            slots,
            source: Some(source),
            made: Default::default(),
            heads: OnceLock::new(),
        }
    }

    /// [`Objects::lazy`] with the heads of the objects given, so that reading them makes no object. `heads[i]` must be
    /// the head of the object slot `i` holds or makes.
    pub fn lazy_with_heads(
        finished: Vec<Option<Object>>,
        source: Arc<dyn ObjectSource>,
        heads: Arc<[ObjectHead]>,
    ) -> Objects {
        let list = Objects::lazy(finished, source);
        assert_eq!(heads.len(), list.len(), "one head per object");
        let _ = list.heads.set(heads);
        list
    }

    /// The head of every object (ADR-054 decision 2), in order. A pass over all objects that reads only what a head
    /// holds reads these: on a list given its heads it makes no object; otherwise the heads are computed from the
    /// objects once, and kept until the list changes.
    pub fn heads(&self) -> &[ObjectHead] {
        self.heads.get_or_init(|| self.iter().map(ObjectHead::of).collect())
    }

    /// The heads of the list, shared.
    pub fn heads_shared(&self) -> Arc<[ObjectHead]> {
        self.heads();
        self.heads.get().expect("computed above").clone()
    }

    pub fn len(&self) -> usize {
        self.slots.len()
    }

    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    /// The object of slot `i`, made now if it is not yet.
    pub fn get(&self, i: usize) -> Option<&Object> {
        let slot = self.slots.get(i)?;
        Some(slot.get_or_init(|| {
            let source = self.source.as_ref().expect("an empty slot has a source");
            self.made.fetch_add(1, Ordering::Relaxed);
            Box::new(source.object(i))
        }))
    }

    /// The object of slot `i` for change, made now if it is not yet.
    pub fn get_mut(&mut self, i: usize) -> Option<&mut Object> {
        self.get(i)?;
        self.heads = OnceLock::new();
        self.slots[i].get_mut().map(|o| &mut **o)
    }

    /// Whether slot `i` holds its object (finished from the start, or made by an earlier read).
    pub fn is_made(&self, i: usize) -> bool {
        self.slots.get(i).is_some_and(|s| s.get().is_some())
    }

    /// How many objects the source made so far (0 for a list of finished objects).
    pub fn made(&self) -> usize {
        self.made.load(Ordering::Relaxed)
    }

    /// Every object, in order; makes the objects not yet made.
    pub fn iter(&self) -> Iter<'_> {
        Iter {
            objects: self,
            front: 0,
            back: self.slots.len(),
        }
    }

    /// Every object for change; makes the objects not yet made.
    pub fn iter_mut(&mut self) -> impl DoubleEndedIterator<Item = &mut Object> + ExactSizeIterator {
        self.make_all();
        self.heads = OnceLock::new();
        self.slots.iter_mut().map(|s| &mut **s.get_mut().expect("made above"))
    }

    /// Makes every object not yet made.
    pub fn make_all(&self) {
        for i in 0..self.slots.len() {
            self.get(i);
        }
    }

    pub fn push(&mut self, object: Object) {
        self.heads = OnceLock::new();
        let slot = OnceLock::new();
        let _ = slot.set(Box::new(object));
        self.slots.push(slot);
    }

    pub fn reserve(&mut self, n: usize) {
        self.slots.reserve(n);
    }

    pub fn clear(&mut self) {
        self.heads = OnceLock::new();
        self.slots.clear();
    }

    /// The objects as a plain list; makes the objects not yet made.
    pub fn into_vec(self) -> Vec<Object> {
        self.make_all();
        self.slots
            .into_iter()
            .map(|s| *s.into_inner().expect("made above"))
            .collect()
    }
}

impl From<Vec<Object>> for Objects {
    fn from(objects: Vec<Object>) -> Objects {
        let mut list = Objects::new();
        list.slots.reserve(objects.len());
        for o in objects {
            list.push(o);
        }
        list
    }
}

impl FromIterator<Object> for Objects {
    fn from_iter<I: IntoIterator<Item = Object>>(iter: I) -> Objects {
        Objects::from(iter.into_iter().collect::<Vec<_>>())
    }
}

impl Extend<Object> for Objects {
    fn extend<I: IntoIterator<Item = Object>>(&mut self, iter: I) {
        for o in iter {
            self.push(o);
        }
    }
}

impl std::ops::Index<usize> for Objects {
    type Output = Object;
    fn index(&self, i: usize) -> &Object {
        match self.get(i) {
            Some(o) => o,
            None => panic!("object index {i} out of range for a program of {} objects", self.len()),
        }
    }
}

impl std::ops::IndexMut<usize> for Objects {
    fn index_mut(&mut self, i: usize) -> &mut Object {
        let n = self.len();
        match self.get_mut(i) {
            Some(o) => o,
            None => panic!("object index {i} out of range for a program of {n} objects"),
        }
    }
}

/// The objects of a list in order, made as they are reached.
#[derive(Clone)]
pub struct Iter<'a> {
    objects: &'a Objects,
    front: usize,
    back: usize,
}

impl<'a> Iterator for Iter<'a> {
    type Item = &'a Object;
    fn next(&mut self) -> Option<&'a Object> {
        if self.front >= self.back {
            return None;
        }
        self.front += 1;
        self.objects.get(self.front - 1)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.back - self.front;
        (n, Some(n))
    }
    fn nth(&mut self, n: usize) -> Option<&'a Object> {
        self.front = self.front.saturating_add(n).min(self.back);
        self.next()
    }
}

impl DoubleEndedIterator for Iter<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.front >= self.back {
            return None;
        }
        self.back -= 1;
        self.objects.get(self.back)
    }
}

impl ExactSizeIterator for Iter<'_> {}

impl<'a> IntoIterator for &'a Objects {
    type Item = &'a Object;
    type IntoIter = Iter<'a>;
    fn into_iter(self) -> Iter<'a> {
        self.iter()
    }
}

impl<'a> IntoIterator for &'a mut Objects {
    type Item = &'a mut Object;
    type IntoIter = Box<dyn DoubleEndedIterator<Item = &'a mut Object> + 'a>;
    fn into_iter(self) -> Self::IntoIter {
        Box::new(self.iter_mut())
    }
}

impl IntoIterator for Objects {
    type Item = Object;
    type IntoIter = std::vec::IntoIter<Object>;
    fn into_iter(self) -> Self::IntoIter {
        self.into_vec().into_iter()
    }
}

impl Clone for Objects {
    /// A copy shares the source; slots made later are made in each copy on its own.
    fn clone(&self) -> Objects {
        Objects {
            slots: self.slots.clone(),
            source: self.source.clone(),
            made: Arc::new(AtomicUsize::new(self.made())),
            heads: self.heads.clone(),
        }
    }
}

impl PartialEq for Objects {
    fn eq(&self, other: &Objects) -> bool {
        self.len() == other.len() && self.iter().zip(other.iter()).all(|(a, b)| a == b)
    }
}

impl std::fmt::Debug for Objects {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl Serialize for Objects {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_seq(self.iter())
    }
}

impl<'de> Deserialize<'de> for Objects {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Objects, D::Error> {
        Vec::<Object>::deserialize(d).map(Objects::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ObjKind;

    fn object(i: usize) -> Object {
        Object {
            kind: ObjKind::Codeunit,
            id: Some(i as u32),
            name: format!("C{i}"),
            ..Object::default()
        }
    }

    struct Counting(AtomicUsize);
    impl ObjectSource for Counting {
        fn object(&self, i: usize) -> Object {
            self.0.fetch_add(1, Ordering::SeqCst);
            std::thread::sleep(std::time::Duration::from_millis(5));
            object(i)
        }
    }

    #[test]
    fn heads_given_with_a_lazy_list_make_no_object() {
        let heads: Arc<[ObjectHead]> = (0..3).map(|i| ObjectHead::of(&object(i))).collect();
        let list = Objects::lazy_with_heads(vec![None, None, None], Arc::new(Counting(AtomicUsize::new(0))), heads);
        assert_eq!(list.heads()[2].name, "C2");
        assert_eq!(list.made(), 0);
    }

    #[test]
    fn heads_follow_a_change_of_the_list() {
        let mut list = Objects::from(vec![object(0), object(1)]);
        assert_eq!(list.heads()[1].name, "C1");
        list[1].name = "Changed".into();
        assert_eq!(list.heads()[1].name, "Changed");
        list.push(object(2));
        assert_eq!(list.heads().len(), 3);
        for o in &mut list {
            o.id = None;
        }
        assert!(list.heads().iter().all(|h| h.id.is_none()));
    }

    #[test]
    fn a_slot_is_made_on_first_read_and_kept() {
        let source = Arc::new(Counting(AtomicUsize::new(0)));
        let list = Objects::lazy(vec![Some(object(0)), None, None], source.clone());
        assert_eq!(list.len(), 3);
        assert_eq!(list.made(), 0);
        assert!(list.is_made(0) && !list.is_made(1));
        assert_eq!(list[2].name, "C2");
        assert_eq!(list[2].name, "C2");
        assert_eq!(list.made(), 1);
        assert_eq!(source.0.load(Ordering::SeqCst), 1);
        let names: Vec<_> = list.iter().map(|o| o.name.clone()).collect();
        assert_eq!(names, ["C0", "C1", "C2"]);
        assert_eq!(list.made(), 2);
        assert_eq!(list, Objects::from(vec![object(0), object(1), object(2)]));
    }

    /// Test plan item 7: threads that touch the same object make it once and see the same object.
    #[test]
    fn threads_touching_one_slot_make_it_once() {
        let source = Arc::new(Counting(AtomicUsize::new(0)));
        let list = Objects::lazy(vec![None, None], source.clone());
        let seen: Vec<usize> = std::thread::scope(|s| {
            let hs: Vec<_> = (0..8).map(|_| s.spawn(|| &list[1] as *const Object as usize)).collect();
            hs.into_iter().map(|h| h.join().unwrap()).collect()
        });
        assert!(seen.iter().all(|&p| p == seen[0]));
        assert_eq!(source.0.load(Ordering::SeqCst), 1);
        assert_eq!(list.made(), 1);
    }

    #[test]
    fn iterators_and_changes() {
        let source = Arc::new(Counting(AtomicUsize::new(0)));
        let mut list = Objects::lazy(vec![None, Some(object(1)), None], source);
        assert_eq!(
            list.iter().rev().map(|o| o.id).collect::<Vec<_>>(),
            [Some(2), Some(1), Some(0)]
        );
        assert_eq!(list.iter().nth(2).map(|o| o.id), Some(Some(2)));
        list[0].name = "X".into();
        list.push(object(3));
        for o in &mut list {
            o.id = o.id.map(|n| n + 10);
        }
        let back = list.into_vec();
        assert_eq!(back.len(), 4);
        assert_eq!(back[0].name, "X");
        assert_eq!(back[3].id, Some(13));
    }

    #[test]
    fn serializes_as_a_plain_list() {
        let list = Objects::lazy(vec![None, None], Arc::new(Counting(AtomicUsize::new(0))));
        let plain = postcard::to_allocvec(&vec![object(0), object(1)]).unwrap();
        assert_eq!(postcard::to_allocvec(&list).unwrap(), plain);
        let back: Objects = postcard::from_bytes(&plain).unwrap();
        assert_eq!(back, list);
    }
}
