//! The walk that freezes the object graph into an [`Arena`] (design section 2.2). It starts from the SingleInstance
//! globals in ascending object index, then the live manual bindings in order (the order `Session::state_lines` uses), then
//! the live records of `Session::temp_blob_impl_fields`, and
//! keeps its own stack: no depth limit. Each handle is looked up by its address; a new one gets the next number.
//!
//! Every owner is accounted for: after the walk each handle's strong count must equal the strong references the walk
//! found to it. More means something outside the image holds it ([`NotImaged::HeldOutside`]).

use super::NotImaged;
use super::arena::{Arena, DotNetNode, Frozen, InstanceNode, Node, NodeId, QueryNode};
use crate::Session;
use al_builtins::plain::{Plain, PlainFilter};
use al_builtins::value::{Instance, LibraryValue, ListVec, QueryState, RecordState, TableGlobals, TempId, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// A clone of a node's handle, held while the walk runs, so the strong counts can be compared at its end.
pub(crate) enum Holder {
    Instance(Rc<RefCell<Instance>>),
    Array(Rc<RefCell<Vec<Value>>>),
    List(Rc<RefCell<ListVec>>),
    Dictionary(Rc<RefCell<Vec<(Value, Value)>>>),
    Record(Rc<RefCell<RecordState>>),
    Globals(TableGlobals),
    Library(al_builtins::library_kind::LibraryKind, Rc<RefCell<LibraryValue>>),
    Blob(al_builtins::stream::BlobRef),
    TempTable(TempId),
    Row(Rc<Vec<Value>>),
    Query(Rc<RefCell<QueryState>>),
    DotNet(Rc<al_builtins::value::DotNetObject>),
}

impl Holder {
    /// The strong owners of the handle besides this clone.
    fn owners(&self) -> usize {
        match self {
            Holder::Instance(r) => Rc::strong_count(r),
            Holder::Array(r) => Rc::strong_count(r),
            Holder::List(r) => Rc::strong_count(r),
            Holder::Dictionary(r) => Rc::strong_count(r),
            Holder::Record(r) => Rc::strong_count(r),
            Holder::Globals(g) => g.set_address().map_or(0, |(_, n)| n),
            Holder::Library(_, r) => Rc::strong_count(r),
            Holder::Blob(b) => std::sync::Arc::strong_count(b),
            Holder::TempTable(t) => t.sharers(),
            Holder::Row(r) => Rc::strong_count(r),
            Holder::Query(r) => Rc::strong_count(r),
            Holder::DotNet(r) => Rc::strong_count(r),
        }
        .saturating_sub(1)
    }

    /// The kind of the handle, for a path.
    fn kind(&self) -> &'static str {
        match self {
            Holder::Instance(_) => "codeunit instance",
            Holder::Array(_) => "array",
            Holder::List(_) => "List",
            Holder::Dictionary(_) => "Dictionary",
            Holder::Record(_) => "record",
            Holder::Globals(_) => "table globals",
            Holder::Library(..) => "library value",
            Holder::Blob(_) => "Blob",
            Holder::TempTable(_) => "temporary table",
            Holder::Row(_) => "fetched row",
            Holder::Query(_) => "query",
            Holder::DotNet(_) => "DotNet object",
        }
    }
}

/// The walk's state: the arena it builds, and per node its handle, the strong references found to it and where it was
/// first seen.
pub(crate) struct Walker<'a> {
    pub(crate) session: &'a Session,
    pub(crate) arena: Arena,
    seen: HashMap<(u8, usize), NodeId>,
    holders: Vec<Holder>,
    found: Vec<usize>,
    paths: Vec<String>,
    todo: Vec<NodeId>,
    /// The instances `singletons` holds, by object index, and the live bindings, as nodes.
    pub(crate) singletons: Vec<(usize, NodeId)>,
    pub(crate) bindings: Vec<NodeId>,
    /// The live records of `Session::temp_blob_impl_fields`, as nodes, with their field positions.
    pub(crate) temp_blob_fields: Vec<(NodeId, usize)>,
}

