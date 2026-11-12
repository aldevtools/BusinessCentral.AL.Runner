//! Manual bindings held through the globals of a page (al-runner issue 6924).
//!
//! A page's trigger calls a codeunit held in a global of the page, and that codeunit binds a manual subscriber held in a
//! global of its own (a license controller). A binding lives as long as the bound instance (codeunits/
//! TestEventManualBinding.al:161-200, Contract 9), and here the page instance is what holds it. Real BC drops the page
//! instance, and with it the binding (corpus codeunits/TestEventManualBinding.al, Contract 11):
//! - while the page is open the binding fires (:277 Test11a);
//! - a TestPage's page drops it when the TestPage closes, its variable still in scope (:302 Test11c), and so the next test
//!   does not see it (:291 Test11b);
//! - a page no variable holds drops it when it closes: `Page.RunModal(Page::X)` (:317 Test11d) and `Page.Run(Page::X)`
//!   under a PageHandler (:355 Test11g);
//! - a Page variable keeps it after its `RunModal` while the variable is in scope (:330 Test11e), and drops it once the
//!   variable is gone (:343 Test11f).
//!
//! - a local TestPage left open drops it once its variable is gone: at the end of the test method (:393 Test11j) or of a
//!   helper procedure (:404 Test11k); while the variable is in scope it fires (:379 Test11i).
//!
//! The runtime's page values form a reference cycle (`CurrPage` in the page's globals holds the page), so a page never
//! drops by itself: the binding is ended here. Not pinned, and refused when an event would reach the binding: a Page
//! variable's non-modal `Run` while the variable is still in scope.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

use crate::pages::{PageState, st_of};
use al_builtins::native::Signal;
use al_builtins::value::{Instance, Value};

use crate::Session;

type Inst = Rc<RefCell<Instance>>;
type St = Rc<RefCell<PageState>>;

/// A manual binding whose bound instance the globals of a page reached when it was made.
pub(crate) struct PageHeld {
    instance: Weak<RefCell<Instance>>,
    /// The root page of the page tree (a part maps to the page that hosts it).
    page: Weak<RefCell<PageState>>,
    /// Whether the binding still lives is not pinned: an event that would reach it is refused.
    unknown: bool,
}

/// What became of the page that holds a binding.
#[derive(PartialEq)]
enum Fate {
    /// The page instance lives: the binding fires.
    Lives,
    /// The page instance is gone, and with it every binding only it held.
    Gone,
    /// Not pinned.
    Unknown,
}

impl Session {
    /// A page got its instance: a binding its code makes can be held by the page (`note_page_binding`).
    pub(crate) fn note_page_instance(&mut self, st: &St) {
        let n = self.events.pages_with_instance.len();
        if n >= 64 && n.is_power_of_two() {
            self.drop_finished_pages();
        }
        self.events.pages_with_instance.push(Rc::downgrade(st));
    }

    /// Drops the pages nothing can run code of any more (a closed page no variable holds) from the list.
    fn drop_finished_pages(&mut self) {
        self.events
            .page_var_runs
            .retain(|p| p.upgrade().is_some_and(|st| page_outside_refs(&st) > 1));
        self.events.pages_with_instance.retain(|p| {
            p.upgrade()
                .is_some_and(|st| st.borrow().open || page_outside_refs(&st) > 1)
        });
    }

    /// Whether `st` is the only page of its page object that has an instance and still lives (open, or held by a variable). `run_by_currpage` tells
    /// `CurrPage.Run()` from a Page variable's `Run()` by the object of the calling code alone; with a second page of the same
    /// object alive, the caller can be that other page running a variable of its own type.
    pub(crate) fn only_live_page_of_its_object(&self, st: &St) -> bool {
        let object = st.borrow().object;
        self.events
            .pages_with_instance
            .iter()
            .filter_map(Weak::upgrade)
            // As `drop_finished_pages` counts: a closed page no variable holds any more is gone.
            .filter(|p| p.try_borrow().map_or(true, |b| b.open) || page_outside_refs(p) > 1)
            .all(|p| Rc::ptr_eq(&p, st) || p.try_borrow().is_ok_and(|b| b.object != object))
    }

    /// A Page variable's `Run` or `RunModal` (`modal`) shows `st`, with the record `SetRecord` gave it.
    pub(crate) fn run_page_var(&mut self, st: &St, modal: bool) -> Result<Value, Signal> {
        let record = st.borrow().set_record.clone();
        self.note_page_var_run(st, !modal);
        // `CurrPage.Run()` runs from the page's own code; a Page variable's `Run()` from other code.
        let object = st.borrow().object as usize;
        let variable_run = !modal && self.frames.last() != Some(&object);
        st.borrow_mut().run_by_currpage = !modal && !variable_run;
        if variable_run {
            self.note_page_run(st, true, record.is_some());
        } else {
            st.borrow_mut().page_run = None;
        }
        let r = self.run_page_ui(st, record, modal)?;
        if variable_run {
            self.empty_temp_rec_after_variable_run(st)?;
        }
        Ok(r)
    }

