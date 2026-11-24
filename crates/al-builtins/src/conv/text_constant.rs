//! A Label or TextConst that a Variant holds as a TextConstant (`Value::TextConst`, made by `Variant.FromTextConstant#1`):
//! what a reader of the Variant gets. The one owner of that rule; its callers are the four places a Variant's value is
//! read for something other than a Variant: `variant_into` (a typed variable, parameter or exit value), `ops::convert`
//! (any other typed slot, a field among them), the arguments of a builtin (al-vm `exec/run.rs`) and the operands of an
//! operator (al-vm `interp.rs` `binary_op`).
//!
//! - The kind tests read the receiver itself (`Value::is_kind`), and `Format(V)` reads the TextConstant
//!   (types/variant/VaTTextConst.al:144-151 `format`; :274-289): this module leaves those alone.
//! - Into a Text or Code variable or parameter the text is read, for both declarations (VaTTextConst.al:144-151
//!   `to_text`, :274-289 `textParam`, `code`; types/Y81LabelVariant.al, codeunits 94256 to 94258, for a Label). What the
//!   slot then does with the text (its length) is the rule for any text.
//! - Any other reader of a Label gets a Text: the value a Label in a Variant was before the TextConstant existed, kept.
//!   That includes a comparison or any other operator, and a builtin that takes the Variant by reference (`List.Get`,
//!   `Dictionary.Get`), which then writes the Variant back as before.
//! - Any other reader of a TextConst is refused: before the TextConstant existed a TextConst never went into a Variant,
//!   and no case pins these readers (docs/probes-needed.md "Variant").
//!
//! What a reader gets from a Label or TextConst a Variant holds as a TextConstant (`Value::TextConst`
//! with `TextConstFrom`): the one rule, called by `variant_into`, `ops::convert` and al-vm
//! `exec/run.rs` (`text_constant_arguments`, the by-value arguments of every builtin). Entry points:
//! `read`, `read_arguments`.

use crate::describe::Describe;
use crate::native::{Signal, unsupported};
use crate::value::{RtType, TextConstFrom, Value};

/// Who reads the Variant's value.
pub enum Reader<'a> {
    /// A typed variable, parameter or exit value, by the kind of its type (`variant_into`).
    VariantInto(&'a RtType),
    /// Any other typed slot (`ops::convert`).
    Slot(&'a RtType),
    /// The builtin of this name, the value one of its by-value arguments.
    Builtin(&'a str),
    /// The builtin of this name, the value one of its `var` arguments: the builtin may write the argument back.
    BuiltinByRef(&'a str),
    /// An operand of an operator (`=`, `<>`, `<`, `+`, ...).
    Operator,
}

/// `v` as `reader` gets it: the value itself unless it is a TextConstant (see the module).
pub fn read(v: Value, reader: Reader<'_>) -> Result<Value, Signal> {
    // Into another Variant it stays a TextConstant: the copy keeps the kind (`AllFlags(V)`, VariantIsMatrix.al:224-243).
    if matches!(reader, Reader::Slot(RtType::Variant)) {
        return Ok(v);
    }
    let Value::TextConst(text, from) = v else {
        return Ok(v);
    };
    let pinned = matches!(reader, Reader::VariantInto(RtType::Text(_) | RtType::Code(_)));
    match (from, pinned) {
        (TextConstFrom::Label, _) | (TextConstFrom::TextConst, true) => Ok(Value::Text(text)),
        (TextConstFrom::TextConst, false) => {
            let by = match reader {
                Reader::VariantInto(t) | Reader::Slot(t) => format!("converted to {}", t.describe()),
                Reader::Builtin(name) => format!("handed to {name}"),
                Reader::BuiltinByRef(name) => format!("handed by reference to {name}"),
                Reader::Operator => "used as an operand of an operator".to_string(),
            };
            unsupported(format!(
                "a TextConst in a Variant {by} (not pinned: docs/probes-needed.md \"Variant\")"
            ))
        }
    }
}

/// Whether builtin `name` given `count` arguments reads a TextConstant argument itself: `Format(V)` with no length or
/// format (VaTTextConst.al:144-151 `format`), `Clear(V)`, which empties the Variant whatever it holds (al-vm
/// `exec/run.rs`, `SYSTEM_CLEAR_1`), and `Variant.Into#1`, the lowering's read into a typed slot (`variant_into`).
fn reads_itself(name: &str, count: usize) -> bool {
    matches!(
        (name, count),
        ("System.Format#1", 1) | ("System.Clear#1", 1) | ("Variant.Into#1", _)
    )
}

/// The arguments `args` of builtin `name` as it gets them (`read` for each one); `by_ref(i)` says whether argument `i` is
/// a `var` argument. A Label goes in as a Text either way; a TextConst by reference is refused like any other unpinned
/// reader of a TextConst (what the Variant holds after the builtin writes it back is not pinned).
pub fn read_arguments(name: &str, args: &mut [Value], by_ref: impl Fn(usize) -> bool) -> Result<(), Signal> {
    if reads_itself(name, args.len()) {
        return Ok(());
    }
    for (i, a) in args.iter_mut().enumerate() {
        if !matches!(a, Value::TextConst(..)) {
            continue;
        }
        let reader = if by_ref(i) {
            Reader::BuiltinByRef(name)
        } else {
            Reader::Builtin(name)
        };
        let v = std::mem::replace(a, Value::Null);
        *a = read(v, reader)?;
    }
    Ok(())
}
