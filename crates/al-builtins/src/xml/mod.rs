//! XmlDocument, XmlElement, XmlAttribute, XmlText, XmlCData, XmlComment,
//! XmlProcessingInstruction, XmlDeclaration, XmlNode, XmlNodeList and
//! XmlAttributeCollection.
//!
//! Every node type is a handle onto one node tree (`Rc<RefCell<LibraryValue>>`),
//! like JSON: `XmlElement.AsXmlNode()`, `XmlNode.AsXmlElement()` and `GetRoot`
//! return handles to the same node, so an edit through any of them is visible
//! everywhere (xml/TestXmlDeep.al Remove / ReplaceWith act on nodes fetched from
//! node lists). Lists and attribute collections are views. Behaviour the corpus
//! does not pin raises `Signal::Unsupported` (see docs/probes-needed.md).

use crate::describe::Describe;
use crate::json::Handle;
use crate::native::{Ctx, NativeFn, Signal};
use crate::signature::Signature;
use crate::value::{LibraryValue, Value};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

mod attrs;
mod dom;
mod edit;
mod lists;
mod manager;
mod names;
mod node_types;
mod parse;
mod queries;
mod read;
mod read_tree;
mod select;
mod tree;
mod write;
mod write_to;
mod xpath;

use attrs::*;
use dom::*;
use edit::*;
use lists::*;
use manager::*;
use names::*;
use node_types::*;
use parse::*;
use queries::*;
use read::*;
use select::*;
use tree::*;
use write::*;

pub(crate) use dom::uninitialized;
pub use dom::{XData, XMLNS_URI, XmlCell};
pub use lists::node_list_items;
pub use manager::{NsDecl, NsScopes};
pub(crate) use read_tree::{read_tree, read_tree_in};
pub(crate) use write::{escape_attr, escape_text};
use write_to::*;
use xpath::*;

pub(super) type NResult = Result<Value, Signal>;

pub(super) fn list_of(items: Vec<Handle>) -> Value {
    Value::Library(
        "XmlNodeList".into(),
        Rc::new(RefCell::new(LibraryValue::XmlList(items))),
    )
}

pub(super) fn arg_text(a: &[Value], i: usize) -> Result<String, Signal> {
    a.get(i)
        .and_then(crate::native::text_of)
        .map(String::from)
        .ok_or_else(|| Signal::Unsupported(format!("XML argument {i} is not text")))
}

