//! `DotNet` calls (ADR-014). The router: registry of runtime-provided types, then the host
//! provider, then refusal. Marshalling between AL values and the host's wire values lives here
//! too.

pub mod compression;
mod drawing;
mod gettype;
mod last_error;
pub mod policy;
pub mod registry;
pub mod server_classes;
mod query_metadata;
mod staged_dir;

use crate::{Frame, Session};
use al_builtins::describe::Describe;
use al_builtins::native::Signal;
use al_builtins::ops;
use al_builtins::stream::BlobRef;
use al_builtins::value::{DotNetKind, DotNetObject, LibraryValue, RtType, Value};
use al_host::dotnet::{
    DotNetArg, DotNetFailure, DotNetHandle, DotNetHost, DotNetReply, DotNetSession, DotNetSessionConfig, DotNetTarget,
    DotNetType, DotNetValue,
};
use al_ir as ir;
use policy::{Access, Policy};
use registry::{MemberKind, Registry};
use rust_decimal::Decimal;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

/// Marks a refusal that comes from the machine, not from the code under test: no .NET host, no
/// runtime, wrong major version, sidecar not built. The runner counts a baseline case refused
/// this way as skipped, not as a regression (ADR-014 section 11).
pub const UNAVAILABLE: &str = "dotnet-unavailable";

/// What the engine needs to serve `DotNet` calls. Shared by every session.
#[derive(Clone)]
pub struct DotNetConfig {
    pub host: Arc<dyn DotNetHost>,
    pub registry: Arc<Registry>,
    pub policy: Arc<Policy>,
}

impl Default for DotNetConfig {
    fn default() -> Self {
        DotNetConfig {
            host: Arc::new(al_host::dotnet::NoDotNet),
            registry: Arc::new(Registry::builtin()),
            policy: Arc::new(Policy::builtin()),
        }
    }
}

// The registry holds fn pointers and strings, the policy strings, the host is `Send + Sync`.
const _: fn() = || {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<DotNetConfig>();
};

/// A session's .NET side: the host session (opened at the first call that reaches the host),
/// the queue of dropped handles, and the AL streams behind the host's write bridges.
#[derive(Default)]
pub struct DotNetState {
    session: Option<Box<dyn DotNetSession>>,
    release: Rc<RefCell<Vec<DotNetHandle>>>,
    /// `availability()` of the host, asked once per session.
    available: Option<Result<(), String>>,
    bridges: HashMap<u64, BlobRef>,
    /// The last .NET result was a null `string` (ADR-024). Set by the reply of a host call and
    /// cleared at the start of every `DotNet` member, so a settle step that runs right after
    /// the member reads the declared type of the null it got.
    null_string: bool,
}

impl DotNetState {
    pub(crate) fn clear_null_string(&mut self) {
        self.null_string = false;
    }

    /// Hands dropped handles to the host. Called before every call and when the session closes.
    fn flush(&mut self) {
        let dropped: Vec<DotNetHandle> = std::mem::take(&mut *self.release.borrow_mut());
        if !dropped.is_empty() {
            if let Some(s) = &mut self.session {
                s.release(&dropped);
            }
        }
    }
}

impl Drop for DotNetState {
    fn drop(&mut self) {
        self.flush();
        if let Some(s) = self.session.take() {
            s.close();
        }
    }
}

/// What a call is made on, after looking at the receiver value.
enum Target {
    /// A static member (or a constructor) of the declared type.
    Static(DotNetType),
    /// An object the host holds.
    Host(DotNetHandle, String),
    /// An object the registry created.
    Internal(Rc<DotNetObject>, String),
    /// A primitive the variable holds, with the CLR type its kind boxes to (ADR-024).
    Boxed(DotNetValue, String),
}

/// Streams passed in one call, kept so the reply's effects can be applied.
#[derive(Default)]
struct StreamBook {
    ins: Vec<(usize, Rc<RefCell<LibraryValue>>)>,
    outs: Vec<(usize, BlobRef)>,
}

fn unsupported<T>(why: impl Into<String>) -> Result<T, Signal> {
    Err(Signal::Unsupported(why.into()))
}

const TICKS_PER_MS: i64 = 10_000;
const TICKS_PER_DAY: i64 = 864_000_000_000;

