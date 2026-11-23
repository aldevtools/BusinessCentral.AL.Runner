//! The tags of `ReadFromYaml`: percent escapes, which kind a resolved tag names, and what a tagged scalar or key reads
//! as. Every rule is a cell of corpus PR 1549 (`tests/al-language/json/JsonYamlFinalTags.al`, codeunit 79896), read
//! through the assumption `read-from-yaml-final-shapes` until it is pinned; cell names below are its observation names
//! (`Int1_2` is the second cell of `ReadFromYaml_TagScalar_Int_1`). A text or kind no cell measures refuses.

use super::scan::Style;
use super::{Fail, Res};

const CORE: &str = "tag:yaml.org,2002:";

/// `%XX` escapes decoded as UTF-8 bytes, before the tag is resolved (`Escapes1_4`-`Escapes1_9`, `Escapes2_3`-`Escapes2_8`).
/// A `%` without two hex digits after it, or bytes that are no UTF-8, is the YAML error (`Escapes1_10`-`Escapes1_12`).
pub(super) fn unescape(uri: &str) -> Res<String> {
    if !uri.contains('%') {
        return Ok(uri.to_string());
    }
    let b = uri.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            let hex = uri
                .get(i + 1..i + 3)
                .filter(|h| h.bytes().all(|c| c.is_ascii_hexdigit()));
            let byte = hex.and_then(|h| u8::from_str_radix(h, 16).ok()).ok_or(Fail::Bad)?;
            out.push(byte);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(out).map_err(|_| Fail::Bad)
}

/// What a resolved tag names.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum Kind {
    Str,
    Int,
    Float,
    Bool,
    Null,
    Timestamp,
    Map,
    Seq,
    Set,
    Binary,
    /// `!!omap`, `!!pairs`, `!!merge`, `!!value`, `!!yaml`: the YAML error on every node a cell puts them on (`Other1_7`-`Other1_13`,
    /// `WrongNode1_7`, `WrongNode1_8`, `WrongNode2_13`, `Top1_15`).
    Unsupported,
    /// A core tag of another name (`!!foo`, `!!2:int`, `!!int ` after an escape): the YAML error on a scalar (`Other1_10`,
    /// `Escapes1_1`, `Escapes2_5`); on a collection no cell measures it.
    UnknownCore,
    /// A local tag (`!`, `!x`, `!local`, `!<!local>`): the YAML error on every node (`Other1_16`, `Other1_19`, `Other1_20`,
    /// `Other2_1`, `WrongNode1_9`, `WrongNode2_11`, `WrongNode2_12`, `Top1_14`, `Top2_1`, `Keys1_18`, `Escapes2_1`).
    Local,
    /// Any other global tag (`tag:x,2000:foo`, corpus PR 1530 `tg17`): the YAML error on a scalar.
    Other,
}

pub(super) fn kind(tag: &str) -> Kind {
    if let Some(name) = tag.strip_prefix(CORE) {
        return match name {
            "str" => Kind::Str,
            "int" => Kind::Int,
            "float" => Kind::Float,
            "bool" => Kind::Bool,
            "null" => Kind::Null,
            "timestamp" => Kind::Timestamp,
            "map" => Kind::Map,
            "seq" => Kind::Seq,
            "set" => Kind::Set,
            "binary" => Kind::Binary,
            "omap" | "pairs" | "merge" | "value" | "yaml" => Kind::Unsupported,
            _ => Kind::UnknownCore,
        };
    }
    if tag.starts_with('!') { Kind::Local } else { Kind::Other }
}

/// The `%YAML` directive of the document: none, version 1.1, or another accepted version.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum Version {
    None,
    V11,
    Other,
}

/// A tagged scalar as typed by its tag.
#[derive(Debug, PartialEq)]
pub(super) enum Typed {
    Null,
    Text(String),
    Int(i64),
    Float(f64),
    Bool(bool),
}

fn plain_null(v: &str) -> bool {
    matches!(v, "" | "~" | "null" | "Null" | "NULL")
}

const UNMEASURED: &str = "ReadFromYaml of a tagged scalar no probe measured";

