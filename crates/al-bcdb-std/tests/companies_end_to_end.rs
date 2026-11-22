//! A backup's selected companies reach AL code, and a run that selects none keeps the Company-table guard
//! (ADR-046 decision 4a). The company list is a setting of the process (`al_testrun::host::set_companies`), so this
//! binary has one test of its own: no other test can read the list while it changes.

mod common;

use al_testrun::{CaseId, Outcome};
use common::{BASE, run_with_companies};

#[test]
fn selected_backup_companies_are_available_end_to_end_and_single_company_keeps_its_guard() {
    let base = format!(
        "{BASE}
        table 2000000006 Company {{ DataPerCompany = false;
          fields {{ field(1; Name; Text[30]) {{ }} field(2; \"Evaluation Company\"; Boolean) {{ }}
            field(3; \"Display Name\"; Text[250]) {{ }} field(8000; Id; Guid) {{ }} }}
          keys {{ key(PK; Name) {{ }} }} }}"
    );
    let main = r#"codeunit 50001 T { Subtype = Test;
      [Test] procedure Companies() var C: Record Company; begin
        if not C.FindSet() then Error('no companies');
        if C.Count() <> 2 then Error('company count %1', C.Count());
        C.Get('TP'); C.Get('ProbeCo');
      end;
      [Test] procedure OtherCompany() var R: Record exttest; begin
        if R.Count() <> 3 then Error('session rows %1', R.Count());
        R.ChangeCompany('ProbeCo');
        if not R.IsEmpty() then Error('other rows %1', R.Count());
        R.id := 99; R.own := 'other'; R.Insert(); R.Get(99);
        R.own := 'changed'; R.Modify(); R.Get(99);
        if R.own <> 'changed' then Error('modify');
        R.Delete();
        R.ChangeCompany('TP'); if R.Count() <> 3 then Error('session changed');
      end;
    }"#;
    for file in ["typeprobe.bak", "typeprobe.bacpac"] {
        // The CLI declares the provider's selection as the run's companies (`resolved_companies` in
        // alrunner-cli/src/run/test_data.rs); the host declaration decides what the run knows.
        al_testrun::host::set_companies(al_host::Companies::List(vec!["TP".into(), "ProbeCo".into()]));
        let (rt, _) = run_with_companies(file, &base, main, &["ProbeCo".into()]);
        for name in ["Companies", "OtherCompany"] {
            assert_eq!(
                rt.first_outcome(&CaseId {
                    codeunit: 50001,
                    name: name.into()
                }),
                Outcome::Pass,
                "{file} {name}"
            );
        }
        al_testrun::host::set_companies(al_host::Companies::One);
        let (rt, _) = run_with_companies(file, &base, main, &[]);
        let scan = rt.first_outcome(&CaseId {
            codeunit: 50001,
            name: "Companies".into(),
        });
        assert!(
            matches!(scan, Outcome::NotRun(ref why) if why.contains("Company table")),
            "{file}: {scan:?}"
        );
        assert_eq!(
            rt.first_outcome(&CaseId {
                codeunit: 50001,
                name: "OtherCompany".into()
            }),
            Outcome::Fail("The company \"ProbeCo\" does not exist.".into()),
            "{file}"
        );
    }
}
