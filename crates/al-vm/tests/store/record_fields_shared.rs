//! Issue 8570: a fresh record's field buffer may be shared with the other fresh records of its table until its first
//! write (`Fields::Shared`). Sharing is invisible to AL, so these tests pin that every value a fresh record shows, and
//! every write through every mutation path, is what it was before sharing (green guards), and that sharing happens only
//! where the design allows it (`is_shared`: a table without a Blob or a cleared InitValue). Expected values are
//! characterized on the unshared code (step 1); the AL shapes follow record/ACRecordIsDirty.al,
//! record/PWTableGlobals.al:196-260 and install-upgrade/install-order/Y8InstallOrderTests.al:88-110.

use al_builtins::value::Value;
use al_testkit::{AppSource, Built, compile_apps, text};

const ALL_TYPES: &str = r#"
enum 50200 "Rf Color" { value(0; Red) { } value(1; Blue) { } }
table 50201 "Rf Detail"
{
    fields { field(1; K; Integer) { } field(2; Amt; Decimal) { } }
    keys { key(PK; K) { } }
}
table 50202 "Rf All"
{
    fields
    {
        field(1; K; Integer) { }
        field(2; Flag; Boolean) { }
        field(3; Big; BigInteger) { }
        field(4; Dec; Decimal) { }
        field(5; Txt; Text[20]) { }
        field(6; Cod; Code[10]) { }
        field(7; Dt; Date) { }
        field(8; Tm; Time) { }
        field(9; Dtm; DateTime) { }
        field(10; Dur; Duration) { }
        field(11; G; Guid) { }
        field(12; Df; DateFormula) { }
        field(13; Opt; Option) { OptionMembers = A,B,C; }
        field(14; En; Enum "Rf Color") { }
        field(15; Bl; Blob) { }
        field(16; Med; Media) { }
        field(17; MedSet; MediaSet) { }
        field(18; RId; RecordId) { }
        field(19; Flow; Decimal) { FieldClass = FlowField; CalcFormula = sum("Rf Detail".Amt where(K = field(K))); }
        field(20; IInt; Integer) { InitValue = 5; }
        field(21; IDec; Decimal) { InitValue = 2.5; }
        field(22; IBool; Boolean) { InitValue = true; }
        field(23; ITxt; Text[20]) { InitValue = 'hello'; }
        field(24; IOpt; Option) { OptionMembers = A,B,C; InitValue = B; }
        field(25; IEn; Enum "Rf Color") { InitValue = Blue; }
    }
    keys { key(PK; K) { } }
}
codeunit 50203 C
{
    local procedure Dump(var R: Record "Rf All"): Text
    begin
        R.CalcFields(Flow);
        exit(
            Format(R.K) + '|' + Format(R.Flag) + '|' + Format(R.Big) + '|' + Format(R.Dec) + '|' + R.Txt + '|' + R.Cod + '|' +
            Format(R.Dt) + '|' + Format(R.Tm) + '|' + Format(R.Dtm) + '|' + Format(R.Dur) + '|' + Format(R.G) + '|' +
            Format(R.Df) + '|' + Format(R.Opt) + '|' + Format(R.En) + '|' + Format(R.Bl.HasValue()) + '|' +
            Format(R.Med.HasValue()) + '|' + Format(R.MedSet.Count()) + '|' + Format(R.RId) + '|' + Format(R.Flow) + '|' +
            Format(R.IInt) + '|' + Format(R.IDec) + '|' + Format(R.IBool) + '|' + R.ITxt + '|' + Format(R.IOpt) + '|' +
            Format(R.IEn));
    end;

    local procedure Dirty(var R: Record "Rf All")
    begin
        R.K := 9; R.Flag := true; R.Big := 99; R.Dec := 1.5; R.Txt := 'x'; R.Cod := 'Y'; R.Dt := 20260101D; R.IInt := 1;
        R.IDec := 1; R.IBool := false; R.ITxt := 'z'; R.IOpt := R.IOpt::C; R.IEn := R.IEn::Red;
    end;

    procedure Fresh(): Text
    var
        R: Record "Rf All";
    begin
        exit(Dump(R));
    end;

    procedure AfterInit(): Text
    var
        R: Record "Rf All";
    begin
        Dirty(R);
        R.Init();
        exit(Dump(R));
    end;

    procedure AfterClear(): Text
    var
        R: Record "Rf All";
    begin
        Dirty(R);
        Clear(R);
        exit(Dump(R));
    end;

    procedure Reused(): Text
    var
        A: Record "Rf All";
        B: Record "Rf All";
    begin
        Dirty(A);
        exit(Dump(B));
    end;

    local procedure Write(var R: Record "Rf All")
    var
        OutS: OutStream;
    begin
        R.Bl.CreateOutStream(OutS);
        OutS.WriteText('a');
    end;

    procedure BlobDistinct(): Text
    var
        A: Record "Rf All";
        B: Record "Rf All";
    begin
        Write(A);
        exit(Format(A.Bl.HasValue()) + Format(B.Bl.HasValue()));
    end;

    procedure BlobDistinctAfterInit(): Text
    var
        A: Record "Rf All";
        B: Record "Rf All";
    begin
        A.Init();
        B.Init();
        Write(B);
        exit(Format(A.Bl.HasValue()) + Format(B.Bl.HasValue()));
    end;

    procedure BlobDistinctAfterClear(): Text
    var
        A: Record "Rf All";
        B: Record "Rf All";
    begin
        Clear(A);
        Clear(B);
        Write(A);
        exit(Format(A.Bl.HasValue()) + Format(B.Bl.HasValue()));
    end;
}
"#;