    /// A Page variable's `Run` (`modeless`) or `RunModal` shows `st`.
    fn note_page_var_run(&mut self, st: &St, modeless: bool) {
        self.events
            .page_var_runs
            .retain(|p| p.strong_count() > 0 && p.as_ptr() != Rc::as_ptr(st));
        if modeless {
            self.events.page_var_runs.push(Rc::downgrade(st));
        }
    }

    /// `BindSubscription` bound `inst`: notes the page whose globals reach it, if any.
    pub(crate) fn note_page_binding(&mut self, inst: &Inst) {
        let mut holder = None;
        for p in &self.events.pages_with_instance {
            let Some(st) = p.upgrade() else {
                continue;
            };
            let root = root_page(&st);
            if Graph::from_page(&root).index(inst).is_some() {
                holder = Some(Rc::downgrade(&root));
                break;
            }
        }
        self.drop_finished_pages();
        if let Some(page) = holder {
            self.events.page_held.push(PageHeld {
                instance: Rc::downgrade(inst),
                page,
                unknown: false,
            });
        }
    }

    /// A Bind or Unbind of `inst`: what a page held no longer decides its binding.
    pub(crate) fn forget_page_held(&mut self, inst: &Inst) {
        self.events
            .page_held
            .retain(|h| h.instance.strong_count() > 0 && h.instance.as_ptr() != Rc::as_ptr(inst));
    }

    /// Ends every binding that only a gone page held, and marks the ones whose fate is not pinned.
    pub(crate) fn prune_page_held(&mut self) {
        if self.events.page_held.is_empty() {
            return;
        }
        let entries = std::mem::take(&mut self.events.page_held);
        let mut keep = Vec::new();
        let mut ended = Vec::new();
        for mut h in entries {
            let Some(st) = h.page.upgrade() else {
                continue;
            };
            if h.instance.strong_count() == 0 {
                continue;
            }
            match self.page_fate(&st) {
                Fate::Lives => {
                    h.unknown = false;
                    keep.push(h);
                }
                Fate::Unknown => {
                    h.unknown = true;
                    keep.push(h);
                }
                Fate::Gone => {
                    let graph = Graph::from_page(&st);
                    drop(st);
                    if graph.held_only_by_page(&h.instance) {
                        ended.push(h.instance.as_ptr());
                    }
                }
            }
        }
        self.events.page_held = keep;
        if !ended.is_empty() {
            self.events
                .bindings
                .retain(|b| b.instance.strong_count() > 0 && !ended.contains(&b.instance.as_ptr()));
            self.sync_active_instances();
        }
    }

    /// What became of the root page `st` (see the module comment for the pins).
    fn page_fate(&self, st: &St) -> Fate {
        let outside = page_outside_refs(st) > 1;
        let var_run = self.events.page_var_runs.iter().any(|p| p.as_ptr() == Rc::as_ptr(st));
        let s = st.borrow();
        // A Page variable that never ran holds its instance like a codeunit variable does (types/operators-expressions/
        // Y73ThisObjects.al:498 Page_OwnType_And_Codeunit_Variable binds a global of one); once the variable is gone it
        // is not pinned.
        // A page still open that no variable holds any more (a local TestPage left open at the end of a test method or of
        // a helper procedure) is gone: TestEventManualBinding.al:393 Test11j, :404 Test11k (and :379 Test11i: it lives
        // while its variable is in scope).
        if s.open {
            return if outside { Fate::Lives } else { Fate::Gone };
        }
        if !s.ran {
            return if outside { Fate::Lives } else { Fate::Unknown };
        }
        if var_run {
            return if outside { Fate::Unknown } else { Fate::Gone };
        }
        match s.run_static {
            Some(false) if outside => Fate::Lives,
            _ => Fate::Gone,
        }
    }

    /// Refuses an event that would reach the bound instance `inst` while its page's fate is not pinned.
    pub(crate) fn refuse_unpinned_page_binding(&self, inst: &Inst) -> Result<(), Signal> {
        let Some(h) = self
            .events
            .page_held
            .iter()
            .find(|h| h.unknown && h.instance.as_ptr() == Rc::as_ptr(inst))
        else {
            return Ok(());
        };
        let program = &self.engine.program;
        let page = h
            .page
            .upgrade()
            .map(|p| program.objects[p.borrow().object as usize].name.clone())
            .unwrap_or_default();
        Err(Signal::unsupported(format!(
            "a manual subscription of codeunit {} bound through the globals of page {page} would receive an event, and \
             whether that page still holds it is not pinned (a Page variable after its non-modal Run; al-runner issue \
             6924)",
            program.objects[inst.borrow().object as usize].name,
        )))
    }
}

