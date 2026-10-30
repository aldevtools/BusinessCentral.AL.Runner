//! al-runner issue 6377: a FactBox with `Provider = Lines` (a ListPart) and a SubPageLink on a field of the provider's row
//! reads its rows again as soon as that field is typed on the provider's current row. Measured on real BC by corpus writer
//! Y5 (branch al-runtime-rs-staging-y5, pages/lt-y5/Y5ProviderEditLink.al, codeunit 90980): To Bin B1 -> B2 shows B2 at once
//! (First Yes, Next No), the line is not stored yet, a FactBox linked on another field stays B1; a field whose OnValidate
//! calls CurrPage.Update stores B2 and shows B2; a read without First shows B2; typed blank: First No, Code ''; after the
//! edit Next to 20000 shows B2, B1 typed there shows B1, and Previous back to 10000 shows B2. AL source -> compile -> run.

use al_test_cli::report::Outcome;

const OBJECTS: &str = r#"
table 62560 "PFE Header" { fields { field(1; "No."; Code[20]) { } } keys { key(PK; "No.") { } } }
table 62561 "PFE Line" { fields { field(1; "Document No."; Code[20]) { } field(2; "Line No."; Integer) { }
    field(3; "To Bin Code"; Code[20]) { } field(4; "From Bin Code"; Code[20]) { } } keys { key(PK; "Document No.", "Line No.") { } } }
table 62562 "PFE Bin" { fields { field(1; "Code"; Code[20]) { } } keys { key(PK; "Code") { } } }
page 62563 "PFE Document" { PageType = Document; SourceTable = "PFE Header";
    layout { area(Content) { field("No."; Rec."No.") { } part(Lines; "PFE Lines") { SubPageLink = "Document No." = field("No."); } }
        area(FactBoxes) {
            part(ToBin; "PFE Bins") { Provider = Lines; SubPageLink = "Code" = field("To Bin Code"); }
            part(FromBin; "PFE Bins") { Provider = Lines; SubPageLink = "Code" = field("From Bin Code"); } } } }
page 62564 "PFE Lines" { PageType = ListPart; SourceTable = "PFE Line"; InsertAllowed = false;
    layout { area(Content) { repeater(Rows) { field("Line No."; Rec."Line No.") { } field("To Bin Code"; Rec."To Bin Code") { }
        field("From Bin Code"; Rec."From Bin Code") { trigger OnValidate() begin CurrPage.Update(); end; } } } } }
page 62565 "PFE Bins" { PageType = ListPart; SourceTable = "PFE Bin"; Editable = false;
    layout { area(Content) { repeater(Rows) { field("Code"; Rec."Code") { } } } } }
codeunit 62566 "PFE Seed" {
    procedure Seed() var H: Record "PFE Header"; L: Record "PFE Line"; B: Record "PFE Bin"; begin
        H.DeleteAll(); L.DeleteAll(); B.DeleteAll();
        H."No." := 'D1'; H.Insert(); B.Code := 'B1'; B.Insert(); B.Code := 'B2'; B.Insert();
        L."Document No." := 'D1'; L."Line No." := 10000; L."To Bin Code" := 'B1'; L."From Bin Code" := 'B1'; L.Insert();
        L."Line No." := 20000; L."To Bin Code" := 'B2'; L."From Bin Code" := 'B2'; L.Insert(); end; }
"#;

fn passes(tests: &str) {
    let src = format!("{OBJECTS}\ncodeunit 62570 T {{ Subtype = Test; {tests} }}");
    let o = al_testkit::outcomes_in(&src, "T");
    assert!(!o.is_empty());
    let bad: Vec<_> = o.into_iter().filter(|(_, r)| *r != Outcome::Pass).collect();
    assert!(bad.is_empty(), "{bad:?}");
}

#[test]
fn a_provider_factbox_follows_a_linked_field_typed_on_the_providers_row() {
    passes(
        r#"[Test] procedure Plain() var S: Codeunit "PFE Seed"; D: TestPage "PFE Document"; L: Record "PFE Line"; begin
            S.Seed(); D.OpenEdit(); D.GoToKey('D1');
            if not D.ToBin.First() then Error('before'); if D.ToBin.Code.Value() <> 'B1' then Error('before %1', D.ToBin.Code.Value());
            D.Lines."To Bin Code".SetValue('B2');
            if D.Lines."Line No.".Value() <> '10000' then Error('line %1', D.Lines."Line No.".Value());
            L.Get('D1', 10000); if L."To Bin Code" <> 'B1' then Error('stored %1', L."To Bin Code");
            if not D.ToBin.First() then Error('after'); if D.ToBin.Code.Value() <> 'B2' then Error('after %1', D.ToBin.Code.Value());
            if D.ToBin.Next() then Error('next');
            if not D.FromBin.First() then Error('other'); if D.FromBin.Code.Value() <> 'B1' then Error('other %1', D.FromBin.Code.Value());
            D.Close(); end;
        [Test] procedure WithoutFirst() var S: Codeunit "PFE Seed"; D: TestPage "PFE Document"; begin
            S.Seed(); D.OpenEdit(); D.GoToKey('D1'); D.Lines."To Bin Code".SetValue('B2');
            if D.ToBin.Code.Value() <> 'B2' then Error('read %1', D.ToBin.Code.Value()); D.Close(); end;
        [Test] procedure Blank() var S: Codeunit "PFE Seed"; D: TestPage "PFE Document"; begin
            S.Seed(); D.OpenEdit(); D.GoToKey('D1'); D.Lines."To Bin Code".SetValue('');
            if D.ToBin.First() then Error('blank first'); if D.ToBin.Code.Value() <> '' then Error('blank %1', D.ToBin.Code.Value()); D.Close(); end;
        [Test] procedure Update() var S: Codeunit "PFE Seed"; D: TestPage "PFE Document"; L: Record "PFE Line"; begin
            S.Seed(); D.OpenEdit(); D.GoToKey('D1');
            if D.FromBin.Code.Value() <> 'B1' then Error('before %1', D.FromBin.Code.Value());
            D.Lines."From Bin Code".SetValue('B2');
            L.Get('D1', 10000); if L."From Bin Code" <> 'B2' then Error('stored %1', L."From Bin Code");
            if D.Lines."Line No.".Value() <> '10000' then Error('line %1', D.Lines."Line No.".Value());
            if not D.FromBin.First() then Error('after'); if D.FromBin.Code.Value() <> 'B2' then Error('after %1', D.FromBin.Code.Value());
            D.Close(); end;
        [Test] procedure MoveAndBack() var S: Codeunit "PFE Seed"; D: TestPage "PFE Document"; begin
            S.Seed(); D.OpenEdit(); D.GoToKey('D1'); D.Lines."To Bin Code".SetValue('B2');
            D.Lines.Next(); if D.ToBin.Code.Value() <> 'B2' then Error('next %1', D.ToBin.Code.Value());
            D.Lines."To Bin Code".SetValue('B1'); if D.ToBin.Code.Value() <> 'B1' then Error('typed %1', D.ToBin.Code.Value());
            D.Lines.Previous();
            if D.Lines."To Bin Code".Value() <> 'B2' then Error('provider %1', D.Lines."To Bin Code".Value());
            if D.ToBin.Code.Value() <> 'B2' then Error('back %1', D.ToBin.Code.Value()); D.Close(); end;"#,
    );
}
