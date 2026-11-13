//! The import of a text into a Guid field by a port without FormatEvaluate (Legacy). No pinned corpus test measures it yet: it
//! runs under the assumption `xmlport-guid-legacy-import`, whose probe is corpus pull request 1340
//! (`xmlports/import/XpLegacyGuidImport.al`, values from a local BC run and that pull request's green corpus CI; al-runner
//! issue 7854). Only the cells that file measures are answered; every other text keeps main's
//! refusal (`xp_value`).

use super::shape::{XmlGuid, xml_guid};
use super::*;

/// How the text of a field node stood in the source.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::xmlports) enum Written {
    /// The value of an attribute.
    Attribute,
    /// The text of an element; `self_closed`: the element was written `<x />`.
    Element { self_closed: bool },
    /// A value of a line of a text format; `quoted`: it stood in the field delimiter.
    Value { quoted: bool },
}

/// Where the Guid text stands: the three places the probe measures.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum LegacyGuidSite {
    /// A field attribute of a Format = Xml port (port 75181, attribute `Ref`).
    XmlAttribute,
    /// A field element of a Format = Xml port (port 75181, element `Ref2`).
    XmlElement,
    /// A field of a Format = VariableText port (port 75182).
    VariableText,
}

impl LegacyGuidSite {
    /// The site of a text `written` for a node of kind `kind` in a port of format `format`, each fact as the probe's ports
    /// have it (XpLegacyGuidImport.al):
    /// - a field attribute of a Format = Xml port (port 75181, :32-51);
    /// - a field element of that port written with a start and an end tag (:174 `<Ref2></Ref2>`);
    /// - a field of a Format = VariableText port with the default delimiter and separators (`default_separators`, from
    ///   `assigned::default_separators`; port 75182, :53-71), the value standing in the field delimiter (:263-267).
    ///
    /// `None` for everything else: a self-closed element, a value outside the delimiter, other delimiters and separators,
    /// FixedText and other node kinds are not measured.
    pub(super) fn of(
        format: ir::XmlFormat,
        default_separators: bool,
        kind: ir::XmlNodeKind,
        written: Written,
    ) -> Option<Self> {
        match (format, kind, written) {
            (ir::XmlFormat::Xml, ir::XmlNodeKind::FieldAttr, Written::Attribute) => Some(Self::XmlAttribute),
            (ir::XmlFormat::Xml, ir::XmlNodeKind::Field, Written::Element { self_closed: false }) => {
                Some(Self::XmlElement)
            }
            (ir::XmlFormat::VariableText, ir::XmlNodeKind::Field, Written::Value { quoted: true })
                if default_separators =>
            {
                Some(Self::VariableText)
            }
            _ => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Brackets {
    Braces,
    Parentheses,
    None,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Digits {
    /// 8-4-4-4-12 digits with dashes.
    Dashed,
    /// The 32 digits alone.
    Bare,
    /// 8-4-4-4-11 digits with dashes, one digit short.
    Short,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Case {
    Upper,
    Lower,
    /// Digits only (the null Guid).
    NoLetters,
}

/// The form of a Guid text by the parts the probe varies: brackets, digits, letter case and spaces around.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Form {
    brackets: Brackets,
    digits: Digits,
    case: Case,
    spaced: bool,
}

/// What the import of `raw` (the text before the trim) at `site` gives, as measured by XpLegacyGuidImport.al (corpus pull
/// request 1340). `Unpinned` for every text the file does not measure.
pub(super) fn legacy_guid(site: LegacyGuidSite, raw: &str) -> XmlGuid {
    use LegacyGuidSite::*;
    // An empty attribute is the null Guid (:211 `empty`); an empty element and an empty VariableText field fail (:243, :266).
    if raw.is_empty() {
        return if site == XmlAttribute {
            XmlGuid::Value([0; 16])
        } else {
            XmlGuid::Invalid
        };
    }
    // The one failing text measured at every site is `not-a-guid` (:213, :244, :267). A text with each of its facts fails.
    if like_the_invalid_sample(raw) {
        return XmlGuid::Invalid;
    }
    let t = raw.trim_matches(' ');
    // Any other text with a character that is no hex digit, dash or bracket is not measured.
    if t.chars()
        .any(|c| !(c.is_ascii_hexdigit() || matches!(c, '-' | '{' | '}' | '(' | ')')))
    {
        return XmlGuid::Unpinned;
    }
    let Some(form) = form_of(raw, t) else {
        return XmlGuid::Unpinned;
    };
    let f = |brackets, digits, case, spaced| Form {
        brackets,
        digits,
        case,
        spaced,
    };
    use Brackets as B;
    use Case as C;
    use Digits as D;
    let value = match site {
        // :203-210 export form, null in braces, lower in braces, upper and lower without braces, parentheses, 32 digits,
        // spaces around; :178-183 the round trip of the null Guid.
        XmlAttribute => [
            f(B::Braces, D::Dashed, C::Upper, false),
            f(B::Braces, D::Dashed, C::NoLetters, false),
            f(B::Braces, D::Dashed, C::Lower, false),
            f(B::None, D::Dashed, C::Upper, false),
            f(B::None, D::Dashed, C::Lower, false),
            f(B::Parentheses, D::Dashed, C::Upper, false),
            f(B::None, D::Bare, C::Upper, false),
            f(B::Braces, D::Dashed, C::Upper, true),
        ]
        .contains(&form),
        // :240-242 and :263-265 export form, lower in braces, upper without braces; :178-194 the round trips (null Guid too).
        XmlElement | VariableText => [
            f(B::Braces, D::Dashed, C::Upper, false),
            f(B::Braces, D::Dashed, C::NoLetters, false),
            f(B::Braces, D::Dashed, C::Lower, false),
            f(B::None, D::Dashed, C::Upper, false),
        ]
        .contains(&form),
    };
    if value {
        return xml_guid(t);
    }
    // :214 one digit short in braces, upper case, in an attribute.
    if site == XmlAttribute && form == f(B::Braces, D::Short, C::Upper, false) {
        return XmlGuid::Invalid;
    }
    XmlGuid::Unpinned
}

/// `raw` has the facts of `not-a-guid` (XpLegacyGuidImport.al:213, :244, :267): only ASCII lower-case letters and dashes (so
/// no white space, bracket, digit or upper-case letter), at least one dash and at least one letter outside a-f.
fn like_the_invalid_sample(raw: &str) -> bool {
    raw.chars().all(|c| c.is_ascii_lowercase() || c == '-')
        && raw.contains('-')
        && raw.chars().any(|c| ('g'..='z').contains(&c))
}

/// The form of `t` (`raw` without the spaces around), or `None` for brackets that do not pair, digits in other groups and
/// mixed case.
fn form_of(raw: &str, t: &str) -> Option<Form> {
    let (brackets, inner) = match (t.chars().next(), t.chars().last()) {
        (Some('{'), Some('}')) if t.len() > 1 => (Brackets::Braces, &t[1..t.len() - 1]),
        (Some('('), Some(')')) if t.len() > 1 => (Brackets::Parentheses, &t[1..t.len() - 1]),
        _ => (Brackets::None, t),
    };
    if inner.contains(['{', '}', '(', ')']) {
        return None;
    }
    let lens: Vec<usize> = inner.split('-').map(str::len).collect();
    let digits = match lens.as_slice() {
        [8, 4, 4, 4, 12] => Digits::Dashed,
        [32] => Digits::Bare,
        [8, 4, 4, 4, 11] => Digits::Short,
        _ => return None,
    };
    let upper = inner.chars().any(|c| c.is_ascii_uppercase());
    let lower = inner.chars().any(|c| c.is_ascii_lowercase());
    let case = match (upper, lower) {
        (true, false) => Case::Upper,
        (false, true) => Case::Lower,
        (false, false) => Case::NoLetters,
        (true, true) => return None,
    };
    Some(Form {
        brackets,
        digits,
        case,
        spaced: t.len() != raw.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use LegacyGuidSite::*;

    fn kind(g: XmlGuid) -> &'static str {
        match g {
            XmlGuid::Value(_) => "value",
            XmlGuid::Invalid => "invalid",
            XmlGuid::Unpinned => "unpinned",
        }
    }

    #[test]
    fn the_measured_attribute_forms() {
        for t in [
            "{6F9619FF-8B86-D011-B42D-00C04FC964FF}",
            "{00000000-0000-0000-0000-000000000000}",
            "{6f9619ff-8b86-d011-b42d-00c04fc964ff}",
            "6F9619FF-8B86-D011-B42D-00C04FC964FF",
            "6f9619ff-8b86-d011-b42d-00c04fc964ff",
            "(6F9619FF-8B86-D011-B42D-00C04FC964FF)",
            "6F9619FF8B86D011B42D00C04FC964FF",
            " {6F9619FF-8B86-D011-B42D-00C04FC964FF} ",
            "",
        ] {
            assert_eq!(kind(legacy_guid(XmlAttribute, t)), "value", "{t}");
        }
        assert_eq!(kind(legacy_guid(XmlAttribute, "not-a-guid")), "invalid");
        assert_eq!(
            kind(legacy_guid(XmlAttribute, "{6F9619FF-8B86-D011-B42D-00C04FC964F}")),
            "invalid"
        );
    }

    #[test]
    fn the_measured_element_and_variable_text_forms() {
        for site in [XmlElement, VariableText] {
            for t in [
                "{0A1B2C3D-4E5F-6789-ABCD-EF0123456789}",
                "{0a1b2c3d-4e5f-6789-abcd-ef0123456789}",
                "0A1B2C3D-4E5F-6789-ABCD-EF0123456789",
            ] {
                assert_eq!(kind(legacy_guid(site, t)), "value", "{site:?} {t}");
            }
            assert_eq!(kind(legacy_guid(site, "")), "invalid", "{site:?}");
            assert_eq!(kind(legacy_guid(site, "not-a-guid")), "invalid", "{site:?}");
        }
    }

    #[test]
    fn forms_the_probe_does_not_measure_are_unpinned() {
        // Measured only in an attribute.
        for site in [XmlElement, VariableText] {
            for t in [
                "6f9619ff-8b86-d011-b42d-00c04fc964ff",
                "(6F9619FF-8B86-D011-B42D-00C04FC964FF)",
                "6F9619FF8B86D011B42D00C04FC964FF",
                " {6F9619FF-8B86-D011-B42D-00C04FC964FF} ",
                "{6F9619FF-8B86-D011-B42D-00C04FC964F}",
            ] {
                assert_eq!(kind(legacy_guid(site, t)), "unpinned", "{site:?} {t}");
            }
        }
        // Measured nowhere.
        for t in [
            "{6F9619FF-8b86-D011-B42D-00C04FC964FF}",
            "{6F9619FF8B86D011B42D00C04FC964FF}",
            "(6f9619ff-8b86-d011-b42d-00c04fc964ff)",
            "{6F9619FF-8B86-D011-B42D-00C04FC964FF)",
            " 6F9619FF-8B86-D011-B42D-00C04FC964FF",
            "6F9619FF-8B86-D011-B42D-00C04FC964F",
            "{6f9619ff-8b86-d011-b42d-00c04fc964f}",
            "6F9619FF",
        ] {
            assert_eq!(kind(legacy_guid(XmlAttribute, t)), "unpinned", "{t}");
        }
    }

    #[test]
    fn a_text_unlike_the_invalid_sample_is_unpinned() {
        for site in [XmlAttribute, XmlElement, VariableText] {
            for t in [
                "\t{6F9619FF-8B86-D011-B42D-00C04FC964FF}\t",
                "\n  {6F9619FF-8B86-D011-B42D-00C04FC964FF}\n",
                "\u{a0}{6F9619FF-8B86-D011-B42D-00C04FC964FF}",
                "{6F9619FF-8B86-D011-B42D-00C04FC964FF}x",
                "6F9619FF-8B86-D011-B42D-00C04FC964FG",
                "{6F9619FF-8B86-D011-B42D-00C04FC964FF }",
                "NOT-A-GUID",
                "Not-a-guid",
                "x",
                "g",
                "-",
                "abc-def",
                "not-a-guid-1",
                " not-a-guid",
                "not-a-guid ",
                "not a guid",
                "(not-a-guid)",
            ] {
                assert_eq!(kind(legacy_guid(site, t)), "unpinned", "{site:?} {t:?}");
            }
        }
    }

    #[test]
    fn the_site_needs_every_fact_of_the_measured_ports() {
        use ir::XmlFormat as F;
        use ir::XmlNodeKind as K;
        let (attr, open_close, quoted) = (
            Written::Attribute,
            Written::Element { self_closed: false },
            Written::Value { quoted: true },
        );
        assert_eq!(LegacyGuidSite::of(F::Xml, true, K::FieldAttr, attr), Some(XmlAttribute));
        assert_eq!(LegacyGuidSite::of(F::Xml, true, K::Field, open_close), Some(XmlElement));
        assert_eq!(
            LegacyGuidSite::of(F::VariableText, true, K::Field, quoted),
            Some(VariableText)
        );
        // One fact flipped at a time.
        let self_closed = Written::Element { self_closed: true };
        let unquoted = Written::Value { quoted: false };
        assert_eq!(LegacyGuidSite::of(F::Xml, true, K::Field, self_closed), None);
        assert_eq!(LegacyGuidSite::of(F::VariableText, true, K::Field, unquoted), None);
        assert_eq!(LegacyGuidSite::of(F::VariableText, false, K::Field, quoted), None);
        assert_eq!(LegacyGuidSite::of(F::FixedText, true, K::Field, quoted), None);
        assert_eq!(LegacyGuidSite::of(F::VariableText, true, K::FieldAttr, quoted), None);
        assert_eq!(LegacyGuidSite::of(F::Xml, true, K::Field, attr), None);
        assert_eq!(LegacyGuidSite::of(F::Xml, true, K::FieldAttr, open_close), None);
    }
}
