//! The `DotNet` provider seam (ADR-014 section 2).
//!
//! Plain data and two traits. Nothing here touches the OS: a front end that can start a
//! process (`al-test-cli`) implements [`DotNetHost`]; the VM only sees the trait.

use std::fmt;

/// What a `dotnet { }` block declares for one alias.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct DotNetType {
    pub assembly: String,
    pub version: Option<String>,
    pub culture: Option<String>,
    pub public_key_token: Option<String>,
    /// The CLR type name. Generic names keep the CLR spelling (``List`1[System.String]``).
    pub name: String,
}

impl fmt::Display for DotNetType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.name, self.assembly)
    }
}

/// A reference to an object that lives in the host. Unique per session, never reused.
pub type DotNetHandle = u64;

/// A value on the wire between the VM and the host (ADR-014 section 4).
#[derive(Debug, Clone, PartialEq)]
pub enum DotNetValue {
    Null,
    /// A null whose declared CLR type is `string` (ADR-024). BC reads it as an empty text in an
    /// expression and as an empty Variant when assigned; a null of any other type is `Null`.
    /// Only the host sends it, never the VM.
    NullString,
    Bool(bool),
    Byte(u8),
    Char(u16),
    Int32(i32),
    Int64(i64),
    /// Decimal as its exact text, so no float round trip.
    Decimal(String),
    /// Text and Code.
    Str(String),
    /// `System.DateTime` as ticks of 100 ns since 0001-01-01, kind unspecified.
    DateTime(i64),
    /// `System.TimeSpan` in ticks of 100 ns.
    TimeSpan(i64),
    Guid([u8; 16]),
    /// A .NET enum value: CLR type name, member name, underlying ordinal.
    Enum {
        ty: String,
        name: String,
        ordinal: i64,
    },
    /// An object held by the host; `ty` is its CLR type name.
    Handle {
        handle: DotNetHandle,
        ty: String,
    },
    /// An AL `InStream` handed to .NET: the unread bytes (section 4, streams).
    InStream(Vec<u8>),
    /// An AL `OutStream` handed to .NET: the host creates a writable bridge stream.
    OutStream,
}

/// An argument: a value plus whether the AL caller passed a variable that .NET may write back.
#[derive(Debug, Clone, PartialEq)]
pub struct DotNetArg {
    pub value: DotNetValue,
    pub by_ref: bool,
}

/// What a call is made on.
#[derive(Debug, Clone, PartialEq)]
pub enum DotNetTarget {
    Static(DotNetType),
    Instance(DotNetHandle),
    /// A primitive an AL `DotNet` variable holds (ADR-024): the host boxes it for the call.
    Value(DotNetValue),
}

/// What the host did with AL streams during one call (ADR-014 section 4, "Streams").
#[derive(Debug, Clone, PartialEq, Default)]
pub struct StreamEffects {
    /// Per `InStream` argument position: how many bytes .NET consumed.
    pub consumed: Vec<(usize, usize)>,
    /// `OutStream` arguments of this call, as (argument position, bridge id). The bridge lives
    /// until the session ends; the VM keeps the AL stream that belongs to the id.
    pub bridges: Vec<(usize, u64)>,
    /// Bytes written into any live bridge since the last reply, by bridge id.
    pub written: Vec<(u64, Vec<u8>)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DotNetReply {
    pub value: DotNetValue,
    /// New values of by-reference arguments, by argument position.
    pub by_ref: Vec<(usize, DotNetValue)>,
    pub streams: StreamEffects,
}

impl DotNetReply {
    pub fn value(value: DotNetValue) -> Self {
        DotNetReply {
            value,
            by_ref: Vec::new(),
            streams: StreamEffects::default(),
        }
    }
}

/// A .NET exception, with its inner exceptions (outermost first).
#[derive(Debug, Clone, PartialEq)]
pub struct DotNetException {
    pub clr_type: String,
    pub message: String,
    pub inner: Vec<(String, String)>,
}

impl fmt::Display for DotNetException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.clr_type, self.message)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DotNetFailure {
    /// The call cannot be served here: no .NET, type or member not found, sidecar died, ...
    Refused(String),
    /// The called .NET code threw.
    Exception(DotNetException),
}

impl fmt::Display for DotNetFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DotNetFailure::Refused(why) => f.write_str(why),
            DotNetFailure::Exception(e) => e.fmt(f),
        }
    }
}

/// Why the host cannot serve any call in this run. The text finishes the sentence
/// `DotNet 'T' ...` (ADR-014 section 11): `needs a .NET host; none is configured`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DotNetUnavailable(pub String);

impl fmt::Display for DotNetUnavailable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Per-session settings. Additive: new fields get defaults.
#[derive(Debug, Clone, Default)]
pub struct DotNetSessionConfig {}

