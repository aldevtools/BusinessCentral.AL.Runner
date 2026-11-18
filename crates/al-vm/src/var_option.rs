//! An Option handed to a `var` parameter whose declared member list differs from the variable's (al-runner issues 7994 and
//! 8291): which members the parameter reads while the call runs, and which the caller's variable has after it. Not pinned:
//! assumption `var-option-caller-members` (corpus PR 1432, tests/al-language/types/enum-option/OptionVarMemberLists.al and
//! OptionVarMemberlessEvaluate.al; corpus PR 1439, OptionVarMemberlessWrites.al (OVMW) and OptionVarMemberlessCallers.al
//! (OVMC); corpus PR 1444, OptionVarMemberlessUnheard.al (OVMU), an event publisher no subscriber hears). The callers the
//! probes do not measure are refused where the call is lowered (al-sema `lower/var_option_arg.rs`).

use al_builtins::assumptions::{Assumption, assume};
use al_builtins::builtin_id::{BuiltinId, ids};
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
/// - A subscriber's parameter with members, handed a value with another list: refused (not measured). A procedure's
///   parameter with members reads its own (OptionVarMemberLists.al:398), the usual conversion; a publisher's takes it as on
///   main (al-sema refuses the callers the probes do not measure).
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

/// Whether place `p`, handed to a `var` parameter of type `param` of an event publisher no subscriber hears, keeps its
/// value: with another member list than the parameter's, or an element of an array of memberless Options that holds its
/// caller's members. Such a call changes nothing: the caller's variable keeps its value and its members, and what follows
/// reads them (a local BC of the newest supported major, OVMU). Measured, and kept (the call writes nothing back): a
/// local (OVMU:115), a memberless local (:327), a parameter passed by value (:202), a `var` parameter with or without
/// members (:347) and a named return value (:372) of the running routine, a global of its object (:176), a field of a
/// local or `var` record, also one with captions (:216, :313), an element of a local, global or CopyArray filled array or
/// of a `var` or by-value array parameter (:244, :281): `Ok(true)`, flagged. A whole array handed to a memberless array
/// parameter shares the caller's elements, and its write-back changes nothing either (:244, :281): `Ok(false)`, main's
/// path. Every other place refuses: a global of a base object, of another extension or of a record's instance.
fn unheard_publisher_caller(p: &al_ir::Place, param: &RtType) -> Result<bool, Signal> {
    match (p, param) {
        (
            al_ir::Place::Local(_) | al_ir::Place::Global(_) | al_ir::Place::Field(..) | al_ir::Place::Index(..),
            RtType::Option(_),
        ) => {
            assume(Assumption::VarOptionCallerMembers);
            Ok(true)
        }
        (_, RtType::Array(..)) => Ok(false),
        _ => Err(Signal::unsupported(
            "an Option variable handed directly to an event publisher's `var` Option parameter with another member list, which no subscriber hears, from a place the probe does not measure (assumption var-option-caller-members)",
        )),
    }
}

/// The member list of an Option type, or of the elements of an array of Options.
fn option_members(t: &RtType) -> Option<Arc<[String]>> {
    match t {
        RtType::Option(m) => Some(m.clone()),
        RtType::Array(_, el) => option_members(el),
        _ => None,
    }
}

/// What stores a value into a place, told by the site of the store (`Session::write_as`).
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Store {
    /// `A[i] := x` and every other write of a value (`Session::write`).
    Assign,
    /// `A[i] := x`, `x` a variable of the running routine's `var` section, into an element of an array of memberless Options
    /// (al-sema marks it: `ir::Expr::VarSectionValue`, `Session::store_var_section`).
    AssignVarSection,
    /// The value a procedure's or an event publisher's `var` parameter hands back to its argument (`Session::write_back`).
    CallBack,
    /// A builtin's write-back of its `var` argument.
    Builtin(BuiltinId),
}

