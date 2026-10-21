//! `FieldRef.Value` of a Blob field of a stored row read without CalcFields, assigned to another Blob. Temp Blob Impl.
//! FromFieldRef does this and then asks HasValue to decide whether to CalcField. The runtime's copy said HasValue true and its
//! stream read nothing, which no BC run can give: Microsoft's Tests-ERM 134112 TestMultipleLinesText (Test Sales Header Work
//! Descr.) passes on BC and failed here. What the copy holds is not pinned; Temp Blob Impl. needs it to be empty, which is the
//! registered assumption fieldref-value-of-unloaded-blob-is-empty (561 Tests-ERM tests reach it).

use al_testkit::{run, text};

const SRC: &str = r#"
table 50100 "Blob T"
{
    fields { field(1; K; Integer) { } field(2; B; Blob) { } }
    keys { key(PK; K) { } }
}
codeunit 50101 "Blob Probe"
{
    procedure ValueOfUnloadedBlob(): Text
    var
        T: Record "Blob T";
        Other: Record "Blob T" temporary;
        RecRef: RecordRef;
        FldRef: FieldRef;
        OutS: OutStream;
    begin
        T.K := 1;
        T.B.CreateOutStream(OutS);
        OutS.WriteText('Hello');
        T.Insert();
        T.Get(1);
        RecRef.GetTable(T);
        FldRef := RecRef.Field(2);
        Other.B := FldRef.Value();
        exit(Format(Other.B.HasValue()));
    end;

    procedure ValueOfLoadedBlob(): Text
    var
        T: Record "Blob T";
        Other: Record "Blob T" temporary;
        RecRef: RecordRef;
        FldRef: FieldRef;
        OutS: OutStream;
        InS: InStream;
        S: Text;
    begin
        T.K := 1;
        T.B.CreateOutStream(OutS);
        OutS.WriteText('Hello');
        T.Insert();
        T.Get(1);
        T.CalcFields(B);
        RecRef.GetTable(T);
        FldRef := RecRef.Field(2);
        Other.B := FldRef.Value();
        Other.B.CreateInStream(InS);
        InS.ReadText(S);
        exit(S);
    end;
}
"#;

#[test]
fn fieldref_value_of_a_blob_read_without_calcfields_is_empty() {
    assert_eq!(text(run(SRC, "ValueOfUnloadedBlob")), "No");
}

/// After CalcFields the field holds the content and FieldRef.Value hands it on.
#[test]
fn fieldref_value_of_a_loaded_blob_runs() {
    assert_eq!(text(run(SRC, "ValueOfLoadedBlob")), "Hello");
}