impl Session {
    /// A DotNet operation on evaluated operands (the bytecode's `DotNet` instruction).
    pub(crate) fn dotnet_apply(
        &mut self,
        ty: Option<usize>,
        recv: Value,
        mut values: Vec<Value>,
        places: Vec<Option<ir::Place>>,
        op: &ir::DotNetOp,
        f: &mut Frame,
    ) -> Result<Value, Signal> {
        self.note_dotnet();
        self.dotnet.null_string = false;
        let decl = ty.map(|i| {
            let d = &self.engine.program.dotnet_types[i];
            DotNetType {
                assembly: d.assembly.clone(),
                version: d.version.clone(),
                culture: d.culture.clone(),
                public_key_token: d.public_key_token.clone(),
                name: d.type_name.clone(),
            }
        });
        let (kind, member) = match op {
            ir::DotNetOp::Construct => (
                MemberKind::Construct,
                decl.as_ref().map_or(String::new(), |d| d.name.clone()),
            ),
            ir::DotNetOp::Call(n) => (MemberKind::Call, n.clone()),
            ir::DotNetOp::Get(n) => (MemberKind::Get, n.clone()),
            ir::DotNetOp::Set(n) => (MemberKind::Set, n.clone()),
        };
        let target = match (kind, &recv) {
            (MemberKind::Construct, _) => match &decl {
                Some(d) => Target::Static(d.clone()),
                None => return unsupported("DotNet constructor without a declared type"),
            },
            (_, Value::DotNet(o)) => match &o.kind {
                DotNetKind::Host { handle, clr_type } => Target::Host(*handle, clr_type.clone()),
                DotNetKind::Internal { type_name, .. } => Target::Internal(o.clone(), type_name.clone()),
                // An enum value is boxed for the call, as a primitive is: its members (`Equals`, `ToString`,
                // the static members of its type) are .NET's
                // (tests/al-language-onprem-dotnet/lt-dotnet-interop/XmlNodeTypeEquals.al, XmlNodeTypeFormat.al).
                DotNetKind::Enum {
                    clr_type,
                    name,
                    ordinal,
                } => Target::Boxed(
                    DotNetValue::Enum {
                        ty: clr_type.clone(),
                        name: name.clone(),
                        ordinal: *ordinal,
                    },
                    clr_type.clone(),
                ),
            },
            // `X.GetType()` on an unassigned `System.Object` variable raises in a codeunit
            // (tests/al-language-onprem-dotnet/lt-dotnet-interop/DotNetTypeOfDotNet.al:224 `Unassigned_error`, after
            // GetDotNetType(X) answered System.Object: lt-bb-dotnet/BbGetDotNetType.al:364).
            (MemberKind::Call, Value::Null)
                if member.eq_ignore_ascii_case("GetType")
                    && values.is_empty()
                    && decl.as_ref().is_some_and(|d| d.name == "System.Object") =>
            {
                let o = &self.engine.program.objects[f.object];
                if o.kind != al_ir::ObjKind::Codeunit || o.uncompiled {
                    return unsupported(
                        "GetType of an unassigned DotNet variable outside a codeunit (the object kind's spelling in the error is not pinned)",
                    );
                }
                let message = format!(
                    "A DotNet variable has not been instantiated. Attempting to call System.Object.GetType in CodeUnit {}: {}",
                    o.name, o.routines[f.routine].name
                );
                return al_builtins::native::raise(message, "DotNetVariableNotCreated");
            }
            (_, Value::Null) => match &decl {
                Some(d) => Target::Static(d.clone()),
                None => {
                    return unsupported(format!(
                        "DotNet member '{member}' on an unassigned DotNet value (the error BC raises is not pinned)"
                    ));
                }
            },
            // A primitive the variable holds (a `DotNet DateTime` that got a `DateTime`): the host boxes
            // it for the call (ADR-024).
            (_, other) => match boxed_clr_type(other) {
                Some(t) => {
                    let wire = self
                        .value_to_wire(other, 0, &mut StreamBook::default())
                        .map_err(|e| prefix(e, t))?;
                    Target::Boxed(wire, t.to_string())
                }
                None => return unsupported(format!("DotNet member '{member}' on {}", other.describe())),
            },
        };
        let clr_type = match &target {
            Target::Static(d) => d.name.clone(),
            Target::Host(_, t) | Target::Internal(_, t) | Target::Boxed(_, t) => t.clone(),
        };
        let shown = if kind == MemberKind::Construct {
            format!("new {clr_type}")
        } else {
            format!("{clr_type}.{member}")
        };

        // 1. The registry: a registered type is served here for all its members.
        let engine = self.engine.clone();
        if let Some(rt) = engine.dotnet.registry.get(&clr_type) {
            let Some(m) = rt.member(&member, kind) else {
                return unsupported(format!(
                    "DotNet {shown}: a member the runtime does not implement for a registry type (never forwarded to .NET, ADR-014 section 1)"
                ));
            };
            let this = match &target {
                Target::Internal(o, _) => Some(o),
                _ => None,
            };
            if let Some(call_ref) = m.call_ref {
                let result = call_ref(self, this, &mut values)?;
                for (i, place) in places.iter().enumerate() {
                    let Some(place) = place else { continue };
                    let ty = self.place_type(place, f);
                    let v = fit(values[i].clone(), &ty).map_err(|e| prefix(e, &shown))?;
                    self.write(place, v, f)?;
                }
                return Ok(result);
            }
            return (m.call)(self, this, &values);
        }

        // 1b. The clock members of DateTime and DateTimeOffset read the host clock, the one
        // `CurrentDateTime` reads, so a run is deterministic under a fixed clock (owner decision
        // 2026-10-14, docs/owner-review.md). Not a registry type: no .NET needed for DateTime.
        if let (Target::Static(d), true) = (&target, matches!(kind, MemberKind::Get | MemberKind::Call)) {
            if values.is_empty() {
                if let Some(r) = self.host_clock_member(d, &member, &shown, f) {
                    return r;
                }
            }
        }

        // 1c. A folder AL Runner staged itself for the company build (the DemoTool's resources): Create Getting Started
        // Data lists `TemporaryPath() + '..\WalkMeTour'` through File Management.GetServerDirectoryFilesList. Which
        // files the folder holds is a detail of the host's set-up that no test sees, so the listing is answered from the
        // staged folder, as Windows BC answers it (windows-check-results 2026-10-22 QX item 14: the files, without case,
        // in name order). Every other folder stays refused by the policy below.
        if let (Target::Static(d), MemberKind::Call, [Value::Text(p) | Value::Code(p)]) =
            (&target, kind, values.as_slice())
            && d.name.eq_ignore_ascii_case("System.IO.Directory")
            && member.eq_ignore_ascii_case("GetFiles")
            && let Some(files) = self.host.files.staged_folder_files(p)
        {
            return self.staged_file_array(files, &shown);
        }

        // 2. The host provider, after the policy (ADR-014 section 8).
        let assembly = match &target {
            Target::Static(d) => Some(d.assembly.as_str()),
            _ => None,
        };
        let access = if kind == MemberKind::Construct {
            Access::Construct(values.len())
        } else {
            Access::Member(&member)
        };
        if let Err(why) = engine.dotnet.policy.check(assembly, &clr_type, access) {
            return unsupported(format!("DotNet {shown}: {why}"));
        }
        if matches!(target, Target::Internal(..)) {
            return unsupported(format!(
                "DotNet {shown}: a runtime object whose type is no longer registered"
            ));
        }

        // Marshal before the session is borrowed.
        let mut book = StreamBook::default();
        let mut wire = Vec::with_capacity(values.len());
        for (i, v) in values.iter().enumerate() {
            wire.push(DotNetArg {
                value: self.value_to_wire(v, i, &mut book).map_err(|e| prefix(e, &shown))?,
                by_ref: places[i].is_some(),
            });
        }
        self.dotnet_ready(&clr_type)?;
        let dn = &mut self.dotnet;
        let session = dn.session.as_mut().expect("opened by dotnet_ready");
        let reply = match (kind, &target) {
            (MemberKind::Construct, Target::Static(d)) => session.construct(d, wire),
            (MemberKind::Set, _) => {
                let value = wire.into_iter().next().map(|a| a.value).unwrap_or(DotNetValue::Null);
                session.set_property(host_target(&target), &member, value)
            }
            (MemberKind::Get, _) => session.get_property(host_target(&target), &member),
            (_, _) => session.invoke(host_target(&target), &member, wire),
        };
        match reply {
            Ok(r) => self.apply_reply(r, &book, &places, &shown, f),
            Err(DotNetFailure::Refused(why)) => unsupported(format!("DotNet {shown}: {why}")),
            Err(DotNetFailure::Exception(e)) => {
                // Pinned for the MailAddress constructor only, exactly these two exception types:
                // corpus tests/al-language-onprem-dotnet/lt-af-dotnet/AfMailAddress.al:77 (Format),
                // :88 and :99 (Argument, Format). Any other exception stays a refusal (not pinned).
                if kind == MemberKind::Construct && clr_type.eq_ignore_ascii_case("System.Net.Mail.MailAddress") {
                    let code = match e.clr_type.as_str() {
                        "System.FormatException" => Some("DotNetInvoke:Format"),
                        "System.ArgumentException" => Some("DotNetInvoke:Argument"),
                        _ => None,
                    };
                    if let Some(code) = code {
                        return al_builtins::native::raise(
                            format!("A call to {clr_type} failed with this message: {}", e.message),
                            code,
                        );
                    }
                }
                // The runtime's .NET host has only the encodings .NET ships, no code pages (1252 and the like). Whether
                // BC's server provides them is not pinned (docs/probes-needed.md, "Encoding.GetEncoding with a code page"):
                // the refusal says that, instead of a .NET exception BC may never raise.
                if clr_type.eq_ignore_ascii_case("System.Text.Encoding")
                    && member.eq_ignore_ascii_case("GetEncoding")
                    && e.clr_type == "System.NotSupportedException"
                {
                    let page = match values.first() {
                        Some(Value::Integer(n)) => format!(" (code page {n})"),
                        _ => String::new(),
                    };
                    return unsupported(format!(
                        "DotNet {shown}: the runtime's .NET host has no code-page encodings{page}; whether Business Central \
                         provides them is not pinned (docs/probes-needed.md)"
                    ));
                }
                let inner: String = e.inner.iter().map(|(t, m)| format!(" <- {t}: {m}")).collect();
                unsupported(format!(
                    "DotNet {shown} raised the .NET exception {}: {}{inner}; the text AL would see is not pinned (ADR-014 section 7)",
                    e.clr_type, e.message
                ))
            }
        }
    }

