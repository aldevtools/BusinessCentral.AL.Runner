//! The format regions whose texts the corpus pins, and what `Format` and a dataset column write in each.
//!
//! Values: reports/runtime/YReportPropsTests.al:34-148 (de-DE, en-GB, fr-FR, ja-JP, zz-ZZ), misc/lt-probes-needed-8/
//! PnH4CurrReport.al:241-249 (de-DE, fr-FR, da-DK) and reports/runtime/BdRegionTests.al (all of them, with 5 March 2024
//! 09:05:07 as the one-digit day and hour). BdRegionTests.al asserts en-GB, da-DK, nl-NL, es-ES, sv-SE and nb-NO under `#if BC29PLUS`
//! only (`Observer.Discard()` below), so a text no earlier test pinned is refused there on BC 27 and 28.

use super::{Value, civil_from_day};

/// The format regions whose texts the corpus pins (exact spelling; any other region is refused).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Region {
    DeDE,
    EnGB,
    FrFR,
    JaJP,
    DaDK,
    /// `en-US` as a report's `FormatRegion` property: the session's own texts, in a column as in `Format` (`1/31/2025 2:30:15 PM`,
    /// BdRegionTests.al:267).
    EnUS,
    /// `zz-ZZ`, a region no culture has: en-US texts, but `MM/dd/yyyy` dates and a 24-hour clock in a dataset column
    /// (reports/runtime/YReportPropsTests.al:FormatRegion_UnknownRegion_XmlOrError).
    ZzZZ,
    NlNL,
    DeCH,
    ItIT,
    EsES,
    /// `es-ES_tradnl`: the texts of es-ES, measured on every version (BdRegionTests.al:1348, :1484).
    EsTradnl,
    SvSE,
    NbNO,
}

impl Region {
    /// The region a `FormatRegion` text names, `None` when it is not pinned.
    pub fn parse(name: &str) -> Option<Region> {
        Some(match name {
            "de-DE" => Region::DeDE,
            "en-GB" => Region::EnGB,
            "fr-FR" => Region::FrFR,
            "ja-JP" => Region::JaJP,
            "da-DK" => Region::DaDK,
            "zz-ZZ" => Region::ZzZZ,
            "en-US" => Region::EnUS,
            "nl-NL" => Region::NlNL,
            "de-CH" => Region::DeCH,
            "it-IT" => Region::ItIT,
            "es-ES" => Region::EsES,
            "es-ES_tradnl" => Region::EsTradnl,
            "sv-SE" => Region::SvSE,
            "nb-NO" => Region::NbNO,
            _ => return None,
        })
    }

    /// The region `CurrReport.FormatRegion := name` names: the regions of [`Region::parse`], and `de-de`, which the platform takes as
    /// de-DE and reads back as typed (reports/runtime/BdRegionTests.al:890 BdSetter_Lower).
    pub fn parse_setter(name: &str) -> Option<Region> {
        match name {
            "de-de" => Some(Region::DeDE),
            _ => Region::parse(name),
        }
    }

    /// Whether a text is pinned on this version. `g29` says the target is BC 29 or later, where BdRegionTests.al asserts every
    /// region. `new_shape` marks a text of en-GB or da-DK that no test pinned on BC 27 and 28 (a one-digit day, an hour, a Time,
    /// a negative amount, a da-DK column or long date); nl-NL, es-ES, sv-SE and nb-NO are pinned on BC 29 only.
    fn pinned(self, new_shape: bool, g29: bool) -> bool {
        match self {
            Region::NlNL | Region::EsES | Region::SvSE | Region::NbNO => g29,
            Region::EnGB | Region::DaDK => g29 || !new_shape,
            _ => true,
        }
    }

    /// The group separator and the decimal separator of an amount. fr-FR groups with U+00A0 on BC 27, 28 and 29
    /// (reports/PnH4CurrReport.al:247,311, reports/runtime/YReportPropsTests.al:74; corpus bump 62, where
    /// the corpus CI's newer Linux image stopped giving U+202F on BC 27 and 28). de-CH groups with U+2019 (BdRegionTests.al:311),
    /// sv-SE and nb-NO with U+00A0 (:371, :398).
    pub(super) fn separators(self) -> (char, char) {
        match self {
            Region::DeDE | Region::DaDK | Region::NlNL | Region::ItIT | Region::EsES | Region::EsTradnl => ('.', ','),
            Region::FrFR | Region::SvSE | Region::NbNO => ('\u{A0}', ','),
            Region::EnGB | Region::JaJP | Region::ZzZZ | Region::EnUS => (',', '.'),
            Region::DeCH => ('\u{2019}', '.'),
        }
    }