fn run(src: &str, routine: &str) -> String {
    text(al_testkit::run_in(src, "C", routine))
}

// Characterized on the unshared code (step 1). Every field of a fresh variable, in field order.
const FRESH: &str = "0|No|0|0|||||||{00000000-0000-0000-0000-000000000000}||A|Red|No|No|0||0|5|2.5|Yes|hello|B|Blue";

#[test]
fn a_fresh_record_shows_the_default_or_init_value_of_every_field_type() {
    assert_eq!(run(ALL_TYPES, "Fresh"), FRESH);
}

#[test]
fn init_gives_the_fresh_values_again() {
    // Init keeps the primary key field K (Dirty set it to 9).
    assert_eq!(run(ALL_TYPES, "AfterInit"), FRESH.replacen('0', "9", 1));
}

#[test]
fn clear_gives_the_fresh_values_again() {
    assert_eq!(run(ALL_TYPES, "AfterClear"), FRESH);
}

#[test]
fn a_write_to_one_variable_does_not_show_in_a_fresh_one() {
    assert_eq!(run(ALL_TYPES, "Reused"), FRESH);
}

#[test]
fn blob_slots_are_distinct_per_record() {
    assert_eq!(run(ALL_TYPES, "BlobDistinct"), "YesNo");
    assert_eq!(run(ALL_TYPES, "BlobDistinctAfterInit"), "NoYes");
    assert_eq!(run(ALL_TYPES, "BlobDistinctAfterClear"), "YesNo");
}

