//! The layout files of an app folder: the files a report names with `LayoutFile` in a rendering section and with the legacy
//! `RDLCLayout`, `WordLayout` and `ExcelLayout` properties, by their path from the app folder. A legacy property's file
//! exports as its bytes (corpus reports/layouts-saveas/Y17LayoutExportTests.al:105-124 Export_LegacyProperties); an app
//! under test missed it (al-runner issue 5712, the reporter's `WordLayout = 'layouts/ALR Layout Repro.docx'`).

#[test]
fn an_app_folder_holds_the_files_of_the_legacy_layout_properties_and_of_its_rendering_section() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join("layouts")).unwrap();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("app.json"),
        r#"{ "publisher": "AL Harness", "version": "1.0.0.0" }"#,
    )
    .unwrap();
    for (name, bytes) in [
        ("layouts/ALR Layout Repro.docx", &b"word"[..]),
        ("layouts/Legacy.rdl", b"rdlc"),
        ("layouts/Legacy.xlsx", b"excel"),
        ("layouts/Section.docx", b"section"),
    ] {
        std::fs::write(root.join(name), bytes).unwrap();
    }
    std::fs::write(
        root.join("src/Reports.al"),
        "report 50100 \"ALR Layout Repro\"\n{\n    WordLayout = 'layouts/ALR Layout Repro.docx';\n    DefaultLayout = Word;\n    dataset { dataitem(I; Integer) { column(N; Number) { } } }\n}\n\
         report 50101 \"ALR Legacy\"\n{\n    RDLCLayout = './layouts/Legacy.rdl';\n    ExcelLayout = './layouts/Legacy.xlsx';\n    ExcelLayoutMultipleDataSheets = true;\n    dataset { dataitem(I; Integer) { column(N; Number) { } } }\n}\n\
         report 50102 \"ALR Section\"\n{\n    dataset { dataitem(I; Integer) { column(N; Number) { } } }\n    rendering { layout(L) { Type = Word; LayoutFile = 'layouts/Section.docx'; } }\n}\n\
         report 50103 \"ALR Missing\"\n{\n    WordLayout = 'layouts/Missing.docx';\n    dataset { dataitem(I; Integer) { column(N; Number) { } } }\n}\n",
    )
    .unwrap();
    let m = al_testrun::runtime::load_manifest(root).unwrap();
    let held: Vec<(&str, &[u8])> = m.layout_files.iter().map(|(p, b)| (p.as_str(), b.as_slice())).collect();
    assert_eq!(
        held,
        vec![
            ("layouts/ALR Layout Repro.docx", &b"word"[..]),
            ("layouts/Legacy.rdl", b"rdlc"),
            ("layouts/Legacy.xlsx", b"excel"),
            ("layouts/Section.docx", b"section"),
        ]
    );
}
