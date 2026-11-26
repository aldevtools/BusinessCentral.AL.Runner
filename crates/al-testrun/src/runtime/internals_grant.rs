//! The apps of a compile that the platform package's `InternalsVisibleTo` grants access to its internal members
//! (issue 7853), the symbols each app is compiled with, and the workspace loads (fresh, and from a stored compile of
//! the workspace layer) that apply both.

use super::Manifest;
use al_sema::workspace::AppSource;
use std::collections::BTreeSet;

/// The ids of the apps in `apps` (aligned with `manifests`) that a grant of the platform package names by app id and
/// publisher, both in any case. alc 18.0.39.10160 compares these two and not the app name (probe apps in
/// al-sema's alcodes/tests/internals_grant.rs).
pub(crate) fn granted_app_ids(apps: &[AppSource], manifests: &[Manifest]) -> Vec<String> {
    let grants: Vec<&al_deps::package::InternalsGrant> = manifests
        .iter()
        .flat_map(|m| &m.platform_internals_visible_to)
        .collect();
    apps.iter()
        .zip(manifests)
        .filter(|(a, m)| {
            grants
                .iter()
                .any(|g| g.id.eq_ignore_ascii_case(&a.id) && g.publisher.eq_ignore_ascii_case(&m.publisher))
        })
        .map(|(a, _)| a.id.to_ascii_lowercase())
        .collect()
}

/// Per app of the program, the symbols it is compiled with: a dependency app's are those of its own app.json, the
/// project's apps take the project's (`None`; hard rule 6: a project that defines CLEAN25 does not change Base
/// Application).
pub(crate) fn dependency_symbols(kinds: &[al_sema::AppKind], manifests: &[Manifest]) -> Vec<Option<BTreeSet<String>>> {
    kinds
        .iter()
        .enumerate()
        .map(|(i, k)| {
            (*k == al_sema::AppKind::Dependency).then(|| {
                manifests
                    .get(i)
                    .map(|m| m.symbols.iter().cloned().collect())
                    .unwrap_or_default()
            })
        })
        .collect()
}

/// [`al_sema::Workspace::load_apps_symbols`] with the platform package's grants set (see [`granted_app_ids`]).
pub(crate) fn load_workspace(
    apps: Vec<AppSource>,
    kinds: Vec<al_sema::AppKind>,
    symbols: &BTreeSet<String>,
    dep_symbols: &[Option<BTreeSet<String>>],
    lazy: Vec<Option<al_sema::LazyDep>>,
    manifests: &[Manifest],
) -> al_sema::Workspace {
    let granted = granted_app_ids(&apps, manifests);
    let mut ws = al_sema::Workspace::load_apps_symbols(apps, kinds, symbols, dep_symbols, lazy);
    ws.set_internals_granted(granted);
    ws
}

/// [`al_sema::Workspace::load_stored`] with the symbols and the grants [`load_workspace`] gives a fresh load: a read
/// compile's workspace is the fresh one's (ADR-022 amendment 2026-11-15, "Equality requirement").
pub(crate) fn load_stored(
    apps: Vec<AppSource>,
    kinds: Vec<al_sema::AppKind>,
    symbols: &BTreeSet<String>,
    lazy: Vec<Option<al_sema::LazyDep>>,
    stored: al_sema::StoredWorkspace,
    manifests: &[Manifest],
) -> Result<al_sema::Workspace, (Vec<AppSource>, String)> {
    let dep_symbols = dependency_symbols(&kinds, manifests);
    let granted = granted_app_ids(&apps, manifests);
    let mut ws = al_sema::Workspace::load_stored(apps, kinds, symbols, &dep_symbols, lazy, stored)?;
    ws.set_internals_granted(granted);
    Ok(ws)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(id: &str) -> AppSource {
        AppSource {
            id: id.into(),
            name: "Probe".into(),
            dependencies: vec![],
            files: vec![],
        }
    }

    fn manifest(publisher: &str, grants: &[(&str, &str)]) -> Manifest {
        Manifest {
            publisher: publisher.into(),
            platform_internals_visible_to: grants
                .iter()
                .map(|(id, publisher)| al_deps::package::InternalsGrant {
                    id: id.to_string(),
                    publisher: publisher.to_string(),
                })
                .collect(),
            ..Default::default()
        }
    }

    const GUIDED: &str = "c64d75f0-e9f1-4d0f-9949-cd453b9b1466";
    const SYSTEM: &str = "8874ed3a-0643-4247-9ced-7a7002f7135d";

    #[test]
    fn an_app_is_granted_by_id_and_publisher_in_any_case() {
        let system = manifest("Microsoft", &[(GUIDED, "Microsoft")]);
        let apps = [app(&GUIDED.to_ascii_uppercase()), app(SYSTEM)];
        assert_eq!(
            granted_app_ids(&apps, &[manifest("microsoft", &[]), system.clone()]),
            [GUIDED]
        );
        // Another publisher, or another id, is not granted (alc gives AL0161 there).
        assert!(granted_app_ids(&apps, &[manifest("Probe", &[]), system.clone()]).is_empty());
        let other = [app("1b1b1b1b-0000-0000-0000-000000007853"), app(SYSTEM)];
        assert!(granted_app_ids(&other, &[manifest("Microsoft", &[]), system]).is_empty());
    }
}