impl<'a> Walker<'a> {
    pub(crate) fn new(session: &'a Session) -> Walker<'a> {
        Walker {
            session,
            arena: Arena::default(),
            seen: HashMap::new(),
            holders: Vec::new(),
            found: Vec::new(),
            paths: Vec::new(),
            todo: Vec::new(),
            singletons: Vec::new(),
            bindings: Vec::new(),
            temp_blob_fields: Vec::new(),
        }
    }

    /// The object's id, for a path.
    pub(crate) fn id(&self, object: usize) -> u32 {
        self.session.engine.program.objects[object].id.unwrap_or(0)
    }

    /// Walks the whole graph and checks every owner.
    pub(crate) fn walk(mut self) -> Result<Walker<'a>, NotImaged> {
        let mut singles: Vec<(&usize, &Rc<RefCell<Instance>>)> = self.session.singletons.iter().collect();
        singles.sort_by_key(|(o, _)| **o);
        for (&o, inst) in singles {
            let path = format!("codeunit {}", self.id(o));
            let id = self.handle(Holder::Instance(inst.clone()), 1, path);
            self.singletons.push((o, id));
        }
        for (k, b) in self.session.events.bindings.iter().enumerate() {
            let Some(inst) = b.instance.upgrade() else { continue };
            let id = self.handle(Holder::Instance(inst), 0, format!("binding {k}"));
            self.bindings.push(id);
        }
        for (k, (r, pos)) in self.session.temp_blob_impl_fields.live().into_iter().enumerate() {
            let id = self.handle(Holder::Record(r), 0, format!("Temp Blob field {k}"));
            self.temp_blob_fields.push((id, pos));
        }
        while let Some(id) = self.todo.pop() {
            let node = self.fill(id)?;
            self.arena.nodes[id.0 as usize] = node;
        }
        for (i, h) in self.holders.iter().enumerate() {
            if h.owners() != self.found[i] {
                return Err(NotImaged::HeldOutside {
                    path: format!("{} ({})", self.paths[i], h.kind()),
                });
            }
        }
        Ok(self)
    }

    /// The node of a handle, numbered on first sight; `strong`: the strong references this sight is (0 for a `Weak`).
    pub(crate) fn handle(&mut self, h: Holder, strong: usize, path: String) -> NodeId {
        let key = match &h {
            Holder::Instance(r) => (0, Rc::as_ptr(r) as usize),
            Holder::Array(r) => (1, Rc::as_ptr(r) as usize),
            Holder::List(r) => (2, Rc::as_ptr(r) as usize),
            Holder::Dictionary(r) => (3, Rc::as_ptr(r) as usize),
            Holder::Record(r) => (4, Rc::as_ptr(r) as usize),
            Holder::Globals(g) => (5, g.set_address().map_or(0, |(a, _)| a)),
            Holder::Library(_, r) => (6, Rc::as_ptr(r) as usize),
            Holder::Blob(b) => (7, std::sync::Arc::as_ptr(b) as usize),
            Holder::TempTable(t) => (8, t.id() as usize),
            Holder::Row(r) => (9, Rc::as_ptr(r) as usize),
            Holder::Query(r) => (10, Rc::as_ptr(r) as usize),
            Holder::DotNet(r) => (11, Rc::as_ptr(r) as usize),
        };
        if let Some(&id) = self.seen.get(&key) {
            self.found[id.0 as usize] += strong;
            return id;
        }
        let id = NodeId(self.arena.nodes.len() as u32);
        self.seen.insert(key, id);
        self.arena.nodes.push(Node::Pending);
        self.holders.push(h);
        self.found.push(strong);
        self.paths.push(path);
        self.todo.push(id);
        id
    }

    /// Where node `id` was first seen.
    pub(crate) fn path(&self, id: NodeId) -> &str {
        &self.paths[id.0 as usize]
    }