    /// `GetEnumerator` of a .NET collection (the value `foreach_dotnet_next` steps), or the error of a variable that was
    /// never instantiated (`collection` is the loop's collection expression, a variable for that text).
    pub(crate) fn foreach_dotnet_start(
        &mut self,
        coll: Value,
        collection: Option<&ir::Expr>,
        f: &mut Frame,
    ) -> Result<Value, Signal> {
        let engine = self.engine.clone();
        let (handle, clr_type) = match &coll {
            Value::DotNet(o) => match &o.kind {
                DotNetKind::Host { handle, clr_type } => (*handle, clr_type.clone()),
                DotNetKind::Enum { clr_type, .. } => {
                    return unsupported(format!("foreach over a .NET enum value of {clr_type} (not pinned)"));
                }
                DotNetKind::Internal { type_name, .. } => {
                    return unsupported(format!("foreach over a runtime-provided {type_name} (not pinned)"));
                }
            },
            _ => {
                // Null: the variable was never instantiated. The text names the variable's CLR
                // type, the object and the procedure.
                let Some(ir::Expr::Load(place)) = collection else {
                    return unsupported("foreach over a null DotNet value that is not a variable");
                };
                let RtType::DotNet(i) = self.place_type(place, f) else {
                    return unsupported("foreach over a null that is not a DotNet variable");
                };
                let o = &engine.program.objects[f.object];
                let kind = match o.kind {
                    al_ir::ObjKind::Codeunit if !o.uncompiled => "CodeUnit",
                    _ => {
                        let other = o.kind;
                        return unsupported(format!(
                            "foreach over an uninstantiated DotNet variable in a {other} (the object kind's spelling in the error is not pinned)"
                        ));
                    }
                };
                let message = format!(
                    "A DotNet variable has not been instantiated. Attempting to call {}.GetEnumerator in {kind} {}: {}",
                    engine.program.dotnet_types[i as usize].type_name, o.name, o.routines[f.routine].name
                );
                return al_builtins::native::raise(message, "DotNetVariableNotCreated");
            }
        };
        if let Err(why) = engine
            .dotnet
            .policy
            .check(None, &clr_type, Access::Member("GetEnumerator"))
        {
            return unsupported(format!("DotNet {clr_type}.GetEnumerator: {why}"));
        }
        let enumerator = self
            .host_call(DotNetTarget::Instance(handle), "GetEnumerator", &clr_type, f)
            .map_err(|e| e.signal(&clr_type, "GetEnumerator"))?;
        let Value::DotNet(en) = &enumerator else {
            return unsupported(format!("{clr_type}.GetEnumerator did not return an object"));
        };
        if !matches!(&en.kind, DotNetKind::Host { .. }) {
            return unsupported(format!("{clr_type}.GetEnumerator did not return a host object"));
        }
        Ok(enumerator)
    }

    /// The next element of an enumerator, converted to the loop variable's type; `None` when the collection is used up.
    pub(crate) fn foreach_dotnet_next(
        &mut self,
        var: &ir::Place,
        enumerator: &Value,
        f: &mut Frame,
    ) -> Result<Option<Value>, Signal> {
        let var_ty = self.place_type(var, f);
        let Value::DotNet(en) = enumerator else {
            return unsupported("a foreach loop over a .NET collection that is not set up");
        };
        let DotNetKind::Host {
            handle: en_handle,
            clr_type: en_type,
        } = &en.kind
        else {
            return unsupported("a foreach loop over a .NET collection that is not set up");
        };
        let (en_handle, en_type) = (*en_handle, en_type.clone());
        let more = self.host_call(DotNetTarget::Instance(en_handle), "MoveNext", &en_type, f);
        let more = match more {
            Ok(v) => v,
            Err(MoveNextFailure::Exception(e)) if e.clr_type == "System.InvalidOperationException" => {
                return al_builtins::native::raise(
                    format!("A call to {en_type}.MoveNext failed with this message: {}", e.message),
                    "DotNetInvoke:InvalidOperation",
                );
            }
            Err(other) => return Err(other.signal(&en_type, "MoveNext")),
        };
        if !matches!(more, Value::Boolean(true)) {
            return Ok(None);
        }
        let current = self
            .host_get(en_handle, "Current", &en_type, f)
            .map_err(|e| e.signal(&en_type, "Current"))?;
        let item = match self.dotnet_coerce(current.clone(), &var_ty) {
            Ok(v) => v,
            Err(unfit) => {
                return Err(match (&current, &var_ty) {
                    (Value::Integer(_), RtType::Text(_) | RtType::Code(_)) => {
                        match al_builtins::native::raise::<()>(
                            "Invalid Conversion. Source: Int32, Destination: NavText.",
                            "Argument",
                        ) {
                            Err(e) => e,
                            Ok(()) => unreachable!(),
                        }
                    }
                    (Value::Text(_), RtType::Date) => {
                        match al_builtins::native::raise::<()>(
                            "Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime.",
                            "Conversion",
                        ) {
                            Err(e) => e,
                            Ok(()) => unreachable!(),
                        }
                    }
                    _ => prefix(unfit, "foreach element"),
                });
            }
        };
        Ok(Some(item))
    }

