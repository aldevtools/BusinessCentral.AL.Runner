//! A library cell (al-builtins `LibraryValue`) in the arena, variant by variant. A state of plain owned data is kept
//! as it is ([`OwnedLibrary`]); a record handle is a node. The image defers the nodes of JSON and XML trees
//! (their parents are `Weak`), the Http types (shared header lists), FilterPageBuilder and a data transfer's plan, each
//! with its own reason. What lies outside the session (a client of the network, a file, an open stream, a SecretText's
//! text) is never imaged.

use super::NotImaged;
use super::arena::{Frozen, NodeId};
use super::freeze::{Holder, Walker, deferred, never};
use super::thaw::Maker;
use al_builtins::library_kind::LibraryKind;
use al_builtins::text::Txt;
use al_builtins::value::{
    CookieState, EmptiedRef, ErrorInfoBuilt, ErrorInfoState, HandlerContext, LibraryValue, ModuleInfoState,
    NotificationState, SessionSettingsState, WebServiceActionState,
};

/// What a library cell holds.
#[derive(Debug)]
pub(crate) enum LibraryNode {
    Empty,
    ClearedStream,
    XmlNodeOfUnassignedElement,
    ClosedRecordRef,
    HandedContext,
    XmlNameTable,
    /// A TextBuilder's or BigText's text as the cell holds it: a `Txt` keeps a lone surrogate half, a `String` would
    /// hold U+FFFD in its place (issue 9202).
    Text(Txt),
    TextCap(Txt, i32),
    SystemTable(i64),
    Version([i32; 4]),
    RecordId {
        table: u32,
        table_no: u32,
        name: String,
        key: Vec<Frozen>,
    },
    RecordRef(NodeId),
    FieldRef(NodeId, usize),
    KeyFieldRef(NodeId, usize),
    KeyRef(NodeId, usize),
    UserNameFieldRef {
        rec: NodeId,
        no: u32,
        value: Option<String>,
    },
    ErrorInfo(Box<ErrorInfoNode>),
    Owned(OwnedLibrary),
}

/// The library states that are plain owned data, kept as they are.
#[derive(Debug)]
pub(crate) enum OwnedLibrary {
    EmptiedRef(EmptiedRef),
    HandlerContext(Box<HandlerContext>),
    WebServiceAction(WebServiceActionState),
    SessionSettings(SessionSettingsState),
    Notification(NotificationState),
    ModuleInfo(ModuleInfoState),
    DataTransfer(Option<(i32, i32)>),
    Cookie(CookieState),
    XmlReadOptions {
        preserve_whitespace: bool,
    },
    XmlWriteOptions {
        preserve_whitespace: bool,
        unindented: bool,
    },
}

/// `ErrorInfoState` with its three values frozen.
#[derive(Debug)]
pub(crate) struct ErrorInfoNode {
    message: String,
    title: String,
    detailed_message: String,
    control_name: String,
    verbosity: i32,
    data_classification: i32,
    error_type: Option<i32>,
    collectible: bool,
    table_id: i32,
    record_id: Option<Frozen>,
    system_id: Option<Frozen>,
    field_no: i32,
    page_no: i32,
    custom_dimensions: Option<Frozen>,
    built: ErrorInfoBuilt,
    /// The `Callstack` a collected error carries, kept by the cell in a thread-local side table (al-builtins
    /// errorinfo.rs `CALLSTACKS`), not in the state.
    pub(crate) callstack: Option<String>,
}

