//! A Text method answers on a call stack text that holds the untracked-frame mark when its result depends only on the text
//! before the first mark (al-runner issue 7520, same reasoning as the comparison of marked texts in `ops::known_prefix_order`):
//! the lines of the tracked frames. Whatever depends on the tail stays refused. The prefix of a collected error's stack is
//! `"C"(CodeUnit 50100).<routine> line N`, a CR LF after each line, then the mark (a frame of a procedure with a hyphen in its
//! name stands below the raising routine; its format is not measured, so the text ends there with the mark)
//! (errors/PnBErrorsCollections.al:282-306 pins the lines of the corpus app's own frames).

use al_builtins::native::Signal;
use al_testkit::{boolean, int, run_in, text};

const SRC: &str = r#"
codeunit 50100 C
{
    // The hyphen in the name is a frame format the corpus has not measured: the call stack of the collected error ends
    // with the untracked mark below this frame.
    procedure "Hop-Collect"(var Info: ErrorInfo)
    begin
        Collect(Info);
    end;

    [ErrorBehavior(ErrorBehavior::Collect)]
    procedure Collect(var Info: ErrorInfo)
    var
        E: ErrorInfo;
    begin
        E := ErrorInfo.Create('boom');
        E.Collectible := true;
        Error(E);
        foreach E in GetCollectedErrors(true) do
            Info := E;
    end;

    procedure Stack(): Text
    var
        Info: ErrorInfo;
    begin
        "Hop-Collect"(Info);
        exit(Info.Callstack());
    end;

    procedure StackAsCode(): Code[2048]
    var
        Info: ErrorInfo;
        C: Code[2048];
    begin
        "Hop-Collect"(Info);
        C := Info.Callstack();
        exit(C);
    end;

    // Known-prefix answers.
    procedure IndexOfInPrefix(): Integer
    begin
        exit(Stack().IndexOf('CodeUnit'));
    end;

    procedure IndexOfStartInPrefix(): Integer
    begin
        exit(Stack().IndexOf('C', 2));
    end;

    procedure StrPosInPrefix(): Integer
    begin
        exit(StrPos(Stack(), 'Collect'));
    end;

    procedure StrPosOfCode(): Integer
    begin
        exit(StrPos(StackAsCode(), 'Collect'));
    end;

    procedure ContainsInPrefix(): Boolean
    begin
        exit(Stack().Contains('Collect'));
    end;

    procedure StartsWithTrue(): Boolean
    begin
        exit(Stack().StartsWith('"C"(CodeUnit 5'));
    end;

    procedure StartsWithFalse(): Boolean
    begin
        exit(Stack().StartsWith('Table'));
    end;

    procedure SubstringFromPrefix(): Text
    var
        S: Text;
    begin
        S := Stack();
        S := S.Substring(S.IndexOf('Collect'));
        exit(CopyStr(S, 1, 7));
    end;

    procedure SubstringInsidePrefix(): Text
    begin
        exit(Stack().Substring(1, 8));
    end;

    // Two lines of the stack stand before the mark: the entry's frame carries a hyphen in its name.
    procedure StackTwoLines(): Text
    var
        Info: ErrorInfo;
    begin
        Collect(Info);
        exit(Info.Callstack());
    end;

    procedure ReproShape(): Text
    begin
        exit("Hop-ReproShape"());
    end;

    local procedure "Hop-ReproShape"(): Text
    var
        S: Text;
        LF: Char;
    begin
        LF := 10;
        S := StackTwoLines();
        S := S.Substring(S.IndexOf(Format(LF)) + 1);
        exit(CopyStr(S, 1, 3));
    end;

    procedure ReproShapeKeepsMark(): Boolean
    begin
        exit("Hop-ReproShapeKeepsMark"());
    end;

    local procedure "Hop-ReproShapeKeepsMark"(): Boolean
    var
        S: Text;
        LF: Char;
    begin
        LF := 10;
        S := StackTwoLines();
        S := S.Substring(S.IndexOf(Format(LF)) + 1);
        // The rest still holds the unknown tail: a needle that is not in its known part stays refused.
        exit(StrPos(S, 'Stack') > 0);
    end;

    procedure CopyStrFromPrefix(): Text
    begin
        exit(CopyStr(Stack(), 10, 4));
    end;

    procedure CopyStrToEnd(): Text
    var
        S: Text;
    begin
        S := CopyStr(Stack(), 10);
        exit(CopyStr(S, 1, 3));
    end;

    procedure DelStrToEnd(): Text
    begin
        exit(DelStr(Stack(), 9));
    end;

    procedure DelStrInside(): Text
    var
        S: Text;
    begin
        S := DelStr(Stack(), 1, 4);
        exit(CopyStr(S, 1, 4));
    end;

    procedure RemoveToEnd(): Text
    begin
        exit(Stack().Remove(8));
    end;

    procedure ToUpperPrefix(): Text
    begin
        exit(CopyStr(Stack().ToUpper(), 1, 8));
    end;

    procedure ToLowerPrefix(): Text
    begin
        exit(CopyStr(LowerCase(Stack()), 1, 8));
    end;

    procedure UpperCasePrefix(): Text
    begin
        exit(CopyStr(UpperCase(Stack()), 1, 8));
    end;

    procedure ReplacePrefix(): Text
    begin
        exit(CopyStr(Stack().Replace('CodeUnit', 'CU'), 1, 6));
    end;

    procedure ConvertStrPrefix(): Text
    begin
        exit(CopyStr(ConvertStr(Stack(), 'C', 'X'), 1, 8));
    end;

    procedure TextBuilderToText(): Integer
    var
        B: TextBuilder;
    begin
        B.Append(Stack());
        exit(B.ToText().IndexOf('Collect'));
    end;

    procedure LastErrorCallStackIndexOf(): Boolean
    var
        Info: ErrorInfo;
    begin
        asserterror Error('x');
        exit(GetLastErrorCallStack().IndexOf('CodeUnit') > 0);
    end;

    procedure SessionCallstackIndexOf(): Boolean
    begin
        exit(StrPos(SessionInformation.Callstack, 'CodeUnit') > 0);
    end;

    // Refused: the answer depends on the tail.
    procedure IndexOfNotInPrefix(): Integer
    begin
        exit(Stack().IndexOf('zzzzqq'));
    end;

    procedure StrPosNotInPrefix(): Integer
    begin
        exit(StrPos(Stack(), 'zzzzqq'));
    end;

    procedure ContainsNotInPrefix(): Boolean
    begin
        exit(Stack().Contains('zzzzqq'));
    end;

    procedure StrLenOfStack(): Integer
    begin
        exit(StrLen(Stack()));
    end;

    procedure EndsWithOfStack(): Boolean
    begin
        exit(Stack().EndsWith('x'));
    end;

    procedure LastIndexOfStack(): Integer
    begin
        exit(Stack().LastIndexOf('C'));
    end;

    procedure PadStrOfStack(): Text
    begin
        exit(PadStr(Stack(), 4000));
    end;

    procedure SplitOfStack(): Integer
    var
        Parts: List of [Text];
    begin
        Parts := Stack().Split('x');
        exit(Parts.Count());
    end;

    procedure SubstringPastPrefixWithLength(): Text
    begin
        // Substring clamps a length past the end (types/TestFormatEvaluateTextContracts.al:Text_OutOfRangeArguments).
        exit(CopyStr(Stack().Substring(1, 4000), 1, 8));
    end;

    procedure DelStrPastPrefix(): Text
    begin
        exit(DelStr(Stack(), 1, 4000));
    end;

    procedure ReplaceAcrossTheMark(): Text
    var
        S: Text;
        LF: Char;
    begin
        LF := 10;
        // The needle starts with the line feed that ends the prefix, so it could continue into the tail.
        S := Stack().Replace(Format(LF) + 'zz', 'y');
        exit(CopyStr(S, 1, 1));
    end;

    procedure SelectStrInPrefix(): Text
    begin
        exit(SelectStr(2, 'a,b,' + Stack()));
    end;

    procedure SelectStrIntoTail(): Text
    begin
        exit(SelectStr(3, 'a,b,' + Stack()));
    end;

    procedure StartsWithLongerThanPrefix(): Boolean
    var
        CR: Char;
        LF: Char;
    begin
        CR := 13;
        LF := 10;
        exit(Stack().StartsWith('"C"(CodeUnit 50100).Collect line 6' + Format(CR) + Format(LF) + 'x'));
    end;
}
"#;

