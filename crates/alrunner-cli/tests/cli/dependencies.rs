//! Dependency apps given as archives or folders.

use super::*;

/// A stored (uncompressed) ZIP with CRC-32s, enough for `al-deps`.
pub(super) fn zip(entries: &[(&str, &str)]) -> Vec<u8> {
    fn crc32(data: &[u8]) -> u32 {
        let mut c = !0u32;
        for &b in data {
            c ^= u32::from(b);
            for _ in 0..8 {
                c = if c & 1 != 0 { (c >> 1) ^ 0xEDB8_8320 } else { c >> 1 };
            }
        }
        !c
    }
    let mut out = Vec::new();
    let mut central = Vec::new();
    for (name, data) in entries {
        let data = data.as_bytes();
        let crc = crc32(data);
        let offset = out.len() as u32;
        out.extend(0x0403_4b50u32.to_le_bytes());
        out.extend([20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        out.extend(crc.to_le_bytes());
        out.extend((data.len() as u32).to_le_bytes());
        out.extend((data.len() as u32).to_le_bytes());
        out.extend((name.len() as u16).to_le_bytes());
        out.extend([0, 0]);
        out.extend(name.as_bytes());
        out.extend(data);
        central.extend(0x0201_4b50u32.to_le_bytes());
        central.extend([20, 0, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        central.extend(crc.to_le_bytes());
        central.extend((data.len() as u32).to_le_bytes());
        central.extend((data.len() as u32).to_le_bytes());
        central.extend((name.len() as u16).to_le_bytes());
        central.extend([0; 12]);
        central.extend(offset.to_le_bytes());
        central.extend(name.as_bytes());
    }
    let start = out.len() as u32;
    let size = central.len() as u32;
    out.extend(central);
    out.extend(0x0605_4b50u32.to_le_bytes());
    out.extend([0, 0, 0, 0]);
    out.extend((entries.len() as u16).to_le_bytes());
    out.extend((entries.len() as u16).to_le_bytes());
    out.extend(size.to_le_bytes());
    out.extend(start.to_le_bytes());
    out.extend([0, 0]);
    out
}

/// A test app in a source archive, and a library it depends on in another.
fn archives(root: &Path) -> (PathBuf, PathBuf) {
    let lib_id = "22222222-0000-0000-0000-000000000001";
    let lib = root.join("Lib.Source.zip");
    std::fs::write(
        &lib,
        zip(&[
            ("app.json", &app_json(lib_id, "Lib", &[])),
            (
                "src/Helper.Codeunit.al",
                "codeunit 50300 \"Lib Helper\" { procedure Answer(): Integer begin exit(42); end; }",
            ),
        ]),
    )
    .unwrap();
    let app = root.join("Tests-Lib.Source.zip");
    std::fs::write(
        &app,
        zip(&[
            ("app.json", &app_json("22222222-0000-0000-0000-000000000002", "Tests-Lib", &[(lib_id, "Lib")])),
            (
                "T.Codeunit.al",
                "codeunit 50301 \"Lib Tests\" { Subtype = Test; [Test] procedure UsesTheLibrary() var H: Codeunit \"Lib Helper\"; begin if H.Answer() <> 42 then Error('wrong'); end; }",
            ),
        ]),
    )
    .unwrap();
    (app, lib)
}

#[test]
fn a_source_archive_runs_with_its_dependency_given_by_dep() {
    let d = tempfile::tempdir().unwrap();
    let (app, lib) = archives(d.path());
    let o = alrunner(&[
        "test",
        app.to_str().unwrap(),
        "--dep",
        lib.to_str().unwrap(),
        "--no-download",
        "--verbose",
    ]);
    let t = text(&o);
    assert_eq!(code(&o), 0, "{t}");
    assert!(t.contains("Tests: 1 total, 1 passed"), "{t}");
    assert!(t.contains("dependencies: 1 apps"), "{t}");
    // The same, with the library found in a folder of packages.
    let o = alrunner(&[
        "test",
        app.to_str().unwrap(),
        "--package-cache",
        d.path().to_str().unwrap(),
        "--no-download",
    ]);
    assert_eq!(code(&o), 0, "{}", text(&o));
}

#[test]
fn a_dependency_app_reads_its_own_packaged_resources() {
    // session/W5NavAppResources.al:GetResourceAsText_NameMatching: the name is matched exactly, relative to the
    // resource folder. The library's own code asks; the folder comes from the `resourceFolders` of its app.json.
    let d = tempfile::tempdir().unwrap();
    let lib_id = "22222222-0000-0000-0000-000000000011";
    let lib = d.path().join("Res.Source.zip");
    std::fs::write(
        &lib,
        zip(&[
            (
                "app.json",
                &format!(
                    r#"{{"id":"{lib_id}","name":"Res","publisher":"Test","version":"1.0.0.0","dependencies":[],"resourceFolders":[".resources"]}}"#
                ),
            ),
            (
                "src/Helper.Codeunit.al",
                "codeunit 50310 \"Res Helper\" { procedure Greeting(): Text begin exit(NavApp.GetResourceAsText('sub/hello.txt')); end; }",
            ),
            (".resources/sub/hello.txt", "Hello from a dependency"),
        ]),
    )
    .unwrap();
    let app = d.path().join("Tests-Res.Source.zip");
    std::fs::write(
        &app,
        zip(&[
            ("app.json", &app_json("22222222-0000-0000-0000-000000000012", "Tests-Res", &[(lib_id, "Res")])),
            (
                "T.Codeunit.al",
                "codeunit 50311 \"Res Tests\" { Subtype = Test; [Test] procedure ReadsIt() var H: Codeunit \"Res Helper\"; begin if H.Greeting() <> 'Hello from a dependency' then Error('got %1', H.Greeting()); end; }",
            ),
        ]),
    )
    .unwrap();
    let o = alrunner(&[
        "test",
        app.to_str().unwrap(),
        "--dep",
        lib.to_str().unwrap(),
        "--no-download",
    ]);
    let t = text(&o);
    assert_eq!(code(&o), 0, "{t}");
    assert!(t.contains("Tests: 1 total, 1 passed"), "{t}");
}

/// al-runner issue 6048: a compiled `.app` with source records the `preprocessorSymbols` its publisher compiled it
/// with in `NavxManifest.xml` (`<PreprocessorSymbols><PreprocessorSymbol Name=".."/>`; alc writes exactly the
/// app.json list there and defines no symbol of its own). Its code is compiled with those symbols, so `#if` takes the
/// branch alc took.
#[test]
fn a_compiled_dependency_app_is_compiled_with_the_preprocessor_symbols_of_its_manifest() {
    let d = tempfile::tempdir().unwrap();
    let lib_id = "22222222-0000-0000-0000-000000000021";
    let pkg = d.path().join("pkg");
    std::fs::create_dir_all(&pkg).unwrap();
    let manifest = format!(
        r#"<Package xmlns="http://schemas.microsoft.com/navx/2015/manifest">
  <App Id="{lib_id}" Name="Pre Ext" Publisher="Test" Version="1.0.0.0" Runtime="17.0" Target="Cloud" ShowMyCode="False" />
  <Dependencies />
  <PreprocessorSymbols>
    <PreprocessorSymbol Name="NEWBASE" />
  </PreprocessorSymbols>
  <SuppressWarnings />
</Package>"#
    );
    std::fs::write(
        pkg.join("Test_Pre Ext_1.0.0.0.app"),
        zip(&[
            ("NavxManifest.xml", &manifest),
            (
                "src/Ext.al",
                "codeunit 50320 \"Pre Ext Value\" { procedure Value(): Integer begin\n#if NEWBASE\n exit(1);\n#else\n exit(2);\n#endif\n end; }",
            ),
        ]),
    )
    .unwrap();
    let app = d.path().join("tests");
    write(
        &app,
        "app.json",
        &app_json(
            "22222222-0000-0000-0000-000000000022",
            "Pre Tests",
            &[(lib_id, "Pre Ext")],
        ),
    );
    write(
        &app,
        "T.Codeunit.al",
        "codeunit 50321 \"Pre Tests\" { Subtype = Test; [Test] procedure UsesItsOwnSymbols() var V: Codeunit \"Pre Ext Value\"; begin if V.Value() <> 1 then Error('Value = %1', V.Value()); end; }",
    );
    let o = alrunner(&[
        "test",
        app.to_str().unwrap(),
        "--dep",
        pkg.to_str().unwrap(),
        "--no-download",
    ]);
    let t = text(&o);
    assert_eq!(code(&o), 0, "{t}");
    assert!(t.contains("Tests: 1 total, 1 passed"), "{t}");
}

#[test]
fn a_missing_dependency_refuses_the_tests_that_need_it_instead_of_failing_the_run() {
    let d = tempfile::tempdir().unwrap();
    let (app, _lib) = archives(d.path());
    let o = alrunner(&["test", app.to_str().unwrap(), "--no-download"]);
    let t = text(&o);
    assert_eq!(code(&o), 7, "{t}");
    assert!(t.contains("1 not run"), "{t}");
    assert!(t.contains("is outside the workspace"), "{t}");
}

/// Al-runner issue 5749: a run-time lookup by id of an object no loaded app declares, in a run that skipped a
/// dependency, may land in the skipped app, where BC would find the object. The test is refused and names the app; it
/// does not fail with BC's missing-object error. A test that does not reach the lookup still passes.
#[test]
fn a_lookup_by_id_into_a_skipped_dependency_refuses_the_test_and_names_the_app() {
    let d = tempfile::tempdir().unwrap();
    let lib_id = "22222222-0000-0000-0000-000000000021";
    let lib = d.path().join("ById Lib.Source.zip");
    std::fs::write(
        &lib,
        zip(&[
            ("app.json", &app_json(lib_id, "ById Lib", &[])),
            (
                "src/Dep.Codeunit.al",
                "codeunit 50320 \"ById Dep\" { trigger OnRun() begin end; }",
            ),
        ]),
    )
    .unwrap();
    let app = d.path().join("Tests-ById.Source.zip");
    std::fs::write(
        &app,
        zip(&[
            (
                "app.json",
                &app_json(
                    "22222222-0000-0000-0000-000000000022",
                    "Tests-ById",
                    &[(lib_id, "ById Lib")],
                ),
            ),
            (
                "T.Codeunit.al",
                "codeunit 50321 \"ById Tests\" { Subtype = Test;
                    [Test] procedure ByIdNotReached() begin if Go(false) <> 4 then Error('go(false)'); end;
                    [Test] procedure ByIdReached() begin if Go(true) <> 4 then Error('go(true)'); end;
                    [Test] procedure ByIdReachedUnderAssertError() begin asserterror Codeunit.Run(50320); end;
                    [Test] procedure RecordRefOpenById() var R: RecordRef; begin R.Open(50322); end;
                    [Test] procedure PageRunById() begin Page.Run(50323); end;
                    local procedure Go(Flag: Boolean): Integer begin if Flag then Codeunit.Run(50320); exit(4); end; }",
            ),
        ]),
    )
    .unwrap();
    let o = alrunner(&["test", app.to_str().unwrap(), "--no-download", "--output-json"]);
    let v = json_of(&o);
    assert_eq!(
        row(&v, "Codeunit50321.ByIdNotReached")["status"],
        "pass",
        "{}",
        text(&o)
    );
    for m in [
        "ByIdReached",
        "ByIdReachedUnderAssertError",
        "RecordRefOpenById",
        "PageRunById",
    ] {
        let r = row(&v, &format!("Codeunit50321.{m}"));
        assert_eq!(r["notRun"], true, "{m}: {r}");
        let msg = r["message"].as_str().unwrap();
        assert!(msg.contains("`ById Lib`") && msg.contains("skipped"), "{m}: {msg}");
    }
    // With the dependency given, the lookup finds the codeunit.
    let o = alrunner(&[
        "test",
        app.to_str().unwrap(),
        "--dep",
        lib.to_str().unwrap(),
        "--no-download",
        "--output-json",
    ]);
    let v = json_of(&o);
    assert_eq!(row(&v, "Codeunit50321.ByIdReached")["status"], "pass", "{}", text(&o));
}

#[test]
fn an_app_folder_may_be_given_as_a_dependency_and_a_build_fills_microsoft_variables() {
    let d = tempfile::tempdir().unwrap();
    // The library is a folder; its version is one of Microsoft's build variables.
    let lib = d.path().join("lib");
    write(
        &lib,
        "app.json",
        r#"{"id":"22222222-0000-0000-0000-000000000009","name":"Lib","publisher":"Test","version":"$(app_currentVersion)","dependencies":[]}"#,
    );
    write(
        &lib,
        "H.al",
        "codeunit 50400 \"Lib Helper\" { procedure Answer(): Integer begin exit(7); end; }",
    );
    let app = d.path().join("app");
    write(
        &app,
        "app.json",
        &app_json(
            "22222222-0000-0000-0000-00000000000a",
            "App",
            &[("22222222-0000-0000-0000-000000000009", "Lib")],
        ),
    );
    write(
        &app,
        "T.al",
        "codeunit 50401 \"T\" { Subtype = Test; [Test] procedure Uses() var H: Codeunit \"Lib Helper\"; begin if H.Answer() <> 7 then Error('wrong'); end; }",
    );
    // Without a build the variable cannot be filled: said, not guessed.
    let o = alrunner(&[
        "test",
        app.to_str().unwrap(),
        "--dep",
        lib.to_str().unwrap(),
        "--no-download",
    ]);
    assert!(text(&o).contains("$(app_"), "{}", text(&o));
    // An exact build fills it.
    let o = alrunner(&[
        "test",
        app.to_str().unwrap(),
        "--dep",
        lib.to_str().unwrap(),
        "--no-download",
        "--bc-version",
        "27.5.46862.48827",
    ]);
    assert_eq!(code(&o), 0, "{}", text(&o));
    assert!(text(&o).contains("Tests: 1 total, 1 passed"), "{}", text(&o));
}
