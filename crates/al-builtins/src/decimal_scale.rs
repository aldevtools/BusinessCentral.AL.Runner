//! The decimals a Decimal shows where its scale is seen, the report XML dataset (al-vm `reports.rs`). The one owner of
//! that rule: the dataset writes what [`shown`] gives (al-runner issues 7440 and 7958). The value keeps its zeros (`Round` in
//! `mathfns.rs` keeps those of its precision), since a calculation sees those of one too.
//!
//! A Decimal shows the decimals it holds, zeros included, except zero, one and minus one, which show none. Pinned for one and
//! minus one in every column source shape (reports/dataset/DatasetDecimalPlacesValueOrigins.al:241-247, :364, :397, :425-570)
//! and for the scale of a calculation (reports/layouts-saveas/TestReportSaveAsColumnTypesContracts.al:309 `18.5185170`);
//! zero and the zeros of any other value by tests/al-language/reports/dataset/DatasetDecimalRoundScaleSources.al:101-615 and
//! DatasetDecimalRoundScale.al:29-183.
//!
//! The decimals a Decimal shows where its scale is seen (zero, one and minus one none); the report
//! dataset writes it and Round marks by it.

use rust_decimal::Decimal;

/// `d` with the decimals it shows: zero, one and minus one without decimals, every other value as it is.
pub fn shown(d: Decimal) -> Decimal {
    if d.is_zero() || d.abs() == Decimal::ONE {
        d.normalize()
    } else {
        d
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn d(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    #[test]
    fn zero_one_and_minus_one_show_no_decimals_every_other_value_its_own() {
        for (v, want) in [
            ("1.00000", "1"),
            ("-1.00", "-1"),
            ("0.00", "0"),
            ("3.00", "3.00"),
            ("2.50", "2.50"),
            ("2.5000", "2.5000"),
            ("1.23457", "1.23457"),
        ] {
            assert_eq!(shown(d(v)).to_string(), want, "{v}");
        }
    }
}
