//! Expressions. Precedence, from the AL operators reference
//! (learn.microsoft.com/.../developer/devenv-al-operators, "Operator hierarchy"):
//!
//! 1. `.` `[ ]` `( )` `::`          — postfix, handled in `postfix`
//! 2. `not` unary `-` unary `+`
//! 3. `*` `/` `div` `mod` `and`
//! 4. `+` `-` `or` `xor`   (xor below and: corpus types/PBinaryUnaryExpressions.al:72, `true xor true and false` is true)
//! 5. `>` `>=` `<` `<=` `=` `<>` `in` `is` `as`
//! 6. `..`                          — only inside set literals and case labels
//!
//! The conditional `c ? a : b` is not in that table; it is parsed below level 5
//! (lowest), which is what `exit(N > 3 ? 'big' : 'small')` in the corpus needs.

use crate::SyntaxKind::{self, *};
use crate::parser::Parser;

/// Words that can never be a name in expression position.
#[cfg(test)]
const RESERVED: &[&str] = &[
    "begin",
    "end",
    "if",
    "then",
    "else",
    "case",
    "of",
    "while",
    "do",
    "repeat",
    "until",
    "for",
    "to",
    "downto",
    "foreach",
    "in",
    "exit",
    "var",
    "procedure",
    "trigger",
    "local",
    "and",
    "or",
    "xor",
    "not",
    "div",
    "mod",
    "with",
    "asserterror",
];

pub(crate) fn is_reserved(word: &str) -> bool {
    // Lower-cased into a small buffer and matched by length and bytes: this runs for every identifier the parser meets.
    let w = word.as_bytes();
    if w.len() < 2 || w.len() > 11 {
        return false;
    }
    let mut buf = [0u8; 11];
    for (d, s) in buf.iter_mut().zip(w) {
        *d = s.to_ascii_lowercase();
    }
    matches!(
        &buf[..w.len()],
        b"begin"
            | b"end"
            | b"if"
            | b"then"
            | b"else"
            | b"case"
            | b"of"
            | b"while"
            | b"do"
            | b"repeat"
            | b"until"
            | b"for"
            | b"to"
            | b"downto"
            | b"foreach"
            | b"in"
            | b"exit"
            | b"var"
            | b"procedure"
            | b"trigger"
            | b"local"
            | b"and"
            | b"or"
            | b"xor"
            | b"not"
            | b"div"
            | b"mod"
            | b"with"
            | b"asserterror"
    )
}

pub(crate) fn at_expr_start(p: &Parser) -> bool {
    match p.current() {
        Ident => !is_reserved(p.nth_text(0)) || p.at_kw("not"),
        QuotedIdent | Integer | Decimal | StringLit | DateLit | TimeLit | DateTimeLit | BigIntLit | LParen | LBrack
        | Minus | Plus => true,
        _ => false,
    }
}

pub(crate) fn expr(p: &mut Parser) {
    let cp = p.checkpoint();
    relational(p);
    if p.at(Question) {
        p.start_at(cp, TernaryExpr);
        p.bump_any();
        expr(p);
        p.expect(Colon);
        expr(p);
        p.finish();
    }
}

fn relational(p: &mut Parser) {
    let cp = p.checkpoint();
    additive(p);
    while matches!(p.current(), Eq | Neq | Lt | Le | Gt | Ge) || p.at_any_kw(&["in", "is", "as"]) {
        let type_operator = p.at_kw("is") || p.at_kw("as");
        let membership = p.at_kw("in");
        p.start_at(cp, BinExpr);
        p.bump_any();
        if type_operator {
            type_name(p);
        } else if membership && p.at(LBrack) {
            // The set of `in` ends at its bracket: `x in [1] and y` is `(x in [1]) and y`.
            unary(p);
        } else {
            additive(p);
        }
        p.finish();
        if membership {
            boolean_tail(p, cp);
        }
    }
}

/// `in` binds tighter than `and`, `or` and `xor` on both sides (corpus types/lt-in-precedence-date-guid-filters/P143Prec_*.al,
/// pinned on BC 27, 28 and 29: `true in [true] and false`, `a and i in [j]`, `1 + 2 in [3] and true`): what follows a finished
/// `in` through `and`, `or` and `xor` continues the expression with the `in` as the left operand.
fn boolean_tail(p: &mut Parser, cp: crate::builder::Checkpoint) {
    while p.at_any_kw(&["and", "or", "xor"]) {
        p.start_at(cp, BinExpr);
        p.bump_any();
        let operand = p.checkpoint();
        unary(p);
        membership_of_operand(p, operand);
        p.finish();
    }
}

