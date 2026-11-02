//! Text to value conversion on import (types, durations, dates) and the helpers it needs.

use super::*;

impl Session {
    /// The value an imported text becomes in a field of type `ty`. An empty text is the type's
    /// default (xmlport/TestXmlPortExportImportContracts.al:XmlPortImport_EmptyIntegerElement_ImportsZero,
    /// and an export writes a blank date as an empty element).
    pub(super) fn xp_value(&mut self, text: &str, ty: &RtType, xml: bool) -> Result<Value, Signal> {
        if xml {
            return self.xp_value_xml(text, ty, None);
        }
        // A Legacy port reads the display text only: the XML date is refused and nothing is stored
        // (xmlport/TestXmlPortFormatEvaluateContracts.al:Import_LegacyXmlPort_TakesTheDisplayText_AndRefusesTheXmlDate).
        if matches!(ty, RtType::Date)
            && al_builtins::iso::parse_date(text).is_some()
            && text.as_bytes().get(4) == Some(&b'-')
        {
            return Err(xerr("The date is not valid.", "DateInvalid"));
        }
        match ty {
            RtType::Text(len) | RtType::Code(len) => {
                let n = text.encode_utf16().count();
                if let Some(m) = len.filter(|m| n > *m as usize) {
                    // xmlport/TestXmlPortExportImportContracts.al:XmlPortImport_ValueTooLongForField_Throws
                    return Err(xerr(
                        format!(
                            "The length of the string is {n}, but it must be less than or equal to {m} characters. Value: {}",
                            ops::shown_in_length_error(text)
                        ),
                        "StringLengthExceeded",
                    ));
                }
                Ok(ops::convert(Value::Text(text.to_string().into()), ty)?)
            }
            RtType::Integer | RtType::BigInteger => {
                if text.is_empty() {
                    return Ok(self.default_value(ty));
                }
                let digits = text.strip_prefix('-').unwrap_or(text);
                let ok = !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit());
                match (ty, ok.then(|| text.parse::<i64>().ok()).flatten()) {
                    (RtType::Integer, Some(n)) if i32::try_from(n).is_ok() => Ok(Value::Integer(n as i32)),
                    (RtType::BigInteger, Some(n)) => Ok(Value::BigInteger(n)),
                    _ => Err(evaluate_error(
                        text,
                        if *ty == RtType::Integer {
                            "Integer"
                        } else {
                            "BigInteger"
                        },
                    )),
                }
            }
            RtType::DateTime if text.ends_with(" AM") || text.ends_with(" PM") => {
                // The display text an export writes: `03/05/24 02:30 PM`
                // (xmlport/TestXmlPortExportImportContracts.al:XmlPortImport_AcceptsTheTextItExports).
                let Some((date, clock)) = text.split_once(' ') else {
                    return Err(evaluate_error(text, "DateTime"));
                };
                let Value::Date(day, _) = self.xp_value(date, &RtType::Date, false)? else {
                    return Err(evaluate_error(text, "DateTime"));
                };
                let (hm, ampm) = clock.rsplit_once(' ').unwrap_or((clock, ""));
                // The display text of a DateTime with or without seconds (`03/05/24 12:34:56 PM` is what the
                // Legacy probe imports, xmlport/TestXmlPortFormatEvaluateContracts.al:167).
                let clock = if hm.matches(':').count() == 2 {
                    format!("{hm} {ampm}")
                } else {
                    format!("{hm}:00 {ampm}")
                };
                let ms = parse_time_12h(&clock).ok_or_else(|| evaluate_error(text, "DateTime"))?;
                Ok(Value::DateTime((i64::from(day) - 1) * 86_400_000 + ms + 1))
            }
            RtType::Decimal | RtType::Date | RtType::DateTime | RtType::Boolean => {
                if text.is_empty() && *ty != RtType::Boolean {
                    return Ok(self.default_value(ty));
                }
                let name = match ty {
                    RtType::Decimal => "Decimal",
                    RtType::Date => "Date",
                    RtType::DateTime => "DateTime",
                    _ => "Boolean",
                };
                let mut args = [self.default_value(ty), Value::Text(text.to_string().into())];
                self.ret_used = true;
                let ok = al_builtins::evaluate::evaluate(self, None, &mut args)?;
                if matches!(ok, Value::Boolean(true)) {
                    Ok(args[0].clone())
                } else {
                    Err(evaluate_error(text, name))
                }
            }
            RtType::Time => {
                if text.trim().is_empty() {
                    return Ok(Value::Time(0));
                }
                parse_time_12h(text.trim())
                    .map(|ms| Value::Time(ms + 1))
                    .ok_or_else(|| evaluate_error(text, "Time"))
            }
            RtType::Duration => {
                if text.trim().is_empty() {
                    return Ok(Value::Duration(0));
                }
                parse_duration_text(text)
                    .map(Value::Duration)
                    .ok_or_else(|| evaluate_error(text, "Duration"))
            }
            RtType::Option(members) => {
                let t = text.trim();
                let found = members.iter().position(|m| m.eq_ignore_ascii_case(t));
                match found {
                    Some(i) => Ok(Value::Option(i as i32, members.clone().into())),
                    None if t.is_empty() => Ok(Value::Option(0, members.clone().into())),
                    None => Err(evaluate_error(text, "Option")),
                }
            }
            RtType::Enum(id) => {
                self.note_enum(*id);
                let members = self.engine.program.objects[*id as usize].enum_values.clone();
                let t = text.trim();
                match members.iter().find(|(_, n)| n.eq_ignore_ascii_case(t)) {
                    Some((o, _)) => Ok(Value::Enum(*id, *o)),
                    None if t.is_empty() => Ok(Value::Enum(*id, members.first().map_or(0, |m| m.0))),
                    None => Err(evaluate_error(text, "Enum")),
                }
            }
            // A TableFilter field reads its text as Evaluate does: the table-qualified text, another table's, the empty text
            // (xmlport/lt-y67-xmlport-tablefilter/Y67XpTableFilter.al:140-171 TableFilterField_Import). A text Evaluate rejects
            // is not pinned: BC's error for an unknown table (`Id: 1..9`) is pinned only in part.
            RtType::Library(k) if k.as_str() == "TableFilter" => {
                let mut args = [self.default_value(ty), Value::Text(text.to_string().into())];
                self.ret_used = true;
                match al_builtins::evaluate::evaluate(self, None, &mut args)? {
                    Value::Boolean(true) => Ok(args[0].clone()),
                    _ => unsupported(
                        "XmlPort import of a text Evaluate does not take into a TableFilter field (error not pinned)",
                    ),
                }
            }
            other => unsupported(format!(
                "XmlPort import into a field of type {} (not pinned)",
                other.describe()
            )),
        }
    }

    /// The value an imported text becomes under `FormatEvaluate = Xml`: the XML forms only
    /// (xmlport/TestXmlPortFormatEvaluateContracts.al:Import_XmlPort_TakesTheXmlForms_AndRefusesTheDisplayText).
    /// A Date that is not `yyyy-mm-dd` fails with the beginning of BC's text; the rest of that text is not
    /// pinned (docs/probes-needed.md). Display text for another type, and the types the probe does not
    /// cover, are refused.
    pub(super) fn xp_value_xml(
        &mut self,
        text: &str,
        ty: &RtType,
        members: Option<Vec<(i32, String, String)>>,
    ) -> Result<Value, Signal> {
        match ty {
            // corpus xmlport/PWXmlPortFormatEvaluate.al:Import_Xml_Time: `hh:mm:ss` with optional
            // milliseconds; the 12-hour display text is refused.
            RtType::Time if text.is_empty() => Ok(Value::Time(0)),
            RtType::Time => al_builtins::iso::parse_time(text)
                .map(Value::Time)
                .ok_or_else(|| evaluate_error(text, "Time")),
            // Import_Xml_Duration: the ISO 8601 form (`PT1H`); milliseconds and words are refused.
            RtType::Duration if text.is_empty() => Ok(Value::Duration(0)),
            RtType::Duration => parse_iso_duration(text)
                .map(Value::Duration)
                .ok_or_else(|| evaluate_error(text, "Duration")),
            // Import_Xml_Option, Import_Xml_Enum: the member name, its ordinal or its caption.
            RtType::Option(names) => {
                let t = text.trim();
                if t.is_empty() {
                    return Ok(Value::Option(0, names.clone().into()));
                }
                let by_name = names.iter().position(|m| m.eq_ignore_ascii_case(t));
                let by_caption = members
                    .as_ref()
                    .and_then(|m| m.iter().position(|(_, _, c)| c.eq_ignore_ascii_case(t)));
                let by_ordinal = t.parse::<usize>().ok().filter(|i| *i < names.len());
                match by_name.or(by_ordinal).or(by_caption) {
                    Some(i) => Ok(Value::Option(i as i32, names.clone().into())),
                    None => Err(evaluate_error(text, "Option")),
                }
            }
            RtType::Enum(id) => {
                self.note_enum(*id);
                let t = text.trim();
                let list = match members {
                    Some(m) => m,
                    None => self.engine.program.objects[*id as usize]
                        .enum_values
                        .iter()
                        .map(|(o, n)| (*o, n.clone(), n.clone()))
                        .collect(),
                };
                if t.is_empty() {
                    return Ok(Value::Enum(*id, list.first().map_or(0, |m| m.0)));
                }
                let by_name = list.iter().find(|(_, n, _)| n.eq_ignore_ascii_case(t));
                let by_ordinal = t
                    .parse::<i32>()
                    .ok()
                    .and_then(|o| list.iter().find(|(v, _, _)| *v == o));
                let by_caption = list.iter().find(|(_, _, c)| c.eq_ignore_ascii_case(t));
                match by_name.or(by_ordinal).or(by_caption) {
                    Some((o, _, _)) => Ok(Value::Enum(*id, *o)),
                    // An ordinal the enum does not declare is imported as it is; a text that names no member raises with the
                    // captions of the members (xmlport/PWXmlPortFormatEvaluate.al:Import_Xml_Enum_Invalid, `7` and `Two`).
                    None if !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit()) => match t.parse::<i32>() {
                        Ok(o) => Ok(Value::Enum(*id, o)),
                        Err(_) => unsupported(format!("XmlPort import of the ordinal {t} into an Enum (not pinned)")),
                    },
                    None if t == text => Err(xerr(
                        format!(
                            "'{t}' is not an option. The existing options are: {}",
                            list.iter().map(|(_, _, c)| c.as_str()).collect::<Vec<_>>().join(",")
                        ),
                        "InvalidOptionString",
                    )),
                    None => unsupported(
                        "XmlPort import of a text with surrounding spaces that names no Enum member (not pinned)",
                    ),
                }
            }
            RtType::Text(_) | RtType::Code(_) | RtType::Integer | RtType::BigInteger => self.xp_value(text, ty, false),
            // An empty element is the type's default, a Boolean's too (XpTImportBasic.al:Import_EmptyValuesExplicit).
            RtType::Date | RtType::DateTime | RtType::Decimal | RtType::Boolean if text.is_empty() => {
                Ok(self.default_value(ty))
            }
            RtType::Date => {
                match al_builtins::iso::parse_date(text).filter(|_| text.as_bytes().get(4) == Some(&b'-')) {
                    Some(d) => Ok(Value::Date(d, false)),
                    None => Err(xerr(
                        format!("The format of the Date '{text}' does not match your device's date settings."),
                        "DateInvalid",
                    )),
                }
            }
            RtType::DateTime => {
                match al_builtins::iso::parse_datetime(text).filter(|_| text.as_bytes().get(4) == Some(&b'-')) {
                    Some(v) => Ok(Value::DateTime(v)),
                    None => Err(evaluate_error(text, "DateTime")),
                }
            }
            RtType::Decimal => {
                let plain = {
                    let digits = text.strip_prefix('-').unwrap_or(text);
                    !digits.is_empty()
                        && digits.split('.').count() <= 2
                        && digits.split('.').all(|p| p.bytes().all(|b| b.is_ascii_digit()))
                        && !digits.starts_with('.')
                };
                match text.parse::<rust_decimal::Decimal>() {
                    Ok(d) if plain => Ok(Value::Decimal(d)),
                    _ => Err(evaluate_error(text, "Decimal")),
                }
            }
            // Import_Xml_DisplayTextOfDecimalBooleanDateTime: `true`, `false` and `1`, `0`; `Yes` is refused.
            RtType::Boolean => match text {
                "true" | "1" => Ok(Value::Boolean(true)),
                "false" | "0" => Ok(Value::Boolean(false)),
                _ => Err(evaluate_error(text, "Boolean")),
            },
            // QrXpContainerOccurs.al TS_Guid_Import_* on branch al-runtime-rs-staging-pub-ts (former assumption `xmlport-guid-xml` (pinned on corpus master since bump 1030)).
            RtType::Guid => {
                // Pinned by xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al TS_Guid_Import_Element, TS_Guid_Import_Attribute, TS_Guid_Export (was assumption `xmlport-guid-xml`).
                match xml_guid(text) {
                    XmlGuid::Value(g) => Ok(Value::Guid(g)),
                    XmlGuid::Invalid => Err(xerr(
                        "Invalid format of GUID string. The correct format of the GUID string is: CDEF7890-ABCD-0123-1234-567890ABCDEF where 0-9, A-F symbolizes hexadecimal digits.",
                        "InvalidGuidFormat",
                    )),
                    XmlGuid::Unpinned => unsupported(format!(
                        "XmlPort import of `{text}` into a Guid field with FormatEvaluate = Xml (this form is not pinned)"
                    )),
                }
            }
            other => unsupported(format!(
                "XmlPort import into a field of type {} with FormatEvaluate = Xml (the XML form is not pinned)",
                other.describe()
            )),
        }
    }
}

