//! A Worksheet without the new-row line, filtered in filter group 2 on a key field, whose action deletes every line and leaves
//! Rec on a primary key no row has (page 5510 "Production Journal" Post): the list stands on the blank Rec, which takes the
//! filter value, as when Rec keeps the deleted row's key (al-runner issue 6027, the reporter's repro green on real BC
//! `N:B1;C:0:B1;`; corpus RtFilteredGoneRow.al TD_LeavesKey_Line1WithUpdate_OneLine; three lines per major, :656). Without
//! CurrPage.Update(false), or with another count of lines shown before the action, it is refused.

const SRC: &str = r#"
table 51500 "UK Line"
{
    fields { field(1; "Batch Name"; Code[10]) { } field(2; "Line No."; Integer) { } }
    keys { key(PK; "Batch Name", "Line No.") { Clustered = true; } }
}
codeunit 51501 "UK Log"
{
    SingleInstance = true;
    var E: Text;
    procedure Add(T: Text) begin E += T + ';'; end;
    procedure Take() R: Text begin R := E; E := ''; end;
}
page 51502 "UK Ws"
{
    PageType = Worksheet; SourceTable = "UK Line"; InsertAllowed = false;
    layout { area(Content) { repeater(Rows) { field("Line No."; Rec."Line No.") { } } } }
    actions
    {
        area(Processing)
        {
            action(Post) { trigger OnAction() begin Rec.DeleteAll(); Rec."Line No." := 1; CurrPage.Update(false); end; }
            action(PostNoUpdate) { trigger OnAction() begin Rec.DeleteAll(); Rec."Line No." := 1; end; }
            action(PostKeepKey) { trigger OnAction() begin Rec.DeleteAll(); CurrPage.Update(false); end; }
        }
    }
    trigger OnOpenPage() begin Rec.FilterGroup(2); Rec.SetRange("Batch Name", 'B1'); Rec.FilterGroup(0); end;
    trigger OnNewRecord(BelowxRec: Boolean) var L: Codeunit "UK Log"; begin L.Add('N:' + Rec."Batch Name"); end;
    trigger OnAfterGetRecord() begin Shown += 1; end;
    var Shown: Integer;
    trigger OnAfterGetCurrRecord() var L: Codeunit "UK Log"; begin L.Add('C:' + Format(Rec."Line No.") + ':' + Rec."Batch Name"); end;
}
codeunit 51503 C
{
    procedure Run(Which: Integer; Lines: Integer): Text
    begin
        exit(RunOther(Which, Lines, false));
    end;
    procedure RunOther(Which: Integer; Lines: Integer; OtherBatch: Boolean): Text
    var Line: Record "UK Line"; L: Codeunit "UK Log"; Ws: TestPage "UK Ws"; Seen: Text; I: Integer;
    begin
        Line.DeleteAll();
        for I := 1 to Lines do begin Line."Batch Name" := 'B1'; Line."Line No." := 10000 * I; Line.Insert(); end;
        if OtherBatch then begin Line."Batch Name" := 'B2'; Line."Line No." := 10000; Line.Insert(); end;
        Ws.OpenEdit();
        L.Take();
        case Which of
            0: Ws.Post.Invoke();
            1: Ws.PostNoUpdate.Invoke();
            2: Ws.PostKeepKey.Invoke();
        end;
        Seen := L.Take();
        Ws.Close();
        exit(Seen + '|' + Format(Line.Count()));
    end;
    procedure Post(): Text begin exit(Run(0, 1)); end;
    procedure PostOtherBatchRowRemains(): Text begin exit(RunOther(0, 1, true)); end;
    procedure PostThreeLines(): Text begin exit(Run(0, 3)); end;
    procedure PostFourLines(): Text begin exit(Run(0, 4)); end;
    procedure PostTwoLines(): Text begin exit(Run(0, 2)); end;
    procedure PostNoUpdate(): Text begin exit(Run(1, 1)); end;
    procedure PostKeepKey(): Text begin exit(Run(2, 1)); end;
}
"#;

fn run(routine: &str) -> Result<al_builtins::value::Value, al_builtins::native::Signal> {
    al_testkit::run_in(SRC, "C", routine)
}

#[test]
fn rec_left_on_a_key_no_row_has_stands_on_the_blank_rec_with_the_filter_value() {
    assert_eq!(al_testkit::text(run("PostKeepKey")), "N:B1;C:0:B1;|0");
    assert_eq!(al_testkit::text(run("Post")), "N:B1;C:0:B1;|0");
    // A row of another batch stays in the table, outside the view (corpus RtFilteredGoneRow.al:677 _OtherBatchRowRemains).
    assert_eq!(al_testkit::text(run("PostOtherBatchRowRemains")), "N:B1;C:0:B1;|1");
}

/// Runs `routine` of codeunit `C` on a BC `bc` target.
fn run_on(bc: &str, routine: &str) -> Result<al_builtins::value::Value, al_builtins::native::Signal> {
    let target = al_version::Target::for_bc(al_version::BcVersion::parse(bc).unwrap());
    let apps = vec![al_sema::workspace::AppSource {
        id: "a".into(),
        name: "a".into(),
        dependencies: vec![],
        files: vec![("f.al".into(), SRC.into())],
    }];
    let (ws, l) = al_sema::lower::compile_for(apps, &Default::default(), &target);
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let blocked: Vec<Vec<Option<String>>> = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let oi = l.program.objects.iter().position(|o| o.name == "C").unwrap();
    let ri = l.program.objects[oi]
        .routines
        .iter()
        .position(|r| r.name == routine)
        .unwrap();
    let engine = al_vm::Engine::with_target(l.program, blocked, Some(target));
    let mut s = al_vm::Session::new(engine);
    let inst = s.instance(oi);
    s.call(oi, ri, inst, vec![]).map(|(v, _)| v)
}

/// Three deleted lines run OnNewRecord and OnAfterGetCurrRecord once on BC 28 and twice on BC 27 and from BC 29 (corpus
/// RtFilteredGoneRow.al:656 TD_LeavesKey_Line1WithUpdate_ThreeLines, `#if BC28PLUS and not BC29PLUS`).
#[test]
fn three_deleted_lines_run_the_triggers_as_the_major_does() {
    assert_eq!(
        al_testkit::text(run_on("27.5", "PostThreeLines")),
        "N:B1;C:0:B1;N:B1;C:0:B1;|0"
    );
    assert_eq!(al_testkit::text(run_on("28.5", "PostThreeLines")), "N:B1;C:0:B1;|0");
    assert_eq!(
        al_testkit::text(run_on("29.0", "PostThreeLines")),
        "N:B1;C:0:B1;N:B1;C:0:B1;|0"
    );
}

/// Two lines, or four and more, are not pinned.
#[test]
fn another_count_of_deleted_lines_is_refused() {
    for routine in ["PostTwoLines", "PostFourLines"] {
        let r = run_on("28.5", routine);
        assert!(
            matches!(r, Err(al_builtins::native::Signal::Unsupported(_))),
            "{routine}: {r:?}"
        );
    }
}

#[test]
fn without_curr_page_update_it_is_refused() {
    let r = run("PostNoUpdate");
    assert!(matches!(r, Err(al_builtins::native::Signal::Unsupported(_))), "{r:?}");
}
