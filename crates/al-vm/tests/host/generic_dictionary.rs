//! `System.Collections.Generic.Dictionary`2` as Base Application's ODataUtility.GenerateFilterText uses it: a new
//! dictionary of the open generic type, filled by GetNAVFilters only from Tenant Web Service Filter rows, then walked by
//! `foreach` in CombineFiltersFromTables. With no filter rows the walk finds nothing and the filter text is ''
//! (tests/al-language-onprem-dotnet/lt-qx-dotnet/QxDotNetDemoShapes.al:473, al-language-onprem/platform/PtWebServiceRows.al:326).
//! The AL below is my own minimal version of that sequence.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::text;

const SRC: &str = r#"
dotnet { assembly("mscorlib") {
    type("System.Collections.Generic.Dictionary`2"; GenericDictionary2) { }
    type("System.Collections.Generic.KeyValuePair`2"; GenericKeyValuePair2) { }
} }
codeunit 50100 C
{
    local procedure Combine(D: DotNet GenericDictionary2): Text
    var KeyValuePair: DotNet GenericKeyValuePair2; F: Text;
    begin
        foreach KeyValuePair in D do
            F += 'x';
        exit(F);
    end;
    local procedure Walk(D: DotNet GenericDictionary2): Text
    var Kvp: DotNet GenericKeyValuePair2; A: Text; B: Text;
    begin
        foreach Kvp in D do begin
            A += StrSubstNo('Table%1', Kvp.Key) + '=' + Format(Kvp.Value) + ';';
            B += Format(Kvp.Key) + '|' + StrSubstNo('%1', Kvp.Value) + ';';
        end;
        exit(A + B);
    end;
    procedure F(): Text
    var D: DotNet GenericDictionary2; T: Text;
    begin
        __BODY__
    end;
}
"#;

fn run(body: &str) -> Result<Value, Signal> {
    let src = SRC.replace("__BODY__", body);
    al_testkit::run_in(&src, "C", "F")
}

#[test]
fn a_new_dictionary_walked_by_foreach_finds_nothing() {
    let r = run("D := D.Dictionary(); T := Combine(D); exit('[' + T + ']');");
    assert_eq!(text(r), "[]");
}

// The shape of RequestPageParametersHelper.GetViewFromDynamicRequestPage: a dictionary made with a capacity, filled with
// ContainsKey/Add, walked with foreach into a KeyValuePair (tests/al-language-onprem-dotnet/lt-y17-dotnet-dictionary/
// Y17DotNetDictionary.al:56 DnDict_Capacity_AddContainsForeach).
const FILL: &str = "D := D.Dictionary(3); \
    if not D.ContainsKey(18) then D.Add(18, 'view18'); \
    if not D.ContainsKey(36) then D.Add(36, 'view36'); \
    if not D.ContainsKey(18) then D.Add(18, 'again'); \
    if not D.ContainsKey(5) then D.Add(5, 'view5');";

#[test]
fn a_dictionary_with_a_capacity_walks_its_entries_in_insertion_order() {
    let r = run(&format!("{FILL} exit(Walk(D) + '|' + Format(D.Count));"));
    assert_eq!(
        text(r),
        "Table18=view18;Table36=view36;Table5=view5;18|view18;36|view36;5|view5;|3"
    );
}

#[test]
fn item_and_contains_key_read_the_entries() {
    let r = run(&format!(
        "{FILL} exit(Format(D.Item(36)) + '|' + Format(D.ContainsKey(5)) + '|' + Format(D.ContainsKey(99)));"
    ));
    assert_eq!(text(r), "view36|Yes|No");
}

#[test]
fn a_dictionary_with_capacity_zero_is_empty() {
    let r = run("D := D.Dictionary(0); exit('[' + Walk(D) + ']' + Format(D.Count));");
    assert_eq!(text(r), "[]0");
}

#[test]
fn a_duplicate_add_raises_and_keeps_the_entries() {
    // Y17DotNetDictionary.al:117 DnDict_DuplicateAdd.
    let r = run(&format!(
        "{FILL} asserterror D.Add(18, 'x'); exit(GetLastErrorText() + '|' + GetLastErrorCode() + '|' + Format(D.Count) + '|' + Format(D.Item(18)));"
    ));
    assert_eq!(
        text(r),
        "A call to System.Collections.Generic.Dictionary`2[System.Object,System.Object].Add failed with this message: \
         An item with the same key has already been added. Key: 18|DotNetInvoke:Argument|3|view18"
    );
}

#[test]
fn item_of_a_missing_key_raises() {
    // Y17DotNetDictionary.al:138 DnDict_Item_Missing.
    let r = run(&format!(
        "{FILL} asserterror T := Format(D.Item(99)); exit(GetLastErrorText() + '|' + GetLastErrorCode());"
    ));
    assert_eq!(
        text(r),
        "A call to System.Collections.Generic.Dictionary`2[System.Object,System.Object].Item failed with this message: \
         The given key '99' was not present in the dictionary.|DotNetInvoke:KeyNotFound"
    );
}

#[test]
fn a_negative_capacity_raises() {
    // Y17DotNetDictionary.al:154 DnDict_Capacity_Negative.
    let r = run("asserterror D := D.Dictionary(-1); exit(GetLastErrorText() + '|' + GetLastErrorCode());");
    assert_eq!(
        text(r),
        "A call to System.Collections.Generic.Dictionary`2 failed with this message: \
         Specified argument was out of the range of valid values. (Parameter 'capacity')|DotNetInvoke:ArgumentOutOfRange"
    );
}

#[test]
fn a_key_that_is_not_an_integer_is_refused() {
    // Only boxed Int32 keys are pinned: how other kinds compare is not.
    let r = run("D := D.Dictionary(1); D.Add('k', 'x'); exit('');");
    assert!(matches!(r, Err(Signal::Unsupported(_))), "{r:?}");
}

#[test]
fn a_value_that_is_not_a_text_is_refused() {
    let r = run("D := D.Dictionary(1); D.Add(1, 2); exit('');");
    assert!(matches!(r, Err(Signal::Unsupported(_))), "{r:?}");
}

#[test]
fn add_to_a_dictionary_without_a_capacity_is_refused() {
    // Only the capacity constructor is pinned with entries (Y17DotNetDictionary.al:36).
    let r = run("D := D.Dictionary(); D.Add(1, 'x'); exit('');");
    assert!(matches!(r, Err(Signal::Unsupported(_))), "{r:?}");
}

#[test]
fn a_member_that_is_not_pinned_is_refused() {
    let r = run("D := D.Dictionary(1); D.Remove(1); exit('');");
    assert!(matches!(r, Err(Signal::Unsupported(_))), "{r:?}");
}
