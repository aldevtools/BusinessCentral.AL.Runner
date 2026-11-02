use super::*;

pub(super) fn strlen(_: &mut dyn Ctx, _: Option<&mut Value>, args: &mut [Value]) -> Result<Value, Signal> {
    Ok(Value::Integer(text_arg(args, 0)?.utf16_len() as i32))
}

pub(super) fn strpos(_: &mut dyn Ctx, _: Option<&mut Value>, args: &mut [Value]) -> Result<Value, Signal> {
    let (s, sub) = (text_arg(args, 0)?, text_arg(args, 1)?);
    // text/TestTextProcessingContracts.al StrPos_EmptySubstring_ReturnsZero.
    if sub.is_empty() {
        return Ok(Value::Integer(0));
    }
    // A match at a char boundary is the same match unit by unit; the position counts UTF-16 units. No copy of the
    // text: a loop that calls StrPos on a long text (Pageworks' asset parsing) would copy it each time.
    // One character is a byte search (no searcher to build); longer needles are searched as a string.
    let mut cs = sub.chars();
    let found = match (cs.next(), cs.next()) {
        (Some(c), None) => s.find(c),
        _ => s.find(sub.as_str()),
    };
    let pos = found.map_or(0, |b| {
        if s.is_ascii() {
            b + 1
        } else {
            s[..b].encode_utf16().count() + 1
        }
    });
    Ok(Value::Integer(pos as i32))
}

/// `StrCheckSum(String [, WeightString] [, Modulus])`: `(M - (sum of digit * weight) mod M) mod M`, the
/// formula of the documentation (methods-auto/text/text-strchecksum-method.md, its example gives 0
/// for '4378' with weights '1234' and modulus 7). The corpus pins only that the result is not
/// negative, is below the modulus and is not 0 for '12345' (session/TestSystemExtended.al:218-256);
/// values and the four errors: types/TestProbeScalarContracts.al:25-123.
pub(super) fn strchecksum(_: &mut dyn Ctx, _: Option<&mut Value>, args: &mut [Value]) -> Result<Value, Signal> {
    let digits = |s: &str| -> Option<Vec<i64>> { s.chars().map(|c| c.to_digit(10).map(i64::from)).collect() };
    let source = text_arg(args, 0)?;
    let Some(source) = digits(&source) else {
        return raise("The value string must be numeric.", "ALSystemString");
    };
    let mut weights = match args.get(1) {
        Some(_) => {
            let Some(w) = digits(&text_arg(args, 1)?) else {
                return raise("The weight string must be numeric.", "ALSystemString");
            };
            w
        }
        None => Vec::new(),
    };
    // An empty source is 0 whatever the weights (types/TestProbeScalarContracts.al:StrCheckSum_Values, `empty_w1`).
    if source.is_empty() {
        return Ok(Value::Integer(0));
    }
    if weights.len() > source.len() {
        return raise(
            "The weight parameter cannot be longer than the value string.",
            "ALSystemString",
        );
    }
    // A short weight string is padded with 1 up to the length of the source.
    weights.resize(source.len(), 1);
    let modulus = int_arg(args, 2).unwrap_or(10);
    if modulus < 1 {
        return raise("The modulus parameter cannot be less than 1.", "ALSystemString");
    }
    if source.is_empty() {
        return Ok(Value::Integer(0));
    }
    let sum: i64 = source.iter().zip(&weights).map(|(d, w)| d * w).sum();
    Ok(Value::Integer(((modulus - sum % modulus) % modulus) as i32))
}

pub(super) fn copystr(_: &mut dyn Ctx, _: Option<&mut Value>, args: &mut [Value]) -> Result<Value, Signal> {
    let whole = text_arg(args, 0)?;
    // A Decimal position or length: `textfns::decimal_as_int`, the position first (CopyStr.Codeunit.al CopyStrDecimalPosLen1
    // `c2_2.5_3.4` names 2.5, on writer TG's branch).
    let num = |i: usize| -> Result<Option<i64>, Signal> {
        match args.get(i) {
            None => Ok(None),
            Some(Value::Decimal(d)) => crate::textfns::decimal_as_int(d).map(Some),
            Some(v) => match int_arg(args, i) {
                Some(n) => Ok(Some(n)),
                None => unsupported(&format!("CopyStr with an argument {i} of type {}", v.describe())),
            },
        }
    };
    let pos = num(1)?.unwrap_or(1);
    copystr_of(&whole, pos, num(2)?)
}

