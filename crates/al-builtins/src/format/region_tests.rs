//! The region tables of report/lt-bd-format-region/BdRegionTests.al: 31 January 2025 14:30:15 and 5 March 2024 09:05:07,
//! 1234567.891 and -1234.5, as `Format` writes them (TextsCol) and as a dataset column writes them.

use super::*;
use rust_decimal::Decimal;
use std::str::FromStr;

struct R {
    region: &'static str,
    g29: bool,
}

impl FormatCtx for R {
    fn enum_name(&self, _: u32, o: i32) -> String {
        o.to_string()
    }
    fn region(&self) -> Option<String> {
        Some(self.region.into())
    }
    fn format_gate(&self, gate: al_version::Gate) -> bool {
        self.g29 && gate == al_version::Gate::ProbedValuesDifferFrom29
    }
}

fn dec(s: &str) -> Value {
    Value::Decimal(Decimal::from_str(s).unwrap())
}

fn date(y: i32, m: u32, d: u32) -> Value {
    Value::Date(day_from_civil(y, m, d), false)
}

fn stamp(y: i32, m: u32, d: u32, h: i64, mi: i64, s: i64) -> Value {
    Value::DateTime((i64::from(day_from_civil(y, m, d)) - 1) * 86_400_000 + (h * 3600 + mi * 60 + s) * 1000 + 1)
}

fn time(h: i64, mi: i64, s: i64) -> Value {
    Value::Time((h * 3600 + mi * 60 + s) * 1000 + 1)
}

/// The ten values of `Fmt()` of BdRegionReports.al / BdRegionFixtures.al, joined with `#`.
fn texts(region: &'static str, g29: bool) -> Result<String, FormatFail> {
    let r = R { region, g29 };
    let d2 = date(2024, 3, 5);
    let values = [
        (dec("1234567.891"), 0),
        (dec("-1234.5"), 0),
        (date(2025, 1, 31), 0),
        (d2.clone(), 0),
        (d2, 4),
        (stamp(2025, 1, 31, 14, 30, 15), 0),
        (stamp(2024, 3, 5, 9, 5, 7), 0),
        (time(14, 30, 15), 0),
        (time(9, 5, 7), 0),
        (Value::Integer(-1234567), 0),
    ];
    let mut out = Vec::new();
    for (v, n) in &values {
        out.push(format_value(v, 0, *n, &r)?);
    }
    Ok(out.join("#"))
}

fn columns(region: &'static str, g29: bool) -> Option<String> {
    let r = Region::parse(region)?;
    let values = [
        dec("1234567.891"),
        dec("-1234.5"),
        date(2025, 1, 31),
        date(2024, 3, 5),
        stamp(2025, 1, 31, 14, 30, 15),
        stamp(2024, 3, 5, 9, 5, 7),
        time(14, 30, 15),
        time(9, 5, 7),
    ];
    let mut out = Vec::new();
    for v in &values {
        out.push(region_column(r, v, g29)?);
    }
    Some(out.join(";"))
}

#[test]
fn the_regions_measured_on_every_version() {
    // BdRegionTests.al:131-132 de-DE, :174-175 and :181-182 fr-FR (U+00A0), :199-200 ja-JP, :311-312 de-CH (U+2019), :328-329 it-IT.
    for g29 in [false, true] {
        assert_eq!(
            texts("de-DE", g29).unwrap(),
            "1.234.567,891#-1.234,5#31.01.25#05.03.24#5. March 2024#31.01.25 14:30#05.03.24 09:05#14:30:15#09:05:07#-1234567"
        );
        assert_eq!(
            texts("fr-FR", g29).unwrap(),
            "1\u{A0}234\u{A0}567,891#-1\u{A0}234,5#31/01/25#05/03/24#5 March 2024#31/01/25 14:30#05/03/24 09:05#14:30:15#09:05:07#-1234567"
        );
        assert_eq!(
            texts("ja-JP", g29).unwrap(),
            "1,234,567.891#-1,234.5#25/01/31#24/03/05#2024年3月5日#25/01/31 14:30#24/03/05 09:05#14:30:15# 9:05:07#-1234567"
        );
        assert_eq!(
            texts("de-CH", g29).unwrap(),
            "1\u{2019}234\u{2019}567.891#-1\u{2019}234.5#31.01.25#05.03.24#5. March 2024#31.01.25 14:30#05.03.24 09:05#14:30:15#09:05:07#-1234567"
        );
        assert_eq!(
            texts("it-IT", g29).unwrap(),
            "1.234.567,891#-1.234,5#31/01/25#05/03/24#5 March 2024#31/01/25 14:30#05/03/24 09:05#14:30:15#09:05:07#-1234567"
        );
        assert_eq!(
            texts("es-ES_tradnl", g29).unwrap(),
            "1.234.567,891#-1.234,5#31/01/25#05/03/24#5 de March de 2024#31/01/25 14:30#05/03/24 09:05#14:30:15# 9:05:07#-1234567"
        );
    }
}

