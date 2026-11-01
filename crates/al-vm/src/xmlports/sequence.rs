//! The child-order check of an imported element against the schema.

use super::*;

/// Checks the child elements of an imported element against the schema's sequence: the children
/// in declaration order, each once (a table element any number of times) unless it is optional
/// (`MinOccurs = Zero`). The messages are .NET's schema validation messages
/// (xmlport/TestXmlPortExportImportContracts.al:XmlPortImport_SchemaMismatch_ThrowsXmlPortInvalidXml).
pub(super) struct Sequence {
    /// `UseLax = true` on the port.
    lax: bool,
    decl: Vec<usize>,
    parent: String,
    pos: usize,
    seen: usize,
    optional: Vec<bool>,
    repeat: Vec<bool>,
    /// The name an element has in the document: `prefix:Name` for a prefixed field element.
    names: Vec<String>,
    /// The name as an error lists it, quoted: `'Name'`, or `'Name' in namespace 'uri'` for a prefixed element.
    shown: Vec<String>,
}

impl Sequence {
    pub(super) fn new(schema: &Schema, node: usize) -> Sequence {
        let decl: Vec<usize> = schema
            .children(node)
            .into_iter()
            .filter(|&c| {
                !matches!(
                    schema.node(c).kind,
                    ir::XmlNodeKind::TextAttr | ir::XmlNodeKind::FieldAttr
                )
            })
            .collect();
        Sequence {
            lax: schema.xp().use_lax,
            parent: schema.node(node).xml_name.clone(),
            pos: 0,
            seen: 0,
            optional: decl.iter().map(|&c| schema.node(c).min_zero).collect(),
            repeat: decl.iter().map(|&c| repeats(schema, c)).collect(),
            names: decl.iter().map(|&c| doc_name(schema, c)).collect(),
            shown: decl
                .iter()
                .map(|&c| {
                    let n = schema.node(c);
                    match n
                        .ns_prefix
                        .as_deref()
                        .and_then(|p| schema.xp().namespaces.iter().find(|(q, _)| q == p))
                    {
                        Some((_, uri)) => format!("'{}' in namespace '{uri}'", n.xml_name),
                        None => format!("'{}'", n.xml_name),
                    }
                })
                .collect(),
            decl,
        }
    }

    /// The names that may come next.
    pub(super) fn expected(&self) -> Vec<String> {
        let mut out = Vec::new();
        if self.pos >= self.decl.len() {
            return out;
        }
        let done = self.seen >= 1 || self.optional[self.pos];
        if self.seen == 0 || self.repeat[self.pos] {
            out.push(self.shown[self.pos].clone());
        }
        if done {
            for j in self.pos + 1..self.decl.len() {
                out.push(self.shown[j].clone());
                if !self.optional[j] {
                    break;
                }
            }
        }
        out
    }

    /// Several possible elements are listed in one pair of quotes, `'Opt, Once'` (xmlport/W7XmlPortTests.al:129); with a
    /// namespace among them the form is not pinned.
    pub(super) fn list(&self, head: String) -> Result<String, Signal> {
        let expected = self.expected();
        match expected.len() {
            0 => Ok(head),
            1 => Ok(format!("{head} List of possible elements expected: {}.", expected[0])),
            _ if expected.iter().any(|e| e.contains(" in namespace ")) => unsupported(
                "a schema error that lists several possible elements, one of them in a namespace (the list form is not pinned)",
            ),
            _ => {
                let names: Vec<&str> = expected.iter().map(|e| e.trim_matches('\'')).collect();
                Ok(format!(
                    "{head} List of possible elements expected: '{}'.",
                    names.join(", ")
                ))
            }
        }
    }

    /// The schema node that reads child element `ce`, `None` for an element a UseLax port skips.
    /// An element that declares a default namespace (`xmlns="uri"`) is in that namespace, which no node of the port is: a strict
    /// port names it in the error, a UseLax port skips it (YXmlPortProps.al:UseLax_Import_AcceptsUnknownElements). A UseLax port's
    /// schema ends every sequence with "any element in namespace '##other'", which its error lists. Both are pinned only for an
    /// element after the last element of the sequence; elsewhere the list is not pinned and is refused.
    pub(super) fn accept(&mut self, schema: &Schema, ce: &Elem) -> Result<Option<usize>, Signal> {
        let name = ce.name.as_str();
        if let Some(ns) = ce.attr("xmlns").filter(|ns| !ns.is_empty()) {
            if !self.expected().is_empty() {
                return unsupported(
                    "XmlPort import of an element of another namespace where the sequence still expects elements (not pinned)",
                );
            }
            if self.lax {
                return Ok(None);
            }
            return Err(schema_error(format!(
                "The element '{}' has invalid child element '{name}' in namespace '{ns}'.",
                self.parent
            )));
        }
        if let Some(found) = self.accept_name(schema, name)? {
            return Ok(Some(found));
        }
        if self.lax {
            if !self.expected().is_empty() {
                return unsupported(
                    "XmlPort import into a UseLax port of an unknown element where the sequence still expects elements (not pinned)",
                );
            }
            return Err(schema_error(format!(
                "The element '{}' has invalid child element '{name}'. List of possible elements expected: any element in namespace '##other'.",
                self.parent
            )));
        }
        Err(schema_error(self.list(format!(
            "The element '{}' has invalid child element '{name}'.",
            self.parent
        ))?))
    }

    /// The schema node of the sequence named `name`, `None` when none may come here.
    pub(super) fn accept_name(&mut self, _schema: &Schema, name: &str) -> Result<Option<usize>, Signal> {
        if self.pos < self.decl.len() {
            let cur = self.pos;
            if self.names[cur] == name && (self.seen == 0 || self.repeat[cur]) {
                self.seen += 1;
                return Ok(Some(self.decl[cur]));
            }
            if self.seen >= 1 || self.optional[cur] {
                for j in cur + 1..self.decl.len() {
                    if self.names[j] == name {
                        self.pos = j;
                        self.seen = 1;
                        return Ok(Some(self.decl[j]));
                    }
                    if !self.optional[j] {
                        break;
                    }
                }
            }
        }
        Ok(None)
    }

    pub(super) fn finish(&self, _schema: &Schema) -> Result<(), Signal> {
        if self.decl.is_empty() {
            return Ok(());
        }
        let done = self.seen >= 1 || self.optional[self.pos];
        if done && (self.pos + 1..self.decl.len()).all(|j| self.optional[j]) {
            return Ok(());
        }
        Err(schema_error(self.list(format!(
            "The element '{}' has incomplete content.",
            self.parent
        ))?))
    }
}
