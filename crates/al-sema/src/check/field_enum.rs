//! `Rec.Field` reads the field's own enum, whatever enum of that name the file finds (al-runner issue 8317; corpus PR 1442,
//! merged, not pinned yet: types/enum-option/EnumMemberFieldType_Tests.al).
use super::*;

impl<'a> Checker<'a> {
    /// The field's own enum, when `left` is `Rec.Field` of an Enum field and the file reads the enum's name otherwise in a
    /// shape the probe measured: the file finds the name twice, or finds another enum of that name in its own namespace or
    /// through a `using` (EnumMemberFieldType_Tests.al:29, :59, :77; assumption enum-member-of-field-type). `None` where the
    /// file finds the field's enum itself, and in every shape not measured: the file's reading stays.
    pub(crate) fn field_enum_reading(&self, sc: &Scope, left: &SyntaxNode) -> Option<usize> {
        let (own, written) = self.field_enum(sc, left)?;
        let measured = match self.ws.resolve_ref(sc.file, ObjKind::Enum, &split_qualified(&written)) {
            Resolution::Found(e) => e != own && self.own_namespace_or_using(sc.file, e),
            Resolution::Ambiguous(_) => true,
            Resolution::Missing => false,
        };
        measured.then_some(own)
    }

    /// The type of member `node` (`e`), with the enum named by its namespace where the field's enum is not the file's
    /// reading of its name ([`Self::field_enum_reading`]), so that the lowering finds that enum from this file; the field's
    /// resolution then carries the assumption.
    pub(crate) fn field_member_type(&mut self, sc: &mut Scope, node: &SyntaxNode, e: E) -> E {
        if !matches!(e, E::Val(Ty::Enum(_))) {
            return e;
        }
        let Some(own) = self.field_enum_reading(sc, node) else {
            return e;
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

    /// The enum of the record field `Rec.Field` that `left` reads, named in the file that declares the field, with the
    /// field's type name as written there.
    fn field_enum(&self, sc: &Scope, left: &SyntaxNode) -> Option<(usize, String)> {
        if left.kind() != SyntaxKind::MemberExpr {
            return None;
        }
        let key = |n: &SyntaxNode| (sc.file, n.text_range().start().into(), n.text_range().end().into());
        let base = left.first_child()?;
        if !matches!(self.types.get(&key(&base)), Some(Ty::Record { .. })) {
            return None;
        }
        let res = self
            .resolutions
            .get(&key(left))
            .or_else(|| left.children().nth(1).and_then(|m| self.resolutions.get(&key(&m))))
            .map(Res::inner)?;
        let Res::Field { table, id } = res else {
            return None;
        };
        let field = self.members[*table].fields.iter().find(|f| f.id == *id)?;
        let Ty::Enum(written) = &field.ty else {
            return None;
        };
        let file = self.ws.objects[field.declared_in?].file;
        Some((self.resolve_quiet(file, ObjKind::Enum, written)?, written.to_string()))
    }

    /// Object `object` is declared in the namespace file `file` declares, or in a namespace the file has a `using` of.
    fn own_namespace_or_using(&self, file: usize, object: usize) -> bool {
        let Some(ns) = &self.ws.objects[object].namespace else {
            return false;
        };
        let f = &self.ws.files[file];
        f.namespace.as_ref().is_some_and(|n| n.eq_ignore_ascii_case(ns))
            || f.usings.iter().any(|u| u.eq_ignore_ascii_case(ns))
    }
}