// A table with no Blob and no cleared InitValue: its fresh records share one buffer until their first write.
const PLAIN: &str = r#"
table 50210 "Rf Line"
{
    fields { field(1; K; Integer) { } field(2; Amt; Decimal) { } }
    keys { key(PK; K) { } }
}
table 50211 "Rf Plain"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Name; Text[30]) { }
        field(3; Qty; Integer) { InitValue = 3; }
        field(4; Amount; Decimal) { }
        field(5; Total; Decimal) { FieldClass = FlowField; CalcFormula = sum("Rf Line".Amt where(K = field(Id))); }
    }
    keys { key(PK; Id) { } }

    procedure Touch(): Text
    begin
        Rec.Qty := 11;
        exit(Format(Rec.Qty) + '/' + Format(xRec.Qty));
    end;

    procedure Report(): Text
    begin
        exit(Format(Rec.Id) + ',' + Name + ',' + Format(Qty) + ',' + Format(Amount) + ',' + Format(Total));
    end;
}
codeunit 50212 C
{
    // "A=...|B=..." of two variables.
    local procedure Both(var A: Record "Rf Plain"; var B: Record "Rf Plain"): Text
    begin
        exit('A=' + A.Report() + '|B=' + B.Report());
    end;

    procedure Assign(): Text
    var
        A: Record "Rf Plain";
        B: Record "Rf Plain";
    begin
        A.Name := 'a';
        A.Qty := 4;
        exit(Both(A, B));
    end;

    procedure AppendText(): Text
    var
        A: Record "Rf Plain";
        B: Record "Rf Plain";
    begin
        A.Name += 'ab';
        A.Name += 'cd';
        exit(Both(A, B));
    end;

    procedure ValidateField(): Text
    var
        A: Record "Rf Plain";
        B: Record "Rf Plain";
    begin
        A.Validate(Qty, 8);
        exit(Both(A, B));
    end;

    procedure EvaluateField(): Text
    var
        A: Record "Rf Plain";
        B: Record "Rf Plain";
    begin
        Evaluate(A.Qty, '7');
        exit(Both(A, B));
    end;

    procedure FieldRefValue(): Text
    var
        A: Record "Rf Plain";
        B: Record "Rf Plain";
        RR: RecordRef;
        FR: FieldRef;
    begin
        RR.GetTable(A);
        FR := RR.Field(3);
        FR.Value := 6;
        RR.SetTable(A);
        exit(Both(A, B));
    end;

    procedure FieldRefValidate(): Text
    var
        A: Record "Rf Plain";
        B: Record "Rf Plain";
        RR: RecordRef;
        FR: FieldRef;
    begin
        RR.GetTable(A);
        FR := RR.Field(3);
        FR.Validate(9);
        RR.SetTable(A);
        exit(Both(A, B));
    end;

    procedure EvaluateIntoFieldRef(): Text
    var
        A: Record "Rf Plain";
        B: Record "Rf Plain";
        RR: RecordRef;
        FR: FieldRef;
        Q: Integer;
    begin
        RR.GetTable(A);
        FR := RR.Field(3);
        Evaluate(Q, '12');
        FR.Value := Q;
        RR.SetTable(A);
        exit(Both(A, B));
    end;

    procedure InitIt(): Text
    var
        A: Record "Rf Plain";
        B: Record "Rf Plain";
    begin
        A.Init();
        A.Name := 'after init';
        exit(Both(A, B));
    end;

    procedure InitTemporary(): Text
    var
        A: Record "Rf Plain" temporary;
        B: Record "Rf Plain" temporary;
    begin
        A.Init();
        A.Id := 1;
        A.Insert();
        exit(Both(A, B));
    end;

    procedure ClearIt(): Text
    var
        A: Record "Rf Plain";
        B: Record "Rf Plain";
    begin
        Clear(A);
        A.Qty := 2;
        Clear(A);
        exit(Both(A, B));
    end;

    procedure TransferFromOther(): Text
    var
        A: Record "Rf Plain";
        B: Record "Rf Plain";
        C: Record "Rf Plain";
    begin
        C.Id := 5; C.Name := 'c'; C.Qty := 6; C.Amount := 7;
        A.TransferFields(C);
        exit(Both(A, B) + '|C=' + C.Report());
    end;

    procedure CopyFromOther(): Text
    var
        A: Record "Rf Plain";
        B: Record "Rf Plain";
        C: Record "Rf Plain";
    begin
        C.Name := 'c'; C.Qty := 6;
        A.Copy(C);
        A.Qty := 1;
        exit(Both(A, B) + '|C=' + C.Report());
    end;

    procedure AssignRecord(): Text
    var
        A: Record "Rf Plain";
        B: Record "Rf Plain";
        C: Record "Rf Plain";
    begin
        A := C;
        A.Qty := 2;
        exit(Both(A, B) + '|C=' + C.Report());
    end;

    procedure CalcFlow(): Text
    var
        A: Record "Rf Plain";
        B: Record "Rf Plain";
        L: Record "Rf Line";
    begin
        L.K := 0; L.Amt := 3.5; L.Insert();
        A.CalcFields(Total);
        exit(Both(A, B));
    end;

    procedure InsertStampsSystemId(): Text
    var
        A: Record "Rf Plain";
        B: Record "Rf Plain";
    begin
        A.Id := 1;
        A.Insert();
        exit(Format(IsNullGuid(A.SystemId)) + Format(IsNullGuid(B.SystemId)));
    end;

    procedure IsDirtyAfterWrite(): Text
    var
        A: Record "Rf Plain";
        B: Record "Rf Plain";
        S: Text;
    begin
        S := Format(A.IsDirty()) + Format(B.IsDirty());
        A.Qty := 1;
        exit(S + '|' + Format(A.IsDirty()) + Format(B.IsDirty()) + '|' + Format(B.Qty));
    end;

    procedure BlankXRec(): Text
    var
        A: Record "Rf Plain";
        B: Record "Rf Plain";
    begin
        exit(A.Touch() + ';' + B.Touch() + ';' + Format(B.Qty));
    end;
}
"#;

