//! The values of `SessionInformation.SqlStatementsExecuted()` and `SqlRowsRead()` (issue 5580).
//!
//! BC counts real SQL work; the corpus pins properties of the count (session-system/session/SqlCounters.al:4: "only
//! properties are observed (sign of a difference, equal or not)"), and the size of the change only for the shapes
//! SqlCounters.al:204-395 measured (issue 5761). The runtime's counters are a model: the VM adds one for each pinned increase between two reads
//! (`al-vm/src/sqlcount.rs`). So a value read from a counter is tainted ([`Value::SqlCount`]): it may be stored in a
//! BigInteger or a Variant, passed on, and subtracted from another read of the same counter, and it may reach a comparison
//! only where the pins decide the answer:
//!
//! - a read against 0 with `>=` / `<` (not negative: SqlCounters.al:74-84, session-system/TestMiscTypes.al:97-114);
//! - two reads of the same counter with only classified segments between them (the same epoch), or a difference of two such
//!   reads against 0 or against another difference whose sign makes the answer certain (the sign of a change:
//!   SqlCounters.al:51-58, :105-202).
//!
//! Two reads of one chain have between them only segments whose size the corpus pins (al-vm `sqlcount.rs`): their
//! difference is that size, a plain BigInteger (SqlCounters.al:204-395). Where only a lower bound of a segment is pinned
//! (the first Insert into a table whose Global Triggers setup enables OnDatabaseInsert, SkSqlBudget.al:396-449), the
//! difference across it is [`SqlForm::AtLeast`]: it answers a comparison with a number only where the bound decides it.
//!
//! An Integer or a Decimal holds a read as a BigInteger does, and a Dictionary or List holds it as a value (not a key), with
//! the type of its slot kept (al-runner issue 6924; SqlCounters.al P6924_Counters_StoredInInteger_NotNegative,
//! P6924_Counters_StoredInDecimal_NotNegativeAndWhole, P6924_Counters_DictionaryOfGuidInteger_AddGetAndLaterDifference).
//! `Format` gives a marked text ([`crate::native::SQL_COUNT_TEXT_MARK`]): it may be passed on and logged
//! (P6924_Counters_FormattedInTelemetryDimension_NoError), and every text function that reads it refuses (`Txt::has_marker`).
//! `StrSubstNo` with a counter value (or its formatted text) as a value fills it in through `Format`, so its text is marked
//! the same way (al-runner issue 7129; al-vm `sqlcount::guard`).
//!
//! Two reads of one counter of different epochs (work no test pins ran between them) subtract to [`SqlForm::Unknown`]: it may
//! be stored, formatted and logged like the rest, and no arithmetic or comparison takes it (al-runner issue 6924).
//!
//! Every other use (another number, the digits of the text, a record field, a key, `*`, `/`) would show the model's number,
//! which nothing pins: it is refused.

use crate::native::Signal;
use crate::ops::Arith;
use crate::value::{SqlCount, SqlCounter, SqlForm, SqlType, Value};
use std::cmp::Ordering;

/// The refusal for a use the pins do not decide.
pub fn refusal(what: &str) -> Signal {
    Signal::unsupported(refusal_text(what))
}

/// The text of [`refusal`].
pub fn refusal_text(what: &str) -> String {
    format!(
        "{what} of a SQL counter value (SessionInformation.SqlStatementsExecuted or SqlRowsRead): the corpus pins only that a \
         counter is not negative and the sign of its change around pinned operations, never a count"
    )
}

/// Whether `v` is a text that holds a formatted counter value (`Format` of one, [`crate::native::SQL_COUNT_TEXT_MARK`]).
pub fn formatted(v: &Value) -> bool {
    match v {
        Value::Text(t) | Value::Code(t) => t.has_marker() && t.as_str().contains(crate::native::SQL_COUNT_TEXT_MARK),
        _ => false,
    }
}

/// An operand of a tainted operation.
enum Opnd {
    Read(i64, SqlCounter, u32, u32, u32),
    Delta(i64),
    AtLeast(i64),
    Zero,
    Num(i64),
    Other,
}

