//! Dependency objects whose lowered form is supplied instead of compiled (ADR-022).
//!
//! A compile cache holds the lowered IR of a set of dependency apps, built by a complete (eager)
//! compile of those apps alone. A run that has the cache compiles only the workspace's own apps
//! and the few dependency objects the workspace changes, and takes every other dependency object
//! from the cache. The dependency apps come first in the program, so an object index or a
//! base-field position means the same in the cache and in the run.
//!
//! Some lowered dependency code embeds facts about the whole program: the extensions of a table
//! or page, the implementers of an interface. A block lists the objects whose such facts it
//! embeds (`DepFacts::embeds`). When the workspace extends or implements one of them, the block
//! is not used: the object is lowered again from source, with the group of objects that extend
//! each other (`relower_set`).

use crate::workspace::{ObjKind, Workspace};
use al_ir as ir;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashSet};

/// What the lowering of other objects reads about a dependency object, besides its IR. Stored
/// beside the block, so a run does not parse the object to learn it.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct DepFacts {
    /// The key of the object (ADR-033).
    pub key: ir::sym::ObjKey,
    /// Identity, checked against the object the run loaded at the same index.
    pub kind: Option<ObjKind>,
    pub id: Option<u32>,
    pub name: String,
    pub app: u32,
    /// The objects whose extensions, implementers or reasons the lowering of this object read, itself
    /// included (see the module comment).
    pub reads: Vec<ir::sym::ObjKey>,
    /// The table whose record is the implicit `Rec` of the object.
    pub table_of: Option<u32>,
    pub page_temporary: bool,
    /// Why the test client cannot drive the page yet (empty: it can).
    pub page_reasons: Vec<String>,
    /// The page objects a page shows as parts, and those its actions run.
    pub page_parts: Vec<u32>,
    pub page_runs: Vec<u32>,
    /// The page extensions of a page or report, in the order they apply.
    pub page_extensions: Vec<u32>,
    /// The global slot of each variable of a page extension, by lower-case name.
    pub ext_slots: Vec<(String, u32)>,
    /// The source offset of every routine, in the order the object numbers them.
    pub routine_offsets: Vec<u32>,
    /// The role of every routine (`lower::ROLE_*`), in the same order.
    pub routine_roles: Vec<u8>,
    /// For a table: the number of each field, in the order of the table's field list (ADR-033):
    /// what the symbolic form of other objects names a field by.
    pub fields: Vec<u32>,
    /// For a page (or a report with a request page): the name of each control in lower case, in the
    /// order of the page's control list (ADR-033).
    pub controls: Vec<String>,
    /// For an enum or enum extension: the implementations it declares and the ordinals of its values (ADR-033).
    pub enum_impls: Vec<ir::sym::EnumImplDecl>,
    pub enum_ordinals: Vec<u32>,
    /// For a codeunit: the interfaces it implements, those it lists and the ones they extend (ADR-033).
    pub implements: Vec<ir::sym::ObjKey>,
    /// The key of every routine, in the same order (ADR-033): what the symbolic form of other
    /// objects names a routine of this one by.
    pub routine_keys: Vec<ir::sym::LocalKey>,
    /// Why a variable of the xmlport or report is refused (the first reason), if it is.
    pub xml_reason: Option<String>,
    pub report_reason: Option<String>,
    /// The report extensions that change the run of a report, in object id order (ADR-025).
    pub report_extensions: Vec<u32>,
    /// The layout of a query, which the lowering of code that uses its columns reads.
    pub query: Option<crate::queries::QueryLayout>,
    /// How many source positions the object has (ADR-054): a run keeps their ids free and reads them with the block.
    pub positions: u32,
    /// Per routine: why it cannot run (empty: it can).
    pub unsupported: Vec<Vec<String>>,
    /// Per routine: the routines it calls directly, as (object, routine). What a variable of a
    /// page, report or xmlport can drive is added when asked (`Lowered::calls_of`), from `drivers`.
    pub calls: Vec<Vec<(u32, u32)>>,
    /// Per routine: the types of the variables it sees that can drive other objects (a page, test page or codeunit
    /// variable; `CurrPage` left out), its locals first, then the object's globals: what `Lowered::calls_of` reads, so
    /// the object is not decoded to answer it.
    pub drivers: Vec<Vec<al_builtins::value::RtType>>,
}

/// The lowered form of one dependency object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DepBlock {
    /// The object as the build linked it (ADR-054, owner decision 2026-10-27): the extensions of the dependency set
    /// merged in and every reference a program position, so a run that touches it only decodes it. Dependency objects
    /// come first in the program, so their positions are the same in the build and in a run. `None` for a placeholder.
    pub object: Option<ir::Object>,
    /// The id of the object's first source position in the program of the build. A run whose ids for the object start
    /// elsewhere moves them.
    pub pos_start: u32,
    /// The object's source positions, in id order.
    pub positions: Vec<ir::SrcPos>,
}

impl DepBlock {
    /// A block with nothing in it. A run and a layer build never read the lowered form of a supplied object while they
    /// compile (the lowering of other objects reads its facts), so they do not decode its block then.
    pub fn placeholder() -> DepBlock {
        DepBlock {
            object: None,
            pos_start: 0,
            positions: Vec::new(),
        }
    }
}