fn plain(routine: &str) -> String {
    run_c(PLAIN, routine)
}

fn run_c(src: &str, routine: &str) -> String {
    text(al_testkit::run_in(src, "C", routine))
}

// The default of the other variable, "B=...": Qty carries the InitValue 3.
const B_FRESH: &str = "B=0,,3,0,0";

macro_rules! paths {
    ($($name:ident: $routine:literal => $a:literal;)*) => {$(
        #[test]
        fn $name() {
            assert_eq!(plain($routine), format!("A={}|{}", $a, B_FRESH));
        }
    )*};
}

paths! {
    assignment_changes_only_that_variable: "Assign" => "0,a,4,0,0";
    text_append_changes_only_that_variable: "AppendText" => "0,abcd,3,0,0";
    validate_changes_only_that_variable: "ValidateField" => "0,,8,0,0";
    evaluate_into_a_field_changes_only_that_variable: "EvaluateField" => "0,,7,0,0";
    fieldref_value_changes_only_that_variable: "FieldRefValue" => "0,,6,0,0";
    fieldref_validate_changes_only_that_variable: "FieldRefValidate" => "0,,9,0,0";
    evaluate_into_a_fieldref_changes_only_that_variable: "EvaluateIntoFieldRef" => "0,,12,0,0";
    init_changes_only_that_variable: "InitIt" => "0,after init,3,0,0";
    clear_changes_only_that_variable: "ClearIt" => "0,,3,0,0";
    calcfields_changes_only_that_variable: "CalcFlow" => "0,,3,0,3.5";
}

#[test]
fn init_and_insert_of_a_temporary_record_change_only_that_variable() {
    assert_eq!(plain("InitTemporary"), "A=1,,3,0,0|B=0,,3,0,0");
}

#[test]
fn transferfields_changes_only_the_target() {
    assert_eq!(plain("TransferFromOther"), "A=5,c,6,7,0|B=0,,3,0,0|C=5,c,6,7,0");
}

#[test]
fn copy_changes_only_the_target() {
    assert_eq!(plain("CopyFromOther"), "A=0,c,1,0,0|B=0,,3,0,0|C=0,c,6,0,0");
}

#[test]
fn assigning_a_record_then_writing_leaves_the_source_alone() {
    assert_eq!(plain("AssignRecord"), "A=0,,2,0,0|B=0,,3,0,0|C=0,,3,0,0");
}

#[test]
fn insert_stamps_only_the_inserted_variables_system_id() {
    assert_eq!(plain("InsertStampsSystemId"), "NoYes");
}

// record/ACRecordIsDirty.al:31-87 AC_IsDirty_Lifecycle.
#[test]
fn is_dirty_turns_true_in_the_written_variable_only() {
    assert_eq!(plain("IsDirtyAfterWrite"), "NoNo|YesNo|3");
}

// record/PWTableGlobals.al:196-260: a table procedure's xRec stays blank after Rec is written.
#[test]
fn the_blank_xrec_stays_blank_after_the_first_write() {
    assert_eq!(plain("BlankXRec"), "11/3;11/3;11");
}

// ---- Sharing: only a table with nothing fresh and nothing cleared shares its initial buffer. ----