fn opnd(v: &Value) -> Opnd {
    match v {
        Value::SqlCount(c) => match c.form {
            SqlForm::Read {
                counter,
                epoch,
                chain,
                floor,
            } => Opnd::Read(c.n, counter, epoch, chain, floor),
            SqlForm::Delta => Opnd::Delta(c.n),
            SqlForm::AtLeast => Opnd::AtLeast(c.n),
            // Nothing is known of it: no arithmetic and no comparison takes it.
            SqlForm::Unknown => Opnd::Other,
        },
        Value::Integer(0) | Value::BigInteger(0) => Opnd::Zero,
        Value::Decimal(d) if d.is_zero() => Opnd::Zero,
        Value::Integer(n) => Opnd::Num(i64::from(*n)),
        Value::BigInteger(n) => Opnd::Num(*n),
        _ => Opnd::Other,
    }
}

fn delta(n: i64, ty: SqlType) -> Value {
    Value::SqlCount(SqlCount {
        n,
        form: SqlForm::Delta,
        ty,
    })
}

/// The number type of an operand: a counter value's slot, or the plain number's own type.
fn ty_of(v: &Value) -> SqlType {
    match v {
        Value::SqlCount(c) => c.ty,
        Value::Integer(_) => SqlType::Integer,
        Value::Decimal(_) => SqlType::Decimal,
        _ => SqlType::BigInteger,
    }
}

/// A plain number of type `ty` (a difference whose size the corpus pins).
fn number(n: i64, ty: SqlType) -> Value {
    match ty {
        SqlType::Integer => i32::try_from(n).map_or(Value::BigInteger(n), Value::Integer),
        SqlType::BigInteger => Value::BigInteger(n),
        SqlType::Decimal => Value::Decimal(n.into()),
    }
}

/// `v` with the type of its slot set to `ty`.
fn retyped(v: &Value, ty: SqlType) -> Value {
    match v {
        Value::SqlCount(c) => Value::SqlCount(SqlCount { ty, ..*c }),
        other => other.clone(),
    }
}

/// A counter value converted to a number type: it stays tainted, with the type of the slot (an Integer or Decimal slot,
/// SqlCounters.al P6924_Counters_StoredInInteger_NotNegative and P6924_Counters_StoredInDecimal_NotNegativeAndWhole);
/// `None` for any other type.
pub fn into_number(c: SqlCount, ty: SqlType) -> Value {
    Value::SqlCount(SqlCount { ty, ..c })
}

/// `Round(x, 1)` of a counter value: a count is whole, so it rounds to itself, a Decimal (SqlCounters.al
/// P6924_Counters_StoredInDecimal_NotNegativeAndWhole `Stmt = Round(Stmt, 1)`). `None` for any other precision.
pub fn round_whole(v: &Value, precision: Option<&Value>) -> Option<Value> {
    let one = match precision? {
        Value::Integer(n) => *n == 1,
        Value::BigInteger(n) => *n == 1,
        Value::Decimal(d) => *d == rust_decimal::Decimal::ONE,
        _ => false,
    };
    match v {
        Value::SqlCount(_) if one => Some(retyped(v, SqlType::Decimal)),
        _ => None,
    }
}

/// The sum of two differences, when its sign follows from theirs (both the same sign, or one of them 0).
fn certain_sum(a: i64, b: i64) -> Option<i64> {
    (a.signum() * b.signum() >= 0).then(|| a + b)
}

