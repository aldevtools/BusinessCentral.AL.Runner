//! The platform's normaliser of User."Authentication Email" (moved out of `usertable.rs`).

/// Why an authentication email was not normalised.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum EmailProblem {
    /// Not an address the server keeps: it refuses it (pinned).
    Invalid,
    /// A form the corpus does not show: refused as unsupported.
    Unpinned(String),
}

/// The characters of an unquoted local part besides letters and digits (RFC 5322 atext and the dot), all kept as given
/// (Y66AuthEmailLocalPart.al:45-63, '!#$%&''*+/=?^_`{|}~@example.com' = ok, and the braces of a formatted Guid).
const LOCAL_SIGNS: &str = "!#$%&'*+/=?^_`{|}~-.";

/// A letter or digit, ASCII or not ('jürgen@example.com' and 'a@exämple.com' are kept, Y66AuthEmailLocalPart.al:107, :112).
fn letter_or_digit(c: char) -> bool {
    c.is_alphanumeric()
}

/// The platform's normaliser for "Authentication Email" (`doubled_dot_invalid` from BC 29 on, gate `ProbedValuesDifferFrom29`
/// with the evidence Y66AuthEmailLocalPart.al:89, the BC29PLUS guards of Y66_AuthEmail_VersionDependent): surrounding whitespace goes, `Name <a@b.c>`
/// becomes `a@b.c`, a comma separated list becomes its last address, white space alone becomes empty
/// (TestUserAuthEmailTrigger.al:44-125). An address without `@` is invalid (:143-159).
///
/// The forms corpus writer Y66 measured on real BC 27.5, 28.5 and 29.0 (tests/al-language-onprem/access-control/
/// lt-y66-auth-email-local-part/Y66AuthEmailLocalPart.al, al-runner issue 6662; not on corpus master yet, assumption
/// `user-auth-email-rfc-forms`): a quoted local part is kept with its quotes; an unquoted one may hold letters, digits and
/// `!#$%&'*+/=?^_`{|}~-.` and end with a dot, but not start with one; a comment right before the `@` is dropped; a word and a
/// space before the address drop the word (`a b@example.com` keeps `b@example.com`); the domain may be dotless, an IPv4
/// literal in brackets, or hold `_` and a leading `-`; two `@`, an empty local part and an empty domain are invalid; a doubled
/// dot is invalid from BC 29 on and kept before (`doubled_dot_invalid`). Anything else is `Unpinned`.
pub(crate) fn normalise_email(text: &str, doubled_dot_invalid: bool) -> Result<String, EmailProblem> {
    let unpinned = |what: &str| Err(EmailProblem::Unpinned(format!("{what} in the address '{text}'")));
    let all = text.trim();
    if all.is_empty() {
        return Ok(String::new());
    }
    if let Some(c) = all.chars().find(|c| ";\\".contains(*c) || c.is_control()) {
        return unpinned(&format!("the character '{}'", c.escape_default()));
    }
    if all.contains('"') && all.contains(',') {
        return unpinned("a quote and a comma");
    }
    let last = all.rsplit(',').next().unwrap_or(all).trim();
    if last.is_empty() {
        return unpinned("an empty entry");
    }
    let address = match (last.find('<'), last.find('>')) {
        (None, None) => last,
        (Some(open), Some(close)) if close == last.len() - 1 && open < close => {
            if last[open + 1..close].contains('<') {
                return unpinned("a nested angle bracket");
            }
            if last[..open].contains(|c| "\"()[]".contains(c)) {
                return unpinned("a quote, parenthesis or bracket in the display name");
            }
            last[open + 1..close].trim()
        }
        _ => return unpinned("an unbalanced angle bracket"),
    };
    let flagged = address.contains(|c: char| !(c.is_ascii_alphanumeric() || "@._+-".contains(c)))
        || address.starts_with('@')
        || address.ends_with('@')
        || address.ends_with('.')
        || address.contains("..")
        || address.matches('@').count() > 1;
    let r = split_address(address, doubled_dot_invalid, &unpinned);
    if flagged && !matches!(r, Err(EmailProblem::Unpinned(_))) {
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::UserAuthEmailRfcForms);
    }
    r
}

