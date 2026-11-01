//! HTTP date forms (RFC 1123, RFC 850, asctime) as `HttpHeaders.Add` accepts them.

use super::*;

pub(super) const WEEKDAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
pub(super) const WEEKDAYS_LONG: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];
pub(super) const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// Days since 1970-01-01 of a proleptic Gregorian date.
pub(super) fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

pub(super) fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { yoe + era * 400 + 1 } else { yoe + era * 400 }, m, d)
}

pub(super) fn digits_of(s: &str, len: std::ops::RangeInclusive<usize>) -> Option<i64> {
    (len.contains(&s.len()) && s.chars().all(|c| c.is_ascii_digit())).then(|| s.parse().ok())?
}

/// `HH:MM:SS` as seconds of the day; the hour may have one digit (SeHeaderValues.al:530-571 Y2_Shapes_DatesTwoDigitYearsOffsets
/// `ifmodifiedsince_8`, `8:49:37`).
pub(super) fn time_of_day(s: &str) -> Option<i64> {
    let mut p = s.split(':');
    let (h, m, sec) = (p.next()?, p.next()?, p.next()?);
    if p.next().is_some() {
        return None;
    }
    let (h, m, sec) = (digits_of(h, 1..=2)?, digits_of(m, 2..=2)?, digits_of(sec, 2..=2)?);
    (h < 24 && m < 60 && sec < 60).then_some(h * 3600 + m * 60 + sec)
}

/// Seconds since 1970 of a valid date and time; the weekday given, when there is one, must be the date's.
pub(super) fn instant(y: i64, mon: &str, d: i64, secs: i64, weekday: Option<usize>) -> Option<i64> {
    // The month in any case (SeHeaderValues.al:530-571 `ifmodifiedsince_5`, `sun, 06 nov 1994 08:49:37 GMT`).
    let m = MONTHS.iter().position(|x| x.eq_ignore_ascii_case(mon))? as i64 + 1;
    let days = days_from_civil(y, m, d);
    if civil_from_days(days) != (y, m, d) {
        return None;
    }
    if weekday.is_some_and(|w| w as i64 != (days + 4).rem_euclid(7)) {
        return None;
    }
    Some(days * 86_400 + secs)
}

/// An instant written the way .NET writes a date header: `Tue, 15 Nov 1994 08:12:31 GMT`.
pub(super) fn rfc1123(t: i64) -> String {
    let days = t.div_euclid(86_400);
    let secs = t.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    format!(
        "{}, {d:02} {} {y:04} {:02}:{:02}:{:02} GMT",
        WEEKDAYS[(days + 4).rem_euclid(7) as usize],
        MONTHS[(m - 1) as usize],
        secs / 3600,
        secs / 60 % 60,
        secs % 60
    )
}