    /// What node `id` holds.
    fn fill(&mut self, id: NodeId) -> Result<Node, NotImaged> {
        let path = self.path(id).to_string();
        match &self.holders[id.0 as usize] {
            Holder::Instance(r) => {
                let r = r.clone();
                self.instance(&r, &path).map(Node::Instance)
            }
            Holder::Array(a) => {
                let a = a.clone();
                let a = borrowed(a.try_borrow(), &path, "array")?;
                Ok(Node::Array(self.values(&a, &path)?))
            }
            Holder::List(l) => {
                let l = l.clone();
                let l = borrowed(l.try_borrow(), &path, "List")?;
                Ok(Node::List {
                    items: self.values(&l, &path)?,
                    version: l.version,
                    read_only: l.read_only,
                })
            }
            Holder::Dictionary(d) => {
                let d = d.clone();
                let d = borrowed(d.try_borrow(), &path, "Dictionary")?;
                let mut entries = Vec::with_capacity(d.len());
                for (i, (k, v)) in d.iter().enumerate() {
                    let at = format!("{path}, entry [{i}]");
                    entries.push((self.value(k, &at)?, self.value(v, &at)?));
                }
                Ok(Node::Dictionary(entries))
            }
            Holder::Record(r) => {
                let r = r.clone();
                let r = borrowed(r.try_borrow(), &path, "Record")?;
                Ok(Node::Record(Box::new(self.record(&r, &path)?)))
            }
            Holder::Globals(g) => {
                let g = g.share();
                self.globals(&g, &path)
            }
            Holder::Library(kind, cell) => {
                let (kind, cell) = (kind.clone(), cell.clone());
                let lv = borrowed(cell.try_borrow(), &path, "library value")?;
                Ok(Node::Library(kind.clone(), self.library(&kind, &cell, &lv, &path)?))
            }
            Holder::Blob(b) => Ok(Node::Blob(b.borrow().clone())),
            Holder::TempTable(t) => {
                let t = t.clone();
                self.temp_table(&t, &path).map(Node::TempTable)
            }
            Holder::Row(r) => {
                let r = r.clone();
                Ok(Node::Row(self.field_values(&r, &path)?))
            }
            Holder::Query(q) => {
                let q = q.clone();
                let q = borrowed(q.try_borrow(), &path, "Query")?;
                self.query(&q, &path).map(|q| Node::Query(Box::new(q)))
            }
            Holder::DotNet(d) => {
                let d = d.clone();
                self.dotnet(&d, &path).map(Node::DotNet)
            }
        }
    }

    /// A codeunit instance.
    fn instance(&mut self, rc: &Rc<RefCell<Instance>>, path: &str) -> Result<InstanceNode, NotImaged> {
        let inst = borrowed(rc.try_borrow(), path, "Codeunit instance")?;
        let Instance {
            object,
            globals,
            initialized,
            rerun,
            base,
            exts,
            xp_text,
            born,
            single,
            unmeasured,
            // Counts the instance as live while it exists: a new count goes with the new instance.
            live: _,
        } = &*inst;
        if unmeasured.is_some() {
            return Err(NotImaged::Value {
                path: path.to_string(),
                what: "codeunit instance whose content is not measured".into(),
            });
        }
        let mut frozen = Vec::with_capacity(globals.len());
        for (k, g) in globals.iter().enumerate() {
            frozen.push(self.value(g, &format!("{path} global {k}"))?);
        }
        let base = (base.as_ref().and_then(std::rc::Weak::upgrade))
            .map(|b| self.handle(Holder::Instance(b), 0, format!("{path} base")));
        let exts = (exts.iter())
            .map(|(o, e)| {
                let at = format!("{path} extension {}", self.id(*o as usize));
                (*o, self.handle(Holder::Instance(e.clone()), 1, at))
            })
            .collect();
        Ok(InstanceNode {
            object: *object,
            globals: frozen,
            initialized: *initialized,
            rerun: *rerun,
            base,
            exts,
            xp_text: xp_text.clone(),
            born: born.0,
            single: *single,
        })
    }

    /// A table globals set: its instances and its xRec set.
    fn globals(&mut self, g: &TableGlobals, path: &str) -> Result<Node, NotImaged> {
        let instances = (g.instances().into_iter())
            .map(|(o, i)| {
                let at = format!("{path} instance {}", self.id(o as usize));
                (o, self.handle(Holder::Instance(i), 1, at))
            })
            .collect();
        let xrec = g
            .made_xrec()
            .map(|x| self.handle(Holder::Globals(x), 1, format!("{path} xRec")));
        Ok(Node::Globals { instances, xrec })
    }

