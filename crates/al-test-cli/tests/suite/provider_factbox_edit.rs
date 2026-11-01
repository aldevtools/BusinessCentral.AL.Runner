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

const INDIRECT: &str = r#"
table 62580 "PFI Header" { fields { field(1; "No."; Code[20]) { } } keys { key(PK; "No.") { } } }
table 62581 "PFI Line" { fields { field(1; "Document No."; Code[20]) { } field(2; "Line No."; Integer) { }
    field(3; Quantity; Decimal) { trigger OnValidate() begin "Qty. (Base)" := Quantity * 2; end; }
    field(4; "Qty. (Base)"; Decimal) { } field(5; "Page Qty"; Decimal) { } } keys { key(PK; "Document No.", "Line No.") { } } }
table 62582 "PFI Bucket" { fields { field(1; Qty; Decimal) { } field(2; Label; Text[30]) { } } keys { key(PK; Qty) { } } }
page 62583 "PFI Document" { PageType = Document; SourceTable = "PFI Header";
    layout { area(Content) { field("No."; Rec."No.") { } part(Lines; "PFI Lines") { SubPageLink = "Document No." = field("No."); } }
        area(FactBoxes) { part(Bucket; "PFI Buckets") { Provider = Lines; SubPageLink = Qty = field("Qty. (Base)"); } } } }
page 62584 "PFI Lines" { PageType = ListPart; SourceTable = "PFI Line"; InsertAllowed = false;
    layout { area(Content) { repeater(Rows) { field("Line No."; Rec."Line No.") { } field(Quantity; Rec.Quantity) { }
        field("Page Qty"; Rec."Page Qty") { trigger OnValidate() begin Rec."Qty. (Base)" := Rec."Page Qty" * 3; end; }
        field("Qty. (Base)"; Rec."Qty. (Base)") { Editable = false; } } } } }
page 62585 "PFI Buckets" { PageType = ListPart; SourceTable = "PFI Bucket"; Editable = false;
    layout { area(Content) { repeater(Rows) { field(Label; Rec.Label) { } } } } }
codeunit 62586 "PFI Seed" {
    procedure Seed() var H: Record "PFI Header"; L: Record "PFI Line"; B: Record "PFI Bucket"; begin
        H.DeleteAll(); L.DeleteAll(); B.DeleteAll();
        B.Qty := 2; B.Label := 'base 2'; B.Insert(); B.Qty := 4; B.Label := 'base 4'; B.Insert(); B.Qty := 9; B.Label := 'base 9'; B.Insert();
        H."No." := 'D1'; H.Insert();
        L."Document No." := 'D1'; L."Line No." := 10000; L.Quantity := 1; L."Qty. (Base)" := 2; L.Insert(); end; }
"#;

// The linked field changes through another field's OnValidate, of the table field or of the page control: the FactBox
// follows at once (corpus writer Y5, branch al-runtime-rs-staging-y5b, pages/lt-y5/Y5ProviderIndirectLink.al:157 table
// OnValidate `base 4`, :191 page control OnValidate `base 9`, :221 a value with no matching row: First No, Label '').
#[test]
fn a_provider_factbox_follows_a_linked_field_changed_through_validation() {
    let src = format!(
        "{INDIRECT}\ncodeunit 62590 T {{ Subtype = Test; {} }}",
        r#"[Test] procedure TableValidate() var S: Codeunit "PFI Seed"; D: TestPage "PFI Document"; L: Record "PFI Line"; begin
            S.Seed(); D.OpenEdit(); D.GoToKey('D1');
            if not D.Bucket.First() then Error('before'); if D.Bucket.Label.Value() <> 'base 2' then Error('before %1', D.Bucket.Label.Value());
            D.Lines.Quantity.SetValue(2);
            if D.Lines."Line No.".Value() <> '10000' then Error('line'); if D.Lines."Qty. (Base)".Value() <> '4.00' then Error('provider %1', D.Lines."Qty. (Base)".Value());
            L.Get('D1', 10000); if L."Qty. (Base)" <> 2 then Error('stored %1', L."Qty. (Base)");
            if D.Bucket.Label.Value() <> 'base 4' then Error('after %1', D.Bucket.Label.Value());
            if not D.Bucket.First() then Error('after first'); if D.Bucket.Label.Value() <> 'base 4' then Error('after first %1', D.Bucket.Label.Value());
            D.Close(); end;
        [Test] procedure PageValidate() var S: Codeunit "PFI Seed"; D: TestPage "PFI Document"; L: Record "PFI Line"; begin
            S.Seed(); D.OpenEdit(); D.GoToKey('D1');
            if D.Bucket.Label.Value() <> 'base 2' then Error('before %1', D.Bucket.Label.Value());
            D.Lines."Page Qty".SetValue(3);
            if D.Lines."Qty. (Base)".Value() <> '9.00' then Error('provider %1', D.Lines."Qty. (Base)".Value());
            L.Get('D1', 10000); if L."Qty. (Base)" <> 2 then Error('stored %1', L."Qty. (Base)");
            if D.Bucket.Label.Value() <> 'base 9' then Error('after %1', D.Bucket.Label.Value());
            if not D.Bucket.First() then Error('after first'); if D.Bucket.Label.Value() <> 'base 9' then Error('after first %1', D.Bucket.Label.Value());
            D.Close(); end;
        [Test] procedure NoBucket() var S: Codeunit "PFI Seed"; D: TestPage "PFI Document"; begin
            S.Seed(); D.OpenEdit(); D.GoToKey('D1');
            D.Lines.Quantity.SetValue(5);
            if D.Lines."Qty. (Base)".Value() <> '10.00' then Error('provider %1', D.Lines."Qty. (Base)".Value());
            if D.Bucket.First() then Error('first'); if D.Bucket.Label.Value() <> '' then Error('label %1', D.Bucket.Label.Value());
            D.Close(); end;"#
    );
    let o = al_testkit::outcomes_in(&src, "T");
    assert!(!o.is_empty());
    let bad: Vec<_> = o.into_iter().filter(|(_, r)| *r != Outcome::Pass).collect();
    assert!(bad.is_empty(), "{bad:?}");
}