/// `h:mm:ss AM` as milliseconds since midnight.
pub(super) fn parse_time_12h(t: &str) -> Option<i64> {
    let (clock, ampm) = t.rsplit_once(' ')?;
    let mut it = clock.split(':');
    let (h, m, s): (i64, i64, i64) = (
        it.next()?.parse().ok()?,
        it.next()?.parse().ok()?,
        it.next()?.parse().ok()?,
    );
    if it.next().is_some() || !(1..=12).contains(&h) || m > 59 || s > 59 {
        return None;
    }
    let pm = match ampm {
        "AM" => false,
        "PM" => true,
        _ => return None,
    };
    Some((((h % 12) + if pm { 12 } else { 0 }) * 3600 + m * 60 + s) * 1000)
}

/// `1 day 1 hour 1 minute 1 second 1 millisecond` (the text an export writes) as milliseconds.
/// An ISO 8601 duration (`P1DT1H1M1.001S`, `PT1H`) in milliseconds.
pub(super) fn parse_iso_duration(t: &str) -> Option<i64> {
    let rest = t.strip_prefix('P')?;
    let (days, time) = match rest.split_once('T') {
        Some((d, tm)) => (d, tm),
        None => (rest, ""),
    };
    if days.is_empty() && time.is_empty() {
        return None;
    }
    let mut total = 0i64;
    let mut take = |part: &str, units: &[(char, f64)]| -> Option<()> {
        let mut num = String::new();
        for c in part.chars() {
            if c.is_ascii_digit() || c == '.' {
                num.push(c);
                continue;
            }
            let mult = units.iter().find(|(u, _)| *u == c)?.1;
            let n: f64 = num.parse().ok()?;
            total += (n * mult).round() as i64;
            num.clear();
        }
        num.is_empty().then_some(())
    };
    take(days, &[('D', 86_400_000.0)])?;
    take(time, &[('H', 3_600_000.0), ('M', 60_000.0), ('S', 1000.0)])?;
    Some(total)
}

