//! A `var` Option argument whose declared member list differs from the parameter's (al-runner issues 7994 and 8291). The run
//! gives a parameter declared without members the caller's members, and the caller's variable its own back after the call
//! (al-vm `var_option`, assumption var-option-caller-members, corpus PR 1432: tests/al-language/types/enum-option/
//! OptionVarMemberLists.al and OptionVarMemberlessEvaluate.al; corpus PR 1439: OptionVarMemberlessCallers.al and
//! OptionVarMemberlessWrites.al). This module refuses each pair of call kind (procedure, interface method) and caller the
//! probes do not measure, leaves an event publisher called directly to the run (al-vm
//! `Session::var_option_publisher_refusal`), and routes `A[i] := x` into an array of memberless Options to the run.
use super::*;
use al_builtins::option_members::memberless;

/// The kinds of call the probes tell apart for a `var` Option argument.
#[derive(Clone, Copy, PartialEq)]
enum Call {
    /// A procedure of a codeunit, a table or the routine's own object.
    Procedure,
    /// An interface method.
    Interface,
}

/// The record a field argument belongs to.
#[derive(Clone, Copy, PartialEq)]
enum Record {
    /// A record of the routine's `var` section.
    Declared,
    Global,
    /// The implicit Rec of a table.
    Rec,
    VarParameter,
    ByValueParameter,
}

/// The array an element argument belongs to.
#[derive(Clone, Copy, PartialEq)]
enum Array {
    /// An array of the routine's `var` section.
    Declared,
    Global,
    ByValueParameter,
}

/// What a `var` argument names, as far as the measured callers tell them apart.
#[derive(Clone, Copy, PartialEq)]
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
    /// An Option field without OptionCaption and OptionCaptionML.
    Field(Record),
    /// An Option field whose OptionCaption names each member as it is (Base Application's "Item Entry Relation"."Source
    /// Subtype"): no probe table has one.
    CaptionNamesMembers,
    /// An Option field whose OptionCaption or OptionCaptionML differs from its members: the callee would read its captions,
    /// which the value does not carry.
    CaptionField,
    /// An element of an array.
    Element(Array),
    Other,
}

const REFUSED: &str = "an Option variable handed to a `var` Option parameter with another member list, from a caller the probe does not measure (assumption var-option-caller-members)";

/// Whether the probes measure `caller`, with members, handed by `call` to a `var` Option parameter declared without
/// members (corpus PR 1432, OptionVarMemberLists.al (OVML); corpus PR 1439, OptionVarMemberlessCallers.al (OVMC) and
/// OptionVarMemberlessWrites.al (OVMW)). An event's cells reach the publisher through a procedure (`RaiseMemberless`), so
/// they measure the procedure call.
fn measured_memberless(call: Call, caller: Caller) -> bool {
    use Call::{Interface, Procedure};
    matches!(
        (call, caller),
        // OVML:104, :241, :510
        (Procedure, Caller::DeclaredLocal)
            // OVML:330; OVMC:229
            | (Procedure, Caller::Global)
            // OVMC:117, :196
            | (Procedure, Caller::ByValueParameter)
            // OVMC:164
            | (Procedure, Caller::NamedReturn)
            // OVML:283
            | (Procedure, Caller::Field(Record::Declared))
            // OVMC:332
            | (Procedure, Caller::Field(Record::Global))
            // OVMW:411
            | (Procedure, Caller::Field(Record::VarParameter))
            // OVMC:179
            | (Procedure, Caller::Field(Record::ByValueParameter))
            // OVMW:447
            | (Procedure, Caller::Field(Record::Rec))
            // OVML:491; OVMC:353
            | (Procedure, Caller::Element(Array::Declared))
            // OVMC:145
            | (Procedure, Caller::Element(Array::ByValueParameter))
            // OVMC:388
            | (Interface, Caller::DeclaredLocal)
            // OVMC:419
            | (Interface, Caller::Field(Record::Declared))
            // OVMC:481
            | (Interface, Caller::Global)
            // OVMC:511
            | (Interface, Caller::Field(Record::Global))
            // OVMC:573
            | (Interface, Caller::Element(Array::Global))
    )
}

