//! The values of SingleInstance globals as text that compares across sessions ([`Session::state_lines`], al-testrun
//! `login_replay.rs`, al-runner issue 8211). Every kind of value has a rule here: an exact form, or a reason why it has
//! none. A value with no form gives `Err` with why and its path (ids, indexes and type names only, never a value);
//! [`Session::state_lines`] then names it as opaque, never as equal.
//!
//! Exact means: two equal texts hold the same. No element is sampled, cut or hashed; runs of equal elements are written
//! once. What two places share is kept: a codeunit instance has a number by first sight; any other handle (a record, a
//! library value, a List, a Dictionary, a query, a JSON or XML node, a header list) held by more than one owner
//! (`Rc::strong_count` > 1) has a number by first sight too, and its content is written once as `shared <n> <content>`. A
//! handle with one owner cannot be shared, so it is written in place.
//!
//! the one owner of how a value reads in `Session::state_lines` (`Render`): every `Value` variant and
//! every `LibraryValue` variant has a rule (a match with no wildcard, so a new variant has to get
//! one): an exact form, or `Err` with why it has none and its path (ids, indexes and type names
//! only). Exact forms: a codeunit instance by a number by first sight, its globals once; an array and
//! a List element by element with runs of equal elements once (a List with its change count); a
//! Dictionary entry by entry as stored; a Record and a RecordRef by one rule (`record`): a temporary
//! table by its number by first sight (its rows once), a real one by its id, and everything the
//! variable carries (view, position, buffer, marks, load state, company, its table globals'
//! instances; its own read as whether its counts are current); a FieldRef and a KeyRef by their
//! record buffer and index; a RecordRef not open or closed as that; JSON and XML nodes by their tree
//! (a node reached from a variable with the node it hangs in); HTTP messages, content and headers,
//! ErrorInfo, FilterPageBuilder and a query by their state with the values and handles they hold by
//! these rules; text, TextBuilder, Version, Notification, ModuleInfo and the other plain library
//! states by value; plain values by `single_track::fp`. Any handle more than one owner holds is
//! numbered by first sight and written once (`shared <n>`). No form: a .NET object of the process, a
//! SQL counter value, a placeholder output's size, a test page or control, SecretText, HttpClient,
//! File, an open InStream or OutStream, a CRM record, a temporary record whose rows are not measured.
//! Entry points: `Render::new`, `Render::value`, `content`.

use crate::Session;
use al_builtins::library_kind::LibraryKind;
use al_builtins::value::{Instance, LibraryValue, RecordState, Value};
use std::cell::RefCell;
use std::rc::Rc;

/// How deep values may nest in one another (instances, arrays, lists, dictionaries, records, nodes).
const DEEPEST: u8 = 8;

type Out = Result<String, String>;
type Handle = Rc<RefCell<LibraryValue>>;

/// The content of a value as the debug format writes it: the one place the state lines use it. The text is compared
/// across sessions and shown only in the check's report line (`ALRUNNER_LOGIN_REPLAY_CHECK`).
pub(crate) fn content(x: &dyn std::fmt::Debug) -> String {
    format!("{x:?}")
}

/// The renderer of one [`Session::state_lines`] call: what it numbered, and the lines it wrote for them.
pub(crate) struct Render<'a> {
    session: &'a Session,
    /// The temporary tables seen, by id: records that share one (`Copy(x, true)`, `RecordRef.GetTable`) name one table.
    temp_tables: Vec<u64>,
    /// The codeunit instances seen, by address (never written out): two globals that hold one instance name one number,
    /// two instances two; a cycle ends at an instance already numbered.
    instances: Vec<usize>,
    /// The handles with more than one owner seen, by address (never written out), numbered as `instances` are.
    handles: Vec<usize>,
    /// What a numbered table, instance or handle holds, one line each, written once.
    pub(crate) lines: Vec<String>,
}

