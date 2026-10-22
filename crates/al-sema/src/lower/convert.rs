//! Name resolution helpers, runtime types and the conversion of a value to a target type.
use super::*;

impl<'a> Lowerer<'a> {
    /// What `n` resolved to; a name a `with` statement supplied is read as the member it names.
    pub(crate) fn res(&self, n: &SyntaxNode) -> Option<&Res> {
        let r = self.checked.resolutions.get(&key(self.file, n)).map(Res::inner);
        if let (Some(d), Some(r)) = (&self.checked.tdd_decls, r) {
            if !d.is_empty() {
                self.tdd_note(d, r);
            }
        }
        r
    }

    /// `--tdd` (ADR-051 decision 10): a use of a stub field or enum value, or of a global whose type is a stub object,
    /// marks the statement being lowered.
    fn tdd_note(&self, d: &crate::check::tdd_decl::TddDecls, r: &Res) {
        let sig = match r {
            Res::Field { table, id } => d.fields.get(&(*table, *id)),
            Res::EnumValue { object, value } => d.values.get(&(*object, *value)),
            Res::Global(n) => self.tdd_globals.get(n),
            _ => None,
        };
        if let Some(sig) = sig {
            if let Some(frame) = self.tdd_marked.borrow_mut().last_mut() {
                if !frame.contains(sig) {
                    frame.push(sig.clone());
                }
            }
        }
    }

    /// The record `n` is a member of, when a `with` statement supplied the name; else the implicit `Rec`.
    pub(crate) fn implicit_rec(&self, n: &SyntaxNode) -> L<ir::Place> {
        match self
            .checked
            .resolutions
            .get(&key(self.file, n))
            .and_then(Res::with_stmt)
        {
            Some(w) => self
                .with_places
                .get(&w)
                .cloned()
                .ok_or_else(|| "a name of a with statement outside its statement".to_string()),
            None => Ok(ir::Place::Global(
                self.rec_slot.ok_or("implicit Rec is not lowered for this object yet")?,
            )),
        }
    }

    /// Whether a `with` statement supplied the name `n`.
    pub(crate) fn supplied_by_with(&self, n: &SyntaxNode) -> bool {
        self.checked
            .resolutions
            .get(&key(self.file, n))
            .is_some_and(|r| r.with_stmt().is_some())
    }

