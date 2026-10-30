//! Which package satisfies each dependency (ADR-013 decision 2, "Layer B").
//!
//! Mirrors AL Runner's `DependencyResolver`, with the owner's addition that the
//! project's own `.alpackages` is the first place looked up. Everything here is
//! a pure function over values: no file, no network.

use crate::version::Version;
use std::collections::HashMap;
use std::fmt;

/// Id of the platform app `System` (declares the system tables).
pub const SYSTEM_APP_ID: &str = "8874ed3a-0643-4247-9ced-7a7002f7135d";
/// Id of the `Application` app (an aggregate of the Microsoft apps; no objects of its own).
pub const APPLICATION_APP_ID: &str = "c1335042-3002-4257-bf8a-75c898ccb1b8";
/// The five Microsoft apps that ship one package per build: System, Application,
/// System Application, Business Foundation and Base Application.
pub const MICROSOFT_PLATFORM_APP_IDS: [&str; 5] = [
    SYSTEM_APP_ID,
    APPLICATION_APP_ID,
    "63ca2fa4-4f03-4f2b-a480-172fef340d3f",
    "f3552374-a1f2-4356-848e-196002525837",
    "437dbf0e-84ff-417a-965d-ed2bb9650972",
];

/// One declared dependency: an `app.json` entry or a manifest `<Dependency>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DepDecl {
    /// App id (a GUID). Empty when the declaration names the app only.
    pub id: String,
    pub name: String,
    pub publisher: String,
    /// The declared version is a minimum.
    pub min: Version,
    /// A missing optional dependency is skipped with a note, never an error.
    pub optional: bool,
}

/// An app package that could satisfy a dependency.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub id: String,
    pub name: String,
    pub publisher: String,
    pub version: Version,
    /// The package carries AL source with bodies. A package without it never
    /// shadows one that has it.
    pub has_source: bool,
    /// Search tier, lowest first: 0 is the project's `.alpackages`, then
    /// `--deps` directories, then the artifact cache.
    pub tier: usize,
    /// Built from source in this run (the project's own workspace apps).
    /// Outranks every package.
    pub from_workspace: bool,
    /// Where it came from, for messages.
    pub origin: String,
    /// Its declared dependencies.
    pub deps: Vec<DepDecl>,
    /// Its `Platform` and `Application` floors; each is an extra root.
    pub platform: Option<Version>,
    pub application: Option<Version>,
}

