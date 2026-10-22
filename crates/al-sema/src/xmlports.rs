//! XmlPort objects (Format = Xml): the schema walk, the properties and triggers the runtime
//! understands, and the IR it is lowered to.
//!
//! The runtime runs only what the corpus pins. Every property, trigger or element kind it does
//! not understand becomes a reason on the xmlport, so a variable of that xmlport is refused
//! statically (hard rule 4) instead of running with a guessed behaviour.

use crate::check::posmap::FxMap as HashMap;
use crate::members::FieldClass;
use crate::workspace::{ObjKind, Resolution, Workspace};
use al_ir as ir;
use al_syntax::ast;

/// Object properties that change nothing the runtime can observe.
const INERT_OBJECT_PROPERTIES: &[&str] = &[
    "caption",
    "captionml",
    "permissions",
    "transactiontype",
    "usedefaultnamespace",
];

/// A schema element before its names are resolved.
pub struct RawNode {
    pub name: String,
    pub xml_name: String,
    pub kind: ir::XmlNodeKind,
    pub parent: Option<usize>,
    pub section: ast::Section,
    pub args: Vec<String>,
}

pub struct Layout {
    pub nodes: Vec<RawNode>,
    pub direction: ir::XmlDirection,
    pub format: ir::XmlFormat,
    pub field_separator: Option<String>,
    pub field_delimiter: Option<String>,
    pub record_separator: Option<String>,
    pub table_separator: Option<String>,
    pub text_encoding_declared: bool,
    pub utf8: bool,
    pub xml_utf8: bool,
    pub namespaces: Vec<(String, String)>,
    pub inline_schema: bool,
    pub use_lax: bool,
    pub xml_v11: bool,
    /// `Encoding` as written.
    pub encoding: Option<String>,
    pub format_evaluate_xml: bool,
    /// `FormatEvaluate` as declared.
    pub format_evaluate_declared: Option<bool>,
    pub no_default_fields_validation: bool,
    pub reasons: Vec<String>,
    /// Reasons only running the port refuses (see `ir::XmlPort::run_unsupported`).
    pub run_reasons: Vec<String>,
    /// Reasons only an export refuses (see `ir::XmlPort::export_unsupported`).
    pub export_reasons: Vec<String>,
}

/// `Table.Field` at the first dot outside quotes: a table name may hold a dot (`"Data Exch. Def".Code`).
fn split_source_at_dot(source: &str) -> Option<(&str, &str)> {
    let mut quote: Option<char> = None;
    for (i, c) in source.char_indices() {
        match (quote, c) {
            (None, '"' | '\'') => quote = Some(c),
            (Some(q), c) if c == q => quote = None,
            (None, '.') => return Some((&source[..i], &source[i + 1..])),
            _ => {}
        }
    }
    None
}

fn bool_prop(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "yes" => Some(true),
        "false" | "no" => Some(false),
        _ => None,
    }
}

/// The triggers each element kind may declare. Anything else is refused: only these are pinned
/// by the corpus (xmlport/TestXmlPortAdvanced.al).
fn allowed_triggers(kind: ir::XmlNodeKind) -> &'static [&'static str] {
    use ir::XmlNodeKind as K;
    match kind {
        K::Text | K::TextAttr => &["onbeforepassvariable", "onafterassignvariable"],
        K::Field | K::FieldAttr => &["onafterassignfield", "onbeforepassfield"],
        K::Table => &[
            "onprexmlitem",
            "onaftergetrecord",
            "onafterinitrecord",
            "onbeforeinsertrecord",
            "onafterinsertrecord",
            "onbeforemodifyrecord",
            "onaftermodifyrecord",
        ],
    }
}

fn allowed_properties(kind: ir::XmlNodeKind) -> &'static [&'static str] {
    match kind {
        ir::XmlNodeKind::Table => &[
            "xmlname",
            "linktable",
            "linkfields",
            "autosave",
            "autoupdate",
            "autoreplace",
            "usetemporary",
            "minoccurs",
            "maxoccurs",
        ],
        // `Unbound = true` on a field or text node changes neither the import nor the export of a field or text attribute
        // (xmlport/lt-y-xmlport-props/YXmlPortProps.al:682-722); the export of an unbound field element is refused when it runs.
        ir::XmlNodeKind::Field => &[
            "xmlname",
            "minoccurs",
            "maxoccurs",
            "width",
            "namespaceprefix",
            "autocalcfield",
            "unbound",
            "fieldvalidate",
        ],
        // `occurrence` is checked in `visit`: Required or Optional on a field attribute of a table element.
        // `FieldValidate` on a field attribute: Yes validates, No does not (pages/lt-report-page-properties/06-xmlport-fvat.al:118).
        ir::XmlNodeKind::FieldAttr => &[
            "xmlname",
            "namespaceprefix",
            "autocalcfield",
            "unbound",
            "occurrence",
            "fieldvalidate",
        ],
        // `occurrence` is checked in `visit`: Required or Optional on a text attribute of a text or table element.
        ir::XmlNodeKind::TextAttr => &["xmlname", "namespaceprefix", "width", "unbound", "occurrence"],
        ir::XmlNodeKind::Text => &["xmlname", "minoccurs", "maxoccurs"],
    }
}

