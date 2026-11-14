//! A Label or TextConst variable going into a Variant (al-runner issue 7987; al-sema lower/label_variant.rs). In the pinned
//! cells the Variant holds a TextConstant: a local Label or TextConst and a global TextConst assigned to a local Variant
//! (types/TestTernaryContracts.al:564, :572; types/variant/VaTTextConst.al:287-289), and a global TextConst handed to a
//! Variant parameter of a local procedure (VaTTextConst.al:288). A global Label assigned the same way runs on the assumption
//! `label-global-into-variant` (corpus PR 1382, VariantIsMatrix.al:224). Every other shape keeps main's result: a Label is a
//! Text there, a TextConst is refused.

use al_builtins::assumptions::{Assumption, take_hits};
use al_builtins::native::Signal;

const SRC: &str = r#"
codeunit 69975 C
{
    var
        GLbl: Label 'Hello';
        GTc: TextConst ENU = 'Hello';
        GLocked: Label 'Hello', Locked = true;
        GComment: Label 'Hello', Comment = 'a note';
        GVar: Variant;

    local procedure Kinds(V: Variant): Text
    var
        T: Text;
        C: Code[20];
    begin
        T := V;
        C := V;
        exit(Format(V.IsText()) + '/' + Format(V.IsCode()) + '/' + Format(V.IsTextConstant()) + '/' + Format(V.IsChar())
            + '=' + Format(V) + '|' + T + '|' + C);
    end;

    local procedure Pass(V: Variant): Text
    begin
        exit(Kinds(V));
    end;

    [TryFunction]
    local procedure TryPass(V: Variant)
    begin
    end;

    procedure PublicPass(V: Variant): Text
    begin
        exit(Kinds(V));
    end;

    local procedure IntoParam(var V: Variant)
    begin
        V := GLbl;
    end;

    procedure LocalLabel(): Text
    var
        Lbl: Label 'Hello';
        V: Variant;
    begin
        V := Lbl;
        exit(Kinds(V));
    end;

    procedure LocalTextConst(): Text
    var
        Tc: TextConst ENU = 'Hello';
        V: Variant;
    begin
        V := Tc;
        exit(Kinds(V));
    end;

    procedure GlobalTextConst(): Text
    var
        V: Variant;
    begin
        V := GTc;
        exit(Kinds(V));
    end;

    procedure GlobalTextConstArg(): Text
    begin
        exit(Pass(GTc));
    end;

    procedure GlobalLabel(): Text
    var
        V: Variant;
    begin
        V := GLbl;
        exit(Kinds(V));
    end;

    procedure LabelArg(): Text
    begin
        exit(Pass(GLbl));
    end;

    procedure LockedLabel(): Text
    var
        V: Variant;
    begin
        V := GLocked;
        exit(Kinds(V));
    end;

    procedure CommentLabel(): Text
    var
        Cmt: Label 'Hello', Comment = 'a note';
        V: Variant;
    begin
        V := Cmt;
        exit(Kinds(V));
    end;

    procedure GlobalCommentLabel(): Text
    var
        V: Variant;
    begin
        V := GComment;
        exit(Kinds(V));
    end;

    procedure GlobalTarget(): Text
    begin
        GVar := GLbl;
        exit(Kinds(GVar));
    end;

    procedure ParamTarget(): Text
    var
        V: Variant;
    begin
        IntoParam(V);
        exit(Kinds(V));
    end;

    procedure ArrayTarget(): Text
    var
        A: array[2] of Variant;
    begin
        A[1] := GLbl;
        exit(Kinds(A[1]));
    end;

    procedure TextConstGlobalTarget(): Text
    begin
        GVar := GTc;
        exit(Kinds(GVar));
    end;

    procedure TextConstLocalArg(): Text
    var
        Tc: TextConst ENU = 'Hello';
    begin
        exit(Pass(Tc));
    end;

    procedure TextConstPublicArg(): Text
    begin
        exit(PublicPass(GTc));
    end;

    procedure TextConstTryArg(): Boolean
    begin
        exit(TryPass(GTc));
    end;
}
"#;

const TEXT_CONSTANT: &str = "No/No/Yes/No=Hello|Hello|HELLO";
const TEXT: &str = "Yes/No/No/No=Hello|Hello|HELLO";

fn run(routine: &str) -> (Result<al_builtins::value::Value, Signal>, Vec<Assumption>) {
    take_hits();
    let r = al_testkit::run_in(SRC, "C", routine);
    (r, take_hits())
}

#[test]
fn pinned_cells_hold_a_text_constant_unflagged() {
    for routine in ["LocalLabel", "LocalTextConst", "GlobalTextConst", "GlobalTextConstArg"] {
        let (r, hits) = run(routine);
        assert_eq!(al_testkit::text(r), TEXT_CONSTANT, "{routine}");
        assert!(
            !hits.contains(&Assumption::LabelGlobalIntoVariant),
            "{routine} is pinned"
        );
        assert!(!hits.contains(&Assumption::VariantIsMatrix), "{routine} is pinned");
    }
}

#[test]
fn a_global_label_holds_a_text_constant_flagged() {
    let (r, hits) = run("GlobalLabel");
    assert_eq!(al_testkit::text(r), TEXT_CONSTANT);
    assert!(hits.contains(&Assumption::LabelGlobalIntoVariant));
}

#[test]
fn other_label_shapes_keep_the_text_conversion() {
    for routine in [
        "LabelArg",
        "LockedLabel",
        "CommentLabel",
        "GlobalCommentLabel",
        "GlobalTarget",
        "ParamTarget",
        "ArrayTarget",
    ] {
        let (r, hits) = run(routine);
        assert_eq!(al_testkit::text(r), TEXT, "{routine}");
        assert!(!hits.contains(&Assumption::LabelGlobalIntoVariant), "{routine}");
    }
}

#[test]
fn other_textconst_shapes_stay_refused() {
    for routine in [
        "TextConstGlobalTarget",
        "TextConstLocalArg",
        "TextConstTryArg",
        "TextConstPublicArg",
    ] {
        let (r, _) = run(routine);
        assert!(matches!(r, Err(Signal::Unsupported(_))), "{routine}: {r:?}");
    }
}
