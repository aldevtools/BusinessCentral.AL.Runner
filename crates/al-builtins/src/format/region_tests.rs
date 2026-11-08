//! The region tables of reports/runtime/BdRegionTests.al: 31 January 2025 14:30:15 and 5 March 2024 09:05:07,
//! 1234567.891 and -1234.5, as `Format` writes them (TextsCol) and as a dataset column writes them.

use super::*;
use rust_decimal::Decimal;
use std::str::FromStr;

struct R {
    region: &'static str,
    g29: bool,
    language: i32,
}

impl FormatCtx for R {
    fn enum_name(&self, _: u32, o: i32) -> String {
        o.to_string()
    }
    fn region(&self) -> Option<String> {
        Some(self.region.into())
    }
    fn language(&self) -> i32 {
        self.language
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
    texts_in(region, g29, 1033)
}

/// The same after `CurrReport.Language := language`.
fn texts_in(region: &'static str, g29: bool, language: i32) -> Result<String, FormatFail> {
    let r = R { region, g29, language };
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

fn columns(region: &'static str) -> Option<String> {
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
        out.push(region_column(r, v)?);
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
fn the_regions_first_measured_on_bc_29_are_the_same_on_27_and_28() {
    // BdRegionTests.al: en-GB :147, da-DK :239, nl-NL :283, es-ES :344, sv-SE :371, nb-NO :398 under `#if BC29PLUS`, and the same
    // values on BC 27 and 28 in Y89_Old_Property_* and Y89_Old_Setter_* (:1506-1710, `#if not BC29PLUS`).
    for g29 in [false, true] {
        assert_eq!(
            texts("en-GB", g29).unwrap(),
            "1,234,567.891#-1,234.5#31/01/25#05/03/24#05 March 2024#31/01/25 14:30#05/03/24 09:05#14:30:15#09:05:07#-1234567"
        );
        assert_eq!(
            texts("da-DK", g29).unwrap(),
            "1.234.567,891#-1.234,5#31-01-25#05-03-24#5. March 2024#31-01-25 14:30#05-03-24 09:05#14:30:15#09:05:07#-1234567"
        );
        assert_eq!(
            texts("nl-NL", g29).unwrap(),
            "1.234.567,891#-1.234,5#31-01-25#05-03-24#5 March 2024#31-01-25 14:30#05-03-24 09:05#14:30:15#09:05:07#-1234567"
        );
        assert_eq!(
            texts("es-ES", g29).unwrap(),
            "1.234.567,891#-1.234,5#31/01/25#05/03/24#5 de March de 2024#31/01/25 14:30#05/03/24 09:05#14:30:15# 9:05:07#-1234567"
        );
        assert_eq!(
            texts("sv-SE", g29).unwrap(),
            "1\u{A0}234\u{A0}567,891#-1\u{A0}234,5#25-01-31#24-03-05#den 5 March 2024#25-01-31 14:30#24-03-05 09:05#14:30:15#09:05:07#-1234567"
        );
        assert_eq!(
            texts("nb-NO", g29).unwrap(),
            "1\u{A0}234\u{A0}567,891#-1\u{A0}234,5#31.01.25#05.03.24#5. March 2024#31.01.25 14:30#05.03.24 09:05#14:30:15#09:05:07#-1234567"
        );
    }
}

#[test]
fn a_region_after_a_language_writes_the_month_of_the_language() {
    // BdRegionTests.al:1712-1912 Y89_Lang_* and :1346-1480 BdSetterLanguage_* (every version): the long date takes the month name
    // of the language in the order of the region; de-AT groups with U+00A0.
    let want = [
        ("da-DK", 1030, "5. marts 2024"),
        ("nl-NL", 1043, "5 maart 2024"),
        ("es-ES", 1034, "5 de marzo de 2024"),
        ("es-ES_tradnl", 1034, "5 de marzo de 2024"),
        ("sv-SE", 1053, "den 5 mars 2024"),
        ("nb-NO", 1044, "5. mars 2024"),
        ("en-GB", 2057, "05 March 2024"),
        ("de-AT", 3079, "5. M\u{E4}rz 2024"),
        ("de-DE", 1031, "5. M\u{E4}rz 2024"),
        ("fr-FR", 1030, "5 marts 2024"),
    ];
    for g29 in [false, true] {
        for (region, language, long) in want {
            let t = texts_in(region, g29, language).unwrap();
            assert_eq!(t.split('#').nth(4), Some(long), "{region} after {language}");
        }
        assert_eq!(
            texts_in("de-AT", g29, 3079).unwrap(),
            "1\u{A0}234\u{A0}567,891#-1\u{A0}234,5#31.01.25#05.03.24#5. M\u{E4}rz 2024#31.01.25 14:30#05.03.24 09:05#14:30:15#09:05:07#-1234567"
        );
    }
    // A month whose name in that language no test pins (January in de-AT may be `J\u{E4}nner`) is refused.
    let r = R {
        region: "de-AT",
        g29: true,
        language: 3079,
    };
    assert!(format_value(&date(2025, 1, 31), 0, 4, &r).is_err());
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
        // Y89_Lang_DeAT_3079_D (BdRegionTests.al:1890).
        (
            "de-AT",
            "1234567,891;-1234,5;31.01.2025;05.03.2024;31.01.2025 14:30:15;05.03.2024 09:05:07;14:30:15;09:05:07",
        ),
    ];
    for (r, w) in want {
        assert_eq!(columns(r).as_deref(), Some(w), "{r}");
    }
}
