//! Which format strings `Format(Rec.Field, Length, FormatString)` of a Decimal field with places formats by the field's rule
//! (types/lt-at-partner-gaps/R6901DecimalFieldFormat.al, al-runner issue 6901).

/// The format string holds only codes the probe measured on fields of every places: `<Integer>`, `<Integer,N>`, `<Integer Thousand>`,
/// `<Decimals>`, `<Decimals,N>`, `<Decimal>`, `<Decimal,N>`, `<Sign>`, `<Filler Character,0>` and `<Standard Format,0|1|9>`, with
/// literal text between them. Any other code (`<Comma,..>`, `<Thousands,..>`, a date code, `<Precision,..>`) is not covered.
pub(super) fn covered_field_template(template: &str) -> bool {
    let mut rest = template;
    let mut codes = 0;
    while let Some(open) = rest.find('<') {
        let after = &rest[open + 1..];
        let Some(close) = after.find('>') else {
            return false;
        };
        let code = &after[..close];
        let (name, arg) = match code.split_once(',') {
            Some((n, a)) => (n, Some(a)),
            None => (code, None),
        };
        let number = |a: &str| !a.is_empty() && a.len() <= 2 && a.bytes().all(|b| b.is_ascii_digit());
        let ok = match (name.to_ascii_lowercase().as_str(), arg) {
            ("integer", None | Some(_)) | ("decimals" | "decimal", None | Some(_)) if arg.is_none_or(number) => true,
            ("integer thousand", None) | ("sign", None) => true,
            ("filler character", Some("0")) => true,
            ("standard format", Some("0" | "1" | "9")) => true,
            _ => false,
        };
        if !ok {
            return false;
        }
        codes += 1;
        rest = &after[close + 1..];
    }
    codes > 0
}

#[cfg(test)]
mod tests {
    use super::covered_field_template as c;

    #[test]
    fn digit_codes_are_covered_and_the_rest_is_not() {
        assert!(c("<Integer,4><Filler character,0><Decimal,4><Filler character,0>"));
        assert!(c("<Sign><Integer Thousand><Decimals>"));
        assert!(c("<Integer>.<Decimals>"));
        assert!(c("<Standard Format,9>"));
        assert!(!c("<Standard Format,2>"));
        assert!(!c("<Integer><Comma,.><Decimals>"));
        assert!(!c("<Precision,2:4><Integer>"));
        assert!(!c("<Integer,x>"));
        assert!(!c("<Filler Character,#>"));
        assert!(!c("plain text"));
        assert!(!c("<Integer"));
    }
}
