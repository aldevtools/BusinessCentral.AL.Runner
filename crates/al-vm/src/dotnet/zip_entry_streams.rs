//! The stream `ZipArchiveEntry.Open()` returns is closed at the end of the statement when no variable keeps it: a later
//! `CreateEntry` of the same archive then works. A stream kept in a variable stays open, also after `CopyStream` into it
//! (al-language-onprem-dotnet/lt-y44-zip-create/Y44ZipCreateMode.al, corpus PR 962: Zip_TwoEntries_CopyStreamIntoOpen :244-255
//! `CopyStream(ZipArchiveEntry.Open(), InS)` three times, Zip_OpenTemporaryNoCopy_SecondCreate :293-294
//! `ZipArchiveEntry.Open().WriteByte(65)`, Zip_OpenKeptInVariable_SecondCreate :274-277 a kept stream blocks the next CreateEntry).
//!
//! A value nobody else holds (its `Rc` count is one when it is used) is the statement's temporary. Only the streams of
//! `ZipArchiveEntry.Open` are closed this way; what BC does with other temporaries is not pinned.

use super::{DotNetFailure, DotNetHandle, DotNetTarget, unsupported};
use crate::{Frame, Session};
use al_builtins::native::Signal;
use al_builtins::value::{DotNetKind, Value};
use al_ir as ir;
use std::rc::Rc;

/// The host handle of `v` when it is an entry stream that only `v` holds.
fn temporary_entry_stream(open: &std::collections::HashSet<DotNetHandle>, v: &Value) -> Option<(DotNetHandle, String)> {
    let Value::DotNet(o) = v else { return None };
    match &o.kind {
        DotNetKind::Host { handle, clr_type } if Rc::strong_count(o) == 1 && open.contains(handle) => {
            Some((*handle, clr_type.clone()))
        }
        _ => None,
    }
}

impl Session {
    /// `dotnet_apply`, plus: the result of `ZipArchiveEntry.Open()` is noted as an entry stream, and an entry stream that is
    /// the call's temporary receiver is closed after the call.
    pub(crate) fn dotnet_apply_closing_entry_streams(
        &mut self,
        ty: Option<usize>,
        recv: Value,
        values: Vec<Value>,
        places: Vec<Option<ir::Place>>,
        op: &ir::DotNetOp,
        f: &mut Frame,
    ) -> Result<Value, Signal> {
        let opens = matches!(op, ir::DotNetOp::Call(m) if m.eq_ignore_ascii_case("Open"))
            && matches!(&recv, Value::DotNet(o) if matches!(&o.kind,
                DotNetKind::Host { clr_type, .. } if clr_type == "System.IO.Compression.ZipArchiveEntry"));
        let temporary = temporary_entry_stream(&self.dotnet.entry_streams, &recv);
        // Kept until the stream is closed, so the handle is not released before.
        let keep = temporary.as_ref().map(|_| recv.clone());
        let result = self.dotnet_apply(ty, recv, values, places, op, f)?;
        if let Some((handle, clr_type)) = temporary {
            self.close_entry_stream(handle, &clr_type)?;
        }
        drop(keep);
        if opens
            && let Value::DotNet(o) = &result
            && let DotNetKind::Host { handle, .. } = &o.kind
        {
            self.dotnet.entry_streams.insert(*handle);
        }
        Ok(result)
    }

    /// After `CopyStream(target, ...)` into a .NET stream: a temporary entry stream is closed.
    pub(super) fn close_temporary_entry_stream(&mut self, target: &Value) -> Result<(), Signal> {
        match temporary_entry_stream(&self.dotnet.entry_streams, target) {
            Some((handle, clr_type)) => self.close_entry_stream(handle, &clr_type),
            None => Ok(()),
        }
    }

    /// CopyStream from a .NET stream that is no MemoryStream: only a temporary entry stream (nothing read it before), read
    /// whole, is pinned (Y44ZipCreateMode.al ReadZip `CopyStream(BufOut, ZipArchiveEntry.Open())`, Zip_TwoEntries_CopyStreamIntoOpen
    /// :246-255 `content_<i>`). Anything else is refused.
    pub(super) fn entry_stream_read_allowed(
        &self,
        src: &Value,
        clr_type: &str,
        count: Option<i64>,
    ) -> Result<(), Signal> {
        if clr_type == "System.IO.MemoryStream" {
            return Ok(());
        }
        match (count, temporary_entry_stream(&self.dotnet.entry_streams, src)) {
            (None, Some(_)) => Ok(()),
            _ => unsupported(format!(
                "DotNet CopyStream on {clr_type}: reading it is pinned only for the temporary stream of ZipArchiveEntry.Open() without a count"
            )),
        }
    }

    fn close_entry_stream(&mut self, handle: DotNetHandle, clr_type: &str) -> Result<(), Signal> {
        self.dotnet.entry_streams.remove(&handle);
        self.dotnet_ready(clr_type, "Dispose")?;
        let session = self.dotnet.session.as_mut().expect("opened by dotnet_ready");
        let reply = match session.invoke(DotNetTarget::Instance(handle), "Dispose", Vec::new()) {
            Ok(r) => r,
            Err(DotNetFailure::Refused(why)) => return unsupported(format!("DotNet {clr_type}.Dispose: {why}")),
            Err(DotNetFailure::Exception(e)) => {
                return unsupported(format!(
                    "DotNet {clr_type}.Dispose of a temporary entry stream raised the .NET exception {}: {}",
                    e.clr_type, e.message
                ));
            }
        };
        // Closing the entry writes its bytes through the archive into the AL stream behind it.
        for (id, bytes) in &reply.streams.written {
            match self.dotnet.bridges.get(id) {
                Some(blob) => al_builtins::stream::write_bytes(blob, bytes),
                None => {
                    return unsupported(format!(
                        "DotNet {clr_type}.Dispose: bytes for an unknown stream bridge {id}"
                    ));
                }
            }
        }
        Ok(())
    }
}
