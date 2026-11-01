//! A `DotNet` System.IO.MemoryStream passed straight to the InStream parameter of XmlDocument.ReadFrom, Media.ImportStream and
//! JsonObject.ReadFrom (gap G5-22; the SMTP API and Recommended Apps of the System Application and the Base App). Pinned by
//! tests/al-language-onprem-dotnet/lt-y51-dotnet-builtin-args/Y51DotNetBuiltinArgs.al (BC 27.5, 28.5, 29.0):
//! - the builtin reads the WHOLE stream whatever its Position (`pos3` gives the same document as `pos0`);
//! - XmlDocument.ReadFrom leaves the Position at the end, also when the text is not XML (`pos_after_*`);
//! - Media.ImportStream leaves it at the end when it imports (`text/plain`, from Position 4 too); a `image/png` of bytes that are no
//!   picture imports nothing on BC 27 and 28 and leaves the Position at 0 (pinned from Position 0 only), and imports on BC 29;
//! - JsonObject.ReadFrom closes the stream (`Position` afterwards: ObjectDisposed);
//! - a variable that holds no object is the error `Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDotNet to
//!   Microsoft.Dynamics.Nav.Runtime.NavInStream.` (Conversion), for ReadFrom and ImportStream (JsonObject.ReadFrom: not pinned).
//!
//! Another .NET stream type is refused.

use super::{stream_failure, unsupported};
use crate::Session;
use al_builtins::builtin_id::{BuiltinId, ids};
use al_builtins::native::Signal;
use al_builtins::value::{DotNetKind, LibraryValue, Value};
use al_host::dotnet::{DotNetFailure, DotNetHandle, DotNetTarget, DotNetValue};

const MEMORY_STREAM: &str = "System.IO.MemoryStream";

/// A MemoryStream a builtin call reads as its InStream argument, for what the call does to it afterwards.
pub(crate) struct DotNetStreamArg {
    id: BuiltinId,
    handle: DotNetHandle,
    len: i64,
    start: i64,
}

impl Session {
    /// For a builtin `id` that takes a DotNet MemoryStream as its first (InStream) argument: that argument becomes an InStream
    /// over the stream's bytes. `None` when the call has no such argument.
    pub(crate) fn dotnet_stream_into_instream(
        &mut self,
        id: BuiltinId,
        values: &mut [Value],
    ) -> Result<Option<DotNetStreamArg>, Signal> {
        if !matches!(
            id,
            ids::MEDIA_IMPORTSTREAM_1
                | ids::XMLDOCUMENT_READFROM_1
                | ids::XMLDOCUMENT_READFROM_2
                | ids::JSONOBJECT_READFROM_1
        ) {
            return Ok(None);
        }
        let handle = match values.first() {
            Some(Value::Null) if id == ids::JSONOBJECT_READFROM_1 => {
                return unsupported("JsonObject.ReadFrom of a DotNet stream that holds no object (not pinned)");
            }
            Some(Value::Null) => {
                return al_builtins::native::raise(
                    "Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavDotNet to Microsoft.Dynamics.Nav.Runtime.NavInStream.",
                    "Conversion",
                );
            }
            Some(Value::DotNet(o)) => match &o.kind {
                DotNetKind::Host { handle, clr_type } if clr_type == MEMORY_STREAM => *handle,
                _ => {
                    return unsupported(format!(
                        "a DotNet {o:?} as an InStream argument (only a MemoryStream is pinned)"
                    ));
                }
            },
            _ => return Ok(None),
        };
        let engine = self.engine.clone();
        if let Err(why) = engine
            .dotnet
            .policy
            .check(None, MEMORY_STREAM, super::Access::Member("Read"))
        {
            return unsupported(format!("DotNet {MEMORY_STREAM}: {why}"));
        }
        let start = if id == ids::MEDIA_IMPORTSTREAM_1 {
            self.memory_stream_position(handle)?
        } else {
            0
        };
        self.dotnet_ready(MEMORY_STREAM, "Read")?;
        let session = self.dotnet.session.as_mut().expect("opened by dotnet_ready");
        // The whole stream; the sidecar leaves the Position at 0.
        let bytes = stream_failure(MEMORY_STREAM, session.stream_read(handle, None))?;
        let len = bytes.len() as i64;
        values[0] = Value::Library(
            al_builtins::library_kind::LibraryKind::InStream,
            std::rc::Rc::new(std::cell::RefCell::new(LibraryValue::InStream(
                al_builtins::stream::InState::snapshot(bytes, al_builtins::stream::Encoding::default()),
            ))),
        );
        Ok(Some(DotNetStreamArg { id, handle, len, start }))
    }

    /// What the call that read `arg` does to the stream afterwards (see the module comment).
    pub(crate) fn dotnet_stream_after(
        &mut self,
        arg: DotNetStreamArg,
        result: &Result<Value, Signal>,
    ) -> Result<(), Signal> {
        let Ok(value) = result else { return Ok(()) };
        match arg.id {
            ids::JSONOBJECT_READFROM_1 => self.memory_stream_call(arg.handle, "Dispose"),
            ids::MEDIA_IMPORTSTREAM_1 => match value {
                Value::Guid(g) if g.iter().any(|b| *b != 0) => self.memory_stream_set_position(arg.handle, arg.len),
                _ if arg.start == 0 => Ok(()),
                _ => unsupported(
                    "Media.ImportStream that imports nothing from a DotNet stream not at Position 0 (not pinned)",
                ),
            },
            _ => self.memory_stream_set_position(arg.handle, arg.len),
        }
    }

    fn memory_stream_position(&mut self, handle: DotNetHandle) -> Result<i64, Signal> {
        self.dotnet_ready(MEMORY_STREAM, "Position")?;
        let session = self.dotnet.session.as_mut().expect("opened by dotnet_ready");
        match stream_failure(
            MEMORY_STREAM,
            session.get_property(DotNetTarget::Instance(handle), "Position"),
        )?
        .value
        {
            DotNetValue::Int64(n) => Ok(n),
            other => unsupported(format!("DotNet {MEMORY_STREAM}.Position answered {other:?}")),
        }
    }

    fn memory_stream_set_position(&mut self, handle: DotNetHandle, to: i64) -> Result<(), Signal> {
        self.dotnet_ready(MEMORY_STREAM, "Position")?;
        let session = self.dotnet.session.as_mut().expect("opened by dotnet_ready");
        let r = session.set_property(DotNetTarget::Instance(handle), "Position", DotNetValue::Int64(to));
        stream_failure(MEMORY_STREAM, r).map(|_| ())
    }

    fn memory_stream_call(&mut self, handle: DotNetHandle, member: &str) -> Result<(), Signal> {
        self.dotnet_ready(MEMORY_STREAM, member)?;
        let session = self.dotnet.session.as_mut().expect("opened by dotnet_ready");
        match session.invoke(DotNetTarget::Instance(handle), member, Vec::new()) {
            Ok(_) => Ok(()),
            Err(e) => stream_failure(MEMORY_STREAM, Err::<(), DotNetFailure>(e)),
        }
    }
}
