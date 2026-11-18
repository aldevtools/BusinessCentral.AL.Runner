//! A compile as the workspace layer of the compile cache stores it (ADR-022 amendment 2026-11-15, decision 3), and the
//! `Lowered` a stored compile gives back (decision 4): equal to the one a fresh compile of the same inputs gives, except
//! for the timings and the work counts. The objects lowered here (the workspace's and the dependency objects lowered
//! again) are stored as blocks (`DepBlock`, in the al-cache layer format); everything else a later step reads is in
//! [`StoredLowered`]. Both lists are closed by construction: `Lowered`, `Program` and `DrivenCtx` are destructured
//! without `..`, and a field that is not stored is named with the reason.

use super::{Calls, CompileTimings, DrivenCtx, LowerWork, Lowered, ObjectHash, Refusals};
use crate::check::posmap::FxMap as HashMap;
use crate::provided::{DepBlock, DepSource};
use crate::workspace::stored::StoredDiagnostic;
use al_builtins::refusal::Refusal;
use al_ir as ir;
use std::sync::Arc;

/// What a stored compile keeps of `DrivenCtx`, its maps as sorted lists.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct StoredDriven {
    page_extensions: Vec<(usize, Vec<usize>)>,
    request_page_order: Vec<(usize, Vec<usize>)>,
    page_parts: Vec<Vec<usize>>,
    page_runs: Vec<Vec<usize>>,
    runs_triggers: Vec<bool>,
    report_extension: Vec<Vec<usize>>,
    dependency: Vec<bool>,
    codeunit: Vec<bool>,
    triggers_only: Vec<bool>,
    routine_roles: Vec<Vec<u8>>,
}

fn sorted_map(m: &HashMap<usize, Vec<usize>>) -> Vec<(usize, Vec<usize>)> {
    let mut v: Vec<(usize, Vec<usize>)> = m.iter().map(|(k, v)| (*k, v.clone())).collect();
    v.sort_unstable();
    v
}

impl StoredDriven {
    fn of(d: &DrivenCtx) -> StoredDriven {
        let DrivenCtx {
            page_extensions,
            request_page_order,
            page_parts,
            page_runs,
            runs_triggers,
            report_extension,
            dependency,
            codeunit,
            triggers_only,
            routine_roles,
        } = d;
        StoredDriven {
            page_extensions: sorted_map(page_extensions),
            request_page_order: sorted_map(request_page_order),
            page_parts: page_parts.clone(),
            page_runs: page_runs.clone(),
            runs_triggers: runs_triggers.clone(),
            report_extension: report_extension.clone(),
            dependency: dependency.clone(),
            codeunit: codeunit.clone(),
            triggers_only: triggers_only.clone(),
            routine_roles: routine_roles.clone(),
        }
    }

    fn into_driven(self) -> DrivenCtx {
        let StoredDriven {
            page_extensions,
            request_page_order,
            page_parts,
            page_runs,
            runs_triggers,
            report_extension,
            dependency,
            codeunit,
            triggers_only,
            routine_roles,
        } = self;
        DrivenCtx {
            page_extensions: page_extensions.into_iter().collect(),
            request_page_order: request_page_order.into_iter().collect(),
            page_parts,
            page_runs,
            runs_triggers,
            report_extension,
            dependency,
            codeunit,
            triggers_only,
            routine_roles,
        }
    }
}

