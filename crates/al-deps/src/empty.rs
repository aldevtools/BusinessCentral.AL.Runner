//! A package that holds no objects at all: no `.al` file and a `SymbolReference.json` that lists none (an app kept as
//! an empty placeholder, al-runner issue 6924). Its AL source is complete, since there is none to carry, so it serves
//! a dependency as an app with no files, as the `Application` aggregate needs no source. Tooling only (ADR-009): the
//! dependents compile against the same nothing on BC.

/// The largest `SymbolReference.json` read to find out. A package with objects that has no source (a symbols-only
/// package such as Base Application's) has a file far larger than this, and is not inflated for the question.
pub const MAX_SYMBOLS_LEN: usize = 64 * 1024;

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
    use super::holds_no_objects;

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
