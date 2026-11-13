//! The name check of XmlElement.Create and XmlAttribute.Create (al-runner issue 7964 item 1). A plain name goes on as before
//! (`check_name`). A name that is not plain raises BC's error only in the cells the pinned corpus covers ([`pinned`]) and
//! the cells corpus PR 1382 measured (xml/XmlNameValidity.al:113-241, not pinned yet, assumption `xml-create-invalid-name`),
//! per entry point; every other name keeps `check_name`'s refusal.

use super::*;
use crate::assumptions::{Assumption, assume};

/// The entry point that checks the name, with the namespace argument of the namespace forms.
#[derive(Clone, Copy)]
pub(super) enum Site<'a> {
    /// `XmlElement.Create(Name)`, no content.
    Element,
    /// `XmlElement.Create(Name, Ns)`, no content.
    ElementNs(&'a str),
    /// `XmlAttribute.Create(Name, Value)`.
    Attribute,
    /// `XmlAttribute.Create(Name, Ns, Value)`.
    AttributeNs(&'a str),
}

/// How a name that is not plain breaks the rules, as the probe's names do.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Fault {
    Empty,
    /// The first character cannot begin a name.
    Begins(char),
    /// The first bad character, after one or more ASCII letters.
    Includes(char),
}

fn fault_of(name: &str) -> Option<Fault> {
    let first = name.chars().next()?;
    if !first.is_ascii_alphabetic() {
        return Some(Fault::Begins(first));
    }
    name.chars().find(|c| !c.is_ascii_alphabetic()).map(Fault::Includes)
}

/// Whether the pinned corpus covers `name`'s fault at `site`: XmlElement.Create(Name) with '', '1a', 'a b', 'a<b', 'a:b',
/// 'a/b', '-a' and 'a&b' (xml/XmlElement.al:98-141); XmlAttribute.Create(Name, Value) with '', 'a b', '1a', 'a=b' and
/// 'xml:lang' (xml/XmlAttributes.al:104-131).
fn pinned(fault: Fault, site: Site<'_>) -> bool {
    match site {
        Site::Element => matches!(fault, Fault::Empty | Fault::Begins('1' | '-') | Fault::Includes(' ' | '<' | ':' | '/' | '&')),
        Site::Attribute => matches!(fault, Fault::Empty | Fault::Begins('1') | Fault::Includes(' ' | '=' | ':')),
        Site::ElementNs(_) | Site::AttributeNs(_) => false,
    }
}

/// Whether the probe measured `name`'s fault at `site`: the empty name at every entry point (:113, :162, :194, :226);
/// '1.0' at every entry point (:113, :162, :194, :226); '9abc' without a namespace (:130, :209); '.a' and ':a' with
/// XmlElement.Create(Name) (:145); ':' after letters ('urn:Example-Namespace:TEST', 'p:name', 'a:') and ' ' after letters
/// ('name with space') at every entry point (:113, :130, :145, :162, :177, :194, :209, :226, :241). The namespace forms used
/// 'urn:x'; XmlElement.Create('p:name', '') (:177) used the empty namespace.
fn measured(fault: Fault, site: Site<'_>) -> bool {
    let common = matches!(fault, Fault::Empty | Fault::Begins('1') | Fault::Includes(':' | ' '));
    match site {
        Site::Element => common || matches!(fault, Fault::Begins('9' | '.' | ':')),
        Site::Attribute => common || fault == Fault::Begins('9'),
        Site::ElementNs("urn:x") | Site::AttributeNs("urn:x") => common,
        Site::ElementNs("") => fault == Fault::Includes(':'),
        Site::ElementNs(_) | Site::AttributeNs(_) => false,
    }
}

/// Checks `name` for `site`: a plain name passes; a pinned or measured fault raises BC's error (an empty name the Argument error, any
/// other the XmlException with the character and its position, `verify_ncname`); anything else is refused by `check_name`.
pub(super) fn check_create_name(name: &str, site: Site<'_>) -> Result<(), Signal> {
    if is_ncname(name) {
        return Ok(());
    }
    let fault = if name.is_empty() { Some(Fault::Empty) } else { fault_of(name) };
    match fault.map(|f| (pinned(f, site), measured(f, site))) {
        Some((true, _)) => {}
        Some((false, true)) => assume(Assumption::XmlCreateInvalidName),
        _ => return check_name(name),
    }
    verify_ncname(name)?;
    unsupported(format!("XML name `{name}` that the name check accepts (not pinned)"))
}