    /// A query variable without a result set; one with a result set is deferred (its table stamps are of the session's
    /// store).
    fn query(&mut self, q: &QueryState, path: &str) -> Result<QueryNode, NotImaged> {
        let QueryState {
            object,
            filters,
            filter_replaced,
            top,
            security_filtering,
            run,
            instance,
            in_before_open,
            closed,
            reopen,
        } = q;
        if run.is_some() {
            return Err(deferred(path, "Query with an open result set"));
        }
        let mut plain = Vec::with_capacity(filters.len());
        for (field, expr) in filters {
            let f = al_builtins::filter::FieldFilter {
                lang: 0,
                field: *field,
                expr: expr.clone(),
                group: 0,
            };
            let p = PlainFilter::of(&f).ok_or_else(|| no_form(path, "query filter"))?;
            plain.push((*field, p.expr));
        }
        let instance = (instance.as_ref()).map(|i| self.handle(Holder::Instance(i.clone()), 1, format!("{path} code")));
        Ok(QueryNode {
            object: *object,
            filters: plain,
            filter_replaced: *filter_replaced,
            top: *top,
            security_filtering: *security_filtering,
            instance,
            in_before_open: *in_before_open,
            closed: *closed,
            reopen: reopen.clone(),
        })
    }

    /// A .NET object the check renders exactly: an enum value, or an object the runtime serves itself with its state. The
    /// server configuration served by the .NET process (also rendered: the same in every session) is a handle of the
    /// process, which a thawn session does not hold.
    fn dotnet(&mut self, d: &al_builtins::value::DotNetObject, path: &str) -> Result<DotNetNode, NotImaged> {
        use al_builtins::value::DotNetKind;
        match &d.kind {
            DotNetKind::Enum {
                clr_type,
                name,
                ordinal,
            } => Ok(DotNetNode::Enum {
                clr_type: clr_type.clone(),
                name: name.clone(),
                ordinal: *ordinal,
            }),
            DotNetKind::Internal { type_name, state } => {
                let state = borrowed(state.try_borrow(), path, "DotNet object")?;
                Ok(DotNetNode::Internal {
                    type_name: type_name.clone(),
                    state: self.values(&state, path)?,
                })
            }
            DotNetKind::Host { .. } => Err(deferred(path, "DotNet object of the .NET process")),
        }
    }

    pub(crate) fn values(&mut self, items: &[Value], path: &str) -> Result<Vec<Frozen>, NotImaged> {
        let mut out = Vec::with_capacity(items.len());
        for (i, v) in items.iter().enumerate() {
            out.push(self.value(v, &format!("{path}, element [{i}]"))?);
        }
        Ok(out)
    }

    /// The values of a record's fields or a row: plain, or a Blob buffer (a field holds no other handle).
    pub(crate) fn field_values(&mut self, items: &[Value], path: &str) -> Result<Vec<Frozen>, NotImaged> {
        let mut out = Vec::with_capacity(items.len());
        for (i, v) in items.iter().enumerate() {
            let at = format!("{path}, field [{i}]");
            out.push(match v {
                Value::Blob(b) => Frozen::Node(self.handle(Holder::Blob(b.clone()), 1, at)),
                _ => Frozen::Plain(Plain::of(v).ok_or_else(|| no_form(&at, "a field value"))?),
            });
        }
        Ok(out)
    }