/// The page that hosts the part `st`, up to the root of the tree.
fn root_page(st: &St) -> St {
    let mut at = st.clone();
    loop {
        let host = at.borrow().host.as_ref().and_then(|(h, _)| h.upgrade());
        match host {
            Some(h) => at = h,
            None => return at,
        }
    }
}

/// The references to `st` from outside its own page tree, plus the caller's own one: the `CurrPage` globals of its
/// instances do not count.
pub(crate) fn page_outside_refs(st: &St) -> usize {
    let mut own = 0;
    for inst in tree_instances(st) {
        for g in &inst.borrow().globals {
            if let Value::TestPage(p) = g
                && Rc::ptr_eq(st_of(p), st)
            {
                own += 1;
            }
        }
    }
    Rc::strong_count(st).saturating_sub(own)
}

/// The instances of the page tree of `st`: the page's, its extensions' and those of its parts (open, closed or hidden).
fn tree_instances(st: &St) -> Vec<Inst> {
    let mut out = Vec::new();
    let mut todo = vec![st.clone()];
    while let Some(p) = todo.pop() {
        let p = p.borrow();
        out.extend(p.instance.iter().cloned());
        out.extend(p.ext_instances.iter().map(|(_, i)| i.clone()));
        for (_, child) in p.parts.iter().chain(&p.closed_parts).chain(&p.hidden_parts) {
            todo.push(child.clone());
        }
    }
    out
}

/// The codeunit instances the globals of a page tree reach, with the references between them.
struct Graph {
    /// The page tree's own instances first (`roots` of them), then the codeunits they reach; one reference each.
    nodes: Vec<Inst>,
    roots: usize,
    /// For each node, the references to it from the globals of the nodes (codeunit variables and arrays of them).
    refs_in: Vec<usize>,
    edges: Vec<Vec<usize>>,
}

impl Graph {
    fn from_page(st: &St) -> Graph {
        let mut nodes = tree_instances(st);
        nodes.dedup_by(|a, b| Rc::ptr_eq(a, b));
        let roots = nodes.len();
        let mut at: HashMap<*const RefCell<Instance>, usize> =
            nodes.iter().enumerate().map(|(i, n)| (Rc::as_ptr(n), i)).collect();
        let mut refs_in = vec![0; roots];
        let mut edges = Vec::new();
        let mut i = 0;
        while i < nodes.len() {
            let mut out = Vec::new();
            let mut held = Vec::new();
            for g in &nodes[i].borrow().globals {
                codeunits_in(g, &mut held);
            }
            for c in held {
                let j = *at.entry(Rc::as_ptr(&c)).or_insert_with(|| {
                    nodes.push(c.clone());
                    refs_in.push(0);
                    nodes.len() - 1
                });
                refs_in[j] += 1;
                out.push(j);
            }
            edges.push(out);
            i += 1;
        }
        Graph {
            nodes,
            roots,
            refs_in,
            edges,
        }
    }

    fn index(&self, inst: &Inst) -> Option<usize> {
        self.nodes.iter().position(|n| Rc::ptr_eq(n, inst))
    }

    /// Whether nothing outside the page tree reaches `inst`: no codeunit the tree reaches on the way to it has a
    /// reference from elsewhere (a variable of a running routine, a SingleInstance codeunit, another object's global).
    fn held_only_by_page(&self, inst: &Weak<RefCell<Instance>>) -> bool {
        let Some(target) = self.nodes.iter().position(|n| Rc::as_ptr(n) == inst.as_ptr()) else {
            return false;
        };
        // The graph's own reference to each node is the 1.
        let outside = |i: usize| Rc::strong_count(&self.nodes[i]) > self.refs_in[i] + 1;
        let mut live: Vec<bool> = (0..self.nodes.len()).map(|i| i >= self.roots && outside(i)).collect();
        let mut todo: Vec<usize> = (0..self.nodes.len()).filter(|&i| live[i]).collect();
        while let Some(i) = todo.pop() {
            for &j in &self.edges[i] {
                if !live[j] {
                    live[j] = true;
                    todo.push(j);
                }
            }
        }
        target >= self.roots && !live[target]
    }
}

/// The codeunit instances `v` holds: a codeunit variable, or the cells of an array of them.
fn codeunits_in(v: &Value, out: &mut Vec<Inst>) {
    match v {
        Value::Codeunit(c) => out.push(c.clone()),
        Value::Array(cells) => {
            for c in cells.borrow().iter() {
                codeunits_in(c, out);
            }
        }
        _ => {}
    }
}
