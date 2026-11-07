//! Statements and code blocks.

use super::expressions::{at_expr_start, expr, range_or_expr};
use crate::SyntaxKind::*;
use crate::parser::Parser;

pub(crate) fn block(p: &mut Parser) {
    p.start(Block);
    p.expect_kw("begin");
    stmt_list(p, &["end"]);
    p.expect_kw("end");
    p.finish();
}

/// Statements separated by `;`, up to (not including) one of `terminators`.
fn stmt_list(p: &mut Parser, terminators: &[&str]) {
    while !p.at_eof() && p.which_kw(terminators).is_none() {
        if p.eat(Semicolon) {
            continue;
        }
        if at_stmt_start(p) {
            stmt(p);
        } else {
            p.err_and_bump("expected a statement");
        }
    }
}

/// The keywords that start a statement, in the order `stmt` tests them.
const STMT_KWS: [&str; 12] = [
    "begin",
    "if",
    "case",
    "while",
    "repeat",
    "for",
    "foreach",
    "exit",
    "break",
    "asserterror",
    "continue",
    "with",
];

/// The keyword that starts a statement. `continue` is one only where a statement of one word ends: before `;`, `end`,
/// `else`, `until` or the end of the input; anywhere else it is a name (corpus types/PUContinue.al,
/// language/ControlFlow.al).
fn stmt_kw(p: &Parser) -> Option<usize> {
    match p.which_kw(&STMT_KWS) {
        Some(10)
            if !(matches!(p.nth(1), Semicolon | __Last) || ["end", "else", "until"].iter().any(|k| p.nth_kw(1, k))) =>
        {
            None
        }
        kw => kw,
    }
}

fn at_stmt_start(p: &Parser) -> bool {
    stmt_kw(p).is_some() || at_expr_start(p)
}

pub(crate) fn stmt(p: &mut Parser) {
    // One look-ahead names the keyword; an expression statement (no keyword) goes straight to the last arm.
    let kw = stmt_kw(p);
    if kw == Some(0) {
        block(p);
    } else if kw == Some(1) {
        p.start(IfStmt);
        p.bump_any();
        expr(p);
        p.expect_kw("then");
        opt_stmt(p);
        if p.eat_kw("else") {
            opt_stmt(p);
        }
        p.finish();
    } else if kw == Some(2) {
        case_stmt(p);
    } else if kw == Some(3) {
        p.start(WhileStmt);
        p.bump_any();
        expr(p);
        p.expect_kw("do");
        opt_stmt(p);
        p.finish();
    } else if kw == Some(4) {
        p.start(RepeatStmt);
        p.bump_any();
        stmt_list(p, &["until"]);
        p.expect_kw("until");
        expr(p);
        p.finish();
    } else if kw == Some(5) {
        p.start(ForStmt);
        p.bump_any();
        expr(p);
        p.expect(Assign);
        expr(p);
        if !p.eat_kw("to") {
            p.expect_kw("downto");
        }
        expr(p);
        p.expect_kw("do");
        opt_stmt(p);
        p.finish();
    } else if kw == Some(6) {
        p.start(ForeachStmt);
        p.bump_any();
        expr_until_in(p);
        p.expect_kw("in");
        expr(p);
        p.expect_kw("do");
        opt_stmt(p);
        p.finish();
    } else if kw == Some(7) {
        p.start(ExitStmt);
        p.bump_any();
        if p.eat(LParen) {
            if !p.at(RParen) {
                expr(p);
            }
            p.expect(RParen);
        }
        p.finish();
    } else if kw == Some(8) {
        p.start(BreakStmt);
        p.bump_any();
        p.finish();
    } else if kw == Some(10) {
        p.start(ContinueStmt);
        p.bump_any();
        p.finish();
    } else if kw == Some(11) {
        p.start(WithStmt);
        p.bump_any();
        expr(p);
        p.expect_kw("do");
        opt_stmt(p);
        p.finish();
    } else if kw == Some(9) {
        p.start(AssertErrorStmt);
        p.bump_any();
        opt_stmt(p);
        p.finish();
    } else {
        let cp = p.checkpoint();
        expr(p);
        if matches!(
            p.current(),
            Assign | PlusAssign | MinusAssign | StarAssign | SlashAssign
        ) {
            p.start_at(cp, AssignStmt);
            p.bump_any();
            expr(p);
        } else {
            p.start_at(cp, ExprStmt);
        }
        p.finish();
    }
}

