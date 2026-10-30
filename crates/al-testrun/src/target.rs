//! Reads the version fields of an app.json (ADR-015 step 4).

use al_deps::build::BuildQuery;
use al_version::{AppVersions, Chosen};
use std::path::Path;

/// Layer A's question for a run (ADR-013 decision 2): which Microsoft build supplies the apps.
///
/// `requested` is `--bc-version` as typed; an oracle leg passes its own. With no request the
/// build comes from the major of the target that `al_version::choose_target` chose from the app's
/// floor (step 2), never below the floor itself (step 3). The run's `Target` and the build
/// therefore always agree on the major.
pub fn build_query(requested: Option<&str>, chosen: &Chosen, app: &AppVersions) -> BuildQuery {
    let floor = app.floor().map(|f| (u32::from(f.major), u32::from(f.minor)));
    BuildQuery::new(requested, chosen.target.bc.major, floor)
}

/// `runtime`, `platform` and `application` of `dir/app.json`. A field that is absent stays `None`.
pub fn read_app_versions(dir: &Path) -> Result<AppVersions, String> {
    let path = dir.join("app.json");
    let text = al_deps::load::read_text(&path)?;
    app_versions_from_json(&text).map_err(|e| format!("{}: {e}", path.display()))
}

pub fn app_versions_from_json(text: &str) -> Result<AppVersions, String> {
    let json: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let field = |k: &str| json.get(k).and_then(serde_json::Value::as_str);
    // AL1039: alc reads `runtime` as `X.Y` only. A one-part string, more than two parts, letters and a JSON number are
    // refused before anything compiles (al-runner issue 5506, alc measured by the owner; text from that measurement,
    // no corpus case pins it yet: docs/probes-needed.md). An empty string stays "not declared" (not measured).
    if let Some(v) = json.get("runtime").filter(|v| !v.is_null()) {
        let shown = v.as_str().map_or_else(|| v.to_string(), str::to_string);
        let x_y = shown.split_once('.').is_some_and(|(x, y)| {
            [x, y]
                .iter()
                .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
        });
        if !shown.is_empty() && !x_y {
            return Err(format!(
                "app.json: error AL1039: The version number '{shown}' for the property 'runtime' does not match the expected format: X.Y where X and Y represent positive integers."
            ));
        }
    }
    AppVersions::parse(field("runtime"), field("platform"), field("application")).map_err(|e| format!("app.json: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use al_version::{BcVersion, Runtime};

    #[test]
    fn reads_runtime_platform_and_application() {
        let v = app_versions_from_json(r#"{"id":"x","runtime":"16.0","platform":"27.0.0.0","application":"27.1.0.0"}"#)
            .unwrap();
        assert_eq!(v.runtime, Some(Runtime { major: 16, minor: 0 }));
        assert_eq!(v.platform, Some(BcVersion::new(27, 0)));
        assert_eq!(v.floor(), Some(&BcVersion::new(27, 1)));
    }

    #[test]
    fn absent_fields_stay_absent() {
        assert_eq!(app_versions_from_json(r#"{"id":"x"}"#).unwrap(), AppVersions::default());
    }

    #[test]
    fn a_bad_runtime_is_an_error() {
        assert!(app_versions_from_json(r#"{"runtime":"soon"}"#).is_err());
        assert!(app_versions_from_json("{").is_err());
    }

    // al-runner issue 5506 (alc measured by the owner): a `runtime` that is not `X.Y` is AL1039, also a one-part string
    // and a JSON number, which the runner used to read as 14.0 or as no runtime at all.
    #[test]
    fn a_runtime_not_in_x_y_form_is_al1039() {
        for (json, shown) in [
            (r#"{"runtime":"14"}"#, "14"),
            (r#"{"runtime":"abc"}"#, "abc"),
            (r#"{"runtime":14}"#, "14"),
            (r#"{"runtime":"14.0.0"}"#, "14.0.0"),
            (r#"{"runtime":"15.0.0.0"}"#, "15.0.0.0"),
        ] {
            let e = app_versions_from_json(json).unwrap_err();
            assert!(
                e.contains(&format!(
                    "AL1039: The version number '{shown}' for the property 'runtime' does not match the expected format: X.Y where X and Y represent positive integers."
                )),
                "{json}: {e}"
            );
        }
        assert!(app_versions_from_json(r#"{"runtime":"14.0"}"#).is_ok());
    }

    fn chosen(requested: Option<&str>, json: &str) -> (Chosen, AppVersions) {
        let app = app_versions_from_json(json).unwrap();
        let req = requested.map(|r| BcVersion::parse(r).unwrap());
        let c = al_version::choose_target(req.as_ref(), &app, &BcVersion::new(27, 0)).unwrap();
        (c, app)
    }

    #[test]
    fn without_a_request_the_build_follows_the_floors_major_and_respects_the_floor() {
        let (c, app) = chosen(None, r#"{"platform":"28.0.0.0","application":"28.2.0.0"}"#);
        let q = build_query(None, &c, &app);
        assert_eq!((q.prefix.as_str(), q.explicit, q.floor), ("28", false, Some((28, 2))));
    }

    #[test]
    fn an_old_floor_runs_on_bc_27_so_the_build_is_a_27() {
        // ADR-015 3.2 step 3: a floor of BC 24 still runs on 27.
        let (c, app) = chosen(None, r#"{"application":"24.0.0.0"}"#);
        let q = build_query(None, &c, &app);
        assert_eq!(q.prefix, "27");
        assert_eq!(q.floor, Some((24, 0)), "the floor is kept; every 27 is above it");
    }

    #[test]
    fn an_explicit_request_is_the_prefix_even_below_the_floor() {
        let (c, app) = chosen(Some("27.5"), r#"{"application":"28.0.0.0"}"#);
        let q = build_query(Some("27.5"), &c, &app);
        assert_eq!((q.prefix.as_str(), q.explicit, q.floor), ("27.5", true, Some((28, 0))));
    }

    #[test]
    fn no_floor_and_no_request_uses_the_fallback_major() {
        let (c, app) = chosen(None, r#"{"id":"x"}"#);
        let q = build_query(None, &c, &app);
        assert_eq!((q.prefix.as_str(), q.floor), ("27", None));
    }

    #[test]
    fn reads_the_pinned_corpus_app() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus/tests/al-language");
        let v = read_app_versions(&dir).unwrap();
        // Corpus bump 24: the corpus app targets the newest runtime (18.0); each oracle leg compiles it with its own (oracle.toml app_runtime).
        assert_eq!(v.runtime, Some(Runtime { major: 18, minor: 0 }), "ADR-015 2.0");
    }
}