/// The project whose dependencies are resolved.
#[derive(Debug, Clone, Default)]
pub struct Project {
    pub deps: Vec<DepDecl>,
    pub platform: Option<Version>,
    pub application: Option<Version>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissingPolicy {
    /// A dependency with no satisfying package is an error.
    Error,
    /// It is recorded in `Plan::skipped` and the run goes on without it.
    Skip,
}

#[derive(Debug, Clone)]
pub struct Options {
    pub missing: MissingPolicy,
    /// The places that were searched, named in the "not found" error.
    pub searched: Vec<String>,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            missing: MissingPolicy::Error,
            searched: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectError {
    /// No package that carries the app's AL source was found. `symbol_only` names the packages of
    /// the app that were found without source (they do not count as found, owner decision 2026-10-15).
    Missing {
        dep: DepDecl,
        searched: Box<[String]>,
        symbol_only: Box<Vec<String>>,
    },
    /// Packages exist for the id, but all are below the declared minimum.
    BelowMinimum { dep: DepDecl, found: Vec<Version> },
    /// The dependencies lead back to an app already being resolved.
    Cycle(Vec<String>),
    /// Two dependents need versions no single package satisfies.
    Conflict(Box<Conflict>),
}

/// Two dependents need versions of one app that no single package satisfies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conflict {
    /// The dependency that came second and is not satisfied.
    pub dep: DepDecl,
    /// The version chosen for the first dependent, and where that package came from.
    pub chosen: Version,
    pub chosen_origin: String,
    /// The app that declares `dep` ("the project" for a root dependency) and where it came from.
    pub needed_by: String,
    pub needed_by_origin: String,
    /// The packages in the search that have a version `dep` accepts, as "version from origin".
    pub satisfying: Vec<String>,
}

impl fmt::Display for SelectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SelectError::Missing {
                dep,
                searched,
                symbol_only,
            } => {
                write!(
                    f,
                    "dependency `{}` by {} ({}) version {} or higher was not found{}; searched: {}; put its .app with AL source (or its source archive) in the project's .alpackages or pass --dep DIR",
                    dep.name,
                    dep.publisher,
                    dep.id,
                    dep.min,
                    if symbol_only.is_empty() {
                        String::new()
                    } else {
                        format!(
                            " with AL source (found only packages without AL source, which cannot serve it: {})",
                            symbol_only.join(", ")
                        )
                    },
                    if searched.is_empty() {
                        "(nothing)".to_string()
                    } else {
                        searched.join(", ")
                    }
                )
            }
            SelectError::BelowMinimum { dep, found } => write!(
                f,
                "dependency `{}` needs version {} or higher; only found {}",
                dep.name,
                dep.min,
                found.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(", ")
            ),
            SelectError::Cycle(path) => write!(f, "dependency cycle: {}", path.join(" -> ")),
            SelectError::Conflict(c) => {
                let Conflict {
                    dep,
                    chosen,
                    chosen_origin,
                    needed_by,
                    needed_by_origin,
                    satisfying,
                } = &**c;
                write!(
                    f,
                    "dependency `{}` needs version {} or higher for `{needed_by}` (from {needed_by_origin}), but version {chosen} from {chosen_origin} was already chosen for another app; ",
                    dep.name, dep.min
                )?;
                if satisfying.is_empty() {
                    write!(
                        f,
                        "no package found has version {} or higher; to fix it, remove the older package or replace it with version {} or higher of `{}`",
                        dep.min, dep.min, dep.name
                    )
                } else {
                    write!(
                        f,
                        "found with a version that fits: {}; to fix it, remove the older package {chosen_origin} or replace it with version {} or higher",
                        satisfying.join(", "),
                        dep.min
                    )
                }
            }
        }
    }
}

impl std::error::Error for SelectError {}

#[derive(Debug, Clone, Default)]
pub struct Plan {
    /// Indices into the candidate pool, dependencies before dependents.
    pub order: Vec<usize>,
    /// Lines to print: below-minimum Microsoft apps, skipped optional dependencies.
    pub notes: Vec<String>,
    /// Dependencies that no package satisfied (only with `MissingPolicy::Skip`).
    pub skipped: Vec<SelectError>,
    /// Optional dependencies nobody satisfied (the platform's `System` and the `Application` floor of an app are
    /// optional roots). They are not an error, but a run that can get them from Microsoft's build should: the platform
    /// tables (`Integer`, `Company`, `AllObj`, ...) are `System`'s.
    pub optional_missing: Vec<DepDecl>,
}

/// Chooses a package for every dependency of `project`, transitively, and
/// returns them in dependency order.
pub fn resolve(project: &Project, pool: &[Candidate], opts: &Options) -> Result<Plan, SelectError> {
    let mut r = Resolver {
        pool,
        opts,
        chosen: HashMap::new(),
        state: vec![State::Unvisited; pool.len()],
        stack: Vec::new(),
        plan: Plan::default(),
    };
    r.visit_deps(&project.deps, project.platform, project.application)?;
    Ok(r.plan)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    Unvisited,
    Visiting,
    Done,
}

struct Resolver<'a> {
    pool: &'a [Candidate],
    opts: &'a Options,
    /// Lower-case app id to the pool index chosen for it.
    chosen: HashMap<String, usize>,
    state: Vec<State>,
    /// The packages being resolved (pool indices), for the cycle path and the dependent of a conflict.
    stack: Vec<usize>,
    plan: Plan,
}

