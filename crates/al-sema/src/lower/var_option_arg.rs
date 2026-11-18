//! A `var` Option argument whose declared member list differs from the parameter's (al-runner issues 7994 and 8291). The run
//! gives a parameter declared without members the caller's members, and the caller's variable its own back after the call
//! (al-vm `var_option`, assumption var-option-caller-members, corpus PR 1432: tests/al-language/types/enum-option/
//! OptionVarMemberLists.al and OptionVarMemberlessEvaluate.al). This module refuses the callers the probe does not measure,
//! for a procedure's, an event publisher's and an interface method's `var` parameter alike, and the stores into a memberless
//! `var` Option parameter the probe does not measure.
use super::*;
use al_builtins::option_members::memberless;

/// What a `var` argument names, as far as the measured callers tell them apart.
#[derive(PartialEq)]
enum Caller {
    /// A variable of the routine's `var` section.
    DeclaredLocal,
    /// A parameter of the routine passed by value.
    ByValueParameter,
    /// The routine's named return value.
    NamedReturn,
    Global,
    /// A parameter of the routine passed by `var` and declared without members.
    MemberlessVarParameter,
    /// A parameter of the routine passed by `var` and declared with members.
    VarParameterWithMembers,
    /// A field of a local record (not the named return value) or of a global record, without OptionCaption or OptionCaptionML,
    /// or with an OptionCaption that names each member as it is.
    Field,
    /// An Option field whose OptionCaption or OptionCaptionML differs from its members: the callee would read its captions,
    /// which the value does not carry.
    CaptionField,
    /// An element of an array of the routine's `var` section, of a parameter passed by value, or of a global.
    Element,
    Other,
}

const REFUSED: &str = "an Option variable handed to a `var` Option parameter with another member list, from a caller the probe does not measure (assumption var-option-caller-members)";

