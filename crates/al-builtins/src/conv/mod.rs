//! Implicit conversions from the run matrix of corpus writer Y31 (`tests/al-language/types/operators-expressions/`):
//! the table of its cells (`rows.rs`, generated from the corpus files by `tests/conv_matrix_rows.rs`) and the engine that
//! converts a value of a covered cell (`engine.rs`). A cell the matrix does not cover keeps the conversion of `ops::convert`.
//!
//! Implicit conversions at run time from the run matrix of corpus writer Y31

mod arith;
pub mod byte_char;
mod compare;
mod engine;
mod rows;
pub mod text_constant;
mod variant;
mod variant_records;
mod variant_scalars;

pub use arith::arith_cell;
pub(crate) use arith::datetime_fraction_error;
pub use compare::compare_cell;
pub use engine::{convert_cell, variant_convert, variant_error};
pub use variant::{AS_CONDITION, AT_ASSIGNMENT, REGISTRY, kind_code, variant_into};
pub use variant_records::record_slot_code;

/// A type of the matrix. Text[N] and Text are one kind, as are the Code lengths and an Enum with or without an interface.
/// `Variant` stays the last kind: it sizes the table of [`verdict`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    Integer,
    BigInteger,
    Decimal,
    Char,
    Byte,
    Option,
    Enum,
    Text,
    Code,
    Label,
    Guid,
    Duration,
    Boolean,
    Date,
    Time,
    DateTime,
    Variant,
}

/// What a family of the matrix measures. `AddAssign` stays the last family: it sizes the table of [`verdict`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Family {
    /// `:=`, a by-value argument and `exit`, from a variable, a field or a literal of the source kind.
    Cv,
    /// The same, from a Variant that holds a value of the source kind.
    CvVariant,
    /// `B ? t : f`: the true branch's kind, then the false branch's.
    Cd,
    /// `=` and `<>`, `<`, `in [...]` and a case label: the left (selector) kind, then the right (element).
    CmpEq,
    CmpLt,
    CmpIn,
    CmpCase,
    /// `+` and `-`: the left kind, then the right.
    Add,
    Sub,
    /// `t += v`: the kind of the value `v`, then the target's.
    AddAssign,
}

/// What BC gave for the values of a cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// A value for every value tried.
    Values,
    /// A value for some, an error for others, or different errors.
    Mixed,
    /// This error (code, text) for every value tried.
    Error(&'static str, &'static str),
}

/// The number of [`Kind`]s and of [`Family`]s, for the dense table of [`verdict`]. They are the last variant's number plus
/// one. The matches below name every variant, so a variant added to either enum does not compile until it is listed there;
/// the assertion then fails unless the last variant is still the one these counts use.
const KINDS: usize = Kind::Variant as usize + 1;
const FAMILIES: usize = Family::AddAssign as usize + 1;

const fn is_last_kind(k: Kind) -> bool {
    match k {
        Kind::Variant => true,
        Kind::Integer
        | Kind::BigInteger
        | Kind::Decimal
        | Kind::Char
        | Kind::Byte
        | Kind::Option
        | Kind::Enum
        | Kind::Text
        | Kind::Code
        | Kind::Label
        | Kind::Guid
        | Kind::Duration
        | Kind::Boolean
        | Kind::Date
        | Kind::Time
        | Kind::DateTime => false,
    }
}

const fn is_last_family(f: Family) -> bool {
    match f {
        Family::AddAssign => true,
        Family::Cv
        | Family::CvVariant
        | Family::Cd
        | Family::CmpEq
        | Family::CmpLt
        | Family::CmpIn
        | Family::CmpCase
        | Family::Add
        | Family::Sub => false,
    }
}

const _: () = assert!(is_last_kind(Kind::Variant) && is_last_family(Family::AddAssign));

/// The verdict of a cell, `None` when the matrix does not cover it. Every comparison and assignment of a number, a Char or a
/// Byte asks it, so the table is indexed, not hashed. A cell listed twice keeps its last row.
pub fn verdict(family: Family, src: Kind, dst: Kind) -> Option<Verdict> {
    static CELLS: std::sync::OnceLock<Vec<Option<Verdict>>> = std::sync::OnceLock::new();
    let at = |f: Family, s: Kind, d: Kind| (f as usize * KINDS + s as usize) * KINDS + d as usize;
    CELLS.get_or_init(|| {
        let mut t = vec![None; FAMILIES * KINDS * KINDS];
        for (f, s, d, v) in rows::CELLS {
            t[at(*f, *s, *d)] = Some(*v);
        }
        t
    })[at(family, src, dst)]
}

/// The values BC showed for single source values of a cell a code page maps (`rows::POINTS`): (the source as AL, the
/// value as BC showed it).
pub fn points(src: Kind, dst: Kind) -> impl Iterator<Item = (&'static str, &'static str)> {
    rows::POINTS
        .iter()
        .filter(move |(s, d, _, _)| *s == src && *d == dst)
        .map(|(_, _, a, b)| (*a, *b))
}
