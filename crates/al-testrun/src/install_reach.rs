//! Whether an incremental compile of a warm server (ADR-029, `Compiled::update_files`) changed code the install
//! baseline can run. When it did not, the baseline of the old program is the baseline of the new one, and the server
//! keeps it in memory instead of running the install code again (`baseline_cache::carry`). The key of the file in the
//! compile cache does not change: a run of its own still finds the baseline by the full source.
//!
//! The reach is the static reach of `impact_static::Graph`, worked out over the workspace apps alone. The dependency
//! apps cannot name an object of an app that depends on them, so every edge of that graph from a dependency app (or
//! from the platform) into workspace code ends at an *entry*:
//!
//! - an event subscriber (an event raised anywhere, `Company-Initialize`'s and the install events included);
//! - a trigger (a codeunit's `OnRun`, run by an id from a variable or by a job; install and upgrade triggers);
//! - a routine of a codeunit that implements an interface (an interface call reaches every implementer);
//! - any routine of an object that is not a codeunit (table and table extension triggers run on every write of the
//!   table, pages, reports, xmlports and queries are run by id; enums and the rest have no other way in).
//!
//! Inside the workspace the graph's other edges either are calls (`Lowered::calls_of`), which the search follows, or
//! end at an entry (an event, an object run, a table written). So the workspace routines the install code can reach
//! are among those an entry reaches through calls: [`reach`]. An edit keeps the baseline only when every routine it
//! changed is outside that set, in the old program and in the new one, and it changed nothing but routines of
//! codeunits that are neither install nor upgrade codeunits. Anything else, or anything this module cannot compare,
//! runs the install code again.

use al_ir::{ObjKind, Object, Program, Routine};
use al_sema::lower::Lowered;
use std::collections::{HashMap, HashSet};

/// A routine as far as a run can observe it: its IR with the position ids made relative, the source positions those
/// ids stand for, and why it is refused.
#[derive(PartialEq)]
struct Face {
    routine: Routine,
    positions: Vec<Option<(u32, u32, u32, u32)>>,
    refused: Vec<String>,
}

fn face(program: &Program, routine: &Routine, refused: Option<&Vec<String>>) -> Face {
    let ids = routine.position_ids();
    let mut r = routine.clone();
    if let Some(&first) = ids.first() {
        r.shift_positions(0u32.wrapping_sub(first));
    }
    Face {
        routine: r,
        positions: ids
            .iter()
            .map(|&id| {
                program
                    .positions
                    .get(id as usize)
                    .map(|p| (p.file, p.line, p.col, p.end_line))
            })
            .collect(),
        refused: refused.cloned().unwrap_or_default(),
    }
}

/// A routine's name for matching the old and the new version of an object: name, kind, parameters, and the how-many-th
/// routine of that name it is.
type RoutineKey = (String, bool, usize, usize);

fn keys(o: &Object) -> Vec<RoutineKey> {
    let mut seen: HashMap<(String, bool, usize), usize> = HashMap::new();
    o.routines
        .iter()
        .map(|r| {
            let k = (r.name.to_lowercase(), r.is_trigger, r.params);
            let n = seen.entry(k.clone()).or_default();
            *n += 1;
            (k.0, k.1, k.2, *n)
        })
        .collect()
}

/// An object of an edit as it was before.
struct Old {
    object: usize,
    /// The object without its routines.
    shell: Object,
    faces: Vec<(RoutineKey, Face)>,
}

/// What [`after`] compares the new program with: the objects the edit compiles again as they were, and the workspace
/// routines the install code could reach in the old program.
pub(crate) struct Before {
    old: Vec<Old>,
    /// The routines of `old` the install code could reach.
    reached: Vec<(usize, usize)>,
}

/// The workspace routines the install code can reach (see the module comment): every entry, and what entries call.
fn reach(ws: &al_sema::Workspace, lowered: &Lowered) -> HashSet<(usize, usize)> {
    let objs = &lowered.program.objects;
    let workspace = |o: usize| !lowered.dependency.get(o).copied().unwrap_or(true);
    let mut seen: HashSet<(usize, usize)> = HashSet::new();
    let mut stack: Vec<(usize, usize)> = Vec::new();
    for oi in (0..objs.len()).filter(|&o| workspace(o)) {
        let o = &objs[oi];
        let implements = ws.objects.get(oi).is_none_or(|w| !w.implements.is_empty());
        for (ri, r) in o.routines.iter().enumerate() {
            let entry = o.kind != ObjKind::Codeunit
                || r.is_trigger
                || implements
                || install_or_upgrade(o)
                || r.attributes
                    .iter()
                    .any(|a| a.name.eq_ignore_ascii_case("EventSubscriber"));
            if entry && seen.insert((oi, ri)) {
                stack.push((oi, ri));
            }
        }
    }
    let mut calls = Vec::new();
    while let Some((o, r)) = stack.pop() {
        calls.clear();
        lowered.calls_into(o, r, &mut calls);
        for &(co, cr) in &calls {
            if workspace(co) && cr < objs[co].routines.len() && seen.insert((co, cr)) {
                stack.push((co, cr));
            }
        }
    }
    seen
}

