use super::*;

/// `Enum.AsInteger()`: the ordinal (types/TestEnum.al:27-40).
pub(super) fn enum_as_integer(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Result<Value, Signal> {
    match r {
        Some(Value::Enum(_, o)) => Ok(Value::Integer(*o)),
        other => unsupported(&format!("AsInteger of {}", other.describe())),
    }
}

/// The enum a type-level call (`"Name".Names()`) or a call on a variable names: lowering passes
/// the enum object as the receiver value.
pub(super) fn enum_receiver(
    ctx: &dyn Ctx,
    r: Option<&mut Value>,
    what: &str,
) -> Result<(u32, Vec<(i32, String)>), Signal> {
    let Some(Value::Enum(id, _)) = r else {
        return unsupported(&format!("{what} without an enum receiver"));
    };
    let id = *id;
    ctx.enum_members(id).map(|m| (id, m)).map_or_else(
        || unsupported(&format!("{what}: this context has no enum metadata")),
        Ok,
    )
}

/// `Enum.Names()`: every declared name, the blank ordinal included
/// (types/TestEnumNamesContracts.al:19-60). The order is the declaration order, which matches
/// `Ordinals()` position by position; BC's order is not pinned (docs/probes-needed.md).
pub(super) fn enum_names(ctx: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Result<Value, Signal> {
    let (_, members) = enum_receiver(ctx, r, "Enum.Names")?;
    Ok(read_only_list(
        members.into_iter().map(|(_, n)| Value::Text(n.into())).collect(),
    ))
}

/// The lists of `Enum.Names()` and `Enum.Ordinals()` cannot be changed (collections/lt-cs-odd-leads/CsCollections.al:
/// EnumOrdinals_Mutations, EnumNames_Mutations).
pub(super) fn read_only_list(items: Vec<Value>) -> Value {
    let mut l: crate::value::ListVec = items.into();
    l.read_only = true;
    Value::List(Rc::new(std::cell::RefCell::new(l)))
}

/// `Enum.Ordinals()` (session/TestSystemExtended.al:381-390, types/TestLanguageEdgeCases.al:91-105).
pub(super) fn enum_ordinals(ctx: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Result<Value, Signal> {
    let (_, members) = enum_receiver(ctx, r, "Enum.Ordinals")?;
    Ok(read_only_list(
        members.into_iter().map(|(o, _)| Value::Integer(o)).collect(),
    ))
}

/// `Enum.FromInteger(n)`: any number is carried, declared or not (types/TestEnum.al:83-91,
/// types/TestEnumDefaultImplTests.Codeunit.al:70).
pub(super) fn enum_from_integer(ctx: &mut dyn Ctx, r: Option<&mut Value>, args: &mut [Value]) -> Result<Value, Signal> {
    let (id, _) = enum_receiver(ctx, r, "Enum.FromInteger")?;
    // An Option argument converts to its ordinal, as any Integer parameter takes it.
    let n = match args.first() {
        Some(Value::Option(o, _)) => Some(*o),
        // A Variant that holds an Enum, a whole Decimal or a Text of digits takes the ordinal or the number
        // (enum/lt-enum-fromint-variant/EnumFromIntegerVariant.al:FromInteger_VariantHolding asserts the holders Integer, Enum, Decimal and Text; no other holder is added).
        // A fraction or a text that is not a whole number is not pinned.
        Some(Value::Enum(_, o)) => Some(*o),
        Some(Value::Decimal(d)) if d.fract().is_zero() => i32::try_from(d.trunc()).ok(),
        Some(Value::Text(t)) if !t.is_empty() && t.trim_start_matches('-').chars().all(|c| c.is_ascii_digit()) => {
            t.parse::<i32>().ok()
        }
        _ => int_arg(args, 0).and_then(|n| i32::try_from(n).ok()),
    };
    match n {
        Some(n) => Ok(Value::Enum(id, n)),
        None => unsupported("Enum.FromInteger with a number outside the Integer range"),
    }
}
