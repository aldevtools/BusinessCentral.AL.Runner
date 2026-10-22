//! The reach report (`AL_REACH_REPORT=1`): how much of the compile the run needs.
//!
//! The demand-driven compile checks and lowers every routine something reaches. This report
//! reads the finished compile (the call graph, the refusal reasons, the stubs) and says where
//! the reached routines come from and how many of them only matter through a routine that
//! lowering refuses: any test that reaches a refused routine is not run, so code reached only
//! through it never executes by that path.
//!
//! A routine is *reached* when it is not a stub. Weight is the size in bytes of the routine's
//! lowered form as JSON, a proxy for its check and lowering cost.

use al_sema::lower::Lowered;
use al_sema::workspace::Workspace;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write;

const STUB: &str = "not compiled: no code reached it";

struct Graph {
    /// First flat index of each object's routines.
    base: Vec<usize>,
    /// Per flat index: (object, routine).
    at: Vec<(usize, usize)>,
    reached: Vec<bool>,
    refused: Vec<bool>,
    weight: Vec<u32>,
    dep: Vec<bool>,
    edges: Vec<Vec<usize>>,
    incoming: Vec<u32>,
}

impl Graph {
    fn id(&self, o: usize, r: usize) -> usize {
        self.base[o] + r
    }
}

fn build(lowered: &Lowered) -> Graph {
    let objects = &lowered.program.objects;
    let mut base = Vec::with_capacity(objects.len());
    let mut at = Vec::new();
    for (o, obj) in objects.iter().enumerate() {
        base.push(at.len());
        for r in 0..obj.routines.len() {
            at.push((o, r));
        }
    }
    let n = at.len();
    let mut g = Graph {
        base,
        at,
        reached: vec![false; n],
        refused: vec![false; n],
        weight: vec![0; n],
        dep: vec![false; n],
        edges: vec![Vec::new(); n],
        incoming: vec![0; n],
    };
    for i in 0..n {
        let (o, r) = g.at[i];
        let reasons = &lowered.unsupported[o][r];
        // A stub's reason comes after the object's global reasons, if it has any.
        let stub = reasons.iter().any(|s| s.starts_with(STUB));
        g.reached[i] = !stub;
        g.refused[i] = !stub && !reasons.is_empty();
        g.dep[i] = lowered.dependency[o];
        g.edges[i] = lowered
            .calls_of(o, r)
            .iter()
            .filter(|&&(co, cr)| co < objects.len() && cr < objects[co].routines.len())
            .map(|&(co, cr)| g.base[co] + cr)
            .collect();
    }
    for i in 0..n {
        for &t in &g.edges[i] {
            g.incoming[t] += 1;
        }
    }
    // Dependency routines carry no source positions; the size of the lowered routine stands in.
    for i in 0..n {
        let (o, r) = g.at[i];
        if g.reached[i] {
            g.weight[i] = serde_json::to_string(&objects[o].routines[r]).map_or(0, |t| t.len() as u32);
        }
    }
    g
}

fn closure(g: &Graph, from: &[usize], skip: &[bool], prune_refused: bool) -> Vec<usize> {
    let mut seen: HashSet<usize> = HashSet::new();
    let mut stack: Vec<usize> = from.iter().copied().filter(|&i| g.reached[i] && !skip[i]).collect();
    let mut out = Vec::new();
    while let Some(i) = stack.pop() {
        if !seen.insert(i) {
            continue;
        }
        out.push(i);
        if prune_refused && g.refused[i] {
            continue;
        }
        for &t in &g.edges[i] {
            if g.reached[t] && !skip[t] && !seen.contains(&t) {
                stack.push(t);
            }
        }
    }
    out
}

fn is_subscriber(lowered: &Lowered, (o, r): (usize, usize)) -> bool {
    lowered.program.objects[o].routines[r]
        .attributes
        .iter()
        .any(|a| a.name.eq_ignore_ascii_case("EventSubscriber"))
}

fn label(lowered: &Lowered, g: &Graph, i: usize) -> String {
    let (o, r) = g.at[i];
    let obj = &lowered.program.objects[o];
    format!("{} \"{}\" . {}", obj.kind, obj.name, obj.routines[r].name)
}

