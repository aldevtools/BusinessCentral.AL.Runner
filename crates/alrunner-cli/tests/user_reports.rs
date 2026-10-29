//! `al-runner test` on the shapes of issues outside users reported. Nothing here is Microsoft's code.

mod support;

use std::path::Path;
use std::process::{Command, Output};
use support::{text, zip};

fn alrunner(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_al-runner"))
        .env("AL_CHECK_THREADS", "2")
        .env("AL_LOWER_THREADS", "2")
        .env("AL_TEST_THREADS", "2")
        .env("ALRUNNER_NO_PATH_NOTE", "1")
        .env_remove("ALRUNNER_NO_COMPILE_CACHE")
        .env_remove("AL_EAGER_DEPENDENCIES")
        .args(args)
        .output()
        .expect("run al-runner")
}

fn write(dir: &Path, rel: &str, content: &str) {
    let p = dir.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, content).unwrap();
}

/// An app of the 6044 shape: its id, name and the apps it depends on (by index into `APPS`).
struct App {
    id: &'static str,
    name: &'static str,
    deps: &'static [usize],
    al: &'static str,
}

const W: usize = 0;
const A1: usize = 3;
const A2: usize = 4;
const A3: usize = 5;
const P1: usize = 6;
const P2: usize = 7;

/// al-runner issue 6044, reopened: workspace = W (a base app), X (on W, P1 and P2), T (tests for X). `--dep` holds
/// compiled `.app` packages that carry source: P1 on W and two other dependency apps (A1, and A2 which stands on A1),
/// P2 on W and one other dependency app (A3, on nothing).
const APPS: &[App] = &[
    App {
        id: "60440000-0000-0000-0000-000000000001",
        name: "W Base",
        deps: &[],
        al: r#"
table 60440 "W Setup" { fields { field(1; K; Code[10]) { } field(2; V; Text[30]) { } } keys { key(PK; K) { } } }
codeunit 60441 "W Xml" { procedure Tag(T: Text): Text begin exit('<' + T + '>'); end; }
codeunit 60442 "W Events"
{
    procedure Compute(): Integer var R: Integer; begin OnCompute(R); exit(R); end;
    [IntegrationEvent(false, false)] procedure OnCompute(var R: Integer) begin end;
}
interface "W Channel" { procedure Code(): Integer; }
enum 60443 "W Channel Kind" implements "W Channel" { Extensible = true; value(0; None) { Implementation = "W Channel" = "W No Channel"; } }
codeunit 60444 "W No Channel" implements "W Channel" { procedure Code(): Integer begin exit(0); end; }
codeunit 60445 "W Send"
{
    procedure Send(K: Enum "W Channel Kind"): Integer var C: Interface "W Channel"; begin C := K; exit(C.Code()); end;
    procedure Insert(): Text var S: Record "W Setup"; begin S.K := 'B'; S.Insert(true); S.Get('B'); exit(S.V); end;
}
"#,
    },
    App {
        id: "60440000-0000-0000-0000-000000000002",
        name: "X Product",
        deps: &[W, P1, P2],
        al: r#"
codeunit 60450 "X Mgt"
{
    procedure Wrap(T: Text): Text var N: Codeunit "P1 Net"; begin exit(N.Wrap(T)); end;
    procedure SetupV(): Text var N: Codeunit "P1 Net"; S: Record "W Setup"; begin S := N.Setup(); exit(S.V); end;
    procedure Buffered(): Integer
    var
        TempBuffer: Record "P2 Buffer" temporary;
    begin
        TempBuffer.No := 1;
        TempBuffer.Insert();
        TempBuffer.No := 2;
        TempBuffer.Insert();
        exit(TempBuffer.Count());
    end;
    procedure Filled(): Integer var F: Codeunit "P2 Fill"; begin exit(F.Fill()); end;
}
"#,
    },
    App {
        id: "60440000-0000-0000-0000-000000000003",
        name: "X Tests",
        deps: &[1, W, P1, P2],
        al: r#"
codeunit 60460 "X Tests"
{
    Subtype = Test;
    [Test] procedure Wraps() var X: Codeunit "X Mgt"; begin if X.Wrap('a') <> '<a>' then Error('wrap'); end;
    [Test] procedure SetsUp() var X: Codeunit "X Mgt"; begin if X.SetupV() <> 'net' then Error('setup'); end;
    [Test] procedure BuffersInP2() var X: Codeunit "X Mgt"; begin if X.Buffered() <> 2 then Error('buffer'); end;
    [Test] procedure EventInP2() var E: Codeunit "W Events"; begin if E.Compute() <> 10 then Error('event %1', E.Compute()); end;
    [Test] procedure ChannelInP2() var S: Codeunit "W Send"; begin if S.Send("W Channel Kind"::Mail) <> 6 then Error('channel'); end;
    [Test] procedure TriggerInP2() var S: Codeunit "W Send"; begin if S.Insert() <> 'p2' then Error('trigger'); end;
    [Test] procedure FillsInP2() var X: Codeunit "X Mgt"; begin if X.Filled() <> 5 then Error('fill'); end;
}
"#,
    },
    App {
        id: "60440000-0000-0000-0000-000000000011",
        name: "A1 Lib",
        deps: &[],
        al: r#"codeunit 60470 "A1 Lib" { procedure One(): Integer begin exit(1); end; }"#,
    },
    App {
        id: "60440000-0000-0000-0000-000000000012",
        name: "A2 Onb",
        deps: &[A1],
        al: r#"codeunit 60471 "A2 Onb" { procedure Two(): Integer var L: Codeunit "A1 Lib"; begin exit(L.One() + 1); end; }"#,
    },
    App {
        id: "60440000-0000-0000-0000-000000000013",
        name: "A3 Conn",
        deps: &[],
        al: r#"codeunit 60472 "A3 Conn" { procedure Three(): Integer begin exit(3); end; }"#,
    },
    App {
        id: "60440000-0000-0000-0000-000000000021",
        name: "P1 Network",
        deps: &[W, A1, A2],
        al: r#"
codeunit 60480 "P1 Net"
{
    procedure Wrap(T: Text): Text var XmlDoc: Codeunit "W Xml"; O: Codeunit "A2 Onb"; begin if O.Two() <> 2 then exit(''); exit(XmlDoc.Tag(T)); end;
    procedure Setup(): Record "W Setup" var S: Record "W Setup"; begin S.K := 'A'; S.V := 'net'; exit(S); end;
}
"#,
    },
    App {
        id: "60440000-0000-0000-0000-000000000022",
        name: "P2 Connector",
        deps: &[W, A3],
        al: r#"
table 60490 "P2 Buffer" { fields { field(1; No; Integer) { } } keys { key(PK; No) { } } }
codeunit 60492 "P2 Subs"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"W Events", 'OnCompute', '', false, false)]
    local procedure Handle(var R: Integer)
    var
        TempBuffer: Record "P2 Buffer" temporary;
    begin
        TempBuffer.No := 10;
        TempBuffer.Insert();
        R += TempBuffer.Count() * 10;
    end;
}
enumextension 60493 "P2 Channel" extends "W Channel Kind" { value(60493; Mail) { Implementation = "W Channel" = "P2 Mail"; } }
codeunit 60494 "P2 Mail" implements "W Channel" { procedure Code(): Integer var C: Codeunit "A3 Conn"; begin exit(C.Three() * 2); end; }
tableextension 60495 "P2 Setup Ext" extends "W Setup" { trigger OnBeforeInsert() begin Rec.V := 'p2'; end; }
codeunit 60491 "P2 Fill" { procedure Fill(): Integer var C: Codeunit "A3 Conn"; X: Codeunit "W Xml"; begin if X.Tag('') <> '<>' then exit(0); exit(C.Three() + 2); end; }
"#,
    },
];

