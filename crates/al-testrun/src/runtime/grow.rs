//! A run that grows the compile of an earlier run of the same process by apps (ADR-063): the corpus runs a BC version in
//! passes, and a pass after the main one adds its test apps and dependency apps to the main pass's compile instead of
//! compiling everything again. Each pass keeps its own engine, store, install baseline and sessions; only the compile is
//! shared. `AL_FRESH_PASS_COMPILE=1` turns it off: every pass compiles from source.
use super::*;

/// The compile of a run, kept for a later run of the same process that adds apps to it
/// ([`CorpusRuntime::try_run_suites_growable`], [`CorpusRuntime::try_run_suites_grown`]).
pub struct KeptCompile {
    pub(crate) ws: al_sema::Workspace,
    pub(crate) growable: al_sema::lower::Growable,
    /// Per app of the compile, in its order: the preprocessor symbols of a dependency app (its own app.json's).
    pub(crate) app_symbols: Vec<Option<BTreeSet<String>>>,
    /// The project's symbols and the target the compile was made for.
    pub(crate) symbols: BTreeSet<String>,
    pub(crate) target: Option<al_version::Target>,
}

/// What a compile keeps for a later run (`Compiled::grow_kept`).
pub(crate) struct GrowKept {
    pub(crate) growable: al_sema::lower::Growable,
    /// Per app of the compile: the preprocessor symbols of a dependency app.
    pub(crate) app_symbols: Vec<Option<BTreeSet<String>>>,
    /// How many apps, from the first, the later run also has.
    pub(crate) shared: usize,
}

/// How a compile takes part in growing (`compile_suites_once`).
pub(crate) enum Grow {
    /// A compile of its own, nothing kept.
    Off,
    /// Keep the compile for a later run, when it can be grown (`Compiled::grow_kept`). The apps (indexes in the
    /// caller's order) the later run also has come first in the program; the kept compile is cut back to them.
    Keep(Vec<usize>),
    /// Grow this kept compile by the run's other apps.
    From(Box<KeptCompile>),
}

/// Whether `AL_FRESH_PASS_COMPILE` asks every pass to compile from source.
pub fn fresh_pass_compile() -> bool {
    std::env::var_os("AL_FRESH_PASS_COMPILE").is_some_and(|v| !v.is_empty() && v != "0")
}

impl KeptCompile {
    /// The order of `apps` in the grown program: the kept compile's apps first, in its order, then the others in the
    /// caller's order, as (program app -> caller app). An error says why the run cannot grow the kept compile: one of its
    /// apps is not among `apps` or not the same (sources, kind, dependencies, symbols), or the symbols or target differ.
    pub(crate) fn order_for(
        &self,
        apps: &[al_sema::workspace::AppSource],
        kinds: &[al_sema::AppKind],
        manifests: &[Manifest],
        symbols: &BTreeSet<String>,
        target: Option<&al_version::Target>,
    ) -> Result<Vec<usize>, String> {
        if *symbols != self.symbols {
            return Err("the project's preprocessor symbols differ".into());
        }
        if target != self.target.as_ref() {
            return Err("the target differs".into());
        }
        let symbols_of = |i: usize| -> Option<BTreeSet<String>> {
            (kinds[i] == al_sema::AppKind::Dependency).then(|| {
                manifests
                    .get(i)
                    .map(|m| m.symbols.iter().cloned().collect())
                    .unwrap_or_default()
            })
        };
        let mut taken = vec![false; apps.len()];
        let mut order = Vec::with_capacity(apps.len());
        for (j, kept) in self.ws.apps.iter().enumerate() {
            let found = (0..apps.len()).find(|&i| {
                !taken[i]
                    && apps[i].id.eq_ignore_ascii_case(&kept.id)
                    && apps[i].name == kept.name
                    && kinds[i] == self.ws.kinds[j]
                    && apps[i].dependencies == kept.dependencies
                    && symbols_of(i) == self.app_symbols[j]
                    && apps[i].files == kept.files
            });
            let Some(i) = found else {
                return Err(format!(
                    "app `{}` of the kept compile is not in this run, or not the same",
                    kept.name
                ));
            };
            taken[i] = true;
            order.push(i);
        }
        order.extend((0..apps.len()).filter(|&i| !taken[i]));
        Ok(order)
    }
}