/// The right operand of `is` and `as`: an interface name, `A`, `"A B"` or `Ns.A`. It is a name,
/// not a value, so a call or an index does not continue it: `(X as I).Name()` takes parentheses.
fn type_name(p: &mut Parser) {
    if !matches!(p.current(), Ident | QuotedIdent) {
        p.error("expected a type name");
        return;
    }
    let cp = p.checkpoint();
    p.start(NameRef);
    p.bump_any();
    p.finish();
    while p.at(Dot) && matches!(p.nth(1), Ident | QuotedIdent) {
        p.start_at(cp, MemberExpr);
        p.bump_any();
        member_name(p);
        p.finish();
    }
}

fn additive(p: &mut Parser) {
    let cp = p.checkpoint();
    multiplicative(p);
    while matches!(p.current(), Plus | Minus) || p.at_any_kw(&["or", "xor"]) {
        let boolean = p.at_any_kw(&["or", "xor"]);
        p.start_at(cp, BinExpr);
        p.bump_any();
        let operand = p.checkpoint();
        multiplicative(p);
        if boolean {
            membership_of_operand(p, operand);
        }
        p.finish();
    }
}

/// `a and b in [..]` is `a and (b in [..])` (Base Application's Purchase Header writes it, and Microsoft's compiler
/// accepts it; the table of operators would read `(a and b) in [..]`, a Boolean and an enum). The operand of `and`,
/// `or` and `xor` that starts at `operand` takes the `in` that follows it.
fn membership_of_operand(p: &mut Parser, operand: crate::builder::Checkpoint) {
    if p.at_kw("in") {
        p.start_at(operand, BinExpr);
        p.bump_any();
        unary(p);
        p.finish();
    }
}

fn multiplicative(p: &mut Parser) {
    let cp = p.checkpoint();
    unary(p);
    while matches!(p.current(), Star | Slash) || p.at_any_kw(&["and", "div", "mod"]) {
        let boolean = p.at_kw("and");
        p.start_at(cp, BinExpr);
        p.bump_any();
        let operand = p.checkpoint();
        unary(p);
        if boolean {
            membership_of_operand(p, operand);
        }
        p.finish();
    }
}

fn unary(p: &mut Parser) {
    if matches!(p.current(), Minus | Plus) || p.at_kw("not") {
        p.start(PrefixExpr);
        p.bump_any();
        unary(p);
        p.finish();
    } else {
        postfix(p);
    }
}

fn postfix(p: &mut Parser) {
    let cp = p.checkpoint();
    if !primary(p) {
        return;
    }
    loop {
        match p.current() {
            Dot | ColonColon => {
                let kind = if p.at(Dot) { MemberExpr } else { ScopeExpr };
                p.start_at(cp, kind);
                p.bump_any();
                member_name(p);
                p.finish();
            }
            LParen => {
                p.start_at(cp, CallExpr);
                arg_list(p);
                p.finish();
            }
            LBrack => {
                p.start_at(cp, IndexExpr);
                p.bump_any();
                expr_list(p, RBrack);
                p.expect(RBrack);
                p.finish();
            }
            _ => break,
        }
    }
}

fn member_name(p: &mut Parser) {
    if matches!(p.current(), Ident | QuotedIdent | Integer) {
        p.start(NameRef);
        p.bump_any();
        p.finish();
    } else {
        p.error("expected a member name");
    }
}

pub(crate) fn arg_list(p: &mut Parser) {
    p.start(ArgList);
    p.expect(LParen);
    expr_list(p, RParen);
    p.expect(RParen);
    p.finish();
}

/// Comma-separated expressions up to (not including) `close`.
fn expr_list(p: &mut Parser, close: SyntaxKind) {
    if p.at(close) {
        return;
    }
    loop {
        expr(p);
        if !p.eat(Comma) {
            break;
        }
    }
}

/// Returns false (after recording an error) if no primary expression starts here.
fn primary(p: &mut Parser) -> bool {
    match p.current() {
        Integer | Decimal | StringLit | DateLit | TimeLit | DateTimeLit | BigIntLit => {
            p.start(Literal);
            p.bump_any();
            p.finish();
        }
        Ident if p.at_kw("true") || p.at_kw("false") => {
            p.start(Literal);
            p.bump_any();
            p.finish();
        }
        Ident if !is_reserved(p.nth_text(0)) => {
            p.start(NameRef);
            p.bump_any();
            p.finish();
        }
        QuotedIdent => {
            p.start(NameRef);
            p.bump_any();
            p.finish();
        }
        LParen => {
            p.start(ParenExpr);
            p.bump_any();
            expr(p);
            p.expect(RParen);
            p.finish();
        }
        LBrack => set_expr(p),
        _ => {
            p.error("expected an expression");
            return false;
        }
    }
    true
}

