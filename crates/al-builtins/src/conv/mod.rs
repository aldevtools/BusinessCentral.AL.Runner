//! Implicit conversions from the run matrix of corpus writer Y31 (`tests/al-language/types/lt-y31-conversion-matrix/`):
//! the table of its cells (`rows.rs`, generated from the corpus files by `tests/conv_matrix_rows.rs`) and the engine that
//! converts a value of a covered cell (`engine.rs`). A cell the matrix does not cover keeps the conversion of `ops::convert`.

mod arith;
pub mod byte_char;
mod compare;
mod engine;
mod rows;
mod variant;
mod variant_scalars;

pub use arith::arith_cell;
pub(crate) use arith::datetime_fraction_error;
pub use compare::compare_cell;
pub use engine::{convert_cell, variant_convert, variant_error};
pub use variant::{REGISTRY, kind_code, variant_into};

/// A type of the matrix. Text[N] and Text are one kind, as are the Code lengths and an Enum with or without an interface.
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

/// What a family of the matrix measures.
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

/// The verdict of a cell, `None` when the matrix does not cover it.
pub fn verdict(family: Family, src: Kind, dst: Kind) -> Option<Verdict> {
    type Cells = std::collections::HashMap<(Family, Kind, Kind), Verdict>;
    static CELLS: std::sync::OnceLock<Cells> = std::sync::OnceLock::new();
    CELLS
        .get_or_init(|| rows::CELLS.iter().map(|(f, s, d, v)| ((*f, *s, *d), *v)).collect())
        .get(&(family, src, dst))
        .copied()
}

/// The values BC showed for single source values of a cell a code page maps (`rows::POINTS`): (the source as AL, the
/// value as BC showed it).
pub fn points(src: Kind, dst: Kind) -> impl Iterator<Item = (&'static str, &'static str)> {
    rows::POINTS
        .iter()
        .filter(move |(s, d, _, _)| *s == src && *d == dst)
        .map(|(_, _, a, b)| (*a, *b))
}