/// A set of lowered dependency objects, indexed like the objects of the dependency apps.
pub trait DepSource: Sync {
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn facts(&self, object: usize) -> &DepFacts;
    /// The head of `object`, as the build linked it (ADR-054).
    fn head(&self, object: usize) -> &ir::ObjectHead;
    /// The block of `object`, read and checked now. An error means the source is damaged.
    fn block(&self, object: usize) -> Result<DepBlock, String>;
    /// The heads of all objects, as values of their own: what a run's program keeps (ADR-054). A source that decodes
    /// its heads may decode them for this call instead of keeping them, so they are not decoded and then copied.
    fn heads_owned(&self) -> Vec<ir::ObjectHead> {
        (0..self.len()).map(|o| self.head(o).clone()).collect()
    }
    /// Dependency files that did not parse when the set was built, as (path, why). A run that
    /// uses the set parses none of them, so it has to be told.
    fn parse_problems(&self) -> &[(String, String)] {
        &[]
    }
}

/// A complete set, in memory: what a build of the dependency apps produces.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct DepSet {
    pub facts: Vec<DepFacts>,
    pub blocks: Vec<DepBlock>,
    /// The head of each object (ADR-054 decision 2), from the object as the build linked it: what a pass over all
    /// objects reads, so a run reads no block to make it.
    pub heads: Vec<ir::ObjectHead>,
    pub parse_problems: Vec<(String, String)>,
}

impl DepSource for DepSet {
    fn len(&self) -> usize {
        self.facts.len()
    }
    fn facts(&self, object: usize) -> &DepFacts {
        &self.facts[object]
    }
    fn head(&self, object: usize) -> &ir::ObjectHead {
        &self.heads[object]
    }
    fn block(&self, object: usize) -> Result<DepBlock, String> {
        Ok(self.blocks[object].clone())
    }
    fn parse_problems(&self) -> &[(String, String)] {
        &self.parse_problems
    }
}

/// One layer of a stacked set (owner decision 2026-10-15, ADR-022 amendment): the dependency apps
/// are compiled in layers along their order. A layer holds the blocks of its own objects, which
/// come after every object of the layers below (so their positions are the positions of a run
/// that has all the layers), and a block for each object of a lower layer that its apps change
/// (an extension, an implementer): the lowered form of that object with the layer present.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct DepLayer {
    /// How many objects the layers below hold.
    pub base_len: usize,
    /// The program index of the object each slot of `set` describes, ascending: the overridden
    /// objects (below `base_len`) first, then the layer's own, `base_len..`, with no gap.
    pub objects: Vec<u32>,
    /// Facts and blocks by slot. `parse_problems`: those of the layer's own files.
    pub set: DepSet,
}

impl DepLayer {
    /// A set that has nothing below it: the first layer.
    pub fn whole(set: DepSet) -> DepLayer {
        DepLayer {
            base_len: 0,
            objects: (0..set.facts.len() as u32).collect(),
            set,
        }
    }
}

/// What [`Stack`] reads from one layer.
pub trait LayerSource: Sync {
    fn base_len(&self) -> usize;
    /// The program index of each slot (see [`DepLayer::objects`]).
    fn objects(&self) -> &[u32];
    fn facts_at(&self, slot: usize) -> &DepFacts;
    fn head_at(&self, slot: usize) -> &ir::ObjectHead;
    /// The heads of all slots as values of their own (see [`DepSource::heads_owned`]).
    fn heads_owned(&self) -> Vec<ir::ObjectHead> {
        (0..self.objects().len()).map(|s| self.head_at(s).clone()).collect()
    }
    /// The block of `slot`, read and checked now.
    fn block_at(&self, slot: usize) -> Result<DepBlock, String>;
    fn parse_problems(&self) -> &[(String, String)];
}

impl<T: LayerSource + ?Sized> LayerSource for &T {
    fn base_len(&self) -> usize {
        (**self).base_len()
    }
    fn objects(&self) -> &[u32] {
        (**self).objects()
    }
    fn facts_at(&self, slot: usize) -> &DepFacts {
        (**self).facts_at(slot)
    }
    fn head_at(&self, slot: usize) -> &ir::ObjectHead {
        (**self).head_at(slot)
    }
    fn heads_owned(&self) -> Vec<ir::ObjectHead> {
        (**self).heads_owned()
    }
    fn block_at(&self, slot: usize) -> Result<DepBlock, String> {
        (**self).block_at(slot)
    }
    fn parse_problems(&self) -> &[(String, String)] {
        (**self).parse_problems()
    }
}

impl LayerSource for DepLayer {
    fn base_len(&self) -> usize {
        self.base_len
    }
    fn objects(&self) -> &[u32] {
        &self.objects
    }
    fn facts_at(&self, slot: usize) -> &DepFacts {
        &self.set.facts[slot]
    }
    fn head_at(&self, slot: usize) -> &ir::ObjectHead {
        &self.set.heads[slot]
    }
    fn block_at(&self, slot: usize) -> Result<DepBlock, String> {
        Ok(self.set.blocks[slot].clone())
    }
    fn parse_problems(&self) -> &[(String, String)] {
        &self.set.parse_problems
    }
}

/// The layers, lowest first, as one [`DepSource`]: an object comes from the highest layer that
/// holds it.
pub struct Stack<'a> {
    layers: Vec<std::sync::Arc<dyn LayerSource + Send + 'a>>,
    /// Per object: (layer, slot) of the highest layer that holds it.
    at: Vec<(u32, u32)>,
    problems: Vec<(String, String)>,
}

