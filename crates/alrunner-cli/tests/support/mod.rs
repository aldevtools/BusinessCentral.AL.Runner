//! Helpers the CLI test binaries share.
#![allow(dead_code)]

pub fn crc32(data: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &b in data {
        crc ^= u32::from(b);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// A ZIP with stored entries.
pub fn zip(entries: &[(&str, String)]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut central = Vec::new();
    for (name, text) in entries {
        let data = text.as_bytes();
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
    let cd_offset = out.len() as u32;
    let n = entries.len() as u16;
    let cd_len = central.len() as u32;
    out.extend(central);
    out.extend(0x0605_4b50u32.to_le_bytes());
    out.extend([0, 0, 0, 0]);
    out.extend(n.to_le_bytes());
    out.extend(n.to_le_bytes());
    out.extend(cd_len.to_le_bytes());
    out.extend(cd_offset.to_le_bytes());
    out.extend([0, 0]);
    out
}

/// A folder of `tests/fixtures/impact`.
pub fn impact_fixture(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/impact")
        .join(name)
}

/// The impact fixture's extension without its own test codeunit, under `root/ext-only`.
pub fn ext_only(root: &std::path::Path) -> std::path::PathBuf {
    let ext = impact_fixture("ext");
    let only = root.join("ext-only");
    std::fs::create_dir_all(&only).unwrap();
    std::fs::copy(ext.join("app.json"), only.join("app.json")).unwrap();
    let al = std::fs::read_to_string(ext.join("Subscriber.Codeunit.al")).unwrap();
    let cut = al
        .find("// The extension's own tests")
        .expect("the fixture's own tests");
    std::fs::write(only.join("Subscriber.Codeunit.al"), &al[..cut]).unwrap();
    only
}

/// stdout and stderr of a run.
pub fn text(o: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
}

/// The cells of a row of the `Sources` table, after the part's name.
pub fn source_row(out: &str, name: &str) -> String {
    let at = out
        .find("\nSources\n")
        .unwrap_or_else(|| panic!("no Sources table in:\n{out}"));
    let line = out[at..]
        .lines()
        .find(|l| l.trim_start().starts_with(&format!("{name} ")))
        .unwrap_or_else(|| panic!("no row {name} in:\n{out}"));
    line.trim_start()[name.len()..]
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn app(id: &str, name: &str, al: &str) -> Vec<u8> {
    let json = format!(
        r#"{{"id":"{id}","name":"{name}","publisher":"Fixture","version":"1.0.0.0","idRanges":[{{"from":50000,"to":59999}}]}}"#
    );
    zip(&[("app.json", json), ("src/Code.al", al.to_string())])
}

pub fn test_codeunit(id: u32, name: &str, tests: &[(&str, bool)]) -> String {
    let mut body = String::new();
    for (t, pass) in tests {
        let stmt = if *pass { "" } else { "Error('boom');" };
        body.push_str(&format!(
            "    [Test]\n    procedure {t}()\n    begin\n        {stmt}\n    end;\n\n"
        ));
    }
    format!("codeunit {id} \"{name}\"\n{{\n    Subtype = Test;\n    TestType = UnitTest;\n\n{body}}}\n")
}

/// The fake artifact of `build` under `home/.bcartifacts.cache/sandbox/<build>/platform` and a fake BCApps checkout
/// (written again for each build: the same files).
pub fn fake_artifact(root: &std::path::Path, build: &str, fail_b: bool) {
    let test = root.join(format!(
        "home/.bcartifacts.cache/sandbox/{build}/platform/Applications/Fake/Test"
    ));
    std::fs::create_dir_all(&test).unwrap();
    let write = |file: &str, bytes: Vec<u8>| std::fs::write(test.join(file), bytes).unwrap();
    write(
        "Tests-Fake-A.Source.zip",
        app(
            "a0000000-0000-0000-0000-000000000001",
            "Tests-Fake-A",
            &test_codeunit(50100, "Fake A Tests", &[("APasses", true), ("ADisabled", false)]),
        ),
    );
    write(
        "Tests-Fake-B.Source.zip",
        app(
            "a0000000-0000-0000-0000-000000000002",
            "Tests-Fake-B",
            &test_codeunit(50200, "Fake B Tests", &[("BRuns", !fail_b)]),
        ),
    );
    write(
        "Fake Test Library.Source.zip",
        app(
            "a0000000-0000-0000-0000-000000000003",
            "Fake Test Library",
            "codeunit 50300 \"Fake Library\"\n{\n    procedure Help()\n    begin\n    end;\n}\n",
        ),
    );
    write(
        "_Exclude_Fake Tests.Source.zip",
        app(
            "a0000000-0000-0000-0000-000000000004",
            "_Exclude_Fake Tests",
            &test_codeunit(50400, "Fake Excluded Tests", &[("XFails", false)]),
        ),
    );
    // Not a test app: no test in its path.
    let source = test.parent().unwrap().join("Source");
    std::fs::create_dir_all(&source).unwrap();
    std::fs::write(source.join("Fake App.Source.zip"), b"never read").unwrap();
    // BCApps: the W1 test project (default lane UnitTest, an empty company), no Legacy bucket for the fakes, and one
    // disabled test.
    let bcapps = root.join("bcapps");
    let settings = bcapps.join("build/projects/Test Apps W1/.AL-Go");
    std::fs::create_dir_all(&settings).unwrap();
    std::fs::write(
        settings.join("settings.json"),
        r#"{"testType":"UnitTest","companyName":"Empty Company","buildModes":["L"],
            "ConditionalSettings":[{"buildModes":["L"],"settings":{"testType":"Legacy","bucketNumber":1,"companyName":"CRONUS International Ltd."}}]}"#,
    )
    .unwrap();
    let scripts = bcapps.join("build/scripts");
    std::fs::create_dir_all(&scripts).unwrap();
    std::fs::write(
        scripts.join("TestConfiguration.json"),
        r#"{"LegacyTests-Bucket1":["Tests-Other"]}"#,
    )
    .unwrap();
    let disabled = bcapps.join("src/DisabledTests");
    std::fs::create_dir_all(&disabled).unwrap();
    std::fs::write(
        disabled.join("Fake.json"),
        r#"[{"codeunitId":50100,"codeunitName":"Fake A Tests","method":"ADisabled"}]"#,
    )
    .unwrap();
}

/// One more fake Microsoft test app with one passing test, in the artifact of `build`.
pub fn add_test_app(root: &std::path::Path, build: &str, name: &str, codeunit: u32, test: &str) {
    let dir = root.join(format!(
        "home/.bcartifacts.cache/sandbox/{build}/platform/Applications/Fake/Test"
    ));
    std::fs::write(
        dir.join(format!("{name}.Source.zip")),
        app(
            &format!("a0000000-0000-0000-0000-0000000{codeunit:05}"),
            name,
            &test_codeunit(codeunit, &format!("{name} Tests"), &[(test, true)]),
        ),
    )
    .unwrap();
}