impl<'a> Render<'a> {
    pub(crate) fn new(session: &'a Session) -> Render<'a> {
        Render {
            session,
            temp_tables: Vec::new(),
            instances: Vec::new(),
            handles: Vec::new(),
            lines: Vec::new(),
        }
    }

    /// `v` as text. Every variant is named, so a new one is a compile error here and has to get a rule.
    pub(crate) fn value(&mut self, v: &Value, depth: u8) -> Out {
        match v {
            Value::Codeunit(rc) => self.instance(rc, depth),
            Value::Array(a) => self.array(a, depth),
            Value::List(l) => self.shared(Rc::as_ptr(l) as usize, Rc::strong_count(l), |s| s.list(l, depth)),
            Value::Dictionary(d) => {
                self.shared(Rc::as_ptr(d) as usize, Rc::strong_count(d), |s| s.dictionary(d, depth))
            }
            Value::Record(r) => self.record_handle(r, depth).map(|t| format!("Record {t}")),
            Value::Library(kind, cell) => self.shared(Rc::as_ptr(cell) as usize, Rc::strong_count(cell), |s| {
                s.library(kind, cell, depth)
            }),
            Value::Query(q) => self.shared(Rc::as_ptr(q) as usize, Rc::strong_count(q), |s| s.query(q, depth)),
            Value::TextConst(..) => Ok(content(v)),
            Value::TestPage(_) => Err("TestPage (a page the test client drives: it lives in its own session)".into()),
            Value::TestControl(_) => Err("TestControl (a control of a page the test client drives)".into()),
            Value::SqlCount(_) => Err("SQL counter value (its number counts the reads of its own session)".into()),
            Value::PlaceholderSize(..) => {
                Err("byte size of a placeholder report output (it answers only in the session that wrote it)".into())
            }
            Value::DotNet(_) => match crate::single_track::fp(v, &self.session.engine) {
                crate::single_track::Fp::Val(bytes) => Ok(content(&bytes)),
                crate::single_track::Fp::Opaque(_) => {
                    Err("DotNet object (an object of the .NET process: a handle of its session)".into())
                }
            },
            Value::Null
            | Value::Boolean(_)
            | Value::Integer(_)
            | Value::BigInteger(_)
            | Value::Decimal(_)
            | Value::Byte(_)
            | Value::Char(_)
            | Value::Text(_)
            | Value::Code(_)
            | Value::Date(..)
            | Value::Time(_)
            | Value::DateTime(_)
            | Value::Duration(_)
            | Value::Guid(_)
            | Value::DateFormula(_)
            | Value::Option(..)
            | Value::Enum(..)
            | Value::Blob(_)
            | Value::Unknown(_) => match crate::single_track::fp(v, &self.session.engine) {
                crate::single_track::Fp::Val(bytes) => Ok(content(&bytes)),
                crate::single_track::Fp::Opaque(what) => Err(what.to_string()),
            },
        }
    }

    /// A handle at `address` with `owners` strong owners: written in place by `render` when it has one owner; else
    /// `shared <n>`, and once `shared <n> <content>`.
    fn shared(&mut self, address: usize, owners: usize, render: impl FnOnce(&mut Self) -> Out) -> Out {
        if owners <= 1 {
            return render(self);
        }
        if let Some(n) = self.handles.iter().position(|a| *a == address) {
            return Ok(format!("shared {n}"));
        }
        self.handles.push(address);
        let n = self.handles.len() - 1;
        let text = render(self)?;
        self.lines.push(format!("shared {n} {text}"));
        Ok(format!("shared {n}"))
    }

    /// Elements as runs: `[<i>]: <element>`, and a run of equal elements once as `[a..b]: <element>`. `Err`: the first
    /// element with no comparable form (`<what>, element [<i>]: ...`).
    fn runs(&mut self, items: &[Value], what: &str, depth: u8) -> Out {
        let mut runs: Vec<(usize, usize, String)> = Vec::new();
        for (i, e) in items.iter().enumerate() {
            let text = (self.value(e, depth + 1)).map_err(|why| format!("{what}, element [{i}]: {why}"))?;
            match runs.last_mut() {
                Some((_, last, same)) if *same == text => *last = i,
                _ => runs.push((i, i, text)),
            }
        }
        let runs: Vec<String> = (runs.into_iter())
            .map(|(first, last, text)| match first == last {
                true => format!("[{first}]: {text}"),
                false => format!("[{first}..{last}]: {text}"),
            })
            .collect();
        Ok(format!("{what} [{}]", runs.join(", ")))
    }

    /// An array: `array of <n> elements [...]`, element by element ([`Render::runs`]; on a real app an instance a login
    /// leaves holds one).
    fn array(&mut self, a: &Rc<RefCell<Vec<Value>>>, depth: u8) -> Out {
        let Ok(a) = a.try_borrow() else {
            return Err("array (in use)".into());
        };
        let what = format!("array of {} elements", a.len());
        if depth > DEEPEST {
            return Err(format!("{what}: nested more than {DEEPEST} deep"));
        }
        self.runs(&a, &what, depth)
    }

    /// A List: element by element as an array, with its change count and whether it is read-only.
    fn list(&mut self, l: &Rc<RefCell<al_builtins::value::ListVec>>, depth: u8) -> Out {
        let Ok(l) = l.try_borrow() else {
            return Err("List (in use)".into());
        };
        let what = format!("List of {} elements", l.len());
        if depth > DEEPEST {
            return Err(format!("{what}: nested more than {DEEPEST} deep"));
        }
        let items = self.runs(&l, &what, depth)?;
        Ok(format!("{items} version {} read-only {}", l.version, l.read_only))
    }

    /// A Dictionary: its entries in the order they are stored (removed entries included: they decide where the next
    /// one goes), key and value by these rules.
    fn dictionary(&mut self, d: &Rc<RefCell<Vec<(Value, Value)>>>, depth: u8) -> Out {
        let Ok(d) = d.try_borrow() else {
            return Err("Dictionary (in use)".into());
        };
        let what = format!("Dictionary of {} entries", d.len());
        if depth > DEEPEST {
            return Err(format!("{what}: nested more than {DEEPEST} deep"));
        }
        let mut entries = Vec::with_capacity(d.len());
        for (i, (k, v)) in d.iter().enumerate() {
            let at = |why: String| format!("{what}, entry [{i}]: {why}");
            let k = self.value(k, depth + 1).map_err(at)?;
            let v = self.value(v, depth + 1).map_err(at)?;
            entries.push(format!("{k} => {v}"));
        }
        Ok(format!("{what} [{}]", entries.join(", ")))
    }

    /// A codeunit instance: `instance <n>`, and once `instance <n> <kind> <id> globals [...]` with every other field of the
    /// instance (whether it was initialized, the Execute check's `born`, the report rerun mark, whether a session watches it,
    /// an xmlport's separators, the instance it extends and its extensions' instances by number). `Err`: why it has no
    /// comparable form, with the path to the value that has none (`instance of codeunit <id> global <k>: ...`), or that
    /// what it holds is not measured (`Instance::unmeasured`).
    fn instance(&mut self, rc: &Rc<RefCell<Instance>>, depth: u8) -> Out {
        let address = Rc::as_ptr(rc) as usize;
        if let Some(n) = self.instances.iter().position(|a| *a == address) {
            return Ok(format!("instance {n}"));
        }
        let Ok(inst) = rc.try_borrow() else {
            return Err("Codeunit instance (in use)".into());
        };
        let object = &self.session.engine.program.objects[inst.object as usize];
        let (kind, id) = (content(&object.kind).to_lowercase(), object.id.unwrap_or(0));
        if depth > DEEPEST {
            return Err(format!(
                "instance of {kind} {id}: nested more than {DEEPEST} instances deep"
            ));
        }
        self.instances.push(address);
        let n = self.instances.len() - 1;
        let Instance {
            object: _,
            globals,
            initialized,
            rerun,
            base,
            exts,
            xp_text,
            born,
            single,
            unmeasured,
            // Counts the instance as live while it exists (`instance_cycles`): no state of the instance.
            live: _,
        } = &*inst;
        if !unmeasured.is_empty() {
            return Err(format!("instance of {kind} {id}: what it holds is not measured"));
        }
        let globals = (globals.iter().enumerate())
            .map(|(k, g)| {
                (self.value(g, depth + 1)).map_err(|why| format!("instance of {kind} {id} global {k}: {why}"))
            })
            .collect::<Result<Vec<String>, String>>()?;
        let base = match base.as_ref().and_then(std::rc::Weak::upgrade) {
            Some(b) => self.instance(&b, depth + 1)?,
            None => "none".into(),
        };
        let mut extensions = Vec::with_capacity(exts.len());
        for (object, ext) in exts {
            let ext_id = self.session.engine.program.objects[*object as usize].id.unwrap_or(0);
            extensions.push(format!("{ext_id}: {}", self.instance(ext, depth + 1)?));
        }
        self.lines.push(format!(
            "instance {n} {kind} {id} globals [{}] initialized {initialized} rerun {rerun} born restricted {} single \
             {single} xmlport separators {} base {base} extensions [{}]",
            globals.join(", "),
            born.0,
            content(xp_text),
            extensions.join(", ")
        ));
        Ok(format!("instance {n}"))
    }

    /// A record buffer behind a Record, RecordRef, FieldRef or KeyRef, numbered when several hold it.
    fn record_handle(&mut self, r: &Rc<RefCell<RecordState>>, depth: u8) -> Out {
        self.shared(Rc::as_ptr(r) as usize, Rc::strong_count(r), |s| s.record(r, depth))
    }

    /// What a record buffer carries, the one rule for a Record and a RecordRef: its table (a temporary one by its number
    /// in `temp_tables`, its rows written once; a real one by the table's id), and everything the variable carries (view:
    /// filters of every group, key, sort order; position, buffer, marks, load state, company), with the instances of
    /// its table's globals by number. Base Application's "Acc. Sched. KPI Event Handler" and "Change Log Management" hold
    /// temporary records after the login on BC 28.5.
    fn record(&mut self, r: &Rc<RefCell<RecordState>>, depth: u8) -> Out {
        let Ok(r) = r.try_borrow() else {
            return Err("Record (in use)".into());
        };
        if r.crm {
            return Err("Record of a CRM table (its rows live in the table connection)".into());
        }
        if depth > DEEPEST {
            return Err(format!("Record: nested more than {DEEPEST} deep"));
        }
        let table = match (&r.temp_id, r.temporary) {
            (Some(t), true) if !t.rows_unmeasured() => format!("temp table {}", self.temp_table(t.id())),
            (Some(_), true) => return Err("temporary Record whose rows are not measured".into()),
            (None, false) => format!(
                "table {}",
                self.session.engine.program.objects[r.table as usize].id.unwrap_or(0)
            ),
            _ => return Err("Record whose temporary flag and table disagree".into()),
        };
        // A clone leaves out what is not the record's content (the temporary table's id, which `temp_tables` numbers)
        // and what a clone does not copy (the table globals, rendered below, the own read and the probed source).
        let mut view = r.clone();
        view.temp_id = None;
        let mut text = format!(
            "{table}: {} own read {} probed {}",
            content(&view),
            self.own_read(&r),
            content(&r.probed_source)
        );
        for (what, set) in [
            ("globals", r.globals.instances()),
            ("xRec globals", r.globals.xrec_instances()),
        ] {
            if set.is_empty() {
                continue;
            }
            let mut held = Vec::with_capacity(set.len());
            for (object, inst) in &set {
                let id = self.session.engine.program.objects[*object as usize].id.unwrap_or(0);
                held.push(format!("{id}: {}", self.instance(inst, depth + 1)?));
            }
            text.push_str(&format!(" {what} [{}]", held.join(", ")));
        }
        Ok(text)
    }

    /// A record's own Insert and Get (`RecordState::own_read`). Its counts are of the session and the record's store, and
    /// al-vm `assigned_audit.rs` reads them only as equal to the counts at a later write: written as whether they equal
    /// the counts now (the counts only grow), not as numbers, which count from what the session and the store did before.
    fn own_read(&self, r: &RecordState) -> String {
        let Some(m) = &r.own_read.0 else {
            return "none".into();
        };
        let store = match &r.temp_id {
            Some(t) => self.session.temps.get(&t.id()).map_or(0, |s| s.change_counter()),
            None => self.session.store.change_counter(),
        };
        let current = m.commits == self.session.commits && m.store_changes == store;
        let facts = (m.got, m.cleared, &m.system_id, m.version, current);
        format!("(got, cleared, SystemId, version, current) {}", content(&facts))
    }

    /// The number of temporary table `id`; on first sight its rows are written (`MemStore::export_current`).
    fn temp_table(&mut self, id: u64) -> usize {
        if let Some(n) = self.temp_tables.iter().position(|t| *t == id) {
            return n;
        }
        self.temp_tables.push(id);
        let n = self.temp_tables.len() - 1;
        let rows = self.session.temps.get(&id).map(|s| s.export_current());
        for t in rows.unwrap_or_default() {
            for (_, row) in &t.rows {
                self.lines.push(format!(
                    "temp table {n} row {}|{}|{}",
                    t.company,
                    t.object,
                    content(row)
                ));
            }
        }
        n
    }

    /// A query variable: its filters, cap, result set and cursor, and the instance of its code by number.
    fn query(&mut self, q: &Rc<RefCell<al_builtins::value::QueryState>>, depth: u8) -> Out {
        let Ok(q) = q.try_borrow() else {
            return Err("Query (in use)".into());
        };
        let mut view = q.clone();
        view.instance = None;
        let instance = match &q.instance {
            Some(i) => self.instance(i, depth + 1)?,
            None => "none".into(),
        };
        Ok(format!("Query {} instance {instance}", content(&view)))
    }
}

impl Render<'_> {
    /// A library value (`Value::Library`) by what its cell holds. Every variant is named, so a new one is a compile error
    /// here and has to get a rule.
    fn library(&mut self, kind: &LibraryKind, cell: &Handle, depth: u8) -> Out {
        use LibraryValue as L;
        let Ok(lv) = cell.try_borrow() else {
            return Err(format!("{kind} (in use)"));
        };
        match &*lv {
            L::Empty if *kind == LibraryKind::RecordRef => Ok("RecordRef (not open)".into()),
            L::Empty => Ok(format!("{kind} (empty)")),
            L::ClosedRecordRef => Ok("RecordRef (closed)".into()),
            L::Text(_) if *kind == LibraryKind::SecretText => Err("SecretText (its text is never written out)".into()),
            L::ClearedStream
            | L::XmlNodeOfUnassignedElement
            | L::EmptiedRef(_)
            | L::HandedContext
            | L::HandlerContext(_)
            | L::Text(_)
            | L::TextCap(..)
            | L::SystemTable(_)
            | L::RecordId { .. }
            | L::WebServiceAction(_)
            | L::SessionSettings(_)
            | L::Notification(_)
            | L::ModuleInfo(_)
            | L::DataTransfer(_)
            | L::DataTransferPlan(_)
            | L::Cookie(_)
            | L::Version(_)
            | L::XmlNameTable
            | L::XmlReadOptions { .. }
            | L::XmlWriteOptions { .. }
            | L::XmlNamespaces(_) => Ok(format!("{kind} {}", content(&*lv))),
            L::RecordRef(r) => self.record_handle(r, depth).map(|t| format!("RecordRef {t}")),
            L::FieldRef(r, p) => self
                .record_handle(r, depth)
                .map(|t| format!("FieldRef field {p} of {t}")),
            L::KeyFieldRef(r, p) => (self.record_handle(r, depth)).map(|t| format!("FieldRef key field {p} of {t}")),
            L::KeyRef(r, k) => self.record_handle(r, depth).map(|t| format!("KeyRef key {k} of {t}")),
            L::UserNameFieldRef { rec, no, value } => (self.record_handle(rec, depth))
                .map(|t| format!("FieldRef user name field {no} value {} of {t}", content(value))),
            L::FilterPageBuilder(f) => {
                let mut controls = Vec::with_capacity(f.controls.len());
                for c in &f.controls {
                    let id = self.session.engine.program.objects[c.table as usize].id.unwrap_or(0);
                    let record = self.record_handle(&c.record, depth + 1)?;
                    controls.push(format!("{} table {id}: {record}", content(&c.name)));
                }
                Ok(format!(
                    "FilterPageBuilder caption {} controls [{}]",
                    content(&f.caption),
                    controls.join(", ")
                ))
            }
            L::ErrorInfo(e) => {
                let mut view = e.clone();
                (view.record_id, view.system_id, view.custom_dimensions) = (None, None, None);
                let record_id = self.optional(&e.record_id, depth)?;
                let system_id = self.optional(&e.system_id, depth)?;
                let dimensions = self.optional(&e.custom_dimensions, depth)?;
                // A collected error's `Callstack` is kept by the cell in a side table (al-builtins errorinfo.rs).
                let stack = al_builtins::errorinfo::callstack_of(cell);
                Ok(format!(
                    "ErrorInfo {} record id {record_id} system id {system_id} custom dimensions {dimensions} call stack {}",
                    content(&view),
                    content(&stack)
                ))
            }
            L::Json(_) | L::Xml(_) => self.tree(kind, &lv, depth).map(|t| format!("{kind} {t}")),
            L::XmlList(nodes) => {
                let mut held = Vec::with_capacity(nodes.len());
                for h in nodes {
                    held.push(self.top_node(kind, h, depth + 1)?);
                }
                Ok(format!("{kind} [{}]", held.join(", ")))
            }
            L::XmlAttrs(h) => self.top_node(kind, h, depth + 1).map(|t| format!("{kind} of {t}")),
            L::HttpHeaders(h) => self.headers(h).map(|t| format!("HttpHeaders {t}")),
            L::HttpContent(c) => {
                let mut view = c.clone();
                view.headers = al_builtins::value::new_header_list(al_builtins::value::HeaderKind::Content);
                Ok(format!(
                    "HttpContent {} headers {}",
                    content(&view),
                    self.headers(&c.headers)?
                ))
            }
            L::HttpRequest(r) => {
                let mut view = r.clone();
                view.headers = al_builtins::value::new_header_list(al_builtins::value::HeaderKind::Request);
                view.content = None;
                let body = self.optional(&r.content, depth)?;
                let headers = self.headers(&r.headers)?;
                Ok(format!(
                    "HttpRequestMessage {} content {body} headers {headers}",
                    content(&view)
                ))
            }
            L::HttpResponse(r) => {
                let mut view = r.clone();
                view.headers = al_builtins::value::new_header_list(al_builtins::value::HeaderKind::Response);
                view.content = Value::Null;
                let body = self.value(&r.content, depth + 1)?;
                let headers = self.headers(&r.headers)?;
                Ok(format!(
                    "HttpResponseMessage {} content {body} headers {headers}",
                    content(&view)
                ))
            }
            L::TestHttpRequest(r) => {
                let mut view = r.clone();
                (view.query, view.request_type) = (Value::Null, Value::Null);
                let query = self.value(&r.query, depth + 1)?;
                let request_type = self.value(&r.request_type, depth + 1)?;
                Ok(format!(
                    "TestHttpRequestMessage {} query {query} type {request_type}",
                    content(&view)
                ))
            }
            L::TestHttpResponse(r) => {
                let mut view = r.clone();
                view.headers = al_builtins::value::new_header_list(al_builtins::value::HeaderKind::Response);
                view.content = Value::Null;
                let body = self.value(&r.content, depth + 1)?;
                let headers = self.headers(&r.headers)?;
                Ok(format!(
                    "TestHttpResponseMessage {} content {body} headers {headers}",
                    content(&view)
                ))
            }
            L::HttpClient(_) => {
                Err("HttpClient (a client of the network: what it holds lies outside the session)".into())
            }
            L::File(_) => Err("File (a file of the server: its handle lies outside the session)".into()),
            L::InStream(_) => Err("InStream (an open stream: it reads a buffer it shares with its source)".into()),
            L::OutStream(..) => {
                Err("OutStream (an open stream: it writes a Blob buffer it shares with its owner)".into())
            }
        }
    }