fn install_or_upgrade(o: &Object) -> bool {
    o.kind == ObjKind::Codeunit
        && o.properties.iter().any(|(k, v)| {
            k.eq_ignore_ascii_case("Subtype")
                && (v.trim().eq_ignore_ascii_case("Install") || v.trim().eq_ignore_ascii_case("Upgrade"))
        })
}

fn shell(o: &Object) -> Object {
    let mut s = o.clone();
    s.routines = Vec::new();
    s
}

fn faces(program: &Program, lowered: &Lowered, oi: usize) -> Vec<(RoutineKey, Face)> {
    let o = &program.objects[oi];
    let refused = lowered.unsupported.get(oi);
    keys(o)
        .into_iter()
        .zip(&o.routines)
        .enumerate()
        .map(|(ri, (k, r))| (k, face(program, r, refused.and_then(|f| f.get(ri)))))
        .collect()
}

/// Takes what [`after`] needs of the program before the objects `dirty` are compiled again.
pub(crate) fn before(ws: &al_sema::Workspace, lowered: &Lowered, dirty: &[usize]) -> Result<Before, String> {
    let program = &lowered.program;
    let mut old = Vec::new();
    for &oi in dirty {
        if lowered.dependency.get(oi).copied().unwrap_or(true) {
            return Err("the edit changed an object of a dependency app".into());
        }
        old.push(Old {
            object: oi,
            shell: shell(&program.objects[oi]),
            faces: faces(program, lowered, oi),
        });
    }
    let reach = reach(ws, lowered);
    let mut reached = Vec::new();
    for &oi in dirty {
        for ri in 0..program.objects[oi].routines.len() {
            if reach.contains(&(oi, ri)) {
                reached.push((oi, ri));
            }
        }
    }
    Ok(Before { old, reached })
}

/// `Ok` when the edit `before` was taken from cannot change what the install code does; else why it can.
pub(crate) fn after(ws: &al_sema::Workspace, lowered: &Lowered, before: Before) -> Result<(), String> {
    let program = &lowered.program;
    let new_reach = reach(ws, lowered);
    for old in &before.old {
        let oi = old.object;
        let o = &program.objects[oi];
        let what = || format!("{} {} \"{}\"", o.kind.label(), o.id.unwrap_or_default(), o.name);
        let new_faces = faces(program, lowered, oi);
        let same_shell = shell(o) == old.shell;
        let mut changed_old: Vec<usize> = Vec::new();
        let mut changed_new: Vec<usize> = Vec::new();
        let mut matched = vec![false; old.faces.len()];
        let by_key: HashMap<&RoutineKey, usize> = old.faces.iter().enumerate().map(|(i, (k, _))| (k, i)).collect();
        for (ri, (k, f)) in new_faces.iter().enumerate() {
            match by_key.get(k) {
                Some(&oldi) => {
                    matched[oldi] = true;
                    if !same_shell || old.faces[oldi].1 != *f {
                        changed_old.push(oldi);
                        changed_new.push(ri);
                    }
                }
                None => changed_new.push(ri),
            }
        }
        changed_old.extend((0..old.faces.len()).filter(|&i| !matched[i]));
        // An object without routines whose declarations changed.
        if !same_shell && changed_new.is_empty() && changed_old.is_empty() {
            return Err(format!("the edit changed the declarations of {}", what()));
        }
        if changed_new.is_empty() && changed_old.is_empty() {
            continue;
        }
        if o.kind != ObjKind::Codeunit || old.shell.kind != ObjKind::Codeunit {
            return Err(format!("the edit changed {}", what()));
        }
        if install_or_upgrade(o) || install_or_upgrade(&old.shell) {
            return Err(format!("the edit changed the install or upgrade codeunit {}", what()));
        }
        if let Some(&ri) = changed_old.iter().find(|&&ri| before.reached.contains(&(oi, ri))) {
            return Err(format!(
                "the install code can reach {}, which the edit changed",
                routine_name(&old.faces[ri].0, &what())
            ));
        }
        if let Some(&ri) = changed_new.iter().find(|&&ri| new_reach.contains(&(oi, ri))) {
            return Err(format!(
                "the install code can reach {}, which the edit changed",
                routine_name(&new_faces[ri].0, &what())
            ));
        }
    }
    Ok(())
}

fn routine_name(k: &RoutineKey, object: &str) -> String {
    format!("{} of {object}", k.0)
}