/// The report, as text.
pub fn report(ws: &Workspace, lowered: &Lowered) -> String {
    let _ = ws;
    let g = build(lowered);
    let objects = &lowered.program.objects;
    let n = g.at.len();
    let mut out = String::new();
    let w = |ids: &[usize]| ids.iter().map(|&i| u64::from(g.weight[i])).sum::<u64>();

    let reached_dep: Vec<usize> = (0..n).filter(|&i| g.reached[i] && g.dep[i]).collect();
    let reached_ws: Vec<usize> = (0..n).filter(|&i| g.reached[i] && !g.dep[i]).collect();
    let refused_dep: Vec<usize> = reached_dep.iter().copied().filter(|&i| g.refused[i]).collect();
    let refused_ws: Vec<usize> = reached_ws.iter().copied().filter(|&i| g.refused[i]).collect();
    let all_dep = (0..n).filter(|&i| g.dep[i]).count();
    let _ = writeln!(out, "== reach report ==");
    let _ = writeln!(
        out,
        "routines: dependency {} listed, {} reached (weight {}), {} of the reached refused; workspace {} reached (weight {}), {} refused",
        all_dep,
        reached_dep.len(),
        w(&reached_dep),
        refused_dep.len(),
        reached_ws.len(),
        w(&reached_ws),
        refused_ws.len()
    );

    // Tests: workspace test codeunits; a test is blocked when its call closure meets a refusal.
    let mut tests = Vec::new();
    let mut handlers_etc = Vec::new();
    for (o, obj) in objects.iter().enumerate() {
        if lowered.dependency[o] {
            continue;
        }
        let is_test_cu = obj.is_kind(al_ir::ObjKind::Codeunit)
            && obj
                .properties
                .iter()
                .any(|(k, v)| k.eq_ignore_ascii_case("Subtype") && v.eq_ignore_ascii_case("Test"));
        if !is_test_cu {
            continue;
        }
        for (r, rt) in obj.routines.iter().enumerate() {
            let id = g.id(o, r);
            if !g.reached[id] {
                continue;
            }
            if rt.attributes.iter().any(|a| a.name.eq_ignore_ascii_case("Test")) {
                tests.push(id);
            } else if rt.is_trigger
                || rt
                    .attributes
                    .iter()
                    .any(|a| a.name.to_ascii_lowercase().ends_with("handler"))
            {
                handlers_etc.push(id);
            }
        }
    }
    let blocked = |root: usize| -> bool {
        closure(&g, &[root], &vec![false; n], true)
            .iter()
            .any(|&i| g.refused[i])
    };
    let (mut runnable, mut stopped) = (Vec::new(), 0usize);
    for &t in &tests {
        if blocked(t) {
            stopped += 1;
        } else {
            runnable.push(t);
        }
    }
    let _ = writeln!(
        out,
        "tests: {} reached [Test] routines, {} reach a refused routine (NotRun), {} can run",
        tests.len(),
        stopped,
        runnable.len()
    );
    let no_skip = vec![false; n];
    let can_run = closure(
        &g,
        &runnable.iter().chain(&handlers_etc).copied().collect::<Vec<_>>(),
        &no_skip,
        false,
    );
    let can_run_dep: Vec<usize> = can_run.iter().copied().filter(|&i| g.dep[i]).collect();
    let _ = writeln!(
        out,
        "dependency routines the runnable tests (and their triggers/handlers) can reach by static calls: {} of {} reached (weight {} of {})",
        can_run_dep.len(),
        reached_dep.len(),
        w(&can_run_dep),
        w(&reached_dep)
    );

    // Layers.
    let mut claimed = vec![false; n];
    let layer = |name: &str, from: Vec<usize>, claimed: &mut Vec<bool>, out: &mut String| -> usize {
        let c = closure(&g, &from, claimed, false);
        let dep: Vec<usize> = c.iter().copied().filter(|&i| g.dep[i]).collect();
        let _ = writeln!(
            out,
            "layer {name}: {} roots, +{} dependency routines (weight {})",
            from.len(),
            dep.len(),
            w(&dep)
        );
        for &i in &c {
            claimed[i] = true;
        }
        dep.len()
    };
    let ws_roots: Vec<usize> = (0..n).filter(|&i| g.reached[i] && !g.dep[i]).collect();
    layer(
        "workspace code (every workspace routine is checked)",
        ws_roots.clone(),
        &mut claimed,
        &mut out,
    );
    let mut install = Vec::new();
    for (o, obj) in objects.iter().enumerate() {
        if !lowered.dependency[o] || !obj.is_kind(al_ir::ObjKind::Codeunit) {
            continue;
        }
        let install_cu = obj
            .properties
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("Subtype") && v.eq_ignore_ascii_case("Install"));
        let ci = obj.id == Some(2) && obj.name.eq_ignore_ascii_case("Company-Initialize");
        for (r, rt) in obj.routines.iter().enumerate() {
            let id = g.id(o, r);
            let name = rt.name.to_ascii_lowercase();
            if g.reached[id]
                && ((install_cu && matches!(name.as_str(), "oninstallappercompany" | "oninstallapperdatabase"))
                    || (ci && name == "onrun"))
            {
                install.push(id);
            }
        }
    }
    layer("install baseline roots", install.clone(), &mut claimed, &mut out);

    // The rest: heads are routines nothing remaining calls; grouped by what kind of thing they are.
    let rest: Vec<usize> = (0..n).filter(|&i| g.reached[i] && g.dep[i] && !claimed[i]).collect();
    let rest_set: HashSet<usize> = rest.iter().copied().collect();
    let mut has_rest_caller: HashSet<usize> = HashSet::new();
    for &i in &rest {
        for &t in &g.edges[i] {
            if t != i && rest_set.contains(&t) {
                has_rest_caller.insert(t);
            }
        }
    }
    let heads: Vec<usize> = rest.iter().copied().filter(|i| !has_rest_caller.contains(i)).collect();
    let group = |i: usize| -> String {
        let (o, r) = g.at[i];
        let obj = &objects[o];
        let rt = &obj.routines[r];
        let what = if is_subscriber(lowered, (o, r)) {
            "event subscriber".to_string()
        } else if rt.is_trigger {
            "trigger".to_string()
        } else {
            "procedure".to_string()
        };
        format!("{} {}", obj.kind, what)
    };
    let mut groups: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for &h in &heads {
        groups.entry(group(h)).or_default().push(h);
    }
    let _ = writeln!(
        out,
        "layer activations: {} dependency routines left, {} heads (nothing left calls them):",
        rest.len(),
        heads.len()
    );
    let mut rows = Vec::new();
    let mut taken = claimed.clone();
    let mut keys: Vec<_> = groups.keys().cloned().collect();
    keys.sort();
    for k in keys {
        let hs = &groups[&k];
        let c = closure(&g, hs, &taken, false);
        for &i in &c {
            taken[i] = true;
        }
        rows.push((c.len(), k, hs.len(), w(&c)));
    }
    rows.sort_by_key(|a| std::cmp::Reverse(a.0));
    for (c, k, hs, wt) in &rows {
        let _ = writeln!(out, "  {k:<34} {hs:>6} heads  +{c:>6} routines  weight {wt}");
    }
    let leftover = rest.iter().filter(|&&i| !taken[i]).count();
    let _ = writeln!(out, "  (cycles without a head: {leftover})");

    // Biggest single heads.
    let mut big: Vec<(usize, usize)> = Vec::new();
    for &h in &heads {
        let c = closure(&g, &[h], &claimed, false);
        big.push((c.len(), h));
    }
    big.sort_by(|a, b| b.cmp(a));
    let _ = writeln!(out, "biggest single activation heads (total closure, not exclusive):");
    for (c, h) in big.iter().take(15) {
        let _ = writeln!(out, "  {c:>6}  {}", label(lowered, &g, *h));
    }

    // Objects with most reached routines.
    let mut per_obj: HashMap<usize, (usize, u64)> = HashMap::new();
    for &i in &reached_dep {
        let e = per_obj.entry(g.at[i].0).or_default();
        e.0 += 1;
        e.1 += u64::from(g.weight[i]);
    }
    let mut per_kind: BTreeMap<String, (usize, usize, u64)> = BTreeMap::new();
    for (&o, &(c, wt)) in &per_obj {
        let e = per_kind.entry(objects[o].kind_text()).or_default();
        e.0 += 1;
        e.1 += c;
        e.2 += wt;
    }
    let _ = writeln!(
        out,
        "reached dependency routines by object kind (objects, routines, weight):"
    );
    for (k, (ob, c, wt)) in &per_kind {
        let _ = writeln!(out, "  {k:<16} {ob:>6} {c:>7} {wt:>9}");
    }
    let mut top: Vec<_> = per_obj.iter().collect();
    top.sort_by_key(|a| std::cmp::Reverse(a.1.0));
    let _ = writeln!(out, "objects with the most reached routines:");
    for &(&o, &(c, wt)) in top.iter().take(12) {
        let _ = writeln!(
            out,
            "  {c:>5} routines weight {wt:>6}  {} \"{}\"",
            objects[o].kind, objects[o].name
        );
    }

    // Refusals among the reached.
    let mut reasons: HashMap<String, (usize, usize, u64, String)> = HashMap::new();
    for &i in refused_dep.iter().chain(&refused_ws) {
        let (o, r) = g.at[i];
        let raw = &lowered.unsupported[o][r][0];
        let key = crate::refusals::normalise_reason(raw);
        let e = reasons.entry(key).or_insert((
            0,
            0,
            0,
            format!("{} . {}", objects[o].name, objects[o].routines[r].name),
        ));
        e.0 += 1;
        if g.dep[i] {
            e.1 += 1;
        }
        e.2 += u64::from(g.weight[i]);
    }
    let mut rs: Vec<_> = reasons.into_iter().collect();
    rs.sort_by_key(|a| std::cmp::Reverse(a.1.0));
    let _ = writeln!(
        out,
        "first refusal reasons of reached routines (routines, of them dependency, weight): {} kinds",
        rs.len()
    );
    for (k, (c, d, wt, ex)) in rs.iter().take(20) {
        let _ = writeln!(
            out,
            "  {c:>5} {d:>5} {wt:>6}  {}   [{ex}]",
            crate::refusals::clip(k, 100)
        );
    }

    // Which objects hold the object-level refusals (global variables, data items): every routine of
    // such an object is refused with the same first reason, whatever its body does.
    let mut by_object: HashMap<usize, (usize, String)> = HashMap::new();
    for &i in &refused_dep {
        let (o, r) = g.at[i];
        let first = &lowered.unsupported[o][r][0];
        if first.starts_with("global `")
            || first.starts_with("dataitem `")
            || first.starts_with("table element `")
            || first.starts_with("variable `")
        {
            let e = by_object.entry(o).or_insert((0, first.clone()));
            e.0 += 1;
        }
    }
    let object_level: usize = by_object.values().map(|e| e.0).sum();
    let _ = writeln!(
        out,
        "object-level refusals (global/dataitem/element): {} objects, {} reached dependency routines ({} of the refused)",
        by_object.len(),
        object_level,
        refused_dep.len()
    );
    let mut raw: HashMap<String, (usize, usize)> = HashMap::new();
    for (c, first) in by_object.values() {
        let tail = first.split_once(": ").map_or(first.as_str(), |x| x.1).to_string();
        let e = raw.entry(tail).or_default();
        e.0 += 1;
        e.1 += c;
    }
    let mut raw: Vec<_> = raw.into_iter().collect();
    raw.sort_by_key(|a| std::cmp::Reverse(a.1.1));
    for (k, (ob, c)) in raw.iter().take(25) {
        let _ = writeln!(
            out,
            "  {ob:>4} objects {c:>5} routines  {}",
            crate::refusals::clip(k, 110)
        );
    }

    // Pruning: stop at refused routines.
    let mut roots: Vec<usize> = ws_roots.clone();
    roots.extend(&install);
    roots.extend(&heads);
    roots.extend((0..n).filter(|&i| g.reached[i] && g.dep[i] && !taken[i] && !claimed[i]));
    let kept = closure(&g, &roots, &no_skip, true);
    let kept_dep: Vec<usize> = kept.iter().copied().filter(|&i| g.dep[i]).collect();
    let _ = writeln!(
        out,
        "pruned at refused routines (call edges only; activations stay roots): {} of {} dependency routines stay reached (weight {} of {}); {} (weight {}) only matter through a refused routine's calls",
        kept_dep.len(),
        reached_dep.len(),
        w(&kept_dep),
        w(&reached_dep),
        reached_dep.len() - kept_dep.len(),
        w(&reached_dep) - w(&kept_dep)
    );
    // The same, but a routine can be refused only if its first reason is not about reach itself.
    let _ = (&heads, &install);
    out
}

