//! First review LOW 10: `run::app_json_text`, the app.json facts in the key of serve's kept compile and of the workspace
//! layer, keeps each field apart, so two apps whose fields differ never give the same text.

fn app_with(dir: &std::path::Path, name: &str, publisher: &str) -> crate::apps::AppInput {
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(
        dir.join("app.json"),
        format!(
            r#"{{"id":"88888888-0000-0000-0000-000000000001","name":"{name}","publisher":"{publisher}","version":"1.0.0.0","dependencies":[]}}"#
        ),
    )
    .unwrap();
    std::fs::write(dir.join("src/c.al"), "codeunit 50800 C { }\n").unwrap();
    crate::apps::load(dir, None).unwrap()
}

/// A space moved from the name into the publisher is another text.
#[test]
fn a_space_moved_between_two_fields_is_another_text() {
    let d = tempfile::tempdir().unwrap();
    let a = app_with(&d.path().join("a"), "A B", "C");
    let b = app_with(&d.path().join("b"), "A", "B C");
    assert_ne!(crate::run::app_json_text(&a), crate::run::app_json_text(&b));
}