/// `P1DT1H1M1.001S` for 90061001 ms; zero parts are left out.
pub(super) fn iso_duration(ms: i64) -> String {
    let (days, rest) = (ms / 86_400_000, ms % 86_400_000);
    let (h, rest) = (rest / 3_600_000, rest % 3_600_000);
    let (m, rest) = (rest / 60_000, rest % 60_000);
    let (sec, milli) = (rest / 1000, rest % 1000);
    let mut out = String::from("P");
    if days != 0 {
        out.push_str(&format!("{days}D"));
    }
    if h != 0 || m != 0 || rest != 0 {
        out.push('T');
        if h != 0 {
            out.push_str(&format!("{h}H"));
        }
        if m != 0 {
            out.push_str(&format!("{m}M"));
        }
        if rest != 0 {
            out.push_str(&if milli == 0 {
                format!("{sec}S")
            } else {
                format!("{sec}.{milli:03}S")
            });
        }
    }
    out
}

/// The white space characters of XML.
pub(super) const XML_SPACE: [char; 4] = [' ', '\t', '\n', '\r'];

/// `t` with each run of XML white space made one space and the ends trimmed.
pub(super) fn collapse_white_space(t: &str) -> String {
    t.split(XML_SPACE)
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn parse_duration_text(t: &str) -> Option<i64> {
    let mut total = 0i64;
    let mut words = t.split_whitespace();
    let mut any = false;
    while let Some(n) = words.next() {
        let n: i64 = n.parse().ok()?;
        let unit = words.next()?.trim_end_matches('s');
        total += n * match unit {
            "day" => 86_400_000,
            "hour" => 3_600_000,
            "minute" => 60_000,
            "second" => 1000,
            "millisecond" => 1,
            _ => return None,
        };
        any = true;
    }
    any.then_some(total)
}