/// The scalar `v` of `style` under the tag `kind`. Under `%YAML 1.1` only `!!int` (`Directives2_4`, `Directives2_6`,
/// `Directives2_7`) and `!!bool` (`Directives2_5`) are measured; under another version no tagged number or Boolean is.
pub(super) fn scalar(kind: Kind, v: &str, style: Style, version: Version) -> Res<Typed> {
    let plain = style == Style::Plain;
    let versioned = match version {
        Version::None => false,
        Version::V11 => !matches!(kind, Kind::Int | Kind::Bool | Kind::Str | Kind::Map),
        Version::Other => !matches!(kind, Kind::Str | Kind::Map),
    };
    if versioned {
        return Err(Fail::Refuse(UNMEASURED));
    }
    // The null forms each kind is measured with: `Str1_4`, `Str1_5`, corpus PR 1530 `tg12`; `Int2_6`, `WrongNode1_12`;
    // `Bool2_11`, `Bool3_4`, `WrongNode1_13`; `WrongNode1_14`; `WrongNode1_10`.
    if plain && plain_null(v) {
        let measured_null = match kind {
            Kind::Str | Kind::Bool => matches!(v, "" | "~" | "null"),
            Kind::Int => matches!(v, "" | "~"),
            Kind::Float | Kind::Map => v.is_empty(),
            // `Null1_3`-`Null1_6`, `Top1_10`, corpus PR 1530 `nl9`; `WrongNode1_11`.
            Kind::Null => return Err(Fail::Bad),
            Kind::Seq if v.is_empty() => return Err(Fail::Bad),
            Kind::Local if v.is_empty() => return Err(Fail::Bad),
            _ => false,
        };
        return if measured_null {
            Ok(Typed::Null)
        } else {
            Err(Fail::Refuse(UNMEASURED))
        };
    }
    if style == Style::Block && kind != Kind::Str {
        return Err(Fail::Refuse(UNMEASURED));
    }
    match kind {
        // `Str1_*`, `Str2_1`, `Str2_2`, `Position1_19`, `Position2_4`: an empty quoted or block text stays a text
        // (`Str1_8`, `Str2_2`); a quoted null word is not measured.
        Kind::Str if !plain && !v.is_empty() && plain_null(v) => Err(Fail::Refuse(UNMEASURED)),
        Kind::Str => Ok(Typed::Text(v.to_string())),
        Kind::Int if version == Version::V11 => int11(v).map(Typed::Int),
        Kind::Int => int(v).map(Typed::Int),
        Kind::Float => float(v).map(Typed::Float),
        Kind::Bool => boolean(v).map(Typed::Bool),
        Kind::Timestamp => timestamp(v, style),
        // `Null1_*`, `Position2_5`; `!!map x` (`Other1_15`), `!!seq x` (`Other1_14`, `WrongNode1_6`), `!!set x` (`Other1_11`),
        // `!!binary` (`Other1_6`, corpus PR 1530 `tg10`), the other names above.
        Kind::Null
        | Kind::Map
        | Kind::Seq
        | Kind::Set
        | Kind::Binary
        | Kind::Unsupported
        | Kind::UnknownCore
        | Kind::Local
        | Kind::Other => Err(Fail::Bad),
    }
}

/// `!!int`: a decimal (`007`, `+5`, `-0`, `1_000`), `0x` hex (`0x1F`, `-0x1F`) or `0b` binary (`0b101`) whole number up to
/// 2147483647 (`Int1_*`, `Int2_*`, `Int3_1`). A number above (`Int1_11`-`Int1_14`, `Int3_1`), `0o17` (`Int1_7`), `0x`
/// (`Int2_13`), a decimal point or exponent and a word (`Int2_1`-`Int2_5`, `Int2_9`-`Int2_11`) are the YAML error. Other
/// underscores, a sign before `0b` or `+0x`, upper-case prefixes and negative numbers past -2147483647 are not measured.
fn int(v: &str) -> Res<i64> {
    let (neg, body) = match v.as_bytes().first() {
        Some(b'-') => (true, &v[1..]),
        Some(b'+') => (false, &v[1..]),
        _ => (false, v),
    };
    let signed = body.len() != v.len();
    let refuse = || Err(Fail::Refuse(UNMEASURED));
    let (digits, radix) = if let Some(h) = body.strip_prefix("0x") {
        if signed && !neg {
            return refuse();
        }
        (h, 16)
    } else if let Some(b) = body.strip_prefix("0b") {
        if signed {
            return refuse();
        }
        (b, 2)
    } else if body.starts_with("0X") || body.starts_with("0B") || body.starts_with("0O") {
        return refuse();
    } else {
        // `007` reads 7 (`Int1_2`); whether a leading zero makes an octal number without `%YAML 1.1` is not measured
        // where the two differ (`010`).
        let leading_zero = body.len() > 1 && body.starts_with('0') && !body.contains('_');
        if leading_zero && u64::from_str_radix(body, 8).ok() != body.parse::<u64>().ok() {
            return refuse();
        }
        if body.contains('_') {
            let groups: Vec<&str> = body.split('_').collect();
            let digit_groups = groups
                .iter()
                .all(|g| !g.is_empty() && g.bytes().all(|c| c.is_ascii_digit()));
            if !digit_groups {
                return refuse();
            }
        }
        (body, 10)
    };
    let clean: String = digits.chars().filter(|&c| c != '_').collect();
    if radix != 10 && digits.contains('_') {
        return refuse();
    }
    if clean.is_empty() || !clean.chars().all(|c| c.is_digit(radix)) {
        return Err(Fail::Bad);
    }
    let magnitude = u128::from_str_radix(&clean, radix).ok();
    match magnitude {
        Some(m) if m <= 2_147_483_647 => Ok(if neg { -(m as i64) } else { m as i64 }),
        // Measured too large: positive decimals (`Int1_11`-`Int1_14`) and 16 hex digits (`Int3_1`).
        _ if !neg && (radix == 10 || (radix == 16 && clean.len() == 16)) => Err(Fail::Bad),
        _ => refuse(),
    }
}