/// `CopyStr` of a text the caller holds by reference (the VM's fast path reads a variable without copying it).
pub fn copystr_of(whole: &crate::text::Txt, pos: i64, len: Option<i64>) -> Result<Value, Signal> {
    // A text of ASCII only is indexed by byte, with no copy: a loop of CopyStr over a long text stays linear in
    // the loop (Pageworks' asset parsing).
    let ascii = whole.is_ascii();
    let s: Vec<u16> = if ascii {
        Vec::new()
    } else {
        whole.encode_utf16().collect()
    };
    let total = if ascii { whole.len() } else { s.len() };
    // text/TestProbeTextDateFormatContracts.al:Text_SubstringBounds, `copystr_start0`, `copystr_neg_count`.
    // The two-argument form allows a start up to 2048, the three-argument form up to the largest Integer (text/lt-text-functions/
    // CopyStr.Codeunit.al:CopyStrErrors `pos0` against `pos0len`).
    if pos < 1 {
        let top = if len.is_some() { "2,147,483,647" } else { "2048" };
        return raise(
            format!(
                "The value of COPYSTR parameter 2 is outside of the permitted range. The current value is: {pos}. The permitted range is: from 1 to {top}."
            ),
            "OutsidePermittedRange",
        );
    }
    let start = (pos as usize - 1).min(total);
    let end = match len {
        Some(len) if len < 0 => {
            return raise(
                format!(
                    "The value of COPYSTR parameter 3 is outside of the permitted range. The current value is: {len}. The permitted range is: from 0 to 2,147,483,647."
                ),
                "OutsidePermittedRange",
            );
        }
        // The count may reach to the largest Integer counted from the start (CopyStrErrors `af81`).
        Some(len) if len > 2_147_483_648 - pos => {
            return raise(
                format!(
                    "The value of COPYSTR parameter 3 is outside of the permitted range. The current value is: {}. The permitted range is: from 0 to {}.",
                    group_thousands(len),
                    group_thousands(2_147_483_648 - pos)
                ),
                "OutsidePermittedRange",
            );
        }
        Some(len) => (start + len as usize).min(total),
        None => total,
    };
    if ascii {
        return Ok(Value::Text(whole[start..end].to_string().into()));
    }
    Ok(Value::Text(String::from_utf16_lossy(&s[start..end]).into()))
}

pub(super) fn group_thousands(n: i64) -> String {
    let digits = n.abs().to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if n < 0 { format!("-{out}") } else { out }
}

pub(super) fn strsubstno(ctx: &mut dyn Ctx, _: Option<&mut Value>, args: &mut [Value]) -> Result<Value, Signal> {
    let template = text_arg(args, 0)?;
    Ok(Value::Text(substitute_fields(&template, &args[1..], ctx)?.into()))
}