#[test]
fn the_regions_measured_on_bc_29_only_are_refused_below() {
    // BdRegionTests.al: en-GB :147, da-DK :239, nl-NL :283, es-ES :344, sv-SE :371, nb-NO :398 are asserted under `#if BC29PLUS`; the
    // other versions hold `Observer.Discard()`.
    assert_eq!(
        texts("en-GB", true).unwrap(),
        "1,234,567.891#-1,234.5#31/01/25#05/03/24#05 March 2024#31/01/25 14:30#05/03/24 09:05#14:30:15#09:05:07#-1234567"
    );
    assert_eq!(
        texts("da-DK", true).unwrap(),
        "1.234.567,891#-1.234,5#31-01-25#05-03-24#5. March 2024#31-01-25 14:30#05-03-24 09:05#14:30:15#09:05:07#-1234567"
    );
    assert_eq!(
        texts("nl-NL", true).unwrap(),
        "1.234.567,891#-1.234,5#31-01-25#05-03-24#5 March 2024#31-01-25 14:30#05-03-24 09:05#14:30:15#09:05:07#-1234567"
    );
    assert_eq!(
        texts("es-ES", true).unwrap(),
        "1.234.567,891#-1.234,5#31/01/25#05/03/24#5 de March de 2024#31/01/25 14:30#05/03/24 09:05#14:30:15# 9:05:07#-1234567"
    );
    assert_eq!(
        texts("sv-SE", true).unwrap(),
        "1\u{A0}234\u{A0}567,891#-1\u{A0}234,5#25-01-31#24-03-05#den 5 March 2024#25-01-31 14:30#24-03-05 09:05#14:30:15#09:05:07#-1234567"
    );
    assert_eq!(
        texts("nb-NO", true).unwrap(),
        "1\u{A0}234\u{A0}567,891#-1\u{A0}234,5#31.01.25#05.03.24#5. March 2024#31.01.25 14:30#05.03.24 09:05#14:30:15#09:05:07#-1234567"
    );
    for r in ["en-GB", "da-DK", "nl-NL", "es-ES", "sv-SE", "nb-NO"] {
        assert!(texts(r, false).is_err(), "{r} before the gate is not pinned");
    }
}

#[test]
fn dataset_columns_of_each_region() {
    // BdRegionTests.al columns lines: AmtCol;NegCol;D1Col;D2Col;DT1Col;DT2Col;T1Col;T2Col.
    let want = [
        (
            "de-DE",
            "1234567,891;-1234,5;31.01.2025;05.03.2024;31.01.2025 14:30:15;05.03.2024 09:05:07;14:30:15;09:05:07",
        ),
        (
            "en-GB",
            "1234567.891;-1234.5;31/01/2025;05/03/2024;31/01/2025 14:30:15;05/03/2024 09:05:07;14:30:15;09:05:07",
        ),
        (
            "fr-FR",
            "1234567,891;-1234,5;31/01/2025;05/03/2024;31/01/2025 14:30:15;05/03/2024 09:05:07;14:30:15;09:05:07",
        ),
        (
            "ja-JP",
            "1234567.891;-1234.5;2025/01/31;2024/03/05;2025/01/31 14:30:15;2024/03/05 9:05:07;14:30:15;9:05:07",
        ),
        (
            "zz-ZZ",
            "1234567.891;-1234.5;01/31/2025;03/05/2024;01/31/2025 14:30:15;03/05/2024 09:05:07;14:30:15;09:05:07",
        ),
        (
            "da-DK",
            "1234567,891;-1234,5;31-01-2025;05-03-2024;31-01-2025 14:30:15;05-03-2024 09:05:07;14:30:15;09:05:07",
        ),
        (
            "nl-NL",
            "1234567,891;-1234,5;31-1-2025;5-3-2024;31-1-2025 14:30:15;5-3-2024 09:05:07;14:30:15;09:05:07",
        ),
        (
            "de-CH",
            "1234567.891;-1234.5;31.01.2025;05.03.2024;31.01.2025 14:30:15;05.03.2024 09:05:07;14:30:15;09:05:07",
        ),
        (
            "it-IT",
            "1234567,891;-1234,5;31/01/2025;05/03/2024;31/01/2025 14:30:15;05/03/2024 09:05:07;14:30:15;09:05:07",
        ),
        (
            "es-ES",
            "1234567,891;-1234,5;31/01/2025;05/03/2024;31/01/2025 14:30:15;05/03/2024 9:05:07;14:30:15;9:05:07",
        ),
        (
            "es-ES_tradnl",
            "1234567,891;-1234,5;31/01/2025;05/03/2024;31/01/2025 14:30:15;05/03/2024 9:05:07;14:30:15;9:05:07",
        ),
        (
            "sv-SE",
            "1234567,891;-1234,5;2025-01-31;2024-03-05;2025-01-31 14:30:15;2024-03-05 09:05:07;14:30:15;09:05:07",
        ),
        (
            "nb-NO",
            "1234567,891;-1234,5;31.01.2025;05.03.2024;31.01.2025 14:30:15;05.03.2024 09:05:07;14:30:15;09:05:07",
        ),
    ];
    for (r, w) in want {
        assert_eq!(columns(r, true).as_deref(), Some(w), "{r}");
    }
    for r in ["de-DE", "fr-FR", "ja-JP", "zz-ZZ", "de-CH", "it-IT", "es-ES_tradnl"] {
        assert!(columns(r, false).is_some(), "{r} is pinned on every version");
    }
    for r in ["nl-NL", "es-ES", "sv-SE", "nb-NO", "da-DK"] {
        assert!(columns(r, false).is_none(), "{r} before the gate is not pinned");
    }
}
