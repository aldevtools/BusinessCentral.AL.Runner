//! How a backup's kept index was obtained (`TableProvider::index_state`), on bcdb's synthetic `typeprobe` database.

use al_bcdb_std::BackupProvider;
use al_host::testdata::{IndexState, TableProvider};
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../al-bcdb/tests/fixtures")
        .join(name)
}

fn cost(p: &BackupProvider) -> String {
    p.cost().unwrap()
}

#[test]
fn the_index_state_is_built_then_read_and_off_without_a_directory() {
    let dir = tempfile::tempdir().unwrap();
    let bak = fixture("typeprobe.bak");
    let first = BackupProvider::open_with_index_dir(&bak, Some("TP"), Some(dir.path())).unwrap();
    assert_eq!(first.index_state(), Some(IndexState::Built));
    assert!(cost(&first).contains("(kept map and catalog built and saved)"), "{}", cost(&first));
    let second = BackupProvider::open_with_index_dir(&bak, Some("TP"), Some(dir.path())).unwrap();
    assert_eq!(second.index_state(), Some(IndexState::Read));
    assert!(cost(&second).contains("(kept map and catalog used)"), "{}", cost(&second));
    let none = BackupProvider::open(&bak, Some("TP")).unwrap();
    assert_eq!(none.index_state(), Some(IndexState::Off));
    assert!(cost(&none).contains("(map and catalog not kept)"), "{}", cost(&none));
    // A .bacpac has no page map: nothing is kept, with or without a directory.
    let bacpac =
        BackupProvider::open_with_index_dir(&fixture("typeprobe.bacpac"), Some("TP"), Some(dir.path())).unwrap();
    assert_eq!(bacpac.index_state(), Some(IndexState::Off));
    assert!(cost(&bacpac).contains("(map and catalog not kept)"), "{}", cost(&bacpac));
}