impl Lowerer<'_> {
    /// The refusal of a call that hands `a` (lowered to `arg`) to `var` parameter `param` of procedure `target` (`None`: an
    /// interface method), or `None`, when the member lists differ name by name (an event publisher: always `None`, the run
    /// decides):
    /// - a parameter declared without members, the caller with members: the pairs of call and caller `measured_memberless`
    ///   names.
    /// - a parameter with members: a record field or an array element (the usual conversion, as on main); from a procedure
    ///   call, a variable of the `var` section with other members or without members (OVML:398, :432) and a parameter passed
    ///   by value (OVMC:131, :215).
    /// - a procedure's `var` array parameter of memberless Options, the caller an array with members of the `var` section
    ///   or passed by value (OVML:468; OVMC:145).
    ///
    /// Refused: a field whose captions differ from its members (BC shows the caption, OVMC:269, :450, which the value does
    /// not carry); a `var` parameter with members as the caller of a memberless parameter (after a hop through other members
    /// a rejected Evaluate leaves the first member, OVMW:387, which the run cannot tell from a direct caller); every other
    /// pair of call and caller.
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
        let call = match target {
            None => Call::Interface,
            // An event publisher runs no code of its own: the run refuses the call where a subscriber hears it and keeps
            // main's result where none does (al-vm `Session::var_option_publisher_refusal`).
            Some((o, r))
                if self.routine_has_attribute(o, r, &["IntegrationEvent", "BusinessEvent", "InternalEvent"]) =>
            {
                return None;
            }
            Some(_) => Call::Procedure,
        };
        let caller = self.var_option_caller(a, arg);
        let measured = match (memberless(&declared), memberless(&has), array) {
            (true, false, false) => match caller {
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
                _ => measured_memberless(call, caller),
            },
            (false, _, false) => {
                matches!(arg, ir::Arg::Ref(ir::Place::Field(..) | ir::Place::Index(..)))
                    || (call == Call::Procedure && matches!(caller, Caller::DeclaredLocal | Caller::ByValueParameter))
            }
            (true, false, true) => {
                call == Call::Procedure && matches!(caller, Caller::DeclaredLocal | Caller::ByValueParameter)
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
            ir::Place::Field(base, _) => match self.record_of(base) {
                None => Caller::Other,
                Some(_) if self.option_caption_owner(a).is_some() && self.captions_name_members(a) => {
                    Caller::CaptionNamesMembers
                }
                Some(_) if self.option_caption_owner(a).is_some() => Caller::CaptionField,
                Some(record) => Caller::Field(record),
            },
            ir::Place::Index(base, _) => self.array_of(base).map_or(Caller::Other, Caller::Element),
            _ => Caller::Other,
        }
    }

    /// The kind of record `base` is, for a field argument; `None`: the named return value or a record of another place.
    fn record_of(&self, base: &ir::Place) -> Option<Record> {
        let params = self.var_params.len();
        match base {
            ir::Place::Local(s) if Some(*s) == self.named_ret => None,
            ir::Place::Local(s) if *s >= params => Some(Record::Declared),
            ir::Place::Local(s) if self.var_params[*s] => Some(Record::VarParameter),
            ir::Place::Local(_) => Some(Record::ByValueParameter),
            ir::Place::Global(s) if Some(*s) == self.rec_slot => Some(Record::Rec),
            ir::Place::Global(_) => Some(Record::Global),
            _ => None,
        }
    }

    /// The kind of array `base` is, for an element argument; `None`: a `var` parameter, the named return value or an array
    /// of another place.
    fn array_of(&self, base: &ir::Place) -> Option<Array> {
        let params = self.var_params.len();
        match base {
            ir::Place::Local(s) if Some(*s) == self.named_ret => None,
            ir::Place::Local(s) if *s >= params => Some(Array::Declared),
            ir::Place::Local(s) if !self.var_params[*s] => Some(Array::ByValueParameter),
            ir::Place::Global(_) => Some(Array::Global),
            _ => None,
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
    /// OptionVarMemberlessWrites.al:472); in any other element the run converts it as the assignment would. `A := B` of an
    /// array of Options with members into one of Options without members refuses: the probe notes that BC does not compile
    /// it (AL0145, OptionVarMemberlessWrites.al:31) and measures CopyArray instead (:530, :551). `None`: another statement.
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
                    "the assignment of an array of Options with members to an array of Options declared without members: the probe notes that BC does not compile it (AL0145, assumption var-option-caller-members)".into(),
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
