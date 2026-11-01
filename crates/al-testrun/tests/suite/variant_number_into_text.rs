//! A Variant variable holding a number into a record's Text or Code field raises TypeConversion (corpus types/lt-variant-to-text-fields/
//! VariantNumberToField.al); a call result as below, into a record's Text field it becomes its Format text (assumption number-to-text-is-format; Base
//! Application's Workflow Table Relation Value.CreateNew, `Value := FieldRef.Value()`); into a Text variable an Integer raises
//! TypeConversion (corpus types/lt-variant/VaTCompare.al:Concat_After_Conversion, af117).

use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun};
use al_testrun::{CaseId, Outcome};

const SRC: &str = r#"
enum 50500 "VNT Kind" { value(0; Alpha) { } value(1; Beta) { } }
table 50500 "VNT Row" { fields { field(1; Id; Integer) { } field(2; Txt; Text[50]) { } field(3; Cd; Code[20]) { } } keys { key(PK; Id) { Clustered = true; } } }
codeunit 50500 "T"
{
    Subtype = Test;
    [Test]
    procedure IntoAField()
    var V: Variant; Row: Record "VNT Row"; B: BigInteger;
    begin
        B := 2645; V := B;
        asserterror Row.Txt := V;
        if GetLastErrorCode() <> 'TypeConversion' then Error('code %1', GetLastErrorCode());
        asserterror Row.Cd := V;
        if GetLastErrorCode() <> 'TypeConversion' then Error('code %1', GetLastErrorCode());
    end;
    [Test]
    procedure FromAFieldRef()
    var Row: Record "VNT Row"; Dst: Record "VNT Row"; RR: RecordRef; FR: FieldRef;
    begin
        Row.Id := 2645; Row.Insert();
        RR.GetTable(Row);
        FR := RR.Field(1);
        Dst.Txt := FR.Value();
        if Dst.Txt <> '2645' then Error('text %1', Dst.Txt);
    end;
    [Test]
    procedure AnEnumVariantIntoAField()
    var V: Variant; Row: Record "VNT Row"; K: Enum "VNT Kind";
    begin
        K := K::Beta; V := K;
        asserterror Row.Txt := V;
        if GetLastErrorCode() <> 'TypeConversion' then Error('code %1', GetLastErrorCode());
    end;
    [Test]
    procedure IntoAVariable()
    var V: Variant; T: Text;
    begin
        V := 5;
        asserterror T := V;
        if GetLastErrorCode() <> 'TypeConversion' then Error('code %1', GetLastErrorCode());
    end;
}
"#;

#[test]
fn a_field_ref_number_becomes_text_in_a_field_and_a_variant_number_raises() {
    let rt = CorpusRuntime::run_suites_with(
        vec![AppSource {
            id: "main".into(),
            name: "main".into(),
            dependencies: vec![],
            files: vec![("main.al".into(), SRC.into())],
        }],
        vec![al_sema::AppKind::Workspace],
        &[],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions::default(),
    );
    for name in [
        "IntoAField",
        "FromAFieldRef",
        "AnEnumVariantIntoAField",
        "IntoAVariable",
    ] {
        let o = rt.first_outcome(&CaseId {
            codeunit: 50500,
            name: name.into(),
        });
        assert_eq!(o, Outcome::Pass, "{name}: {o:?}");
    }
}
