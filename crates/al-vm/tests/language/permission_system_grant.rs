//! al-runner issue 6717: a named `system` grant (`system "Tools, Security, Roles" = X`) in a permission set. A permission
//! row is keyed on (object type, object id) (ADR-062 pin 10: `page * = X` is one Metadata Permission row of type Page), so
//! a System row grants Execute on that system object and nothing on table data. The runner has no id for a named system
//! object, so only what that row decides (a System Execute access) stays unknown; the set's other grants and the sets
//! it includes compose.

use al_testkit::{Built, compile};

const SRC: &str = r#"
dotnet
{
    assembly("Microsoft.Dynamics.Nav.PermissionTestHelper")
    {
        type("Microsoft.Dynamics.Nav.Runtime.PermissionTestHelper"; "PermissionTestHelper") { }
    }
}

table 50100 "PS Row"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }
}

permissionset 50100 "PS System"
{
    Assignable = true;
    Permissions = system "Tools, Security, Roles" = X,
                  tabledata "PS Row" = R;
}

permissionset 50101 "PS Outer"
{
    Assignable = true;
    IncludedPermissionSets = "PS Inner";
    Permissions = system "Tools, Security, Roles" = X;
}

permissionset 50102 "PS Inner"
{
    Assignable = false;
    Permissions = tabledata "PS Row" = R;
}

permissionset 50103 "PS Outer Write"
{
    Assignable = true;
    IncludedPermissionSets = "PS Inner Write";
    Permissions = system "Tools, Security, Roles" = X;
}

permissionset 50104 "PS Inner Write"
{
    Assignable = false;
    Permissions = tabledata "PS Row" = RIMD;
}

permissionset 50105 "PS Full Excl System"
{
    Assignable = true;
    Permissions = tabledata "PS Row" = RIMD;
    ExcludedPermissionSets = "PS System";
}

permissionset 50106 "PS Unknown Grant"
{
    Assignable = false;
    Permissions = tabledata "PS Missing" = R;
}

permissionset 50107 "PS Full Excl Unknown"
{
    Assignable = true;
    Permissions = tabledata "PS Row" = RIMD;
    ExcludedPermissionSets = "PS Unknown Grant";
}

codeunit 50100 "PS Probe"
{
    procedure InsertUnder(SetId: Code[20]): Text
    var
        H: DotNet PermissionTestHelper;
        Row: Record "PS Row";
    begin
        H := H.PermissionTestHelper();
        H.AddEffectivePermissionSet(SetId);
        Row.Id := 1;
        Row.Insert();
        exit('inserted');
    end;
}
"#;

fn insert_under(set: &str) -> Result<al_builtins::value::Value, al_builtins::native::Signal> {
    let mut b = Built::new(compile(SRC));
    let (oi, ri) = b.routine_in("PS Probe", "InsertUnder");
    b.call(oi, ri, vec![al_builtins::value::Value::Code(set.into())]).map(|(v, _)| v)
}

/// The set's tabledata grant composes: the Insert it does not grant raises BC's error (TestMockBasics.al:72-73).
#[test]
fn a_named_system_grant_leaves_the_sets_tabledata_grants_composed() {
    let msg = al_testkit::error_message(insert_under("PS SYSTEM"));
    assert!(
        msg.starts_with("Sorry, the current permissions prevented the action. (TableData 50100 PS Row Insert"),
        "{msg}"
    );
}

/// The included set composes too, whether it denies or grants the write.
#[test]
fn a_named_system_grant_keeps_the_included_sets() {
    let msg = al_testkit::error_message(insert_under("PS OUTER"));
    assert!(
        msg.starts_with("Sorry, the current permissions prevented the action. (TableData 50100 PS Row Insert"),
        "{msg}"
    );
    assert_eq!(al_testkit::text(insert_under("PS OUTER WRITE")), "inserted");
}

/// An excluded set with a named system grant takes its tabledata grants away as any excluded set does (Full minus R
/// keeps Insert, CpPermLeads.al:191-212).
#[test]
fn an_excluded_set_with_a_named_system_grant_takes_its_tabledata_grants() {
    assert_eq!(al_testkit::text(insert_under("PS FULL EXCL SYSTEM")), "inserted");
}

/// An excluded set whose grants are not all read would take away too little: nothing of the excluding set is kept, and
/// a denial is refused, not raised.
#[test]
fn an_excluded_set_that_is_not_read_in_full_leaves_the_set_unknown() {
    let why = match insert_under("PS FULL EXCL UNKNOWN") {
        Err(al_builtins::native::Signal::Unsupported(why)) => why,
        other => panic!("{other:?}"),
    };
    assert!(why.contains("not fully composed"), "{why}");
}