/// The date forms network/lt-se-header-values/SeHeaderValues.al:SE_Shapes_Dates measured (corpus bump PR 737), all stored as
/// RFC 1123 GMT: `Tue, 15 Nov 1994 08:12:31 GMT` (also lt-http-header-rules/P87Values.al, AyHeaderAddResult.al:119), the same with
/// `UTC`, RFC 850 `Tuesday, 15-Nov-94 08:12:31 GMT`, asctime `Tue Nov 15 08:12:31 1994` and `15 Nov 1994 08:12:31 +0100` (the offset
/// is taken off). `None` for another text.
/// Writer Y2 (SeHeaderValues.al:530-571, Y2_Shapes_DatesTwoDigitYearsOffsets) measured more of the RFC 1123 form: the weekday and
/// month in lower case (with `GMT` in capitals), no blank after the comma, a one-digit hour and a numeric offset (`-0500` is
/// stored five hours later, `+0000` as GMT).
pub(super) fn parse_date_forms(v: &str) -> Option<i64> {
    let spaced = match v.split_once(',') {
        Some((wd, rest)) if !rest.starts_with(' ') && !wd.contains(' ') => format!("{wd}, {rest}"),
        _ => v.to_string(),
    };
    let p: Vec<&str> = spaced.split_whitespace().collect();
    let weekday = |w: &str, names: &[&str; 7]| names.iter().position(|x| *x == w);
    let weekday_any_case = |w: &str| WEEKDAYS.iter().position(|x| x.eq_ignore_ascii_case(w));
    match p.as_slice() {
        [wd, d, mon, y, time, zone] if wd.ends_with(',') && matches!(*zone, "GMT" | "UTC") => {
            let w = weekday_any_case(&wd[..wd.len() - 1])?;
            instant(
                digits_of(y, 4..=4)?,
                mon,
                digits_of(d, 1..=2)?,
                time_of_day(time)?,
                Some(w),
            )
        }
        [wd, d, mon, y, time, zone] if wd.ends_with(',') && zone.len() == 5 && zone.starts_with(['+', '-']) => {
            let w = weekday_any_case(&wd[..wd.len() - 1])?;
            let hh = digits_of(&zone[1..3], 2..=2)?;
            let mm = digits_of(&zone[3..5], 2..=2)?;
            let offset = (hh * 3600 + mm * 60) * if zone.starts_with('-') { -1 } else { 1 };
            Some(
                instant(
                    digits_of(y, 4..=4)?,
                    mon,
                    digits_of(d, 1..=2)?,
                    time_of_day(time)?,
                    Some(w),
                )? - offset,
            )
        }
        [wd, dmy, time, "GMT"] if wd.ends_with(',') => {
            let w = weekday(&wd[..wd.len() - 1], &WEEKDAYS_LONG)?;
            let mut parts = dmy.split('-');
            let (d, mon, yy) = (parts.next()?, parts.next()?, parts.next()?);
            if parts.next().is_some() {
                return None;
            }
            // A two-digit year of 50 or more is in the 1900s (94 is the measured one); a smaller one is not measured.
            let yy = digits_of(yy, 2..=2)?;
            if yy < 50 {
                return None;
            }
            instant(1900 + yy, mon, digits_of(d, 1..=2)?, time_of_day(time)?, Some(w))
        }
        [wd, mon, d, time, y] if WEEKDAYS.contains(wd) => {
            let w = weekday(wd, &WEEKDAYS)?;
            instant(
                digits_of(y, 4..=4)?,
                mon,
                digits_of(d, 1..=2)?,
                time_of_day(time)?,
                Some(w),
            )
        }
        [d, mon, y, time, zone] if zone.len() == 5 && zone.starts_with(['+', '-']) => {
            let hh = digits_of(&zone[1..3], 2..=2)?;
            let mm = digits_of(&zone[3..5], 2..=2)?;
            let offset = (hh * 3600 + mm * 60) * if zone.starts_with('-') { -1 } else { 1 };
            Some(
                instant(
                    digits_of(y, 4..=4)?,
                    mon,
                    digits_of(d, 1..=2)?,
                    time_of_day(time)?,
                    None,
                )? - offset,
            )
        }
        _ => None,
    }
}

/// A date header value: `Ok(Some(stored))`, `Ok(None)` for a refused one (InvalidHeader:Format), or a refusal of a text no probe
/// measured. Refused: a text without a digit or without a space (`not a date`, `yesterday`, `soon`, `0`, `2020-01-01`,
/// `1994-11-15T08:12:31Z`: lt-http/HttpHeaders.al:351, AyHeaderAddResult.al:131-132, :231, :315-328, P87Values.al:413-454) and a
/// date in lower case (SeHeaderValues.al:SE_Shapes_Dates `ifunmodifiedsince_4`).
pub(super) fn http_date(v: &str) -> Result<Option<String>, Signal> {
    if let Some(t) = parse_date_forms(v) {
        let stored = rfc1123(t);
        return Ok(Some(stored));
    }
    if !v.contains(|c: char| c.is_ascii_digit()) || !v.contains(' ') {
        return Ok(None);
    }
    // Refused (SeHeaderValues.al:530-571): the RFC 1123 form with a two-digit year (`Sun, 06 Nov 49 08:49:37 GMT` and `50`) and a
    // date without a time (`06 Nov 1994`).
    let p: Vec<&str> = v.split_whitespace().collect();
    match p.as_slice() {
        [wd, _, _, y, _, "GMT"] if wd.ends_with(',') && matches!(*y, "49" | "50") => return Ok(None),
        [d, mon, y]
            if d.len() <= 2
                && MONTHS.contains(mon)
                && y.len() == 4
                && d.parse::<u8>().is_ok()
                && y.parse::<u16>().is_ok() =>
        {
            return Ok(None);
        }
        _ => {}
    }
    if !v.contains(|c: char| c.is_ascii_uppercase()) {
        let capitalized: String = v
            .split(' ')
            .map(|w| {
                let mut c = w.chars();
                c.next()
                    .map_or_else(String::new, |f| f.to_ascii_uppercase().to_string() + c.as_str())
            })
            .collect::<Vec<_>>()
            .join(" ")
            .replace("Gmt", "GMT");
        if parse_date_forms(&capitalized).is_some() {
            return Ok(None);
        }
    }
    unsupported(format!("HttpHeaders.Add of the date '{v}': its form is not measured"))
}