/// `l op r` where either side is a counter value.
pub fn arith(op: Arith, l: &Value, r: &Value) -> Result<Value, Signal> {
    let what = match op {
        Arith::Add | Arith::Sub => "arithmetic other than the difference of two reads",
        _ => "multiplication or division",
    };
    let neg = |b: i64| if op == Arith::Sub { -b } else { b };
    // The result has the wider of the two types, as AL arithmetic does (Integer, BigInteger, Decimal).
    let ty = ty_of(l).max(ty_of(r));
    let out = match (op, opnd(l), opnd(r)) {
        (Arith::Add | Arith::Sub, Opnd::Read(..) | Opnd::Delta(_) | Opnd::AtLeast(_), Opnd::Zero) => {
            Some(retyped(l, ty))
        }
        (Arith::Add, Opnd::Zero, Opnd::Read(..) | Opnd::Delta(_) | Opnd::AtLeast(_)) => Some(retyped(r, ty)),
        (Arith::Sub, Opnd::Zero, Opnd::Delta(b)) => Some(delta(-b, ty)),
        (Arith::Sub, Opnd::Read(a, ca, ea, ha, _), Opnd::Read(b, cb, eb, hb, _))
            if ca == cb && ea == eb && ha == hb =>
        {
            Some(number(a - b, ty))
        }
        // Across a change whose lower bound alone is pinned: at least the model's difference.
        (Arith::Sub, Opnd::Read(a, ca, ea, _, fa), Opnd::Read(b, cb, eb, hb, _))
            if ca == cb && ea == eb && fa != 0 && fa == hb =>
        {
            Some(Value::SqlCount(SqlCount {
                n: a - b,
                form: SqlForm::AtLeast,
                ty,
            }))
        }
        (Arith::Sub, Opnd::Read(a, ca, ea, ..), Opnd::Read(b, cb, eb, ..)) if ca == cb && ea == eb => {
            Some(delta(a - b, ty))
        }
        // Across work whose change no test pins (another epoch): a difference of unknown size and sign, which may only be
        // stored, formatted and logged (al-runner issue 6924, a telemetry helper that logs the statements a process ran;
        // SqlCounters.al P6924_Counters_FormattedInTelemetryDimension_NoError pins that logging the formatted difference
        // raises no error). Every use that reads it is refused ([`Opnd::Other`]).
        (Arith::Sub, Opnd::Read(a, ca, ..), Opnd::Read(b, cb, ..)) if ca == cb => Some(Value::SqlCount(SqlCount {
            n: a - b,
            form: SqlForm::Unknown,
            ty,
        })),
        (Arith::Add | Arith::Sub, Opnd::Delta(a), Opnd::Delta(b)) => certain_sum(a, neg(b)).map(|n| delta(n, ty)),
        _ => None,
    };
    out.ok_or_else(|| refusal(what))
}

/// The sign of `l - r` where the pins decide it, for an operator that needs the whole order.
fn certain_order(l: &Opnd, r: &Opnd) -> Option<Ordering> {
    match (l, r) {
        (Opnd::Read(a, ca, ea, ..), Opnd::Read(b, cb, eb, ..)) if ca == cb && ea == eb => Some(a.cmp(b)),
        (Opnd::Delta(a), Opnd::Delta(b)) => certain_sum(*a, -*b).map(|d| d.cmp(&0)),
        (Opnd::Delta(a), Opnd::Zero) => Some(a.cmp(&0)),
        (Opnd::Zero, Opnd::Delta(b)) => Some(0.cmp(b)),
        _ => None,
    }
}

/// The order of two values where either is a counter value, for a comparison that needs it whole (`=`, a `case`, a sort).
pub fn order(l: &Value, r: &Value) -> Result<Ordering, Signal> {
    certain_order(&opnd(l), &opnd(r)).ok_or_else(|| refusal("a comparison"))
}

/// The comparison operators on two values where either is a counter value. `>=` and `<` also answer for a read against 0,
/// which is never negative.
pub fn compare(op: Cmp, l: &Value, r: &Value) -> Result<bool, Signal> {
    let (a, b) = (opnd(l), opnd(r));
    if let Some(o) = certain_order(&a, &b) {
        return Ok(op.holds(o));
    }
    let num = |o: &Opnd| match o {
        Opnd::Zero => Some(0),
        Opnd::Num(k) => Some(*k),
        _ => None,
    };
    match (&a, &b) {
        (Opnd::AtLeast(m), o) if num(o).is_some() => {
            return at_least(op, *m, num(o).unwrap_or(0)).ok_or_else(|| refusal("a comparison"));
        }
        (o, Opnd::AtLeast(m)) if num(o).is_some() => {
            return at_least(op.flipped(), *m, num(o).unwrap_or(0)).ok_or_else(|| refusal("a comparison"));
        }
        _ => {}
    }
    match (&a, &b, op) {
        (Opnd::Read(..), Opnd::Zero, Cmp::Ge) | (Opnd::Zero, Opnd::Read(..), Cmp::Le) => Ok(true),
        (Opnd::Read(..), Opnd::Zero, Cmp::Lt) | (Opnd::Zero, Opnd::Read(..), Cmp::Gt) => Ok(false),
        _ => Err(refusal("a comparison")),
    }
}

