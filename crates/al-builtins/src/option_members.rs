//! The member list of an Option declared without members, and the mark of one that a routine stored into its `var`
//! parameter while the parameter held its caller's members (assumption var-option-caller-members).
//!
//! A `var` parameter declared without members reads the members of the variable handed in (corpus PR 1432,
//! OptionVarMemberLists.al:241). What a name read inside the routine shows after the routine stored into such a parameter
//! is not measured: the store gives the value the list [`stored`] returns, and the readers of a member name refuse it
//! ([`is_stored`]). The ordinal readers, comparisons and conversions read it as any memberless Option.

use std::sync::{Arc, LazyLock};

/// Whether `m` is the member list of an Option declared without members: `Option` and `Option ""` both read as one nameless
/// member (al-sema `ast::TypeRef::option_members`).
pub fn memberless(m: &[String]) -> bool {
    m.len() == 1 && m[0].is_empty()
}

static STORED: LazyLock<Arc<[String]>> = LazyLock::new(|| Arc::from(vec![String::new()]));

/// The memberless list of a value stored into a `var` parameter declared without members that held its caller's members.
/// The list reads as any memberless list; only its identity tells it apart ([`is_stored`]).
pub fn stored() -> Arc<[String]> {
    STORED.clone()
}

/// Whether `m` is the list [`stored`] returns.
pub fn is_stored(m: &Arc<[String]>) -> bool {
    Arc::ptr_eq(m, &STORED)
}

/// The refusal of a member-name read (`what`) of a value with the list [`stored`].
pub fn stored_refusal(what: &str) -> String {
    format!(
        "{what} of a `var` Option parameter declared without members, after the routine stored into it while it held its caller's members (not measured, assumption var-option-caller-members)"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stored_list_reads_as_memberless_and_only_it_is_stored() {
        assert!(memberless(&stored()));
        assert!(is_stored(&stored()));
        let other: Arc<[String]> = Arc::from(vec![String::new()]);
        assert!(memberless(&other));
        assert!(!is_stored(&other));
        assert!(!memberless(&["A".to_string(), "B".to_string()]));
    }
}
