//! Runtime-provided .NET classes (ADR-014 section 1): server internals that cannot work outside
//! a service tier (`NavTenantSettingsHelper`, `NavUserAccountHelper`, `ALConfigSettings`,
//! `NavDesignerALFunctions`, ...). The registry is consulted before the host. A type found here
//! is served here for all its members; a member the registry does not implement is refused as
//! `Type.Member` and never forwarded to .NET.
//!
//! Every member names its pin: a corpus test (`file:line`) or a probe id from
//! `docs/probes-needed.md`. A member without either does not build (`Registry::new` refuses it).
//! The shipped registry (`server_classes.rs`) mirrors AL Runner v2 until a probe on real BC pins
//! each value (owner decision 2026-10-14, ADR-014). Each member's result type is declared in
//! `al_builtins::dotnet_sigs`, which the compiler reads; a test keeps the two in step.

use crate::Session;
use al_builtins::native::Signal;
use al_builtins::value::{DotNetObject, Value};
use std::rc::Rc;

/// What a member is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberKind {
    Construct,
    Call,
    Get,
    Set,
}

/// Evidence for a member's behaviour (hard rule 3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pin {
    /// A corpus test, as `file:line`.
    Corpus(String),
    /// A probe listed in `docs/probes-needed.md` and merged into the corpus repo.
    Probe(String),
    /// AL Runner v2, as `file:line`, until a probe on real BC pins the value (owner decision
    /// 2026-10-14, ADR-014). Replaced by a `Probe` when the probe batch reports.
    AlRunner(String),
    /// A test of Microsoft's own apps that passes on real BC and needs exactly this behavior (hard rule 3,
    /// exception of 2026-10-15), as `file:line`. The member is also registered as an assumption in
    /// `al_builtins::assumptions`, and a probe row waits in `docs/probes-needed.md`.
    Microsoft(String),
}

impl Pin {
    fn is_empty(&self) -> bool {
        match self {
            Pin::Corpus(s) | Pin::Probe(s) | Pin::AlRunner(s) | Pin::Microsoft(s) => s.trim().is_empty(),
        }
    }
}

/// The implementation of one member. `this` is the registry object the call is made on (`None`
/// for a static member or a constructor); the result of a constructor is the new object.
pub type MemberFn = fn(&mut Session, Option<&Rc<DotNetObject>>, &[Value]) -> Result<Value, Signal>;

/// A member that assigns to `var`/`out` arguments: it receives the arguments mutably, and the
/// router writes every changed argument that was passed by reference back to its variable.
pub type MemberRefFn = fn(&mut Session, Option<&Rc<DotNetObject>>, &mut [Value]) -> Result<Value, Signal>;

pub struct RegistryMember {
    pub name: String,
    pub kind: MemberKind,
    pub pin: Pin,
    pub call: MemberFn,
    /// When set, it is called instead of `call` (which then only names the member).
    pub call_ref: Option<MemberRefFn>,
}

pub struct RegistryType {
    /// The CLR type name the `dotnet { }` block declares.
    pub clr_name: String,
    pub members: Vec<RegistryMember>,
}

#[derive(Default)]
pub struct Registry {
    types: Vec<RegistryType>,
}

impl Registry {
    /// The registry shipped with the runtime: the BC server classes of `server_classes.rs`.
    /// As on a host that does not run .NET on Windows.
    pub fn builtin() -> Registry {
        Registry::new(super::server_classes::types()).expect("the shipped registry names a source for every member")
    }

    /// The shipped registry for a .NET host on Windows (`windows`) or elsewhere; the front end that starts the sidecar
    /// decides (ADR-006). On Windows it claims no System.Drawing type: GDI+ is there, so every System.Drawing type
    /// and member goes to .NET and a Bitmap and a Graphics are always real .NET objects (ADR-027 row 14, ADR-044
    /// note of 2026-10-29). Elsewhere the ADR-044 members of O365 Brand Color.MakePicture are served here.
    pub fn builtin_for(windows: bool) -> Registry {
        let mut r = Registry::builtin();
        if windows {
            r.types.retain(|t| !t.clr_name.starts_with("System.Drawing."));
        }
        r
    }

    /// A registry with no type, for tests that exercise the router without any server class.
    pub fn empty() -> Registry {
        Registry::default()
    }

    pub fn types(&self) -> &[RegistryType] {
        &self.types
    }

    /// Builds a registry, refusing any member without a pin.
    pub fn new(types: Vec<RegistryType>) -> Result<Registry, String> {
        for t in &types {
            for m in &t.members {
                if m.pin.is_empty() {
                    return Err(format!(
                        "registry member {}.{} names neither a corpus pin nor a probe",
                        t.clr_name, m.name
                    ));
                }
            }
        }
        Ok(Registry { types })
    }

    pub fn is_empty(&self) -> bool {
        self.types.is_empty()
    }

    pub fn get(&self, clr_name: &str) -> Option<&RegistryType> {
        self.types.iter().find(|t| t.clr_name.eq_ignore_ascii_case(clr_name))
    }
}

impl RegistryType {
    /// The member for an access. A property read (`Get`) also finds a `Call` member, because AL
    /// lets a method be called without parentheses.
    pub fn member(&self, name: &str, kind: MemberKind) -> Option<&RegistryMember> {
        // A constructor is the type's only one, whatever name the caller's declaration gives it.
        let by = |k: MemberKind| {
            self.members
                .iter()
                .find(|m| m.kind == k && (k == MemberKind::Construct || m.name.eq_ignore_ascii_case(name)))
        };
        by(kind).or_else(|| {
            if kind == MemberKind::Get {
                by(MemberKind::Call)
            } else {
                None
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DRAWING: [&str; 6] = [
        "System.Drawing.Bitmap",
        "System.Drawing.Graphics",
        "System.Drawing.ColorTranslator",
        "System.Drawing.Color",
        "System.Drawing.SolidBrush",
        "System.Drawing.Imaging.ImageFormat",
    ];

    /// Off Windows the ADR-044 registry serves the eight MakePicture members of these types.
    #[test]
    fn off_windows_the_registry_claims_the_drawing_types() {
        let r = Registry::builtin_for(false);
        for t in DRAWING {
            assert!(r.get(t).is_some(), "{t}");
        }
        assert!(Registry::builtin().get("System.Drawing.Bitmap").is_some());
    }

    /// On Windows every System.Drawing type goes to .NET (ADR-027 row 14, orchestrator 2026-10-29), so a Bitmap and a
    /// Graphics are always real .NET objects. The server classes stay.
    #[test]
    fn on_windows_the_registry_claims_no_drawing_type_and_keeps_the_server_classes() {
        let (on, off) = (Registry::builtin_for(true), Registry::builtin_for(false));
        for t in DRAWING {
            assert!(on.get(t).is_none(), "{t}");
        }
        assert!(on.types().iter().all(|t| !t.clr_name.starts_with("System.Drawing.")));
        assert_eq!(on.types().len(), off.types().len() - DRAWING.len());
    }
}
