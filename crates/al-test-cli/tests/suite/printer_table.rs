//! The Printer system table (2000000039) has no rows (metadata/platform-tables/Y21EnvColumns.al:30-40 Y21_Printer_NoRows;
//! al-runner issue 6916). RecordRef.Open stays refused in a Cloud app (Y21EnvColumns.al:42-50 Y21_Printer_RecordRefRefused).
//!
//! Needs the `corpus` submodule, including its `.alpackages` (`System.app`).

use super::y48_platform_probes::outcomes;
use al_test_cli::report::Outcome;

#[test]
fn printer_is_an_empty_table() {
    let got = outcomes(
        r#"codeunit 50000 T { Subtype = Test;
  [Test] procedure Empty() var P: Record Printer; begin
    if P.Count() <> 0 then Error('count %1', P.Count());
    if P.FindFirst() then Error('find first');
    if not P.IsEmpty() then Error('not empty');
    if P.FindSet() then Error('find set');
    if P.Get('x') then Error('get'); end;
  [Test] procedure ReportersFilter() var P: Record Printer; begin
    P.Reset(); P.SetFilter(Name, '@%1', 'x');
    if P.FindFirst() then Error('found'); if P.Count() <> 0 then Error('count'); end; }"#,
    );
    for (name, o) in &got {
        assert!(matches!(o, Outcome::Pass), "{name}: {o:?}");
    }
    assert_eq!(got.len(), 2);
}