fn refused(routine: &str) {
    match run_in(SRC, "C", routine) {
        Err(Signal::Unsupported(m)) => assert!(!m.is_empty(), "{routine}"),
        other => panic!("{routine}: expected a refusal, got {other:?}"),
    }
}

#[test]
fn index_of_a_needle_in_the_prefix() {
    assert_eq!(int(run_in(SRC, "C", "IndexOfInPrefix")), 5);
    assert_eq!(int(run_in(SRC, "C", "IndexOfStartInPrefix")), 2);
}

#[test]
fn str_pos_of_a_needle_in_the_prefix_of_text_and_code() {
    assert_eq!(int(run_in(SRC, "C", "StrPosInPrefix")), 21);
    // A Code variable cannot hold a call stack text: the conversion is refused before any method runs.
    refused("StrPosOfCode");
}

#[test]
fn contains_and_starts_with_decided_by_the_prefix() {
    assert!(boolean(run_in(SRC, "C", "ContainsInPrefix")));
    assert!(boolean(run_in(SRC, "C", "StartsWithTrue")));
    assert!(!boolean(run_in(SRC, "C", "StartsWithFalse")));
}

#[test]
fn substring_from_the_prefix_keeps_the_unknown_tail() {
    assert_eq!(text(run_in(SRC, "C", "SubstringPastPrefixWithLength")), "\"C\"(Code");
    assert_eq!(text(run_in(SRC, "C", "SubstringInsidePrefix")), "\"C\"(Code");
    assert_eq!(text(run_in(SRC, "C", "SubstringFromPrefix")), "Collect");
    assert_eq!(text(run_in(SRC, "C", "ReproShape")), "\"C\"");
    assert!(boolean(run_in(SRC, "C", "ReproShapeKeepsMark")));
}

