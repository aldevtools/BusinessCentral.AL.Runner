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
/// record of the running routine's `var` section or of a `var` record parameter, also one with captions (:216, :313), an
/// element of a local, global or CopyArray filled array or of a `var` or by-value array parameter (:244, :281): `Ok(true)`,
/// flagged. A whole array handed to a memberless array parameter shares the caller's elements, and its write-back changes
/// nothing either (:244, :281): `Ok(false)`, main's path. A field of any other record: `Ok(false)`, main's write-back,
/// which stores the value the field held. Every other place refuses: a global of a base object, of another extension or of a record's instance.
fn unheard_publisher_caller(p: &al_ir::Place, param: &RtType, running: &al_ir::Routine) -> Result<bool, Signal> {
    // The records whose fields the probe hands on: a record of the running routine's `var` section (:216) and a `var`
    // record parameter (:313).
    let measured_record = |base: &al_ir::Place| match base {
        al_ir::Place::Local(s) if *s < running.params => running.by_var.get(*s) == Some(&true),
        al_ir::Place::Local(s) if running.ret_local == Some(*s) => false,
        al_ir::Place::Local(_) => true,
        _ => false,
    };
    match (p, param) {
        (al_ir::Place::Local(_) | al_ir::Place::Global(_) | al_ir::Place::Index(..), RtType::Option(_)) => {
            assume(Assumption::VarOptionCallerMembers);
            Ok(true)
        }
        (al_ir::Place::Field(base, _), RtType::Option(_)) if measured_record(base) => {
            assume(Assumption::VarOptionCallerMembers);
            Ok(true)
        }
        (al_ir::Place::Field(..), RtType::Option(_)) | (_, RtType::Array(..)) => Ok(false),
        _ => Err(Signal::unsupported(
            "an Option variable handed directly to an event publisher's `var` Option parameter with another member list, which no subscriber hears, from a place the probe does not measure (assumption var-option-caller-members)",
        )),
    }
}

/// The refusal of an error that ends a call `Session::heard_publisher_caller` lets run: on BC the caller's variable keeps
/// what the subscriber wrote before it raised the error (OptionVarMemberlessHeard.al:368, :371), and the run keeps only what
/// the subscribers before it wrote back (`Session::publish`).
pub(crate) fn heard_publisher_error() -> Signal {
    Signal::unsupported(
        "an error raised while an event publisher runs that was handed directly an Option variable of another member list: BC keeps the write the subscriber made before it (assumption var-option-caller-members)",
    )
}

/// An argument of a heard publisher call that `Session::heard_publisher_caller` lets run.
pub(crate) struct HeardArg {
    /// The argument's position.
    i: usize,
    /// The ordinal the caller handed in.
    handed: i32,
    /// The number of the publisher parameter's members.
    members: usize,
    /// The place takes back any number: a variable of the running routine's `var` section, measured with a subscriber
    /// that stores 7 or -1 (OptionVarMemberlessHeard.al:493).
    any_number: bool,
}