impl Walker<'_> {
    /// A library cell's content. Every variant is named, so a new one is a compile error here and has to get a rule.
    pub(crate) fn library(
        &mut self,
        kind: &LibraryKind,
        cell: &std::rc::Rc<std::cell::RefCell<LibraryValue>>,
        lv: &LibraryValue,
        path: &str,
    ) -> Result<LibraryNode, NotImaged> {
        use LibraryNode as N;
        use LibraryValue as L;
        let rec = |w: &mut Self, r: &std::rc::Rc<std::cell::RefCell<al_builtins::value::RecordState>>| {
            w.handle(Holder::Record(r.clone()), 1, format!("{path} record"))
        };
        Ok(match lv {
            L::Empty => N::Empty,
            L::ClearedStream => N::ClearedStream,
            L::XmlNodeOfUnassignedElement => N::XmlNodeOfUnassignedElement,
            L::ClosedRecordRef => N::ClosedRecordRef,
            L::HandedContext => N::HandedContext,
            L::XmlNameTable => N::XmlNameTable,
            L::Text(_) if *kind == LibraryKind::SecretText => return Err(never(path, "SecretText")),
            L::Text(t) => N::Text(t.clone()),
            L::TextCap(t, n) => N::TextCap(t.clone(), *n),
            L::SystemTable(n) => N::SystemTable(*n),
            L::Version(v) => N::Version(*v),
            L::RecordId {
                table,
                table_no,
                name,
                key,
            } => N::RecordId {
                table: *table,
                table_no: *table_no,
                name: name.clone(),
                key: self.field_values(key, &format!("{path} key"))?,
            },
            L::RecordRef(r) => N::RecordRef(rec(self, r)),
            L::FieldRef(r, p) => N::FieldRef(rec(self, r), *p),
            L::KeyFieldRef(r, p) => N::KeyFieldRef(rec(self, r), *p),
            L::KeyRef(r, k) => N::KeyRef(rec(self, r), *k),
            L::UserNameFieldRef { rec: r, no, value } => N::UserNameFieldRef {
                rec: rec(self, r),
                no: *no,
                value: value.clone(),
            },
            L::ErrorInfo(e) => {
                let mut node = self.error_info(e, path)?;
                node.callstack = al_builtins::errorinfo::callstack_of(cell).map(|s| s.to_string());
                N::ErrorInfo(Box::new(node))
            }
            L::EmptiedRef(e) => N::Owned(OwnedLibrary::EmptiedRef(*e)),
            L::HandlerContext(c) => N::Owned(OwnedLibrary::HandlerContext(c.clone())),
            L::WebServiceAction(s) => N::Owned(OwnedLibrary::WebServiceAction(s.clone())),
            L::SessionSettings(s) => N::Owned(OwnedLibrary::SessionSettings(s.clone())),
            L::Notification(s) => N::Owned(OwnedLibrary::Notification(s.clone())),
            L::ModuleInfo(s) => N::Owned(OwnedLibrary::ModuleInfo(s.clone())),
            L::DataTransfer(s) => N::Owned(OwnedLibrary::DataTransfer(*s)),
            L::Cookie(s) => N::Owned(OwnedLibrary::Cookie(s.clone())),
            L::XmlReadOptions { preserve_whitespace } => N::Owned(OwnedLibrary::XmlReadOptions {
                preserve_whitespace: *preserve_whitespace,
            }),
            L::XmlWriteOptions {
                preserve_whitespace,
                unindented,
            } => N::Owned(OwnedLibrary::XmlWriteOptions {
                preserve_whitespace: *preserve_whitespace,
                unindented: *unindented,
            }),
            L::Json(_) | L::Xml(_) | L::XmlList(_) | L::XmlAttrs(_) | L::XmlNamespaces(_) => {
                return Err(deferred(path, &kind.to_string()));
            }
            L::HttpRequest(_)
            | L::HttpContent(_)
            | L::HttpHeaders(_)
            | L::HttpResponse(_)
            | L::TestHttpRequest(_)
            | L::TestHttpResponse(_) => return Err(deferred(path, &kind.to_string())),
            L::FilterPageBuilder(_) => return Err(deferred(path, "FilterPageBuilder")),
            L::DataTransferPlan(_) => return Err(deferred(path, "DataTransfer plan")),
            L::HttpClient(_) => return Err(never(path, "HttpClient")),
            L::File(_) => return Err(never(path, "File")),
            L::InStream(_) => return Err(never(path, "InStream")),
            L::OutStream(..) => return Err(never(path, "OutStream")),
        })
    }

    fn error_info(&mut self, e: &ErrorInfoState, path: &str) -> Result<ErrorInfoNode, NotImaged> {
        let ErrorInfoState {
            message,
            title,
            detailed_message,
            control_name,
            verbosity,
            data_classification,
            error_type,
            collectible,
            table_id,
            record_id,
            system_id,
            field_no,
            page_no,
            custom_dimensions,
            built,
        } = e;
        let opt = |w: &mut Self, v: &Option<al_builtins::value::Value>, what: &str| {
            v.as_ref().map(|v| w.value(v, &format!("{path} {what}"))).transpose()
        };
        Ok(ErrorInfoNode {
            message: message.clone(),
            title: title.clone(),
            detailed_message: detailed_message.clone(),
            control_name: control_name.clone(),
            verbosity: *verbosity,
            data_classification: *data_classification,
            error_type: *error_type,
            collectible: *collectible,
            table_id: *table_id,
            record_id: opt(self, record_id, "record id")?,
            system_id: opt(self, system_id, "system id")?,
            field_no: *field_no,
            page_no: *page_no,
            custom_dimensions: opt(self, custom_dimensions, "custom dimensions")?,
            built: *built,
            callstack: None,
        })
    }
}