/// The routines that lowering refuses among what `roots` reach through their calls (the call graph of the compile,
/// not stopping at a refused routine), grouped by their folded reason ([`crate::refusals::normalise_reason`]), the
/// reason with the most routines first, then by reason. What the company data codeunit could stop at
/// (`--company-data demotool`): a run stops at the first one it meets, this lists them all at once.
pub fn refused_reached_from(lowered: &Lowered, roots: &[(usize, usize)]) -> Vec<(String, Vec<String>)> {
    let g = build(lowered);
    let from: Vec<usize> = roots
        .iter()
        .filter(|&&(o, r)| o < lowered.program.objects.len() && r < lowered.program.objects[o].routines.len())
        .map(|&(o, r)| g.id(o, r))
        .collect();
    let skip = vec![false; g.at.len()];
    group_refused(lowered, &g, closure(&g, &from, &skip, false))
}

/// The refused routines of app `app` (index in the program), grouped as [`refused_reached_from`] groups them. For a
/// compile that took the app from the compile cache, whose call graph is not recorded.
pub fn refused_in_app(lowered: &Lowered, app: u32) -> Vec<(String, Vec<String>)> {
    let g = build(lowered);
    let all = (0..g.at.len())
        .filter(|&i| lowered.program.objects[g.at[i].0].app == app)
        .collect();
    group_refused(lowered, &g, all)
}