    /// Whether the amount `v` (negative or not) is pinned.
    pub(super) fn amount_pinned(self, negative: bool, g29: bool) -> bool {
        self.pinned(negative, g29)
    }

    /// Ja-JP and es-ES write the hour of a Time without a leading zero: ` 9:05:07` in `Format`, `9:05:07` in a column
    /// (BdRegionTests.al:199, :344); every other region writes two digits.
    fn hour_unpadded(self) -> bool {
        matches!(self, Region::JaJP | Region::EsES | Region::EsTradnl)
    }

    /// A date as `Format` (two-digit year) or a dataset column (four-digit year) writes it, `None` when the shape is not pinned.
    /// Day and month are padded in `Format` in every region and in a column except nl-NL (`31-1-2025`, `5-3-2024`: BdRegionTests.al:283).
    pub(super) fn date(self, y: i32, m: u32, d: u32, column: bool, g29: bool) -> Option<String> {
        // A one-digit day was pinned before BdRegionTests.al in `Format` of de-DE, fr-FR and da-DK (PnH4CurrReport.al:241-249) and in
        // a column of de-DE and zz-ZZ; a da-DK column is new.
        let legacy_day = if column {
            matches!(self, Region::DeDE | Region::ZzZZ)
        } else {
            matches!(self, Region::DeDE | Region::FrFR | Region::DaDK)
        };
        let new_shape = (d < 10 && !legacy_day) || (column && self == Region::DaDK);
        if !(1000..=9999).contains(&y) || !self.pinned(new_shape, g29) {
            return None;
        }
        let year = if column {
            format!("{y:04}")
        } else {
            format!("{:02}", y.rem_euclid(100))
        };
        let (dd, mm) = if column && self == Region::NlNL {
            (d.to_string(), m.to_string())
        } else {
            (format!("{d:02}"), format!("{m:02}"))
        };
        Some(match self {
            Region::DeDE | Region::DeCH | Region::NbNO => format!("{dd}.{mm}.{year}"),
            Region::EnGB | Region::FrFR | Region::ItIT | Region::EsES | Region::EsTradnl => {
                format!("{dd}/{mm}/{year}")
            }
            Region::DaDK | Region::NlNL => format!("{dd}-{mm}-{year}"),
            Region::JaJP => format!("{year}/{mm}/{dd}"),
            Region::SvSE => format!("{year}-{mm}-{dd}"),
            // The column of zz-ZZ is `MM/dd/yyyy` on BC 27, 28 and 29 (YReportPropsTests.al:128 `01/31/2025`, BdRegionTests.al:213,220
            // `03/05/2024`; corpus bump 43); its `Format` is en-US's, which the caller writes.
            Region::ZzZZ if column => format!("{mm}/{dd}/{year}"),
            Region::ZzZZ | Region::EnUS => return None,
        })
    }

    /// `Format(Date, 0, 4)`: the month name in the session's language (en-US), the order of the region
    /// (YReportPropsTests.al:40-96: `31. January 2025`, `31 January 2025`, `2025年1月31日`; BdRegionTests.al:131-398 with the 5th:
    /// `5. March 2024`, en-GB `05 March 2024`, `5 March 2024`, `5 de March de 2024`, `den 5 March 2024`).
    pub(super) fn long_date(self, y: i32, m: u32, d: u32, month: &str, g29: bool) -> Option<String> {
        if !self.pinned(d < 10 || self == Region::DaDK, g29) {
            return None;
        }
        Some(match self {
            Region::DeDE | Region::DaDK | Region::DeCH | Region::NbNO => format!("{d}. {month} {y}"),
            Region::EnGB => format!("{d:02} {month} {y}"),
            Region::FrFR | Region::NlNL | Region::ItIT => format!("{d} {month} {y}"),
            Region::JaJP => format!("{y}\u{5E74}{m}\u{6708}{d}\u{65E5}"),
            Region::EsES | Region::EsTradnl => format!("{d} de {month} de {y}"),
            Region::SvSE => format!("den {d} {month} {y}"),
            Region::ZzZZ | Region::EnUS => return None,
        })
    }

    /// The 12-hour clock of a column in en-US (`2:30:15 PM`, `9:05:07 AM`: BdRegionTests.al:267).
    fn clock_12(ms: i64) -> String {
        let (h, mi, s) = (ms / 3_600_000, ms / 60_000 % 60, ms / 1000 % 60);
        format!(
            "{}:{mi:02}:{s:02} {}",
            if h % 12 == 0 { 12 } else { h % 12 },
            if h < 12 { "AM" } else { "PM" }
        )
    }

