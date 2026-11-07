//! The license as a ceiling of the permission check (ADR-062 amendment 1 point 5; ADR-060 decision 6).
//!
//! The effective level of an access is the lower of the license level and the session's level; the code's object
//! `Permissions` property then turns an Indirect level into an allowed access. The permission sets' part is
//! [`Session::sets_verdict`](crate::permissions); this module adds the license's part for TableData (Read, Insert, Modify,
//! Delete) below the system object range.
//!
//! License facts measured on BC 28.5 with a customer license, 2026-11-06 (owner decision): with a table licensed Read
//! Yes and Insert, Modify and Delete Indirect, a SUPER user's Insert (RecordRef or typed) from code without a
//! `Permissions` property fails with `Sorry, the current permissions prevented the action. (TableData <id> <table name>
//! Insert: <app name>)`, code `DB:ClientInsertDenied`; from a codeunit whose `Permissions` grants the table it works; a
//! direct read works. Modify, Delete and Read at a license level of Indirect raise the denial the corpus pins for the same
//! access under a user-level indirect grant (`permissions/testpermissions/CpPermLeads.al:91-103` Insert, Modify and
//! Read; `record/Y19PermWrites.al:172` Delete), through the same paths (`write_denied`, `read_denied`), which refuse every
//! shape they do not pin.
//!
//! Not served, so refused when the license decides: a blank license level (BC denies; the error of a license denial is
//! not pinned, ADR-060 decision 6), a mask whose meaning is not pinned (`al_host::license`), an access where an inherent
//! permission meets a license level below Yes (whether it lifts the license is not measured). Not checked: system
//! objects (2000000000 and up; BC overrides the license for some of them) and Execute (calls, page opens, reports).
use crate::permissions::{Access, PERMISSIONS_MOCK, SetModel, Verdict};
use crate::{Engine, Session};
use al_host::license::Level;

/// The first object number of the system range, where the license is not applied.
const SYSTEM_FIRST: u32 = 2_000_000_000;

impl Engine {
    /// Whether the license gives some table of the program below the system range less than Yes on Read, Insert, Modify
    /// or Delete (or a mask whose meaning is not pinned). False when the run has no license file, and for the license of
    /// the BC artifacts, which grants everything there: an unrestricted session then checks nothing.
    pub(crate) fn license_caps(&self) -> bool {
        *self.license_caps.get_or_init(|| {
            let Some(lic) = self.license.as_deref() else {
                return false;
            };
            self.program.objects.heads().iter().any(|o| {
                let Some(id) = o.id.filter(|&id| id < SYSTEM_FIRST) else {
                    return false;
                };
                if o.kind != al_ir::ObjKind::Table {
                    return false;
                }
                !matches!(lic.levels("TableData", i64::from(id)), Ok(Some(l)) if l[..4].iter().all(|&l| l == Level::Yes))
            })
        })
    }
}

impl Session {
    /// Whether record accesses go through the permission gates: a restricted session, or a run whose license limits a table.
    #[inline]
    pub(crate) fn permission_checks(&self) -> bool {
        self.permissions.is_some() || self.engine.license_caps()
    }

    /// The license's part of the decision for an access the permission sets allow (see the module text). `target` is the
    /// accessed table's object index; `flowfield`: a FlowField calculation, which counts as an object grant
    /// (CpPermLeads.al:29-63).
    pub(crate) fn license_verdict(&mut self, access: Access, target: Option<usize>, flowfield: bool) -> Verdict {
        if access.kind != 0 || access.id >= SYSTEM_FIRST {
            return Verdict::Allow;
        }
        let engine = self.engine.clone();
        let Some(lic) = engine.license.as_deref() else {
            return Verdict::Allow;
        };
        let Some(caller) = self.permission_caller.or_else(|| self.frames.last().copied()) else {
            return Verdict::Allow;
        };
        let o = &engine.program.objects.heads()[caller];
        if o.kind == al_ir::ObjKind::Codeunit && o.id == Some(PERMISSIONS_MOCK) {
            return Verdict::Allow;
        }
        let level = match lic.level("TableData", i64::from(access.id), access.op_license()) {
            Ok(Some(l)) => l,
            Ok(None) => {
                return Verdict::Refuse(format!(
                    "{}: the license has no TableData list, so its level is not known (ADR-060)",
                    access.describe()
                ));
            }
            Err(why) => return Verdict::Refuse(format!("{}: {why}", access.describe())),
        };
        match level {
            Level::Yes => return Verdict::Allow,
            Level::Blank => {
                return Verdict::Refuse(format!(
                    "{}: the license gives no permission (blank); BC denies it, but the error of a denial by the license with a blank level is not pinned (ADR-060 decision 6)",
                    access.describe()
                ));
            }
            Level::Indirect => {}
        }
        let refuse = |why: &str| {
            Verdict::Refuse(format!(
                "{}: the license grants it only indirectly, and {why}",
                access.describe()
            ))
        };
        if self.permissions_off {
            return refuse("permission checks are off (--permissions off)");
        }
        let p = access.op as usize;
        if let Some(t) = target {
            match crate::platform::table_inherent(&engine, t) {
                Some(Ok(l)) if l[p] != 0 => {
                    return refuse(
                        "the table's InherentPermissions names it: whether that lifts the license level is not measured",
                    );
                }
                Some(Err(why)) => return refuse(&format!("an inherent permission is not read: {why}")),
                _ => {}
            }
        }
        let model = SetModel::of(&engine);
        for (&object, frame) in self.frames.iter().zip(&self.call_stack) {
            for g in model.inherent_of(&engine, object, frame.routine).iter() {
                match g {
                    Ok(Some((k, id, l))) if *k == access.kind && *id == access.id && l[p] != 0 => {
                        return refuse(
                            "an [InherentPermissions] attribute names it: whether that lifts the license level is not measured",
                        );
                    }
                    Ok(_) => {}
                    Err(why) => return refuse(&format!("an inherent permission is not read: {why}")),
                }
            }
        }
        let grant = if flowfield {
            true
        } else if o.kind.extended().is_some() {
            return refuse(&format!(
                "the code of {} {} makes it; which Permissions property covers an extension object's code is not pinned (ADR-062)",
                o.kind, o.name
            ));
        } else {
            match &*model.object_grants(&engine, caller) {
                Ok(c) => c.level_of(access.kind, access.id, p) != 0,
                Err(why) => {
                    return refuse(&format!(
                        "the Permissions property of {} {} is not read: {why}",
                        o.kind, o.name
                    ));
                }
            }
        };
        if grant {
            Verdict::Allow
        } else {
            // No object grant: BC names the operation without `Indirect` (the license facts for Insert; the corpus pins
            // for a user-level indirect grant, see the module text).
            Verdict::Deny {
                indirect: false,
                by: caller,
            }
        }
    }
}

impl Access {
    /// The operation of the license (al-host's own enum).
    fn op_license(self) -> al_host::license::Op {
        use crate::permissions::Op;
        match self.op {
            Op::Read => al_host::license::Op::Read,
            Op::Insert => al_host::license::Op::Insert,
            Op::Modify => al_host::license::Op::Modify,
            Op::Delete => al_host::license::Op::Delete,
            Op::Execute => al_host::license::Op::Execute,
        }
    }
}
