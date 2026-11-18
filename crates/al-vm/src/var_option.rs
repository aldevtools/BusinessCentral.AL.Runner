//! An Option handed to a `var` parameter whose declared member list differs from the variable's (al-runner issues 7994 and
//! 8291): which members the parameter reads while the call runs, and which the caller's variable has after it. Not pinned:
//! assumption `var-option-caller-members` (corpus PR 1432, tests/al-language/types/enum-option/OptionVarMemberLists.al and
//! OptionVarMemberlessEvaluate.al; corpus PR 1439, OptionVarMemberlessWrites.al (OVMW) and OptionVarMemberlessCallers.al
//! (OVMC)). The callers the probes do not measure are refused where the call is lowered (al-sema `lower/var_option_arg.rs`).

use al_builtins::assumptions::{Assumption, assume};
use al_builtins::native::Signal;
use al_builtins::option_members::memberless;
use al_builtins::value::{RtType, Value};
use std::sync::Arc;

/// The value a `var` parameter of type `ty` takes when it is handed `v`; `None`: the usual conversion.
/// - Declared without members, it keeps the members of the value handed in: Format, StrSubstNo and Evaluate in the callee
///   read the caller's members (OptionVarMemberLists.al:241, :364, :510, :531; OptionVarMemberlessEvaluate.al:77-244;
///   OVMC:117-573).
/// - An array declared of Options without members shares the caller's elements, which keep their members (OptionVarMemberLists.al:468;
///   OVMW:472-513): the usual binding.
/// - A subscriber's parameter with members, handed a value with another list: refused (not measured). A procedure's or a
///   publisher's parameter with members reads its own (OptionVarMemberLists.al:398), the usual conversion.
pub(crate) fn var_binding(v: &Value, ty: &RtType, subscriber: bool) -> Option<Result<Value, Signal>> {
    let (Value::Option(_, has), RtType::Option(declared)) = (v, ty) else {
        if let (Value::Array(a), RtType::Array(_, el)) = (v, ty)
            && matches!(el.as_ref(), RtType::Option(d) if memberless(d))
            && matches!(a.borrow().first(), Some(Value::Option(_, m)) if !memberless(m))
        {
            assume(Assumption::VarOptionCallerMembers);
        }
        return None;
    };
    if memberless(declared) && !memberless(has) {
        assume(Assumption::VarOptionCallerMembers);
        return Some(Ok(v.clone()));
    }
    (subscriber && !memberless(declared) && has != declared).then(|| {
        Err(Signal::unsupported(
            "an event subscriber's `var` Option parameter with another member list than the value the publisher hands it (not measured, assumption var-option-caller-members)",
        ))
    })
}

/// The value a `var` argument's local or global Option variable of type `declared` takes back after the call, which left it
/// `v`; `current` is the variable's value now. The variable has its own members again: a declared list, or, declared
/// without members, the members it holds (a memberless caller stays memberless, OptionVarMemberLists.al:432; a memberless
/// `var` parameter handed on keeps its caller's, :364). After an empty body, `Value := 1` (:224), a write to another list
/// (`Value::C` gives Third, :398), a TryFunction and an error after the write (:552, :566), a store of any Option and Clear
/// (OVMW:226, :269). An array is not touched: its elements are the caller's (OVMW:472). `None`: `v` as it is.
pub(crate) fn written_back(v: &Value, declared: &RtType, current: impl FnOnce() -> Option<Value>) -> Option<Value> {
    let (Value::Option(o, has), RtType::Option(d)) = (v, declared) else {
        return None;
    };
    let own: Arc<[String]> = if memberless(d) {
        match current()? {
            Value::Option(_, m) => m,
            _ => return None,
        }
    } else {
        d.clone()
    };
    (own != *has).then(|| {
        assume(Assumption::VarOptionCallerMembers);
        Value::Option(*o, own)
    })
}

/// The value a `var` parameter slot declared `declared` takes when the routine stores `v` into it while it holds `current`
/// (`var_param`: the slot is a `var` parameter of the running routine). Declared without members and holding the caller's
/// members, it keeps them and takes the ordinal of `v`, whatever `v`'s members: an Integer, an Option with other members,
/// without members or with the same, Evaluate and Clear (OVMW:112, :130, :162, :212, :226, :269;
/// OptionVarMemberlessEvaluate.al:244). Any other value is stored as it is.
pub(crate) fn param_store(v: Value, declared: &RtType, var_param: bool, current: &Value) -> Value {
    match (&v, declared, current) {
        (Value::Option(o, _), RtType::Option(d), Value::Option(_, own))
            if var_param && memberless(d) && !memberless(own) =>
        {
            assume(Assumption::VarOptionCallerMembers);
            Value::Option(*o, own.clone())
        }
        _ => v,
    }
}

