//! Overloads of a procedure AL code declares when an argument is an enum or goes into an enum parameter (al-runner
//! issue 7445). What converts is pinned:
//! - an enum to an enum of another type only when the TARGET states `AssignmentCompatibility = true` (must-not-compile
//!   0074-enum-to-enum-target-without-assignment-compatibility.al:1, 0406-arg-other-enum-plain.al:1;
//!   types/enum-option/PnE13EnumConv.al:81-111 `TakeDst(S)`);
//! - an Integer to an enum (PnE13EnumConv.al:70-73 `TakeDst(I)`), and an Integer, BigInteger or Option to an enum and an
//!   enum to them by value (compile matrix, `check/conv.rs` `conv_accepts`);
//! - by `var`, only the same enum, or an Option for an enum and back (compile matrix arg_var, `conv_argument`).
//!
//! So when exactly one overload takes every argument it is the one BC calls (the others do not compile). When two or
//! more take every argument and one of them needs one of these conversions, which one BC calls is not pinned: the
//! checker notes AR1008 and the lowering refuses the call (hard rule 4).
use super::*;

/// How an argument fits a procedure parameter where an enum is involved.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum EnumFit {
    /// No enum on either side, or a type the front end cannot resolve: the other rules decide.
    NotEnum,
    /// The same enum.
    Same,
    /// A pinned conversion takes it.
    Converts,
    /// It does not compile.
    No,
}

impl<'a> Checker<'a> {
    /// How argument `i` of type `arg` fits parameter `i` of the procedure `target`.
    pub(crate) fn enum_arg_fit(&self, sc: &Scope, target: &Res, i: usize, arg: &Ty) -> EnumFit {
        let Res::Proc { object, offset } = target else {
            return EnumFit::NotEnum;
        };
        let Some(p) = self.members[*object]
            .procedure_at(*offset)
            .and_then(|s| s.params.get(i))
        else {
            return EnumFit::NotEnum;
        };
        let pfile = self.ws.objects[*object].file;
        let enum_at = |file: usize, t: &Ty| match t {
            Ty::Enum(n) => Some(self.resolve_quiet(file, ObjKind::Enum, n)),
            _ => None,
        };
        let (a, t) = (enum_at(sc.file, arg), enum_at(pfile, &p.ty));
        let numeric = |t: &Ty| matches!(t, Ty::Integer | Ty::BigInteger | Ty::Option(_));
        match (a, t) {
            (Some(Some(e)), Some(Some(f))) if e == f => EnumFit::Same,
            (Some(Some(_)), Some(Some(_))) if p.by_var => EnumFit::No,
            (Some(Some(_)), Some(Some(f))) if self.members[f].assignment_compatible => EnumFit::Converts,
            (Some(Some(_)), Some(Some(_))) => EnumFit::No,
            // An enum the front end cannot resolve is not judged.
            (Some(None), _) | (_, Some(None)) => EnumFit::NotEnum,
            (Some(Some(_)), None) | (None, Some(Some(_))) => {
                let other = if a.is_some() { &p.ty } else { arg };
                match (other, p.by_var) {
                    (Ty::Option(_), _) => EnumFit::Converts,
                    (o, false) if numeric(o) => EnumFit::Converts,
                    (o, true) if numeric(o) => EnumFit::No,
                    _ => EnumFit::NotEnum,
                }
            }
            (None, None) => EnumFit::NotEnum,
        }
    }