fn app_json(a: &App) -> String {
    let deps: Vec<String> = a
        .deps
        .iter()
        .map(|&d| {
            let d = &APPS[d];
            format!(
                r#"{{"id":"{}","name":"{}","publisher":"Test","version":"1.0.0.0"}}"#,
                d.id, d.name
            )
        })
        .collect();
    format!(
        r#"{{"id":"{}","name":"{}","publisher":"Test","version":"1.0.0.0","runtime":"14.0","dependencies":[{}]}}"#,
        a.id,
        a.name,
        deps.join(",")
    )
}

/// A compiled `.app` that carries its source, as alc writes it with `includeSourceInSymbolFile`.
fn package(dir: &Path, a: &App) {
    let deps: String = a
        .deps
        .iter()
        .map(|&d| {
            let d = &APPS[d];
            format!(
                r#"    <Dependency Id="{}" Name="{}" Publisher="Test" MinVersion="1.0.0.0" />
"#,
                d.id, d.name
            )
        })
        .collect();
    let manifest = format!(
        r#"<Package xmlns="http://schemas.microsoft.com/navx/2015/manifest">
  <App Id="{}" Name="{}" Publisher="Test" Version="1.0.0.0" Runtime="14.0" Target="Cloud" ShowMyCode="True" />
  <Dependencies>
{deps}  </Dependencies>
</Package>"#,
        a.id, a.name
    );
    std::fs::write(
        dir.join(format!("Test_{}_1.0.0.0.app", a.name)),
        zip(&[
            ("NavxManifest.xml", manifest),
            (format!("src/{}.al", a.name.replace(' ', "")).as_str(), a.al.to_string()),
        ]),
    )
    .unwrap();
}

/// Writes the workspace folders and the package folder; returns the arguments of `al-runner test`.
fn issue_6044_shape(root: &Path) -> Vec<String> {
    let pkg = root.join("packages");
    std::fs::create_dir_all(&pkg).unwrap();
    for &i in &[A1, A2, A3, P1, P2] {
        package(&pkg, &APPS[i]);
    }
    let mut args = vec!["test".to_string()];
    for a in &APPS[..3] {
        let dir = root.join(a.name.replace(' ', ""));
        write(&dir, "app.json", &app_json(a));
        write(&dir, "src/Main.al", a.al);
        args.push(dir.to_str().unwrap().to_string());
    }
    for a in [
        "--dep",
        pkg.to_str().unwrap(),
        "--no-download",
        "--cache-dir",
        root.join("cache").to_str().unwrap(),
    ] {
        args.push(a.to_string());
    }
    args
}

fn run(args: &[String], extra: &[&str]) -> String {
    let mut all: Vec<&str> = args.iter().map(String::as_str).collect();
    all.extend_from_slice(extra);
    let o = alrunner(&all);
    let t = text(&o);
    assert!(!t.contains("outside the workspace"), "{t}");
    assert!(t.contains("Tests: 7 total, 7 passed"), "{t}");
    assert_eq!(o.status.code(), Some(0), "{t}");
    t
}

#[test]
fn issue_6044_dependency_apps_on_a_workspace_app_through_other_dependency_apps_without_the_compile_cache() {
    let d = tempfile::tempdir().unwrap();
    let args = issue_6044_shape(d.path());
    run(&args, &["--no-compile-cache"]);
}

#[test]
fn issue_6044_dependency_apps_on_a_workspace_app_through_other_dependency_apps_with_the_compile_cache() {
    let d = tempfile::tempdir().unwrap();
    let args = issue_6044_shape(d.path());
    // The run that builds the cache, then the run that reads it.
    run(&args, &[]);
    run(&args, &[]);
}