/// Walks the `schema` section of xmlport `obj` and checks the object's own properties.
pub fn layout(obj: &ast::Object) -> Layout {
    let mut out = Layout {
        nodes: Vec::new(),
        direction: ir::XmlDirection::Both,
        format: ir::XmlFormat::Xml,
        field_separator: None,
        field_delimiter: None,
        record_separator: None,
        table_separator: None,
        text_encoding_declared: false,
        utf8: false,
        xml_utf8: false,
        namespaces: Vec::new(),
        inline_schema: false,
        use_lax: false,
        xml_v11: false,
        encoding: None,
        format_evaluate_xml: false,
        format_evaluate_declared: None,
        no_default_fields_validation: false,
        reasons: Vec::new(),
        run_reasons: Vec::new(),
        export_reasons: Vec::new(),
    };
    for p in obj.properties() {
        let (name, value) = (p.name().to_ascii_lowercase(), p.value_text());
        match name.as_str() {
            "direction" => match value.trim().to_ascii_lowercase().as_str() {
                "import" => out.direction = ir::XmlDirection::Import,
                "export" => out.direction = ir::XmlDirection::Export,
                "both" => out.direction = ir::XmlDirection::Both,
                other => out.reasons.push(format!("Direction = {other} is not supported")),
            },
            "format" => match value.trim().to_ascii_lowercase().as_str() {
                "xml" => {}
                "variabletext" => out.format = ir::XmlFormat::VariableText,
                "fixedtext" => out.format = ir::XmlFormat::FixedText,
                other => out.reasons.push(format!("Format = {other} is not supported")),
            },
            // UseRequestPage only matters for Run, which is not implemented: Export and Import
            // never open the request page.
            "userequestpage" => {}
            // xmlport/TestXmlPortTextFormatsContracts.al: VariableText and FixedText pin these; an Xml port's encoding is not pinned.
            "textencoding" if value.trim().eq_ignore_ascii_case("utf8") => out.utf8 = true,
            "textencoding" => {
                out.text_encoding_declared = true;
                out.run_reasons.push(format!(
                    "xmlport property textencoding = {} is not pinned for a run",
                    value.trim()
                ));
            }
            // xmlport/lt-af-xmlport/AfXmlPortEncoding.al: UTF8 and UTF16 on a Format = Xml port; any other value is refused below.
            "encoding" => out.encoding = Some(value.trim().to_string()),
            // xmlport/lt-y-xmlport-props/YXmlPortProps.al:52, 563: true writes the schema inside the root (the export refuses shapes it does not pin).
            "inlineschema" => match value.trim().to_ascii_lowercase().as_str() {
                "true" => out.inline_schema = true,
                "false" => {}
                other => out
                    .run_reasons
                    .push(format!("xmlport property inlineschema = {other} is not pinned")),
            },
            // xmlport/lt-y-xmlport-props/YXmlPortProps.al:582 (UseLax_Import_AcceptsUnknownElements): the import skips an element of
            // another namespace at the end of a sequence; the run checks the shapes it pins.
            "uselax" => match value.trim().to_ascii_lowercase().as_str() {
                "true" => out.use_lax = true,
                "false" => {}
                other => out
                    .run_reasons
                    .push(format!("xmlport property uselax = {other} is not pinned")),
            },
            // YXmlPortProps.al:607 (XmlVersionNo_Export_Declaration): V10 and V11 export the same `version="1.0"` declaration, and
            // the import reads a declaration as it always does (`1.1` is refused by the reader on a V11 port as on any other).
            "xmlversionno" => match value.trim().to_ascii_lowercase().as_str() {
                "v10" => {}
                "v11" => out.xml_v11 = true,
                other => out
                    .run_reasons
                    .push(format!("xmlport property xmlversionno = {other} is not pinned")),
            },
            // YXmlPortProps.al:628 (FileName_Export_Output): an export to a stream ignores it, named or empty. Only Xmlport.Run
            // would use it, which the runtime does not implement.
            "filename" => {}
            // YXmlPortProps.al:641: grant permissions the runtime does not enforce; an export runs as without them. The values
            // read back through the XmlPort Metadata row (platform/objects.rs).
            "inherentpermissions" | "inherententitlements" => {}
            // xmlport/lt-y-xmlport-props/YXmlPortProps.al:213: one `prefix = 'uri'` pair.
            "namespaces" => match crate::scan::split_double(&value, ',').as_slice() {
                [one] => match one.split_once('=') {
                    Some((prefix, uri)) if !prefix.trim().is_empty() && uri.trim().starts_with('\'') => {
                        out.namespaces
                            .push((crate::scan::unquote_xmlport(prefix), crate::scan::unquote_xmlport(uri)));
                    }
                    _ => out.run_reasons.push(format!(
                        "xmlport namespaces `{}` is not a single prefix = 'uri' pair (not pinned)",
                        value.trim()
                    )),
                },
                _ => out
                    .run_reasons
                    .push("xmlport namespaces with several entries (not pinned)".into()),
            },
            "fieldseparator" => out.field_separator = Some(crate::scan::unquote_xmlport(&value)),
            "fielddelimiter" => out.field_delimiter = Some(crate::scan::unquote_xmlport(&value)),
            "recordseparator" => out.record_separator = Some(crate::scan::unquote_xmlport(&value)),
            "tableseparator" => {
                // Kept for the getter; it separates two tables, and a text port with two is refused when it runs.
                // A single table is assumed unaffected (docs/probes-needed.md, tableseparator).
                out.table_separator = Some(crate::scan::unquote_xmlport(&value));
            }
            // xmlport/TestXmlPortFormatEvaluateContracts.al: Legacy is the display text, Xml the XML forms.
            "formatevaluate" => match value.trim().to_ascii_lowercase().as_str() {
                "legacy" => out.format_evaluate_declared = Some(false),
                "xml" => {
                    out.format_evaluate_xml = true;
                    out.format_evaluate_declared = Some(true);
                }
                other => out
                    .run_reasons
                    .push(format!("xmlport property formatevaluate = {other} is not pinned")),
            },
            // xmlport/lt-bj-xmlport-default-validation/BjXmlPortDefaultValidation.al:255-334: true and left out validate the
            // same; false validates only FieldValidate = Yes. Pinned for Format = Xml; the import refuses another format where
            // validation would be seen (al-vm xmlports.rs).
            "defaultfieldsvalidation" => match bool_prop(&value) {
                Some(on) => out.no_default_fields_validation = !on,
                None => out.run_reasons.push(format!(
                    "xmlport property defaultfieldsvalidation = {} is not pinned",
                    value.trim()
                )),
            },
            n if INERT_OBJECT_PROPERTIES.contains(&n) => {}
            other => out
                .run_reasons
                .push(format!("xmlport property {other} is not supported yet")),
        }
    }
    if let Some(enc) = out.encoding.clone() {
        match enc.to_ascii_lowercase().as_str() {
            "utf8" if out.format == ir::XmlFormat::Xml => out.xml_utf8 = true,
            "utf16" if out.format == ir::XmlFormat::Xml => {}
            _ => out.run_reasons.push(format!(
                "xmlport property encoding = {enc} is not pinned (only UTF8 and UTF16 on a Format = Xml port are)"
            )),
        }
    }
    for top in obj.sections() {
        match top.name().to_ascii_lowercase().as_str() {
            "schema" => visit(&top, None, &mut out),
            // platform/lt-ag-contoso/AgXmlPortImport.al:94-96 (xmlport 78422), :312-327: a `requestpage { }` with
            // nothing in it changes nothing for Import and Export. The same holds with empty `layout` and `actions`
            // sections inside it, both or one alone (xmlports 87564-87566 at :835-912, imported and exported at
            // :476-512; the DemoTool's xmlport 101899 has this shape).
            "requestpage" if request_page_is_empty(&top) || request_page_has_only_empty_sections(&top) => {}
            "requestpage" => out.run_reasons.push("request pages are not supported yet".into()),
            other => out
                .reasons
                .push(format!("xmlport section {other} is not supported yet")),
        }
    }
    for i in 0..out.nodes.len() {
        if let Some(why) = occurs_refusal(&out, i) {
            out.run_reasons.push(why);
        }
    }
    out
}

