//! The error of a member of a DotNet variable that holds no object (code `DotNetVariableNotCreated`), where it is pinned.

use super::registry::MemberKind;
use super::system_environment;
use crate::{Frame, Session};
use al_builtins::native::Signal;
use al_builtins::value::Value;

impl Session {
    /// `Some` with the error when the instance member `member` of the declared type `ty`, on a variable that holds no object,
    /// raises a pinned error; `None` to go on (a static member of the type, or not pinned).
    ///
    /// - `X.GetType()` on an unassigned `System.Object` variable (tests/al-language-onprem-dotnet/dotnet/dotnet-interop/
    ///   DotNetTypeOfDotNet.al:224 `Unassigned_error`, after GetDotNetType(X) answered System.Object: dotnet/dotnet/BbGetDotNetType.al:364).
    /// - `CreateEntry(Path)`, the property `Entries` and `Dispose()` of System.IO.Compression.ZipArchive
    ///   (dotnet/zip-create/Y44ZipCreateMode.al:358-367 Zip_CreateEntry_NoObject, corpus PR 962).
    /// - The property `Uri` of System.Security.Cryptography.Xml.Reference (dotnet/dotnet/ClearSignedXmlReference.al:178-179).
    ///
    /// The text names the routine and its codeunit; outside a codeunit the object kind's spelling is not pinned.
    pub(super) fn not_created(
        &self,
        kind: MemberKind,
        member: &str,
        no_args: bool,
        ty: &str,
        f: &Frame,
    ) -> Option<Result<Value, Signal>> {
        let is = |m: &str| member.eq_ignore_ascii_case(m);
        let called = match (kind, ty) {
            (MemberKind::Call, "System.Object") if is("GetType") && no_args => "System.Object.GetType",
            (MemberKind::Call, "System.IO.Compression.ZipArchive") if is("CreateEntry") && !no_args => {
                "System.IO.Compression.ZipArchive.CreateEntry"
            }
            (MemberKind::Call, "System.IO.Compression.ZipArchive") if is("Dispose") && no_args => {
                "System.IO.Compression.ZipArchive.Dispose"
            }
            (MemberKind::Get, "System.IO.Compression.ZipArchive") if is("Entries") => {
                "System.IO.Compression.ZipArchive.Entries"
            }
            // The property Uri of a cleared Reference (dotnet/dotnet/ClearSignedXmlReference.al:178-179, step 7, AL Runner
            // issue 7394).
            (MemberKind::Get, "System.Security.Cryptography.Xml.Reference") if is("Uri") => {
                "System.Security.Cryptography.Xml.Reference.Uri"
            }
            // BC 29.0 reads NewLine of a variable never constructed; BC 27.5 and 28.5 raise
            // (Y58ServerHelpers.al SystemEnvironment_NewLine_WithoutConstructor, `#if BC29PLUS`).
            (MemberKind::Call | MemberKind::Get, system_environment::TYPE)
                if is("NewLine") && self.engine.target.as_ref().map_or(29, |t| t.bc.major) < 29 =>
            {
                system_environment::NEW_LINE
            }
            _ => return None,
        };
        let o = &self.engine.program.objects[f.object];
        if o.kind != al_ir::ObjKind::Codeunit || o.uncompiled {
            return Some(Err(Signal::unsupported(format!(
                "{called} on a DotNet variable that holds no object, outside a codeunit (the object kind's spelling in the error is not pinned)"
            ))));
        }
        let message = format!(
            "A DotNet variable has not been instantiated. Attempting to call {called} in CodeUnit {}: {}",
            o.name, o.routines[f.routine].name
        );
        Some(al_builtins::native::raise(message, "DotNetVariableNotCreated"))
    }
}
