//! A package that holds no objects at all: no `.al` file and a `SymbolReference.json` that lists none (an app kept as
//! an empty placeholder, al-runner issue 6924). Its AL source is complete, since there is none to carry, so it serves
//! a dependency as an app with no files, as the `Application` aggregate needs no source. Tooling only (ADR-009): the
//! dependents compile against the same nothing on BC.
//!
//! The second kind of evidence is a Microsoft source archive with no `.al` file (al-runner issue 8102): Manufacturing,
//! Manufacturing Tests, Service Management, Service Management Tests and Tests-Local on BC 27, 28 and 29, Report Layouts
//! Tests and `_Exclude_ReportLayouts` on BC 27 and 28. In each of these builds the artifact's compiled package of the
//! same app (`Microsoft_<name>.app` next to the archive) has a `SymbolReference.json` whose every list is empty: BC
//! compiled the archive to an app with no objects. The `Application` aggregate keeps its own rule (`select.rs`).

/// The largest `SymbolReference.json` read to find out. A package with objects that has no source (a symbols-only
/// package such as Base Application's) has a file far larger than this, and is not inflated for the question.
const MAX_SYMBOLS_LEN: usize = 64 * 1024;

/// Whether a package with no `.al` file holds no objects: one of Microsoft's source archives above (`source_archive`,
/// its manifest from `app.json`, publisher `Microsoft` as every measured archive), or a package whose
/// `SymbolReference.json` (`symbols`, read by `read` only when it is small enough) lists none. Another publisher's
/// archive without a `.al` file, and the `Application` aggregate, are not the first kind.
pub fn package_holds_no_objects(
    manifest: &crate::package::Manifest,
    source_archive: bool,
    symbols: Option<&crate::zip::Entry>,
    read: impl FnOnce(&crate::zip::Entry) -> Option<Vec<u8>>,
) -> bool {
    let microsoft_archive = source_archive
        && manifest.publisher == "Microsoft"
        && !manifest.id.eq_ignore_ascii_case(crate::select::APPLICATION_APP_ID);
    microsoft_archive
        || symbols
            .filter(|e| e.size <= MAX_SYMBOLS_LEN)
            .and_then(read)
            .is_some_and(|bytes| holds_no_objects(&bytes))
}

/// Whether the bytes of a `SymbolReference.json` show a package with no objects. Only positive evidence counts: the
/// file is a JSON object that names at least one list of objects (`Tables`, `Codeunits`, ..., or `Namespaces`), and
/// every list in it, and in each of its namespaces, is empty. A file that cannot be read, or one without any list
/// (`{}`), proves nothing and keeps the package symbols only.
pub fn holds_no_objects(symbols: &[u8]) -> bool {
    let bytes = symbols.strip_prefix(b"\xef\xbb\xbf").unwrap_or(symbols);
    let Ok(serde_json::Value::Object(top)) = serde_json::from_slice::<serde_json::Value>(bytes) else {
        return false;
    };
    let mut lists = 0;
    no_objects_in(&top, &mut lists) && lists > 0
}

/// Every array of `object` is empty, but `Namespaces`, whose entries are objects of the same shape. `lists` counts
/// the arrays seen.
fn no_objects_in(object: &serde_json::Map<String, serde_json::Value>, lists: &mut usize) -> bool {
    object.iter().all(|(key, value)| match value {
        serde_json::Value::Array(items) => {
            *lists += 1;
            if key == "Namespaces" {
                items
                    .iter()
                    .all(|n| matches!(n, serde_json::Value::Object(o) if no_objects_in(o, lists)))
            } else {
                items.is_empty()
            }
        }
        _ => true,
    })
}

#[cfg(test)]
mod tests {
    use super::{holds_no_objects, package_holds_no_objects};
    use crate::package::{Vars, manifest_of_source_zip};

    /// A source archive without a `.al` file holds no objects only for publisher `Microsoft`, not for the
    /// `Application` aggregate (its own rule in `select.rs`), and not when the manifest came from a compiled package.
    #[test]
    fn only_a_microsoft_source_archive_without_al_files_holds_no_objects() {
        let manifest = |id: &str, publisher: &str| {
            let json = format!(r#"{{"id":"{id}","name":"N","publisher":"{publisher}","version":"1.0.0.0"}}"#);
            manifest_of_source_zip(&json, &Vars::default()).unwrap()
        };
        let none = |_: &crate::zip::Entry| None;
        assert!(package_holds_no_objects(
            &manifest("4d6c453d-e61d-4872-b35d-6c731c0fd93e", "Microsoft"),
            true,
            None,
            none
        ));
        assert!(!package_holds_no_objects(
            &manifest("4d6c453d-e61d-4872-b35d-6c731c0fd93e", "Microsoft"),
            false,
            None,
            none
        ));
        assert!(!package_holds_no_objects(&manifest("aa", "P"), true, None, none));
        assert!(!package_holds_no_objects(
            &manifest("aa", "microsoft"),
            true,
            None,
            none
        ));
        let application = manifest(&crate::select::APPLICATION_APP_ID.to_ascii_uppercase(), "Microsoft");
        assert!(!package_holds_no_objects(&application, true, None, none));
    }

    #[test]
    fn only_symbols_that_list_no_object_show_an_empty_package() {
        assert!(holds_no_objects(br#"{"Tables":[],"Codeunits":[],"Namespaces":[]}"#));
        assert!(holds_no_objects(
            b"\xef\xbb\xbf{\"Namespaces\":[{\"Name\":\"A\",\"Pages\":[]}]}"
        ));
        assert!(!holds_no_objects(br#"{"Tables":[],"Codeunits":[{"Id":1}]}"#));
        assert!(!holds_no_objects(
            br#"{"Namespaces":[{"Name":"A","Codeunits":[{"Id":1}]}]}"#
        ));
        assert!(!holds_no_objects(br#"{"Namespaces":["odd"]}"#));
        assert!(!holds_no_objects(b"{}"), "no list, no evidence");
        assert!(!holds_no_objects(b"not json"));
    }
}
