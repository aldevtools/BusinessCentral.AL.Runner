//! A `var` Option argument whose declared member list differs from the parameter's (al-runner issues 7994 and 8291). The run
//! gives a parameter declared without members the caller's members, and the caller's variable its own back after the call
//! (al-vm `var_option`, assumption var-option-caller-members, corpus PR 1432: tests/al-language/types/enum-option/
//! OptionVarMemberLists.al and OptionVarMemberlessEvaluate.al). This module refuses the callers the probe does not measure.
use super::*;

/// What a `var` argument names, as far as the measured callers tell them apart.
#[derive(PartialEq)]
enum Caller {
    /// A variable declared in the routine's `var` section, or a parameter passed by value, which is a local variable of the
    /// routine too (not measured as such; Base Application's "Config. Package Management".ApplyPackageRecords hands one to an
    /// event, reached by the pinned corpus case table-objects Y76_Package_AutoIncrementField_NotInPackage).
    Local,
    Global,
    /// A field of a local record variable, without OptionCaption or OptionCaptionML.
    LocalRecordField,
    /// An Option field with OptionCaption or OptionCaptionML: the callee would read its captions, which the value does not carry.
    CaptionField,
    /// An element of a local array variable.
    LocalArrayElement,
    Other,
}

impl Lowerer<'_> {
    /// The refusal of a call that hands `a` (lowered to `arg`) to `var` parameter `param` of routine `routine` of `object`, or
    /// `None`. Measured, when the member lists differ name by name:
    /// - a parameter declared without members (`Option`, `Option ""`), the caller with members: a local
    ///   (OptionVarMemberLists.al MemberlessVar_EmptyBody_CallerFormat, MemberlessVar_CalleeFormat_CallerLocal), a global
    ///   (MemberlessVar_EmptyBody_Global), a field of a local record (MemberlessVar_CalleeFormat_RecordField), an element of a
    ///   local array (MemberlessVar_ArrayElementCaller); to an event publisher a local (Event_MemberlessPublisher) or a global
    ///   (not measured as such; Base Application's "Config. Package Management" hands its global ApplyMode to
    ///   OnInsertPackageRecordOnAfterDelayedInsert and compares it after the call, reached by the pinned corpus case
    ///   table-objects Y76_Package_AutoIncrementField_NotInPackage);
    /// - a parameter with members, the caller a local with other members or without members, not to an event publisher
    ///   (OtherMembersVar_CalleeFormatAndAssign, BlankMemberVar_EmptyBody, MemberlessCaller_MembersVar);
    /// - a `var` array parameter of memberless Options, the caller a local array with members (MemberlessVarArray_Callee).
    ///
    /// A record field or an array element handed to a parameter with members keeps main's result: the run converts the value
    /// to the parameter's list and the write back to the field's or the element's. A parameter with members of an array
    /// keeps main's result too. Every other caller refuses.
    pub(super) fn var_option_arg_refusal(
        &mut self,
        object: usize,
        routine: usize,
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
        let memberless = |m: &[String]| m.len() == 1 && m[0].is_empty();
        let publisher =
            self.routine_has_attribute(object, routine, &["IntegrationEvent", "BusinessEvent", "InternalEvent"]);
        let caller = self.var_option_caller(a, arg);
        let measured = match (memberless(&declared), memberless(&has), array) {
            (true, false, false) => match caller {
                Caller::Local | Caller::Global => true,
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
                    || (caller == Caller::Local && !publisher)
            }
            (true, false, true) => caller == Caller::Local && !publisher,
            (false, _, true) | (true, true, _) => true,
        };
        (!measured).then(|| {
            "an Option variable handed to a `var` Option parameter with another member list, from a caller the probe does not measure (assumption var-option-caller-members)".into()
        })
    }

    fn var_option_caller(&mut self, a: &SyntaxNode, arg: &ir::Arg) -> Caller {
        let declared_local = |l: &Self, p: &ir::Place| matches!(p, ir::Place::Local(s) if !l.var_params.get(*s).copied().unwrap_or(false));
        match arg {
            ir::Arg::Ref(p) if declared_local(self, p) => Caller::Local,
            ir::Arg::Ref(ir::Place::Global(_)) => Caller::Global,
            ir::Arg::Ref(ir::Place::Field(..)) if self.option_caption_owner(a).is_some() => Caller::CaptionField,
            ir::Arg::Ref(ir::Place::Field(base, _)) if declared_local(self, base) => Caller::LocalRecordField,
            ir::Arg::Ref(ir::Place::Index(base, _)) if declared_local(self, base) => Caller::LocalArrayElement,
            _ => Caller::Other,
        }
    }
}