    /// [`ty_of`](Self::ty_of) without the copy.
    pub(crate) fn ty_ref(&self, n: &SyntaxNode) -> &'a Ty {
        static UNKNOWN: Ty = Ty::Unknown;
        let checked: &'a CheckResult = self.checked;
        checked.types.get(&key(self.file, n)).unwrap_or(&UNKNOWN)
    }

    pub(crate) fn ty_of(&self, n: &SyntaxNode) -> Ty {
        self.checked
            .types
            .get(&key(self.file, n))
            .cloned()
            .unwrap_or(Ty::Unknown)
    }

    pub(crate) fn resolve_obj(&self, kind: ObjKind, name: &str) -> L<usize> {
        self.resolve_obj_in(self.file, kind, name)
    }

    /// Object `name` as the file `file` sees it.
    pub(crate) fn resolve_obj_in(&self, file: usize, kind: ObjKind, name: &str) -> L<usize> {
        match self.ws.resolve_ref(file, kind, &crate::types::qualified_parts(name)) {
            Resolution::Found(i) => Ok(i),
            _ => Err(self.ws.outside_workspace(kind, &crate::types::display_name(name))),
        }
    }

    /// The `TableType` properties of table `table`, trimmed, in source order.
    pub(crate) fn table_types(&self, table: usize) -> std::rc::Rc<Vec<String>> {
        if let Some(hit) = self.memo.table_type.borrow().get(&table) {
            return hit.clone();
        }
        let kinds: std::rc::Rc<Vec<String>> = std::rc::Rc::new(
            self.ws
                .object_ast(table)
                .properties()
                .filter(|p| p.name().eq_ignore_ascii_case("TableType"))
                .map(|p| p.value_text().trim().to_string())
                .collect(),
        );
        self.memo.table_type.borrow_mut().insert(table, kinds.clone());
        kinds
    }

    pub(crate) fn is_temporary_table(&self, table: usize) -> bool {
        self.table_types(table)
            .iter()
            .any(|k| k.eq_ignore_ascii_case("temporary"))
    }

    /// The codeunits that implement `iface` (or an interface that extends it), in object order.
    pub(crate) fn implementers(&self, iface: usize) -> std::rc::Rc<Vec<usize>> {
        self.embeds.borrow_mut().insert(iface);
        if let Some(hit) = self.memo.implementers.borrow().get(&iface) {
            return hit.clone();
        }
        let found: std::rc::Rc<Vec<usize>> = std::rc::Rc::new(
            (0..self.ws.objects.len())
                .filter(|&cu| self.ws.objects[cu].kind == ObjKind::Codeunit && self.implements(cu, iface))
                .collect(),
        );
        self.memo.implementers.borrow_mut().insert(iface, found.clone());
        found
    }

    pub(crate) fn rt(&self, t: &Ty) -> L<RtType> {
        if let Some(r) = scalar_rt(self.ws, self.file, t) {
            return r;
        }
        Ok(match t {
            Ty::Object(ObjKind::Codeunit, n) => RtType::Codeunit(self.resolve_obj(ObjKind::Codeunit, n)? as u32),
            // An xmlport variable is an instance with its own globals, like a codeunit's.
            Ty::Object(ObjKind::XmlPort, n) => {
                let x = self.resolve_obj(ObjKind::XmlPort, n)?;
                self.embeds.borrow_mut().insert(x);
                match self.xml_reasons[x].as_ref() {
                    None => RtType::Codeunit(x as u32),
                    Some(why) => return Err(format!("XmlPort \"{}\": {why}", crate::types::display_name(n))),
                }
            }
            // A report variable is an instance with its own globals, like a codeunit's.
            Ty::Object(ObjKind::Report, n) => {
                let r = self.resolve_obj(ObjKind::Report, n)?;
                self.embeds.borrow_mut().insert(r);
                match self.report_reasons[r].as_ref() {
                    None => RtType::Codeunit(r as u32),
                    Some(why) => return Err(format!("Report \"{}\": {why}", crate::types::display_name(n))),
                }
            }
            Ty::Object(ObjKind::Page, n) => {
                let p = self.resolve_obj(ObjKind::Page, n)?;
                self.embeds.borrow_mut().insert(p);
                match self.page_reasons[p].first() {
                    None => RtType::Page(p as u32),
                    Some(why) => return Err(format!("Page \"{n}\": {why}")),
                }
            }
            Ty::Object(ObjKind::Query, n) => {
                let q = self.resolve_obj(ObjKind::Query, n)?;
                match self.checked.queries[q].as_ref().and_then(|l| l.reasons.first()) {
                    None => RtType::Query(q as u32),
                    Some(why) => return Err(format!("Query \"{}\": {why}", crate::types::display_name(n))),
                }
            }
            Ty::TestPage(n) => {
                let p = self.resolve_obj(ObjKind::Page, n)?;
                self.embeds.borrow_mut().insert(p);
                match self.page_reasons[p].first() {
                    None => RtType::TestPage(p as u32),
                    Some(why) => return Err(format!("TestPage \"{}\": {why}", crate::types::display_name(n))),
                }
            }
            // A request page is the page of a report; the handler's variable is typed by the report.
            Ty::TestRequestPage(n) => {
                let r = self.resolve_obj(ObjKind::Report, n)?;
                self.embeds.borrow_mut().insert(r);
                match self.page_reasons[r].first() {
                    None => RtType::TestPage(r as u32),
                    Some(why) => {
                        return Err(format!("TestRequestPage \"{}\": {why}", crate::types::display_name(n)));
                    }
                }
            }
            Ty::Object(ObjKind::Interface, n) => {
                let i = self.resolve_obj(ObjKind::Interface, n)?;
                RtType::Interface(i as u32)
            }
            Ty::List(t) => RtType::List(Box::new(self.rt(t)?)),
            Ty::Dictionary(k, v) => RtType::Dictionary(Box::new(self.rt(k)?), Box::new(self.rt(v)?)),
            Ty::Array(d, t) => RtType::Array(d.clone(), Box::new(self.rt(t)?)),
            Ty::Builtin(n) if al_builtins::sysenum::members(n).is_some() => {
                RtType::Option(al_builtins::sysenum::option_members(n).expect("checked"))
            }
            Ty::Builtin(n) => RtType::Library((&**n).into()),
            Ty::DotNet(alias) if alias.is_empty() => {
                return Err("a DotNet value whose type is only known at run time cannot be stored as such".into());
            }
            Ty::DotNet(alias) => match self.ws.resolve_dotnet(self.file, alias) {
                Some(i) => RtType::DotNet(i as u32),
                None => {
                    return Err(format!(
                        "DotNet '{alias}' has no dotnet {{ }} declaration in this app or its dependencies"
                    ));
                }
            },
            Ty::Record { table, temporary } => {
                let t = self.resolve_obj(ObjKind::Table, table)?;
                // A table of another type (CRM, ExternalSQL, ...) is served by a table connection.
                let kind = self.table_types(t).first().cloned();
                // A CRM table is lowered: the VM refuses its first use (`Session::connection_check`).
                if let Some(k) = kind.filter(|k| {
                    !k.eq_ignore_ascii_case("normal")
                        && !k.eq_ignore_ascii_case("temporary")
                        && !k.eq_ignore_ascii_case("crm")
                }) {
                    return Err(format!(
                        "table \"{}\" has TableType = {k}: it is served by a table connection, which the runtime does not model",
                        crate::types::display_name(table)
                    ));
                }
                // `TableType = Temporary` makes every record of the table temporary (corpus
                // record/TestTempTableContracts.al:16).
                RtType::Record(t as u32, *temporary || self.is_temporary_table(t))
            }
            other => return Err(format!("type {other} is not lowered yet")),
        })
    }

    /// Whether codeunit `cu` implements interface `iface`: it lists `iface`, or an interface that
    /// extends it, after `implements` (codeunit/TestInterfaceRuntimeContracts.al:
    /// Interface_ExtendedInterface_IsAndAs: a codeunit that implements `Probe IBar` answers a call
    /// through a `Probe IFoo` variable, and `is "Probe IFoo"` holds).
    pub(crate) fn implements(&self, cu: usize, iface: usize) -> bool {
        let file = self.ws.objects[cu].file;
        let family = self.ws.interface_family(iface);
        self.ws.objects[cu].implements.iter().any(
            |p| matches!(self.ws.resolve_ref(file, ObjKind::Interface, p), Resolution::Found(i) if family.contains(&i)),
        )
    }

    /// A variable declared `TextConst` going into a Variant: in BC the Variant then answers no Is... test while Format and a
    /// Text read its text (types/lt-variant/VaTTextConst.al:144-151, TextConst_In_Variant). That value kind is not built, so the
    /// conversion is refused. A Label is a Text in a Variant (Base App hands labels to Variant parameters). Probe wanted for the
    /// other operations on such a Variant (docs/probes-needed.md).
    pub(crate) fn refuse_textconst_into_variant(&self, target: &Ty, node: &SyntaxNode) -> L<()> {
        if !matches!(target, Ty::Variant) {
            return Ok(());
        }
        let textconst = match self.res(node) {
            Some(Res::Local(n)) => self.local_textconsts.contains(n),
            Some(Res::Global(n)) => self.global_textconsts.contains(n),
            _ => false,
        };
        if textconst {
            return Err("a TextConst in a Variant (the Variant answers no Is... test; not built)".into());
        }
        Ok(())
    }

    /// `value` (of type `src`) converted for a slot of type `target`. An interface slot takes a
    /// codeunit that implements it (corpus codeunit/TestCodeunitInterface.al:14-26) or an enum
    /// that implements it (types/TestEnumDefaultImplTests.Codeunit.al:30-42); everything else
    /// is refused.
    pub(crate) fn convert_to(&mut self, target: &Ty, src: &Ty, value: ir::Expr) -> L<ir::Expr> {
        // A text literal of one character is a Char once it goes into a Variant (types/lt-variant/VaTIs*.al:Is_Text_Code_Char_Byte
        // `V := 'a'` is IsChar and not IsText; `V := 'ab'` and `V := ''` are Text; a Text variable holding 'a' stays Text).
        if matches!(self.rt(target), Ok(RtType::Variant))
            && matches!(&value, ir::Expr::Const(ir::Const::Text(s)) if s.encode_utf16().count() == 1)
        {
            let ch = self.convert_to(&Ty::Char, src, value)?;
            return self.convert_to(target, &Ty::Char, ch);
        }
        // The literal of an option member (`O::B`) is an Integer in a Variant; a platform enum's member keeps its type
        // (types/lt-variant-is-text-streams/LTP89IsMisc.al:NearTypes `option_literal`, LTP89IsEnumsA.al `literal`).
        if matches!(self.rt(target), Ok(RtType::Variant)) {
            if let ir::Expr::Const(ir::Const::Option(i, members)) = &value {
                if !al_builtins::sysenum::is_platform(members) {
                    return self.convert_to(target, &Ty::Integer, ir::Expr::Const(ir::Const::Integer(*i)));
                }
            }
        }
        // A Media or MediaSet value in a Variant answers no Is... test (not even IsGuid) and formats as its media id
        // (media/lt-variant-media/VariantMedia.al:Variant_HoldingEmptyMediaField, _HoldingImportedMediaField;
        // types/lt-variant-unsupported-kinds/VariantUnsupportedKinds.al:Variant_BlobAndMediaFieldValues).
        if matches!(self.rt(target), Ok(RtType::Variant))
            && matches!(src, Ty::Builtin(n) if n.eq_ignore_ascii_case("Media") || n.eq_ignore_ascii_case("MediaSet"))
        {
            return Ok(ir::Expr::Builtin {
                id: super::call::builtin_id("Media.ToVariant#1")?,
                receiver: None,
                args: vec![ir::Arg::Value(value)],
                ret_used: true,
            });
        }
        if !self.tys_on {
            return self.convert_raw(target, src, value);
        }
        let inner = al_ir::types::kind_of(&value);
        let e = self.convert_raw(target, src, value)?;
        // A wrapper made here is one more node above the records of `value`.
        if let ir::Expr::Convert(t, x) | ir::Expr::DotNetCoerce(t, x) = &e {
            if let Some(top) = self.tys.last().copied() {
                if al_ir::types::kind_of(x) == inner
                    && top.kind == inner
                    && top.size as usize == al_ir::types::count_expr(x)
                {
                    let ty = if matches!(e, ir::Expr::Convert(..)) {
                        self.exact_ty(t)
                    } else {
                        al_ir::types::ExprTy::Unknown
                    };
                    self.tys.push(tys::TyRec {
                        ty,
                        kind: al_ir::types::kind_of(&e),
                        arity: 1,
                        size: top.size + 1,
                    });
                }
            }
        }
        Ok(e)
    }

    /// The false branch `value` (of type `f`) of a conditional expression whose true branch has type `t`. The true branch sets
    /// the type of the expression and the false branch is converted to it when it is taken (corpus types/TestTernaryContracts.al:
    /// BN_Ternary_IntegerAndDecimalBranches :236-280, an Integer true branch makes a Decimal 0.25 overflow; BN_Ternary_Code20TrueBranch
    /// :289-337, a Code[20] true branch makes a 30-character Text fail StringLengthExceeded, a Text true branch takes a Code as it is;
    /// BN_Ternary_VariantBranch :173-233, a Text true branch makes a Variant holding an Integer fail TypeConversion, a Variant true
    /// branch keeps the Text). Other pairs of different types are refused: their conversion is not pinned.
    ///
    /// `target` is the type of the plain assignment's target when the expression is its whole right side. A Text or Code
    /// longer than a Text true branch, assigned to a target of exactly the true branch's type, is taken as it is when it fits
    /// and fails StringLengthExceeded with the target's length when it does not (types/lt-qx-ternary-sized/QxTernarySized.al:
    /// 142-144, :152-155 `T80 := Cond ? T80 : T100`): whether BC converts it to the true branch's type first or not, the
    /// result is the same there. Into another target the two differ, which no test pins: refused.
    pub(crate) fn ternary_false_branch(&mut self, t: &Ty, f: &Ty, value: ir::Expr, target: Option<&Ty>) -> L<ir::Expr> {
        if t == f || matches!(t, Ty::Unknown) || matches!(f, Ty::Unknown) {
            return Ok(value);
        }
        // A text literal that fits a Text of any length is the same value in it.
        let literal_fits = |n: &Option<u32>| match &value {
            ir::Expr::Const(ir::Const::Text(s)) => n.is_none_or(|n| s.encode_utf16().count() as u64 <= u64::from(n)),
            _ => false,
        };
        let pinned = match (t, f) {
            (Ty::Integer, Ty::Decimal) | (Ty::Decimal, Ty::Integer) => true,
            (Ty::Code(_), Ty::Text(_) | Ty::Code(_)) => true,
            (Ty::Text(None), Ty::Text(_) | Ty::Code(_)) => true,
            (Ty::Text(n), Ty::Text(_)) if literal_fits(n) => true,
            // A Text or Code no longer than the Text true branch fits it: its conversion cannot fail.
            (Ty::Text(Some(n)), Ty::Text(Some(m)) | Ty::Code(Some(m))) if m <= n => true,
            (Ty::Text(Some(_)), Ty::Text(Some(_)) | Ty::Code(Some(_))) if target == Some(t) => true,
            (Ty::Text(_), Ty::Variant) => true,
            (Ty::Variant, _) => true,
            _ => false,
        };
        if !pinned {
            // A longer Text or Code false branch anywhere else is not pinned. Microsoft's DemoTool needs it to run (Base App
            // AccScheduleLine.Table.al:113 `Description := Description = '' ? GLAccCategory.Description : Description`, a
            // Text[80] true branch and a Text[100] false branch into a Text[100] field), so it runs on the registered
            // assumption `ternary-longer-text-false-branch-fits` (hard rule 3 exception): a value no longer than the true
            // branch passes unchanged, a longer one is refused when the branch is taken.
            if let (Ty::Text(Some(n)), Ty::Text(Some(_)) | Ty::Code(Some(_))) = (t, f) {
                self.assume(al_builtins::assumptions::Assumption::TernaryLongerTextFalseBranchFits);
                return Ok(ir::Expr::FitOrRefuse {
                    max: *n,
                    why: format!(
                        "a conditional expression whose false branch ({f}) holds a value longer than its true branch ({t}): whether BC converts it is not pinned"
                    ),
                    inner: Box::new(value),
                });
            }
            return Err(format!(
                "a conditional expression whose branches have different types ({t} and {f}): the conversion of the false branch is not pinned"
            ));
        }
        self.convert_to(t, f, value)
    }

    fn convert_raw(&self, target: &Ty, src: &Ty, value: ir::Expr) -> L<ir::Expr> {
        let slot = self.rt(target)?;
        // A number variable into a Text or Code is AL0122 (must-not-compile 0069 for an Integer; the other numbers are not judged by
        // the checker). Only a Variant (FieldRef.Value) converts at run time (types/lt-variant-to-text-fields/VariantFieldRefBigIntToText.al), so the direct
        // assignment stays refused.
        if matches!(slot, RtType::Text(_) | RtType::Code(_))
            && matches!(src, Ty::Integer | Ty::BigInteger | Ty::Decimal)
        {
            return Err(format!(
                "{src} into a {} (a compile error in AL; only a Variant holding a number converts)",
                if matches!(slot, RtType::Text(_)) {
                    "Text"
                } else {
                    "Code"
                }
            ));
        }
        // An identity conversion is not emitted (ADR-031 item 2.2): the value already has the slot's type. Only the
        // types whose conversion is the identity qualify; a Text or Code keeps its conversion (it checks the length of
        // what a concatenation made), as does everything that copies, wraps or normalizes.
        if is_identity_scalar(&slot) && self.rt(src).is_ok_and(|from| from == slot) {
            return Ok(value);
        }
        // A DotNet result has no static type: it must fit the target exactly (ADR-014).
        if matches!(src, Ty::DotNet(a) if a.is_empty()) && !matches!(slot, RtType::DotNet(_)) {
            return Ok(ir::Expr::DotNetCoerce(slot, Box::new(value)));
        }
        let RtType::Interface(iface) = slot else {
            return Ok(ir::Expr::Convert(slot, Box::new(value)));
        };
        let iface = iface as usize;
        let iname = &self.ws.objects[iface].name;
        match src {
            // The same interface, or one it extends: the value is the same instance
            // (codeunit/TestInterfaceRuntimeContracts.al:Interface_ExtendedInterface_IsAndAs).
            Ty::Object(ObjKind::Interface, n) => {
                let from = self.resolve_obj(ObjKind::Interface, n)?;
                if from != iface && !self.ws.interface_bases(from).contains(&iface) {
                    return Err(format!(
                        "interface \"{}\" into interface \"{iname}\", which it does not extend",
                        crate::types::display_name(n)
                    ));
                }
                Ok(ir::Expr::Convert(slot, Box::new(value)))
            }
            Ty::Object(ObjKind::Codeunit, n) => {
                let cu = self.resolve_obj(ObjKind::Codeunit, n)?;
                if !self.implements(cu, iface) {
                    return Err(format!(
                        "codeunit \"{}\" does not implement interface \"{iname}\"",
                        crate::types::display_name(n)
                    ));
                }
                Ok(ir::Expr::Convert(slot, Box::new(value)))
            }
            Ty::Enum(n) => self.enum_impl(n, iface, value),
            other => Err(format!("{other} into interface \"{iname}\" is not lowered yet")),
        }
    }

    /// The cast of an enum value to interface `iface`: per declared ordinal the value's own
    /// `Implementation`, else the enum's `DefaultImplementation`; undeclared ordinals take
    /// `UnknownValueImplementation` (corpus types/TestEnumDefaultImplTests.Codeunit.al:58-100,
    /// types/TestEnumExtImplTests.Codeunit.al:33-76: an enumextension adds values with their own
    /// `Implementation` and falls back to the base enum's default).
    pub(crate) fn enum_impl(&self, enum_name: &str, iface: usize, value: ir::Expr) -> L<ir::Expr> {
        use crate::members::ImplSlot;
        let e = self.resolve_obj(ObjKind::Enum, enum_name)?;
        self.embeds.borrow_mut().insert(e);
        self.embeds.borrow_mut().insert(iface);
        let iname = &self.ws.objects[iface].name;
        let find = |slot: &ImplSlot| -> L<Option<usize>> {
            for im in self.members[e].enum_impls.iter().filter(|i| i.slot == *slot) {
                let file = self.ws.objects[im.declared_in].file;
                if !matches!(self.ws.resolve_ref(file, ObjKind::Interface, &im.interface), Resolution::Found(i) if i == iface)
                {
                    continue;
                }
                return match self.ws.resolve_ref(file, ObjKind::Codeunit, &im.codeunit) {
                    Resolution::Found(c) => Ok(Some(c)),
                    _ => Err(format!("codeunit `{}` is outside the workspace", im.codeunit.join("."))),
                };
            }
            Ok(None)
        };
        let default = find(&ImplSlot::Default)?;
        let unknown = find(&ImplSlot::Unknown)?;
        let mut by_ordinal = Vec::new();
        for (ordinal, vname) in &self.members[e].enum_values {
            match find(&ImplSlot::Value(*ordinal))?.or(default) {
                Some(c) => by_ordinal.push((*ordinal as i32, c)),
                None => {
                    return Err(format!(
                        "enum \"{enum_name}\" value `{vname}` has no implementation of interface \"{iname}\""
                    ));
                }
            }
        }
        let at = self.expansion(super::localize::Expanded::EnumImpl {
            enum_: e,
            interface: iface,
            by_ordinal,
            unknown,
        });
        Ok(ir::Expr::EnumImpl {
            value: Box::new(value),
            by_ordinal: vec![(ir::walk::EXPAND_ORDINAL, at)],
            unknown: None,
        })
    }

    /// `x is I` and `x as I` on an interface value: whether the codeunit instance the variable
    /// holds implements `I` (corpus codeunit/TestInterfaceRuntimeContracts.al:
    /// Interface_ExtendedInterface_IsAndAs pins both for an implementing instance). The
    /// implementing codeunits are known here, so the test is a set of object indexes.
    pub(crate) fn interface_test(&mut self, n: &SyntaxNode, kids: &[SyntaxNode]) -> L<ir::Expr> {
        let is = sig_tokens(n)[0].text().eq_ignore_ascii_case("is");
        if !matches!(self.ty_ref(&kids[0]), Ty::Object(ObjKind::Interface, _)) {
            return Err("`is` and `as` on a value that is not an interface variable are not lowered yet".into());
        }
        let name = crate::check::type_name_text(&kids[1]);
        let iface = self.resolve_obj(ObjKind::Interface, &name)?;
        let objects: Vec<u32> = self.implementers(iface).iter().map(|&cu| cu as u32).collect();
        let value = Box::new(self.expr(&kids[0])?);
        let at = self.expansion(super::localize::Expanded::InterfaceTest {
            interface: iface,
            objects,
        });
        Ok(ir::Expr::InterfaceTest {
            value,
            objects: vec![ir::walk::EXPAND_U32, at as u32],
            cast: !is,
        })
    }

    /// A call of an interface procedure on `receiver`: it runs the same-named procedure of
    /// whichever codeunit the variable holds, so every implementation is a possible target.
    pub(crate) fn interface_call(
        &mut self,
        iface: usize,
        offset: usize,
        receiver: Option<&SyntaxNode>,
        args: &[SyntaxNode],
    ) -> L<ir::Expr> {
        let Some(receiver) = receiver else {
            return Err("an interface procedure called without a receiver is not lowered yet".into());
        };
        if !matches!(self.ty_ref(receiver), Ty::Object(ObjKind::Interface, _)) {
            return Err("an interface procedure called on something that is not an interface variable".into());
        }
        let sig = self.members[iface]
            .procedures
            .iter()
            .find(|p| p.offset == offset)
            .ok_or("interface procedure signature")?
            .clone();
        let mut targets = Vec::new();
        for &cu in self.implementers(iface).iter() {
            let same_shape: Vec<_> = self.members[cu]
                .procedures
                .iter()
                .filter(|p| {
                    !p.local
                        && p.name.eq_ignore_ascii_case(&sig.name)
                        && p.params.len() == sig.params.len()
                        && p.params.iter().zip(&sig.params).all(|(a, b)| a.by_var == b.by_var)
                })
                .collect();
            let exact = same_shape
                .iter()
                .find(|p| p.params.iter().zip(&sig.params).all(|(a, b)| a.ty == b.ty));
            let Some(imp) = exact.or(same_shape.first()) else {
                return Err(format!(
                    "codeunit \"{}\" implements \"{}\" without a procedure `{}`",
                    self.ws.objects[cu].name, self.ws.objects[iface].name, sig.name
                ));
            };
            let routine = self.routine_at.get(&(cu, imp.offset)).ok_or("procedure not found")?;
            self.calls.push((cu, routine));
            targets.push((cu, routine));
        }
        if targets.is_empty() {
            return Err(format!(
                "interface \"{}\" has no implementing codeunit in the workspace",
                self.ws.objects[iface].name
            ));
        }
        let receiver = Box::new(self.expr(receiver)?);
        let mut out = Vec::new();
        for (i, a) in args.iter().enumerate() {
            let p = sig.params.get(i).ok_or("too many arguments")?;
            if p.by_var {
                out.push(ir::Arg::Ref(self.place(a)?));
            } else {
                let value = self.expr(a)?;
                self.refuse_textconst_into_variant(&p.ty, a)?;
                out.push(ir::Arg::Value(self.convert_to(&p.ty, self.ty_ref(a), value)?));
            }
        }
        let params = {
            let node = ast::routines_in(self.ws.object_ast(iface).syntax())
                .into_iter()
                .find(|r| usize::from(r.syntax().text_range().start()) == offset)
                .ok_or("interface procedure node")?;
            super::localize::parameters_of(&node)
        };
        let at = self.expansion(super::localize::Expanded::InterfaceCall {
            interface: iface,
            name: sig.name.to_lowercase(),
            params,
            targets,
        });
        Ok(ir::Expr::InterfaceCall {
            receiver,
            targets: vec![(ir::walk::EXPAND, at)],
            args: out,
        })
    }
}

/// Whether converting a value of this type to a slot of the same type changes nothing.
fn is_identity_scalar(t: &RtType) -> bool {
    matches!(
        t,
        RtType::Boolean
            | RtType::Integer
            | RtType::BigInteger
            | RtType::Decimal
            | RtType::Date
            | RtType::Time
            | RtType::DateTime
            | RtType::Duration
            | RtType::Guid
    )
}