fn section_is_bare(s: &ast::Section) -> bool {
    s.properties().next().is_none() && s.triggers().next().is_none() && s.var_decls().next().is_none()
}

/// A `requestpage` section with nothing in it: no property, trigger, variable or section.
fn request_page_is_empty(rp: &ast::Section) -> bool {
    section_is_bare(rp) && rp.sections().next().is_none()
}

/// A `requestpage` section that holds only empty `layout` and `actions` sections (and at least one of them).
fn request_page_has_only_empty_sections(rp: &ast::Section) -> bool {
    section_is_bare(rp)
        && rp.sections().all(|s| {
            matches!(s.name().to_ascii_lowercase().as_str(), "layout" | "actions")
                && section_is_bare(&s)
                && s.sections().next().is_none()
        })
}

fn visit(section: &ast::Section, parent: Option<usize>, out: &mut Layout) {
    use ir::XmlNodeKind as K;
    for s in section.sections() {
        let section_name = s.name().to_ascii_lowercase();
        let kind = match section_name.as_str() {
            "textelement" => K::Text,
            "textattribute" => K::TextAttr,
            "tableelement" => K::Table,
            "fieldelement" => K::Field,
            "fieldattribute" => K::FieldAttr,
            other => {
                out.reasons
                    .push(format!("xmlport schema element {other} is not supported yet"));
                continue;
            }
        };
        let args = s.args();
        let name = args.first().cloned().unwrap_or_default();
        let mut xml_name = name.clone();
        for p in s.properties() {
            let pn = p.name().to_ascii_lowercase();
            if pn == "sourcetableview" && kind == K::Table {
                // An import ignores the view: a row its filter leaves out is imported as any other (platform/lt-ag-contoso/
                // AgXmlPortImport.al:62-80, :298-309, an element without a link). Not pinned: a filtered view on a linked
                // element, and every export (which rows in which order; :312 sees a single row).
                let linked = s.properties().any(|q| q.name().eq_ignore_ascii_case("linktable"));
                if linked && p.value_text().to_ascii_lowercase().contains("where") {
                    out.run_reasons.push(format!(
                        "sourcetableview with a filter on the linked {section_name} {name} is not pinned"
                    ));
                } else {
                    out.export_reasons.push(format!(
                        "sourcetableview of {section_name} {name} is not supported yet on export"
                    ));
                }
            } else if !allowed_properties(kind).contains(&pn.as_str()) {
                // Refused when the port runs, not when it is declared: Base App's xmlport 1 sets
                // MinOccurs, MaxOccurs, Occurrence and SourceTableView, and tests reach it only
                // through a request page they cancel (handlers/TestReqPageBoundVisibility.al:172).
                out.run_reasons
                    .push(format!("property {pn} of {section_name} {name} is not supported yet"));
            } else if pn == "fieldvalidate" && bool_prop(&p.value_text()).is_none() {
                out.run_reasons.push(format!(
                    "fieldvalidate = {} of {section_name} {name} is not pinned",
                    p.value_text().trim()
                ));
            } else if pn == "xmlname" {
                xml_name = crate::scan::unquote_xmlport(&p.value_text());
            } else if pn == "occurrence" {
                // Pinned for a text attribute of the root, of a container text element and of a table element below it
                // (xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al:441-560, QS_TextAttr_*; earlier on a table
                // element below the root: report/lt-u-report-props/UPropertyTests.al:336). Below a field element it is not.
                // On a field attribute it is pinned for one of a table element, on import (platform/lt-ag-contoso/
                // AgXmlPortImport.al:27-53, :248 FieldAttribute_Occurrence_RequiredAndOptional_Import).
                let value = p.value_text().trim().to_ascii_lowercase();
                let parent_ok = parent.is_some_and(|i| match kind {
                    K::FieldAttr => out.nodes[i].kind == K::Table,
                    _ => matches!(out.nodes[i].kind, K::Text | K::Table),
                });
                if !parent_ok || !matches!(value.as_str(), "required" | "optional") {
                    out.run_reasons.push(format!(
                        "property occurrence = {value} of {section_name} {name} is not supported (not pinned)"
                    ));
                }
            }
        }
        for t in s.triggers() {
            let tn = t.name().to_ascii_lowercase();
            if !allowed_triggers(kind).contains(&tn.as_str()) {
                out.run_reasons
                    .push(format!("trigger {tn} of {section_name} {name} is not supported yet"));
            }
        }
        // A field element may hold a field attribute, a text attribute, a text element and a field element, each without nodes of
        // its own (xmlport/lt-af-xmlport/AfXmlPortAttrChildren.al:36-41, 71-75, 107-109). Other nodes below it are not pinned.
        if kind == K::Field {
            for c in s.sections() {
                let ck = c.name().to_ascii_lowercase();
                let leaf = c.sections().next().is_none();
                if !(matches!(ck.as_str(), "fieldattribute" | "textattribute")
                    || (matches!(ck.as_str(), "textelement" | "fieldelement") && leaf))
                {
                    out.run_reasons.push(format!(
                        "child node {ck} of the field element {name} is not supported (not pinned)"
                    ));
                }
            }
        }
        let mut args = args;
        if kind == K::Table {
            if let Some(table) = crate::types::section_object_name(&s, 1) {
                args[1] = table;
            }
        }
        let idx = out.nodes.len();
        out.nodes.push(RawNode {
            name,
            xml_name,
            kind,
            parent,
            section: s.clone(),
            args,
        });
        if matches!(kind, K::Text | K::Table | K::Field) {
            visit(&s, Some(idx), out);
        }
    }
}

impl Layout {
    /// Whether the text element at `i` has any child node (a container carries no value).
    pub fn has_children(&self, i: usize) -> bool {
        self.nodes.iter().any(|n| n.parent == Some(i))
    }

    pub fn property(&self, i: usize, name: &str) -> Option<String> {
        self.nodes[i]
            .section
            .properties()
            .find(|p| p.name().eq_ignore_ascii_case(name))
            .map(|p| p.value_text())
    }

