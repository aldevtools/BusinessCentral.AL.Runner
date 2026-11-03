//! `Page.Run` and `Page.RunModal` with a fully qualified page name, from BC 28 (corpus master pages/lt-y74-fqn-run/
//! Y74FqnRunForms.al):
//! - `Namespace.Name` and `Namespace."Name"` find the page, upper and lower case alike (Run_Name_Spellings);
//! - a name without a namespace, and a page number as text, raise `The provided object name is not fully qualified.` with the
//!   code `InvalidOperation` (Err_NoNamespace, Err_PageIdAsText);
//! - a name that is no page, or that no object has, is not pinned (the corpus holds those cases back as Linux suspects) and refused.

use super::*;

impl Session {
    /// The page number of the page named `name` (`Namespace.Name`).
    pub(super) fn page_id_by_qualified_name(&self, name: &str) -> Result<i32, Signal> {
        let name = name.trim();
        // The last dot outside double quotes splits the namespace from the object name.
        let mut quoted = false;
        let mut split = None;
        for (i, c) in name.char_indices() {
            match c {
                '"' => quoted = !quoted,
                '.' if !quoted => split = Some(i),
                _ => {}
            }
        }
        let Some(at) = split else {
            return Err(error_code(
                "The provided object name is not fully qualified.",
                "InvalidOperation",
            ));
        };
        let (ns, object) = (&name[..at], name[at + 1..].trim().trim_matches('"'));
        let found = self.engine.program.objects.iter().find(|o| {
            o.kind == ir::ObjKind::Page
                && !o.uncompiled
                && o.name.eq_ignore_ascii_case(object)
                && o.namespace.as_deref().unwrap_or("").eq_ignore_ascii_case(ns)
        });
        match found.and_then(|o| o.id) {
            Some(id) => Ok(id as i32),
            None => unsupported("Page.Run with a fully qualified name that names no page (not pinned)"),
        }
    }
}
