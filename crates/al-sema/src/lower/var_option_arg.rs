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
    Global,
    /// A field of a record of the routine's `var` section, without OptionCaption or OptionCaptionML.
    LocalRecordField,
    /// An Option field with OptionCaption or OptionCaptionML: the callee would read its captions, which the value does not carry.
    CaptionField,
    /// An element of an array of the routine's `var` section.
    LocalArrayElement,
    Other,
}

const REFUSED: &str = "an Option variable handed to a `var` Option parameter with another member list, from a caller the probe does not measure (assumption var-option-caller-members)";

impl Lowerer<'_> {
    /// The refusal of a call that hands `a` (lowered to `arg`) to `var` parameter `param` of `target` (object, routine; `None`:
    /// an interface method, which is never an event publisher), or `None`. Measured, when the member lists differ name by name:
    /// - a parameter declared without members (`Option`, `Option ""`), the caller with members: a variable of the `var`
    ///   section (OptionVarMemberLists.al:104, :241), a global (:330), a field of a local record (:283), an element of a local
    ///   array (:491); to an event publisher a variable of the `var` section (:510, through a memberless `var` parameter that
    ///   stands for it);
    /// - to an event publisher's parameter declared without members, a global and a parameter passed by value: the pinned
    ///   corpus case table-objects/Y76PackageIds.al:78 runs Base Application's "Config. Package Management", which hands its
    ///   global ApplyMode and its by-value parameter ApplyMode (both `Option ,PrimaryKey,NonKeyFields`) to such parameters of
    ///   two integration events;
    /// - a parameter with members, the caller a variable of the `var` section with other members or without members, not to
    ///   an event publisher (:398, :582, :432);
    /// - a `var` array parameter of memberless Options, the caller an array of the `var` section with members (:468).
    ///
    /// A record field or an array element handed to a parameter with members keeps main's result: the run converts the value
    /// to the parameter's list and the write back to the field's or the element's. A parameter with members of an array
    /// keeps main's result too. Every other caller refuses.
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
        let publisher = target.is_some_and(|(object, routine)| {
            self.routine_has_attribute(object, routine, &["IntegrationEvent", "BusinessEvent", "InternalEvent"])
        });
        let caller = self.var_option_caller(a, arg);
        let measured = match (memberless(&declared), memberless(&has), array) {
            (true, false, false) => match caller {
                Caller::DeclaredLocal | Caller::Global => true,
                Caller::ByValueParameter => publisher,
                Caller::LocalRecordField | Caller::LocalArrayElement => !publisher,
                Caller::CaptionField => {
                    return Some(
                        "an Option field with an OptionCaption handed to a `var` Option parameter declared without members: the callee reads its captions, which the value does not carry (assumption var-option-caller-members)".into(),
                    );
                }
                Caller::Other => false,
            },
            (false, _, false) => {
                matches!(arg, ir::Arg::Ref(ir::Place::Field(..) | ir::Place::Index(..)))
                    || (caller == Caller::DeclaredLocal && !publisher)
            }
            (true, false, true) => caller == Caller::DeclaredLocal && !publisher,
            (false, _, true) | (true, true, _) => true,
        };
        (!measured).then(|| REFUSED.into())
    }

    fn var_option_caller(&mut self, a: &SyntaxNode, arg: &ir::Arg) -> Caller {
        let params = self.var_params.len();
        let declared_local =
            |l: &Self, p: &ir::Place| matches!(p, ir::Place::Local(s) if *s >= params && Some(*s) != l.named_ret);
        match arg {
            ir::Arg::Ref(p) if declared_local(self, p) => Caller::DeclaredLocal,
            ir::Arg::Ref(ir::Place::Local(s)) if *s < params && !self.var_params[*s] => Caller::ByValueParameter,
            ir::Arg::Ref(ir::Place::Global(_)) => Caller::Global,
            ir::Arg::Ref(ir::Place::Field(..)) if self.option_caption_owner(a).is_some() => Caller::CaptionField,
            ir::Arg::Ref(ir::Place::Field(base, _)) if declared_local(self, base) => Caller::LocalRecordField,
            ir::Arg::Ref(ir::Place::Index(base, _)) if declared_local(self, base) => Caller::LocalArrayElement,
            _ => Caller::Other,
        }
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

    /// The refusal of `Clear` of a memberless `var` Option parameter, of an array of them or of an element: Clear keeps the
    /// members the value holds, and what a read shows after it is not measured.
    pub(super) fn var_option_clear_refusal(&self, a: &SyntaxNode) -> Option<String> {
        self.memberless_var_option_param(a).map(|_| {
            "Clear of a `var` Option parameter declared without members, which may hold its caller's members (not measured, assumption var-option-caller-members)".into()
        })
    }

    /// The refusal of `target := value` into an element of a memberless `var` Option array parameter when `value` is not an
    /// Integer: measured are `A[1] := 1` (OptionVarMemberLists.al:468) and Evaluate (OptionVarMemberlessEvaluate.al:552).
    pub(super) fn var_option_element_store_refusal(&self, target: &SyntaxNode, value: &SyntaxNode) -> Option<String> {
        if target.kind() != SyntaxKind::IndexExpr || matches!(self.ty_of(value), Ty::Integer) {
            return None;
        }
        self.memberless_var_option_param(target).map(|_| {
            "a store of another Option or a value other than an Integer into an element of a `var` array of Options declared without members (not measured, assumption var-option-caller-members)".into()
        })
    }
}
