//! `Field::Member` on an Enum field reads the field's own enum, whatever enum of that name the file finds (al-runner issue
//! 8317; corpus PR 1442, merged, not pinned yet: types/enum-option/EnumMemberFieldType_Tests.al; corpus PR 1480, merged, not
//! pinned yet: types/enum-option/EnumMemberFieldTypeShapes_Tests.al). `Rec.Field` itself then has the field's enum type (corpus PR 1480,
//! must-not-compile 3290 to 3293).
use super::*;

/// How a file reads the enum of an Enum field where it does not find the field's enum by its name.
pub(crate) enum FieldEnumReading {
    /// The field's own enum (an object index), in a shape a probe measured (assumption enum-member-of-field-type).
    Field(usize),
    /// A shape no probe measured: why the field is refused (checker note AR1010, which refuses the routine).
    Unmeasured(String),
}

/// What holds the record whose field is read.
enum Receiver {
    /// A local variable or a parameter: `Line.Kind`.
    Variable,
    /// A global variable of the object, not `Rec` or `xRec`: `GlobalLine.Kind`.
    GlobalVariable,
    /// The current record: `Rec.Kind`, or the bare field name `Kind`.
    CurrentRecord,
    /// Anything else: `xRec`, `this`, an array element, a call's result, a `with` statement's record.
    Other,
}

impl<'a> Checker<'a> {
    /// How the file reads the enum of the Enum field `left` names (`Rec.Field` or the bare field name), where it finds
    /// another enum by the enum's name, in any namespace tier, or finds the name more than once. Business Central reads the
    /// field's enum in every shape measured (EnumMemberFieldType_Tests.al:29, :59, :77; EnumMemberFieldTypeShapes_Tests.al:
    /// a temporary record, a member of an enum extension, a field of a table extension, a field enum of the global
    /// namespace, Rec and the bare field in a table extension and in a page, usings that find the field's enum and a twin
    /// or two twins, a twin of the global namespace, a global record of a codeunit). Measured: the record a local variable
    /// or parameter in a codeunit, table extension or page, a global variable of a codeunit, or Rec or the bare field name
    /// in a table extension or page; every other shape is
    /// [`FieldEnumReading::Unmeasured`]. `None` where the file finds the field's enum itself, or no enum of that name.
    pub(crate) fn field_enum_reading(&self, sc: &Scope, left: &SyntaxNode) -> Option<FieldEnumReading> {
        let (own, written, receiver) = self.field_enum(sc, left)?;
        let differs = match self.ws.resolve_ref(sc.file, ObjKind::Enum, &split_qualified(&written)) {
            Resolution::Found(e) => e != own,
            Resolution::Ambiguous(_) => true,
            Resolution::Missing => false,
        };
        if !differs {
            return None;
        }
        // A field enum of the global namespace has no qualified name that reaches it from a file that finds another
        // enum by its name, so `Rec.Field` could not have its type here.
        let nameable = self.ws.objects[own].namespace.is_some();
        let kind = self.ws.objects[sc.oi].kind;
        let measured = nameable
            && match receiver {
                Receiver::Variable => matches!(kind, ObjKind::Codeunit | ObjKind::TableExtension | ObjKind::Page),
                // EnumMemberFieldTypeShapes_Tests.al:143, :154 (corpus PR 1492): a global record of a codeunit.
                Receiver::GlobalVariable => kind == ObjKind::Codeunit,
                Receiver::CurrentRecord => matches!(kind, ObjKind::TableExtension | ObjKind::Page),
                Receiver::Other => false,
            };
        Some(if measured {
            FieldEnumReading::Field(own)
        } else {
            FieldEnumReading::Unmeasured(format!(
                "`{}`: this file finds another enum than the field's by the name {}; that the field's enum is read is \
                 measured only for a local variable or parameter in a codeunit, table extension or page, a global \
                 variable of a codeunit, and Rec or the bare field name in a table extension or page, with the field's \
                 enum in a namespace (assumption enum-member-of-field-type, corpus PR 1480, PR 1492)",
                left.text_string().trim(),
                crate::types::display_name(&written)
            ))
        })
    }

