//! The layout files of an app folder: the files a report names with `LayoutFile` in a rendering section and with the legacy
//! `RDLCLayout` and `WordLayout` properties, by their path from the app folder (corpus
//! reports/layouts-saveas/Y17LayoutExportTests.al:105-124 Export_LegacyProperties exports the files of those two properties;
//! an app under test missed them, al-runner issue 5712, the reporter's `WordLayout = 'layouts/ALR Layout Repro.docx'`).
//! No corpus case exports the file of a legacy `ExcelLayout` property: it is not held. A path that leaves the app folder
//! names no file of the app: it is not held.

#[test]
fn an_app_folder_holds_the_files_of_the_legacy_layout_properties_and_of_its_rendering_section() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("app");
    std::fs::create_dir_all(root.join("layouts")).unwrap();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::create_dir_all(dir.path().join("shared")).unwrap();
    std::fs::write(
        root.join("app.json"),
        r#"{ "publisher": "AL Harness", "version": "1.0.0.0" }"#,
    )
    .unwrap();
    for (name, bytes) in [
        ("app/layouts/ALR Layout Repro.docx", &b"word"[..]),
        ("app/layouts/Legacy.rdl", b"rdlc"),
        ("app/layouts/Legacy.xlsx", b"excel"),
        ("app/layouts/Section.docx", b"section"),
        ("app/layouts/Inside.rdl", b"inside"),
        ("shared/X.rdl", b"outside"),
        ("shared/Y.docx", b"outside"),
    ] {
        std::fs::write(dir.path().join(name), bytes).unwrap();
    }
    std::fs::write(
        root.join("src/Reports.al"),
        "report 50100 \"ALR Layout Repro\"\n{\n    WordLayout = 'layouts/ALR Layout Repro.docx';\n    DefaultLayout = Word;\n    dataset { dataitem(I; Integer) { column(N; Number) { } } }\n}\n\
         report 50101 \"ALR Legacy\"\n{\n    RDLCLayout = './layouts/Legacy.rdl';\n    ExcelLayout = './layouts/Legacy.xlsx';\n    ExcelLayoutMultipleDataSheets = true;\n    dataset { dataitem(I; Integer) { column(N; Number) { } } }\n}\n\
         report 50102 \"ALR Section\"\n{\n    dataset { dataitem(I; Integer) { column(N; Number) { } } }\n    rendering { layout(L) { Type = Word; LayoutFile = 'layouts/Section.docx'; } }\n}\n\
         report 50103 \"ALR Missing\"\n{\n    WordLayout = 'layouts/Missing.docx';\n    dataset { dataitem(I; Integer) { column(N; Number) { } } }\n}\n\
         report 50104 \"ALR Inside\"\n{\n    RDLCLayout = 'src/../layouts/Inside.rdl';\n    dataset { dataitem(I; Integer) { column(N; Number) { } } }\n}\n\
         report 50105 \"ALR Outside\"\n{\n    RDLCLayout = '../shared/X.rdl';\n    dataset { dataitem(I; Integer) { column(N; Number) { } } }\n}\n\
         report 50106 \"ALR Outside Section\"\n{\n    dataset { dataitem(I; Integer) { column(N; Number) { } } }\n    rendering { layout(L) { Type = Word; LayoutFile = '../shared/Y.docx'; } }\n}\n\
         report 50107 \"ALR Same Name\"\n{\n    RDLCLayout = 'X.rdl';\n    dataset { dataitem(I; Integer) { column(N; Number) { } } }\n}\n",
    )
    .unwrap();
    let m = al_testrun::runtime::load_manifest(&root).unwrap();
    let held: Vec<(&str, &[u8])> = m.layout_files.iter().map(|(p, b)| (p.as_str(), b.as_slice())).collect();
    assert_eq!(
        held,
        vec![
            ("layouts/ALR Layout Repro.docx", &b"word"[..]),
            ("layouts/Inside.rdl", b"inside"),
            ("layouts/Legacy.rdl", b"rdlc"),
            ("layouts/Section.docx", b"section"),
        ]
    );
}