impl Store {
    /// Whether the store is an assignment: every store but a builtin's write-back.
    pub(crate) fn assigns(self) -> bool {
        !matches!(self, Store::Builtin(_))
    }
}

/// The array an element store goes into, as far as the measured stores tell them apart.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum ArrayOf {
    /// A `var` parameter of the running routine.
    VarParameter,
    /// A parameter of the running routine passed by value.
    ByValueParameter,
    /// An array of the running routine's `var` section.
    Declared,
    /// A global, or an array of another place.
    Other,
}

/// The value an element of an array of Options declared without members (`el`) takes when `how` stores `v` into it, in
/// `array`, while it holds `current` with members: a caller's members, of a `var` array parameter, of a by-value array
/// parameter, or CopyArray filled. `v` comes unconverted from `A[i] := x` (al-sema `memberless_option_array_assign`).
/// Measured (corpus PR 1439, OptionVarMemberlessWrites.al (OVMW); corpus PR 1432, OptionVarMemberlessEvaluate.al (OVME)):
/// - an Integer assigned keeps the held members, with the ordinal: in a `var` array parameter (OVMW:790, :801), a by-value
///   array parameter (OVMW:833) and a local array CopyArray filled (OVMW:560, :813);
/// - in a `var` array parameter only: Evaluate keeps them (OVME:552), and so does Clear of the element (OVMW:799); an Option
///   assigned from a variable of the routine's `var` section (`Store::AssignVarSection`): one whose members share no name with
///   the held ones and are as many keeps them with the ordinal (OVMW:762; OVMU:1022); one without members takes no members,
///   the caller reads the number (OVMW:771; OVMU:1016-1019).
///
/// One with the held members name by name refuses: it takes no members after a memberless store into the same element
/// (OVMW:484-491) but keeps them in a cleared array (OVMU:1020-1021), which the run cannot tell apart. Every other store
/// refuses: another store kind, another value or the value of another place (an element of an array, a
/// `var` parameter, a global: it may hold another list than its own), or another array. `None`: the usual conversion.
pub(crate) fn element_store(
    v: &Value,
    el: &RtType,
    current: &Value,
    how: Store,
    array: ArrayOf,
) -> Option<Result<Value, Signal>> {
    let (RtType::Option(d), Value::Option(_, own)) = (el, current) else {
        return None;
    };
    if !memberless(d) || memberless(own) {
        return None;
    }
    assume(Assumption::VarOptionCallerMembers);
    let var_array = array == ArrayOf::VarParameter;
    let kept = |o: i32| Ok(Value::Option(o, own.clone()));
    let refused = || {
        Err(Signal::unsupported(
            "a store the probe does not measure into an element of an array of Options declared without members that holds its caller's members (assumption var-option-caller-members)",
        ))
    };
    Some(match (how, v) {
        (Store::Assign, Value::Integer(o))
            if matches!(
                array,
                ArrayOf::VarParameter | ArrayOf::ByValueParameter | ArrayOf::Declared
            ) =>
        {
            kept(*o)
        }
        (Store::Builtin(id), Value::Option(o, _))
            if var_array && (id == ids::SYSTEM_EVALUATEMEMBERLESSVAROPTIONELEMENT_1 || id == ids::SYSTEM_CLEAR_1) =>
        {
            kept(*o)
        }
        (Store::AssignVarSection, Value::Option(o, m))
            if var_array && m.len() == own.len() && !m.iter().any(|n| own.contains(n)) =>
        {
            kept(*o)
        }
        (Store::AssignVarSection, Value::Option(o, m)) if var_array && memberless(m) => {
            Ok(Value::Option(*o, d.clone()))
        }
        _ => refused(),
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

    /// The value `how` leaves in the element of the array at `base` (elements of type `el`) that holds `current` when it stores
    /// `v`: `element_store`'s, else the usual conversion.
    pub(crate) fn element_value(
        &self,
        v: Value,
        el: &RtType,
        current: &Value,
        how: Store,
        base: &al_ir::Place,
        f: &crate::Frame,
    ) -> Result<Value, Signal> {
        match element_store(&v, el, current, how, self.array_of(base, f)) {
            Some(kept) => kept,
            None => al_builtins::ops::convert(v, el),
        }
    }

    /// The marks `finish_call` left for the frame of a routine with `var` Option parameters with members (`option_hops_of`);
    /// empty for any other routine.
    pub(crate) fn take_option_hops(&mut self, wanted: bool) -> Vec<bool> {
        if wanted {
            std::mem::take(&mut self.events.next_option_hops)
        } else {
            Vec::new()
        }
    }

    /// The array `base` of an element store is, for `element_store`.
    fn array_of(&self, base: &al_ir::Place, f: &crate::Frame) -> ArrayOf {
        let r = &self.engine.program.objects[f.object].routines[f.routine];
        match base {
            al_ir::Place::Local(s) if *s < r.params && r.by_var.get(*s) == Some(&true) => ArrayOf::VarParameter,
            al_ir::Place::Local(s) if *s < r.params => ArrayOf::ByValueParameter,
            al_ir::Place::Local(s) if Some(*s) != r.ret_local => ArrayOf::Declared,
            _ => ArrayOf::Other,
        }
    }

    /// Per parameter of procedure `routine` of `object`, called from frame `f` with `args` (`values`): whether the caller
    /// hands its `var` Option parameter declared with members a variable the probes measure handing on to an event publisher
    /// with a memberless subscriber (`var_option_subscriber_refusal`). Each measured cell has the caller's members equal to the
    /// parameter's and a variable of the caller's `var` section (OptionVarMemberLists.al:531), a global (OVMC:249), a
    /// parameter passed by value (OVMC:196) or a field without OptionCaption and OptionCaptionML of a record of the caller's
    /// `var` section (OVMC:311). A field whose captions differ shows its caption in the subscriber (OVMC:290), which the value
    /// does not carry.
    pub(crate) fn option_hops_of(
        &mut self,
        object: usize,
        routine: usize,
        args: &[al_ir::Arg],
        values: &[Value],
        f: &mut crate::Frame,
    ) -> Vec<bool> {
        let engine = self.engine.clone();
        let r = &engine.program.objects[object].routines[routine];
        let caller = &engine.program.objects[f.object].routines[f.routine];
        let declared = |s: usize| s >= caller.params && Some(s) != caller.ret_local;
        let mut out = vec![false; r.params.min(args.len())];
        for (i, hop) in out.iter_mut().enumerate() {
            let (Some(true), RtType::Option(d), Some(Value::Option(_, has))) =
                (r.by_var.get(i).copied(), &r.locals[i].ty, values.get(i))
            else {
                continue;
            };
            if memberless(d) || has != d {
                continue;
            }
            *hop = match &args[i] {
                al_ir::Arg::Ref(al_ir::Place::Local(s)) => {
                    declared(*s) || (*s < caller.params && caller.by_var.get(*s) == Some(&false))
                }
                al_ir::Arg::Ref(al_ir::Place::Global(_)) => true,
                al_ir::Arg::Ref(al_ir::Place::Field(base, pos)) => match base.as_ref() {
                    al_ir::Place::Local(s) if declared(*s) => self.field_without_captions(base, *pos, f),
                    _ => false,
                },
                _ => false,
            };
        }
        out
    }

    /// The refusal of a call of event publisher `routine` of `object` that hands `args` (from frame `f`) to its `var` Option
    /// parameters, or an array of them:
    /// - an argument whose declared member list differs from the parameter's, when a subscriber hears the event: the probes
    ///   raise every event through a procedure (OptionVarMemberLists.al:510, :531; OVMC:196-353), never directly. A call no
    ///   subscriber hears runs for the callers `unheard_publisher_caller` names; the result lists the arguments it keeps,
    ///   whose write-back the call skips (`Session::write_backs`).
    /// - a parameter with members that a subscriber declares without members: the subscriber reads the publisher's members
    ///   (`var_binding`). Measured only through a procedure's `var` parameter with the publisher's members that its caller
    ///   handed a variable `option_hops_of` names (OptionVarMemberLists.al:531; OVMC:196, :249, :311). Every other argument
    ///   refuses: a parameter handed on twice, a caller without members, a field whose captions differ (OVMC:290: BC shows
    ///   the caption).
    pub(crate) fn var_option_publisher_args(
        &mut self,
        object: usize,
        routine: usize,
        args: &[al_ir::Arg],
        values: &[Value],
        f: &crate::Frame,
    ) -> Result<Vec<usize>, Signal> {
        let mut kept = Vec::new();
        let engine = self.engine.clone();
        let r = &engine.program.objects[object].routines[routine];
        let running = &engine.program.objects[f.object].routines[f.routine];
        for (i, a) in args.iter().enumerate() {
            let Some(slot) = r.locals.get(i) else {
                continue;
            };
            if let (Some(true), al_ir::Arg::Ref(p)) = (r.by_var.get(i).copied(), a)
                && let Some(declared) = option_members(&slot.ty)
                && let Some(has) = option_members(&self.place_type(p, f))
            {
                let other = has != declared;
                // To a publisher no subscriber hears, an element of an array of memberless Options that holds its caller's
                // members too, whatever the parameter's list (OVMU:281).
                let element = memberless(&has)
                    && matches!(p, al_ir::Place::Index(..))
                    && matches!(values.get(i), Some(Value::Option(_, held)) if !memberless(held));
                if other || element {
                    if self.publisher_heard(object, routine) {
                        if other {
                            return Err(Signal::unsupported(
                                "an Option variable handed directly to an event publisher's `var` Option parameter with another member list, which a subscriber hears (not measured, assumption var-option-caller-members)",
                            ));
                        }
                    } else if unheard_publisher_caller(p, &slot.ty)? {
                        kept.push(i);
                    }
                }
            }
            let RtType::Option(declared) = &slot.ty else {
                continue;
            };
            if r.by_var.get(i) != Some(&true) || memberless(declared) {
                continue;
            }
            let measured = matches!(a, al_ir::Arg::Ref(al_ir::Place::Local(s))
                if *s < running.params
                    && running.by_var.get(*s) == Some(&true)
                    && running.locals.get(*s).map(|l| &l.ty) == Some(&slot.ty)
                    && f.option_hops.get(*s) == Some(&true));
            if !measured && self.memberless_subscriber_param(object, routine, &slot.name) {
                return Err(Signal::unsupported(
                    "an event subscriber's `var` Option parameter declared without members, handed the publisher's Option with members from a caller the probe does not measure (assumption var-option-caller-members)",
                ));
            }
        }
        Ok(kept)
    }

    /// Whether field `pos` of the record at `base` has neither OptionCaption nor OptionCaptionML.
    fn field_without_captions(&mut self, base: &al_ir::Place, pos: usize, f: &mut crate::Frame) -> bool {
        let Ok(Value::Record(rec)) = self.read(base, f) else {
            return false;
        };
        let Ok(t) = self.table(&rec) else {
            return false;
        };
        matches!(t.fields.get(pos).map(|d| &d.ty), Some(RtType::Option(_)))
            && self.field_property(&t, pos, "OptionCaption").is_none()
            && self.field_property(&t, pos, "OptionCaptionML").is_none()
    }

    /// Whether event publisher `routine` of `object` has a subscriber, bound or not, or one the run cannot raise
    /// (`Registry::is_subscribed`).
    fn publisher_heard(&self, object: usize, routine: usize) -> bool {
        let events = &self.engine.events;
        events
            .publishers
            .get(&(object, routine))
            .is_some_and(|p| events.is_subscribed(p.object, &p.event))
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
