//! Tests of `Fields` (PERF-22 design section 4, tests 1-4): copy on first write, clones, the fixed length and the iterators.
//! Memory sharing is invisible to AL, so the corpus cannot pin it (CLAUDE.md rule 1).

use super::Fields;
use crate::value::Value;
use std::rc::Rc;

fn ints(v: &[i32]) -> Vec<Value> {
    v.iter().map(|i| Value::Integer(*i)).collect()
}

fn shared(v: &[i32]) -> (Rc<[Value]>, Fields) {
    let rc: Rc<[Value]> = ints(v).into();
    (Rc::clone(&rc), Fields::shared(rc))
}

fn read(f: &Fields) -> Vec<i32> {
    f.iter()
        .map(|v| match v {
            Value::Integer(i) => *i,
            other => panic!("{other:?}"),
        })
        .collect()
}

fn int_of(rc: &[Value]) -> Vec<i32> {
    rc.iter()
        .map(|v| match v {
            Value::Integer(i) => *i,
            other => panic!("{other:?}"),
        })
        .collect()
}

#[test]
fn a_shared_buffer_reads_as_its_values_and_is_shared() {
    let (_rc, f) = shared(&[1, 2, 3]);
    assert!(f.is_shared());
    assert_eq!(read(&f), [1, 2, 3]);
    assert_eq!(f.len(), 3);
}

#[test]
fn a_clone_of_a_shared_buffer_stays_shared() {
    let (_rc, f) = shared(&[1, 2]);
    let c = f.clone();
    assert!(c.is_shared());
    assert!(f.is_shared());
}

#[test]
fn a_write_through_deref_mut_copies_and_leaves_other_holders_alone() {
    let (rc, mut f) = shared(&[1, 2, 3]);
    let other = f.clone();
    f[1] = Value::Integer(20);
    assert_eq!(read(&f), [1, 20, 3]);
    assert!(!f.is_shared());
    assert_eq!(read(&other), [1, 2, 3]);
    assert!(other.is_shared());
    assert_eq!(int_of(&rc), [1, 2, 3]);
}

#[test]
fn writing_a_clone_makes_only_the_clone_owned() {
    let (_rc, f) = shared(&[1, 2]);
    let mut c = f.clone();
    c[0] = Value::Integer(9);
    assert!(!c.is_shared());
    assert!(f.is_shared());
    assert_eq!(read(&f), [1, 2]);
    assert_eq!(read(&c), [9, 2]);
}

#[test]
fn iter_mut_copies_first() {
    let (rc, mut f) = shared(&[1, 2]);
    for v in f.iter_mut() {
        *v = Value::Integer(0);
    }
    assert!(!f.is_shared());
    assert_eq!(read(&f), [0, 0]);
    assert_eq!(int_of(&rc), [1, 2]);
}

#[test]
fn iterating_a_mutable_reference_copies_first() {
    let (rc, mut f) = shared(&[1, 2]);
    for v in &mut f {
        *v = Value::Integer(5);
    }
    assert!(!f.is_shared());
    assert_eq!(read(&f), [5, 5]);
    assert_eq!(int_of(&rc), [1, 2]);
}

#[test]
fn get_mut_copies_first() {
    let (rc, mut f) = shared(&[1, 2]);
    *f.get_mut(1).unwrap() = Value::Integer(7);
    assert!(!f.is_shared());
    assert_eq!(read(&f), [1, 7]);
    assert_eq!(int_of(&rc), [1, 2]);
}

#[test]
fn mem_replace_of_an_element_copies_first() {
    let (rc, mut f) = shared(&[1, 2]);
    let old = std::mem::replace(&mut f[0], Value::Integer(4));
    assert!(matches!(old, Value::Integer(1)));
    assert!(!f.is_shared());
    assert_eq!(read(&f), [4, 2]);
    assert_eq!(int_of(&rc), [1, 2]);
}

#[test]
fn swapping_elements_copies_first() {
    let (rc, mut f) = shared(&[1, 2]);
    f.swap(0, 1);
    assert!(!f.is_shared());
    assert_eq!(read(&f), [2, 1]);
    assert_eq!(int_of(&rc), [1, 2]);
}

#[test]
fn a_read_never_copies() {
    let (_rc, f) = shared(&[1, 2, 3]);
    let _ = f.iter().count();
    let _ = &f[0];
    let _ = f.get(2);
    assert!(f.is_shared());
}

#[test]
fn the_length_is_fixed_by_a_write() {
    let (_rc, mut f) = shared(&[1, 2, 3]);
    f[2] = Value::Null;
    assert_eq!(f.len(), 3);
    let mut o: Fields = ints(&[1, 2]).into();
    o[0] = Value::Null;
    assert_eq!(o.len(), 2);
}

#[test]
fn into_vec_of_a_shared_buffer_equals_its_elements() {
    let (rc, f) = shared(&[1, 2, 3]);
    assert_eq!(int_of(&f.into_vec()), [1, 2, 3]);
    assert_eq!(int_of(&rc), [1, 2, 3]);
}

#[test]
fn into_iter_of_a_shared_buffer_yields_its_elements() {
    let (_rc, f) = shared(&[4, 5]);
    let got: Vec<Value> = f.into_iter().collect();
    assert_eq!(int_of(&got), [4, 5]);
}

#[test]
fn default_is_empty_and_owned() {
    let f = Fields::default();
    assert!(f.is_empty());
    assert!(!f.is_shared());
}

#[test]
fn from_vec_is_owned() {
    let f: Fields = ints(&[1]).into();
    assert!(!f.is_shared());
    assert_eq!(read(&f), [1]);
}

#[test]
fn a_clone_of_an_owned_buffer_is_a_deep_copy() {
    let mut a: Fields = ints(&[1, 2]).into();
    let b = a.clone();
    a[0] = Value::Integer(10);
    assert_eq!(read(&a), [10, 2]);
    assert_eq!(read(&b), [1, 2]);
    assert!(!b.is_shared());
}

#[test]
fn debug_prints_the_values_alone() {
    let f: Fields = ints(&[1, 2]).into();
    let v = ints(&[1, 2]);
    assert_eq!(format!("{f:?}"), format!("{v:?}"));
    let (_rc, s) = shared(&[1, 2]);
    assert_eq!(format!("{s:?}"), format!("{v:?}"));
}