impl<'a> Stack<'a> {
    /// An error says the layers do not fit each other (wrong order, a gap, an object out of range).
    pub fn new(layers: Vec<&'a dyn LayerSource>) -> Result<Stack<'a>, String> {
        Stack::from_shared(
            layers
                .into_iter()
                .map(|l| std::sync::Arc::new(l) as std::sync::Arc<dyn LayerSource + Send + 'a>)
                .collect(),
        )
    }

    /// [`Stack::new`] of layers it shares ownership of, so the stack can outlive the caller (a run that decodes the
    /// objects on first touch keeps it, ADR-054).
    pub fn from_shared(layers: Vec<std::sync::Arc<dyn LayerSource + Send + 'a>>) -> Result<Stack<'a>, String> {
        let mut at: Vec<(u32, u32)> = Vec::new();
        let mut problems = Vec::new();
        for (li, l) in layers.iter().enumerate() {
            if l.base_len() != at.len() {
                return Err(format!(
                    "layer {li} starts above {} objects, the layers below hold {}",
                    l.base_len(),
                    at.len()
                ));
            }
            let base = at.len();
            let mut own = 0usize;
            let mut last: Option<u32> = None;
            for (slot, &o) in l.objects().iter().enumerate() {
                if last.is_some_and(|p| p >= o) {
                    return Err(format!("layer {li}: objects are not ascending"));
                }
                last = Some(o);
                let o = o as usize;
                if o < base {
                    at[o] = (li as u32, slot as u32);
                } else if o == base + own {
                    at.push((li as u32, slot as u32));
                    own += 1;
                } else {
                    return Err(format!("layer {li}: object {o} leaves a gap"));
                }
            }
            problems.extend(l.parse_problems().iter().cloned());
        }
        Ok(Stack { layers, at, problems })
    }
}

impl DepSource for Stack<'_> {
    fn len(&self) -> usize {
        self.at.len()
    }
    fn facts(&self, object: usize) -> &DepFacts {
        let (l, s) = self.at[object];
        self.layers[l as usize].facts_at(s as usize)
    }
    fn head(&self, object: usize) -> &ir::ObjectHead {
        let (l, s) = self.at[object];
        self.layers[l as usize].head_at(s as usize)
    }
    fn heads_owned(&self) -> Vec<ir::ObjectHead> {
        // Each layer's heads, moved to the objects the layer supplies (a lower layer's slot that a higher one
        // replaces is dropped).
        let mut layers: Vec<Vec<Option<ir::ObjectHead>>> = self
            .layers
            .iter()
            .map(|l| l.heads_owned().into_iter().map(Some).collect())
            .collect();
        self.at
            .iter()
            .map(|&(l, s)| layers[l as usize][s as usize].take().expect("one object per slot"))
            .collect()
    }
    fn block(&self, object: usize) -> Result<DepBlock, String> {
        let (l, s) = self.at[object];
        self.layers[l as usize].block_at(s as usize)
    }
    fn parse_problems(&self) -> &[(String, String)] {
        &self.problems
    }
}

/// Makes the objects a compile cache supplies, the first time a run touches them (ADR-054): decodes the object's
/// block, which holds the object as the build linked it, and moves its source position ids to where this run has
/// them. Also gives the positions of such an object (`Program::positions`).
pub struct CachedObjects {
    src: std::sync::Arc<dyn DepSource + Send>,
    /// Per object: the id of its first source position in this run's program.
    starts: Vec<u32>,
    positions: Vec<std::sync::OnceLock<Vec<ir::SrcPos>>>,
    /// How many objects this source decoded.
    decoded: std::sync::atomic::AtomicUsize,
}

impl CachedObjects {
    pub fn new(src: std::sync::Arc<dyn DepSource + Send>, starts: Vec<u32>) -> CachedObjects {
        CachedObjects {
            src,
            positions: starts.iter().map(|_| std::sync::OnceLock::new()).collect(),
            starts,
            decoded: Default::default(),
        }
    }

    /// How many objects this source decoded so far (every copy of the program shares the source).
    pub fn decoded(&self) -> usize {
        self.decoded.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// The block of `object`. The whole entry was checked when it was opened, so a block that does not read now is an
    /// internal error of the test that touched it (ADR-054 owner decision 2).
    fn read(&self, object: usize) -> DepBlock {
        self.src
            .block(object)
            .unwrap_or_else(|e| panic!("compile cache: the block of object {object} is damaged: {e}"))
    }
}

/// How many objects [`CachedObjects`] decoded in this process, and the time they took in total (`--timings`).
static DECODED: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
static DECODE_NANOS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// The objects a compile cache supplied that were decoded on first touch so far in this process, and the time it took.
pub fn decoded_on_touch() -> (usize, std::time::Duration) {
    use std::sync::atomic::Ordering::Relaxed;
    (
        DECODED.load(Relaxed),
        std::time::Duration::from_nanos(DECODE_NANOS.load(Relaxed)),
    )
}

impl ir::ObjectSource for CachedObjects {
    fn object(&self, i: usize) -> ir::Object {
        use std::sync::atomic::Ordering::Relaxed;
        let started = std::time::Instant::now();
        let object = self.object_now(i);
        DECODED.fetch_add(1, Relaxed);
        self.decoded.fetch_add(1, Relaxed);
        DECODE_NANOS.fetch_add(started.elapsed().as_nanos() as u64, Relaxed);
        object
    }
}

impl CachedObjects {
    fn object_now(&self, i: usize) -> ir::Object {
        let block = self.read(i);
        let mut object = block
            .object
            .unwrap_or_else(|| panic!("compile cache: object {i} has no stored object"));
        let by = self.starts[i].wrapping_sub(block.pos_start);
        if by != 0 {
            for r in &mut object.routines {
                r.shift_positions(by);
            }
        }
        let _ = self.positions[i].set(block.positions);
        object
    }
}

impl ir::PositionSource for CachedObjects {
    fn positions(&self, object: usize) -> &[ir::SrcPos] {
        self.positions[object].get_or_init(|| self.read(object).positions)
    }
}

/// How many objects, from the first, belong to dependency apps. The program has the dependency
/// apps first, so these are the objects a cache can supply. `None` when a dependency object
/// follows a workspace one.
pub fn dependency_prefix(ws: &Workspace) -> Option<usize> {
    let n = (0..ws.objects.len()).take_while(|&i| ws.is_dependency(i)).count();
    (n..ws.objects.len()).all(|i| !ws.is_dependency(i)).then_some(n)
}

/// Whether the dependency objects the run loaded are the ones the source describes.
pub fn check_identity(ws: &Workspace, n_dep: usize, src: &dyn DepSource) -> Result<(), String> {
    if src.len() != n_dep {
        return Err(format!(
            "the cache holds {} dependency objects, the dependency apps have {n_dep}",
            src.len()
        ));
    }
    for oi in 0..n_dep {
        let (o, f) = (&ws.objects[oi], src.facts(oi));
        if f.kind != Some(o.kind) || f.id != o.id || f.name != o.name || f.app as usize != o.app {
            return Err(format!(
                "dependency object {oi} is `{} {}` in the run and `{} {}` in the cache",
                o.kind.label(),
                o.name,
                f.kind.map_or("unknown", |k| k.label()),
                f.name
            ));
        }
    }
    Ok(())
}

/// [`check_identity`] for a source that holds the first `n` objects of `ws`, of any app (ADR-063: a kept compile).
pub fn check_identity_prefix(ws: &Workspace, n: usize, src: &dyn DepSource) -> Result<(), String> {
    check_identity(ws, n, src)
}

/// The dependency objects the workspace's own objects extend or implement: what changes the
/// extension and implementer sets the blocks embed.
pub fn touched(ws: &Workspace, n_dep: usize) -> HashSet<usize> {
    let mut out = HashSet::new();
    for w in n_dep..ws.objects.len() {
        let o = &ws.objects[w];
        if let Some(t) = ws.extension_target(w) {
            out.insert(t);
        }
        let mut interfaces: Vec<usize> = Vec::new();
        for p in &o.implements {
            if let crate::workspace::Resolution::Found(i) = ws.resolve_ref(o.file, ObjKind::Interface, p) {
                interfaces.push(i);
            }
        }
        if o.kind == ObjKind::Interface {
            interfaces.push(w);
        }
        for i in interfaces {
            out.insert(i);
            out.extend(ws.interface_bases(i));
        }
    }
    out.retain(|&i| i < n_dep);
    out
}

/// The blocks that must be lowered again because the declarations of the objects `changed` are not as the
/// cache saw them: those whose lowering read one of them (`DepFacts::reads`).
pub fn stale_blocks(src: &dyn DepSource, changed: &HashSet<&ir::sym::ObjKey>) -> Vec<usize> {
    (0..src.len())
        .filter(|&oi| src.facts(oi).reads.iter().any(|k| changed.contains(k)))
        .collect()
}

/// The dependency objects that cannot come from the source because the workspace changes
/// something their lowered form embeds, and the objects that extend each other with them.
pub fn relower_set(ws: &Workspace, n_dep: usize, src: &dyn DepSource) -> Vec<bool> {
    let touched = touched(ws, n_dep);
    let mut relower = vec![false; n_dep];
    if touched.is_empty() {
        return relower;
    }
    let changed: HashSet<&ir::sym::ObjKey> = touched.iter().map(|&t| &src.facts(t).key).collect();
    for oi in stale_blocks(src, &changed) {
        relower[oi] = true;
    }
    // An object and the objects that extend it are lowered together: the extension's lowering
    // reads the base's layout, and the base's lists the extensions.
    let mut queue: Vec<usize> = (0..n_dep).filter(|&i| relower[i]).collect();
    while let Some(i) = queue.pop() {
        let mut group: Vec<usize> = ws.extensions_of(i).to_vec();
        group.extend(ws.extension_target(i));
        for g in group {
            if g < n_dep && !relower[g] {
                relower[g] = true;
                queue.push(g);
            }
        }
    }
    relower
}

/// `embeds` of an object: itself, what it recorded, and the pages a recorded page shows as
/// parts (their reasons are part of the page's).
pub fn embeds_of(oi: usize, recorded: &BTreeSet<usize>, page_parts: &dyn Fn(usize) -> Vec<usize>) -> Vec<u32> {
    let mut out: BTreeSet<usize> = recorded.clone();
    out.insert(oi);
    let mut queue: Vec<usize> = out.iter().copied().collect();
    while let Some(p) = queue.pop() {
        for q in page_parts(p) {
            if out.insert(q) {
                queue.push(q);
            }
        }
    }
    out.into_iter().map(|i| i as u32).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::Roots;
    use crate::lower::{Lowered, compile_apps_cached, compile_apps_with, lower_for_cache};
    use crate::workspace::{AppKind, AppSource};
    use std::collections::BTreeSet;

    fn app(id: &str, deps: &[&str], src: &str) -> AppSource {
        AppSource {
            id: id.into(),
            name: id.into(),
            dependencies: deps.iter().map(|s| s.to_string()).collect(),
            files: vec![("f.al".into(), src.into())],
        }
    }

    const BASE: &str = r#"
table 50000 T { fields { field(1; A; Integer) { } field(2; B; Code[20]) { } } keys { key(PK; A) { } } }
tableextension 50001 DT extends T { fields { field(10; DX; Integer) { } } }
page 50002 P { PageType = Card; SourceTable = T; layout { area(Content) { field(AC; Rec.A) { } } } }
page 50003 P2 { PageType = Card; SourceTable = T; layout { area(Content) { part(Sub; P) { } } } }
interface IFoo { procedure Bump(): Integer; }
codeunit 50004 DImpl implements IFoo { procedure Bump(): Integer begin exit(1); end; }
codeunit 50005 DUse
{
    procedure CallIt(F: Interface IFoo): Integer begin exit(F.Bump()); end;
    procedure Fields(var R: Record T) begin R.DX := 3; R.A := 2; end;
    procedure Open() var TP: TestPage P2; begin end;
    procedure Is(F: Interface IFoo): Boolean begin exit(F is IFoo); end;
}
enum 50006 E implements IFoo { value(0; X) { Implementation = IFoo = DImpl; } }
codeunit 50007 DEnum { procedure G(En: Enum E): Integer var F: Interface IFoo; begin F := En; exit(F.Bump()); end; }
codeunit 50008 DPlain { procedure Plain(): Integer begin exit(7); end; procedure Twice(): Integer begin exit(Plain() * 2); end; }
"#;

    const MORE: &str = r#"
query 50009 DQ { elements { dataitem(T; T) { column(A; A) { } column(B; B) { } } } }
codeunit 50010 DQUse { procedure Sum(): Integer var Q: Query DQ; begin Q.Open(); if Q.Read() then exit(Q.A); end; }
page 50011 P3 { PageType = List; SourceTable = T; layout { area(Content) { part(Deep; P2) { } } } }
codeunit 50012 DPage { procedure Open() var TP: TestPage P3; begin TP.OpenView(); end; }
report 50013 R { dataset { dataitem(TD; T) { column(A; A) { } } } }
reportextension 50014 DRE extends R { dataset { add(TD) { column(B; B) { } } } }
codeunit 50015 DRun { procedure Go() begin Report.Run(50013); end; }
xmlport 50016 X { schema { textelement(Root) { tableelement(TE; T) { fieldelement(A; TE.A) { } } } } }
codeunit 50017 DX { procedure Imp() var XP: XmlPort X; begin end; }
interface IBar extends IFoo { procedure Name(): Text; }
codeunit 50018 DBar implements IBar { procedure Bump(): Integer begin exit(1); end; procedure Name(): Text begin exit('b'); end; }
codeunit 50019 DUseBar { procedure Use(F: Interface IFoo): Boolean begin exit(F is IFoo); end; }
"#;

    const OTHER: &str = r#"
table 51000 U { fields { field(1; K; Integer) { } } keys { key(PK; K) { } } }
codeunit 51001 OUse { procedure R(var X: Record U) begin X.K := 1; end; }
"#;

    const TABLE_EXT: &str = r#"
tableextension 60000 WT extends T { fields { field(20; WX; Integer) { } } }
codeunit 60004 WUse { procedure W() var R: Record T; begin R.WX := 1; R.DX := 2; end; }
"#;

    const EVERYTHING: &str = r#"
tableextension 60000 WT extends T { fields { field(20; WX; Integer) { } } }
pageextension 60001 WP extends P { layout { addlast(Content) { field(WXC; Rec.WX) { } } } }
codeunit 60002 WImpl implements IFoo { procedure Bump(): Integer begin exit(2); end; }
enumextension 60003 WE extends E { value(1; Y) { Implementation = IFoo = WImpl; } }
codeunit 60004 WUse { procedure W() var R: Record T; begin R.WX := 1; R.DX := 2; end; }
"#;

    fn calls(l: &Lowered) -> Vec<Vec<Vec<(usize, usize)>>> {
        (0..l.program.objects.len())
            .map(|o| {
                (0..l.program.objects[o].routines.len())
                    .map(|r| {
                        let mut v = l.calls_of(o, r);
                        v.sort_unstable();
                        v.dedup();
                        v
                    })
                    .collect()
            })
            .collect()
    }

    /// The complete compile of `deps` + `workspace` and the cached compile of them must agree on
    /// everything the runtime reads.
    pub(crate) fn assert_cached_equals_eager(deps: Vec<AppSource>, workspace: Vec<AppSource>) -> usize {
        let symbols = BTreeSet::new();
        let set = lower_for_cache(deps.clone(), &symbols, None);
        let mut apps = deps.clone();
        apps.extend(workspace);
        let kinds: Vec<AppKind> = (0..apps.len())
            .map(|i| {
                if i < deps.len() {
                    AppKind::Dependency
                } else {
                    AppKind::Workspace
                }
            })
            .collect();
        let roots = Roots {
            eager: true,
            ..Default::default()
        };
        let (_, eager) = compile_apps_with(apps.clone(), kinds.clone(), &symbols, None, roots.clone());
        let (ws, cached) = compile_apps_cached(apps, kinds, &symbols, None, roots, std::sync::Arc::new(set.clone()))
            .map_err(|e| e.1)
            .expect("the cache fits");
        // ADR-054: nothing the cache supplies is decoded by the compile; the comparisons below decodes every object
        // (test plan item 4: the objects as decoded equal the eager link, positions included).
        let supplied = cached.provided.iter().filter(|&&p| p).count();
        assert!(supplied > 0, "the cache supplies objects");
        assert_eq!(
            cached.program.objects.made(),
            0,
            "the compile decoded a supplied object"
        );
        let n_dep = dependency_prefix(&ws).unwrap();
        let relowered = (0..n_dep).filter(|&i| !cached.provided[i]).count();
        assert_eq!(eager.program.objects.len(), cached.program.objects.len());
        for (i, (a, b)) in eager.program.objects.iter().zip(&cached.program.objects).enumerate() {
            assert!(a == b, "object {i} `{}` differs", a.name);
        }
        assert!(*eager.program == *cached.program, "the programs differ");
        assert!(eager.unsupported == cached.unsupported, "the refusals differ");
        assert!(calls(&eager) == calls(&cached), "the call graphs differ");
        assert_eq!(eager.dependency, cached.dependency);
        assert_heads_match(&set, &cached);
        relowered
    }

    fn deps() -> Vec<AppSource> {
        vec![
            app("d1", &[], BASE),
            app("d2", &["d1"], OTHER),
            app("d3", &["d1"], MORE),
        ]
    }

    const USES_DEPENDENCY_OBJECTS: &str = r#"
codeunit 60010 WQ { procedure R() var Q: Query DQ; begin Q.Open(); if Q.Read() then exit(Q.A); end; }
codeunit 60011 WP { procedure P() var TP: TestPage P3; begin TP.OpenView(); end; }
"#;

    const EXTENDS_A_PART_PAGE: &str = r#"
pageextension 60020 WPartExt extends P { layout { addlast(Content) { field(WAC; Rec.B) { } } } }
codeunit 60021 WUse { procedure P() var TP: TestPage P3; begin TP.OpenView(); end; }
"#;

    const EXTENDS_REPORT_AND_INTERFACE: &str = r#"
reportextension 60030 WRE extends R { dataset { add(TD) { column(WB; B) { } } } }
interface IWork extends IBar { procedure More(); }
codeunit 60031 WBar implements IWork { procedure Bump(): Integer begin exit(3); end; procedure Name(): Text begin exit('w'); end; procedure More() begin end; }
"#;

    #[test]
    fn a_changed_declaration_invalidates_exactly_the_blocks_that_read_it() {
        let set = lower_for_cache(deps(), &BTreeSet::new(), None);
        let key_of = |name: &str| set.facts.iter().find(|f| f.name == name).expect(name).key.clone();
        let ifoo = key_of("IFoo");
        let stale = stale_blocks(&set, &HashSet::from([&ifoo]));
        // Exactly the blocks whose read list names the object.
        for (i, f) in set.facts.iter().enumerate() {
            assert_eq!(stale.contains(&i), f.reads.contains(&ifoo), "`{}`", f.name);
        }
        let names: BTreeSet<&str> = stale.iter().map(|&i| set.facts[i].name.as_str()).collect();
        for read_it in ["IFoo", "DUse", "DEnum"] {
            assert!(
                names.contains(read_it),
                "`{read_it}` calls through the interface: {names:?}"
            );
        }
        for did_not in ["DPlain", "OUse", "T"] {
            assert!(
                !names.contains(did_not),
                "`{did_not}` does not read the interface: {names:?}"
            );
        }
        // Adding an implementer changes only blocks that read the interface (the new object is last, so the
        // others keep their places).
        let mut more = deps();
        more.push(app(
            "d4",
            &["d1"],
            "codeunit 70000 NewImpl implements IFoo { procedure Bump(): Integer begin exit(9); end; }",
        ));
        let changed = lower_for_cache(more, &BTreeSet::new(), None);
        let differing: Vec<usize> = (0..set.len()).filter(|&i| set.blocks[i] != changed.blocks[i]).collect();
        assert!(!differing.is_empty(), "the interface calls gained a target");
        for i in &differing {
            assert!(
                stale.contains(i),
                "block `{}` changed and was not read-listed",
                set.facts[*i].name
            );
        }
        assert!(differing.len() < set.len() / 2, "most blocks are untouched");
    }

    #[test]
    fn a_workspace_that_changes_nothing_takes_every_dependency_object_from_the_cache() {
        let relowered = assert_cached_equals_eager(
            deps(),
            vec![app("w", &["d1"], "codeunit 60000 C { procedure P() begin end; }")],
        );
        assert_eq!(relowered, 0);
    }

    #[test]
    fn a_table_extension_in_the_workspace_relowers_the_table_and_what_embeds_it() {
        let relowered = assert_cached_equals_eager(deps(), vec![app("w", &["d1"], TABLE_EXT)]);
        assert!(relowered >= 1, "the table itself is lowered again");
        assert!(relowered < 8, "not the whole set ({relowered})");
    }

    #[test]
    fn extensions_and_implementers_in_the_workspace_give_the_complete_compile() {
        let relowered = assert_cached_equals_eager(deps(), vec![app("w", &["d1"], EVERYTHING)]);
        assert!(
            relowered >= 4,
            "table, page, interface users and the enum ({relowered})"
        );
    }

    fn workspace(id: &str, src: &str) -> Vec<AppSource> {
        vec![app(id, &["d1", "d2", "d3"], src)]
    }

    #[test]
    fn workspace_code_that_uses_dependency_queries_and_pages_gives_the_complete_compile() {
        assert_cached_equals_eager(deps(), workspace("w", USES_DEPENDENCY_OBJECTS));
    }

    #[test]
    fn extending_a_page_that_other_pages_show_as_parts_relowers_those_pages() {
        let relowered = assert_cached_equals_eager(deps(), workspace("w", EXTENDS_A_PART_PAGE));
        assert!(
            relowered >= 3,
            "the page, and the pages that show it as a part ({relowered})"
        );
    }

    #[test]
    fn a_report_extension_and_an_interface_implementation_in_the_workspace() {
        assert_cached_equals_eager(deps(), workspace("w", EXTENDS_REPORT_AND_INTERFACE));
    }

    /// The complete check runs on several threads (the eager compile and the cache build). What a
    /// routine checks to does not depend on the thread, so the program is the one a single thread
    /// makes.
    #[test]
    fn the_complete_check_gives_the_same_program_on_one_thread_and_on_several() {
        let mut src = String::from(BASE);
        for i in 0..150 {
            src.push_str(&format!(
                "codeunit {} \"Gen {i}\" {{ procedure P{i}(var R: Record T): Integer begin R.A := {i}; R.DX := 1; exit(R.A); end; \
                 procedure Q{i}(): Integer begin exit({i}); end; }}\n",
                80000 + i
            ));
        }
        let apps = vec![app("d1", &[], &src)];
        let kinds = vec![AppKind::Dependency];
        let make = |workers| {
            compile_apps_with(
                apps.clone(),
                kinds.clone(),
                &BTreeSet::new(),
                None,
                Roots {
                    eager: true,
                    workers: Some(workers),
                    ..Default::default()
                },
            )
            .1
        };
        let (one, many) = (make(1), make(6));
        assert!(*one.program == *many.program, "the programs differ");
        assert!(one.unsupported == many.unsupported, "the refusals differ");
        assert!(one.program.objects.len() > 150);
    }

    // ---- layers (owner decision 2026-10-15) ----

    /// The layers of `layers` (each a list of apps, lowest first) built one on the other, and the
    /// complete compile of all the apps: the stack must give the same facts, blocks and parse
    /// problems for every object. Returns the number of objects a layer above the first overrides.
    pub(crate) fn assert_stack_equals_complete(layers: Vec<Vec<AppSource>>) -> usize {
        use crate::provided::{DepLayer, Stack};
        let symbols = BTreeSet::new();
        let mut all: Vec<AppSource> = Vec::new();
        let mut built: Vec<DepLayer> = Vec::new();
        let mut overrides = 0;
        for apps in layers {
            let below = all.len();
            all.extend(apps);
            if built.is_empty() {
                built.push(DepLayer::whole(lower_for_cache(all.clone(), &symbols, None)));
            } else {
                let refs: Vec<&dyn LayerSource> = built.iter().map(|l| l as &dyn LayerSource).collect();
                let stack = Stack::new(refs).expect("the layers fit");
                let layer =
                    crate::lower::lower_layer(all.clone(), below, &symbols, None, &stack).expect("a layer builds");
                drop(stack);
                overrides += layer.objects.iter().filter(|&&o| (o as usize) < layer.base_len).count();
                built.push(layer);
            }
        }
        let complete = lower_for_cache(all, &symbols, None);
        let refs: Vec<&dyn LayerSource> = built.iter().map(|l| l as &dyn LayerSource).collect();
        let stack = Stack::new(refs).expect("the layers fit");
        assert_eq!(stack.len(), complete.facts.len());
        for oi in 0..stack.len() {
            assert!(
                stack.facts(oi) == &complete.facts[oi],
                "facts of object {oi} `{}` differ",
                complete.facts[oi].name
            );
            assert!(
                stack.block(oi).unwrap() == complete.blocks[oi],
                "block of object {oi} `{}` differs",
                complete.facts[oi].name
            );
            assert!(
                stack.head(oi) == &complete.heads[oi],
                "head of object {oi} `{}` differs",
                complete.facts[oi].name
            );
        }
        assert_eq!(stack.parse_problems(), &complete.parse_problems[..]);
        // The heads a run moves into its program are the same, in object order.
        assert!(stack.heads_owned() == complete.heads, "the owned heads differ");
        overrides
    }

    use crate::provided::LayerSource;

    /// ADR-054 test plan item 2: the stored head of every object the run takes from `src` equals the head of that object
    /// as the run linked it.
    pub(crate) fn assert_heads_match(src: &dyn DepSource, run: &Lowered) {
        let mut compared = 0;
        for oi in 0..src.len() {
            if !run.provided[oi] {
                continue;
            }
            let linked = ir::ObjectHead::of(&run.program.objects[oi]);
            assert!(
                src.head(oi) == &linked,
                "the stored head of object {oi} `{}` differs from the linked object",
                linked.name
            );
            compared += 1;
        }
        assert!(compared > 0, "no object came from the source");
    }

    #[test]
    fn a_layer_that_changes_nothing_below_it_overrides_nothing() {
        let overrides = assert_stack_equals_complete(vec![
            vec![app("d1", &[], BASE)],
            vec![app(
                "p",
                &["d1"],
                "codeunit 60000 C { procedure P(): Integer begin exit(1); end; }",
            )],
        ]);
        assert_eq!(overrides, 0);
    }

    #[test]
    fn a_layer_with_a_table_extension_overrides_the_table_and_what_embeds_it() {
        let overrides =
            assert_stack_equals_complete(vec![vec![app("d1", &[], BASE)], vec![app("p", &["d1"], TABLE_EXT)]]);
        assert!(overrides >= 1, "the table is lowered again with the layer");
    }

    #[test]
    fn a_layer_with_extensions_and_implementers_equals_the_complete_compile() {
        let overrides = assert_stack_equals_complete(vec![
            vec![
                app("d1", &[], BASE),
                app("d2", &["d1"], OTHER),
                app("d3", &["d1"], MORE),
            ],
            vec![app("p", &["d1", "d2", "d3"], EVERYTHING)],
        ]);
        assert!(
            overrides >= 4,
            "table, page, interface users and the enum ({overrides})"
        );
    }

    #[test]
    fn a_layer_that_extends_and_implements_what_a_layer_below_it_added() {
        // Three layers: the third extends the table the second extended, and implements the
        // interface the first declared, so it overrides objects of both lower layers.
        assert_stack_equals_complete(vec![
            vec![app("d1", &[], BASE)],
            vec![app("p", &["d1"], EVERYTHING)],
            vec![app(
                "q",
                &["d1", "p"],
                r#"
tableextension 61000 QT extends T { fields { field(30; QX; Integer) { } } }
pageextension 61001 QP extends P { layout { addlast(Content) { field(QXC; Rec.QX) { } } } }
codeunit 61002 QImpl implements IFoo { procedure Bump(): Integer begin exit(3); end; }
enumextension 61003 QE extends E { value(2; Z) { Implementation = IFoo = QImpl; } }
codeunit 61004 QUse { procedure W() var R: Record T; begin R.QX := 1; R.WX := 2; R.DX := 3; end; }
"#,
            )],
        ]);
    }

    #[test]
    fn layers_for_queries_pages_parts_and_reports() {
        assert_stack_equals_complete(vec![
            vec![app("d1", &[], BASE), app("d3", &["d1"], MORE)],
            vec![app("p", &["d1", "d3"], EXTENDS_REPORT_AND_INTERFACE)],
            vec![app("q", &["d1", "d3", "p"], EXTENDS_A_PART_PAGE)],
        ]);
    }

    #[test]
    fn a_stack_refuses_layers_that_do_not_fit_each_other() {
        use crate::provided::{DepLayer, Stack};
        let one = DepLayer::whole(lower_for_cache(vec![app("d1", &[], BASE)], &BTreeSet::new(), None));
        let again = one.clone();
        // The second layer claims to start above 0 objects.
        let refs: Vec<&dyn LayerSource> = vec![&one, &again];
        assert!(Stack::new(refs).is_err());
        let mut gap = DepLayer {
            base_len: one.set.facts.len(),
            objects: vec![one.set.facts.len() as u32 + 1],
            set: DepSet::default(),
        };
        gap.set.facts.push(one.set.facts[0].clone());
        gap.set.blocks.push(one.set.blocks[0].clone());
        let refs: Vec<&dyn LayerSource> = vec![&one, &gap];
        assert!(Stack::new(refs).is_err());
    }

    /// A workspace on top of a stack of layers: the same program, refusals and call graph as the
    /// eager compile of all the apps.
    #[test]
    fn a_workspace_on_a_stack_of_layers_equals_the_eager_compile() {
        use crate::provided::{DepLayer, Stack};
        let symbols = BTreeSet::new();
        let l1 = vec![
            app("d1", &[], BASE),
            app("d2", &["d1"], OTHER),
            app("d3", &["d1"], MORE),
        ];
        let mut l12 = l1.clone();
        l12.push(app("p", &["d1", "d2", "d3"], EVERYTHING));
        let base = DepLayer::whole(lower_for_cache(l1.clone(), &symbols, None));
        let refs: Vec<&dyn LayerSource> = vec![&base];
        let layer =
            crate::lower::lower_layer(l12.clone(), l1.len(), &symbols, None, &Stack::new(refs).unwrap()).unwrap();
        let refs: Vec<&dyn LayerSource> = vec![&base, &layer];
        let stack = Stack::new(refs).unwrap();
        let ws_app = app(
            "w",
            &["d1", "p"],
            "tableextension 70000 WT2 extends T { fields { field(40; W2; Integer) { } } }\ncodeunit 70001 W { procedure G() var R: Record T; begin R.W2 := 1; R.WX := 2; end; }",
        );
        let mut apps = l12.clone();
        apps.push(ws_app);
        let kinds: Vec<AppKind> = (0..apps.len())
            .map(|i| {
                if i < l12.len() {
                    AppKind::Dependency
                } else {
                    AppKind::Workspace
                }
            })
            .collect();
        let roots = Roots {
            eager: true,
            ..Default::default()
        };
        let (_, eager) = compile_apps_with(apps.clone(), kinds.clone(), &symbols, None, roots.clone());
        let shared = Stack::from_shared(vec![
            std::sync::Arc::new(base.clone()) as std::sync::Arc<dyn LayerSource + Send>,
            std::sync::Arc::new(layer.clone()),
        ])
        .unwrap();
        let (_, cached) = compile_apps_cached(apps, kinds, &symbols, None, roots, std::sync::Arc::new(shared))
            .map_err(|e| e.1)
            .expect("the layers fit");
        assert!(*eager.program == *cached.program, "the programs differ");
        assert!(eager.unsupported == cached.unsupported, "the refusals differ");
        assert!(calls(&eager) == calls(&cached), "the call graphs differ");
        assert_eq!(eager.dependency, cached.dependency);
        assert_heads_match(&stack, &cached);
    }
}