    /// One host call for `foreach`: `member` with no arguments on `target`.
    fn host_call(
        &mut self,
        target: DotNetTarget,
        member: &str,
        clr_type: &str,
        f: &mut Frame,
    ) -> Result<Value, MoveNextFailure> {
        self.dotnet_ready(clr_type).map_err(MoveNextFailure::Refused)?;
        let session = self.dotnet.session.as_mut().expect("opened by dotnet_ready");
        match session.invoke(target, member, Vec::new()) {
            Ok(r) => self
                .apply_reply(r, &StreamBook::default(), &[], member, f)
                .map_err(MoveNextFailure::Refused),
            Err(DotNetFailure::Refused(why)) => Err(MoveNextFailure::Refused(Signal::Unsupported(why))),
            Err(DotNetFailure::Exception(e)) => Err(MoveNextFailure::Exception(e)),
        }
    }

    fn host_get(
        &mut self,
        handle: DotNetHandle,
        member: &str,
        clr_type: &str,
        f: &mut Frame,
    ) -> Result<Value, MoveNextFailure> {
        self.dotnet_ready(clr_type).map_err(MoveNextFailure::Refused)?;
        let session = self.dotnet.session.as_mut().expect("opened by dotnet_ready");
        match session.get_property(DotNetTarget::Instance(handle), member) {
            Ok(r) => self
                .apply_reply(r, &StreamBook::default(), &[], member, f)
                .map_err(MoveNextFailure::Refused),
            Err(DotNetFailure::Refused(why)) => Err(MoveNextFailure::Refused(Signal::Unsupported(why))),
            Err(DotNetFailure::Exception(e)) => Err(MoveNextFailure::Exception(e)),
        }
    }

    /// `CopyStream` where the target or the source is a `DotNet` stream (ADR-024 section 3). The
    /// bytes go through the host: the sidecar cannot see Rust memory.
    ///
    /// - AL `InStream` into a .NET stream: all the unread bytes; the AL position goes to the end
    ///   (tests/al-language-onprem-dotnet/TestDotNetValueKindsContracts.al, codeunit 61400,
    ///   `CopyStream_WithAMemoryStream`, with the same codeunit pinning the .NET position at the end
    ///   and a stream read by one byte copying the remaining 10). A byte count is not pinned for
    ///   this direction.
    /// - .NET `MemoryStream` into an AL `OutStream`: the whole stream whatever its position, position 0
    ///   afterwards; with a count, from the position, which follows the count (same test).
    pub(crate) fn copy_stream_with_dotnet(
        &mut self,
        out: &Value,
        src: &Value,
        count: Option<i64>,
    ) -> Result<(), Signal> {
        let engine = self.engine.clone();
        if let Some(r) = compression::write_into(out, src, count) {
            return r;
        }
        let host_stream = |v: &Value| -> Result<Option<(DotNetHandle, String)>, Signal> {
            match v {
                Value::DotNet(o) => match &o.kind {
                    DotNetKind::Host { handle, clr_type } => Ok(Some((*handle, clr_type.clone()))),
                    _ => unsupported("CopyStream with a .NET value that is not a stream the host holds"),
                },
                _ => Ok(None),
            }
        };
        match (host_stream(out)?, host_stream(src)?) {
            (Some(_), Some(_)) => unsupported("CopyStream between two DotNet streams (not pinned)"),
            (Some((handle, clr_type)), None) => {
                let Value::Library(_, h) = src else {
                    return unsupported(format!("CopyStream from {} into a DotNet stream", src.describe()));
                };
                if count.is_some() {
                    return unsupported("CopyStream with a byte count into a DotNet stream (not pinned)");
                }
                if let Err(why) = engine.dotnet.policy.check(None, &clr_type, Access::Member("Write")) {
                    return unsupported(format!("DotNet {clr_type}: {why}"));
                }
                let bytes = match &*h.borrow() {
                    LibraryValue::InStream(st) => st.rest(),
                    other => return unsupported(format!("CopyStream from {} into a DotNet stream", other.describe())),
                };
                self.dotnet_ready(&clr_type)?;
                let session = self.dotnet.session.as_mut().expect("opened by dotnet_ready");
                stream_failure(&clr_type, session.stream_write(handle, &bytes))?;
                if let LibraryValue::InStream(st) = &mut *h.borrow_mut() {
                    st.pos = st.len();
                }
                Ok(())
            }
            (None, Some((handle, clr_type))) => {
                let Value::Library(_, h) = out else {
                    return unsupported(format!("CopyStream from a DotNet stream into {}", out.describe()));
                };
                if let Err(why) = engine.dotnet.policy.check(None, &clr_type, Access::Member("Read")) {
                    return unsupported(format!("DotNet {clr_type}: {why}"));
                }
                let blob = match &*h.borrow() {
                    LibraryValue::OutStream(blob, _) => blob.clone(),
                    other => {
                        return unsupported(format!("CopyStream from a DotNet stream into {}", other.describe()));
                    }
                };
                self.dotnet_ready(&clr_type)?;
                let session = self.dotnet.session.as_mut().expect("opened by dotnet_ready");
                let max = count.map(|n| n as usize);
                let bytes = stream_failure(&clr_type, session.stream_read(handle, max))?;
                al_builtins::stream::write_bytes(&blob, &bytes);
                Ok(())
            }
            (None, None) => unsupported("CopyStream without a DotNet stream reached the DotNet path"),
        }
    }