    /// A value. Every variant is named, so a new one is a compile error here and has to get a rule (as in
    /// `state_render.rs`).
    pub(crate) fn value(&mut self, v: &Value, path: &str) -> Result<Frozen, NotImaged> {
        let node = |w: &mut Self, h: Holder| Ok(Frozen::Node(w.handle(h, 1, path.to_string())));
        match v {
            Value::Codeunit(r) => node(self, Holder::Instance(r.clone())),
            Value::Array(a) => node(self, Holder::Array(a.clone())),
            Value::List(l) => node(self, Holder::List(l.clone())),
            Value::Dictionary(d) => node(self, Holder::Dictionary(d.clone())),
            Value::Record(r) => node(self, Holder::Record(r.clone())),
            Value::Library(kind, cell) => node(self, Holder::Library(kind.clone(), cell.clone())),
            Value::Blob(b) => node(self, Holder::Blob(b.clone())),
            Value::Query(q) => node(self, Holder::Query(q.clone())),
            // The check is the one owner of which objects have an exact form (state_render.rs): one it renders as opaque is
            // an object of the .NET process, a handle of this session. One it renders exactly (an enum value, an object the
            // runtime serves itself, such as `ALConfigSettings.Instance`) waits for its node (the first follow-up).
            Value::DotNet(d) => match crate::state_render::Render::new(self.session).value(v, 0) {
                Ok(_) => node(self, Holder::DotNet(d.clone())),
                Err(_) => Err(never(path, "DotNet object of the .NET process")),
            },
            Value::TestPage(_) => Err(never(path, "TestPage")),
            Value::TestControl(_) => Err(never(path, "TestControl")),
            Value::SqlCount(_) => Err(never(path, "SQL counter value")),
            Value::PlaceholderSize(..) => Err(never(path, "byte size of a placeholder report output")),
            Value::Null
            | Value::Boolean(_)
            | Value::Integer(_)
            | Value::BigInteger(_)
            | Value::Decimal(_)
            | Value::Byte(_)
            | Value::Char(_)
            | Value::Text(_)
            | Value::TextConst(..)
            | Value::Code(_)
            | Value::Date(..)
            | Value::Time(_)
            | Value::DateTime(_)
            | Value::Duration(_)
            | Value::Guid(_)
            | Value::DateFormula(_)
            | Value::Option(..)
            | Value::Enum(..)
            | Value::Unknown(_) => Ok(Frozen::Plain(
                Plain::of(v).ok_or_else(|| no_form(path, "a plain value"))?,
            )),
        }
    }
}

/// A borrow of a handle, or why it has no image: it is in use.
pub(crate) fn borrowed<T>(r: Result<T, std::cell::BorrowError>, path: &str, what: &str) -> Result<T, NotImaged> {
    r.map_err(|_| NotImaged::Value {
        path: path.to_string(),
        what: format!("{what} (in use)"),
    })
}

/// A shape v1 of the image defers: its own reason, counted in the report.
pub(crate) fn deferred(path: &str, what: &str) -> NotImaged {
    NotImaged::Value {
        path: path.to_string(),
        what: format!("{what} (deferred)"),
    }
}

/// A value that lies outside the session (a page, a stream, a file, a counter of the session's own reads): never imaged.
pub(crate) fn never(path: &str, what: &str) -> NotImaged {
    NotImaged::Value {
        path: path.to_string(),
        what: format!("{what} (outside the session)"),
    }
}

/// A value with no plain form.
pub(crate) fn no_form(path: &str, what: &str) -> NotImaged {
    NotImaged::Value {
        path: path.to_string(),
        what: format!("{what} with no plain form"),
    }
}

#[cfg(test)]
mod tests {
    use crate::{Engine, Session};
    use al_builtins::value::Value;

    const SRC: &str = r#"
table 50110 BlobRow
{
    fields { field(1; Id; Integer) { } field(2; Data; Blob) { } }
    keys { key(PK; Id) { } }
}
codeunit 50110 Holder
{
    SingleInstance = true;
    var TempBlob: Record BlobRow temporary;
}
"#;

    /// A session over `SRC` whose single instance of Holder holds the temporary record; the record's handle.
    fn session_with_temp_blob() -> (
        Session,
        std::rc::Rc<std::cell::RefCell<al_builtins::value::RecordState>>,
    ) {
        let l = al_testkit::compile(SRC);
        let engine = Engine::new(l.program.clone(), al_testkit::blocked(&l));
        let at = engine.program.objects.iter().position(|o| o.name == "Holder").unwrap();
        let mut session = Session::new(engine);
        let inst = session.codeunit_instance(at);
        let Value::Record(rec) = inst.borrow().globals[0].clone() else {
            panic!("a record global")
        };
        session.commit_transaction();
        (session, rec)
    }