    /// A value a library state may hold (`none` when it holds none).
    fn optional(&mut self, v: &Option<Value>, depth: u8) -> Out {
        match v {
            Some(v) => self.value(v, depth + 1),
            None => Ok("none".into()),
        }
    }

    /// A header list, which a message and every `HttpHeaders` handed out of it share.
    fn headers(&mut self, h: &al_builtins::value::HttpHeaderList) -> Out {
        self.shared(Rc::as_ptr(h) as usize, Rc::strong_count(h), |_| match h.try_borrow() {
            Ok(bag) => Ok(content(&*bag)),
            Err(_) => Err("HttpHeaders (in use)".into()),
        })
    }

    /// A JSON or XML node reached from outside its tree (a variable, a node list): numbered when its tree holds it too,
    /// with the node it hangs in ([`Render::tree`]).
    fn top_node(&mut self, kind: &LibraryKind, h: &Handle, depth: u8) -> Out {
        self.shared(Rc::as_ptr(h) as usize, Rc::strong_count(h), |s| match h.try_borrow() {
            Ok(lv) => s.tree(kind, &lv, depth),
            Err(_) => Err(format!("{kind} (in use)")),
        })
    }

    /// A JSON or XML node reached from outside its tree: its content ([`Render::node`]) and, while its parent lives, `in
    /// <parent>`, the parent by the same rule up to the root (a node the parent also holds is numbered, so the parent's
    /// content names it by its number).
    fn tree(&mut self, kind: &LibraryKind, lv: &LibraryValue, depth: u8) -> Out {
        let own = self.node(kind, lv, depth)?;
        let parent = match lv {
            LibraryValue::Json(c) => c.parent.as_ref().and_then(std::rc::Weak::upgrade),
            LibraryValue::Xml(c) => c.parent.as_ref().and_then(std::rc::Weak::upgrade),
            _ => None,
        };
        match parent {
            Some(p) => Ok(format!("{own} in {}", self.top_node(kind, &p, depth + 1)?)),
            None => Ok(own),
        }
    }

