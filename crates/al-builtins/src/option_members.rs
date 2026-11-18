//! The member list of an Option declared without members.

/// Whether `m` is the member list of an Option declared without members: `Option` and `Option ""` both read as one nameless
/// member (al-sema `ast::TypeRef::option_members`).
pub fn memberless(m: &[String]) -> bool {
    m.len() == 1 && m[0].is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_one_nameless_member_is_memberless() {
        assert!(memberless(&[String::new()]));
        assert!(!memberless(&["A".to_string(), "B".to_string()]));
        assert!(!memberless(&[]));
    }
}