/// The implementation of one catalog overload, if the corpus pins enough of it.
/// Overloads with streams, read/write options or namespace managers are not implemented.
pub fn native_for(sig: &Signature) -> Option<NativeFn> {
    let params: Vec<&str> = sig.params.iter().map(|p| p.ty.as_str()).collect();
    let first_by_var = sig.params.first().is_some_and(|p| p.by_var);
    if sig.receiver == "Variant" && sig.instance {
        return variant_is_fn(&sig.name).filter(|_| params.is_empty());
    }
    let r = sig.receiver.as_str();
    // Every node type shares these.
    let node_types = [
        "XmlDocument",
        "XmlElement",
        "XmlAttribute",
        "XmlText",
        "XmlCData",
        "XmlComment",
        "XmlProcessingInstruction",
        "XmlDeclaration",
        "XmlDocumentType",
        "XmlNode",
    ];
    let is_node = node_types.contains(&r);
    let f: NativeFn = match (r, sig.name.as_str(), params.as_slice()) {
        ("XmlDocument", "ReadFrom", ["Text", "XmlDocument"]) => read_from_text,
        ("XmlDocument", "ReadFrom", ["Text", "XmlReadOptions", "XmlDocument"]) => read_from_text_options,
        ("XmlDocument", "ReadFrom", ["InStream", "XmlDocument"]) => read_from_stream,
        ("XmlDocument", "ReadFrom", ["InStream", "XmlReadOptions", "XmlDocument"]) => read_from_stream_options,
        ("XmlDocument", "Create", _) if !sig.instance => doc_create,
        ("XmlElement", "Create", ["Text"] | ["Text", "Any"]) => element_create_name,
        ("XmlElement", "Create", ["Text", "Text"] | ["Text", "Text", "Any"]) => element_create_ns,
        ("XmlAttribute", "Create", ["Text", "Text"]) => attribute_create,
        ("XmlAttribute", "Create", ["Text", "Text", "Text"]) => attribute_create_ns,
        ("XmlAttribute", "CreateNamespaceDeclaration", [_, _]) => attribute_create_ns_decl,
        ("XmlText", "Create", [_]) => text_create,
        ("XmlCData", "Create", [_]) => cdata_create,
        ("XmlComment", "Create", [_]) => comment_create,
        ("XmlProcessingInstruction", "Create", [_, _]) => pi_create,
        ("XmlDeclaration", "Create", [_, _, _]) => decl_create,
        ("XmlDocumentType", "Create", _) if !sig.instance => doctype_create,
        ("XmlDocumentType", "GetName", [_]) => doctype_get_name,
        ("XmlDocumentType", "GetPublicId", [_]) => doctype_get_public,
        ("XmlDocumentType", "GetSystemId", [_]) => doctype_get_system,
        ("XmlDocumentType", "GetInternalSubset", [_]) => doctype_get_subset,
        ("XmlDocumentType", "SetName", [_]) => doctype_set_name,
        ("XmlDocumentType", "SetPublicId", [_]) => doctype_set_public,
        ("XmlDocumentType", "SetSystemId", [_]) => doctype_set_system,
        ("XmlDocumentType", "SetInternalSubset", [_]) => doctype_set_subset,
        ("XmlDeclaration", "Version", _) => decl_version,
        ("XmlDeclaration", "Encoding", _) => decl_encoding,
        ("XmlDeclaration", "Standalone", _) => decl_standalone,
        ("XmlText" | "XmlCData" | "XmlComment" | "XmlAttribute", "Value", _) => value_prop,
        ("XmlProcessingInstruction", "GetTarget", [_]) => pi_get_target,
        ("XmlProcessingInstruction", "GetData", [_]) => pi_get_data,
        ("XmlProcessingInstruction", "SetTarget", ["Text"]) => pi_set_target,
        ("XmlProcessingInstruction", "SetData", ["Text"]) => pi_set_data,
        ("XmlElement" | "XmlAttribute", "Name", []) => element_name,
        ("XmlElement" | "XmlAttribute", "LocalName", []) => local_name,
        ("XmlElement" | "XmlAttribute", "NamespaceUri", []) => namespace_uri,
        ("XmlAttribute", "NamespacePrefix", []) => namespace_prefix,
        ("XmlAttribute", "IsNamespaceDeclaration", []) => is_namespace_declaration,
        ("XmlElement", "GetPrefixOfNamespace", [_, _]) => get_prefix_of_namespace,
        ("XmlElement", "GetNamespaceOfPrefix", [_, _]) => get_namespace_of_prefix,
        ("XmlElement" | "XmlDocument", "Add", _) => add,
        ("XmlElement" | "XmlDocument", "AddFirst", _) => add_first,
        (_, "AddAfterSelf", _) if is_node => add_after_self,
        (_, "AddBeforeSelf", _) if is_node => add_before_self,
        (_, "Remove", []) if is_node => remove,
        (_, "ReplaceWith", _) if is_node => replace_with,
        ("XmlElement" | "XmlDocument", "RemoveNodes", []) => remove_nodes,
        ("XmlElement" | "XmlDocument", "ReplaceNodes", _) => replace_nodes,
        ("XmlDocument", "GetRoot", [_]) => get_root,
        ("XmlDocument", "GetDeclaration", [_]) => get_declaration,
        ("XmlDocument", "SetDeclaration", [_]) => set_declaration,
        ("XmlDocument", "GetDocumentType", [_]) => get_document_type,
        ("XmlDocument", "NameTable", []) => name_table,
        (_, "GetDocument", [_]) if is_node => get_document,
        (_, "GetParent", [_]) if is_node => get_parent,
        (_, "AsXmlNode", []) if is_node => as_xml_node,
        ("XmlNode", "AsXmlAttribute", []) => as_attribute,
        ("XmlNode", "AsXmlCData", []) => as_cdata,
        ("XmlNode", "AsXmlComment", []) => as_comment,
        ("XmlNode", "AsXmlDeclaration", []) => as_declaration,
        ("XmlNode", "AsXmlDocument", []) => as_document,
        ("XmlNode", "AsXmlDocumentType", []) => as_doctype,
        ("XmlNode", "AsXmlElement", []) => as_element,
        ("XmlNode", "AsXmlProcessingInstruction", []) => as_pi,
        ("XmlNode", "AsXmlText", []) => as_text,
        ("XmlNode", "IsXmlAttribute", []) => is_attribute,
        ("XmlNode", "IsXmlCData", []) => is_cdata,
        ("XmlNode", "IsXmlComment", []) => is_comment,
        ("XmlNode", "IsXmlDeclaration", []) => is_declaration,
        ("XmlNode", "IsXmlDocument", []) => is_document,
        ("XmlNode", "IsXmlDocumentType", []) => is_doctype,
        ("XmlNode", "IsXmlElement", []) => is_element,
        ("XmlNode", "IsXmlProcessingInstruction", []) => is_pi,
        ("XmlNode", "IsXmlText", []) => is_text,
        ("XmlElement", "HasAttributes", []) => has_attributes,
        ("XmlElement", "HasElements", []) => has_elements,
        ("XmlElement", "IsEmpty", []) => is_empty,
        ("XmlElement", "InnerText", []) => inner_text_fn,
        ("XmlElement", "InnerXml", []) => inner_xml,
        ("XmlElement" | "XmlDocument", "GetChildElements", [] | ["Text"] | ["Text", "Text"]) => get_child_elements,
        ("XmlElement" | "XmlDocument", "GetChildNodes", []) => get_child_nodes,
        ("XmlElement" | "XmlDocument", "GetDescendantElements", [] | ["Text"] | ["Text", "Text"]) => {
            get_descendant_elements
        }
        ("XmlElement" | "XmlDocument", "GetDescendantNodes", []) => get_descendant_nodes,
        (_, "SelectSingleNode", ["Text", "XmlNode"]) if is_node => select_single_node,
        (_, "SelectNodes", ["Text", "XmlNodeList"]) if is_node => select_nodes,
        (_, "SelectNodes", ["Text", "XmlNamespaceManager", "XmlNodeList"]) if is_node => select_nodes_ns,
        (_, "SelectSingleNode", ["Text", "XmlNamespaceManager", "XmlNode"]) if is_node => select_single_node_ns,
        ("XmlNamespaceManager", "NameTable", [] | ["XmlNameTable"]) => manager_name_table,
        ("XmlNamespaceManager", "AddNamespace", ["Text", "Text"]) => manager_add_namespace,
        ("XmlNamespaceManager", "LookupNamespace", ["Text", "Text"]) => manager_lookup_namespace,
        ("XmlNamespaceManager", "HasNamespace", ["Text"]) => manager_has_namespace,
        ("XmlNamespaceManager", "LookupPrefix", ["Text", "Text"]) => manager_lookup_prefix,
        ("XmlNamespaceManager", "PushScope", []) => manager_push_scope,
        ("XmlNamespaceManager", "PopScope", []) => manager_pop_scope,
        ("XmlNamespaceManager", "RemoveNamespace", ["Text", "Text"]) => manager_remove_namespace,
        (_, "WriteTo", ["Text"]) if is_node && first_by_var => write_to_text,
        (_, "WriteTo", ["XmlWriteOptions", "Text"]) if is_node && sig.params.get(1).is_some_and(|p| p.by_var) => {
            write_to_text_options
        }
        ("XmlAttribute", "WriteTo", ["OutStream"]) => attribute_write_to_outstream,
        ("XmlDeclaration", "WriteTo", ["OutStream"] | ["XmlWriteOptions", "OutStream"]) => declaration_write_to_outstream,
        // The byte order mark and the markup, as for an element (xml/lt-xml-cdata-comment-pi-text/P90CDataRead.al:
        // WriteTo_OutStream_Attached_AndText, `Ù‹<![CDATA[x]]>`, 16 bytes; the Text, Comment and ProcessingInstruction files pin their own).
        (
            "XmlDocument"
            | "XmlElement"
            | "XmlNode"
            | "XmlText"
            | "XmlCData"
            | "XmlComment"
            | "XmlProcessingInstruction"
            // Pinned like the text kinds (xml/lt-xml-remaining-node-methods/P104WriteDocType.al:199 P104_Stream_Detached,
            // `Ù‹<!DOCTYPE r >`, 16 bytes).
            | "XmlDocumentType",
            "WriteTo",
            ["OutStream"] | ["XmlWriteOptions", "OutStream"],
        ) => write_to_outstream,
        ("XmlWriteOptions", "PreserveWhitespace", ["Boolean"]) => write_options_preserve_whitespace,
        ("XmlReadOptions", "PreserveWhitespace", ["Boolean"]) => read_options_preserve_whitespace,
        ("XmlReadOptions" | "XmlWriteOptions", "PreserveWhitespace", []) => options_preserve_whitespace_get,
        ("XmlElement", "SetAttribute", ["Text", "Text"]) => set_attribute,
        ("XmlElement", "RemoveAttribute", ["Text"] | ["XmlAttribute"] | ["Text", "Text"]) => remove_attribute,
        ("XmlAttributeCollection", "Remove", ["XmlAttribute"] | ["Text"] | ["Text", "Text"]) => attrs_remove,
        ("XmlElement", "RemoveAllAttributes", []) => remove_all_attributes,
        ("XmlElement", "Attributes", []) => attributes,
        ("XmlAttributeCollection", "Count", []) => attrs_count,
        ("XmlAttributeCollection", "Set", ["Text", "Text"]) => attrs_set,
        ("XmlAttributeCollection", "Set", ["Text", "Text", "Text"]) => attrs_set_ns,
        ("XmlElement", "SetAttribute", ["Text", "Text", "Text"]) => set_attribute_ns,
        ("XmlAttributeCollection", "RemoveAll", []) => attrs_remove_all,
        ("XmlAttributeCollection", "Get", ["Integer", "XmlAttribute"]) => attrs_get_index,
        ("XmlAttributeCollection", "Get", ["Text", "XmlAttribute"]) => attrs_get_name,
        ("XmlAttributeCollection", "Get", ["Text", "Text", "XmlAttribute"]) => attrs_get_name_ns,
        ("XmlNodeList", "Count", []) => list_count,
        ("XmlNodeList", "Get", ["Integer", "XmlNode"]) => list_get,
        _ => return None,
    };
    Some(f)
}

/// `Format` of an `XmlDocumentType` or `XmlDeclaration`: its markup. An unassigned variable raises `NotInitialized` naming
/// `XmlNode` (DocTypeErrors.al:FormatOfUninitialisedAndInitialised).
pub fn format_node(name: &str, h: &Handle) -> Result<String, Signal> {
    if matches!(&*h.borrow(), LibraryValue::Empty) {
        return Err(uninitialized("XmlNode"));
    }
    if with(h, |d| matches!(d, XData::DocType { ready: false, .. })) {
        return unsupported("Format of a document type made without a subset (not pinned)");
    }
    let _ = name;
    serialize(h)
}
