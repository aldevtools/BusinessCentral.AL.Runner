use super::*;

/// `Format(V, Length, Number)` with `V` declared as a Variant: the standard format numbers 3 to 8 do not exist for a Variant
/// (types/lt-scalars/ScT70050.al:FormatNumbers_*), though a Decimal variable has 3 and 4.
pub(super) fn format_variant(ctx: &mut dyn Ctx, r: Option<&mut Value>, args: &mut [Value]) -> Result<Value, Signal> {
    let std = int_arg(args, 2).unwrap_or(0) as i32;
    if !matches!(std, 0..=2 | 9) && !matches!(&args[0], Value::Null) {
        return Err(Signal::Error(Box::new(AlError {
            message: format!("Standard format number {std} does not exist for the type 'Variant'."),
            code: "InvalidStandardFormat".into(),
            call_stack: String::new(),
            ..Default::default()
        })));
    }
    format_1(ctx, r, args)
}

/// An Option value with the OptionCaptionML captions of the session language (the lowering passes the property text):
/// `Format` then shows the caption (record/lt-x-table-props/XTablePropsTests.al:219-228). See
/// [`crate::format::ml_text_in_language`] for what is pinned.
pub(super) fn option_caption_ml(ctx: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Result<Value, Signal> {
    let (Some(Value::Option(o, names)), Some(ml)) = (a.first(), a.get(1).and_then(text_of)) else {
        return unsupported("OptionCaptionML without an Option value and its property text");
    };
    let list = crate::format::ml_text_in_language(ml.as_str(), ctx.language())
        .map_err(|e| Signal::Unsupported(format!("OptionCaptionML {e}")))?;
    let captions: Vec<String> = list.split(',').map(str::to_string).collect();
    if captions.len() != names.len() {
        return unsupported("OptionCaptionML with another number of entries than the members (not pinned)");
    }
    Ok(Value::Option(*o, captions.into()))
}

/// `Format(Rec.Field)` of a Decimal field that declares its places (`DecimalPlaces` or an `AutoFormatExpression` precision;
/// lowering appends them as a fourth argument `min:max`). The standard formats 0 and 1 and the XML format 9 show those places; 0 groups
/// thousands (pages/lt-page-autoformat-lookups/P113AutoFormat.al:P113_AF_TableFieldFormatIsPlain, `1,234.500` and, for 1 and 9,
/// `1234.500`; pages/lt-page-presentation-properties/P119Table.al:P119_DecimalPlaces `1.500`).
pub(super) fn format_decimal_field(
    ctx: &mut dyn Ctx,
    r: Option<&mut Value>,
    args: &mut [Value],
) -> Result<Value, Signal> {
    let len = int_arg(args, 1).unwrap_or(0) as i32;
    let places = match args.get(3) {
        Some(Value::Text(t)) => crate::pagefmt::parse_decimal_places(t),
        _ => None,
    };
    // When the places hold (`al-sema` `decimal_field_places`): mode 1, an amount field without AutoFormatExpression, rounds from
    // BC 28 only; BC 27 writes every digit (types/lt-at-partner-gaps/AtPrecisionStandardFormat2.al:684-723, `#else` side, and
    // :883-967). Mode 2, a unit amount without one, has its places under `FIELD_FORMAT_STRINGS` only (:883-967); elsewhere it
    // keeps every digit (:684).
    let mode = int_arg(args, 4).unwrap_or(0);
    let gated = mode == 1;
    let plain = |ctx: &mut dyn Ctx, r: Option<&mut Value>, args: &mut [Value]| {
        if matches!(args.get(2), Some(Value::Text(_) | Value::Code(_))) {
            format_2(ctx, r, &mut args[..3])
        } else {
            let n = args.len().min(3);
            format_1(ctx, r, &mut args[..n])
        }
    };
    if gated && !ctx.format_gate(al_version::Gate::AutoFormatAmountFieldRoundsFrom28) {
        return plain(ctx, r, args);
    }
    let field_string =
        matches!(args.get(2), Some(Value::Text(t) | Value::Code(t)) if FIELD_FORMAT_STRINGS.contains(&t.as_str()));
    if mode == 2 && !field_string {
        return plain(ctx, r, args);
    }
    // A format string: one with a `<Precision,..>` code formats the value by itself, the field's places do not matter
    // (AtPrecisionStandardFormat2.al:684-723, `P22S2` and `P2S2` of every field); `<Standard Format,2>` alone is format number 2
    // with the field's places (`S2`: `1234.6` for the `<Precision,1:1>` field, `1234.57` for the amount from BC 28). The strings
    // of `FIELD_FORMAT_STRINGS` format the value rounded to the field's places (`format_field_string`). Any other string is not
    // pinned.
    let std = match args.get(2) {
        Some(Value::Text(t) | Value::Code(t)) => {
            let t = t.as_str();
            if t.contains("<Precision,") {
                return plain(ctx, r, args);
            }
            if t != "<Standard Format,2>" {
                return match (FIELD_FORMAT_STRINGS.contains(&t), &args[0], places) {
                    (true, Value::Decimal(d), Some(places)) => format_field_string(ctx, *d, len, t, places),
                    _ => Err(Signal::Unsupported(format!(
                        "Format of a Decimal field with DecimalPlaces or an AutoFormat with the format string `{t}` (not pinned)"
                    ))),
                };
            }
            2
        }
        _ => int_arg(args, 2).unwrap_or(0) as i32,
    };
    if let (Value::Decimal(d), Some(places), 0 | 1 | 2 | 9) = (&args[0], places, std) {
        // The amount field is pinned for formats 0, 2 and 9 on values of two decimals or more (AtPrecisionStandardFormat2.al:
        // 663-723: 1234.5678, -0.005), and for format 0 on any value (misc/lt-outside-format-json-list/RwDecimalFieldPlaces.al:83
        // `Amt1NoExpr`: 70, 0, 1234.5 give `70.00`, `0.00`, `1,234.50` from BC 28).
        if gated && (std == 1 || (std != 0 && d.normalize().scale() < 2)) {
            return Err(Signal::Unsupported(format!(
                "Format of an amount field (AutoFormatType 1 without AutoFormatExpression) of {d} in format {std} (not pinned)"
            )));
        }
        let mut text = crate::pagefmt::decimal_text(d, places);
        // A negative value that rounds to zero keeps its minus: -0.005 with `<Precision,1:1>` is `-0.0` in formats 0, 2 and 9
        // (AtPrecisionStandardFormat2.al:684-723, `b default`, `b S2`, `b number 2`, `b number 9`); format 1 is not pinned.
        if d.is_sign_negative() && !d.is_zero() && !text.starts_with('-') {
            if std == 1 {
                return Err(Signal::Unsupported(format!(
                    "Format 1 of a Decimal field with places of {d}, a negative value that rounds to zero (not pinned)"
                )));
            }
            text.insert(0, '-');
        }
        if std != 0 {
            text.retain(|c| c != ',');
        }
        return Ok(Value::Text(crate::format::pad(text, len).into()));
    }
    plain(ctx, r, args)
}

/// The format strings measured on Decimal fields with places: DecimalPlaces 2:2, 0:0, 3:5, 0:5, and AutoFormatType 1 (2:2 from
/// BC 28), 2 and 10 with `<Precision,1:1>` (types/lt-at-partner-gaps/AtPrecisionStandardFormat2.al:805-1047, RO_* tests: the
/// values 5, 1234, 1234.5678, -0.5, 0 and -1234567.25; the same under GlobalLanguage 1031).
pub(super) const FIELD_FORMAT_STRINGS: &[&str] = &[
    "<Integer>",
    "<Integer Thousand>",
    "<Decimals>",
    "<Integer><Decimals>",
    "<Sign><Integer Thousand><Decimals>",
    "<Standard Format,0>",
    "<Standard Format,1>",
    "<Standard Format,9>",
    "<Integer Thousand><Decimals,3>",
    "<Integer>.<Decimals>",
];

/// The places measured with `FIELD_FORMAT_STRINGS`: DecimalPlaces 2:2, 0:0, 3:5 and 0:5, AutoFormatType 1 (2:2), 2 (2:5) and
/// 10 with `<Precision,1:1>` (AtPrecisionStandardFormat2.al:883-1047). Other places are not pinned and are refused.
pub(super) const FIELD_FORMAT_PLACES: &[(u32, u32)] = &[(2, 2), (0, 0), (3, 5), (0, 5), (2, 5), (1, 1)];

/// A string of `FIELD_FORMAT_STRINGS` on a field of `(min, max)` places: the value is first rounded half away from zero to
/// `max` places (0:0 writes 1234.5678 as `1235` and -0.5 as `-1`, `<Integer>` of it `1`: the integer code drops the sign), then
/// `<Decimals>` shows at least `min` places (2:2 writes 5 as `.00`, 0:5 as nothing, 3:5 -0.5 as `.500`), and `<Decimals,3>`
/// keeps its width of two (3:5 1234.5678 is `***`) (AtPrecisionStandardFormat2.al:969-1047). A negative value that rounds to
/// zero, and places outside `FIELD_FORMAT_PLACES`, are not measured and are refused.
pub(super) fn format_field_string(
    ctx: &mut dyn Ctx,
    d: rust_decimal::Decimal,
    len: i32,
    template: &str,
    (min, max): (u32, u32),
) -> Result<Value, Signal> {
    if !FIELD_FORMAT_PLACES.contains(&(min, max)) {
        return Err(Signal::Unsupported(format!(
            "Format of a Decimal field with places {min}:{max} with `{template}` (only the measured places are pinned)"
        )));
    }
    let rounded = d.round_dp_with_strategy(max, rust_decimal::RoundingStrategy::MidpointAwayFromZero);
    if rounded.is_zero() && d.is_sign_negative() && !d.is_zero() {
        return Err(Signal::Unsupported(format!(
            "Format of a Decimal field with places {min}:{max} of {d}, a negative value that rounds to zero, with `{template}` (not pinned)"
        )));
    }
    if let Some(std) = template
        .strip_prefix("<Standard Format,")
        .and_then(|t| t.strip_suffix('>'))
    {
        let mut text = crate::pagefmt::decimal_text(&rounded, (min, max));
        if std != "0" {
            text.retain(|c| c != ',');
        }
        return Ok(Value::Text(crate::format::pad(text, len).into()));
    }
    format_string_out(crate::formatstr::format_with_field_places(
        &Value::Decimal(rounded),
        len,
        template,
        min,
        ctx,
    ))
}

/// `Format(Rec.Field[, Length, FormatNumber])` of a Decimal table field whose AutoFormatExpression is an expression: the
/// places come from the field's AutoFormat evaluated on the record (`Ctx::auto_format_field_places`), then as
/// `format_decimal_field`. Arguments: the value, length, format number, the record, the field's number.
pub(super) fn format_auto_format_field(
    ctx: &mut dyn Ctx,
    r: Option<&mut Value>,
    args: &mut [Value],
) -> Result<Value, Signal> {
    let Some(&Value::Integer(pos)) = args.get(4) else {
        return Err(Signal::Unsupported(
            "Format of an AutoFormat field without its number".into(),
        ));
    };
    let record = args.get(3).cloned().unwrap_or(Value::Null);
    let places = ctx.auto_format_field_places(&record, pos as usize)?;
    if matches!(args.get(5), Some(Value::Integer(11))) {
        return crate::autoformat11::format(ctx, args, &places);
    }
    // A sixth argument 4 or 5: an amount or unit amount without an expression, which BC 27 formats as a plain Decimal
    // (misc/lt-sa-unit-amount-subst/SaUnitAmountSubst.al `A4No`, `A5No`, the `#else` side of BC28PLUS): mode 1 of
    // `format_decimal_field`.
    let mode = i32::from(matches!(args.get(5), Some(Value::Integer(4 | 5))));
    let mut rest = vec![
        args[0].clone(),
        args[1].clone(),
        args[2].clone(),
        Value::Text(places.into()),
        Value::Integer(mode),
    ];
    format_decimal_field(ctx, r, &mut rest)
}

pub(super) fn format_1(ctx: &mut dyn Ctx, _: Option<&mut Value>, args: &mut [Value]) -> Result<Value, Signal> {
    let len = int_arg(args, 1).unwrap_or(0) as i32;
    let std = int_arg(args, 2).unwrap_or(0) as i32;
    // The text of a DotNet object is its ToString(); the length and the format number change
    // nothing for the numbers 0 and 2 (tests/al-language-onprem-dotnet/TestDotNetValueKindsContracts.al,
    // codeunit 61400, FormatOfDotNetObjects_IsToString). Other format numbers are not pinned.
    // The number 9 is pinned for an enum member only (XmlNodeTypeFormat.al: `Format(NodeType, 0, 9)` is `Element`).
    let enum_member = matches!(&args[0], Value::DotNet(o) if matches!(o.kind, crate::value::DotNetKind::Enum { .. }));
    if matches!(&args[0], Value::DotNet(_)) && (matches!(std, 0 | 2) || (std == 9 && enum_member)) {
        if let Some(r) = ctx.format_dotnet(&args[0]) {
            return r.map(|s| Value::Text(s.into()));
        }
    }
    // The first day of the calendar (00010101D) is not a valid date to format with the numbers 0, 1, 4 and 9 (types/lt-datetime/
    // Format.al:Date_Format_0D_And_Bounds); the keys and messages the runtime builds itself format it all the same.
    if matches!(&args[0], Value::Date(1, _)) && matches!(std, 0 | 1 | 4 | 9) {
        return Err(Signal::Error(Box::new(AlError {
            message: "The date is not valid.".into(),
            code: "DateInvalid".into(),
            call_stack: String::new(),
            ..Default::default()
        })));
    }
    // A TestPage control formats as the text it shows, cut or padded after it as a Text is; the format number changes nothing
    // (errors/lt-ms-refusals-132/P132TestPageFormatRisky.al:64-89 P132_FormatOfControl_Standard0 and :92-125 _StandardNumbers:
    // Integer, Decimal, Text, Boolean and DateTime controls with 0, 1, 2 and 9 and a length of 8; errors/lt-ms-refusals-129/
    // P129FormatTestPageControl.al:94-138 P129_FormatTestPageControl_BooleanOptionCodeTextEnum: Boolean, Option, Code, Text and Enum controls with 0, 2 and 9 and a
    // length of 5). Other format numbers are not pinned.
    if matches!(&args[0], Value::TestControl(_)) {
        if !matches!(std, 0 | 1 | 2 | 9) {
            return Err(Signal::Unsupported(format!(
                "Format of a TestPage control with standard format {std} (only 0, 1, 2 and 9 are pinned)"
            )));
        }
        let mut control = args[0].clone();
        let shown = ctx.page(crate::page_op::PageOp::TestFieldValue, Some(&mut control), &mut [])?;
        if !matches!(shown, Value::Text(_)) {
            return Err(Signal::Unsupported(format!(
                "Format of a TestPage control whose value is {} (not pinned)",
                shown.describe()
            )));
        }
        let mut inner = [shown, Value::Integer(len), Value::Integer(0)];
        return format_1(ctx, None, &mut inner);
    }
    // A record as a whole, a RecordRef and a FieldRef need the table's schema.
    let composite = matches!(&args[0], Value::Record(_))
        || matches!(
            &args[0],
            Value::Library(
                crate::library_kind::LibraryKind::RecordRef | crate::library_kind::LibraryKind::FieldRef,
                _
            )
        );
    // Standard format 9 of a Record or a RecordRef gives the text of standard format 0, and an unopened RecordRef formats as
    // the empty text with 0 and 9 (errors/lt-ms-refusals-133/P133DateEvaluateFormat.al:441-566
    // NP3_FormatRecord_StandardFormats_0And9, _Temporary_AndFlowField_0And9, NP3_FormatRecordRef_StandardFormats_0And9,
    // NP3_FormatRecordRef_NotOpen). A FieldRef with a number other than 0 is handled below.
    let record_or_ref = match &args[0] {
        Value::Record(_) => Some(true),
        Value::Library(crate::library_kind::LibraryKind::RecordRef, l) => {
            Some(!matches!(&*l.borrow(), crate::value::LibraryValue::Empty))
        }
        _ => None,
    };
    if record_or_ref == Some(false) && matches!(std, 0 | 9) {
        return Ok(Value::Text(crate::format::pad(String::new(), len).into()));
    }
    // A FieldRef with a format number other than 0 formats its value as `Format(FieldRef.Value, Length, Number)` does, which
    // differs from the field's own Format: a Decimal ignores the field's DecimalPlaces, an Option still shows its caption with 1
    // (errors/lt-ms-refusals-129/P129FormatFieldRef.al:114-149 P129_FormatFieldRef_Numbers 1, 2 and 9 of Integer, BigInteger,
    // Decimal and Duration; :176-208 _BooleanOptionCodeTextEnum 2 and 9; :321-358 BT_FormatFieldRef_AgainstFormatValue and
    // :360-380 _BlobOptionEnum, `FR 0,1` = `FR.Value 0,1` and `FR 0,9` = `FR.Value 0,9` for Decimals with DecimalPlaces, a Date,
    // an Option with captions and an Enum). Other format numbers are not pinned.
    if std != 0 {
        if let Value::Library(_, l) = &args[0] {
            if matches!(
                &*l.borrow(),
                crate::value::LibraryValue::FieldRef(..) | crate::value::LibraryValue::KeyFieldRef(..)
            ) {
                if !matches!(std, 1 | 2 | 9) {
                    return Err(Signal::Unsupported(format!(
                        "Format of a FieldRef with standard format {std} (only 1, 2 and 9 are pinned)"
                    )));
                }
                let mut fr = args[0].clone();
                let value = ctx.ref_op("FieldRef.Value#1", Some(&mut fr), &mut [])?;
                let mut inner = [value, Value::Integer(len), Value::Integer(std)];
                return format_1(ctx, None, &mut inner);
            }
        }
    }
    // A FieldRef with a length and format number 0 cuts the field's text to the length; a shorter text is padded with blanks,
    // after a Text or a Code and before an Integer (P129FormatFieldRef.al:214-237 P129_FormatFieldRef_WithLength: Text 5 `Hello`
    // and 20, Code 5 `ABC  `, Integer 2 `12` and 8 `    1234`, Decimal 4 `1,23`, Option 2 `Do`, Date 5 `03/05`). Padding of
    // other types, the cut of other types and a negative length are not pinned.
    if std == 0 && len != 0 {
        if let Value::Library(_, l) = &args[0] {
            if matches!(
                &*l.borrow(),
                crate::value::LibraryValue::FieldRef(..) | crate::value::LibraryValue::KeyFieldRef(..)
            ) {
                let mut fr = args[0].clone();
                let value = ctx.ref_op("FieldRef.Value#1", Some(&mut fr), &mut [])?;
                let Some(Ok(t)) = ctx.format_composite(&args[0]) else {
                    return Err(Signal::Unsupported(
                        "Format of a FieldRef with a length (not pinned)".into(),
                    ));
                };
                let n = t.chars().count();
                let want = usize::try_from(len).unwrap_or(0);
                let text_like = matches!(value, Value::Text(_) | Value::Code(_));
                let cut_pinned = text_like
                    || matches!(
                        value,
                        Value::Integer(_) | Value::Decimal(_) | Value::Option(..) | Value::Date(..)
                    );
                let out = if len < 0 {
                    None
                } else if n > want && cut_pinned {
                    Some(t.chars().take(want).collect::<String>())
                } else if n <= want && text_like {
                    Some(format!("{t:<want$}"))
                } else if n <= want && matches!(value, Value::Integer(_)) {
                    Some(format!("{t:>want$}"))
                } else {
                    None
                };
                return match out {
                    Some(o) => Ok(Value::Text(o.into())),
                    None => Err(Signal::Unsupported(format!(
                        "Format of a FieldRef of {} with length {len} (only the cut and the padding of P129FormatFieldRef.al are pinned)",
                        value.describe()
                    ))),
                };
            }
        }
    }
    let std9 = std == 9 && record_or_ref == Some(true);
    if composite && (std == 0 || std9) {
        if let Some(r) = ctx.format_composite(&args[0]) {
            return r.map(|t| Value::Text(crate::format::pad(t, len).into()));
        }
    }
    format_value(&args[0], len, std, ctx)
        .map(|s| Value::Text(s.into()))
        .map_err(|f| match f {
            FormatFail::Unsupported(s) => Signal::Unsupported(s),
            FormatFail::Error { message, code } => Signal::Error(Box::new(AlError {
                message,
                code,
                call_stack: String::new(),
                ..Default::default()
            })),
        })
}

/// `Format(Value, Length, FormatString)`: see [`crate::formatstr`].
pub(super) fn format_2(ctx: &mut dyn Ctx, _: Option<&mut Value>, args: &mut [Value]) -> Result<Value, Signal> {
    let len = int_arg(args, 1).unwrap_or(0) as i32;
    let template = text_arg(args, 2)?;
    format_string_out(crate::formatstr::format_with_string(&args[0], len, &template, ctx))
}

/// The result of a format string: the text, or the error or refusal `formatstr` marks in its `Err`.
pub(super) fn format_string_out(r: Result<String, String>) -> Result<Value, Signal> {
    r.map(|s| Value::Text(s.into()))
        .map_err(|e| match e.strip_prefix(crate::formatstr::INVALID_FORMAT_MARK) {
            Some(template) => Signal::Error(Box::new(AlError {
                message: format!(
                    "An invalid field or attribute has been specified for the 'Format' property.\\\\{template}"
                ),
                code: "InvalidFormatString".into(),
                call_stack: String::new(),
                ..Default::default()
            })),
            None => match e
                .strip_prefix(crate::formatstr::RAISE_MARK)
                .and_then(|r| r.split_once(crate::formatstr::RAISE_MARK))
            {
                Some((code, message)) => Signal::Error(Box::new(AlError {
                    message: message.to_string(),
                    code: code.to_string(),
                    call_stack: String::new(),
                    ..Default::default()
                })),
                None => Signal::Unsupported(e),
            },
        })
}