fn set_expr(p: &mut Parser) {
    p.start(SetExpr);
    p.bump_any();
    if !p.at(RBrack) {
        loop {
            range_or_expr(p);
            if !p.eat(Comma) {
                break;
            }
        }
    }
    p.expect(RBrack);
    p.finish();
}

/// `a`, `a..b`, `..b`, `a..` — set elements and case labels.
pub(crate) fn range_or_expr(p: &mut Parser) {
    if p.at(DotDot) {
        p.start(RangeExpr);
        p.bump_any();
        expr(p);
        p.finish();
        return;
    }
    let cp = p.checkpoint();
    expr(p);
    if p.at(DotDot) {
        p.start_at(cp, RangeExpr);
        p.bump_any();
        if at_expr_start(p) {
            expr(p);
        }
        p.finish();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grammar::test_util::{dump, errors};

    fn e(src: &str) -> String {
        dump(src, expr)
    }

    #[test]
    fn multiplicative_binds_tighter_than_additive() {
        assert_eq!(
            e("a + b * 2"),
            "(BinExpr (NameRef a) + (BinExpr (NameRef b) * (Literal 2)))"
        );
    }

    #[test]
    fn binary_operators_are_left_associative() {
        assert_eq!(
            e("a - b - c"),
            "(BinExpr (BinExpr (NameRef a) - (NameRef b)) - (NameRef c))"
        );
    }

    #[test]
    fn and_is_multiplicative_or_is_additive_comparison_is_lowest() {
        assert_eq!(
            e("a = b or c and d"),
            "(BinExpr (NameRef a) = (BinExpr (NameRef b) or (BinExpr (NameRef c) and (NameRef d))))"
        );
    }

    #[test]
    fn xor_binds_looser_than_and() {
        // corpus types/PBinaryUnaryExpressions.al:72
        assert_eq!(
            e("a xor b and c"),
            "(BinExpr (NameRef a) xor (BinExpr (NameRef b) and (NameRef c)))"
        );
    }

    #[test]
    fn keyword_operators_are_case_insensitive() {
        assert_eq!(
            e("a DIV b MOD c"),
            "(BinExpr (BinExpr (NameRef a) DIV (NameRef b)) MOD (NameRef c))"
        );
    }

    #[test]
    fn is_reserved_matches_the_list_in_any_case_and_nothing_else() {
        for w in RESERVED {
            assert!(is_reserved(w), "{w}");
            assert!(is_reserved(&w.to_uppercase()), "{w}");
        }
        for w in [
            "",
            "a",
            "xx",
            "beginn",
            "begi",
            "customer",
            "asserterrors",
            "É",
            "ändern",
        ] {
            assert!(!is_reserved(w), "{w}");
        }
    }

    #[test]
    fn unary_binds_tighter_than_binary() {
        assert_eq!(e("-a * b"), "(BinExpr (PrefixExpr - (NameRef a)) * (NameRef b))");
        assert_eq!(
            e("not a and b"),
            "(BinExpr (PrefixExpr not (NameRef a)) and (NameRef b))"
        );
    }

    #[test]
    fn literals() {
        assert_eq!(
            e("f(true, 'x', 1.5, 0D, 120000T, 0DT, 5L)"),
            "(CallExpr (NameRef f) (ArgList ( (Literal true) , (Literal 'x') , (Literal 1.5) , (Literal 0D) , (Literal 120000T) , (Literal 0DT) , (Literal 5L) )))"
        );
    }

    #[test]
    fn member_call_and_index_chain() {
        assert_eq!(
            e("Rec.\"No.\".Value(1)[2]"),
            "(IndexExpr (CallExpr (MemberExpr (MemberExpr (NameRef Rec) . (NameRef \"No.\")) . (NameRef Value)) (ArgList ( (Literal 1) ))) [ (Literal 2) ])"
        );
    }

    #[test]
    fn empty_call_and_multi_dimensional_index() {
        assert_eq!(e("f()"), "(CallExpr (NameRef f) (ArgList ( )))");
        assert_eq!(e("a[1, 2]"), "(IndexExpr (NameRef a) [ (Literal 1) , (Literal 2) ])");
    }

    #[test]
    fn scope_operator() {
        assert_eq!(
            e("Enum::\"ALT Status\"::Open"),
            "(ScopeExpr (ScopeExpr (NameRef Enum) :: (NameRef \"ALT Status\")) :: (NameRef Open))"
        );
        assert_eq!(
            e("Database::Customer"),
            "(ScopeExpr (NameRef Database) :: (NameRef Customer))"
        );
    }

    #[test]
    fn in_with_set_and_ranges() {
        assert_eq!(
            e("x in [1, 3..5, ..9]"),
            "(BinExpr (NameRef x) in (SetExpr [ (Literal 1) , (RangeExpr (Literal 3) .. (Literal 5)) , (RangeExpr .. (Literal 9)) ]))"
        );
    }

    /// Base Application's Purchase Header, `"Self-Billing Invoice" := Vend."Self-Billing Agreement" and Rec."Document
    /// Type" in [Order, Invoice];`, compiles in Microsoft's build, so the real parser reads `a and (b in [..])`, not
    /// `(a and b) in [..]` (a Boolean and an enum). Only the right operand of `and`, `or` and `xor` takes an `in`;
    /// `a in [..] and b` is unchanged. Not pinned by a corpus test (docs/probes-needed.md, assumption
    /// `in-binds-tighter-than-and`).
    #[test]
    fn an_in_after_and_or_xor_belongs_to_the_right_operand() {
        assert_eq!(
            e("a and b in [1]"),
            "(BinExpr (NameRef a) and (BinExpr (NameRef b) in (SetExpr [ (Literal 1) ])))"
        );
        assert_eq!(
            e("a or b in [1]"),
            "(BinExpr (NameRef a) or (BinExpr (NameRef b) in (SetExpr [ (Literal 1) ])))"
        );
        assert_eq!(
            e("a xor b in [1]"),
            "(BinExpr (NameRef a) xor (BinExpr (NameRef b) in (SetExpr [ (Literal 1) ])))"
        );
        assert_eq!(
            e("(a and b) in [1]"),
            "(BinExpr (ParenExpr ( (BinExpr (NameRef a) and (NameRef b)) )) in (SetExpr [ (Literal 1) ]))"
        );
        assert_eq!(
            e("a + b in [1]"),
            "(BinExpr (BinExpr (NameRef a) + (NameRef b)) in (SetExpr [ (Literal 1) ]))"
        );
    }

    #[test]
    fn empty_set() {
        assert_eq!(e("x in []"), "(BinExpr (NameRef x) in (SetExpr [ ]))");
    }

    #[test]
    fn is_and_as_take_a_type_name_and_bind_like_comparisons() {
        assert_eq!(e("X is \"Y Z\""), "(BinExpr (NameRef X) is (NameRef \"Y Z\"))");
        assert_eq!(
            e("X as Ns.I"),
            "(BinExpr (NameRef X) as (MemberExpr (NameRef Ns) . (NameRef I)))"
        );
        assert_eq!(
            e("a + b is I"),
            "(BinExpr (BinExpr (NameRef a) + (NameRef b)) is (NameRef I))"
        );
        assert_eq!(
            e("(X as I).Name()"),
            "(CallExpr (MemberExpr (ParenExpr ( (BinExpr (NameRef X) as (NameRef I)) )) . (NameRef Name)) (ArgList ( )))"
        );
    }

    #[test]
    fn is_without_a_type_name_is_an_error() {
        assert!(errors("X is", expr) > 0);
        assert!(errors("X as 5", expr) > 0);
    }

    #[test]
    fn ternary_is_lowest_and_right_associative() {
        assert_eq!(
            e("n > 3 ? 'a' : b ? 'c' : 'd'"),
            "(TernaryExpr (BinExpr (NameRef n) > (Literal 3)) ? (Literal 'a') : (TernaryExpr (NameRef b) ? (Literal 'c') : (Literal 'd')))"
        );
    }

    #[test]
    fn parenthesised() {
        assert_eq!(
            e("(a + b) * c"),
            "(BinExpr (ParenExpr ( (BinExpr (NameRef a) + (NameRef b)) )) * (NameRef c))"
        );
    }

    #[test]
    fn missing_operand_is_an_error_but_lossless() {
        assert!(errors("a +", expr) > 0);
        assert!(errors("(a", expr) > 0);
        assert!(errors(")", expr) > 0);
    }
}