#[test]
fn copy_str_and_del_str_and_remove_from_the_prefix() {
    assert_eq!(text(run_in(SRC, "C", "CopyStrFromPrefix")), "nit ");
    assert_eq!(text(run_in(SRC, "C", "CopyStrToEnd")), "nit");
    assert_eq!(text(run_in(SRC, "C", "DelStrToEnd")), "\"C\"(Code");
    assert_eq!(text(run_in(SRC, "C", "DelStrInside")), "Code");
    assert_eq!(text(run_in(SRC, "C", "RemoveToEnd")), "\"C\"(Cod");
}

#[test]
fn case_replace_and_convert_str_keep_the_tail() {
    assert_eq!(text(run_in(SRC, "C", "ToUpperPrefix")), "\"C\"(CODE");
    assert_eq!(text(run_in(SRC, "C", "ToLowerPrefix")), "\"c\"(code");
    assert_eq!(text(run_in(SRC, "C", "UpperCasePrefix")), "\"C\"(CODE");
    assert_eq!(text(run_in(SRC, "C", "ReplacePrefix")), "\"C\"(CU");
    assert_eq!(text(run_in(SRC, "C", "ConvertStrPrefix")), "\"X\"(Xode");
}
#[test]
fn select_str_of_an_item_that_ends_in_the_prefix() {
    assert_eq!(text(run_in(SRC, "C", "SelectStrInPrefix")), "b");
}

#[test]
fn text_builder_to_text_and_the_other_sources() {
    assert!(int(run_in(SRC, "C", "TextBuilderToText")) > 0);
    assert!(boolean(run_in(SRC, "C", "LastErrorCallStackIndexOf")));
    assert!(boolean(run_in(SRC, "C", "SessionCallstackIndexOf")));
}

#[test]
fn what_depends_on_the_tail_stays_refused() {
    for r in [
        "IndexOfNotInPrefix",
        "StrPosNotInPrefix",
        "ContainsNotInPrefix",
        "StrLenOfStack",
        "EndsWithOfStack",
        "LastIndexOfStack",
        "PadStrOfStack",
        "SplitOfStack",
        "DelStrPastPrefix",
        "ReplaceAcrossTheMark",
        "SelectStrIntoTail",
        "StartsWithLongerThanPrefix",
    ] {
        refused(r);
    }
}