    /// `Format` of a `DotNet` object: its `ToString()`; of an enum, its member name.
    pub(crate) fn format_dotnet_value(&mut self, v: &Value) -> Option<Result<String, Signal>> {
        let Value::DotNet(o) = v else { return None };
        Some(match &o.kind {
            DotNetKind::Enum { name, .. } => Ok(name.clone()),
            DotNetKind::Internal { type_name, .. } if type_name == al_builtins::dotnet_sigs::NULL_EXCEPTION => {
                Ok("null".into())
            }
            // A last-error exception: Format is ToString (GetLastErrorObject.al:85).
            DotNetKind::Internal { type_name, .. } if last_error::is_exception(type_name) => {
                last_error::exception_text(o)
            }
            DotNetKind::Internal { type_name, .. } => unsupported(format!("Format of a runtime-provided {type_name}")),
            DotNetKind::Host { handle, clr_type } => {
                let shown = format!("{clr_type}.ToString");
                let engine = self.engine.clone();
                if let Err(why) = engine.dotnet.policy.check(None, clr_type, Access::Member("ToString")) {
                    return Some(unsupported(format!("DotNet {shown}: {why}")));
                }
                if let Err(e) = self.dotnet_ready(clr_type) {
                    return Some(Err(e));
                }
                let session = self.dotnet.session.as_mut().expect("opened by dotnet_ready");
                match session.invoke(DotNetTarget::Instance(*handle), "ToString", Vec::new()) {
                    Ok(DotNetReply {
                        value: DotNetValue::Str(s),
                        ..
                    }) => Ok(s),
                    Ok(_) => unsupported(format!("DotNet {shown} did not return a string")),
                    Err(DotNetFailure::Refused(why)) => unsupported(format!("DotNet {shown}: {why}")),
                    Err(DotNetFailure::Exception(e)) => unsupported(format!(
                        "DotNet {shown} raised the .NET exception {}: {}",
                        e.clr_type, e.message
                    )),
                }
            }
        })
    }

    /// `System.DateTime.Now|UtcNow|Today` and `System.DateTimeOffset.Now|UtcNow` from the host
    /// clock, and `System.Guid.NewGuid` from the host's Guids. The session time zone is UTC (al-host), so `Now` and `UtcNow` agree and the offset is
    /// zero. A `DateTime` comes back by value, as the marshalling table has it; a `DateTimeOffset`
    /// is an object, so the host builds it from the clock's ticks. `None`: not a clock member.
    fn host_clock_member(
        &mut self,
        d: &DotNetType,
        member: &str,
        shown: &str,
        f: &mut Frame,
    ) -> Option<Result<Value, Signal>> {
        let is = |m: &str| member.eq_ignore_ascii_case(m);
        // A read of the .NET clock moves the one session clock on, as `CurrentDateTime` does (ADR-045).
        let reads_clock = matches!(d.name.as_str(), "System.DateTime" | "System.DateTimeOffset")
            && (is("Now") || is("UtcNow") || is("Today"));
        let at = if reads_clock {
            self.read_clock_millis()
        } else {
            self.now_millis()
        };
        let ticks = || -> i64 {
            let Value::DateTime(v) = crate::hostvalues::datetime_of_unix_millis(at) else {
                unreachable!("datetime_of_unix_millis builds a DateTime")
            };
            (v - 1) * TICKS_PER_MS
        };
        match d.name.as_str() {
            "System.DateTime" if is("Now") || is("UtcNow") => Some(Ok(Value::DateTime(ticks() / TICKS_PER_MS + 1))),
            // `Guid.NewGuid()` is the host's fresh Guid, the one `CreateGuid` gives, so a run stays repeatable. The corpus pins
            // that the result is a System.Guid (tests/al-language-onprem-dotnet/lt-dotnet-interop/DotNetTypeOfDotNet.al:`TryGuid`),
            // not that it differs from the next one; a version 4 Guid does.
            "System.Guid" if is("NewGuid") => Some(Ok(Value::Guid(self.fresh_guid()))),
            "System.DateTime" if is("Today") => {
                let midnight = ticks().div_euclid(TICKS_PER_DAY) * TICKS_PER_DAY;
                Some(Ok(Value::DateTime(midnight / TICKS_PER_MS + 1)))
            }
            "System.DateTimeOffset" if is("Now") || is("UtcNow") => {
                let wire = vec![
                    DotNetArg {
                        value: DotNetValue::Int64(ticks()),
                        by_ref: false,
                    },
                    DotNetArg {
                        value: DotNetValue::TimeSpan(0),
                        by_ref: false,
                    },
                ];
                if let Err(e) = self.dotnet_ready(&d.name) {
                    return Some(Err(e));
                }
                let session = self.dotnet.session.as_mut().expect("opened by dotnet_ready");
                Some(match session.construct(d, wire) {
                    Ok(r) => self.apply_reply(r, &StreamBook::default(), &[], shown, f),
                    Err(DotNetFailure::Refused(why)) => unsupported(format!("DotNet {shown}: {why}")),
                    Err(DotNetFailure::Exception(e)) => unsupported(format!(
                        "DotNet {shown} raised the .NET exception {}: {}",
                        e.clr_type, e.message
                    )),
                })
            }
            _ => None,
        }
    }

    /// Makes the host ready for a call: asks `availability()` once, opens the session at the
    /// first call, and flushes the release queue (ADR-014 sections 2 and 6).
    /// The `string[]` .NET returns from `Directory.GetFiles` for `files`, made by .NET itself (`Regex.Split` of the
    /// paths joined with a line feed), so its members are .NET's.
    fn staged_file_array(&mut self, files: Vec<String>, shown: &str) -> Result<Value, Signal> {
        if files.is_empty() || files.iter().any(|f| f.contains('\n')) {
            return unsupported(format!(
                "DotNet {shown}: a staged folder that is empty or holds a name with a line feed"
            ));
        }
        self.dotnet_ready("System.Text.RegularExpressions.Regex")?;
        let decl = DotNetType {
            assembly: "System.Text.RegularExpressions".into(),
            version: None,
            culture: None,
            public_key_token: None,
            name: "System.Text.RegularExpressions.Regex".into(),
        };
        let args = [files.join("\n"), "\n".to_string()]
            .into_iter()
            .map(|s| DotNetArg {
                value: DotNetValue::Str(s),
                by_ref: false,
            })
            .collect();
        let session = self.dotnet.session.as_mut().expect("opened by dotnet_ready");
        let reply = session.invoke(DotNetTarget::Static(decl), "Split", args);
        self.type_reply(reply, shown)
    }

