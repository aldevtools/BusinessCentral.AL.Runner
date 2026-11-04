//! ISO 8601 text to AL date/time values (JSON strings, XML-format Evaluate).
//!
//! Representations follow `Value`: a Date is a day number (1 = 0001-01-01), a
//! Time is milliseconds since midnight plus one, a DateTime is milliseconds
//! since 0001-01-01 00:00 UTC plus one (0 is the undefined value of each).

fn digits(s: &str, n: usize) -> Option<u32> {
    (s.len() == n && s.bytes().all(|b| b.is_ascii_digit())).then(|| s.parse().ok())?
}

pub(crate) fn days_in_month(y: i32, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 => 29,
        _ => 28,
    }
}

/// `yyyy-MM-dd`, optionally followed by a time part that is ignored.
pub fn parse_date(s: &str) -> Option<i32> {
    let head = s.get(..10)?;
    if s.len() > 10 && !s[10..].starts_with('T') {
        return None;
    }
    let (y, rest) = head.split_once('-')?;
    let (m, d) = rest.split_once('-')?;
    let (y, m, d) = (digits(y, 4)? as i32, digits(m, 2)?, digits(d, 2)?);
    if y < 1 || !(1..=12).contains(&m) || d < 1 || d > days_in_month(y, m) {
        return None;
    }
    Some(crate::format::day_from_civil(y, m, d))
}

/// `HH:mm:ss` with optional `.fraction`.
pub fn parse_time(s: &str) -> Option<i64> {
    let (hms, frac) = s.split_once('.').map_or((s, ""), |(a, b)| (a, b));
    let mut it = hms.split(':');
    let (h, m, sec) = (digits(it.next()?, 2)?, digits(it.next()?, 2)?, digits(it.next()?, 2)?);
    if it.next().is_some() || h > 23 || m > 59 || sec > 59 {
        return None;
    }
    let ms = if frac.is_empty() {
        0
    } else {
        if !frac.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let three: String = frac.chars().chain("000".chars()).take(3).collect();
        three.parse::<i64>().ok()?
    };
    Some((i64::from(h) * 3600 + i64::from(m) * 60 + i64::from(sec)) * 1000 + ms + 1)
}

/// `yyyy-MM-ddTHH:mm:ss[.fraction][Z|+hh:mm|-hh:mm]`; no offset means UTC.
pub fn parse_datetime(s: &str) -> Option<i64> {
    let (date, time) = s.split_once('T')?;
    let day = parse_date(date)?;
    let (clock, offset_min) = if let Some(c) = time.strip_suffix('Z') {
        (c, 0)
    } else if let Some(i) = time.rfind(['+', '-']).filter(|&i| i >= 8) {
        let (c, off) = time.split_at(i);
        let sign = if off.starts_with('-') { -1 } else { 1 };
        let (oh, om) = off[1..].split_once(':')?;
        (c, sign * (i64::from(digits(oh, 2)?) * 60 + i64::from(digits(om, 2)?)))
    } else {
        (time, 0)
    };
    let t = parse_time(clock)? - 1;
    Some((i64::from(day) - 1) * 86_400_000 + t - offset_min * 60_000 + 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::day_from_civil;

    #[test]
    fn dates() {
        // json/TestJsonComplete.al JsonObject_GetDate_ReturnsDate parses "2024-01-15".
        assert_eq!(parse_date("2024-01-15"), Some(day_from_civil(2024, 1, 15)));
        assert_eq!(parse_date("2024-01-15T12:00:00"), Some(day_from_civil(2024, 1, 15)));
        assert_eq!(parse_date("2024-13-15"), None);
        assert_eq!(parse_date("2024-02-30"), None);
        assert_eq!(parse_date("hello"), None);
        assert_eq!(parse_date(""), None);
    }

    #[test]
    fn times() {
        // json/TestJsonComplete.al JsonObject_GetTime_ReturnsTime parses "12:00:00".
        assert_eq!(parse_time("12:00:00"), Some(43_200_000 + 1));
        assert_eq!(parse_time("00:00:00"), Some(1));
        assert_eq!(parse_time("12:00:00.5"), Some(43_200_500 + 1));
        assert_eq!(parse_time("25:00:00"), None);
        assert_eq!(parse_time("12:00"), None);
    }

    #[test]
    fn datetimes() {
        // json/TestJsonComplete.al JsonObject_GetDateTime_ReturnsDateTime parses "2024-01-15T12:00:00";
        // session-system/TestSessionEvaluateRegionalPatterns.al:38 "2026-01-02T10:11:12.0000000Z".
        let day = i64::from(day_from_civil(2024, 1, 15)) - 1;
        assert_eq!(
            parse_datetime("2024-01-15T12:00:00"),
            Some(day * 86_400_000 + 43_200_000 + 1)
        );
        assert_eq!(
            parse_datetime("2024-01-15T12:00:00Z"),
            parse_datetime("2024-01-15T12:00:00")
        );
        assert_eq!(
            parse_datetime("2024-01-15T14:00:00+02:00"),
            parse_datetime("2024-01-15T12:00:00")
        );
        assert!(parse_datetime("2026-01-02T10:11:12.0000000Z").is_some());
        assert_eq!(parse_datetime("2024-01-15"), None);
        assert_eq!(parse_datetime("nonsense"), None);
    }
}