/// A comparison operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cmp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

/// `x op k` for an `x` known only to be at least `m`, where that decides it.
fn at_least(op: Cmp, m: i64, k: i64) -> Option<bool> {
    match op {
        Cmp::Gt | Cmp::Ne => (m > k).then_some(true),
        Cmp::Ge => (m >= k).then_some(true),
        Cmp::Lt => (m >= k).then_some(false),
        Cmp::Le | Cmp::Eq => (m > k).then_some(false),
    }
}

impl Cmp {
    /// The operator with its operands swapped: `a op b` is `b op.flipped() a`.
    fn flipped(self) -> Cmp {
        match self {
            Cmp::Lt => Cmp::Gt,
            Cmp::Le => Cmp::Ge,
            Cmp::Gt => Cmp::Lt,
            Cmp::Ge => Cmp::Le,
            o => o,
        }
    }

    fn holds(self, o: Ordering) -> bool {
        match self {
            Cmp::Eq => o == Ordering::Equal,
            Cmp::Ne => o != Ordering::Equal,
            Cmp::Lt => o == Ordering::Less,
            Cmp::Le => o != Ordering::Greater,
            Cmp::Gt => o == Ordering::Greater,
            Cmp::Ge => o != Ordering::Less,
        }
    }
}

/// A counter value into a slot of type `t`: tainted in a Variant, a plain number with the slot's type in a number slot, a
/// refusal for any other type (see `convert`).
pub fn convert(c: SqlCount, v: Value, t: &crate::value::RtType) -> Result<Value, Signal> {
    use crate::value::RtType as T;
    use crate::value::SqlType;
    match t {
        T::Variant => Ok(v),
        T::BigInteger => Ok(into_number(c, SqlType::BigInteger)),
        T::Integer => Ok(into_number(c, SqlType::Integer)),
        T::Decimal => Ok(into_number(c, SqlType::Decimal)),
        _ => Err(refusal("a conversion to another type")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read_in(n: i64, epoch: u32, chain: u32) -> Value {
        Value::SqlCount(SqlCount {
            n,
            form: SqlForm::Read {
                counter: SqlCounter::Statements,
                epoch,
                chain,
                floor: 0,
            },
            ty: SqlType::BigInteger,
        })
    }

    /// A read whose chain is its number: two reads of one epoch with different numbers know only the sign of the change.
    fn read(n: i64, epoch: u32) -> Value {
        read_in(n, epoch, n as u32)
    }

    #[test]
    fn a_difference_of_two_reads_of_one_chain_is_its_size() {
        assert!(matches!(
            arith(Arith::Sub, &read_in(5, 1, 7), &read_in(2, 1, 7)).unwrap(),
            Value::BigInteger(3)
        ));
        assert!(matches!(
            arith(Arith::Sub, &read_in(5, 1, 7), &read_in(2, 1, 8)).unwrap(),
            Value::SqlCount(_)
        ));
        assert!(matches!(
            arith(Arith::Sub, &read_in(5, 2, 7), &read_in(2, 1, 7)).unwrap(),
            Value::SqlCount(SqlCount {
                form: SqlForm::Unknown,
                ..
            })
        ));
    }

    #[test]
    fn a_difference_across_epochs_takes_no_arithmetic_and_no_comparison() {
        let d = arith(Arith::Sub, &read(3, 2), &read(2, 1)).unwrap();
        assert!(arith(Arith::Add, &d, &Value::Integer(0)).is_err());
        assert!(arith(Arith::Sub, &d, &d).is_err());
        assert!(arith(Arith::Add, &d, &arith(Arith::Sub, &read(3, 1), &read(2, 1)).unwrap()).is_err());
        assert!(compare(Cmp::Ge, &d, &Value::Integer(0)).is_err());
        assert!(compare(Cmp::Eq, &d, &d).is_err());
        assert!(order(&d, &Value::Integer(0)).is_err());
        let other = Value::SqlCount(SqlCount {
            n: 1,
            form: SqlForm::Read {
                counter: SqlCounter::RowsRead,
                epoch: 1,
                chain: 1,
                floor: 0,
            },
            ty: SqlType::BigInteger,
        });
        assert!(arith(Arith::Sub, &read(3, 2), &other).is_err());
    }

    #[test]
    fn a_difference_of_two_reads_of_one_epoch_has_a_certain_sign() {
        let d = arith(Arith::Sub, &read(3, 1), &read(2, 1)).unwrap();
        assert!(compare(Cmp::Gt, &d, &Value::Integer(0)).unwrap());
        assert!(!compare(Cmp::Eq, &d, &Value::BigInteger(0)).unwrap());
        assert!(compare(Cmp::Gt, &read(3, 1), &read(2, 1)).unwrap());
        assert!(compare(Cmp::Gt, &d, &Value::Integer(1)).is_err());
        assert!(compare(Cmp::Eq, &read(3, 2), &read(2, 1)).is_err());
    }

    #[test]
    fn a_sum_of_differences_keeps_a_sign_only_when_they_agree() {
        let up = arith(Arith::Sub, &read(3, 1), &read(2, 1)).unwrap();
        let down = arith(Arith::Sub, &read(2, 1), &read(3, 1)).unwrap();
        let zero = arith(Arith::Sub, &read(2, 1), &read(2, 1)).unwrap();
        assert!(arith(Arith::Add, &up, &up).is_ok());
        assert!(arith(Arith::Add, &up, &down).is_err());
        assert!(compare(Cmp::Gt, &up, &zero).unwrap());
        assert!(compare(Cmp::Lt, &down, &up).unwrap());
        assert!(compare(Cmp::Lt, &up, &up).is_err());
    }

    #[test]
    fn a_read_against_zero_answers_only_not_negative() {
        assert!(compare(Cmp::Ge, &read(0, 1), &Value::Integer(0)).unwrap());
        assert!(!compare(Cmp::Lt, &read(0, 1), &Value::Integer(0)).unwrap());
        assert!(compare(Cmp::Le, &Value::Integer(0), &read(0, 1)).unwrap());
        assert!(compare(Cmp::Gt, &read(0, 1), &Value::Integer(0)).is_err());
        assert!(compare(Cmp::Eq, &read(0, 1), &Value::Integer(0)).is_err());
        assert!(arith(Arith::Add, &read(1, 1), &Value::Integer(1)).is_err());
        assert!(arith(Arith::Mul, &read(1, 1), &Value::Integer(1)).is_err());
    }

    #[test]
    fn a_difference_across_a_lower_bound_answers_only_what_the_bound_decides() {
        let floor = |n: i64, chain: u32, floor: u32| {
            Value::SqlCount(SqlCount {
                n,
                form: SqlForm::Read {
                    counter: SqlCounter::Statements,
                    epoch: 1,
                    chain,
                    floor,
                },
                ty: SqlType::BigInteger,
            })
        };
        let d = arith(Arith::Sub, &floor(7, 9, 4), &read_in(5, 1, 4)).unwrap();
        assert!(compare(Cmp::Gt, &d, &Value::BigInteger(1)).unwrap());
        assert!(compare(Cmp::Ge, &d, &Value::Integer(2)).unwrap());
        assert!(!compare(Cmp::Lt, &d, &Value::Integer(2)).unwrap());
        assert!(compare(Cmp::Lt, &Value::Integer(1), &d).unwrap());
        assert!(compare(Cmp::Le, &d, &Value::Integer(1000)).is_err());
        assert!(compare(Cmp::Gt, &d, &Value::Integer(2)).is_err());
        assert!(compare(Cmp::Eq, &d, &Value::Integer(2)).is_err());
        assert!(arith(Arith::Add, &d, &Value::Integer(1)).is_err());
        // Not from the chain the floor names: only the sign.
        assert!(matches!(
            arith(Arith::Sub, &floor(7, 9, 4), &read_in(5, 1, 3)).unwrap(),
            Value::SqlCount(SqlCount {
                form: SqlForm::Delta,
                ..
            })
        ));
    }
}