    /// The type of `node` (`e`), a field of an Enum, with the enum named by its namespace where the field's enum is not the
    /// file's reading of its name in a measured shape ([`Self::field_enum_reading`]), so that the lowering finds that enum
    /// from this file; the field's resolution then carries the assumption. In a shape not measured every use of the field
    /// depends on which enum it has: checker note AR1010, which refuses the routine (`check/diag_index.rs`).
    pub(crate) fn field_member_type(&mut self, sc: &mut Scope, node: &SyntaxNode, e: E) -> E {
        if !matches!(e, E::Val(Ty::Enum(_))) {
            return e;
        }
        let own = match self.field_enum_reading(sc, node) {
            None => return e,
            Some(FieldEnumReading::Unmeasured(why)) => {
                self.diag(sc, node, "AR1010", why);
                return e;
            }
            Some(FieldEnumReading::Field(own)) => own,
        };
        let key = (
            sc.file,
            node.text_range().start().into(),
            node.text_range().end().into(),
        );
        if let Some(res) = self.resolutions.get(&key).cloned() {
            self.record(sc, node, Self::assumed_if(true, res));
        }
        let o = &self.ws.objects[own];
        let mut parts: Vec<String> = o
            .namespace
            .as_deref()
            .map(|n| n.split('.').map(str::to_string).collect())
            .unwrap_or_default();
        parts.push(o.name.clone());
        E::Val(Ty::Enum(parts.join(crate::types::QUAL_SEP).as_str().into()))
    }

    /// `res`, carrying assumption enum-member-of-field-type when `assumed`.
    pub(crate) fn assumed_if(assumed: bool, res: Res) -> Res {
        if !assumed || matches!(res, Res::Assumed { .. }) {
            return res;
        }
        Res::Assumed {
            assumption: al_builtins::assumptions::Assumption::EnumMemberOfFieldType,
            inner: Box::new(res),
        }
    }

    /// The enum of the record field that `left` reads (`Rec.Field`, or the bare field name of the current record), named
    /// in the file that declares the field, with the field's type name as written there, and what holds the record.
    fn field_enum(&self, sc: &Scope, left: &SyntaxNode) -> Option<(usize, String, Receiver)> {
        let key = |n: &SyntaxNode| (sc.file, n.text_range().start().into(), n.text_range().end().into());
        let (res, receiver) = match left.kind() {
            SyntaxKind::MemberExpr => {
                let base = left.first_child()?;
                // `this.Field` records no type for `this`: it is classified by its resolution alone.
                let receiver = match (base.kind(), self.resolutions.get(&key(&base))) {
                    (SyntaxKind::NameRef, Some(Res::Local(_))) => Receiver::Variable,
                    (SyntaxKind::NameRef, Some(Res::Global(g))) if g == "rec" => Receiver::CurrentRecord,
                    (SyntaxKind::NameRef, Some(Res::Global(g))) if g != "xrec" => Receiver::GlobalVariable,
                    _ => Receiver::Other,
                };
                let res = self
                    .resolutions
                    .get(&key(left))
                    .or_else(|| left.children().nth(1).and_then(|m| self.resolutions.get(&key(&m))))?;
                (res, receiver)
            }
            SyntaxKind::NameRef => {
                let res = self.resolutions.get(&key(left))?;
                // A field the current record supplies, also once marked by `field_member_type`; not one of a `with`.
                let direct = match res {
                    Res::Assumed { inner, .. } => &**inner,
                    r => r,
                };
                let receiver = match direct {
                    Res::Field { .. } => Receiver::CurrentRecord,
                    _ => Receiver::Other,
                };
                (res, receiver)
            }
            _ => return None,
        };
        let Res::Field { table, id } = res.inner() else {
            return None;
        };
        let field = self.members[*table].fields.iter().find(|f| f.id == *id)?;
        let Ty::Enum(written) = &field.ty else {
            return None;
        };
        let file = self.ws.objects[field.declared_in?].file;
        Some((
            self.resolve_quiet(file, ObjKind::Enum, written)?,
            written.to_string(),
            receiver,
        ))
    }
}
