//! The Variant.IsXxx methods that corpus PR 1382 measured first (al-runner issue 7963; types/variant/VariantIsMatrix.al:135-493,
//! not pinned yet, assumption `variant-is-matrix`): IsTransactionType, IsDataClassificationType and IsSecurityFiltering answer
//! only for the kinds the probe held ([`measured`]); every other kind refuses. IsOption of a TransactionType value is false. The
//! IsJson and IsXml methods stay in `xml::lists`.

use super::*;
use crate::assumptions::{Assumption, assume};
use crate::library_kind::LibraryKind;

/// The kinds of value the probe put into a Variant (VariantIsMatrix.al:135 TransactionType, :176 unassigned, :186 simple kinds,
/// :250 date and time kinds, :288 record kinds, :322 streams and collections, :365 JSON, :393 platform options, :445 and :493
/// XML). An XmlNode counts only while it holds an element (:493).
fn measured(c: &mut dyn Ctx, v: Option<&mut Value>) -> Result<bool, Signal> {
    let Some(v) = v else { return Ok(true) };
    Ok(match v {
        Value::Null
        | Value::Boolean(_)
        | Value::Integer(_)
        | Value::BigInteger(_)
        | Value::Decimal(_)
        | Value::Byte(_)
        | Value::Char(_)
        | Value::Text(_)
        | Value::Code(_)
        | Value::TextConst(_)
        | Value::Enum(..)
        | Value::Date(..)
        | Value::Time(_)
        | Value::DateTime(_)
        | Value::Duration(_)
        | Value::DateFormula(_)
        | Value::Guid(_)
        | Value::Record(_)
        | Value::Codeunit(_)
        | Value::List(_)
        | Value::Dictionary(_) => true,
        // An option the app declares, or one of the platform options the probe held (:393 and :135).
        Value::Option(_, members) => {
            !crate::sysenum::is_platform(members)
                || crate::sysenum::kind_of(members).is_some()
                || transaction_type(members)
        }
        Value::Library(LibraryKind::XmlNode, _) => crate::xml::variant_node_is_element(c, Some(v))?,
        Value::Library(kind, _) => {
            matches!(
                kind,
                LibraryKind::RecordId
                    | LibraryKind::RecordRef
                    | LibraryKind::FieldRef
                    | LibraryKind::InStream
                    | LibraryKind::OutStream
                    | LibraryKind::TextBuilder
                    | LibraryKind::Notification
                    | LibraryKind::FilterPageBuilder
                    | LibraryKind::JsonArray
                    | LibraryKind::JsonObject
                    | LibraryKind::JsonToken
                    | LibraryKind::JsonValue
                    | LibraryKind::XmlAttribute
                    | LibraryKind::XmlAttributeCollection
                    | LibraryKind::XmlCData
                    | LibraryKind::XmlComment
                    | LibraryKind::XmlDeclaration
                    | LibraryKind::XmlDocument
                    | LibraryKind::XmlDocumentType
                    | LibraryKind::XmlElement
                    | LibraryKind::XmlNamespaceManager
                    | LibraryKind::XmlNameTable
                    | LibraryKind::XmlNodeList
                    | LibraryKind::XmlProcessingInstruction
                    | LibraryKind::XmlText
            ) || ["XmlReadOptions", "XmlWriteOptions"]
                .iter()
                .any(|n| kind.as_str().eq_ignore_ascii_case(n))
        }
        _ => false,
    })
}

fn transaction_type(members: &[String]) -> bool {
    crate::sysenum::members("transactiontype")
        .is_some_and(|known| members.len() == known.len() && members.iter().zip(known).all(|(a, b)| a == b))
}

/// `Variant.Is<kind>` for one of the methods of this module: a TransactionType value is IsTransactionType (:135), a
/// DataClassification value is IsDataClassificationType (:393), and IsSecurityFiltering is true for none of the held kinds.
fn is_kind(c: &mut dyn Ctx, r: Option<&mut Value>, kind: &str) -> Result<Value, Signal> {
    let held = r.as_deref().map(|v| v.describe()).unwrap_or_default();
    let mut r = r;
    if !measured(c, r.as_deref_mut())? {
        return unsupported(format!(
            "Variant.Is{kind} of a {held} value (corpus PR 1382 measures only the kinds it puts into a Variant)"
        ));
    }
    // The default TransactionType value is IsTransactionType: pinned by types/TestVariantExtended.al:238.
    if kind == "TransactionType" && matches!(r.as_deref(), Some(Value::Option(0, m)) if transaction_type(m)) {
        return Ok(Value::Boolean(true));
    }
    assume(Assumption::VariantIsMatrix);
    let answer = match r {
        None | Some(Value::Null) => false,
        Some(Value::Option(_, members)) => match kind {
            "TransactionType" => transaction_type(members),
            "DataClassificationType" => crate::sysenum::kind_of(members) == Some("dataclassification"),
            _ => false,
        },
        Some(_) => false,
    };
    Ok(Value::Boolean(answer))
}

/// `Variant.IsOption`: a TransactionType value is no Option (:135, every TransactionType value answers only IsTransactionType);
/// every other value answers as `Value::is` says.
fn is_option(r: Option<&mut Value>) -> Result<Value, Signal> {
    if let Some(Value::Option(_, members)) = r.as_deref() {
        if transaction_type(members) {
            assume(Assumption::VariantIsMatrix);
            return Ok(Value::Boolean(false));
        }
    }
    is(r, "Option")
}

macro_rules! variant_is {
    ($($kind:literal),* $(,)?) => {
        &[$((concat!("Variant.Is", $kind, "#1"), |c, r, _| is_kind(c, r, $kind))),*]
    };
}

pub(super) const OPTION: (&str, NativeFn) = ("Variant.IsOption#1", |_, r, _| is_option(r));

pub(super) const REGISTRY: &[(&str, NativeFn)] =
    variant_is!("TransactionType", "DataClassificationType", "SecurityFiltering");