    /// The 24-hour clock of `Format(Time)` (`column` false, an hour below 10 is `09` or, in ja-JP and es-ES, ` 9`) or of a dataset
    /// column (`9` in ja-JP and es-ES). `ms` is the time of day in milliseconds; a fraction is not measured.
    /// `new_shape`: the caller's text is one no test pinned on BC 27 and 28 for en-GB and da-DK (see [`Region::pinned`]).
    pub(super) fn clock(self, ms: i64, column: bool, new_shape: bool, g29: bool) -> Option<String> {
        if ms % 1000 != 0 || !self.pinned(new_shape, g29) {
            return None;
        }
        let (h, mi, s) = (ms / 3_600_000, ms / 60_000 % 60, ms / 1000 % 60);
        let hour = match (self.hour_unpadded(), column) {
            (false, _) => format!("{h:02}"),
            (true, false) => format!("{h:>2}"),
            (true, true) => h.to_string(),
        };
        Some(format!("{hour}:{mi:02}:{s:02}"))
    }

    /// `Format(DateTime)`: the short date, a space and the 24-hour time without seconds, two digits for the hour in every region
    /// (BdRegionTests.al `09:05`). zz-ZZ keeps en-US's text, which the caller writes.
    pub(super) fn date_time_text(self, ms_since_epoch_start: i64, g29: bool) -> Option<String> {
        let of_day = ms_since_epoch_start.rem_euclid(86_400_000);
        let (h, mi) = (of_day / 3_600_000, of_day / 60_000 % 60);
        let (y, m, d) = civil_from_day(ms_since_epoch_start.div_euclid(86_400_000) as i32 + 1);
        if !self.pinned(h < 10 || self == Region::DaDK, g29) {
            return None;
        }
        Some(format!("{} {h:02}:{mi:02}", self.date(y, m, d, false, g29)?))
    }
}

/// The text of a dataset column under the report's `FormatRegion` (reports/runtime/YReportPropsTests.al:34-148 and
/// reports/runtime/BdRegionTests.al), `None` when the shape is not pinned: a Date, a DateTime with seconds and a Time
/// in the region's order, and a Decimal with the region's decimal separator and no grouping (`1234567,891`, `-1234,5`).
/// `g29`: the target is BC 29 or later.
pub fn region_column(r: Region, v: &Value, g29: bool) -> Option<String> {
    if r == Region::EnUS {
        // `1/31/2025`, `3/5/2024 9:05:07 AM`, `2:30:15 PM` (BdRegionTests.al:267, every version).
        return match v {
            Value::Date(day, false) if *day != 0 => {
                let (y, m, d) = civil_from_day(*day);
                Some(format!("{m}/{d}/{y:04}"))
            }
            Value::DateTime(t) if *t != 0 && (t - 1) % 1000 == 0 => {
                let ms = t - 1;
                let (y, m, d) = civil_from_day(ms.div_euclid(86_400_000) as i32 + 1);
                Some(format!(
                    "{m}/{d}/{y:04} {}",
                    Region::clock_12(ms.rem_euclid(86_400_000))
                ))
            }
            Value::Time(t) if *t != 0 && (t - 1) % 1000 == 0 => Some(Region::clock_12(t - 1)),
            Value::Decimal(d) => Some(d.to_string()),
            _ => None,
        };
    }
    match v {
        Value::Date(day, false) if *day != 0 => {
            let (y, m, d) = civil_from_day(*day);
            r.date(y, m, d, true, g29)
        }
        Value::DateTime(t) if *t != 0 => {
            let ms = t - 1;
            let (y, m, d) = civil_from_day(ms.div_euclid(86_400_000) as i32 + 1);
            let of_day = ms.rem_euclid(86_400_000);
            let clock = r.clock(of_day, true, of_day < 36_000_000 || r == Region::DaDK, g29)?;
            Some(format!("{} {clock}", r.date(y, m, d, true, g29)?))
        }
        // A Time column: `14:30:15`, `09:05:07` (BdRegionTests.al:131), ja-JP and es-ES `9:05:07`.
        Value::Time(t) if *t != 0 => r.clock(t - 1, true, true, g29),
        Value::Decimal(d) if r.amount_pinned(d.is_sign_negative(), g29) => {
            // The decimal separator does not depend on the version.
            let (_, decimal) = r.separators();
            Some(d.to_string().replace('.', &decimal.to_string()))
        }
        _ => None,
    }
}