/// What a stored compile keeps of `Lowered` and of its `Program` besides the blocks of the objects lowered here.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct StoredLowered {
    /// `Program::apps` as the compile made them, before the run applies the manifests (`apply_manifests`).
    pub apps: Vec<ir::AppInfo>,
    pub dotnet_types: Vec<ir::DotNetDecl>,
    /// `Program::sources` as the compile made them, before the run adds the lines of an app that includes its source.
    pub sources: Vec<ir::SourceFile>,
    /// How many ids `Program::positions` has, holes included.
    pub positions: usize,
    pub uncompiled_subscribers: Vec<ir::UncompiledSubscriber>,
    pub tdd_stubs: Vec<String>,
    pub assumes: Vec<al_builtins::assumptions::Assumption>,
    /// Per object lowered here, the refusals; empty for an object the cache supplies.
    pub unsupported: Vec<Vec<Vec<Refusal>>>,
    pub refusal_at: Vec<Vec<Option<ir::SrcPos>>>,
    /// Per object lowered here, the calls and the count of direct ones; empty for an object the cache supplies.
    pub calls: Vec<Vec<Vec<(usize, usize)>>>,
    pub ndirect: Vec<Vec<u32>>,
    pub dependency: Vec<bool>,
    pub provided: Vec<bool>,
    pub driven: Option<StoredDriven>,
    pub al_diagnostics: Vec<StoredDiagnostic>,
    pub pos_ranges: Vec<(u32, u32)>,
    pub checker_diags: Vec<StoredDiagnostic>,
    pub skipped: Vec<bool>,
    pub left_out: Vec<bool>,
    pub tdd_sites: Vec<(usize, String)>,
    pub tdd_needs: Vec<crate::check::tdd_decl::TddNeed>,
    pub object_hashes: Vec<Option<ObjectHash>>,
}

/// The parts of a stored compile: [`StoredLowered`], and the block of each object lowered here, by program index.
pub type StoredParts = (StoredLowered, Vec<(u32, DepBlock)>);

/// The parts of `lowered` a stored compile keeps (see the module). `apps` and `sources` are the program's before the run
/// changed them (`StoredLowered::apps`). An error says the compile cannot be stored (it was not a complete compile of
/// the workspace, or its positions are not in object order).
pub fn store(lowered: &Lowered, apps: Vec<ir::AppInfo>, sources: Vec<ir::SourceFile>) -> Result<StoredParts, String> {
    let Lowered {
        // A measurement, not an output.
        timings: _,
        program,
        unsupported,
        refusal_at,
        calls,
        dependency,
        provided,
        driven,
        drivers,
        exported,
        al_diagnostics,
        pos_ranges,
        checker_diags,
        skipped,
        left_out,
        tdd_sites,
        tdd_needs,
        object_hashes,
        // The decoder of the dependency objects: a hit makes its own over the same layers.
        cached_objects: _,
        // How much this compile lowered: a hit lowers nothing on purpose.
        work: _,
    } = lowered;
    if exported.is_some() || !drivers.is_empty() {
        return Err("a compile that exports facts or copied the drivers is not a run's compile".into());
    }
    let ir::Program {
        objects,
        apps: _,
        dotnet_types,
        sources: _,
        positions,
        uncompiled_subscribers,
        tdd_stubs,
        assumes,
    } = &**program;
    let n = objects.len();
    if [provided.len(), dependency.len(), pos_ranges.len(), refusal_at.len()]
        .iter()
        .any(|&l| l != n)
    {
        return Err("the per-object lists do not have one entry per object".into());
    }
    let mut next = 0u32;
    for &(start, len) in pos_ranges {
        if start != next {
            return Err("the source positions are not in object order".into());
        }
        next += len;
    }
    if next as usize != positions.len() {
        return Err("the source positions do not cover the table".into());
    }
    let mut blocks = Vec::new();
    for oi in (0..n).filter(|&oi| !provided[oi]) {
        let object = objects.get_shared(oi).ok_or("an object lowered here is not made")?;
        let (start, len) = pos_ranges[oi];
        blocks.push((
            oi as u32,
            DepBlock {
                object: Some(Arc::clone(object)),
                pos_start: start,
                positions: (start..start + len).map(|id| positions[id as usize]).collect(),
            },
        ));
    }
    let (own_calls, ndirect) = (calls.own(), calls.own_ndirect());
    let stored = StoredLowered {
        apps,
        dotnet_types: dotnet_types.clone(),
        sources,
        positions: positions.len(),
        uncompiled_subscribers: uncompiled_subscribers.clone(),
        tdd_stubs: tdd_stubs.clone(),
        assumes: assumes.clone(),
        unsupported: unsupported.own().to_vec(),
        refusal_at: refusal_at.clone(),
        calls: own_calls.to_vec(),
        ndirect: ndirect.to_vec(),
        dependency: dependency.clone(),
        provided: provided.clone(),
        driven: driven.as_ref().map(StoredDriven::of),
        al_diagnostics: al_diagnostics.iter().map(StoredDiagnostic::of).collect(),
        pos_ranges: pos_ranges.clone(),
        checker_diags: checker_diags.iter().map(StoredDiagnostic::of).collect(),
        skipped: skipped.clone(),
        left_out: left_out.clone(),
        tdd_sites: tdd_sites.clone(),
        tdd_needs: tdd_needs.clone(),
        object_hashes: object_hashes.clone(),
    };
    Ok((stored, blocks))
}