/// `!!int` under `%YAML 1.1`: a leading zero makes an octal number (`Directives2_4`: `010` reads 8); `0x1F` and `+5`
/// read as without the directive (`Directives2_6`, `Directives2_7`). Every other form is not measured.
fn int11(v: &str) -> Res<i64> {
    let body = v.strip_prefix('+').unwrap_or(v);
    if body.len() > 1 && body.starts_with('0') && body.bytes().all(|c| matches!(c, b'0'..=b'7')) {
        return i64::from_str_radix(body, 8)
            .ok()
            .filter(|n| *n <= 2_147_483_647)
            .ok_or(Fail::Refuse(UNMEASURED));
    }
    let measured = (body.starts_with("0x") && body.len() > 2 && body[2..].bytes().all(|c| c.is_ascii_hexdigit()))
        || (!body.starts_with('0') && !body.is_empty() && body.bytes().all(|c| c.is_ascii_digit()));
    if measured && !v.starts_with('-') {
        int(v)
    } else {
        Err(Fail::Refuse(UNMEASURED))
    }
}

/// `!!float`: a decimal number with `.`, an exponent or `_` (`Float1_*`-`Float3_*`), `.inf` and `.nan` in the measured
/// spellings with `+`/`-` on `.inf` (`Float1_7`-`Float1_14`). `nan`, `inf`, hex and words are the YAML error
/// (`Float2_1`, `Float2_2`, `Float2_4`, `Float2_12`). A sign on `.nan`, a negative number past the largest double and a
/// number that underflows to zero are not measured.
fn float(v: &str) -> Res<f64> {
    let (neg, body) = match v.as_bytes().first() {
        Some(b'-') => (true, &v[1..]),
        Some(b'+') => (false, &v[1..]),
        _ => (false, v),
    };
    if matches!(body, ".inf" | ".Inf" | ".INF") {
        return Ok(if neg { f64::NEG_INFINITY } else { f64::INFINITY });
    }
    if matches!(body, ".nan" | ".NaN" | ".NAN") {
        return if body.len() == v.len() {
            Ok(f64::NAN)
        } else {
            Err(Fail::Refuse(UNMEASURED))
        };
    }
    let number = body
        .bytes()
        .all(|c| c.is_ascii_digit() || matches!(c, b'.' | b'e' | b'E' | b'+' | b'-' | b'_'))
        && body.bytes().any(|c| c.is_ascii_digit());
    if !number {
        return Err(Fail::Bad);
    }
    if body.contains('_')
        && !(body.bytes().all(|c| c.is_ascii_digit() || c == b'_')
            && !body.starts_with('_')
            && !body.ends_with('_')
            && !body.contains("__"))
    {
        return Err(Fail::Refuse(UNMEASURED));
    }
    let clean: String = body.chars().filter(|&c| c != '_').collect();
    let x: f64 = clean.parse().map_err(|_| Fail::Refuse(UNMEASURED))?;
    let mantissa_zero = clean
        .split(['e', 'E'])
        .next()
        .unwrap_or("")
        .bytes()
        .all(|c| !matches!(c, b'1'..=b'9'));
    if (x == 0.0 && !mantissa_zero) || (x.is_infinite() && neg) {
        return Err(Fail::Refuse(UNMEASURED));
    }
    Ok(if neg { -x } else { x })
}

/// `!!bool`: yes/no, on/off, y/n and true/false in lower case, capitalised and upper case (`Bool1_*`, `Bool2_1`-`Bool2_8`,
/// `Bool2_13`, `Bool2_14`); `1`, `0`, `t`, `f`, `1.0` and words are the YAML error (`Bool2_9`, `Bool2_10`, `Bool2_12`,
/// `Bool3_1`-`Bool3_3`). Other mixed cases of the words are not measured.
fn boolean(v: &str) -> Res<bool> {
    const WORDS: [(&str, bool); 8] = [
        ("yes", true),
        ("no", false),
        ("on", true),
        ("off", false),
        ("y", true),
        ("n", false),
        ("true", true),
        ("false", false),
    ];
    let lower = v.to_ascii_lowercase();
    let Some(&(_, b)) = WORDS.iter().find(|(w, _)| *w == lower) else {
        return Err(Fail::Bad);
    };
    let capital = {
        let mut c = lower.clone();
        c[..1].make_ascii_uppercase();
        c
    };
    if v == lower || v == capital || v == lower.to_ascii_uppercase() {
        Ok(b)
    } else {
        Err(Fail::Refuse(UNMEASURED))
    }
}