    fn dotnet_ready(&mut self, clr_type: &str) -> Result<(), Signal> {
        let engine = self.engine.clone();
        let avail = self
            .dotnet
            .available
            .get_or_insert_with(|| engine.dotnet.host.availability().map_err(|e| e.0));
        if let Err(reason) = avail {
            return unsupported(format!("{UNAVAILABLE}: DotNet '{clr_type}' {reason}"));
        }
        if self.dotnet.session.is_none() {
            self.dotnet.session = Some(engine.dotnet.host.open_session(&DotNetSessionConfig::default()));
        }
        self.dotnet.flush();
        Ok(())
    }

    fn apply_reply(
        &mut self,
        reply: DotNetReply,
        book: &StreamBook,
        places: &[Option<ir::Place>],
        shown: &str,
        f: &mut Frame,
    ) -> Result<Value, Signal> {
        // Streams first: the bytes .NET read, the new bridges, the bytes written into any bridge.
        for (idx, n) in &reply.streams.consumed {
            if let Some((_, h)) = book.ins.iter().find(|(i, _)| i == idx) {
                if let LibraryValue::InStream(st) = &mut *h.borrow_mut() {
                    st.pos = (st.pos + n).min(st.len());
                }
            }
        }
        for (idx, id) in &reply.streams.bridges {
            if let Some((_, blob)) = book.outs.iter().find(|(i, _)| i == idx) {
                self.dotnet.bridges.insert(*id, blob.clone());
            }
        }
        for (id, bytes) in &reply.streams.written {
            match self.dotnet.bridges.get(id) {
                Some(blob) => al_builtins::stream::write_bytes(blob, bytes),
                None => return unsupported(format!("DotNet {shown}: bytes for an unknown stream bridge {id}")),
            }
        }
        self.dotnet.null_string = matches!(reply.value, DotNetValue::NullString);
        let value = self.wire_to_value(reply.value).map_err(|e| prefix(e, shown))?;
        for (idx, v) in reply.by_ref {
            let Some(Some(place)) = places.get(idx) else { continue };
            let v = self.wire_to_value(v).map_err(|e| prefix(e, shown))?;
            let ty = self.place_type(place, f);
            let v = fit(v, &ty).map_err(|e| prefix(e, shown))?;
            self.write(place, v, f)?;
        }
        Ok(value)
    }

    /// An AL value as a wire value (ADR-014 section 4).
    fn value_to_wire(&self, v: &Value, arg: usize, book: &mut StreamBook) -> Result<DotNetValue, Signal> {
        Ok(match v {
            Value::Null => DotNetValue::Null,
            Value::Boolean(b) => DotNetValue::Bool(*b),
            Value::Integer(i) => DotNetValue::Int32(*i),
            Value::BigInteger(i) => DotNetValue::Int64(*i),
            Value::Byte(b) => DotNetValue::Byte(*b),
            Value::Char(c) => DotNetValue::Char(*c),
            Value::Decimal(d) => DotNetValue::Decimal(d.to_string()),
            Value::Text(s) | Value::Code(s) => DotNetValue::Str(s.to_string()),
            // A Date or DateTime is a .NET DateTime of unspecified kind, with no time zone shift;
            // the kind and zone rule is a probe (docs/probes-needed.md).
            Value::Date(0, _) | Value::DateTime(0) => {
                return unsupported("an undefined Date or DateTime passed to .NET (what BC sends is not pinned)");
            }
            Value::Date(_, true) => return unsupported("a closing date passed to .NET (not pinned)"),
            Value::Date(d, false) => DotNetValue::DateTime((i64::from(*d) - 1) * TICKS_PER_DAY),
            Value::DateTime(ms) => DotNetValue::DateTime((ms - 1) * TICKS_PER_MS),
            Value::Time(_) => return unsupported("a Time passed to .NET (how BC maps it to DateTime is not pinned)"),
            Value::Duration(ms) => DotNetValue::TimeSpan(ms * TICKS_PER_MS),
            Value::Guid(g) => DotNetValue::Guid(*g),
            // A .NET enum parameter takes the ordinal; the host converts to the parameter's enum.
            Value::Option(o, _) => DotNetValue::Int32(*o),
            Value::Enum(_, o) => DotNetValue::Int32(*o),
            Value::DotNet(o) => match &o.kind {
                DotNetKind::Host { handle, clr_type } => DotNetValue::Handle {
                    handle: *handle,
                    ty: clr_type.clone(),
                },
                DotNetKind::Enum {
                    clr_type,
                    name,
                    ordinal,
                } => DotNetValue::Enum {
                    ty: clr_type.clone(),
                    name: name.clone(),
                    ordinal: *ordinal,
                },
                DotNetKind::Internal { type_name, .. } => {
                    return unsupported(format!("a runtime-provided {type_name} object passed to .NET code"));
                }
            },
            Value::Library(_, h) => match &*h.borrow() {
                LibraryValue::InStream(st) => {
                    book.ins.push((arg, h.clone()));
                    DotNetValue::InStream(st.rest())
                }
                LibraryValue::OutStream(blob, _) => {
                    book.outs.push((arg, blob.clone()));
                    DotNetValue::OutStream
                }
                other => return unsupported(format!("an AL library value ({}) passed to .NET", other.describe())),
            },
            other => {
                return unsupported(format!(
                    "{other} passed to .NET: the type has no marshalling row (ADR-014 section 4)",
                    other = other.describe()
                ));
            }
        })
    }

    /// A wire value as an AL value.
    fn wire_to_value(&self, v: DotNetValue) -> Result<Value, Signal> {
        Ok(match v {
            DotNetValue::Null | DotNetValue::NullString => Value::Null,
            DotNetValue::Bool(b) => Value::Boolean(b),
            DotNetValue::Byte(b) => Value::Byte(b),
            DotNetValue::Char(c) => Value::Char(c),
            DotNetValue::Int32(i) => Value::Integer(i),
            DotNetValue::Int64(i) => Value::BigInteger(i),
            DotNetValue::Decimal(s) => match s.parse::<Decimal>() {
                Ok(d) => Value::Decimal(d),
                Err(_) => return unsupported(format!("a .NET decimal '{s}' does not fit an AL Decimal")),
            },
            DotNetValue::Str(s) => Value::Text(s.into()),
            DotNetValue::DateTime(ticks) => Value::DateTime(ticks.div_euclid(TICKS_PER_MS) + 1),
            DotNetValue::TimeSpan(ticks) => Value::Duration(ticks.div_euclid(TICKS_PER_MS)),
            DotNetValue::Guid(g) => Value::Guid(g),
            DotNetValue::Enum { ty, name, ordinal } => Value::DotNet(Rc::new(DotNetObject::new(DotNetKind::Enum {
                clr_type: ty,
                name,
                ordinal,
            }))),
            DotNetValue::Handle { handle, ty } => {
                Value::DotNet(Rc::new(DotNetObject::host(handle, ty, self.dotnet.release.clone())))
            }
            DotNetValue::InStream(_) | DotNetValue::OutStream => {
                return unsupported("the host returned an AL stream value");
            }
        })
    }

