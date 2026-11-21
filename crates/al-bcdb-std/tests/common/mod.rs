//! The setup the end-to-end binaries share: our own small AL tables over bcdb's synthetic `typeprobe`
//! database (no Microsoft data), and one run of them over a backup file.

use al_bcdb_std::BackupProvider;
use al_host::testdata::TestData;
use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun};
use std::path::PathBuf;
use std::sync::Arc;

const BASE_ID: &str = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";
const EXT_ID: &str = "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb";

pub const BASE: &str = r#"
table 50100 "exttest" { fields { field(1; id; Integer) { } field(2; own; Text[50]) { } } keys { key(PK; id) { } } }
table 50101 "exttest4" { fields { field(1; id; Integer) { } field(2; own; Text[50]) { } } keys { key(PK; id) { } } }
table 50102 "probe" { DataPerCompany = false; fields {
    field(1; id; Integer) { }
    field(2; c_tinyint; Byte) { }
    field(3; c_smallint; Integer) { }
    field(4; c_int; Integer) { }
    field(5; c_bigint; BigInteger) { }
    field(6; c_bit; Boolean) { }
    field(8; c_dec18_2; Decimal) { }
    field(9; c_datetime; DateTime) { }
    field(10; c_date; Date) { }
    field(11; c_time7; Time) { }
    field(12; c_guid; Guid) { }
    field(13; c_nvarchar; Text[100]) { }
    field(14; c_varchar; Text[50]) { }
    field(15; c_nchar; Text[10]) { }
  } keys { key(PK; id) { } } }
table 50103 "probe_lob" { fields { field(1; id; Integer) { } } keys { key(PK; id) { } } }
"#;

const EXT: &str = r#"
tableextension 50200 "exttest ext" extends "exttest" { fields { field(50200; extra; Text[20]) { } field(50201; num; Integer) { } } }
"#;

pub fn run_with_companies(path: &str, base: &str, main: &str, companies: &[String]) -> (CorpusRuntime, TestData) {
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../al-bcdb/tests/fixtures")
        .join(path);
    let provider = BackupProvider::open_with_index_dir_and_companies(&file, Some("TP"), companies, None).unwrap();
    let data = TestData::new(Arc::new(provider));
    let app = |id: &str, deps: &[&str], src: &str| AppSource {
        id: id.into(),
        name: id.into(),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files: vec![(format!("{id}.al"), src.into())],
    };
    let rt = CorpusRuntime::run_suites_with(
        vec![
            app("main", &[BASE_ID, EXT_ID], main),
            app(BASE_ID, &[], base),
            app(EXT_ID, &[BASE_ID], EXT),
        ],
        vec![
            al_sema::AppKind::Workspace,
            al_sema::AppKind::Dependency,
            al_sema::AppKind::Dependency,
        ],
        &[],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions {
            test_data: Some(data.clone()),
            ..Default::default()
        },
    );
    (rt, data)
}