/// `!!timestamp`: exactly the measured cells (`Other1_1`-`Other1_5`, corpus PR 1530 `tg24`); the UTC forms depend on the
/// time zone of the measuring tier, so no rule is drawn from them.
fn timestamp(v: &str, style: Style) -> Res<Typed> {
    if style != Style::Plain {
        return Err(Fail::Refuse(UNMEASURED));
    }
    let t = match v {
        "2001-12-14" => "2001-12-14T00:00:00",
        "2001-12-14t21:59:43.10-05:00" | "2001-12-14 21:59:43.10 -5" => "2001-12-15T02:59:43.1+00:00",
        "2002-12-14T21:59:43Z" => "2002-12-14T21:59:43Z",
        "x" => return Err(Fail::Bad),
        _ => return Err(Fail::Refuse(UNMEASURED)),
    };
    Ok(Typed::Text(t.into()))
}

/// The decimal digits and exponent of a finite double, shortest round trip (`1.5e-7` -> (`15`, -7)).
fn digits(x: f64) -> (String, i32) {
    let s = format!("{:e}", x.abs());
    let (m, e) = s.split_once('e').expect("exponent");
    (m.replace('.', ""), e.parse().expect("exponent"))
}

/// A double as BC writes it into JSON: the shortest round trip, `.0` after a whole number, `E+XX`/`E-XX` below 1E-04
/// and from 1E+17 (`Float1_*`-`Float3_*`), `Infinity`, `-Infinity` and `NaN` as texts (`Float1_5`, `Float1_7`-`Float1_14`).
/// Exponents from -4 to -2 and from 6 to 16 are not measured.
pub(super) fn float_json(x: f64) -> Res<super::Yaml> {
    use super::Yaml;
    if x.is_nan() {
        return Ok(Yaml::Text("NaN".into()));
    }
    if x.is_infinite() {
        return Ok(Yaml::Text(if x > 0.0 { "Infinity" } else { "-Infinity" }.into()));
    }
    let (d, e) = digits(x);
    let sign = if x.is_sign_negative() { "-" } else { "" };
    if (-1..=5).contains(&e) {
        let t = format!("{}", x);
        return Ok(Yaml::Raw(if t.contains('.') { t } else { format!("{t}.0") }));
    }
    if e <= -5 || e >= 17 {
        let m = if d.len() > 1 {
            format!("{}.{}", &d[..1], &d[1..])
        } else {
            d
        };
        let es = if e < 0 { '-' } else { '+' };
        return Ok(Yaml::Raw(format!("{sign}{m}E{es}{:02}", e.abs())));
    }
    let _ = sign;
    Err(Fail::Refuse(
        "ReadFromYaml of a !!float whose JSON form no probe measured",
    ))
}

/// The text a tagged scalar key is written as: the value in .NET's text form (`Keys1_1`, `Keys1_2`, `Keys1_7`, `Keys1_8`,
/// `Keys1_13`, `Keys1_14`, `Keys1_16`, `Keys1_17`, `Keys2_4`, `Keys2_5`, `Keys2_7`, `Keys2_8`). An empty `!!str` key is the
/// YAML error (`Keys1_3`); a null under another tag, a double outside the measured forms and a timestamp key are not
/// measured.
pub(super) fn key(kind: Kind, v: &str, style: Style, version: Version) -> Res<String> {
    if kind == Kind::Str {
        return match (v, style) {
            ("", Style::Plain) => Err(Fail::Bad),
            ("null" | "Null" | "NULL", _) => Err(Fail::Refuse(UNMEASURED)),
            _ if style == Style::Block => Err(Fail::Refuse(UNMEASURED)),
            _ => Ok(v.to_string()),
        };
    }
    if matches!(
        kind,
        Kind::Timestamp | Kind::Map | Kind::Seq | Kind::Set | Kind::Binary | Kind::UnknownCore | Kind::Other
    ) {
        return Err(Fail::Refuse(
            "ReadFromYaml of a scalar key under a tag no probe measured",
        ));
    }
    match scalar(kind, v, style, version)? {
        Typed::Int(i) => Ok(i.to_string()),
        Typed::Bool(b) => Ok(if b { "True" } else { "False" }.into()),
        Typed::Float(x) if x == f64::INFINITY => Ok("Infinity".into()),
        Typed::Float(x) if x.is_finite() && (-1..=5).contains(&digits(x).1) => Ok(format!("{x}")),
        _ => Err(Fail::Refuse(UNMEASURED)),
    }
}