pub trait DotNetSession {
    fn construct(&mut self, ty: &DotNetType, args: Vec<DotNetArg>) -> Result<DotNetReply, DotNetFailure>;
    fn invoke(
        &mut self,
        target: DotNetTarget,
        member: &str,
        args: Vec<DotNetArg>,
    ) -> Result<DotNetReply, DotNetFailure>;
    fn get_property(&mut self, target: DotNetTarget, name: &str) -> Result<DotNetReply, DotNetFailure>;
    fn set_property(
        &mut self,
        target: DotNetTarget,
        name: &str,
        value: DotNetValue,
    ) -> Result<DotNetReply, DotNetFailure>;
    /// Appends `bytes` to the .NET stream held under `handle` (ADR-024, `CopyStream` into a
    /// `DotNet` stream). The default refuses; a handle that is not a `System.IO.Stream` is
    /// refused with its CLR type.
    fn stream_write(&mut self, _handle: DotNetHandle, _bytes: &[u8]) -> Result<(), DotNetFailure> {
        Err(DotNetFailure::Refused("this host cannot copy a stream".into()))
    }
    /// Reads from the .NET stream held under `handle` (ADR-024, `CopyStream` out of a `DotNet`
    /// stream). A `MemoryStream` read without a count returns the whole stream whatever its position
    /// and leaves the position at 0; with a count it reads from the position and the position
    /// follows. The default refuses.
    fn stream_read(&mut self, _handle: DotNetHandle, _max: Option<usize>) -> Result<Vec<u8>, DotNetFailure> {
        Err(DotNetFailure::Refused("this host cannot copy a stream".into()))
    }
    /// Drops host objects. Called before the next call and when the session closes.
    fn release(&mut self, handles: &[DotNetHandle]);
    /// Ends the host side for this session (the sidecar process).
    fn close(self: Box<Self>);
}

pub trait DotNetHost: Send + Sync {
    /// Cheap and cached. `Err` means every `DotNet` call in this run is refused.
    fn availability(&self) -> Result<(), DotNetUnavailable>;
    /// One per VM session. Starts nothing until the first call.
    fn open_session(&self, cfg: &DotNetSessionConfig) -> Box<dyn DotNetSession>;
    /// What runs the calls, as text: the .NET runtime, the helper and where it finds assemblies. Two hosts with the
    /// same text answer a call that depends on its arguments alone the same (the key of the kept install baseline).
    /// `None`: the host cannot say.
    fn identity(&self) -> Option<String> {
        None
    }
}

/// The default host: no .NET. Core tests and a WASM build use it.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoDotNet;

impl DotNetHost for NoDotNet {
    fn availability(&self) -> Result<(), DotNetUnavailable> {
        Err(DotNetUnavailable("needs a .NET host; none is configured".into()))
    }
    fn open_session(&self, _cfg: &DotNetSessionConfig) -> Box<dyn DotNetSession> {
        Box::new(NoDotNetSession)
    }
}

struct NoDotNetSession;

impl NoDotNetSession {
    fn refuse<T>() -> Result<T, DotNetFailure> {
        Err(DotNetFailure::Refused("needs a .NET host; none is configured".into()))
    }
}

impl DotNetSession for NoDotNetSession {
    fn construct(&mut self, _: &DotNetType, _: Vec<DotNetArg>) -> Result<DotNetReply, DotNetFailure> {
        Self::refuse()
    }
    fn invoke(&mut self, _: DotNetTarget, _: &str, _: Vec<DotNetArg>) -> Result<DotNetReply, DotNetFailure> {
        Self::refuse()
    }
    fn get_property(&mut self, _: DotNetTarget, _: &str) -> Result<DotNetReply, DotNetFailure> {
        Self::refuse()
    }
    fn set_property(&mut self, _: DotNetTarget, _: &str, _: DotNetValue) -> Result<DotNetReply, DotNetFailure> {
        Self::refuse()
    }
    fn release(&mut self, _: &[DotNetHandle]) {}
    fn close(self: Box<Self>) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_host_is_unavailable_and_refuses_every_call() {
        let host = NoDotNet;
        assert_eq!(
            host.availability(),
            Err(DotNetUnavailable("needs a .NET host; none is configured".into()))
        );
        let mut s = host.open_session(&DotNetSessionConfig::default());
        let ty = DotNetType {
            assembly: "netstandard".into(),
            name: "System.Convert".into(),
            ..Default::default()
        };
        let want = Err(DotNetFailure::Refused("needs a .NET host; none is configured".into()));
        assert_eq!(
            s.invoke(DotNetTarget::Static(ty.clone()), "ToBase64String", vec![]),
            want
        );
        assert_eq!(s.construct(&ty, vec![]), want);
        assert_eq!(s.get_property(DotNetTarget::Instance(1), "Length"), want);
        s.release(&[1, 2]);
        s.close();
    }
}
