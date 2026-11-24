//! What the command line hands the workspace layer of the compile cache (ADR-022 amendment 2026-11-15) and what it says
//! of it: the run's options of the layer, the key of serve's kept compile (the layer's key without the texts), the
//! app.json facts both keys hold, and which report of the layer belongs to a run.

use crate::apps::AppInput;
use crate::args::TestArgs;
use crate::run::WarmCtx;
use std::collections::BTreeSet;

/// Everything the compile depends on but the selection and the text of the workspace files: the key of the workspace
/// layer of the compile cache without the texts (`al_testrun::workspace_cache::warm_key`, one owner of the field list),
/// over the dependency apps as `workspace_cache::dependency_digest` takes them. The resource files are not in it: they
/// are run-time data, which each request hands to the program as they are then (issue 5635). The options that change
/// what the compile holds (`run_inputs.rs`) are part of the key.
pub(crate) fn warm_key(
    a: &TestArgs,
    inputs: &[AppInput],
    dependencies: &al_testrun::deps::Dependencies,
    symbols: &BTreeSet<String>,
    target: &al_version::Target,
) -> String {
    use al_testrun::workspace_cache as wc;
    let json: Vec<String> = inputs.iter().map(app_json_text).collect();
    let apps: Vec<(&al_sema::workspace::AppSource, [u8; 32], &str)> = inputs
        .iter()
        .zip(&json)
        .map(|(i, j)| (&i.source, i.manifest_digest(), j.as_str()))
        .collect();
    let deps = wc::dependency_digest(&dependencies.apps, &dependencies.lazy, &dependencies.summary);
    let compile_options = &crate::run_inputs::RunInputs::of(a).compile_text();
    wc::warm_key(&apps, deps, symbols, &format!("{target:?}"), compile_options)
}

/// What an app's app.json gives the compile that its manifest does not (`WorkspaceCacheOptions::app_json`): its id,
/// name, publisher and version as written, `platform`, `application` and `preprocessorSymbols`. Each value is quoted, so
/// a space in one cannot read as the start of the next.
pub(crate) fn app_json_text(i: &AppInput) -> String {
    let j = &i.json;
    format!(
        "id {:?} name {:?} publisher {:?} version {:?} platform {:?} application {:?} symbols {:?}",
        j.id,
        j.name,
        j.publisher,
        j.version.to_string(),
        j.platform.as_ref().map(ToString::to_string),
        j.application.as_ref().map(ToString::to_string),
        i.symbols
    )
}

/// The workspace layer of the compile cache for a run (ADR-022 amendment 2026-11-15): on with the compile cache (the
/// layer itself turns off with `ALRUNNER_NO_WORKSPACE_CACHE`, `al_testrun::workspace_cache::switched_off`).
pub(crate) fn options(a: &TestArgs, inputs: &[AppInput]) -> Option<al_testrun::workspace_cache::WorkspaceCacheOptions> {
    Some(al_testrun::workspace_cache::WorkspaceCacheOptions {
        compile_options: crate::run_inputs::RunInputs::of(a).compile_text(),
        app_json: inputs.iter().map(app_json_text).collect(),
        write_now: false,
        verbose: a.verbose,
    })
}

/// What the workspace layer did for this run's own compile (`--timings` and serve's `workspaceCache`): a one-shot run
/// always compiled; a request of a server only when it compiled (`WarmUse::Compiled`). A request on the kept compile
/// compiled nothing and used no cache, though its phases still hold the report of the compile it ran on.
pub(crate) fn own_report<'a>(
    p: &'a al_testrun::runtime::Phases,
    warm: Option<&WarmCtx>,
) -> Option<&'a al_testrun::workspace_cache::WorkspaceReport> {
    let compiled = warm.is_none_or(|w| w.used == Some(al_testrun::runtime::WarmUse::Compiled));
    p.workspace_cache.as_ref().filter(|_| compiled)
}

/// First review LOW 10: [`app_json_text`] keeps each field apart, so two apps whose fields differ never give the same
/// text.
#[cfg(test)]
mod tests {
    fn app_with(dir: &std::path::Path, name: &str, publisher: &str) -> crate::apps::AppInput {
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(
            dir.join("app.json"),
            format!(
                r#"{{"id":"88888888-0000-0000-0000-000000000001","name":"{name}","publisher":"{publisher}","version":"1.0.0.0","dependencies":[]}}"#
            ),
        )
        .unwrap();
        std::fs::write(dir.join("src/c.al"), "codeunit 50800 C { }\n").unwrap();
        crate::apps::load(dir, None).unwrap()
    }

    /// A space moved from the name into the publisher is another text.
    #[test]
    fn a_space_moved_between_two_fields_is_another_text() {
        let d = tempfile::tempdir().unwrap();
        let a = app_with(&d.path().join("a"), "A B", "C");
        let b = app_with(&d.path().join("b"), "A", "B C");
        assert_ne!(super::app_json_text(&a), super::app_json_text(&b));
    }
}