fn group_refused(lowered: &Lowered, g: &Graph, routines: Vec<usize>) -> Vec<(String, Vec<String>)> {
    let mut by: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for i in routines {
        if !g.refused[i] {
            continue;
        }
        let (o, r) = g.at[i];
        let reason = lowered.unsupported[o][r].first().cloned().unwrap_or_default();
        by.entry(crate::refusals::normalise_reason(&reason))
            .or_default()
            .push(label(lowered, g, i));
    }
    let mut out: Vec<(String, Vec<String>)> = by.into_iter().collect();
    for (_, v) in &mut out {
        v.sort();
    }
    out.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then_with(|| a.0.cmp(&b.0)));
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_refused_routines_a_root_reaches_are_listed_by_reason() {
        let src = r#"
codeunit 50130 "Build" { trigger OnRun() begin A(); B(); end;
  local procedure A() begin SerialNumber(); end;
  local procedure B() begin C(); end;
  local procedure C() begin SerialNumber(); end; }
codeunit 50131 "Other" { trigger OnRun() begin SerialNumber(); end; }
"#;
        let (_, l) = al_sema::lower::compile(
            vec![al_sema::workspace::AppSource {
                id: "a".into(),
                name: "a".into(),
                dependencies: vec![],
                files: vec![("f.al".into(), src.into())],
            }],
            &Default::default(),
        );
        let oi = l.program.objects.iter().position(|o| o.name == "Build").unwrap();
        let ri = l.program.objects[oi]
            .routines
            .iter()
            .position(|r| r.name == "OnRun")
            .unwrap();
        let got = super::refused_reached_from(&l, &[(oi, ri)]);
        assert_eq!(got.len(), 1, "{got:?}");
        assert_eq!(got[0].1.len(), 2, "A and C, not codeunit Other: {got:?}");
        assert!(got[0].1.iter().all(|r| r.contains("\"Build\"")), "{got:?}");
    }
}