    /// `Expr::DotNetCoerce`: a .NET result stored into a slot of type `t`.
    pub(crate) fn dotnet_coerce(&mut self, v: Value, t: &RtType) -> Result<Value, Signal> {
        // A null `string` assigned to a Text reads as the empty text, to a Variant as the empty
        // Variant (tests/al-language-onprem-dotnet/TestDotNetValueKindsContracts.al, codeunit
        // 61400, `NullStringResult_IsEmptyInExpressions`: `Text := <null string>` has length 0 and
        // `Value := <null string>` is neither Text nor DotNet).
        // A String stored into a SecretText is its content, a null string an empty SecretText
        // (TestDotNetValueKindsContracts.al:463, QA_StringResult_IntoSecretText). Another kind is not pinned (`fit`).
        if matches!(t, RtType::Library(k) if k.as_str() == "SecretText") {
            let content = match &v {
                Value::Null if self.dotnet.null_string => Some(String::new()),
                Value::Text(s) => Some(s.to_string()),
                _ => None,
            };
            if let Some(content) = content {
                return Ok(Value::Library(
                    "SecretText".into(),
                    std::rc::Rc::new(std::cell::RefCell::new(al_builtins::value::LibraryValue::Text(content))),
                ));
            }
        }
        if matches!(v, Value::Null) && self.dotnet.null_string {
            return match t {
                RtType::Text(_) | RtType::Code(_) => Ok(Value::Text(String::new().into())),
                _ => fit(v, t),
            };
        }
        fit(v, t)
    }

    /// `Expr::DotNetOperand`: the run-time kind of a `DotNet` value says what it is (ADR-024).
    /// The kinds the host returns by value are the AL values of the marshalling table
    /// (tests/al-language-onprem-dotnet/TestDotNetValueKindsContracts.al, codeunit 61400,
    /// `ClrResults_ReadAsTheirAlKind`). An object is refused naming its CLR type; a null object is
    /// refused too, because `StrLen` and `Format` of one raise an error `asserterror` cannot catch
    /// and the other uses are not pinned.
    pub(crate) fn settle_operand(&self, v: Value, how: ir::OperandUse) -> Result<Value, Signal> {
        use ir::OperandUse as U;
        match v {
            Value::Boolean(_)
            | Value::Integer(_)
            | Value::BigInteger(_)
            | Value::Decimal(_)
            | Value::Byte(_)
            | Value::Char(_)
            | Value::Date(..)
            | Value::Time(_)
            | Value::DateTime(_)
            | Value::Duration(_)
            | Value::Guid(_) => Ok(v),
            Value::Text(_) | Value::Code(_) => Ok(v),
            Value::Null if self.dotnet.null_string => match how {
                U::Variant => Ok(Value::Null),
                U::Strict => unsupported(
                    "a null .NET string passed to a builtin: BC raises an error that asserterror cannot catch (not modeled)",
                ),
                _ => Ok(Value::Text(String::new().into())),
            },
            Value::Null => unsupported(
                "a null .NET object used as an operand: BC raises an error that asserterror cannot catch for Format and StrLen, and the other uses are not pinned",
            ),
            Value::DotNet(o) => match (&o.kind, how) {
                (_, U::Variant) => Ok(Value::DotNet(o)),
                // Order operators keep an enum an enum: `dotnet_order` compares members of one enum by number
                // (tests/al-language-onprem-dotnet/lt-dotnet-interop/XmlNodeTypeOrder.al).
                (DotNetKind::Enum { .. }, U::Order) => Ok(Value::DotNet(o)),
                (DotNetKind::Enum { ordinal, .. }, U::Enum) => Ok(match i32::try_from(*ordinal) {
                    Ok(i) => Value::Integer(i),
                    Err(_) => Value::BigInteger(*ordinal),
                }),
                (DotNetKind::Enum { clr_type, .. }, _) => unsupported(format!(
                    "a .NET enum of CLR type {clr_type} used with an operator other than +, = and <> (not pinned)"
                )),
                (DotNetKind::Host { clr_type, .. }, _) => unsupported(format!(
                    "a DotNet object of CLR type {clr_type} used as an operand: only primitives and enums are operands (ADR-024)"
                )),
                (DotNetKind::Internal { type_name, .. }, _) => unsupported(format!(
                    "a DotNet object of CLR type {type_name} used as an operand: only primitives and enums are operands (ADR-024)"
                )),
            },
            other => unsupported(format!("{} used as a DotNet operand", other.describe())),
        }
    }
}

/// `=` with a settled `DotNet` operand: two texts compare as CLR strings, so by case
/// (tests/al-language-onprem-dotnet/TestDotNetValueKindsContracts.al, codeunit 61400,
/// `IntegerProperties_TakePartInIntegerExpressions_AndStringCompareIsCaseSensitive`); any other
/// pair compares as AL does.
/// `<`, `<=`, `>`, `>=` with a settled `DotNet` operand. Two members of one .NET enum order by number
/// (tests/al-language-onprem-dotnet/lt-dotnet-interop/XmlNodeTypeOrder.al: `Element < Attribute`,
/// `Attribute > Element`, `Element <= Element` are Yes). Two texts order as .NET orders them only when
/// that is certain: the corpus pins lower case ASCII words (DotNetOperandStringLess.al, 61407), where an ordinal
/// and a culture order agree, so any text with another character, or where the ordinal order and AL's
/// case-insensitive order differ, is refused. Everything else orders as AL does.
pub(crate) fn dotnet_order(l: &Value, r: &Value) -> Result<std::cmp::Ordering, Signal> {
    match (l, r) {
        (Value::DotNet(a), Value::DotNet(b)) => match (&a.kind, &b.kind) {
            (
                DotNetKind::Enum {
                    clr_type: ta,
                    ordinal: oa,
                    ..
                },
                DotNetKind::Enum {
                    clr_type: tb,
                    ordinal: ob,
                    ..
                },
            ) if ta == tb => Ok(oa.cmp(ob)),
            _ => unsupported("ordering two .NET values that are not members of one enum (not pinned)"),
        },
        (Value::DotNet(_), _) | (_, Value::DotNet(_)) => {
            unsupported("ordering a .NET enum against a value that is not a member of it (not pinned)")
        }
        (Value::Text(a) | Value::Code(a), Value::Text(b) | Value::Code(b)) => {
            let plain = |s: &str| s.bytes().all(|c| c.is_ascii_alphanumeric());
            let ordinal = a.as_str().cmp(b.as_str());
            if plain(a.as_str()) && plain(b.as_str()) && ordinal == a.to_ascii_lowercase().cmp(&b.to_ascii_lowercase())
            {
                Ok(ordinal)
            } else {
                unsupported("ordering .NET strings whose order is not the same under every reading (not pinned)")
            }
        }
        _ => ops::compare_operator(l, r, true),
    }
}