/// The `Lowered` of a stored compile: `stored`, the objects lowered here from `blocks` (by program index, every object
/// that `stored.provided` does not mark), and the objects the cache supplies from `src` (the dependency layers the run
/// opened), decoded on first touch as in a fresh compile. `None`: the compile had no dependency layer. An error says the
/// parts do not fit each other; nothing is used then.
pub fn restore(
    stored: StoredLowered,
    blocks: Vec<(u32, DepBlock)>,
    src: Option<Arc<dyn DepSource + Send>>,
) -> Result<Lowered, String> {
    let StoredLowered {
        apps,
        dotnet_types,
        sources,
        positions: n_positions,
        uncompiled_subscribers,
        tdd_stubs,
        assumes,
        unsupported,
        refusal_at,
        calls,
        ndirect,
        dependency,
        provided,
        driven,
        al_diagnostics,
        pos_ranges,
        checker_diags,
        skipped,
        left_out,
        tdd_sites,
        tdd_needs,
        object_hashes,
    } = stored;
    let n = provided.len();
    let lists = [
        dependency.len(),
        pos_ranges.len(),
        refusal_at.len(),
        unsupported.len(),
        calls.len(),
        ndirect.len(),
    ];
    if lists.iter().any(|&l| l != n) {
        return Err("the per-object lists do not have one entry per object".into());
    }
    let supplied = provided.iter().filter(|p| **p).count();
    // A supplied object is one the layers hold.
    let layer_objects = src.as_ref().map_or(0, |s| s.len());
    if provided.iter().skip(layer_objects).any(|p| *p) {
        return Err("an object the layers do not hold is marked as supplied".into());
    }
    let mut objects: Vec<Option<ir::Object>> = (0..n).map(|_| None).collect();
    let mut positions = ir::Positions::with_capacity(n_positions);
    let mut blocks = blocks.into_iter().peekable();
    for oi in 0..n {
        let (start, len) = pos_ranges[oi];
        if start as usize != positions.len() {
            return Err("the source positions are not in object order".into());
        }
        if provided[oi] {
            positions.push_hole(len, oi);
            continue;
        }
        let (at, block) = blocks.next().ok_or("an object lowered here has no block")?;
        if at as usize != oi || block.pos_start != start || block.positions.len() != len as usize {
            return Err(format!("the block of object {oi} does not fit"));
        }
        let object = block.object.ok_or("a block without its object")?;
        objects[oi] = Some(Arc::unwrap_or_clone(object));
        positions.extend_from_slice(&block.positions);
    }
    if blocks.next().is_some() || positions.len() != n_positions {
        return Err("the blocks do not fit the objects".into());
    }
    let cached = src.clone().map(|src| super::routine_lists::CachedFacts {
        src,
        provided: provided.clone().into(),
    });
    let mut program = ir::Program {
        objects: Default::default(),
        apps,
        dotnet_types,
        sources,
        positions,
        uncompiled_subscribers,
        tdd_stubs,
        assumes,
    };
    let mut lowered = Lowered {
        timings: CompileTimings::default(),
        program: Default::default(),
        unsupported: Refusals::new(unsupported, cached.clone()),
        refusal_at,
        calls: Calls::new(calls, ndirect, cached),
        dependency,
        provided,
        driven: driven.map(StoredDriven::into_driven),
        drivers: Vec::new(),
        exported: None,
        al_diagnostics: al_diagnostics.iter().map(StoredDiagnostic::to_diagnostic).collect(),
        pos_ranges,
        checker_diags: checker_diags.iter().map(StoredDiagnostic::to_diagnostic).collect(),
        skipped,
        left_out,
        tdd_sites,
        tdd_needs,
        object_hashes,
        cached_objects: None,
        work: LowerWork {
            objects: n,
            objects_from_cache: supplied,
            ..Default::default()
        },
    };
    match &src {
        Some(src) => super::compile::lazy_cached_objects(&mut program, &mut lowered, objects, src),
        None => {
            program.objects = objects
                .into_iter()
                .map(Option::unwrap_or_default)
                .collect::<Vec<_>>()
                .into()
        }
    }
    lowered.program = Arc::new(program);
    Ok(lowered)
}