    // A "Temp Blob Impl." field the session noted on a record a SingleInstance global still holds (temp_blob_impl_field.rs:
    // `Base64Convert.ToBase64` asks it) is kept by the image: the twin notes the twin's own record of that global.
    #[test]
    fn a_live_temp_blob_record_is_noted_on_the_twins_own_record() {
        let (mut session, rec) = session_with_temp_blob();
        session.temp_blob_impl_fields.note_all(&[(rec.clone(), 1)]);
        let original = std::rc::Rc::as_ptr(&rec);
        drop(rec);
        let image = session.login_image(0).unwrap_or_else(|why| panic!("{}", why.reason()));
        let mut twin = Session::new(session.engine.clone());
        twin.thaw_login_image(&image);
        let noted = twin.temp_blob_impl_fields.live();
        assert_eq!(noted.len(), 1);
        let at = twin
            .engine
            .program
            .objects
            .iter()
            .position(|o| o.name == "Holder")
            .unwrap();
        let Value::Record(own) = twin.codeunit_instance(at).borrow().globals[0].clone() else {
            panic!("a record global")
        };
        assert!(std::rc::Rc::ptr_eq(&noted[0].0, &own), "the twin's own record");
        assert_ne!(std::rc::Rc::as_ptr(&noted[0].0), original, "not the original's");
        assert_eq!(noted[0].1, 1);
    }

    const KINDS: &str = r#"
table 50111 KRow
{
    fields { field(1; Id; Integer) { } field(2; Data; Blob) { } field(3; Name; Text[20]) { } }
    keys { key(PK; Id) { } }
}
codeunit 50111 Kinds
{
    SingleInstance = true;
    var
        L: List of [Text]; D: Dictionary of [Integer, Text]; A: array[2] of Integer; T: Record KRow temporary;
        R: Record KRow; RR: RecordRef; J: JsonObject; JA: JsonArray; X: XmlDocument; N: Notification; E: ErrorInfo;
        TB: TextBuilder; Req: HttpRequestMessage; H: HttpHeaders; C: Codeunit Kinds2; I: InStream; V: Variant;
    procedure FillList() begin L.Add('a'); L.Add('é'); end;
    procedure FillDict() begin D.Add(1, 'x'); end;
    procedure FillArray() begin A[2] := 7; end;
    procedure FillTemp() begin T.Id := 1; T.Name := 'n'; T.Insert(); T.SetRange(Id, 1); end;
    procedure FillRecord() begin R.SetRange(Name, 'a'); R.SetCurrentKey(Id); end;
    procedure FillRecordRef() begin RR.Open(50111); end;
    procedure FillJson() begin J.Add('a', 1); end;
    procedure FillJsonArray() begin JA.Add(1); end;
    procedure FillXml() begin X := XmlDocument.Create(); end;
    procedure FillNotification() begin N.Message('m'); end;
    procedure FillErrorInfo() begin E.Message('m'); end;
    procedure FillTextBuilder() begin TB.Append('t'); end;
    procedure FillHttp() begin Req.Method('GET'); end;
    procedure FillHeaders() begin Req.GetHeaders(H); end;
    procedure FillCodeunit() begin C.Touch(); end;
    procedure FillVariant() begin V := 5; end;
    procedure Nothing() begin end;
}
codeunit 50112 Kinds2 { var M: Integer; procedure Touch() begin M := 1; end; }
"#;

    /// The shapes v1 of the image defers although the check renders them exactly (design decision 10): each its own
    /// counted reason in the `login image:` line, each a follow-up.
    const DEFERRED: [&str; 3] = [
        "JsonObject (deferred)",
        "JsonArray (deferred)",
        "XmlDocument (deferred)",
    ];
    const DEFERRED_HTTP: [&str; 2] = ["HttpRequestMessage (deferred)", "HttpHeaders (deferred)"];