/// The value an element of an array of Options declared without members (`el`) takes when `v` is stored into it while it holds
/// `current` with members: a caller's members, in an element of a `var` or by-value array parameter or of an array CopyArray filled
/// (OVMW:472, :551, :568). `v` comes unconverted from `A[i] := x` (al-sema `memberless_option_array_assign`):
/// - an Integer, or an Option whose members share no name with the held ones and are as many: the held members, with the
///   ordinal (OVMW:472 `Option P,Q,R`, :496, :551, :568; OptionVarMemberLists.al:468);
/// - Evaluate's or Clear's value, with the held list itself: kept (OptionVarMemberlessEvaluate.al:552; OVMW:496, :513);
/// - an Option without members, or with the held members name by name: no members, the caller reads the number (OVMW:472).
///
/// Any other value refuses. `None`: the usual conversion.
pub(crate) fn element_store(v: &Value, el: &RtType, current: &Value) -> Option<Result<Value, Signal>> {
    let (RtType::Option(d), Value::Option(_, own)) = (el, current) else {
        return None;
    };
    if !memberless(d) || memberless(own) {
        return None;
    }
    assume(Assumption::VarOptionCallerMembers);
    Some(match v {
        Value::Integer(o) => Ok(Value::Option(*o, own.clone())),
        Value::Option(o, m) if Arc::ptr_eq(m, own) => Ok(Value::Option(*o, own.clone())),
        Value::Option(o, m) if memberless(m) || m == own => Ok(Value::Option(*o, d.clone())),
        Value::Option(o, m) if m.len() == own.len() && !m.iter().any(|n| own.contains(n)) => {
            Ok(Value::Option(*o, own.clone()))
        }
        _ => Err(Signal::unsupported(
            "a store into an element of an array of Options declared without members that holds its caller's members, of a value the probe does not measure (assumption var-option-caller-members)",
        )),
    })
}

impl crate::Session {
    /// The value a store of an Option into local `i` of the routine running in `f` leaves there (`param_store`); any other value
    /// is stored as is.
    pub(crate) fn option_param_store(&self, i: usize, v: Value, f: &crate::Frame) -> Value {
        if !matches!(v, Value::Option(..)) {
            return v;
        }
        let r = &self.engine.program.objects[f.object].routines[f.routine];
        let var_param = r.by_var.get(i).copied().unwrap_or(false);
        param_store(v, &r.locals[i].ty, var_param, &f.locals[i])
    }

    /// The refusal of a call of event publisher `routine` of `object` that hands `args` (with `values`, from frame `f`) to a
    /// `var` Option parameter with members that a subscriber declares without members: the subscriber reads the publisher's
    /// members (`var_binding`). Measured for a variable of the caller's `var` section (OptionVarMemberLists.al:531), a global
    /// (OVMC:249), a parameter passed by value (OVMC:196) and a record field whose captions are its members (OVMC:311); a
    /// `var` parameter handed on runs as before. From any other caller (a field whose captions differ, OVMC:290, an array
    /// element, a named return value) the call refuses when such a subscriber exists.
    pub(crate) fn var_option_subscriber_refusal(
        &mut self,
        object: usize,
        routine: usize,
        args: &[al_ir::Arg],
        values: &[Value],
        f: &mut crate::Frame,
    ) -> Result<(), Signal> {
        let engine = self.engine.clone();
        let r = &engine.program.objects[object].routines[routine];
        let ret_local = engine.program.objects[f.object].routines[f.routine].ret_local;
        for (i, a) in args.iter().enumerate() {
            let (Some(Value::Option(_, has)), Some(slot)) = (values.get(i), r.locals.get(i)) else {
                continue;
            };
            let RtType::Option(declared) = &slot.ty else {
                continue;
            };
            let candidate = r.by_var.get(i) == Some(&true) && !memberless(declared) && !memberless(has);
            let measured = match a {
                al_ir::Arg::Ref(al_ir::Place::Local(s)) => Some(*s) != ret_local,
                al_ir::Arg::Ref(al_ir::Place::Global(_)) => true,
                al_ir::Arg::Ref(al_ir::Place::Field(base, pos)) => self.captions_are_members(base, *pos, f),
                _ => false,
            };
            if candidate && !measured && self.memberless_subscriber_param(object, routine, &slot.name) {
                return Err(Signal::unsupported(
                    "an event subscriber's `var` Option parameter declared without members, handed the publisher's Option with members from a caller the probe does not measure (assumption var-option-caller-members)",
                ));
            }
        }
        Ok(())
    }

    /// Whether field `pos` of the record at `base` shows its members as they are: no OptionCaption and no OptionCaptionML, or an
    /// OptionCaption that names each member as it is (Base Application's "Item Entry Relation"."Source Subtype").
    fn captions_are_members(&mut self, base: &al_ir::Place, pos: usize, f: &mut crate::Frame) -> bool {
        let Ok(Value::Record(rec)) = self.read(base, f) else {
            return false;
        };
        let Ok(t) = self.table(&rec) else {
            return false;
        };
        let Some(RtType::Option(members)) = t.fields.get(pos).map(|d| &d.ty) else {
            return false;
        };
        match (
            self.field_property(&t, pos, "OptionCaption"),
            self.field_property(&t, pos, "OptionCaptionML"),
        ) {
            (None, None) => true,
            (Some(c), None) => c.trim_matches('\'').split(',').eq(members.iter().map(String::as_str)),
            _ => false,
        }
    }

    /// Whether a subscriber of event publisher `routine` of `object` declares its parameter `name` an Option without members.
    fn memberless_subscriber_param(&self, object: usize, routine: usize, name: &str) -> bool {
        let events = &self.engine.events;
        let Some(p) = events.publishers.get(&(object, routine)) else {
            return false;
        };
        let program = &self.engine.program;
        events
            .subscribers
            .get(&(p.object, p.event.clone()))
            .is_some_and(|subs| {
                subs.iter().any(|s| {
                    let r = &program.objects[s.object].routines[s.routine];
                    r.locals[..r.params.min(r.locals.len())].iter().any(|l| {
                        l.name.eq_ignore_ascii_case(name) && matches!(&l.ty, RtType::Option(m) if memberless(m))
                    })
                })
            })
    }
}