fn same(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

impl Resolver<'_> {
    /// The platform floor and the application floor are implicit roots, resolved
    /// first so that the platform precedes everything that needs it. They are
    /// optional: a missing one is a note, not an error.
    fn visit_deps(
        &mut self,
        deps: &[DepDecl],
        platform: Option<Version>,
        application: Option<Version>,
    ) -> Result<(), SelectError> {
        let implicit = [
            (SYSTEM_APP_ID, "System", platform),
            (APPLICATION_APP_ID, "Application", application),
        ];
        for (id, name, floor) in implicit {
            if let Some(min) = floor {
                self.satisfy(&DepDecl {
                    id: id.into(),
                    name: name.into(),
                    publisher: "Microsoft".into(),
                    min,
                    optional: true,
                })?;
            }
        }
        for d in deps {
            self.satisfy(d)?;
        }
        Ok(())
    }

    fn satisfy(&mut self, dep: &DepDecl) -> Result<(), SelectError> {
        // A lookup by id is authoritative; name and publisher only when the id is empty or unknown.
        let by_id: Vec<usize> = (0..self.pool.len())
            .filter(|&i| !dep.id.is_empty() && same(&self.pool[i].id, &dep.id))
            .collect();
        let found: Vec<usize> = if by_id.is_empty() {
            (0..self.pool.len())
                .filter(|&i| {
                    !dep.name.is_empty()
                        && same(&self.pool[i].name, &dep.name)
                        && same(&self.pool[i].publisher, &dep.publisher)
                })
                .collect()
        } else {
            by_id
        };
        // A package that cannot serve the source is not found (owner decision 2026-10-15); the
        // search goes on. The `Application` aggregate holds no objects by design and counts.
        let serves = |i: usize| {
            let c = &self.pool[i];
            c.has_source || same(&c.id, APPLICATION_APP_ID)
        };
        let matching: Vec<usize> = found.iter().copied().filter(|&i| serves(i)).collect();
        if matching.is_empty() {
            return self.unsatisfied(
                dep,
                SelectError::Missing {
                    dep: dep.clone(),
                    searched: self.opts.searched.clone().into_boxed_slice(),
                    symbol_only: Box::new(found.iter().map(|&i| self.pool[i].origin.clone()).collect()),
                },
            );
        }
        // An app already chosen for another dependent is reused if it satisfies this one too.
        let app_id = self.pool[matching[0]].id.to_ascii_lowercase();
        if let Some(&c) = self.chosen.get(&app_id) {
            if self.pool[c].version >= dep.min || self.microsoft_platform_app(c) {
                // Done: nothing to do. Still being resolved: a cycle.
                return self.visit(c);
            }
            let (needed_by, needed_by_origin) = match self.stack.last() {
                Some(&p) => (self.pool[p].name.clone(), self.pool[p].origin.clone()),
                None => ("the project".to_string(), "its app.json".to_string()),
            };
            return Err(SelectError::Conflict(Box::new(Conflict {
                dep: dep.clone(),
                chosen: self.pool[c].version,
                chosen_origin: self.pool[c].origin.clone(),
                needed_by,
                needed_by_origin,
                satisfying: matching
                    .iter()
                    .filter(|&&i| self.pool[i].version >= dep.min)
                    .map(|&i| format!("{} from {}", self.pool[i].version, self.pool[i].origin))
                    .collect(),
            })));
        }
        let mut satisfying: Vec<usize> = matching
            .iter()
            .copied()
            .filter(|&i| self.pool[i].version >= dep.min)
            .collect();
        let mut below_note = false;
        if satisfying.is_empty() {
            if self.microsoft_platform_app(matching[0]) {
                // These ship one package per build; the minimum is advisory.
                satisfying = matching.clone();
                below_note = true;
            } else {
                let mut found: Vec<Version> = matching.iter().map(|&i| self.pool[i].version).collect();
                found.sort();
                return self.unsatisfied(
                    dep,
                    SelectError::BelowMinimum {
                        dep: dep.clone(),
                        found,
                    },
                );
            }
        }
        // Source-built in this run, then a package with source, then the nearest place
        // searched (the project's .alpackages first), then the highest version.
        let best = satisfying
            .into_iter()
            .min_by_key(|&i| {
                let c = &self.pool[i];
                (
                    !c.from_workspace,
                    !c.has_source,
                    c.tier,
                    std::cmp::Reverse(c.version),
                    i,
                )
            })
            .expect("non-empty");
        if below_note {
            let c = &self.pool[best];
            self.plan.notes.push(format!(
                "`{}` {} is below the declared minimum {}; Microsoft platform apps ship one package per build, using it",
                c.name, c.version, dep.min
            ));
        }
        self.chosen.insert(app_id, best);
        self.visit(best)
    }

    fn microsoft_platform_app(&self, i: usize) -> bool {
        MICROSOFT_PLATFORM_APP_IDS.iter().any(|id| same(id, &self.pool[i].id))
    }

    /// A dependency nobody satisfies: skipped when optional or when the policy
    /// says so, an error otherwise.
    fn unsatisfied(&mut self, dep: &DepDecl, err: SelectError) -> Result<(), SelectError> {
        if dep.optional {
            self.plan.notes.push(format!(
                "optional dependency `{}` {} was not found; skipped",
                dep.name, dep.min
            ));
            self.plan.optional_missing.push(dep.clone());
            return Ok(());
        }
        match (self.opts.missing, &err) {
            (MissingPolicy::Skip, SelectError::Missing { .. }) => {
                self.plan.skipped.push(err);
                Ok(())
            }
            _ => Err(err),
        }
    }

    fn visit(&mut self, i: usize) -> Result<(), SelectError> {
        match self.state[i] {
            State::Done => return Ok(()),
            State::Visiting => {
                let from = self.stack.iter().position(|&n| n == i).unwrap_or(0);
                let mut path: Vec<String> = self.stack[from..].iter().map(|&n| self.pool[n].name.clone()).collect();
                path.push(self.pool[i].name.clone());
                return Err(SelectError::Cycle(path));
            }
            State::Unvisited => {}
        }
        self.state[i] = State::Visiting;
        self.stack.push(i);
        let c = &self.pool[i];
        self.visit_deps(&c.deps, c.platform, c.application)?;
        self.stack.pop();
        self.state[i] = State::Done;
        self.plan.order.push(i);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &str) -> Version {
        s.parse().unwrap()
    }

    fn dep(id: &str, min: &str) -> DepDecl {
        DepDecl {
            id: id.into(),
            name: format!("App {id}"),
            publisher: "Pub".into(),
            min: v(min),
            optional: false,
        }
    }

    fn cand(id: &str, version: &str, deps: Vec<DepDecl>) -> Candidate {
        Candidate {
            id: id.into(),
            name: format!("App {id}"),
            publisher: "Pub".into(),
            version: v(version),
            has_source: true,
            tier: 0,
            from_workspace: false,
            origin: format!("{id}_{version}.app"),
            deps,
            platform: None,
            application: None,
        }
    }

    fn project(deps: Vec<DepDecl>) -> Project {
        Project {
            deps,
            ..Default::default()
        }
    }

    fn names(plan: &Plan, pool: &[Candidate]) -> Vec<String> {
        plan.order.iter().map(|&i| pool[i].origin.clone()).collect()
    }

    #[test]
    fn the_highest_version_at_or_above_the_minimum_wins() {
        let pool = vec![
            cand("a", "1.0.0.0", vec![]),
            cand("a", "3.0.0.0", vec![]),
            cand("a", "2.0.0.0", vec![]),
        ];
        let plan = resolve(&project(vec![dep("a", "1.5")]), &pool, &Options::default()).unwrap();
        assert_eq!(names(&plan, &pool), ["a_3.0.0.0.app"]);
    }

    #[test]
    fn a_candidate_below_the_declared_minimum_is_dropped() {
        let pool = vec![cand("a", "1.0.0.0", vec![]), cand("a", "2.0.0.0", vec![])];
        let plan = resolve(&project(vec![dep("a", "2.0")]), &pool, &Options::default()).unwrap();
        assert_eq!(names(&plan, &pool), ["a_2.0.0.0.app"]);
    }

    #[test]
    fn an_id_match_is_authoritative_and_does_not_fall_through_to_the_name() {
        // Same name and publisher, other id: must not stand in for id `a`.
        let mut other = cand("b", "9.0.0.0", vec![]);
        other.name = "App a".into();
        let pool = vec![cand("a", "1.0.0.0", vec![]), other];
        let err = resolve(&project(vec![dep("a", "2.0")]), &pool, &Options::default()).unwrap_err();
        assert!(matches!(err, SelectError::BelowMinimum { .. }), "{err:?}");
    }

    #[test]
    fn name_and_publisher_are_used_only_when_the_id_is_empty_or_unknown() {
        let pool = vec![cand("a", "2.0.0.0", vec![])];
        let mut by_name = dep("", "1.0");
        by_name.name = "app A".into(); // case-insensitive
        by_name.publisher = "pub".into();
        let plan = resolve(&project(vec![by_name]), &pool, &Options::default()).unwrap();
        assert_eq!(names(&plan, &pool), ["a_2.0.0.0.app"]);
        let mut unknown_id = dep("zzz", "1.0");
        unknown_id.name = "App a".into();
        let plan = resolve(&project(vec![unknown_id]), &pool, &Options::default()).unwrap();
        assert_eq!(names(&plan, &pool), ["a_2.0.0.0.app"]);
    }

    #[test]
    fn candidates_all_below_the_minimum_give_an_error_listing_the_versions() {
        let pool = vec![cand("a", "1.0.0.0", vec![]), cand("a", "1.5.0.0", vec![])];
        let err = resolve(&project(vec![dep("a", "2.0")]), &pool, &Options::default()).unwrap_err();
        let SelectError::BelowMinimum { found, .. } = &err else {
            panic!("{err:?}")
        };
        assert_eq!(found, &[v("1.0"), v("1.5")]);
        let text = err.to_string();
        assert!(
            text.contains("2.0.0.0") && text.contains("1.0.0.0") && text.contains("1.5.0.0"),
            "{text}"
        );
    }

    #[test]
    fn a_missing_app_gives_an_error_naming_the_places_searched() {
        let opts = Options {
            searched: vec![".alpackages".into(), "/deps".into()],
            ..Default::default()
        };
        let err = resolve(&project(vec![dep("a", "1.0")]), &[], &opts).unwrap_err();
        assert!(matches!(err, SelectError::Missing { .. }));
        let text = err.to_string();
        assert!(
            text.contains("App a") && text.contains(".alpackages") && text.contains("/deps"),
            "{text}"
        );
    }

    #[test]
    fn a_cycle_is_an_error_naming_the_path() {
        let pool = vec![
            cand("a", "1.0.0.0", vec![dep("b", "1.0")]),
            cand("b", "1.0.0.0", vec![dep("c", "1.0")]),
            cand("c", "1.0.0.0", vec![dep("a", "1.0")]),
        ];
        let err = resolve(&project(vec![dep("a", "1.0")]), &pool, &Options::default()).unwrap_err();
        let SelectError::Cycle(path) = &err else {
            panic!("{err:?}")
        };
        assert_eq!(path, &["App a", "App b", "App c", "App a"]);
    }

    #[test]
    fn dependencies_come_before_their_dependents() {
        let pool = vec![
            cand("top", "1.0.0.0", vec![dep("mid", "1.0"), dep("base", "1.0")]),
            cand("mid", "1.0.0.0", vec![dep("base", "1.0")]),
            cand("base", "1.0.0.0", vec![]),
        ];
        let plan = resolve(&project(vec![dep("top", "1.0")]), &pool, &Options::default()).unwrap();
        assert_eq!(
            names(&plan, &pool),
            ["base_1.0.0.0.app", "mid_1.0.0.0.app", "top_1.0.0.0.app"]
        );
    }

    #[test]
    fn a_shared_dependency_appears_once() {
        let pool = vec![
            cand("x", "1.0.0.0", vec![dep("base", "1.0")]),
            cand("y", "1.0.0.0", vec![dep("base", "1.0")]),
            cand("base", "1.0.0.0", vec![]),
        ];
        let plan = resolve(
            &project(vec![dep("x", "1.0"), dep("y", "1.0")]),
            &pool,
            &Options::default(),
        )
        .unwrap();
        assert_eq!(plan.order.len(), 3);
    }

    #[test]
    fn a_microsoft_platform_app_below_its_minimum_is_used_with_a_note() {
        let base = MICROSOFT_PLATFORM_APP_IDS[4];
        let pool = vec![cand(base, "27.0.0.0", vec![])];
        let plan = resolve(&project(vec![dep(base, "27.5")]), &pool, &Options::default()).unwrap();
        assert_eq!(plan.order, [0]);
        assert!(
            plan.notes.iter().any(|n| n.contains("below") && n.contains("27.5.0.0")),
            "{:?}",
            plan.notes
        );
        // The same shortfall in an ordinary app is an error.
        let pool = vec![cand("zzz", "27.0.0.0", vec![])];
        assert!(resolve(&project(vec![dep("zzz", "27.5")]), &pool, &Options::default()).is_err());
    }

    #[test]
    fn a_package_without_source_never_shadows_one_that_has_it() {
        let mut symbols_only = cand("a", "9.0.0.0", vec![]);
        symbols_only.has_source = false;
        let pool = vec![symbols_only, cand("a", "1.0.0.0", vec![])];
        let plan = resolve(&project(vec![dep("a", "1.0")]), &pool, &Options::default()).unwrap();
        assert_eq!(names(&plan, &pool), ["a_1.0.0.0.app"]);
    }

    /// Owner decision 2026-10-15: a package that cannot serve the source is not a package that is
    /// found. The search goes on; with nothing else the dependency is missing, and says what was seen.
    #[test]
    fn a_package_without_source_does_not_satisfy_a_dependency() {
        let mut symbols_only = cand("a", "9.0.0.0", vec![dep("b", "1.0")]);
        symbols_only.has_source = false;
        let pool = vec![symbols_only, cand("b", "1.0.0.0", vec![])];
        let opts = Options {
            missing: MissingPolicy::Skip,
            searched: vec!["alpackages".into()],
        };
        let plan = resolve(&project(vec![dep("a", "1.0")]), &pool, &opts).unwrap();
        assert!(plan.order.is_empty(), "neither it nor what only it needs is chosen");
        let SelectError::Missing {
            dep: missing,
            symbol_only,
            ..
        } = &plan.skipped[0]
        else {
            panic!("{:?}", plan.skipped)
        };
        assert_eq!(missing.id, "a");
        assert_eq!(&symbol_only[..], ["a_9.0.0.0.app"]);
        let text = plan.skipped[0].to_string();
        assert!(
            text.contains("without AL source") && text.contains("a_9.0.0.0.app"),
            "{text}"
        );
        // As an error (the default policy) it is the same kind of error.
        assert!(matches!(
            resolve(&project(vec![dep("a", "1.0")]), &pool, &Options::default()),
            Err(SelectError::Missing { .. })
        ));
    }

    #[test]
    fn a_later_place_with_source_satisfies_when_an_earlier_one_has_only_symbols() {
        let mut symbols_only = cand("a", "9.0.0.0", vec![]);
        symbols_only.has_source = false;
        let mut elsewhere = cand("a", "2.0.0.0", vec![]);
        elsewhere.tier = 3;
        let pool = vec![symbols_only, elsewhere];
        let plan = resolve(&project(vec![dep("a", "1.0")]), &pool, &Options::default()).unwrap();
        assert_eq!(names(&plan, &pool), ["a_2.0.0.0.app"]);
    }

    #[test]
    fn the_application_aggregate_has_no_source_by_design_and_is_still_found() {
        let mut aggregate = cand(APPLICATION_APP_ID, "28.0.0.0", vec![dep("b", "1.0")]);
        aggregate.has_source = false;
        let pool = vec![aggregate, cand("b", "1.0.0.0", vec![])];
        let plan = resolve(
            &project(vec![dep(APPLICATION_APP_ID, "28.0")]),
            &pool,
            &Options::default(),
        )
        .unwrap();
        assert_eq!(plan.order.len(), 2);
    }

    #[test]
    fn the_projects_own_alpackages_is_looked_up_before_other_places() {
        let mut elsewhere = cand("a", "3.0.0.0", vec![]);
        elsewhere.tier = 1;
        let pool = vec![elsewhere, cand("a", "2.0.0.0", vec![])]; // tier 0 holds the lower version
        let plan = resolve(&project(vec![dep("a", "1.0")]), &pool, &Options::default()).unwrap();
        assert_eq!(names(&plan, &pool), ["a_2.0.0.0.app"]);
    }

    #[test]
    fn a_later_place_is_used_when_the_earlier_one_does_not_satisfy() {
        let mut elsewhere = cand("a", "3.0.0.0", vec![]);
        elsewhere.tier = 1;
        let pool = vec![cand("a", "1.0.0.0", vec![]), elsewhere];
        let plan = resolve(&project(vec![dep("a", "2.0")]), &pool, &Options::default()).unwrap();
        assert_eq!(names(&plan, &pool), ["a_3.0.0.0.app"]);
    }

    #[test]
    fn an_app_built_from_source_in_this_run_outranks_packages_and_its_dependencies_are_followed() {
        let mut ws = cand("a", "1.0.0.0", vec![dep("base", "1.0")]);
        ws.from_workspace = true;
        ws.origin = "workspace a".into();
        let pool = vec![cand("a", "5.0.0.0", vec![]), ws, cand("base", "1.0.0.0", vec![])];
        let plan = resolve(&project(vec![dep("a", "1.0")]), &pool, &Options::default()).unwrap();
        assert_eq!(names(&plan, &pool), ["base_1.0.0.0.app", "workspace a"]);
    }

    #[test]
    fn a_missing_optional_dependency_is_skipped_with_a_note() {
        let mut d = dep("a", "1.0");
        d.optional = true;
        let plan = resolve(&project(vec![d]), &[], &Options::default()).unwrap();
        assert!(plan.order.is_empty());
        assert!(plan.notes.iter().any(|n| n.contains("App a")), "{:?}", plan.notes);
    }

    #[test]
    fn skip_policy_records_what_is_missing_and_goes_on() {
        let pool = vec![cand("b", "1.0.0.0", vec![])];
        let opts = Options {
            missing: MissingPolicy::Skip,
            ..Default::default()
        };
        let plan = resolve(&project(vec![dep("a", "1.0"), dep("b", "1.0")]), &pool, &opts).unwrap();
        assert_eq!(plan.order, [0]);
        assert_eq!(plan.skipped.len(), 1);
        assert!(matches!(&plan.skipped[0], SelectError::Missing { dep, .. } if dep.id == "a"));
    }

    #[test]
    fn the_platform_and_application_floors_are_implicit_roots() {
        let pool = vec![
            cand(SYSTEM_APP_ID, "27.0.46760.0", vec![]),
            cand(APPLICATION_APP_ID, "27.5.0.0", vec![]),
        ];
        let p = Project {
            platform: Some(v("27.0")),
            application: Some(v("27.0")),
            ..Default::default()
        };
        let plan = resolve(&p, &pool, &Options::default()).unwrap();
        assert_eq!(plan.order, [0, 1]);
        // A chosen package's own floors add roots too.
        let mut a = cand("a", "1.0.0.0", vec![]);
        a.platform = Some(v("27.0"));
        let pool2 = vec![pool[0].clone(), a];
        let plan = resolve(&project(vec![dep("a", "1.0")]), &pool2, &Options::default()).unwrap();
        assert_eq!(plan.order, [0, 1]);
    }

    #[test]
    fn implicit_roots_that_no_package_supplies_are_skipped_not_errors() {
        let p = Project {
            platform: Some(v("27.0")),
            ..Default::default()
        };
        let plan = resolve(&p, &[], &Options::default()).unwrap();
        assert!(plan.order.is_empty());
        assert!(!plan.notes.is_empty());
    }

    #[test]
    fn two_dependents_needing_incompatible_versions_conflict() {
        let mut high = cand("base", "2.0.0.0", vec![]);
        high.tier = 1;
        let pool = vec![
            cand("x", "1.0.0.0", vec![dep("base", "1.0")]),
            cand("y", "1.0.0.0", vec![dep("base", "2.0")]),
            cand("base", "1.0.0.0", vec![]), // tier 0 wins for x
            high,
        ];
        let err = resolve(
            &project(vec![dep("x", "1.0"), dep("y", "1.0")]),
            &pool,
            &Options::default(),
        )
        .unwrap_err();
        assert!(matches!(err, SelectError::Conflict { .. }), "{err:?}");
        // The message names where each version came from and how to fix it.
        let text = err.to_string();
        for part in [
            "`App base`",
            "2.0.0.0",
            "base_1.0.0.0.app",
            "`App y`",
            "y_1.0.0.0.app",
            "base_2.0.0.0.app",
        ] {
            assert!(text.contains(part), "{part} missing from: {text}");
        }
        assert!(text.contains("to fix it"), "{text}");
    }
}