    // The image refuses only what the login replay check has no exact form for: for each kind of value a SingleInstance
    // global holds, when `Session::state_lines` names nothing as opaque (state_render.rs, the one owner of which values
    // have an exact form), the image is taken, or refused with a v1 deferral named in `DEFERRED`. And when the check names
    // a value as opaque, the image refuses too.
    #[test]
    fn the_image_refuses_only_what_the_check_has_no_exact_form_for() {
        let l = al_testkit::compile(KINDS);
        let engine = Engine::new(l.program.clone(), al_testkit::blocked(&l));
        let at = engine.program.objects.iter().position(|o| o.name == "Kinds").unwrap();
        let fills = [
            "Nothing",
            "FillList",
            "FillDict",
            "FillArray",
            "FillTemp",
            "FillRecord",
            "FillRecordRef",
            "FillJson",
            "FillJsonArray",
            "FillXml",
            "FillNotification",
            "FillErrorInfo",
            "FillTextBuilder",
            "FillHttp",
            "FillHeaders",
            "FillCodeunit",
            "FillVariant",
        ];
        let mut seen = Vec::new();
        for fill in fills {
            let mut s = Session::new(engine.clone());
            let ri = s.engine.program.objects[at]
                .routines
                .iter()
                .position(|r| r.name == fill)
                .unwrap();
            let inst = s.codeunit_instance(at);
            s.call(at, ri, inst, vec![]).unwrap_or_else(|e| panic!("{fill}: {e:?}"));
            s.commit_transaction();
            let exact = s.state_lines().opaque.is_empty();
            let image = s.login_image(0);
            match (&image, exact) {
                (Ok(_), true) => {}
                (Err(why), true) => {
                    let why = why.reason();
                    assert!(
                        DEFERRED.contains(&why.as_str()) || DEFERRED_HTTP.contains(&why.as_str()),
                        "{fill}: the check renders it exactly, the image refuses: {why}"
                    );
                }
                (Ok(_), false) => panic!("{fill}: the check has no exact form for it, the image takes it"),
                (Err(_), false) => {}
            }
            seen.push((fill, image.err().map(|w| w.reason())));
        }
        // A value the check renders as opaque (an open stream) is refused by both.
        let mut s = Session::new(engine.clone());
        let inst = s.codeunit_instance(at);
        let stream = al_builtins::stream::InState::snapshot(b"x".to_vec(), Default::default());
        let slot = s.engine.program.objects[at]
            .globals
            .iter()
            .position(|g| g.name == "I")
            .unwrap();
        inst.borrow_mut().globals[slot] = Value::Library(
            al_builtins::library_kind::LibraryKind::InStream,
            std::rc::Rc::new(std::cell::RefCell::new(al_builtins::value::LibraryValue::InStream(
                stream,
            ))),
        );
        drop(inst);
        s.commit_transaction();
        assert!(!s.state_lines().opaque.is_empty());
        assert!(s.login_image(0).is_err());
        assert!(seen.iter().filter(|(_, why)| why.is_none()).count() >= 10, "{seen:?}");
    }

    // A DotNet object the runtime serves itself (`ALConfigSettings.Instance`, dotnet/server_classes.rs:57-62), which the
    // check renders exactly (state_render.rs: a `DotNetKind::Internal` has a form), is imaged: the twin holds its own
    // object of the same kind and state. The #8211 repro's login leaves one in a SingleInstance global.
    #[test]
    fn a_dotnet_object_the_runtime_serves_round_trips() {
        let l = al_testkit::compile(KINDS);
        let engine = Engine::new(l.program.clone(), al_testkit::blocked(&l));
        let at = engine.program.objects.iter().position(|o| o.name == "Kinds").unwrap();
        let slot = engine.program.objects[at]
            .globals
            .iter()
            .position(|g| g.name == "V")
            .unwrap();
        let mut s = Session::new(engine.clone());
        let original = std::rc::Rc::new(al_builtins::value::DotNetObject::new(
            al_builtins::value::DotNetKind::Internal {
                type_name: al_builtins::dotnet_sigs::AL_CONFIG_SETTINGS.into(),
                state: std::cell::RefCell::new(vec![Value::Integer(3)]),
            },
        ));
        let address = std::rc::Rc::as_ptr(&original);
        s.codeunit_instance(at).borrow_mut().globals[slot] = Value::DotNet(original);
        s.commit_transaction();
        let image = s.login_image(0).unwrap_or_else(|why| panic!("{}", why.reason()));
        let mut twin = Session::new(engine);
        twin.thaw_login_image(&image);
        let Value::DotNet(d) = twin.codeunit_instance(at).borrow().globals[slot].clone() else {
            panic!("a DotNet value")
        };
        assert_ne!(std::rc::Rc::as_ptr(&d), address, "the twin's own object");
        let al_builtins::value::DotNetKind::Internal { type_name, state } = &d.kind else {
            panic!("an internal object")
        };
        assert_eq!(type_name, al_builtins::dotnet_sigs::AL_CONFIG_SETTINGS);
        assert!(matches!(state.borrow()[..], [Value::Integer(3)]));
        // The test's own handle is one more owner: the check numbers a shared object.
        drop(d);
        let (ours, theirs) = (s.state_lines(), twin.state_lines());
        assert_eq!(
            ours.lines[..ours.lines.len() - 1],
            theirs.lines[..theirs.lines.len() - 1]
        );
    }