/// The loop variable of `foreach x in ...`: a bare name, so `in` is not
/// taken as the membership operator.
fn expr_until_in(p: &mut Parser) {
    if matches!(p.current(), Ident | QuotedIdent) {
        p.start(NameRef);
        p.bump_any();
        p.finish();
    } else {
        p.error("expected a loop variable");
    }
}

/// A statement that may be empty, as after `then`, `else`, `do`.
fn opt_stmt(p: &mut Parser) {
    if at_stmt_start(p) {
        stmt(p);
    }
}

fn case_stmt(p: &mut Parser) {
    p.start(CaseStmt);
    p.bump_any();
    expr(p);
    p.expect_kw("of");
    while !p.at_eof() && !p.at_kw("end") && !p.at_kw("else") {
        if p.eat(Semicolon) {
            continue;
        }
        if !at_expr_start(p) && !p.at(DotDot) {
            p.err_and_bump("expected a case label");
            continue;
        }
        p.start(CaseBranch);
        loop {
            range_or_expr(p);
            if !p.eat(Comma) {
                break;
            }
        }
        p.expect(Colon);
        opt_stmt(p);
        p.eat(Semicolon);
        p.finish();
    }
    if p.at_kw("else") {
        p.start(CaseElse);
        p.bump_any();
        stmt_list(p, &["end"]);
        p.finish();
    }
    p.expect_kw("end");
    p.finish();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grammar::test_util::{dump, errors};

    fn s(src: &str) -> String {
        dump(src, stmt)
    }

    #[test]
    fn assignment_and_compound_assignment() {
        assert_eq!(s("x := 1"), "(AssignStmt (NameRef x) := (Literal 1))");
        assert_eq!(
            s("Rec.\"No.\" += 2"),
            "(AssignStmt (MemberExpr (NameRef Rec) . (NameRef \"No.\")) += (Literal 2))"
        );
    }

    #[test]
    fn call_statement() {
        assert_eq!(
            s("Rec.Insert()"),
            "(ExprStmt (CallExpr (MemberExpr (NameRef Rec) . (NameRef Insert)) (ArgList ( ))))"
        );
    }

    #[test]
    fn method_call_without_parentheses() {
        assert_eq!(s("Rec.Init"), "(ExprStmt (MemberExpr (NameRef Rec) . (NameRef Init)))");
    }

    #[test]
    fn block_with_separators_and_trailing_semicolon() {
        assert_eq!(
            dump("begin a := 1; b := 2; end", block),
            "(Block begin (AssignStmt (NameRef a) := (Literal 1)) ; (AssignStmt (NameRef b) := (Literal 2)) ; end)"
        );
        assert_eq!(dump("begin end", block), "(Block begin end)");
    }

    #[test]
    fn if_then_else_binds_else_to_nearest_if() {
        assert_eq!(
            s("if a then if b then x() else y()"),
            "(IfStmt if (NameRef a) then (IfStmt if (NameRef b) then (ExprStmt (CallExpr (NameRef x) (ArgList ( )))) else (ExprStmt (CallExpr (NameRef y) (ArgList ( ))))))"
        );
    }

    #[test]
    fn if_with_block_branches() {
        assert_eq!(
            s("if a then begin end else begin end"),
            "(IfStmt if (NameRef a) then (Block begin end) else (Block begin end))"
        );
    }

    #[test]
    fn case_with_labels_ranges_and_else() {
        assert_eq!(
            s("case x of 1, 2: a := 1; 3..5: begin end; else b := 2; c := 3; end"),
            "(CaseStmt case (NameRef x) of (CaseBranch (Literal 1) , (Literal 2) : (AssignStmt (NameRef a) := (Literal 1)) ;) (CaseBranch (RangeExpr (Literal 3) .. (Literal 5)) : (Block begin end) ;) (CaseElse else (AssignStmt (NameRef b) := (Literal 2)) ; (AssignStmt (NameRef c) := (Literal 3)) ;) end)"
        );
    }

    #[test]
    fn case_true_of_with_expression_labels() {
        assert_eq!(
            s("case true of a > 1: x(); end"),
            "(CaseStmt case (Literal true) of (CaseBranch (BinExpr (NameRef a) > (Literal 1)) : (ExprStmt (CallExpr (NameRef x) (ArgList ( )))) ;) end)"
        );
    }

    #[test]
    fn loops() {
        assert_eq!(
            s("while a do x := 1"),
            "(WhileStmt while (NameRef a) do (AssignStmt (NameRef x) := (Literal 1)))"
        );
        assert_eq!(
            s("repeat x := 1; until Rec.Next() = 0"),
            "(RepeatStmt repeat (AssignStmt (NameRef x) := (Literal 1)) ; until (BinExpr (CallExpr (MemberExpr (NameRef Rec) . (NameRef Next)) (ArgList ( ))) = (Literal 0)))"
        );
        assert_eq!(
            s("for i := 1 to 10 do x += i"),
            "(ForStmt for (NameRef i) := (Literal 1) to (Literal 10) do (AssignStmt (NameRef x) += (NameRef i)))"
        );
        assert_eq!(
            s("for i := 9 downto 0 do x()"),
            "(ForStmt for (NameRef i) := (Literal 9) downto (Literal 0) do (ExprStmt (CallExpr (NameRef x) (ArgList ( )))))"
        );
        assert_eq!(
            s("foreach t in L do x(t)"),
            "(ForeachStmt foreach (NameRef t) in (NameRef L) do (ExprStmt (CallExpr (NameRef x) (ArgList ( (NameRef t) )))))"
        );
    }

    #[test]
    fn exit_forms() {
        assert_eq!(s("exit"), "(ExitStmt exit)");
        assert_eq!(s("exit(1)"), "(ExitStmt exit ( (Literal 1) ))");
        assert_eq!(s("exit()"), "(ExitStmt exit ( ))");
    }

    #[test]
    fn asserterror_wraps_a_statement() {
        assert_eq!(
            s("asserterror Rec.Insert(false)"),
            "(AssertErrorStmt asserterror (ExprStmt (CallExpr (MemberExpr (NameRef Rec) . (NameRef Insert)) (ArgList ( (Literal false) )))))"
        );
    }

    #[test]
    fn with_statement_takes_an_expression_and_one_statement() {
        // corpus events/platform-triggers/P114WithStatement.al:30-60
        assert_eq!(
            s("with Row do Id := 7"),
            "(WithStmt with (NameRef Row) do (AssignStmt (NameRef Id) := (Literal 7)))"
        );
        assert_eq!(
            s("with A do with B do begin end"),
            "(WithStmt with (NameRef A) do (WithStmt with (NameRef B) do (Block begin end)))"
        );
    }

    #[test]
    fn break_statement() {
        assert_eq!(s("break"), "(BreakStmt break)");
        assert_eq!(s("continue"), "(ContinueStmt continue)");
        assert_eq!(s("begin continue; end"), "(Block begin (ContinueStmt continue) ; end)");
        // Not at the end of a one-word statement it is a name.
        assert_eq!(s("continue := 1"), "(AssignStmt (NameRef continue) := (Literal 1))");
        assert_eq!(
            s("continue(1)"),
            "(ExprStmt (CallExpr (NameRef continue) (ArgList ( (Literal 1) ))))"
        );
    }

    #[test]
    fn junk_in_a_block_is_an_error_but_parsing_continues_to_end() {
        assert!(errors("begin then; x := 1; end", block) > 0);
        assert!(errors("begin x := ; end", block) > 0);
        assert!(errors("begin x := 1", block) > 0);
    }
}