    /// The nearest table node named `name` at or above node `from`.
    fn table_named(&self, from: Option<usize>, name: &str) -> Option<usize> {
        let mut cur = from;
        while let Some(i) = cur {
            if self.nodes[i].kind == ir::XmlNodeKind::Table && self.nodes[i].name.eq_ignore_ascii_case(name) {
                return Some(i);
            }
            cur = self.nodes[i].parent;
        }
        None
    }
}

/// Everything `build` needs to know about the object being lowered.
pub struct Context<'a> {
    pub ws: &'a Workspace,
    pub members: &'a crate::members::MemberTable<'a>,
    pub oi: usize,
    pub file: usize,
    pub layout: &'a Layout,
    /// The object's global slots, by lower-case name.
    pub slots: &'a HashMap<String, usize>,
    /// (object, source offset) to routine index.
    pub routine_at: &'a crate::lower::RoutineAt,
    pub source_slot: usize,
    pub destination_slot: usize,
}

fn table_object(cx: &Context, name: &str) -> Option<usize> {
    match cx
        .ws
        .resolve_ref(cx.file, ObjKind::Table, &crate::types::qualified_parts(name))
    {
        Resolution::Found(t) => Some(t),
        _ => None,
    }
}

/// The IR of xmlport `cx.oi`. `reasons` are what the schema walk and the lowering of the
/// element variables already found.
/// The `MinOccurs` and `MaxOccurs` values the corpus pins, per kind of node; any other value, or either property on another kind of node,
/// refuses running the port (not declaring it). Pinned:
/// - a leaf text element: MinOccurs Zero or Once, MaxOccurs Once or Unbounded (xmlport/W7XmlPortTests.al:111-149,
///   misc/lt-probes-needed-3/PnC19XmlPort.al:526-540);
/// - a container text element below the root: MinOccurs Zero or Once, MaxOccurs Once; the root: MaxOccurs Once
///   (xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al:67-122, QR_Export_*, QR_Import_*);
/// - a table element below a text element: MinOccurs Zero, MaxOccurs Unbounded; a field element: MaxOccurs Once (the same file,
///   :78-88, :97-99).
fn occurs_refusal(layout: &Layout, i: usize) -> Option<String> {
    use ir::XmlNodeKind as K;
    let raw = &layout.nodes[i];
    let leaf = !layout.has_children(i);
    let root = raw.parent.is_none();
    let (min_ok, max_ok): (&[&str], &[&str]) = match raw.kind {
        K::Text if root => (&[], &["once"]),
        K::Text if leaf => (&["zero", "once"], &["once", "unbounded"]),
        K::Text => (&["zero", "once"], &["once"]),
        // Below a table element (a nested table element) neither is pinned: its trigger order is not yet what BC does
        // (xmlport/lt-xmlport-trigger-flow/XmlPortTriggerFlow.al:93, Import_TriggerOrder_NewRows).
        K::Table if raw.parent.is_some_and(|p| layout.nodes[p].kind == K::Table) => (&[], &[]),
        K::Table => (&["zero"], &["unbounded"]),
        // MinOccurs of a field element was already read before these pins (any value).
        K::Field => (&["zero", "once"], &["once"]),
        _ => return None,
    };
    let kind = match raw.kind {
        K::Text if leaf => "textelement",
        K::Text => "container textelement",
        K::Table => "tableelement",
        _ => "fieldelement",
    };
    for (prop, ok) in [("MinOccurs", min_ok), ("MaxOccurs", max_ok)] {
        if let Some(v) = layout.property(i, prop) {
            let v = v.trim().to_ascii_lowercase();
            if !ok.contains(&v.as_str()) {
                return Some(format!(
                    "property {} = {v} of {kind} {} is not supported (not pinned)",
                    prop.to_ascii_lowercase(),
                    raw.name
                ));
            }
        }
    }
    None
}