fn table_index(b: &Built, name: &str) -> usize {
    b.lowered
        .program
        .objects
        .iter()
        .position(|o| o.name == name)
        .unwrap_or_else(|| panic!("no table {name}"))
}

fn is_shared(b: &mut Built, table: &str) -> bool {
    let idx = table_index(b, table);
    match b
        .session
        .default_value(&al_builtins::value::RtType::Record(idx as _, false))
    {
        Value::Record(r) => r.borrow().fields.is_shared(),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_fresh_record_of_a_table_without_a_blob_is_shared() {
    let mut b = Built::from_source(PLAIN);
    assert!(is_shared(&mut b, "Rf Plain"));
}

#[test]
fn a_fresh_record_of_a_table_with_a_blob_is_not_shared() {
    let mut b = Built::from_source(ALL_TYPES);
    assert!(!is_shared(&mut b, "Rf All"));
}

#[test]
fn a_blob_table_next_to_a_plain_one_does_not_change_the_plain_ones_sharing() {
    let mut b = Built::from_source(ALL_TYPES);
    assert!(is_shared(&mut b, "Rf Detail"));
}

// install-upgrade/install-order/Y8InstallOrderTests.al:88-110: the InitValue of a table extension field of an app not installed
// yet does not apply; the field holds its type's default.
const BASE_APP: &str = r#"
table 50220 "Rf Base"
{
    fields { field(1; Id; Integer) { } field(2; Own; Text[20]) { InitValue = 'own'; } }
    keys { key(PK; Id) { } }
}
table 50221 "Rf Other"
{
    fields { field(1; Id; Integer) { } field(2; Own; Text[20]) { InitValue = 'other'; } }
    keys { key(PK; Id) { } }
}
"#;

const EXT_APP: &str = r#"
tableextension 50222 "Rf Base Ext" extends "Rf Base"
{
    fields { field(50222; Mark; Text[20]) { InitValue = 'ext init'; } }
}
codeunit 50223 C
{
    procedure Read(): Text
    var
        R: Record "Rf Base";
    begin
        exit(R.Own + '/' + R.Mark);
    end;

    procedure ReadOther(): Text
    var
        R: Record "Rf Other";
    begin
        exit(R.Own);
    end;
}
"#;

fn two_apps() -> Built {
    let app = |id: &str, deps: &[&str], src: &str| AppSource {
        id: id.into(),
        name: format!("App {id}"),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files: vec![(format!("{id}.al"), src.into())],
    };
    Built::new(compile_apps(vec![app("a", &[], BASE_APP), app("m", &["a"], EXT_APP)]))
}

fn ext_app(b: &Built) -> u32 {
    let t = &b.lowered.program.objects[table_index(b, "Rf Base")];
    let field = t
        .table
        .as_ref()
        .unwrap()
        .fields
        .iter()
        .find(|f| f.name == "Mark")
        .expect("the extension field");
    b.lowered.program.objects[field.declared_in.expect("declared in the extension") as usize].app
}

#[test]
fn an_init_value_of_an_app_not_installed_yet_does_not_apply() {
    let mut b = two_apps();
    assert_eq!(text(b.run_in("C", "Read")), "own/ext init");
    b.session.set_apps_not_installed(Some([ext_app(&b)].into()));
    assert_eq!(text(b.run_in("C", "Read")), "own/");
    assert_eq!(text(b.run_in("C", "ReadOther")), "other");
    // Once the app is installed a new variable shows the InitValue again.
    b.session.set_apps_not_installed(None);
    assert_eq!(text(b.run_in("C", "Read")), "own/ext init");
}

#[test]
fn a_cleared_init_value_keeps_the_record_from_sharing() {
    let mut b = two_apps();
    assert!(is_shared(&mut b, "Rf Base"));
    b.session.set_apps_not_installed(Some([ext_app(&b)].into()));
    assert!(!is_shared(&mut b, "Rf Base"));
    // A table the extension does not touch still shares.
    assert!(is_shared(&mut b, "Rf Other"));
    b.session.set_apps_not_installed(None);
    assert!(is_shared(&mut b, "Rf Base"));
}