    const OUTSIDE: &str = r#"
table 2000000144 "Power BI Blob"
{
    fields { field(1; Id; Guid) { } field(2; Name; Text[200]) { } }
    keys { key(PK; Id) { } }
}
codeunit 50200 PKeep
{
    SingleInstance = true;
    var E: ErrorInfo; P: Record "Power BI Blob";
    procedure Empty() begin P.DeleteAll(); Commit(); end;
    procedure CountP(): Text begin exit(Format(P.Count())); end;
    procedure InsertP() begin P.Id := CreateGuid(); P.Name := 'x'; P.Insert(); Commit(); end;
    [ErrorBehavior(ErrorBehavior::Collect)]
    procedure Collect()
    var EI: ErrorInfo; L: List of [ErrorInfo];
    begin
        EI := ErrorInfo.Create('boom');
        EI.Collectible(true);
        Error(EI);
        L := GetCollectedErrors(true);
        E := L.Get(1);
    end;
    procedure Stack(): Text begin exit(E.Callstack()); end;
}
"#;

    /// `routine` of PKeep on `s`'s single instance: its value, or its error.
    fn outcome(s: &mut Session, keep: usize, routine: &str) -> String {
        let ri = s.engine.program.objects[keep]
            .routines
            .iter()
            .position(|r| r.name == routine)
            .unwrap();
        let inst = s.codeunit_instance(keep);
        match s.call(keep, ri, inst, vec![]) {
            Ok((v, _)) => format!("{v:?}"),
            Err(e) => format!("ERR {e:?}"),
        }
    }

    /// A session over `OUTSIDE` after `routines`, and its twin through the image.
    fn original_and_twin(routines: &[&str]) -> (Session, Session, usize) {
        let l = al_testkit::compile(OUTSIDE);
        let engine = Engine::new(l.program.clone(), al_testkit::blocked(&l));
        let keep = engine.program.objects.iter().position(|o| o.name == "PKeep").unwrap();
        let mut s = Session::new(engine.clone());
        for r in routines {
            outcome(&mut s, keep, r);
        }
        s.commit_transaction();
        let image = s.login_image(0).unwrap_or_else(|why| panic!("{}", why.reason()));
        let mut twin = Session::new(engine);
        twin.thaw_login_image(&image);
        (s, twin, keep)
    }

    /// The state lines without the SQL fact, which reads the thread's store operation count.
    fn lines(s: &mut Session) -> Vec<String> {
        s.state_lines()
            .lines
            .into_iter()
            .filter(|l| !l.starts_with("sql "))
            .collect()
    }

    // State kept outside `Session`'s fields, owner 1: the committed store's `emptied` marks. A login that emptied a table
    // whose rows BC holds but the runner does not know (records.rs `is_unseeded`) and then inserted a row reads the row;
    // the twin reads it too, and the check (`emptied` line) sees the mark. Review of step 2, H1.
    #[test]
    fn a_table_the_login_emptied_stays_emptied_in_the_twin() {
        let (mut s, mut twin, keep) = original_and_twin(&["Empty", "InsertP"]);
        assert_eq!(lines(&mut s), lines(&mut twin));
        let (a, b) = (outcome(&mut s, keep, "CountP"), outcome(&mut twin, keep, "CountP"));
        assert_eq!(a, b);
        assert!(a.contains('1'), "{a}");
    }

    // State kept outside `Session`'s fields, owner 2: an ErrorInfo's `Callstack`, in al-builtins errorinfo.rs `CALLSTACKS`
    // by the value's cell. The twin's ErrorInfo answers the same `Callstack`. Review of step 2, H2.
    #[test]
    fn an_error_infos_call_stack_follows_it_into_the_twin() {
        let (mut s, mut twin, keep) = original_and_twin(&["Collect"]);
        assert_eq!(lines(&mut s), lines(&mut twin));
        let (a, b) = (outcome(&mut s, keep, "Stack"), outcome(&mut twin, keep, "Stack"));
        assert!(a.contains("Collect"), "{a}");
        assert_eq!(a, b);
    }
}