pub fn build(cx: &Context, mut reasons: Vec<String>) -> ir::XmlPort {
    let layout = cx.layout;
    let mut run_reasons = layout.run_reasons.clone();
    let mut nodes: Vec<ir::XmlNode> = Vec::new();
    for (i, raw) in layout.nodes.iter().enumerate() {
        let sec = &raw.section;
        let kind = raw.kind;
        let flag = |name: &str, default: bool| {
            layout
                .property(i, name)
                .map_or(default, |v| bool_prop(&v).unwrap_or(default))
        };
        let triggers = sec
            .triggers()
            .filter_map(|t| {
                cx.routine_at
                    .get(&(cx.oi, usize::from(t.syntax().text_range().start())))
                    .map(|r| (t.name().to_ascii_lowercase(), r))
            })
            .collect();
        let mut node = ir::XmlNode {
            name: raw.name.clone(),
            xml_name: raw.xml_name.clone(),
            kind,
            parent: raw.parent,
            slot: None,
            table: None,
            table_node: None,
            field: None,
            link_node: None,
            link_fields: Vec::new(),
            auto_save: flag("AutoSave", true),
            auto_update: flag("AutoUpdate", false),
            auto_replace: flag("AutoReplace", false),
            use_temporary: flag("UseTemporary", false),
            min_zero: layout
                .property(i, "MinOccurs")
                .is_some_and(|v| v.trim().eq_ignore_ascii_case("zero")),
            max_unbounded: layout
                .property(i, "MaxOccurs")
                .map(|v| v.trim().eq_ignore_ascii_case("unbounded")),
            width: layout.property(i, "Width").and_then(|v| v.trim().parse().ok()),
            ns_prefix: layout
                .property(i, "NamespacePrefix")
                .map(|v| crate::scan::unquote_xmlport(&v)),
            auto_calc: layout.property(i, "AutoCalcField").and_then(|v| bool_prop(&v)),
            unbound: kind == ir::XmlNodeKind::Field
                && layout.property(i, "Unbound").and_then(|v| bool_prop(&v)) == Some(true),
            required: matches!(kind, ir::XmlNodeKind::TextAttr | ir::XmlNodeKind::FieldAttr)
                .then(|| layout.property(i, "Occurrence"))
                .flatten()
                .map(|v| v.trim().eq_ignore_ascii_case("required")),
            field_validate: layout.property(i, "FieldValidate").and_then(|v| bool_prop(&v)),
            triggers,
        };
        match kind {
            ir::XmlNodeKind::Text | ir::XmlNodeKind::TextAttr => {
                node.slot = cx.slots.get(&raw.name.to_lowercase()).copied();
                if node.slot.is_none() {
                    reasons.push(format!("variable of {} has no slot", raw.name));
                }
                // No corpus test pins a trigger on a container text element (one with child
                // elements; Base App's xmlport 1 has OnAfterAssignVariable on one), so running
                // the port is refused, not declaring it.
                if kind == ir::XmlNodeKind::Text && layout.has_children(i) && !node.triggers.is_empty() {
                    run_reasons.push(format!(
                        "triggers of the container text element {} are not supported (not pinned)",
                        raw.name
                    ));
                }
            }
            ir::XmlNodeKind::Table => {
                node.slot = cx.slots.get(&raw.name.to_lowercase()).copied();
                node.table = raw.args.get(1).and_then(|t| table_object(cx, t));
                if node.slot.is_none() || node.table.is_none() {
                    reasons.push(format!(
                        "table element {}: table {} is outside the workspace",
                        raw.name,
                        raw.args.get(1).cloned().unwrap_or_default()
                    ));
                }
                // UseTemporary with AutoSave = false: the triggers run as for a saved row and see the imported values, the temporary
                // table holds only what a trigger inserts, the database is unchanged (xmlport/lt-aj-xmlport/AjRunAndXmlPort.al:519-600,
                // QS_*_AutoSaveFalse*, and :447 XmlPort_UseTemporary_AutoSaveFalse_Import). Pinned for a table element right below the
                // root, Xml and VariableText, without AutoUpdate or AutoReplace.
                let below_root = raw
                    .parent
                    .is_some_and(|p| layout.nodes[p].kind == ir::XmlNodeKind::Text && layout.nodes[p].parent.is_none());
                if !node.auto_save
                    && node.use_temporary
                    && (!below_root
                        || node.auto_update
                        || node.auto_replace
                        || layout.format == ir::XmlFormat::FixedText)
                {
                    reasons.push(format!(
                        "table element {}: UseTemporary with AutoSave = false",
                        raw.name
                    ));
                }
                let link_table = layout
                    .property(i, "LinkTable")
                    .map(|v| crate::scan::unquote_xmlport(&v));
                let link_fields = layout.property(i, "LinkFields");
                // LinkTable without LinkFields links nothing: an export shows every row under every parent, an import saves the parent
                // before the elements below it (xmlport/lt-xmlport-linktable/XmlPortLinkTableNoLinkFields.al: BothTemporary_Export,
                // PersistentChild_Export, BothTemporary_Import_*, PersistentChild_Import_*, PersistentBoth_Import_*). LinkFields without
                // LinkTable stays refused.
                let link_fields = link_fields.or_else(|| link_table.as_ref().map(|_| String::new()));
                match (link_table, link_fields) {
                    (None, None) => {}
                    (Some(t), Some(f)) => match layout.table_named(raw.parent, &t) {
                        Some(ln) => {
                            node.link_node = Some(ln);
                            for pair in crate::scan::split_double(&f, ',')
                                .into_iter()
                                .filter(|p| !p.trim().is_empty())
                            {
                                let parts = crate::scan::split_double(&pair, '=');
                                let own = parts.first().map(|p| crate::scan::unquote_xmlport(p));
                                let other = parts.get(1).map(|p| p.trim()).and_then(|p| {
                                    let low = p.to_ascii_lowercase();
                                    (low.starts_with("field(") && p.ends_with(')'))
                                        .then(|| crate::scan::unquote_xmlport(&p[6..p.len() - 1]))
                                });
                                let own_pos = own.as_ref().and_then(|n| {
                                    node.table.and_then(|t| {
                                        cx.members[t].fields.iter().position(|x| x.name.eq_ignore_ascii_case(n))
                                    })
                                });
                                let other_pos = other.as_ref().and_then(|n| {
                                    nodes.get(ln).and_then(|l| l.table).and_then(|t| {
                                        cx.members[t].fields.iter().position(|x| x.name.eq_ignore_ascii_case(n))
                                    })
                                });
                                match (own_pos, other_pos) {
                                    (Some(a), Some(b)) => node.link_fields.push((a, b)),
                                    _ => reasons.push(format!(
                                        "table element {}: LinkFields entry `{}` is not understood",
                                        raw.name,
                                        pair.trim()
                                    )),
                                }
                            }
                        }
                        None => reasons.push(format!(
                            "table element {}: LinkTable {t} is not a table element above it",
                            raw.name
                        )),
                    },
                    _ => reasons.push(format!(
                        "table element {}: LinkTable and LinkFields must be given together",
                        raw.name
                    )),
                }
            }
            ir::XmlNodeKind::Field | ir::XmlNodeKind::FieldAttr => {
                let source = raw.args.get(1).cloned().unwrap_or_default();
                let (table_name, field_name) = match split_source_at_dot(&source) {
                    Some((t, f)) => (crate::scan::unquote_xmlport(t), crate::scan::unquote_xmlport(f)),
                    None => (String::new(), crate::scan::unquote_xmlport(&source)),
                };
                match layout.table_named(raw.parent, &table_name) {
                    Some(tn) => {
                        node.table_node = Some(tn);
                        let pos = nodes[tn].table.and_then(|t| {
                            cx.members[t]
                                .fields
                                .iter()
                                .position(|x| x.name.eq_ignore_ascii_case(&field_name))
                        });
                        match pos {
                            Some(p) => {
                                let t = nodes[tn].table.expect("a position implies a table");
                                // A FlowField exports its calculated value (assumption xmlport-exports-flowfield-calculated); a
                                // FlowFilter has no value to write.
                                if !matches!(
                                    cx.members[t].fields[p].class,
                                    FieldClass::Normal | FieldClass::FlowField
                                ) {
                                    reasons.push(format!("{} is bound to a FlowFilter", raw.name));
                                }
                                node.field = Some(p);
                            }
                            None => reasons.push(format!(
                                "{} is bound to unknown field {field_name} of {table_name}",
                                raw.name
                            )),
                        }
                    }
                    None => reasons.push(format!("{} is not inside a table element named {table_name}", raw.name)),
                }
            }
        }
        nodes.push(node);
    }
    reasons.extend(layout.reasons.iter().cloned());
    // Pinned: one declared namespace that a field element of the port uses. A prefix that is not declared, a namespace nobody
    // uses and a prefix on an attribute alone are not (YXmlPortProps.al:NodeProperties_Export_NamesPrefixesAutoCalc).
    {
        let used: Vec<&str> = nodes.iter().filter_map(|n| n.ns_prefix.as_deref()).collect();
        for (prefix, _) in &layout.namespaces {
            if !nodes
                .iter()
                .any(|n| n.kind == ir::XmlNodeKind::Field && n.ns_prefix.as_deref() == Some(prefix.as_str()))
            {
                run_reasons.push(format!(
                    "xmlport namespace `{prefix}` that no field element uses (not pinned)"
                ));
            }
        }
        for u in used {
            if !layout.namespaces.iter().any(|(p, _)| p == u) {
                run_reasons.push(format!(
                    "NamespacePrefix `{u}` is not a declared namespace (not pinned)"
                ));
            }
        }
    }
    ir::XmlPort {
        direction: layout.direction,
        format: layout.format,
        field_separator: layout.field_separator.clone(),
        field_delimiter: layout.field_delimiter.clone(),
        record_separator: layout.record_separator.clone(),
        table_separator: layout.table_separator.clone(),
        text_encoding_declared: layout.text_encoding_declared,
        utf8: layout.utf8,
        xml_utf8: layout.xml_utf8,
        namespaces: layout.namespaces.clone(),
        inline_schema: layout.inline_schema,
        use_lax: layout.use_lax,
        xml_v11: layout.xml_v11,
        format_evaluate_xml: layout.format_evaluate_xml,
        format_evaluate_declared: layout.format_evaluate_declared.is_some(),
        no_default_fields_validation: layout.no_default_fields_validation,
        nodes,
        source_slot: cx.source_slot,
        destination_slot: cx.destination_slot,
        unsupported: reasons,
        run_unsupported: run_reasons,
        export_unsupported: layout.export_reasons.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout_of(src: &str) -> Layout {
        let parse = al_syntax::parse(src, &Default::default());
        let root = ast::SourceFile::cast(parse.syntax()).unwrap();
        layout(&root.objects().next().unwrap())
    }

    #[test]
    fn the_schema_is_walked_in_source_order_with_parents() {
        let l = layout_of(
            "xmlport 1 X { Direction = Export; schema { textelement(Root) { XmlName = 'Docs'; tableelement(T; Cust) { fieldelement(A; T.X) { } fieldattribute(B; T.Y) { } } } } }",
        );
        assert!(l.reasons.is_empty(), "{:?}", l.reasons);
        assert_eq!(l.direction, ir::XmlDirection::Export);
        let names: Vec<_> = l.nodes.iter().map(|n| (n.xml_name.as_str(), n.parent)).collect();
        assert_eq!(
            names,
            vec![("Docs", None), ("T", Some(0)), ("A", Some(1)), ("B", Some(1))]
        );
    }

    // platform/lt-ag-contoso/AgXmlPortImport.al:94-96 (xmlport 78422, RequestPage_EmptySection_ImportAndExport at
    // :312-327): a `requestpage { }` with nothing in it changes nothing for Xmlport.Import and Export. The same holds
    // with empty `layout` and `actions` sections, both or one alone (xmlports 87564-87566 at :835-912,
    // QX_RequestPage_EmptyLayoutAndActions_ImportAndExport at :476). One with a control, an action, a property or a
    // trigger is not pinned.
    #[test]
    fn an_empty_request_page_runs_and_one_with_content_is_refused() {
        let schema = "schema { textelement(R) { } }";
        for sub in [
            "requestpage { }",
            "requestpage { layout { } actions { } }",
            "requestpage { layout { } }",
            "requestpage { actions { } }",
        ] {
            let l = layout_of(&format!("xmlport 1 X {{ {schema} {sub} }}"));
            assert!(l.run_reasons.is_empty(), "{sub}: {:?}", l.run_reasons);
        }
        for full in [
            "requestpage { layout { area(content) { field(F; F) { } } } }",
            "requestpage { actions { area(processing) { action(A) { } } } }",
            "requestpage { SaveValues = true; }",
            "requestpage { trigger OnOpenPage() begin end; }",
        ] {
            let l = layout_of(&format!("xmlport 1 X {{ {schema} {full} }}"));
            assert!(
                l.run_reasons.iter().any(|r| r.contains("request pages")),
                "{full}: {:?}",
                l.run_reasons
            );
        }
    }

    // xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al:67-122 and xmlport/W7XmlPortTests.al:111-149 pin these values;
    // a nested table element's MinOccurs (XmlPortTriggerFlow.al:93) and a container's MaxOccurs = Unbounded are not pinned.
    #[test]
    fn only_the_pinned_occurs_values_run() {
        let ok = layout_of(
            "xmlport 1 X { schema { textelement(R) { MaxOccurs = Once; textelement(Q) { MinOccurs = Once; MaxOccurs = Once; tableelement(H; Cust) { MinOccurs = Zero; MaxOccurs = Unbounded; fieldelement(N; H.X) { MinOccurs = Zero; MaxOccurs = Once; } } } textelement(S) { MinOccurs = Zero; MaxOccurs = Unbounded; } } } }",
        );
        assert!(ok.run_reasons.is_empty(), "{:?}", ok.run_reasons);
        for (src, want) in [
            (
                "textelement(R) { textelement(Q) { MaxOccurs = Unbounded; textelement(L) { } } }",
                "maxoccurs = unbounded of container textelement Q",
            ),
            (
                "textelement(R) { MinOccurs = Zero; textelement(L) { } }",
                "minoccurs = zero of container textelement R",
            ),
            (
                "textelement(R) { tableelement(H; Cust) { MinOccurs = Once; } }",
                "minoccurs = once of tableelement H",
            ),
            (
                "textelement(R) { tableelement(H; Cust) { tableelement(C; Cust) { MinOccurs = Zero; } } }",
                "minoccurs = zero of tableelement C",
            ),
            (
                "textelement(R) { tableelement(H; Cust) { fieldelement(N; H.X) { MaxOccurs = Unbounded; } } }",
                "maxoccurs = unbounded of fieldelement N",
            ),
        ] {
            let l = layout_of(&format!("xmlport 1 X {{ schema {{ {src} }} }}"));
            let all = l.run_reasons.join("; ");
            assert!(all.contains(want), "{src}: {all}");
        }
    }

    #[test]
    fn what_the_corpus_does_not_pin_becomes_a_reason() {
        let l = layout_of(
            "xmlport 1 X { Format = Binary; schema { textelement(Root) { MinOccurs = Zero; trigger OnPreXmlItem() begin end; } } requestpage { SaveValues = true; } }",
        );
        let all = [l.reasons.clone(), l.run_reasons.clone()].concat().join("; ");
        assert!(all.contains("Format = binary"), "{all}");
        assert!(all.contains("property minoccurs"), "{all}");
        assert!(all.contains("trigger onprexmlitem"), "{all}");
        assert!(all.contains("request pages"), "{all}");
        assert!(
            !l.reasons.join(";").contains("request pages"),
            "a request page is a run reason"
        );
    }

    // Mutation-testing additions (docs/status.md debt).

    #[test]
    fn only_utf8_is_an_accepted_text_encoding() {
        for ok in ["UTF8", "utf8", "UTF8 "] {
            let l = layout_of(&format!(
                "xmlport 1 X {{ TextEncoding = {ok}; schema {{ textelement(Root) {{ }} }} }}"
            ));
            assert!(l.reasons.is_empty(), "{ok}: {:?}", l.reasons);
        }
        let l = layout_of("xmlport 1 X { TextEncoding = UTF16; schema { textelement(Root) { } } }");
        assert!(
            l.run_reasons.join(";").contains("property textencoding"),
            "{:?}",
            l.run_reasons
        );
    }

    #[test]
    fn inert_object_properties_are_accepted_and_others_are_reasons() {
        for name in INERT_OBJECT_PROPERTIES {
            let l = layout_of(&format!(
                "xmlport 1 X {{ {name} = 'x'; schema {{ textelement(Root) {{ }} }} }}"
            ));
            assert!(l.reasons.is_empty(), "{name}: {:?}", l.reasons);
        }
        let l = layout_of("xmlport 1 X { Encoding = Windows; schema { textelement(Root) { } } }");
        assert!(
            l.run_reasons.join(";").contains("property encoding"),
            "{:?}",
            l.run_reasons
        );
    }

    #[test]
    fn a_quote_hides_parentheses_from_the_nesting_count() {
        assert_eq!(
            crate::scan::split_double("\"A(B\" = FIELD(X), C = FIELD(Y)", ','),
            vec!["\"A(B\" = FIELD(X)", " C = FIELD(Y)"]
        );
        assert_eq!(
            crate::scan::split_double("\"A)B\" = FIELD(X), C = FIELD(Y)", ','),
            vec!["\"A)B\" = FIELD(X)", " C = FIELD(Y)"]
        );
        assert_eq!(
            crate::scan::split_double("A = FIELD(X, Y), B", ','),
            vec!["A = FIELD(X, Y)", " B"]
        );
    }

    #[test]
    fn link_fields_split_at_top_level_commas() {
        assert_eq!(
            crate::scan::split_double("\"A,B\" = FIELD(X), C = FIELD(Y)", ','),
            vec!["\"A,B\" = FIELD(X)", " C = FIELD(Y)"]
        );
    }
}

#[cfg(test)]
mod lowering_tests {
    use crate::lower::compile;
    use crate::workspace::AppSource;
    use al_ir as ir;

    const SRC: &str = r#"
table 50001 Parent { fields { field(1; No; Integer) { } field(2; Name; Text[30]) { } } keys { key(PK; No) { } } }
table 50002 Child { fields { field(1; No; Integer) { } field(2; ParentNo; Integer) { } } keys { key(PK; No) { } } }
xmlport 50010 X
{
    Direction = Export; Format = Xml; UseRequestPage = false;
    schema
    {
        textelement(Root)
        {
            XmlName = 'Docs';
            tableelement(P; Parent)
            {
                XmlName = 'Doc';
                textattribute(Note) { trigger OnBeforePassVariable() begin Note := 'n'; end; }
                fieldelement(Number; P.No) { }
                tableelement(C; Child)
                {
                    LinkTable = P;
                    LinkFields = ParentNo = FIELD(No);
                    fieldattribute(Id; C.No) { }
                }
            }
        }
    }
    var Counter: Integer;
    procedure Bump() begin Counter += 1; end;
}
"#;

    fn lowered() -> ir::Object {
        let (ws, l) = compile(
            vec![AppSource {
                id: "a".into(),
                name: "a".into(),
                dependencies: vec![],
                files: vec![("f.al".into(), SRC.into())],
            }],
            &Default::default(),
        );
        assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
        l.program
            .objects
            .iter()
            .find(|o| o.kind == al_ir::ObjKind::XmlPort)
            .cloned()
            .unwrap()
    }

    // The element variables take the first global slots, then the two hidden streams, then the
    // declared variables; the nodes point at those slots, at fields by position and at links.
    #[test]
    fn schema_nodes_point_at_slots_fields_and_links() {
        let o = lowered();
        let names: Vec<_> = o.globals.iter().map(|g| g.name.as_str()).collect();
        assert_eq!(
            names,
            ["Root", "P", "Note", "C", "__source", "__destination", "Counter"]
        );
        let xp = o.xmlport.as_ref().unwrap();
        assert!(xp.unsupported.is_empty(), "{:?}", xp.unsupported);
        assert_eq!((xp.source_slot, xp.destination_slot), (4, 5));
        let kinds: Vec<_> = xp
            .nodes
            .iter()
            .map(|n| (n.xml_name.as_str(), n.kind, n.parent))
            .collect();
        use ir::XmlNodeKind::*;
        assert_eq!(
            kinds,
            vec![
                ("Docs", Text, None),
                ("Doc", Table, Some(0)),
                ("Note", TextAttr, Some(1)),
                ("Number", Field, Some(1)),
                ("C", Table, Some(1)),
                ("Id", FieldAttr, Some(4)),
            ]
        );
        let (p, field, child) = (&xp.nodes[1], &xp.nodes[3], &xp.nodes[4]);
        assert_eq!((p.slot, field.table_node, field.field), (Some(1), Some(1), Some(0)));
        assert_eq!((child.link_node, child.link_fields.clone()), (Some(1), vec![(1, 0)]));
        assert!(child.auto_save && !child.auto_update && !child.auto_replace && !child.use_temporary);
        assert!(xp.nodes[2].trigger("onbeforepassvariable").is_some());
        assert_eq!(xp.direction, ir::XmlDirection::Export);
    }

    #[test]
    fn an_unknown_field_or_table_is_a_reason() {
        let bad = SRC.replace("fieldelement(Number; P.No)", "fieldelement(Number; P.Nope)");
        let (_, l) = compile(
            vec![AppSource {
                id: "a".into(),
                name: "a".into(),
                dependencies: vec![],
                files: vec![("f.al".into(), bad)],
            }],
            &Default::default(),
        );
        let o = l
            .program
            .objects
            .iter()
            .find(|o| o.kind == al_ir::ObjKind::XmlPort)
            .unwrap();
        let why = o.xmlport.as_ref().unwrap().unsupported.join(";");
        assert!(why.contains("unknown field Nope"), "{why}");
    }
    // Mutation-testing additions (docs/status.md debt).

    fn reasons_of(src: String) -> String {
        let (_, l) = compile(
            vec![AppSource {
                id: "a".into(),
                name: "a".into(),
                dependencies: vec![],
                files: vec![("f.al".into(), src)],
            }],
            &Default::default(),
        );
        let o = l
            .program
            .objects
            .iter()
            .find(|o| o.kind == al_ir::ObjKind::XmlPort)
            .unwrap();
        o.xmlport.as_ref().unwrap().unsupported.join(";")
    }

    fn run_reasons_of(src: String) -> String {
        let (_, l) = compile(
            vec![AppSource {
                id: "a".into(),
                name: "a".into(),
                dependencies: vec![],
                files: vec![("f.al".into(), src)],
            }],
            &Default::default(),
        );
        let o = l
            .program
            .objects
            .iter()
            .find(|o| o.kind == al_ir::ObjKind::XmlPort)
            .unwrap();
        o.xmlport.as_ref().unwrap().run_unsupported.join(";")
    }

    #[test]
    fn a_field_must_sit_inside_a_table_element_of_the_name_it_binds() {
        // Root is a text element, Q is no element at all, and P is a table element of another name.
        for name in ["Root", "Q"] {
            let why = reasons_of(SRC.replace(
                "fieldelement(Number; P.No)",
                &format!("fieldelement(Number; {name}.No)"),
            ));
            assert!(
                why.contains(&format!("is not inside a table element named {name}")),
                "{name}: {why}"
            );
        }
        assert_eq!(reasons_of(SRC.to_string()), "");
    }

    #[test]
    fn a_quoted_table_name_with_a_dot_binds_its_field() {
        // Base Application's xmlport 1225 binds `"Data Exch. Def".Code`: the dot inside the quotes is part of the name.
        let src = SRC
            .replace("tableelement(P; Parent)", "tableelement(\"P. Q\"; Parent)")
            .replace("fieldelement(Number; P.No)", "fieldelement(Number; \"P. Q\".No)")
            .replace("LinkTable = P;", "LinkTable = \"P. Q\";");
        assert_eq!(reasons_of(src), "");
    }

    #[test]
    fn a_field_element_with_pinned_child_nodes_runs_and_others_refuse_the_run() {
        // AfXmlPortAttrChildren.al pins attributes and a leaf text or field element below a field element.
        let why = run_reasons_of(SRC.replace(
            "fieldelement(Number; P.No) { }",
            "fieldelement(Number; P.No) { textattribute(Utc) { } }",
        ));
        assert_eq!(why, "");
        let why = run_reasons_of(SRC.replace(
            "fieldelement(Number; P.No) { }",
            "fieldelement(Number; P.No) { textelement(Utc) { textelement(Deeper) { } } }",
        ));
        assert!(
            why.contains("child node textelement of the field element Number"),
            "{why}"
        );
    }

    #[test]
    fn link_table_must_name_a_table_element_above() {
        for name in ["Root", "Q", "C"] {
            let why = reasons_of(SRC.replace("LinkTable = P;", &format!("LinkTable = {name};")));
            assert!(
                why.contains(&format!("LinkTable {name} is not a table element above it")),
                "{name}: {why}"
            );
        }
        // LinkFields without LinkTable stays refused; LinkTable alone links nothing (XmlPortLinkTableNoLinkFields.al).
        let why = reasons_of(SRC.replace("LinkTable = P;", ""));
        assert!(why.contains("LinkTable and LinkFields must be given together"), "{why}");
        assert_eq!(reasons_of(SRC.replace("LinkFields = ParentNo = FIELD(No);", "")), "");
    }

    #[test]
    fn a_container_text_element_with_a_trigger_is_not_a_reason_the_run_refuses_it() {
        // The runtime refuses it when the port runs (al-vm/tests/xmlports.rs), so code that only
        // holds the port stays runnable.
        let why = reasons_of(SRC.replace(
            "XmlName = 'Docs';",
            "XmlName = 'Docs'; trigger OnBeforePassVariable() begin end;",
        ));
        assert!(!why.contains("container text element"), "{why}");
        let run = run_reasons_of(SRC.replace(
            "XmlName = 'Docs';",
            "XmlName = 'Docs'; trigger OnBeforePassVariable() begin end;",
        ));
        assert!(run.contains("triggers of the container text element Root"), "{run}");
    }

    #[test]
    fn format_evaluate_xml_and_legacy_are_understood_and_others_are_run_reasons() {
        // xmlport/TestXmlPortFormatEvaluateContracts.al
        for v in ["Xml", "Legacy"] {
            let src = SRC.replace(
                "Direction = Export;",
                &format!("FormatEvaluate = {v}; Direction = Export;"),
            );
            assert!(!reasons_of(src.clone()).contains("formatevaluate"));
            assert!(!run_reasons_of(src).contains("formatevaluate"), "{v}");
        }
        let src = SRC.replace("Direction = Export;", "FormatEvaluate = Other; Direction = Export;");
        assert!(run_reasons_of(src).contains("formatevaluate = other"));
    }

    #[test]
    fn a_table_outside_the_workspace_is_a_reason_even_with_its_variable_in_place() {
        let why = reasons_of(SRC.replace("tableelement(P; Parent)", "tableelement(P; Nowhere)"));
        assert!(
            why.contains("table element P: table Nowhere is outside the workspace"),
            "{why}"
        );
    }

    #[test]
    fn a_link_field_that_is_not_field_of_something_is_not_understood() {
        let why = reasons_of(SRC.replace("LinkFields = ParentNo = FIELD(No);", "LinkFields = ParentNo = No;"));
        assert!(
            why.contains("LinkFields entry `ParentNo = No` is not understood"),
            "{why}"
        );
        // Only a FIELD( prefix makes the right-hand side a field reference; a name that merely
        // ends in a parenthesis and has a real field after six characters is not one.
        let why = reasons_of(SRC.replace(
            "LinkFields = ParentNo = FIELD(No);",
            "LinkFields = ParentNo = ABCDEFNo);",
        ));
        assert!(why.contains("is not understood"), "{why}");
        // The same for a FIELD( that is never closed: its sixth character on is not a field name.
        let why = reasons_of(SRC.replace(
            "LinkFields = ParentNo = FIELD(No);",
            "LinkFields = ParentNo = FIELD(NoX;",
        ));
        assert!(why.contains("is not understood"), "{why}");
    }
}
