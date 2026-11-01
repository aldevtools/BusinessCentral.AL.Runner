//! `XmlNodeList` natives and the `Variant.Is*` checks.

use super::*;

/// The nodes of an `XmlNodeList` as `XmlNode` values, for `foreach`. `None` for any other value.
pub fn node_list_items(v: &Value) -> Option<Vec<Value>> {
    let Value::Library(_, h) = v else { return None };
    match &*h.borrow() {
        LibraryValue::XmlList(items) => Some(items.iter().map(|n| tagged("XmlNode", n.clone())).collect()),
        // An XmlAttributeCollection gives its XmlAttributes in document order (types/PVLabelSplitForeach.al,
        // staging 8217373).
        LibraryValue::XmlAttrs(owner) => Some(attrs_of(owner).into_iter().map(|a| tagged("XmlAttribute", a)).collect()),
        _ => None,
    }
}

pub(super) fn list_items(r: Option<&mut Value>) -> Result<Vec<Handle>, Signal> {
    match r {
        Some(Value::Library(_, h)) => match &*h.borrow() {
            LibraryValue::XmlList(v) => Ok(v.clone()),
            LibraryValue::Empty => Ok(Vec::new()),
            _ => unsupported("XmlNodeList method on something else"),
        },
        _ => unsupported("XmlNodeList method without a receiver"),
    }
}

pub(super) fn list_count(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> NResult {
    Ok(Value::Integer(list_items(r)?.len() as i32))
}

pub(super) fn list_get(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    let items = list_items(r)?;
    let Value::Integer(i) = a[0] else {
        return unsupported("node list index that is not an Integer");
    };
    match usize::try_from(i - 1).ok().and_then(|i| items.get(i).cloned()) {
        Some(n) => {
            a[1] = tagged("XmlNode", n);
            Ok(Value::Boolean(true))
        }
        None if c.ret_used() => Ok(Value::Boolean(false)),
        None => unsupported("XmlNodeList.Get out of range, result unused (error text not pinned yet)"),
    }
}

// ---------------------------------------------------------- Variant.Is*

/// A Variant holding an `XmlNode` reports the kind of the node it holds, never `XmlNode` itself (types/lt-variant-is-text-streams/
/// LTP89IsXmlNodes.al:Node_FromEachKind, NodeVsSpecific_Casting: `IsXmlDocument` for a document node, `IsXmlNode` No).
pub(super) fn variant_has(v: &Value, tag: &str) -> bool {
    match v {
        Value::Library(n, h) if n.eq_ignore_ascii_case("XmlNode") && is_xml(h) => tag != "XmlNode" && kind(h) == tag,
        v => v.is(tag),
    }
}

macro_rules! variant_is {
    ($($fname:ident => $tag:literal),* $(,)?) => {
        $(pub(super) fn $fname(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> NResult {
            Ok(Value::Boolean(r.is_some_and(|v| variant_has(v, $tag))))
        })*

        pub(super) fn variant_is_fn(method: &str) -> Option<NativeFn> {
            Some(match method {
                $(concat!("Is", $tag) => $fname,)*
                _ => return None,
            })
        }
    };
}
variant_is! {
    v_is_xml_attribute => "XmlAttribute",
    v_is_xml_attribute_collection => "XmlAttributeCollection",
    v_is_xml_cdata => "XmlCData",
    v_is_xml_comment => "XmlComment",
    v_is_xml_declaration => "XmlDeclaration",
    v_is_xml_document => "XmlDocument",
    v_is_xml_document_type => "XmlDocumentType",
    v_is_xml_element => "XmlElement",
    v_is_xml_name_table => "XmlNameTable",
    v_is_xml_namespace_manager => "XmlNamespaceManager",
    v_is_xml_node => "XmlNode",
    v_is_xml_node_list => "XmlNodeList",
    v_is_xml_pi => "XmlProcessingInstruction",
    v_is_xml_read_options => "XmlReadOptions",
    v_is_xml_text => "XmlText",
    v_is_xml_write_options => "XmlWriteOptions",
    v_is_json_array => "JsonArray",
    v_is_json_object => "JsonObject",
    v_is_json_token => "JsonToken",
    v_is_json_value => "JsonValue",
}