fn split_address(
    address: &str,
    doubled_dot_invalid: bool,
    unpinned: &dyn Fn(&str) -> Result<String, EmailProblem>,
) -> Result<String, EmailProblem> {
    // A quoted local part: kept with its quotes, an `@` or a space inside it included (Y66AuthEmailLocalPart.al:50-51).
    let (local, domain) = if let Some(rest) = address.strip_prefix('"') {
        let Some(close) = rest.find('"') else {
            return unpinned("an unclosed quote");
        };
        let Some(domain) = rest[close + 1..].strip_prefix('@') else {
            return unpinned("a quoted local part not followed by '@'");
        };
        (address[..close + 2].to_string(), domain)
    } else {
        if address.contains('"') {
            return unpinned("a quote inside the local part");
        }
        let (mut local, domain) = match address.matches('@').count() {
            0 => return Err(EmailProblem::Invalid),
            1 => {
                let (l, d) = address.split_once('@').unwrap_or_default();
                (l.to_string(), d)
            }
            _ => return Err(EmailProblem::Invalid),
        };
        // A comment right before the `@` is dropped ('a(comment)@example.com' keeps 'a@example.com').
        if let Some(open) = local.find('(') {
            let inner = &local[open + 1..];
            if !local.ends_with(')') || inner[..inner.len() - 1].contains(['(', ')']) || open == 0 {
                return unpinned("a comment that is not right before the '@'");
            }
            local.truncate(open);
        }
        if local.contains(')') {
            return unpinned("an unbalanced parenthesis");
        }
        // A word and a space before the address: the word goes ('a b@example.com' keeps 'b@example.com').
        if let Some(at) = local.rfind(char::is_whitespace) {
            if !local[..at].chars().all(|c| letter_or_digit(c) || c == ' ') {
                return unpinned("a display name that is not words");
            }
            local = local[at + 1..].to_string();
        }
        if local.is_empty() {
            return Err(EmailProblem::Invalid);
        }
        if !local.chars().all(|c| letter_or_digit(c) || LOCAL_SIGNS.contains(c)) {
            return unpinned("a character outside the local-part signs");
        }
        if local.starts_with('.') || (doubled_dot_invalid && local.contains("..")) {
            return Err(EmailProblem::Invalid);
        }
        (local, domain)
    };
    if domain.is_empty() {
        return Err(EmailProblem::Invalid);
    }
    if let Some(literal) = domain.strip_prefix('[') {
        let Some(ip) = literal.strip_suffix(']') else {
            return unpinned("an unclosed domain literal");
        };
        let parts: Vec<&str> = ip.split('.').collect();
        if parts.len() != 4
            || !parts
                .iter()
                .all(|p| !p.is_empty() && p.len() <= 3 && p.chars().all(|c| c.is_ascii_digit()))
        {
            return unpinned("a domain literal that is not an IPv4 address");
        }
    } else {
        if !domain.chars().all(|c| letter_or_digit(c) || "._-".contains(c)) {
            return unpinned("a character outside letters, digits and ._- in the domain");
        }
        if domain.starts_with('.') || domain.ends_with('.') {
            return unpinned("a domain that starts or ends with a dot");
        }
        if doubled_dot_invalid && domain.contains("..") {
            return Err(EmailProblem::Invalid);
        }
    }
    Ok(format!("{local}@{domain}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(s: &str) -> String {
        normalise_email(s, true).unwrap()
    }

    fn invalid(s: &str, newest: bool) -> bool {
        normalise_email(s, newest) == Err(EmailProblem::Invalid)
    }

    /// access-control/TestUserAuthEmailTrigger.al:44-125.
    #[test]
    fn the_pinned_forms_are_normalised() {
        assert_eq!(ok("   a@t.example.com    "), "a@t.example.com");
        assert_eq!(ok("Test Person <a@t.example.com>"), "a@t.example.com");
        assert_eq!(ok("  Test Person <a@t.example.com>  "), "a@t.example.com");
        assert_eq!(ok("a@t.example.com, d@t.example.com"), "d@t.example.com");
        assert_eq!(ok("a@t.example.com"), "a@t.example.com");
        assert_eq!(ok("    "), "");
        assert_eq!(ok(""), "");
    }

    /// access-control/TestUserAuthEmailTrigger.al:143-159.
    #[test]
    fn an_address_without_an_at_sign_is_invalid() {
        assert!(invalid("Abc.example.com", true));
    }

    /// access-control/lt-y66-auth-email-local-part/Y66AuthEmailLocalPart.al (al-runner issue 6662): Y66_AuthEmail_LocalParts
    /// :45-63, _Domains :66-86, _VersionDependent :89-125 and the reporter's shape :25-42.
    #[test]
    fn the_forms_y66_measured_are_kept_as_given() {
        for s in [
            "{524B1D71-5F95-43BA-AC9B-E1E5E17D8765}@EXAMPLE.COM",
            "{524b1d71-5f95-43ba-ac9b-e1e5e17d8765}@example.com",
            "a{b}c@example.com",
            "!#$%&'*+/=?^_`{|}~@example.com",
            "\"quoted local\"@example.com",
            "\"a@b\"@example.com",
            "a@[127.0.0.1]",
            "a@localhost",
            "a@ex_ample.com",
            "a@-example.com",
            "a.@example.com",
            "jürgen@example.com",
            "UPPER.Case@Example.COM",
            "a@exämple.com",
        ] {
            assert_eq!(ok(s), s);
        }
        assert_eq!(ok("a b@example.com"), "b@example.com");
        assert_eq!(ok("a(comment)@example.com"), "a@example.com");
        assert_eq!(
            ok("Y66 Name <{524B1D71-5F95-43BA-AC9B-E1E5E17D8766}@example.com>"),
            "{524B1D71-5F95-43BA-AC9B-E1E5E17D8766}@example.com"
        );
    }

    #[test]
    fn the_forms_y66_measured_as_invalid_are_invalid() {
        for s in ["a@b@example.com", "@example.com", "a@", ".a@example.com"] {
            assert!(invalid(s, false) && invalid(s, true), "{s}");
        }
    }

    /// Y66_AuthEmail_VersionDependent: BC 29 refuses a doubled dot, BC 27 and 28 keep it.
    #[test]
    fn a_doubled_dot_is_invalid_from_bc_29_on() {
        for s in ["a..b@example.com", "a@example..com"] {
            assert!(invalid(s, true), "{s}");
            assert_eq!(normalise_email(s, false).unwrap(), s);
        }
    }

    #[test]
    fn forms_the_corpus_does_not_show_are_not_decided() {
        for s in [
            "\"Name\" <a@b.c>",
            "a@b.c;d@e.f",
            "<a@b.c",
            "a@b.c,",
            "a\\b@c.d",
            "a@[b.c]",
            "a@b c.d",
            "(c)a@b.c",
            "a@.b.c",
            "a@b.c.",
            "a@b!c.d",
            "\"a,b\"@c.d",
            "a€@b.c",
        ] {
            assert!(
                matches!(normalise_email(s, true), Err(EmailProblem::Unpinned(_))),
                "{s}"
            );
        }
    }
}
