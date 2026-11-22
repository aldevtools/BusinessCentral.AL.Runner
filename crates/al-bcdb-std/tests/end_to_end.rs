//! From a backup file to AL code: our own small AL tables over bcdb's synthetic `typeprobe`
//! database (no Microsoft data). The rows reach AL as the database holds them, with names taken
//! from the compiled tables only.

mod common;

use al_host::testdata::TestData;
use al_testrun::runtime::CorpusRuntime;
use al_testrun::{CaseId, Outcome};
use common::{BASE, run_with_companies};

const MAIN: &str = r#"
codeunit 50001 "T" { Subtype = Test;
  [Test] procedure Company() begin
    if CompanyName() <> 'TP' then Error(CompanyName());
  end;
  [Test] procedure MergedExtensionFields() var E: Record "exttest"; begin
    if E.Count() <> 3 then Error('count %1', E.Count());
    E.Get(1);
    if (E.own <> 'base-one') or (E.extra <> 'ext-one') or (E.num <> 11) then Error('row 1: %1 %2 %3', E.own, E.extra, E.num);
    E.Get(3);
    if (E.extra <> '') or (E.num <> 0) then Error('row 3 has no companion row: %1 %2', E.extra, E.num);
  end;
  [Test] procedure ScalarTypes() var P: Record "probe"; begin
    if P.Count() <> 8 then Error('count %1', P.Count());
    P.Get(2);
    if (P.c_tinyint <> 255) or (P.c_smallint <> 32767) or (P.c_int <> 2147483647) then Error('ints');
    if P.c_bigint <> 9223372036854775807L then Error('bigint %1', P.c_bigint);
    if not P.c_bit then Error('bit');
    if P.c_nvarchar <> 'Hello World' then Error('text %1', P.c_nvarchar);
    if P.c_date <> DMY2Date(31, 12, 9999) then Error('date %1', P.c_date);
    P.Get(3);
    if P.c_datetime <> 0DT then Error('1753-01-01 is the blank DateTime, got %1', P.c_datetime);
    if Format(P.c_guid) <> '{12345678-9ABC-DEF0-1234-56789ABCDEF0}' then Error('guid %1', Format(P.c_guid));
    if P.c_nvarchar <> 'Ærøskøbing über café' then Error('unicode %1', P.c_nvarchar);
    if P.c_dec18_2 <> -9999999999999999.99 then Error('decimal %1', P.c_dec18_2);
    P.Get(4);
    if P.c_dec18_2 <> 1 then Error('dec %1', P.c_dec18_2);
    if P.c_datetime <> CreateDateTime(DMY2Date(31, 8, 2026), 123456.79T) then Error('datetime %1', Format(P.c_datetime, 0, 9));
    if P.c_date <> DMY2Date(31, 8, 2026) then Error('date %1', P.c_date);
    P.Get(6);
    if (P.c_int <> 0) or P.c_bit or (P.c_nvarchar <> '') or (P.c_date <> 0D) or not IsNullGuid(P.c_guid) then Error('NULLs are blanks');
  end;
  [Test] procedure ATableTheDatabaseHoldsNoRowsFor() var L: Record "probe_lob"; begin
    if not L.IsEmpty() then Error('rows');
  end;
}
"#;

fn run(path: &str) -> (CorpusRuntime, TestData) {
    run_with(path, BASE, MAIN)
}

fn run_with(path: &str, base: &str, main: &str) -> (CorpusRuntime, TestData) {
    run_with_companies(path, base, main, &[])
}

#[test]
fn a_backup_file_gives_al_code_its_rows() {
    for f in ["typeprobe.bak", "typeprobe.bacpac"] {
        let (rt, data) = run(f);
        for t in [
            "Company",
            "MergedExtensionFields",
            "ScalarTypes",
            "ATableTheDatabaseHoldsNoRowsFor",
        ] {
            let o = rt.first_outcome(&CaseId {
                codeunit: 50001,
                name: t.into(),
            });
            assert_eq!(o, Outcome::Pass, "{f} {t}");
        }
        let s = data.stats.summary();
        assert_eq!(s.tables, 2, "{f}: exttest and probe; probe_lob is empty here");
        assert_eq!(s.rows, 11);
        // 11 of probe's 25 columns have no field in the compiled table
        assert_eq!(s.dropped_no_such_field, 11, "{f}");
        assert!(s.unsupported.is_empty() && s.reader.is_empty(), "{f}: {s:?}");
    }
}

#[test]
fn a_value_the_runtime_cannot_hold_refuses_the_table_and_the_summary_says_so() {
    // probe.c_dec38_20 holds 99999999999999999.99999999999999999999, which no Decimal can.
    let base = BASE.replace(
        "field(8; c_dec18_2; Decimal) { }",
        "field(8; c_dec18_2; Decimal) { } field(30; c_dec38_20; Decimal) { }",
    );
    let main = r#"codeunit 50001 "T" { Subtype = Test;
      [Test] procedure Refused() var P: Record "probe"; begin
        if not P.IsEmpty() then Error('a refused table reads as empty'); end; }"#;
    let (rt, data) = run_with("typeprobe.bak", &base, main);
    let o = rt.first_outcome(&CaseId {
        codeunit: 50001,
        name: "Refused".into(),
    });
    assert_eq!(o, Outcome::Pass);
    let s = data.stats.summary();
    assert_eq!(s.unsupported.len(), 1, "{s:?}");
    assert!(
        s.unsupported[0].contains("c_dec38_20") && s.unsupported[0].contains("does not fit a Decimal"),
        "{s:?}"
    );
    assert!(s.line("typeprobe.bak", "TP").contains("1 refused"));
}
