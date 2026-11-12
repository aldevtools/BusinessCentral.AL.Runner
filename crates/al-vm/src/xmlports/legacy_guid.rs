//! The import of a text into a Guid field by a port without FormatEvaluate (Legacy). No pinned corpus test measures it yet: it
//! runs under the assumption `xmlport-guid-legacy-import`, whose probe is corpus pull request 1340
//! (`xmlports/import/XpLegacyGuidImport.al`, values from a local BC run and that pull request's green corpus CI; al-runner
//! issue 7854). Only the cells that file measures are answered; every other text keeps main's
//! refusal (`xp_value`).

use super::shape::{XmlGuid, xml_guid};
use super::*;

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
    /// The site of node kind `kind` in a port of format `format`; `None` for a FixedText port and other kinds (not measured).
    pub(super) fn of(format: ir::XmlFormat, kind: ir::XmlNodeKind) -> Option<Self> {
        match (format, kind) {
            (ir::XmlFormat::Xml, ir::XmlNodeKind::FieldAttr) => Some(Self::XmlAttribute),
            (ir::XmlFormat::Xml, ir::XmlNodeKind::Field) => Some(Self::XmlElement),
            (ir::XmlFormat::VariableText, ir::XmlNodeKind::Field) => Some(Self::VariableText),
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
    let t = raw.trim_matches(' ');
    // A text with a character that is no hex digit, dash or bracket fails at every site (:213, :244, :267 `not-a-guid`).
    if t.chars()
        .any(|c| !(c.is_ascii_hexdigit() || matches!(c, '-' | '{' | '}' | '(' | ')')))
    {
        return XmlGuid::Invalid;
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
    fn only_xml_and_variable_text_fields_have_a_site() {
        assert_eq!(
            LegacyGuidSite::of(ir::XmlFormat::FixedText, ir::XmlNodeKind::Field),
            None
        );
        assert_eq!(
            LegacyGuidSite::of(ir::XmlFormat::VariableText, ir::XmlNodeKind::FieldAttr),
            None
        );
    }
}