    /// A JSON or XML node's content: its own data, its children by [`Render::child`].
    fn node(&mut self, kind: &LibraryKind, lv: &LibraryValue, depth: u8) -> Out {
        use al_builtins::json::JsonNode;
        use al_builtins::xml::XData;
        if depth > DEEPEST {
            return Err(format!("{kind}: nested more than {DEEPEST} deep"));
        }
        let kids = |s: &mut Self, hs: &[Handle]| -> Out {
            let mut held = Vec::with_capacity(hs.len());
            for (i, h) in hs.iter().enumerate() {
                held.push(
                    s.child(kind, h, depth)
                        .map_err(|why| format!("{kind}, child [{i}]: {why}"))?,
                );
            }
            Ok(format!("[{}]", held.join(", ")))
        };
        match lv {
            LibraryValue::Json(c) => {
                let own = match &c.node {
                    JsonNode::Object(members) => {
                        let mut held = Vec::with_capacity(members.len());
                        for (k, h) in members {
                            let child = self
                                .child(kind, h, depth)
                                .map_err(|why| format!("{kind}, member: {why}"))?;
                            held.push(format!("{}: {child}", content(k)));
                        }
                        format!("object {{{}}}", held.join(", "))
                    }
                    JsonNode::Array(items) => format!("array {}", kids(self, items)?),
                    JsonNode::Scalar(v) => format!("value {}", content(v)),
                };
                Ok(format!("{own} unpinned {}", content(&c.unpinned)))
            }
            LibraryValue::Xml(c) => match &c.data {
                XData::Document { decl, children } => {
                    let decl = match decl {
                        Some(h) => self.child(kind, h, depth)?,
                        None => "none".into(),
                    };
                    Ok(format!(
                        "document declaration {decl} children {}",
                        kids(self, children)?
                    ))
                }
                XData::Element {
                    prefix,
                    local,
                    ns,
                    attrs,
                    children,
                    open,
                } => Ok(format!(
                    "element {} attributes {} children {} open {open}",
                    content(&(prefix, local, ns)),
                    kids(self, attrs)?,
                    kids(self, children)?
                )),
                other => Ok(content(other)),
            },
            _ => Err(format!("{kind} (not a node)")),
        }
    }