/// The refusal of a heard publisher call whose subscribers left, in argument `HeardArg::i`, a number outside the publisher's
/// member list that the caller did not hand in, from a place no cell measures with such a number (OptionVarMemberlessHeard.al:493
/// measures it for a variable of the routine's `var` section only).
pub(crate) fn heard_finals(heard: &[HeardArg], finals: &[Value]) -> Result<(), Signal> {
    for a in heard {
        if let Some(Value::Option(o, _)) = finals.get(a.i)
            && !a.any_number
            && *o != a.handed
            && usize::try_from(*o).map_or(true, |o| o >= a.members)
        {
            return Err(Signal::unsupported(
                "an event subscriber left a number outside the publisher's member list in an Option handed directly to the publisher's `var` parameter, from a place the probe does not measure with such a number (assumption var-option-caller-members)",
            ));
        }
    }
    Ok(())
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

/// Clear of a whole array declared of Options with members (`declared`) gives an element that a memberless store or CopyArray
/// left without members its declared members back: the stores that follow read them (OVMU:984-1003, :1020-1024). Flagged.
/// Every other value: `v` as it is.
pub(crate) fn cleared_array(v: Value, declared: &RtType, id: BuiltinId) -> Value {
    if id == ids::SYSTEM_CLEAR_1
        && let (Value::Array(a), RtType::Array(_, el)) = (&v, declared)
        && let RtType::Option(d) = el.as_ref()
        && !memberless(d)
        && matches!(a.borrow().first(), Some(Value::Option(..)))
    {
        for e in a.borrow_mut().iter_mut() {
            if let Value::Option(o, m) = e
                && memberless(m)
            {
                assume(Assumption::VarOptionCallerMembers);
                *e = Value::Option(*o, d.clone());
            }
        }
    }
    v
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
/// One with the held members keeps them with the ordinal (OVMU:1020-1021, in an array Clear gave back its members after a
/// memberless store, OVMU:988-1003).
///
/// An element declared with members that holds none refuses every store: a memberless store or CopyArray from memberless
/// Options left it so (OVMW:771, :539), and on BC it keeps no members when the caller assigns it, so the held list then
/// takes no members either (OVMW:483-491); the run does not model that state. Refusing the caller's store keeps the state
/// from reaching the arms above. Its reads stay (OVMW:488-489). Every other store refuses: another store kind, another value or the value of another place (an element of an array, a
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
    match (memberless(d), memberless(own)) {
        (true, false) => {}
        (false, true) => {
            return Some(Err(Signal::unsupported(
                "a store into an element of an array of Options declared with members that holds none, which a memberless store or CopyArray left (not measured, assumption var-option-caller-members)",
            )));
        }
        _ => return None,
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
        (Store::AssignVarSection, Value::Option(o, m)) if var_array && m == own => kept(*o),
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
    /// - an argument whose declared member list differs from the parameter's, when a subscriber hears the event: runs for the
    ///   callers `heard_publisher_caller` names (the result's flag: an error the call ends in refuses). A call no
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
        f: &mut crate::Frame,
    ) -> Result<(Vec<usize>, Vec<HeardArg>), Signal> {
        let mut kept = Vec::new();
        let mut heard = Vec::new();
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
                            heard.push(self.heard_publisher_caller(object, routine, i, p, values.get(i), f)?);
                        }
                    } else if unheard_publisher_caller(p, &slot.ty, running)? {
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
        Ok((kept, heard))
    }

    /// Whether field `pos` of the record at `base` has neither OptionCaption nor OptionCaptionML.
    fn field_without_captions(&self, base: &al_ir::Place, pos: usize, f: &crate::Frame) -> bool {
        let RtType::Record(id, _) = self.place_type(base, f) else {
            return false;
        };
        let Some(Some(t)) = self.engine.tables.get(id as usize) else {
            return false;
        };
        matches!(t.fields.get(pos).map(|d| &d.ty), Some(RtType::Option(_)))
            && self.field_property(t, pos, "OptionCaption").is_none()
            && self.field_property(t, pos, "OptionCaptionML").is_none()
    }

    /// Whether place `p`, holding `value` and handed directly to `var` parameter `param` of event publisher `object` with
    /// another member list, is a caller the probe measures when a subscriber hears the event: BC binds the parameter as a
    /// procedure's with members, so the subscriber reads the publisher's members and the caller's variable takes back the
    /// ordinal with its own members (main's binding, `ops::convert` in `Session::publish`, and write-back, `written_back`;
    /// corpus probe tests/al-language/types/enum-option/OptionVarMemberlessHeard.al (OVH), on a local BC of the newest
    /// supported major; Base Application's codeunit 5407 hands its memberless NewStatusOption to
    /// OnAfterToProdOrderLineModify). Every cell has a codeunit publisher whose parameter has members and a caller in a
    /// codeunit, holding the members of its declared type: a variable of the running routine's `var` section, with members
    /// or without, of any list length, holding any number and taking back any number a subscriber leaves (OVH:155-173,
    /// :198-214, :444-546; with members, no number below 0 is handed in), and, with the publisher's list length or without
    /// members and an ordinal inside the publisher's list, a parameter passed by value (:262, :265), a named return value
    /// with members (:276), a field of a record of the `var` section, also one with captions (:298-310), of a global record
    /// or of a `var` record parameter, without captions (:313, :315), and an element of an array of the `var` section, with
    /// members or without, or of a global array with members (:338-350). The subscribers' shape does not matter within the
    /// contract `heard_contract` gates: SingleInstance codeunits, both skip flags and two subscribers in either run order
    /// give the same (:548). A number outside the publisher's list that a subscriber leaves in another place refuses after
    /// the call (`heard_finals`).
    /// Refused: a `var` parameter of the running routine handed on, measured to show the subscriber the members of the value
    /// its own caller handed (:268-274), and a global, which its subscribers alias (`Session::publish`), so they would read
    /// its own list where BC shows the publisher's (:232-244); every place, publisher and parameter the cells do not have.
    /// An error raised while the call runs refuses too (`heard_publisher_error`).
    fn heard_publisher_caller(
        &mut self,
        object: usize,
        routine: usize,
        i: usize,
        p: &al_ir::Place,
        value: Option<&Value>,
        f: &mut crate::Frame,
    ) -> Result<HeardArg, Signal> {
        let place = self.heard_place(object, routine, i, p, value, f);
        let (Some(any_number), true, Some(Value::Option(handed, _)), RtType::Option(d)) = (
            place,
            self.heard_contract(object, routine, i, f),
            value,
            &self.engine.program.objects[object].routines[routine].locals[i].ty,
        ) else {
            return Err(Signal::unsupported(
                "an Option variable handed directly to an event publisher's `var` Option parameter with another member list, which a subscriber hears, from a place the probe does not measure (assumption var-option-caller-members)",
            ));
        };
        assume(Assumption::VarOptionCallerMembers);
        Ok(HeardArg {
            i,
            handed: *handed,
            members: d.len(),
            any_number,
        })
    }

    /// Whether the event and the routine that raises it are those of every cell of OptionVarMemberlessHeard.al: an
    /// `IntegrationEvent(false, false)` of a codeunit that declares one `var` Option parameter, not isolated, raised from a
    /// procedure of a codeunit (no trigger, no attribute other than `Test`), whose subscribers are codeunits' static ones
    /// that take parameter `i` by `var` with the publisher's member list or do not take it.
    fn heard_contract(&self, object: usize, routine: usize, i: usize, f: &crate::Frame) -> bool {
        let program = &self.engine.program;
        let events = &self.engine.events;
        let codeunit = |o: usize| program.objects[o].kind == al_ir::ObjKind::Codeunit;
        let r = &program.objects[object].routines[routine];
        let running = &program.objects[f.object].routines[f.routine];
        let Some(publisher) = events.publishers.get(&(object, routine)) else {
            return false;
        };
        let Some(slot) = r.locals.get(i) else {
            return false;
        };
        let options = (0..r.params)
            .filter(|&j| r.by_var.get(j) == Some(&true) && option_members(&r.locals[j].ty).is_some())
            .count();
        let subscribers_measured = events
            .subscribers
            .get(&(publisher.object, publisher.event.clone()))
            .is_some_and(|subs| {
                !subs.is_empty()
                    && subs.iter().all(|s| {
                        let sr = &program.objects[s.object].routines[s.routine];
                        !s.manual
                            && s.element.is_empty()
                            && codeunit(s.object)
                            && sr.locals[..sr.params.min(sr.locals.len())]
                                .iter()
                                .enumerate()
                                .all(|(k, l)| {
                                    !l.name.eq_ignore_ascii_case(&slot.name)
                                        || (sr.by_var.get(k) == Some(&true) && l.ty == slot.ty)
                                })
                    })
            });
        codeunit(object)
            && codeunit(f.object)
            && publisher.kind == crate::events::EventKind::Integration
            && !publisher.include_sender
            && !publisher.global_var_access
            && !publisher.isolated
            && options == 1
            && !running.is_trigger
            && running.attributes.iter().all(|a| a.name.eq_ignore_ascii_case("Test"))
            && subscribers_measured
    }

    /// Whether place `p`, holding `value`, is a caller `heard_publisher_caller` names for parameter `i` of publisher
    /// `routine` of `object`: `None` when it is not, else whether it takes back any number.
    fn heard_place(
        &mut self,
        object: usize,
        routine: usize,
        i: usize,
        p: &al_ir::Place,
        value: Option<&Value>,
        f: &mut crate::Frame,
    ) -> Option<bool> {
        use al_ir::Place::{Field, Global, Index, Local};
        let engine = self.engine.clone();
        let r = &engine.program.objects[f.object].routines[f.routine];
        let declared = |s: usize| s >= r.params && Some(s) != r.ret_local;
        let by_var = |s: usize| s < r.params && r.by_var.get(s) == Some(&true);
        let param = &engine.program.objects[object].routines[routine].locals[i].ty;
        let (RtType::Option(d), Some(Value::Option(o, held)), Some(has)) =
            (param, value, option_members(&self.place_type(p, f)))
        else {
            return None;
        };
        if memberless(d) || *held != has {
            return None;
        }
        // OVH:493: a variable of the `var` section holding a number outside the publisher's list, -1 only without members.
        if let Local(s) = p
            && declared(*s)
        {
            return (*o >= 0 || memberless(&has)).then_some(true);
        }
        let past = usize::try_from(*o).map_or(true, |o| o >= d.len());
        if past || !(memberless(&has) || has.len() == d.len()) {
            return None;
        }
        let measured = match p {
            Local(s) if r.ret_local == Some(*s) => !memberless(&has),
            Local(s) => !by_var(*s),
            Field(base, pos) => {
                let measured = match base.as_ref() {
                    Local(s) if declared(*s) => true,
                    Local(s) if by_var(*s) => self.field_without_captions(base, *pos, f),
                    Global(_) => self.field_without_captions(base, *pos, f),
                    _ => false,
                };
                measured && matches!(self.read(base, f), Ok(Value::Record(rec)) if !rec.borrow().temporary)
            }
            Index(base, _) => match base.as_ref() {
                Local(s) => declared(*s),
                Global(_) => !memberless(&has),
                _ => false,
            },
            _ => false,
        };
        measured.then_some(false)
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