/// The text a TextConst reads in language `language`, from its language codes and texts `args` in declaration order:
/// - the text of the language in force (misc/lt-probe-requests-140/P140TextConstLang.al:20-56, P140_ReadAsText_Deu: `DeuFra`
///   reads 'Hallo' in 1031; :58-91, P140_ReadAsText_Fra: 'Bonjour' in 1036; :163-191, BM_TextConst_Methods: an ENU and DEU
///   TextConst reads its DEU text in 1031);
/// - else the ENU text (P140TextConstLang.al:58-91: `EnuEmptyDeu` reads '' in 1036; types/lt-variant/VaTTextConst.al:
///   TextConst_To_Text_Basics);
/// - else the text of its only language (P140TextConstNoEnu.al:23-71, P140_ReadAsText_DefaultLanguage: `OnlyFra` reads
///   'Seulement'; P140TextConstLang.al:20-56: `OnlyFra` in 1031);
/// - else, measured only for `DEU = .., FRA = ..` read in 1033, the DEU text (P140TextConstNoEnu.al:23-71: `DeuFra` reads
///   'Hallo'). Whether BC takes the first, the alphabetically first or the DEU text is not pinned, so other code lists are
///   refused (docs/probes-needed.md "TextConst").
///
/// Every pinned read runs in 1033, 1031 or 1036, the languages of the codes ENU, DEU and FRA. A read in any other language is
/// refused: it may fall back through a parent culture, so not even its ENU text or a single text is pinned there.
pub(super) fn textconst_text(language: i32, args: &[Value]) -> Result<Value, Signal> {
    const KNOWN: [(&str, i32); 3] = [("ENU", 1033), ("DEU", 1031), ("FRA", 1036)];
    if !KNOWN.iter().any(|(_, id)| *id == language) {
        return unsupported(&format!(
            "a TextConst read in language {language}: only 1033, 1031 and 1036 are pinned (docs/probes-needed.md \"TextConst\")"
        ));
    }
    let mut texts = Vec::new();
    for pair in args.chunks(2) {
        let [Value::Text(code), Value::Text(text)] = pair else {
            return unsupported("System.TextConstText with arguments other than pairs of a language code and a text");
        };
        texts.push((code.to_string(), text.clone()));
    }
    let id = |code: &str| KNOWN.iter().find(|(c, _)| *c == code).map(|(_, id)| *id);
    if let Some((_, t)) = texts.iter().find(|(c, _)| id(c) == Some(language)) {
        return Ok(Value::Text(t.clone()));
    }
    if let Some((_, t)) = texts.iter().find(|(c, _)| c == "ENU") {
        return Ok(Value::Text(t.clone()));
    }
    match texts.as_slice() {
        [(_, t)] => Ok(Value::Text(t.clone())),
        [(deu, t), (fra, _)] if language == 1033 && deu == "DEU" && fra == "FRA" => Ok(Value::Text(t.clone())),
        _ => unsupported(
            "a TextConst with no text in the language in force and no ENU text, other than one text or DEU then FRA: which of its texts BC reads is not pinned (docs/probes-needed.md)",
        ),
    }
}

/// `UpperCase` and `LowerCase` follow `GlobalLanguage` for Turkish (1055) and Azerbaijani (1068): `i` becomes the dotted `İ` and `I`
/// the dotless `ı`; every other letter maps as in the default language, and Greek (1032), German (1031) and Lithuanian (1063) map as
/// the default too (text/lt-text-functions/CaseConv.Codeunit.al:145-180, LT_BU_CaseConv_TurkishGreekLanguage; :218-276,
/// PYC_CaseConv_OtherLanguages). `ToUpper`/`ToLower` do not follow it.
pub fn is_turkic_language(language: i32) -> bool {
    matches!(language, 1055 | 1068)
}

pub(super) fn uppercase(ctx: &mut dyn Ctx, _: Option<&mut Value>, args: &mut [Value]) -> Result<Value, Signal> {
    let t = text_arg(args, 0)?;
    if is_turkic_language(ctx.language()) {
        return Ok(Value::Text(simple_upper(&t.replace('i', "İ")).into()));
    }
    Ok(Value::Text(simple_upper(&t).into()))
}

pub(super) fn lowercase(ctx: &mut dyn Ctx, _: Option<&mut Value>, args: &mut [Value]) -> Result<Value, Signal> {
    let t = text_arg(args, 0)?;
    if is_turkic_language(ctx.language()) {
        return Ok(Value::Text(simple_lower(&t.replace('I', "ı")).into()));
    }
    Ok(Value::Text(simple_lower(&t).into()))
}

/// `ArrayLen(Array)` of a one-dimensional array: the length of the array the variable holds. A `var` or value
/// parameter takes the caller's array whatever size it was declared with (types/W7VarArray.al: a parameter declared
/// `array[3]` called with `array[2]` and `array[5]` sees 2 and 5, BC 27.5 and 29.0).
pub(super) fn array_len(_: &mut dyn Ctx, _: Option<&mut Value>, args: &mut [Value]) -> Result<Value, Signal> {
    match args {
        // The lowering passes the dimension asked for and the number of dimensions when the dimension is outside the array.
        [Value::Integer(dim), Value::Integer(n)] => raise(
            format!(
                "The value of ARRAYLEN parameter 2 is outside of the permitted range. The current value is: {dim}. The permitted range is: from 1 to {n}."
            ),
            "OutsidePermittedRange",
        ),
        _ => array_len_of(args),
    }
}

pub(super) fn array_len_of(args: &[Value]) -> Result<Value, Signal> {
    match args.first() {
        Some(Value::Array(a)) => Ok(Value::Integer(a.borrow().len() as i32)),
        other => Err(Signal::Unsupported(format!("ArrayLen of {}", other.describe()))),
    }
}