impl Maker<'_> {
    /// The library value of `n`, every handle it names made already.
    pub(crate) fn library(&self, n: &LibraryNode) -> LibraryValue {
        use LibraryNode as N;
        use LibraryValue as L;
        match n {
            N::Empty => L::Empty,
            N::ClearedStream => L::ClearedStream,
            N::XmlNodeOfUnassignedElement => L::XmlNodeOfUnassignedElement,
            N::ClosedRecordRef => L::ClosedRecordRef,
            N::HandedContext => L::HandedContext,
            N::XmlNameTable => L::XmlNameTable,
            N::Text(t) => L::Text(t.clone()),
            N::TextCap(t, n) => L::TextCap(t.clone(), *n),
            N::SystemTable(n) => L::SystemTable(*n),
            N::Version(v) => L::Version(*v),
            N::RecordId {
                table,
                table_no,
                name,
                key,
            } => L::RecordId {
                table: *table,
                table_no: *table_no,
                name: name.clone(),
                key: self.values(key),
            },
            N::RecordRef(r) => L::RecordRef(self.record_handle(*r)),
            N::FieldRef(r, p) => L::FieldRef(self.record_handle(*r), *p),
            N::KeyFieldRef(r, p) => L::KeyFieldRef(self.record_handle(*r), *p),
            N::KeyRef(r, k) => L::KeyRef(self.record_handle(*r), *k),
            N::UserNameFieldRef { rec, no, value } => L::UserNameFieldRef {
                rec: self.record_handle(*rec),
                no: *no,
                value: value.clone(),
            },
            N::ErrorInfo(e) => L::ErrorInfo(self.error_info(e)),
            N::Owned(o) => match o {
                OwnedLibrary::EmptiedRef(e) => L::EmptiedRef(*e),
                OwnedLibrary::HandlerContext(c) => L::HandlerContext(c.clone()),
                OwnedLibrary::WebServiceAction(s) => L::WebServiceAction(s.clone()),
                OwnedLibrary::SessionSettings(s) => L::SessionSettings(s.clone()),
                OwnedLibrary::Notification(s) => L::Notification(s.clone()),
                OwnedLibrary::ModuleInfo(s) => L::ModuleInfo(s.clone()),
                OwnedLibrary::DataTransfer(s) => L::DataTransfer(*s),
                OwnedLibrary::Cookie(s) => L::Cookie(s.clone()),
                OwnedLibrary::XmlReadOptions { preserve_whitespace } => L::XmlReadOptions {
                    preserve_whitespace: *preserve_whitespace,
                },
                OwnedLibrary::XmlWriteOptions {
                    preserve_whitespace,
                    unindented,
                } => L::XmlWriteOptions {
                    preserve_whitespace: *preserve_whitespace,
                    unindented: *unindented,
                },
            },
        }
    }

    fn error_info(&self, e: &ErrorInfoNode) -> ErrorInfoState {
        let opt = |v: &Option<Frozen>| v.as_ref().map(|v| self.value(v));
        ErrorInfoState {
            message: e.message.clone(),
            title: e.title.clone(),
            detailed_message: e.detailed_message.clone(),
            control_name: e.control_name.clone(),
            verbosity: e.verbosity,
            data_classification: e.data_classification,
            error_type: e.error_type,
            collectible: e.collectible,
            table_id: e.table_id,
            record_id: opt(&e.record_id),
            system_id: opt(&e.system_id),
            field_no: e.field_no,
            page_no: e.page_no,
            custom_dimensions: opt(&e.custom_dimensions),
            built: e.built,
        }
    }
}