    /// A node reached from its parent: numbered when something else holds it too.
    fn child(&mut self, kind: &LibraryKind, h: &Handle, depth: u8) -> Out {
        self.shared(Rc::as_ptr(h) as usize, Rc::strong_count(h), |s| match h.try_borrow() {
            Ok(lv) => s.node(kind, &lv, depth + 1),
            Err(_) => Err(format!("{kind} (in use)")),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use al_builtins::value::{SqlCount, SqlForm, SqlType};

    /// A session over a table and a codeunit whose globals are a record of the table and a temporary one; the globals of
    /// an instance of it.
    fn session_and_records() -> (Session, Vec<Value>) {
        const SRC: &str = "table 50100 Row { fields { field(1; Id; Integer) { } } }
            codeunit 50100 Keeper { var R: Record Row; T: Record Row temporary; }";
        let (ws, l) = al_sema::lower::compile(
            vec![al_sema::workspace::AppSource {
                id: "a".into(),
                name: "a".into(),
                dependencies: vec![],
                files: vec![("f.al".into(), SRC.into())],
            }],
            &Default::default(),
        );
        assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
        let engine = crate::Engine::new(l.program, Vec::new());
        let at = engine.program.objects.iter().position(|o| o.name == "Keeper").unwrap();
        let mut session = Session::new(engine);
        let globals = session.codeunit_instance(at).borrow().globals.clone();
        (session, globals)
    }

    fn library(kind: LibraryKind, v: LibraryValue) -> Value {
        Value::Library(kind, Rc::new(RefCell::new(v)))
    }

    // Each kind that has no comparable form says why, by its type name only (never its value).
    #[test]
    fn a_value_with_no_comparable_form_says_why_by_its_type_name() {
        let (session, globals) = session_and_records();
        let mut render = Render::new(&session);
        let counter = SqlCount {
            n: 7,
            form: SqlForm::Delta,
            ty: SqlType::BigInteger,
        };
        let host = al_builtins::value::DotNetObject::host(5, "System.Object".into(), Rc::default());
        let opaque = [
            (
                Value::SqlCount(counter),
                "SQL counter value (its number counts the reads of its own session)",
            ),
            (
                Value::PlaceholderSize("zq9".into(), SqlType::Integer),
                "byte size of a placeholder report output (it answers only in the session that wrote it)",
            ),
            (
                Value::DotNet(Rc::new(host)),
                "DotNet object (an object of the .NET process: a handle of its session)",
            ),
            (
                library(LibraryKind::SecretText, LibraryValue::Text("zq9".into())),
                "SecretText (its text is never written out)",
            ),
            (
                library(LibraryKind::HttpClient, LibraryValue::HttpClient(Default::default())),
                "HttpClient (a client of the network: what it holds lies outside the session)",
            ),
            (
                library(LibraryKind::File, LibraryValue::File(Default::default())),
                "File (a file of the server: its handle lies outside the session)",
            ),
            (
                library(
                    LibraryKind::InStream,
                    LibraryValue::InStream(al_builtins::stream::InState::snapshot(
                        b"zq9".to_vec(),
                        Default::default(),
                    )),
                ),
                "InStream (an open stream: it reads a buffer it shares with its source)",
            ),
            (
                library(
                    LibraryKind::OutStream,
                    LibraryValue::OutStream(al_builtins::stream::new_blob(), Default::default(), None),
                ),
                "OutStream (an open stream: it writes a Blob buffer it shares with its owner)",
            ),
        ];
        for (v, why) in opaque {
            assert_eq!(render.value(&v, 0), Err(why.to_string()));
        }
        // A record of a CRM table, a temporary record whose rows are not measured, and one whose flag and table disagree.
        let (Value::Record(real), Value::Record(temp)) = (&globals[0], &globals[1]) else {
            panic!("{globals:?}")
        };
        let crm = Value::Record(Rc::new(RefCell::new(RecordState {
            crm: true,
            ..real.borrow().clone()
        })));
        assert_eq!(
            render.value(&crm, 0),
            Err("Record of a CRM table (its rows live in the table connection)".into())
        );
        let disagree = Value::Record(Rc::new(RefCell::new(RecordState {
            temporary: true,
            ..real.borrow().clone()
        })));
        assert_eq!(
            render.value(&disagree, 0),
            Err("Record whose temporary flag and table disagree".into())
        );
        temp.borrow().temp_id.as_ref().unwrap().mark_rows_unmeasured();
        assert_eq!(
            render.value(&globals[1], 0),
            Err("temporary Record whose rows are not measured".into())
        );
    }

    // Nesting deeper than the limit and a value in use are named as such.
    #[test]
    fn too_deep_and_in_use_are_named() {
        let (session, _) = session_and_records();
        let mut render = Render::new(&session);
        let mut v = Value::Integer(1);
        for _ in 0..10 {
            v = Value::List(Rc::new(RefCell::new(vec![v].into())));
        }
        let why = render.value(&v, 0).unwrap_err();
        assert!(why.ends_with("List of 1 elements: nested more than 8 deep"), "{why}");
        let list = Rc::new(RefCell::new(al_builtins::value::ListVec::from(vec![])));
        let _held = list.borrow_mut();
        assert_eq!(render.value(&Value::List(list.clone()), 0), Err("List (in use)".into()));
    }

    // A handle two places hold is written once, by number, in both; a handle one place holds is written in place.
    #[test]
    fn a_handle_two_places_hold_is_numbered_and_written_once() {
        let (session, _) = session_and_records();
        let mut render = Render::new(&session);
        let list = Rc::new(RefCell::new(al_builtins::value::ListVec::from(vec![Value::Integer(1)])));
        let (a, b) = (Value::List(list.clone()), Value::List(list));
        assert_eq!(render.value(&a, 0), Ok("shared 0".into()));
        assert_eq!(render.value(&b, 0), Ok("shared 0".into()));
        assert_eq!(render.lines.len(), 1, "{:?}", render.lines);
        assert!(
            render.lines[0].starts_with("shared 0 List of 1 elements [[0]: "),
            "{:?}",
            render.lines
        );
        let own = Value::List(Rc::new(RefCell::new(vec![Value::Integer(1)].into())));
        assert!(render.value(&own, 0).unwrap().starts_with("List of 1 elements"));
    }

    // A collected error's `Callstack` lives in a side table keyed by the ErrorInfo's cell (al-builtins errorinfo.rs
    // `CALLSTACKS`), not in its state: two ErrorInfo values that differ only in it do not render the same.
    #[test]
    fn an_error_info_renders_its_call_stack() {
        let (session, _) = session_and_records();
        let state = || al_builtins::value::ErrorInfoState {
            message: "m".into(),
            title: String::new(),
            detailed_message: String::new(),
            control_name: String::new(),
            verbosity: 1,
            data_classification: 0,
            error_type: None,
            collectible: true,
            table_id: 0,
            record_id: None,
            system_id: None,
            field_no: 0,
            page_no: 0,
            custom_dimensions: None,
            built: al_builtins::value::ErrorInfoBuilt::Other,
        };
        let kind = LibraryKind::from("ErrorInfo");
        let (with, without) = (
            library(kind.clone(), LibraryValue::ErrorInfo(state())),
            library(kind, LibraryValue::ErrorInfo(state())),
        );
        al_builtins::errorinfo::set_callstack_of(&with, "\"P\"(CodeUnit 1).Collect line 5\r\n");
        let mut render = Render::new(&session);
        assert_ne!(render.value(&with, 0), render.value(&without, 0));
    }
}