pub(crate) fn dotnet_equal(l: &Value, r: &Value) -> Result<bool, Signal> {
    match (l, r) {
        (Value::Text(a) | Value::Code(a), Value::Text(b) | Value::Code(b)) => Ok(a == b),
        _ => Ok(ops::compare(l, r)? == std::cmp::Ordering::Equal),
    }
}

/// Why a host call of a `foreach` failed: the host refused it, or the .NET code threw.
enum MoveNextFailure {
    Refused(Signal),
    Exception(al_host::dotnet::DotNetException),
}

impl MoveNextFailure {
    fn signal(self, clr_type: &str, member: &str) -> Signal {
        match self {
            MoveNextFailure::Refused(s) => s,
            MoveNextFailure::Exception(e) => Signal::Unsupported(format!(
                "DotNet {clr_type}.{member} raised the .NET exception {}: {}; the text AL would see is not pinned (ADR-014 section 7)",
                e.clr_type, e.message
            )),
        }
    }
}

fn stream_failure<T>(clr_type: &str, r: Result<T, DotNetFailure>) -> Result<T, Signal> {
    match r {
        Ok(v) => Ok(v),
        Err(DotNetFailure::Refused(why)) => unsupported(format!("DotNet CopyStream on {clr_type}: {why}")),
        Err(DotNetFailure::Exception(e)) => unsupported(format!(
            "DotNet CopyStream on {clr_type} raised the .NET exception {}: {}; the text AL would see is not pinned (ADR-014 section 7)",
            e.clr_type, e.message
        )),
    }
}

/// The CLR type an AL primitive boxes to when a `DotNet` variable holds it.
fn boxed_clr_type(v: &Value) -> Option<&'static str> {
    Some(match v {
        Value::Boolean(_) => "System.Boolean",
        Value::Integer(_) => "System.Int32",
        Value::BigInteger(_) => "System.Int64",
        Value::Decimal(_) => "System.Decimal",
        Value::Byte(_) => "System.Byte",
        Value::Char(_) => "System.Char",
        Value::Text(_) | Value::Code(_) => "System.String",
        Value::DateTime(_) => "System.DateTime",
        Value::Duration(_) => "System.TimeSpan",
        Value::Guid(_) => "System.Guid",
        _ => return None,
    })
}

fn host_target(t: &Target) -> DotNetTarget {
    match t {
        Target::Static(d) => DotNetTarget::Static(d.clone()),
        Target::Host(h, _) => DotNetTarget::Instance(*h),
        Target::Boxed(v, _) => DotNetTarget::Value(v.clone()),
        Target::Internal(..) => unreachable!("registry objects never reach the host"),
    }
}

fn prefix(e: Signal, shown: &str) -> Signal {
    match e {
        Signal::Unsupported(s) => Signal::Unsupported(format!("DotNet {shown}: {s}")),
        other => other,
    }
}

/// The kind of a value, for messages.
fn kind_of(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Boolean(_) => "Boolean",
        Value::Integer(_) => "Integer",
        Value::BigInteger(_) => "BigInteger",
        Value::Decimal(_) => "Decimal",
        Value::Byte(_) => "Byte",
        Value::Char(_) => "Char",
        Value::Text(_) => "Text",
        Value::Code(_) => "Code",
        Value::Date(..) => "Date",
        Value::Time(_) => "Time",
        Value::DateTime(_) => "DateTime",
        Value::Duration(_) => "Duration",
        Value::Guid(_) => "Guid",
        Value::DotNet(_) => "DotNet object",
        _ => "value",
    }
}

/// Stores a .NET result into a slot of type `t`. The kinds must fit exactly (Text and Code,
/// numbers widening); BC's conversion of a .NET result to a differently typed AL variable is not
/// pinned, so anything else is Unsupported (ADR-014 section 4).
pub(crate) fn fit(v: Value, t: &RtType) -> Result<Value, Signal> {
    use RtType as T;
    let ok = matches!(
        (&v, t),
        (_, T::Variant)
            | (Value::DotNet(_) | Value::Null, T::DotNet(_))
            | (Value::Text(_) | Value::Code(_), T::Text(_) | T::Code(_))
            | (Value::Boolean(_), T::Boolean)
            | (Value::Integer(_) | Value::Byte(_), T::Integer)
            | (Value::Integer(_) | Value::BigInteger(_) | Value::Byte(_), T::BigInteger)
            | (
                Value::Integer(_) | Value::BigInteger(_) | Value::Byte(_) | Value::Decimal(_),
                T::Decimal
            )
            | (Value::Byte(_), T::Byte)
            | (Value::Char(_), T::Char)
            | (Value::Date(..), T::Date)
            | (Value::DateTime(_), T::DateTime)
            | (Value::Time(_), T::Time)
            | (Value::Duration(_), T::Duration)
            | (Value::Guid(_), T::Guid)
    ) || (matches!(t, T::DotNet(_)) && boxed_clr_type(&v).is_some());
    if !ok {
        return unsupported(format!(
            "a .NET result of kind {} stored into {t}: BC's conversion is not pinned (ADR-014 section 4)",
            kind_of(&v),
            t = t.describe()
        ));
    }
    match (&v, t) {
        // A DotNet variable holds the primitive a member returned (ADR-024).
        (_, T::DotNet(_)) => Ok(v),
        _ => ops::convert(v, t),
    }
}
