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

#[test]
fn add_to_the_dictionary_is_refused() {
    // GetNAVFilters adds a Tenant Web Service Filter's text: no corpus test has a filter row.
    let r = run("D := D.Dictionary(); D.Add(1, 'x'); exit('');");
    assert!(matches!(r, Err(Signal::Unsupported(_))), "{r:?}");
}

#[test]
fn count_of_the_dictionary_is_refused() {
    let r = run("D := D.Dictionary(); exit(Format(D.Count()));");
    assert!(matches!(r, Err(Signal::Unsupported(_))), "{r:?}");
}