    /// The overload BC calls when two or more procedure overloads take every argument and one of them needs an enum
    /// conversion (`full`: each overload that takes every argument, and whether it needs one). An overload loses to
    /// another that declares the same parameter types except where it takes an argument less directly:
    /// - pinned (codeunits/CwInterfaceLeads.al:143-167 `CW_Overload_OptionVsEnum`): an Integer or Option argument takes an
    ///   Option parameter over an enum one; an enum argument takes its own enum over an Option parameter;
    /// - assumed (`enum-overload-own-type`, needed by System Application's Email Account Impl. `IsValidConnector(Enum)` and
    ///   `IsValidConnector(Integer)`, which the Email tests and Company-Initialize reach, and Base Application's Non-Deductible
    ///   VAT `GetNonDeductibleVATPct` on a General Posting Type or a Deferral Document Type, which Gen. Jnl.-Post Line
    ///   reaches): an Integer argument takes an Integer parameter over an enum one; an enum argument takes its own enum
    ///   over an Integer parameter or another enum.
    ///
    /// One overload left: it is returned (AR1009 notes an assumed choice for the lowering to mark). More than one left:
    /// AR1008, the lowering refuses the call, and `None`. No enum conversion involved: `None`, the usual choice stands.
    pub(crate) fn enum_overload_verdict<'c>(
        &mut self,
        sc: &Scope,
        node: &SyntaxNode,
        name: &str,
        full: &[(&'c Cand, bool)],
        args: &[Ty],
    ) -> Option<&'c Cand> {
        if full.len() < 2 || !full.iter().any(|(_, c)| *c) {
            return None;
        }
        let mut assumed = false;
        let left: Vec<&Cand> = full
            .iter()
            .filter(|(c, _)| {
                !full.iter().any(|(d, _)| match self.beats(sc, d, c, args) {
                    Beat::No => false,
                    Beat::Pinned => true,
                    Beat::Assumed => {
                        assumed = true;
                        true
                    }
                })
            })
            .map(|(c, _)| *c)
            .collect();
        if let [one] = left[..] {
            if assumed {
                self.diag(
                    sc,
                    node,
                    "AR1009",
                    format!("`{name}`: the overload that takes an Integer or enum argument as its own type is assumed"),
                );
            }
            return Some(one);
        }
        self.diag(
            sc,
            node,
            "AR1008",
            format!(
                "{} overloads of `{name}` take these arguments, one of them by an enum conversion: which overload BC calls is not pinned",
                left.len()
            ),
        );
        None
    }

    /// Whether overload `d` wins over `c` for these arguments (see `enum_overload_verdict`).
    fn beats(&self, sc: &Scope, d: &Cand, c: &Cand, args: &[Ty]) -> Beat {
        let params = |x: &Cand| match &x.target {
            Res::Proc { object, offset } => self.members[*object].procedure_at(*offset).map(|s| {
                (
                    self.ws.objects[*object].file,
                    s.params.iter().map(|p| p.ty.clone()).collect::<Vec<_>>(),
                )
            }),
            _ => None,
        };
        let (Some((dfile, dp)), Some((_, cp))) = (params(d), params(c)) else {
            return Beat::No;
        };
        let enum_of = |file: usize, t: &Ty| match t {
            Ty::Enum(n) => self.resolve_quiet(file, ObjKind::Enum, n),
            _ => None,
        };
        let mut beat = Beat::No;
        for ((a, dt), ct) in args.iter().zip(&dp).zip(&cp) {
            if dt.to_string().eq_ignore_ascii_case(&ct.to_string()) {
                continue;
            }
            let ae = enum_of(sc.file, a);
            let here = match (a, dt, ct) {
                (Ty::Integer | Ty::Option(_), Ty::Option(_), Ty::Enum(_)) => Beat::Pinned,
                (Ty::Enum(_), Ty::Enum(_), Ty::Option(_)) if ae.is_some() && ae == enum_of(dfile, dt) => Beat::Pinned,
                (Ty::Integer, Ty::Integer, Ty::Enum(_)) => Beat::Assumed,
                (Ty::Enum(_), Ty::Enum(_), Ty::Integer | Ty::Enum(_)) if ae.is_some() && ae == enum_of(dfile, dt) => {
                    Beat::Assumed
                }
                _ => return Beat::No,
            };
            beat = match (beat, here) {
                (Beat::Assumed, _) | (_, Beat::Assumed) => Beat::Assumed,
                _ => Beat::Pinned,
            };
        }
        beat
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Beat {
    No,
    Pinned,
    Assumed,
}