/// The canonical dump of a compile's `Lowered` (ADR-022 amendment 2026-11-15, "Equality requirement"): the whole program
/// with every body decoded, and every field of `Lowered` except `timings`, `work` and `cached_objects`. Destructured
/// without `..`.
pub fn canonical_dump(lowered: &Lowered) -> String {
    use std::fmt::Write as _;
    let Lowered {
        timings: _,
        program,
        unsupported,
        refusal_at,
        calls,
        dependency,
        provided,
        driven,
        drivers,
        exported,
        al_diagnostics,
        pos_ranges,
        checker_diags,
        skipped,
        left_out,
        tdd_sites,
        tdd_needs,
        object_hashes,
        cached_objects: _,
        work: _,
    } = lowered;
    let ir::Program {
        objects,
        apps,
        dotnet_types,
        sources,
        positions,
        uncompiled_subscribers,
        tdd_stubs,
        assumes,
    } = &**program;
    let json = |v: &dyn erased::Json| v.json();
    let mut out = String::new();
    for (oi, o) in objects.iter().enumerate() {
        let _ = writeln!(out, "object {oi} {}", json(o));
    }
    let _ = writeln!(out, "apps {}", json(apps));
    let _ = writeln!(out, "dotnet_types {}", json(dotnet_types));
    let _ = writeln!(out, "sources {}", json(sources));
    let _ = writeln!(out, "positions {}", json(positions));
    let _ = writeln!(out, "uncompiled_subscribers {}", json(uncompiled_subscribers));
    let _ = writeln!(out, "tdd_stubs {tdd_stubs:?}");
    let _ = writeln!(out, "assumes {assumes:?}");
    for o in 0..objects.len() {
        let _ = writeln!(
            out,
            "lowered {o} dependency {} provided {} pos {:?} skipped {:?} left_out {:?} hash {:?}",
            dependency[o],
            provided[o],
            pos_ranges.get(o),
            skipped.get(o),
            left_out.get(o),
            object_hashes.get(o)
        );
        let _ = writeln!(out, "  unsupported {:?}", unsupported.get(o));
        let _ = writeln!(out, "  refusal_at {:?}", refusal_at.get(o));
        let _ = writeln!(out, "  calls {:?}", calls.of(o));
        let routines = objects.get(o).map_or(0, |x| x.routines.len());
        let driven_calls: Vec<Vec<(usize, usize)>> = (0..routines)
            .map(|r| {
                let mut v = Vec::new();
                lowered.calls_into(o, r, &mut v);
                v
            })
            .collect();
        let _ = writeln!(out, "  calls_into {driven_calls:?}");
    }
    let _ = writeln!(out, "driven {:?}", driven.as_ref().map(StoredDriven::of));
    let _ = writeln!(out, "drivers {drivers:?}");
    let _ = writeln!(out, "exported {}", exported.is_some());
    for d in al_diagnostics {
        let _ = writeln!(out, "al_diagnostic {d:?}");
    }
    for d in checker_diags {
        let _ = writeln!(out, "checker_diag {d:?}");
    }
    let _ = writeln!(out, "tdd_sites {tdd_sites:?}");
    let _ = writeln!(out, "tdd_needs {tdd_needs:?}");
    out
}

/// JSON of a value for the dump, behind one object-safe trait so the dump can take any of the program's parts.
mod erased {
    pub(super) trait Json {
        fn json(&self) -> String;
    }

    impl<T: serde::Serialize> Json for T {
        fn json(&self) -> String {
            serde_json::to_string(self).unwrap_or_else(|e| format!("<not serializable: {e}>"))
        }
    }
}
