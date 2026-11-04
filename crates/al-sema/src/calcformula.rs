//! The `CalcFormula` property of a FlowField, as syntax: method, source,
//! where-conditions. Names are kept as written (quotes removed); resolving
//! them against tables is `lower`'s job.
//!
//! Grammar, from the property docs and the corpus formulas
//! (corpus/tests/al-language/record/TestCalcFormula*):
//!
//! ```text
//! formula := ['-'] method '(' source ['where' '(' cond {',' cond} ')'] ')'
//! source  := table ['.' field]   (table may be namespace-qualified: `Ns.Table`)
//! cond    := field '=' ( const(..) | filter(..) | field(ref) )
//! ref     := name | upperlimit(name) | filter(name)
//! ```

use al_ir::CalcMethod;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Formula {
    /// A leading `-` (corpus: record/TestCalcFormulaSignFiltersHeader.Table.al:24).
    pub negate: bool,
    pub method: CalcMethod,
    /// The table's own name (the last segment of `table_path`).
    pub table: String,
    /// The source table as written, namespace segments first (`Ns.Sub.Table`).
    pub table_path: Vec<String>,
    pub field: Option<String>,
    pub conds: Vec<Cond>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cond {
    /// A field of the source table.
    pub field: String,
    pub value: CondValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CondValue {
    /// `const(..)` with the text between the parentheses, as written.
    Const(String),
    /// `filter(..)` with the text between the parentheses, as written.
    Filter(String),
    /// `field(X)`: a field of the table being calculated.
    Field(String),
    /// `field(upperlimit(X))`.
    UpperLimit(String),
    /// `field(filter(X))`.
    FieldFilter(String),
}

pub fn parse(text: &str) -> Result<Formula, String> {
    let mut p = Cursor {
        c: text.chars().collect(),
        i: 0,
    };
    p.formula().map_err(|e| format!("CalcFormula `{}`: {e}", text.trim()))
}

/// A comma-separated list of `field = const(..) | filter(..) | field(..)` conditions, as written
/// in a query's `DataItemTableFilter` and `ColumnFilter` properties.
pub fn parse_conditions(text: &str) -> Result<Vec<Cond>, String> {
    let mut p = Cursor {
        c: text.chars().collect(),
        i: 0,
    };
    let mut conds = Vec::new();
    loop {
        conds.push(p.cond().map_err(|e| format!("`{}`: {e}", text.trim()))?);
        if !p.eat(',') {
            break;
        }
    }
    if p.peek().is_some() {
        return Err(format!("`{}`: unexpected text after the conditions", text.trim()));
    }
    Ok(conds)
}

struct Cursor {
    c: Vec<char>,
    i: usize,
}

impl Cursor {
    fn ws(&mut self) {
        while self.c.get(self.i).is_some_and(|c| c.is_whitespace()) {
            self.i += 1;
        }
    }

    fn peek(&mut self) -> Option<char> {
        self.ws();
        self.c.get(self.i).copied()
    }

    fn eat(&mut self, ch: char) -> bool {
        if self.peek() == Some(ch) {
            self.i += 1;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, ch: char) -> Result<(), String> {
        if self.eat(ch) {
            Ok(())
        } else {
            Err(match self.c.get(self.i) {
                Some(found) => format!("expected `{ch}`, found `{found}`"),
                None => format!("expected `{ch}`, found the end"),
            })
        }
    }

    /// A name: `"quoted"` (quotes removed) or a bare identifier. The flag
    /// says whether it was quoted, because only bare words are keywords.
    fn name(&mut self) -> Result<(String, bool), String> {
        if self.peek() == Some('"') {
            self.i += 1;
            let start = self.i;
            while self.c.get(self.i).is_some_and(|c| *c != '"') {
                self.i += 1;
            }
            if self.i >= self.c.len() {
                return Err("unterminated quoted name".into());
            }
            let n: String = self.c[start..self.i].iter().collect();
            self.i += 1;
            return Ok((n, true));
        }
        let start = self.i;
        while self.c.get(self.i).is_some_and(|c| c.is_alphanumeric() || *c == '_') {
            self.i += 1;
        }
        if start == self.i {
            return Err(match self.c.get(self.i) {
                Some(found) => format!("expected a name, found `{found}`"),
                None => "expected a name, found the end".into(),
            });
        }
        Ok((self.c[start..self.i].iter().collect(), false))
    }

    /// A bare keyword, lower-cased; `None` (and nothing consumed) for a quoted name.
    fn keyword(&mut self) -> Result<String, String> {
        let at = self.i;
        let (n, quoted) = self.name()?;
        if quoted {
            self.i = at;
            return Err(format!("expected a keyword, found \"{n}\""));
        }
        Ok(n.to_lowercase())
    }

    /// The text up to the `)` that closes an already consumed `(`, which is
    /// consumed too. Quotes (`"..."`, `'...'`) hide parentheses.
    fn raw_until_close(&mut self) -> Result<String, String> {
        let open = self.i.checked_sub(1).filter(|&o| self.c.get(o) == Some(&'('));
        let Some(close) = open.and_then(|o| crate::scan::matching_paren(&self.c, o)) else {
            // The cursor is left where it stopped reading, as the scanner did (an error ends the parse).
            self.i = self.c.len();
            return Err("missing `)`".into());
        };
        let raw: String = self.c[self.i..close].iter().collect();
        self.i = close + 1;
        Ok(raw.trim().to_string())
    }

    fn formula(&mut self) -> Result<Formula, String> {
        let negate = self.eat('-');
        let method = match self.keyword()?.as_str() {
            "sum" => CalcMethod::Sum,
            "count" => CalcMethod::Count,
            "average" => CalcMethod::Average,
            "min" => CalcMethod::Min,
            "max" => CalcMethod::Max,
            "exist" => CalcMethod::Exist,
            "lookup" => CalcMethod::Lookup,
            other => return Err(format!("unknown method `{other}`")),
        };
        self.expect('(')?;
        let mut path = vec![self.name()?.0];
        while self.i < self.c.len() && self.c[self.i] == '.' {
            self.i += 1;
            path.push(self.name()?.0);
        }
        // `Table.Field` for the methods that read a field; every segment is the table's
        // (possibly namespace-qualified) name for count and exist, which take no field.
        let field = if matches!(method, CalcMethod::Count | CalcMethod::Exist) || path.len() < 2 {
            None
        } else {
            path.pop()
        };
        let table = path.last().cloned().unwrap_or_default();
        match (method, &field) {
            (CalcMethod::Count | CalcMethod::Exist, _) => {}
            (_, None) => return Err("this method needs a source field".into()),
            _ => {}
        }
        let mut conds = Vec::new();
        if self.peek() != Some(')') {
            if self.keyword()? != "where" {
                return Err("expected `where`".into());
            }
            self.expect('(')?;
            loop {
                conds.push(self.cond()?);
                if !self.eat(',') {
                    break;
                }
            }
            self.expect(')')?;
        }
        self.expect(')')?;
        if self.peek().is_some() {
            return Err("unexpected text after the formula".into());
        }
        Ok(Formula {
            negate,
            method,
            table,
            table_path: path,
            field,
            conds,
        })
    }

    fn cond(&mut self) -> Result<Cond, String> {
        let (field, _) = self.name()?;
        self.expect('=')?;
        let value = match self.keyword()?.as_str() {
            kw @ ("const" | "filter") => {
                self.expect('(')?;
                let raw = self.raw_until_close()?;
                if kw == "const" {
                    CondValue::Const(raw)
                } else {
                    CondValue::Filter(raw)
                }
            }
            "field" => {
                self.expect('(')?;
                let (n, quoted) = self.name()?;
                let lower = n.to_lowercase();
                let value = if !quoted && (lower == "upperlimit" || lower == "filter") && self.peek() == Some('(') {
                    self.i += 1;
                    let (inner, _) = self.name()?;
                    self.expect(')')?;
                    if lower == "filter" {
                        CondValue::FieldFilter(inner)
                    } else {
                        CondValue::UpperLimit(inner)
                    }
                } else {
                    CondValue::Field(n)
                };
                self.expect(')')?;
                value
            }
            other => return Err(format!("expected const, filter or field, found `{other}`")),
        };
        Ok(Cond { field, value })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(field: &str, value: CondValue) -> Cond {
        Cond {
            field: field.into(),
            value,
        }
    }

    #[test]
    fn count_with_a_field_link() {
        let f = parse("count(\"ALT Child\" where(\"Parent Entry No.\" = field(\"Entry No.\")))").unwrap();
        assert_eq!(
            f,
            Formula {
                negate: false,
                method: CalcMethod::Count,
                table: "ALT Child".into(),
                table_path: vec!["ALT Child".into()],
                field: None,
                conds: vec![c("Parent Entry No.", CondValue::Field("Entry No.".into()))],
            }
        );
    }

    #[test]
    fn sum_of_a_field_with_unquoted_names() {
        let f = parse("sum(Line.Amount where(Doc = field(No)))").unwrap();
        assert_eq!(f.method, CalcMethod::Sum);
        assert_eq!((f.table.as_str(), f.field.as_deref()), ("Line", Some("Amount")));
        assert_eq!(f.conds, vec![c("Doc", CondValue::Field("No".into()))]);
    }

    #[test]
    fn namespace_qualified_source_table() {
        // Microsoft test: Tests-ERM codeunit 134331 ERM Purchase Payables,
        // PurchaseInvoiceAmountExclVATWithInheritFromParentAllocationAccount (a FlowField of Base
        // Application reads `exist(Microsoft.Finance.AllocationAccount."Alloc. Acc. Manual Override" where(...))`).
        let f = parse("exist(Ns.Sub.\"Some Table\" where(A = field(B)))").unwrap();
        assert_eq!(f.table_path, ["Ns", "Sub", "Some Table"]);
        assert_eq!((f.table.as_str(), f.field), ("Some Table", None));
        let f = parse("sum(Ns.Line.Amount)").unwrap();
        assert_eq!(f.table_path, ["Ns", "Line"]);
        assert_eq!(f.field.as_deref(), Some("Amount"));
    }

    #[test]
    fn every_method_name_parses() {
        for (name, m) in [
            ("sum", CalcMethod::Sum),
            ("Average", CalcMethod::Average),
            ("MIN", CalcMethod::Min),
            ("max", CalcMethod::Max),
            ("lookup", CalcMethod::Lookup),
        ] {
            let f = parse(&format!("{name}(T.F)")).unwrap();
            assert_eq!(f.method, m);
            assert!(f.conds.is_empty());
        }
        assert_eq!(parse("exist(T)").unwrap().method, CalcMethod::Exist);
        assert_eq!(parse("count(T)").unwrap().field, None);
    }

    #[test]
    fn leading_sign_with_and_without_a_space() {
        // record/TestCalcFormulaSignFiltersHeader.Table.al:24 and :71
        assert!(parse("-sum(T.F)").unwrap().negate);
        assert!(parse("- exist(T where(A = field(B)))").unwrap().negate);
        assert!(!parse("sum(T.F)").unwrap().negate);
    }

    #[test]
    fn const_and_filter_values_keep_their_text() {
        let f = parse(
            "sum(L.Amount where(D = field(N),\n  Open = const(true),\n  S = filter(Open | Released),\n  \"Entry No.\" = filter(2 .. 3)))",
        )
        .unwrap();
        assert_eq!(
            f.conds,
            vec![
                c("D", CondValue::Field("N".into())),
                c("Open", CondValue::Const("true".into())),
                c("S", CondValue::Filter("Open | Released".into())),
                c("Entry No.", CondValue::Filter("2 .. 3".into())),
            ]
        );
    }

    #[test]
    fn parentheses_and_quotes_inside_quoted_names_are_literal() {
        // record/TestCalcFormulaQuotedFilterHeader.Table.al:22
        let f = parse(
            "sum(\"CFQ Line\".Amount where(\"Header No.\" = field(\"No.\"), \"Entry Type\" = filter(\"Payment Discount (VAT Excl.)\")))",
        )
        .unwrap();
        assert_eq!(
            f.conds[1],
            c(
                "Entry Type",
                CondValue::Filter("\"Payment Discount (VAT Excl.)\"".into())
            )
        );
        let g = parse("count(L where(T = filter(<> \" \")))").unwrap();
        assert_eq!(g.conds, vec![c("T", CondValue::Filter("<> \" \"".into()))]);
    }

    #[test]
    fn flow_filter_references() {
        // record/TestCalcFormulaFlowFiltersHeader.Table.al:44,52
        let f = parse(
            "sum(L.A where(D = field(N), P = field(upperlimit(\"Date Filter\")), Q = field(filter(\"Account Totaling\"))))",
        )
        .unwrap();
        assert_eq!(
            f.conds[1..],
            [
                c("P", CondValue::UpperLimit("Date Filter".into())),
                c("Q", CondValue::FieldFilter("Account Totaling".into())),
            ]
        );
    }

    #[test]
    fn const_with_an_object_reference_and_a_string() {
        // record/TestCalcFormulaDatabaseConst.Codeunit.al:7
        let f = parse("count(R where(\"Table ID\" = const(Database::\"CDC Owner\"), K = const('a)b')))").unwrap();
        assert_eq!(
            f.conds,
            vec![
                c("Table ID", CondValue::Const("Database::\"CDC Owner\"".into())),
                c("K", CondValue::Const("'a)b'".into())),
            ]
        );
    }

    #[test]
    fn a_condition_list_as_in_a_query_table_filter() {
        // queries/TestQueryDataItemTableFilter.al: `Status = const(Open), Amount = filter(> 100)`.
        let conds = parse_conditions("Status = const(Open), Amount = filter(> 100)").unwrap();
        assert_eq!(
            conds,
            vec![
                c("Status", CondValue::Const("Open".into())),
                c("Amount", CondValue::Filter("> 100".into())),
            ]
        );
        assert!(parse_conditions("Status = const(Open),").is_err());
        assert!(parse_conditions("Status = const(Open) extra").is_err());
    }

    #[test]
    fn malformed_formulas_are_errors_not_guesses() {
        for bad in [
            "",
            "sum",
            "sum(",
            "sum(T.F where(A))",
            "sum(T.F where(A = 5))",
            "median(T.F)",
            "sum(T.F) extra",
            "sum(T.F where(A = const(1))",
        ] {
            assert!(parse(bad).is_err(), "`{bad}` should be refused");
        }
    }
    // Mutation-testing additions (docs/status.md debt): the raw text of const() and filter() runs
    // to the closing parenthesis, and quotes and nesting decide which one that is.

    fn filter_text(inner: &str) -> String {
        match &parse(&format!("count(T where(A = filter({inner})))")).unwrap().conds[0].value {
            CondValue::Filter(t) => t.clone(),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn nested_parentheses_stay_in_the_raw_text() {
        assert_eq!(filter_text("(A|B)&C"), "(A|B)&C");
        assert_eq!(filter_text("((1..2)|3)"), "((1..2)|3)");
        assert_eq!(filter_text("a(b)c"), "a(b)c");
    }

    #[test]
    fn a_quote_hides_the_parentheses_inside_it() {
        assert_eq!(filter_text("\"a)b\""), "\"a)b\"");
        assert_eq!(filter_text("\"a(b\""), "\"a(b\"");
        assert_eq!(filter_text("'a)b'"), "'a)b'");
        assert_eq!(filter_text("'a(b'"), "'a(b'");
        // A doubled apostrophe is one apostrophe inside the string, so the string goes on.
        assert_eq!(filter_text("'it''s ) fine'"), "'it''s ) fine'");
        // After the string closes, a parenthesis counts again.
        assert_eq!(filter_text("'x'|(y)"), "'x'|(y)");
    }

    #[test]
    fn an_unterminated_filter_or_quote_is_an_error() {
        assert!(parse("count(T where(A = filter(x)").is_err());
        assert!(parse("count(T where(A = filter('x)))").is_err());
        assert!(parse("count(T where(A = filter(\"x)))").is_err());
        assert!(parse("count(T where(A = filter((x)))").is_err());
    }

    #[test]
    fn formulas_that_end_early_are_errors_not_panics() {
        for bad in ["sum(T", "count(T", "sum(T.", "exist(T"] {
            assert!(parse(bad).is_err(), "`{bad}`");
        }
    }

    #[test]
    fn count_and_exist_take_no_field_and_the_others_need_one() {
        // `count(T.F)` reads as table `F` of namespace `T`; a name that resolves to no table is refused there.
        assert_eq!(parse("count(T.F)").unwrap().field, None);
        for bad in [
            "sum(T)",
            "lookup(T where(A = field(B)))",
            "min(T)",
            "max(T)",
            "average(T)",
        ] {
            let e = parse(bad).unwrap_err();
            assert!(e.contains("needs a source field"), "{bad}: {e}");
        }
    }

    #[test]
    fn upperlimit_and_filter_are_flow_filter_functions_only_with_parentheses() {
        // A field that is simply called UpperLimit or Filter.
        let f = parse("sum(T.F where(A = field(UpperLimit), B = field(Filter), C = field(\"filter\")))").unwrap();
        assert_eq!(
            f.conds,
            vec![
                c("A", CondValue::Field("UpperLimit".into())),
                c("B", CondValue::Field("Filter".into())),
                c("C", CondValue::Field("filter".into())),
            ]
        );
        // A quoted name is never the function, even with parentheses after it.
        assert!(parse("sum(T.F where(A = field(\"upperlimit\"(X))))").is_err());
    }
}
