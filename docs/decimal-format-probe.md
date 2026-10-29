# Exact Decimal format: real BC evidence

Measured 2026-10-29 on the existing `bcserver2` CH container, Business Central
28.5.54151.54753. This is a local service-tier measurement, **not an upstream
corpus/oracle pin**. No corpus, oracle, application object or baseline was changed.

## Synthetic probe

App `a9b15fd4-4f3d-4ed8-bb2f-69ed2248fb6d`, publisher `Local Runtime Evidence`,
name `Synthetic Decimal Format Probe`, version `1.0.0.1`; synthetic codeunit
50100 `Synthetic Format Probe`. No SwissSalary dependency or objects.
The installed app used the existing AL compiler and cached platform symbols.

For each text input in the table below the probe performed:

```al
GlobalLanguage(LanguageId);
Evaluate(Value, Input, 9);
Result += Input + '=[' + Format(Value, 0, '<Sign><Integer><Decimals,3><Comma,.>') + '];';
```

`Value` was a Decimal variable. `DecimalEnglish` used language 1033;
`DecimalSwissGerman` used language 2055. Each ended with
`Error('Language=%1;%2', GlobalLanguage(), Result)`, so the SOAP fault contained
the exact measured outputs and confirmed the requested language.
The normal codeunit was exposed temporarily as `SyntheticDecimalProbe`, invoked
through the container-local SOAP endpoint using the existing user's credential.
No new user was created. The initial test-client attempt timed out; its output
was **not** used as evidence.
Both synthetic app versions and the SOAP service were subsequently removed;
verification found zero remaining synthetic app versions or services.

## Identical outputs in both languages

| Decimal input | FORMAT result |
|---|---|
| `0`, `-0` | `0.00` |
| `1234`, `-1234` | `1234.00`, `-1234.00` |
| `0.5`, `-0.5` | `0.50`, `-0.50` |
| `1234.5`, `-1234.5` | `1234.50`, `-1234.50` |
| `0.005`, `-0.005` | `0***`, `-0***` |
| `1.234`, `-1.234` | `1***`, `-1***` |
| `1.235`, `-1.235` | `1***`, `-1***` |
| `1.995`, `-1.995` | `1***`, `-1***` |
| `99999`, `-99999` | `99999.00`, `-99999.00` |
| `1.2300` | `1.23` |
| `999999999999999.99` | `999999999999999.99` |

Width three includes the decimal point: it is **not** three fractional places
and does **not** round excess fractions. This agrees with the existing corpus
`types/lt-format-comma-more/FormatCommaMore.al:76-91` (`0.50` and `1,234,567***`).
The sign and suffix comma element separately agree with
`types/lt-format-comma-format-string/FormatCommaFormatString.al:37-113`.
The exact combination is absent from pinned corpus
`84ebf506049a34bc92f72c6cf61e1d0a3bfc3ed8`.

The regression `exact_decimal_comma_width_three_matches_bc28_in_english_and_swiss_german`
checks all 40 measured input/language pairs. Implementation only adds that exact
Decimal template to the comma dispatcher; all digits, sign, width overflow and
padding remain owned by the existing numeric helpers. Other templates and types
are not broadened. Other BC versions, explicit report format regions and
alternate code spellings remain unmeasured; the upstream probe is tracked in
`probes-needed.md`.