impl Lowerer<'_> {
    /// The refusal of a call that hands `a` (lowered to `arg`) to `var` parameter `param` of procedure `target` (`None`: an
    /// interface method), or `None`. Measured, when the member lists differ name by name (corpus PR 1432, OptionVarMemberLists.al;
    /// corpus PR 1439, OptionVarMemberlessCallers.al (OVMC) and OptionVarMemberlessWrites.al (OVMW)):
    /// - a parameter declared without members, the caller with members: a variable of the `var` section (:104, :241, :510; OVMC:388),
    ///   a global (:330; OVMC:229, :481), a parameter passed by value (OVMC:117, :196), a named return value (OVMC:164), a field
    ///   of a local record (:283; OVMC:419), of a global record (OVMC:332, :511), of a `var` or by-value record parameter (OVMW:411;
    ///   OVMC:179) and of Rec (OVMW:447), an element of a local, global or by-value array (:491; OVMC:353, :573, :145). The
    ///   call kind does not change the result in any of these (procedure, interface method, event publisher); a caller kind
    ///   is run for all three.
    /// - a parameter with members: a record field or an array element (the usual conversion), a variable of the `var` section with
    ///   other members or without members (:398, :432; to an event publisher as to a procedure, OVMC:196), a parameter passed
    ///   by value (OVMC:131, :215), a memberless `var` parameter (it stands for its caller's variable, OVMW:269).
    /// - a procedure's or an interface method's `var` array parameter of memberless Options, the caller an array with members of
    ///   the `var` section or passed by value (:468; OVMC:145).
    ///
    /// Refused: a field whose captions differ from its members (BC shows the caption, OVMC:269, :290, :450, which the value
    /// does not carry); a `var` parameter with members as the caller of a memberless parameter (after a hop through other members a
    /// rejected Evaluate leaves the first member, OVMW:387, which the run cannot tell from a direct caller); every other caller.
    pub(super) fn var_option_arg_refusal(
        &mut self,
        target: Option<(usize, usize)>,
        param: &Ty,
        a: &SyntaxNode,
        arg: &ir::Arg,
    ) -> Option<String> {
        let (declared, has, array) = match (param, self.ty_of(a)) {
            (Ty::Option(d), Ty::Option(h)) => (d.clone(), h, false),
            (Ty::Array(_, d), Ty::Array(_, h)) => match (d.as_ref(), h.as_ref()) {
                (Ty::Option(d), Ty::Option(h)) => (d.clone(), h.clone(), true),
                _ => return None,
            },
            _ => return None,
        };
        if declared == has {
            return None;
        }
        let caller = self.var_option_caller(a, arg);
        let measured = match (memberless(&declared), memberless(&has), array) {
            (true, false, false) => match caller {
                Caller::DeclaredLocal
                | Caller::Global
                | Caller::ByValueParameter
                | Caller::NamedReturn
                | Caller::Field
                | Caller::Element => true,
                Caller::CaptionField => {
                    return Some(
                        "an Option field whose OptionCaption differs from its members handed to a `var` Option parameter declared without members: the callee reads its captions, which the value does not carry (assumption var-option-caller-members)".into(),
                    );
                }
                Caller::VarParameterWithMembers => {
                    return Some(
                        "a `var` parameter with members handed to a `var` Option parameter declared without members: a rejected Evaluate there is measured to leave the first member (assumption var-option-caller-members)".into(),
                    );
                }
                Caller::MemberlessVarParameter | Caller::Other => false,
            },
            (false, _, false) => {
                matches!(arg, ir::Arg::Ref(ir::Place::Field(..) | ir::Place::Index(..)))
                    || matches!(
                        caller,
                        Caller::DeclaredLocal | Caller::ByValueParameter | Caller::MemberlessVarParameter
                    )
            }
            (true, false, true) => {
                let publisher = target.is_some_and(|(o, r)| {
                    self.routine_has_attribute(o, r, &["IntegrationEvent", "BusinessEvent", "InternalEvent"])
                });
                !publisher && matches!(caller, Caller::DeclaredLocal | Caller::ByValueParameter)
            }
            (false, _, true) | (true, true, _) => true,
        };
        (!measured).then(|| REFUSED.into())
    }

    fn var_option_caller(&mut self, a: &SyntaxNode, arg: &ir::Arg) -> Caller {
        let params = self.var_params.len();
        let ir::Arg::Ref(place) = arg else {
            return Caller::Other;
        };
        let declared_or_by_value = |l: &Self, p: &ir::Place| match p {
            ir::Place::Local(s) => (*s >= params && Some(*s) != l.named_ret) || (*s < params && !l.var_params[*s]),
            ir::Place::Global(_) => true,
            _ => false,
        };
        match place {
            ir::Place::Local(s) if Some(*s) == self.named_ret => Caller::NamedReturn,
            ir::Place::Local(s) if *s >= params => Caller::DeclaredLocal,
            ir::Place::Local(s) if !self.var_params[*s] => Caller::ByValueParameter,
            ir::Place::Local(_) => match self.ty_of(a) {
                Ty::Option(m) if memberless(&m) => Caller::MemberlessVarParameter,
                Ty::Option(_) => Caller::VarParameterWithMembers,
                _ => Caller::Other,
            },
            ir::Place::Global(_) => Caller::Global,
            ir::Place::Field(base, _) if !self.measured_record(base) => Caller::Other,
            ir::Place::Field(..) if self.option_caption_owner(a).is_some() && !self.captions_name_members(a) => {
                Caller::CaptionField
            }
            ir::Place::Field(..) => Caller::Field,
            ir::Place::Index(base, _) if declared_or_by_value(self, base) => Caller::Element,
            _ => Caller::Other,
        }
    }

    /// Whether `base` is a record the probe hands a field of: a local of the routine (its `var` section, a `var` or by-value
    /// parameter, Rec) other than the named return value, or a global (OVMC:179, :332, :419, :511; OVMW:411, :447).
    fn measured_record(&self, base: &ir::Place) -> bool {
        match base {
            ir::Place::Local(s) => Some(*s) != self.named_ret,
            ir::Place::Global(_) => true,
            _ => false,
        }
    }

    /// Whether the Option field `a` names has an OptionCaption, and no OptionCaptionML, that names each member as it is
    /// (Base Application's "Item Entry Relation"."Source Subtype", `0,1,...,10`): its captions read as its members.
    fn captions_name_members(&self, a: &SyntaxNode) -> bool {
        let node = if a.kind() == SyntaxKind::ScopeExpr {
            a.children().next()
        } else {
            Some(a.clone())
        };
        let Some(Res::Field { table, id }) = node.and_then(|n| self.res(&n).cloned()) else {
            return false;
        };
        let Some(f) = self.members[table].fields.iter().find(|f| f.id == id) else {
            return false;
        };
        let Ty::Option(members) = &f.ty else {
            return false;
        };
        let ml = f
            .properties
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("OptionCaptionML"));
        let caption = f
            .properties
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("OptionCaption"))
            .and_then(|(_, v)| crate::members::first_string_literal(v));
        !ml && caption.is_some_and(|c| c.split(',').eq(members.iter().map(String::as_str)))
    }

    /// The `var` parameter of the routine being lowered that `a` names, or an element of, when it is declared as Options without
    /// members (`Option` or an array of it): it may hold its caller's members (al-vm `var_option`).
    pub(super) fn memberless_var_option_param(&self, a: &SyntaxNode) -> Option<usize> {
        let name = match a.kind() {
            SyntaxKind::NameRef => a.clone(),
            SyntaxKind::IndexExpr => a.children().next().filter(|n| n.kind() == SyntaxKind::NameRef)?,
            _ => return None,
        };
        let Some(Res::Local(n)) = self.res(&name) else {
            return None;
        };
        let slot = *self.locals.get(n)?;
        let declared = match self.ty_of(&name) {
            Ty::Option(m) => m,
            Ty::Array(_, el) => match *el {
                Ty::Option(m) => m,
                _ => return None,
            },
            _ => return None,
        };
        (self.var_params.get(slot).copied() == Some(true) && memberless(&declared)).then_some(slot)
    }

    /// `A[i] := x` into an element of an array of Options declared without members, `x` an Integer or an Option: the value goes
    /// in unconverted, so that the run tells an Integer, an Option without members and an Option with its own members apart
    /// in an element that holds a caller's members (al-vm `var_option::element_store`, corpus PR 1439,
    /// OptionVarMemberlessWrites.al:472). The run converts it as the assignment would in any other element. `A := B` of an
    /// array of Options with members into one of Options without members refuses: the probe measures CopyArray (:530, :551),
    /// not the assignment of the whole array. `None`: another statement.
    pub(super) fn memberless_option_array_assign(
        &mut self,
        n: &SyntaxNode,
        kids: &[SyntaxNode],
    ) -> Option<L<ir::Stmt>> {
        if !sig_tokens(n).iter().any(|t| t.kind() == SyntaxKind::Assign) {
            return None;
        }
        match (self.ty_of(&kids[0]), self.ty_of(&kids[1])) {
            (Ty::Array(_, to), Ty::Array(_, from)) => match (*to, *from) {
                (Ty::Option(to), Ty::Option(from)) if memberless(&to) && !memberless(&from) => Some(Err(
                    "the assignment of an array of Options with members to an array of Options declared without members (not measured, assumption var-option-caller-members)".into(),
                )),
                _ => None,
            },
            (Ty::Option(m), Ty::Integer | Ty::Option(_)) if kids[0].kind() == SyntaxKind::IndexExpr && memberless(&m) => {
                Some(self.place(&kids[0]).and_then(|place| Ok(ir::Stmt::Assign(place, self.expr(&kids[1])?))))
            }
            _ => None,
        }
    }
}
