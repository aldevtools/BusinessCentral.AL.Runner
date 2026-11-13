//! Frees the object instances that only a reference cycle keeps alive once the code that made them has ended
//! (al-runner issue 7975).
//!
//! AL code can build a cycle of references: a codeunit variable is a reference, so a table's global codeunit can hold a
//! record of that table whose table instance is the table's own (Microsoft's Audit File Export does this), and a page
//! instance's `CurrPage` refers to the page that holds the instance. BC's .NET garbage collector frees such a cycle;
//! the `Rc` that holds an [`Instance`] does not, so a warm server that ran the same tests again grew with every
//! request. Nothing here is observable to AL: an instance this module frees is one no variable, frame or kept state
//! can reach.
//!
//! Every instance is made through [`shared`], which notes it on the thread that made it (`Rc` never leaves that
//! thread). [`collect`] runs after a test codeunit's session has ended. It counts, for each noted instance that is still
//! alive, the references other noted instances hold to it through values they own alone (a reference held through a
//! shared record, list or page counts as one from outside, so the instance stays). An instance with more references
//! than those is reachable from outside, and so is everything it reaches; the rest is garbage, and its globals are
//! cleared, which ends the cycle.

use crate::page_handle::PageHandle;
use crate::value::{Instance, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};

type Shared = Rc<RefCell<Instance>>;

thread_local! {
    /// The instances made on this thread that were alive at the last [`collect`] or made since.
    static MADE: RefCell<Vec<Weak<RefCell<Instance>>>> = const { RefCell::new(Vec::new()) };
}

/// The instances of the process that are alive (made by [`shared`] and not yet dropped).
static LIVE: AtomicUsize = AtomicUsize::new(0);

/// The instances a page value owns alone: told by the crate that owns the page state (al-vm), as
/// [`crate::page_handle::set_closed_page`] is.
static PAGE_INSTANCES: OnceLock<PageInstances> = OnceLock::new();

/// Calls its second argument for the instance a page value owns alone.
pub type PageInstances = fn(&PageHandle, &mut dyn FnMut(&Shared));

/// Registers how the instance of a page value is found. The first registration stays.
pub fn set_page_instances(f: PageInstances) {
    let _ = PAGE_INSTANCES.set(f);
}

/// Counts one live instance while it exists (a field of [`Instance`]).
#[derive(Debug)]
pub struct LiveCount(());

impl Default for LiveCount {
    fn default() -> LiveCount {
        LIVE.fetch_add(1, Ordering::Relaxed);
        LiveCount(())
    }
}

impl Drop for LiveCount {
    fn drop(&mut self) {
        LIVE.fetch_sub(1, Ordering::Relaxed);
    }
}

/// How many instances the process holds now.
pub fn live() -> usize {
    LIVE.load(Ordering::Relaxed)
}

/// The shared handle of a new instance, noted for [`collect`].
pub fn shared(instance: Instance) -> Shared {
    let r = Rc::new(RefCell::new(instance));
    MADE.with(|m| m.borrow_mut().push(Rc::downgrade(&r)));
    r
}

/// Clears the globals of the instances made on this thread that only references among themselves keep alive. Called
/// when no AL code of this thread is between statements whose instances it could free (after a session has ended); an
/// instance that is borrowed counts as reachable.
pub fn collect() {
    let alive: Vec<Shared> = MADE
        .with(|m| std::mem::take(&mut *m.borrow_mut()))
        .iter()
        .filter_map(Weak::upgrade)
        .collect();
    let at: HashMap<*const RefCell<Instance>, usize> =
        alive.iter().enumerate().map(|(i, r)| (Rc::as_ptr(r), i)).collect();
    // The references each instance gets from the other noted ones.
    let mut inner = vec![0usize; alive.len()];
    let mut reach: Vec<Vec<usize>> = vec![Vec::new(); alive.len()];
    let mut borrowed = vec![false; alive.len()];
    for (i, r) in alive.iter().enumerate() {
        let Ok(inst) = r.try_borrow() else {
            borrowed[i] = true;
            continue;
        };
        owned_instances(&inst, &mut |t| {
            if let Some(&j) = at.get(&Rc::as_ptr(t)) {
                inner[j] += 1;
                reach[i].push(j);
            }
        });
    }
    // Reachable from outside: a reference no noted instance accounts for (the one `alive` holds aside), or a borrow.
    let mut live: Vec<bool> = alive
        .iter()
        .enumerate()
        .map(|(i, r)| borrowed[i] || Rc::strong_count(r) - 1 > inner[i])
        .collect();
    let mut todo: Vec<usize> = (0..alive.len()).filter(|&i| live[i]).collect();
    while let Some(i) = todo.pop() {
        for &j in &reach[i] {
            if !live[j] {
                live[j] = true;
                todo.push(j);
            }
        }
    }
    let mut freed = Vec::new();
    for (i, r) in alive.iter().enumerate() {
        if !live[i] {
            let mut inst = r.borrow_mut();
            freed.push((std::mem::take(&mut inst.globals), std::mem::take(&mut inst.exts)));
        }
    }
    // The cleared values drop here, after every borrow above has ended.
    drop(freed);
    let kept: Vec<Weak<RefCell<Instance>>> = alive
        .iter()
        .zip(&live)
        .filter(|(_, l)| **l)
        .map(|(r, _)| Rc::downgrade(r))
        .collect();
    drop(alive);
    MADE.with(|m| m.borrow_mut().extend(kept));
}

/// Calls `f` for every instance reference `inst` holds through values only it owns.
fn owned_instances(inst: &Instance, f: &mut dyn FnMut(&Shared)) {
    for (_, e) in &inst.exts {
        f(e);
    }
    for v in &inst.globals {
        owned_in_value(v, f);
    }
}

fn owned_in_value(v: &Value, f: &mut dyn FnMut(&Shared)) {
    match v {
        Value::Codeunit(c) => f(c),
        Value::Record(r) if Rc::strong_count(r) == 1 => {
            if let Ok(st) = r.try_borrow() {
                st.globals.owned_instances(f);
            }
        }
        Value::List(l) if Rc::strong_count(l) == 1 => {
            if let Ok(items) = l.try_borrow() {
                items.iter().for_each(|v| owned_in_value(v, f));
            }
        }
        Value::Array(a) if Rc::strong_count(a) == 1 => {
            if let Ok(items) = a.try_borrow() {
                items.iter().for_each(|v| owned_in_value(v, f));
            }
        }
        Value::Dictionary(d) if Rc::strong_count(d) == 1 => {
            if let Ok(items) = d.try_borrow() {
                for (k, v) in items.iter() {
                    owned_in_value(k, f);
                    owned_in_value(v, f);
                }
            }
        }
        Value::TestPage(p) => {
            if let Some(page) = PAGE_INSTANCES.get() {
                page(p, f);
            }
        }
        _ => {}
    }
}
