//! `TableRelation` property text → arms (before names are resolved).
//!
//! ```text
//! TableRelation = if (Kind = const(A)) "Parent".Code where("Group" = field("Own Group"))
//!                 else if (Kind = const(B)) Other
//!                 else Third.Field;
//! ```
//!
//! Only these shapes are understood; anything else makes the whole relation
//! `None`, so the runtime refuses it instead of guessing (docs/probes-needed.md).

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedArm {
    /// `(own field name, constant text)` of each `Field = const(x)` condition. [`parse_lenient`]
    /// also keeps `Field = filter(x)` ones.
    pub conditions: Vec<(String, String)>,
    /// Parallel to `conditions`: the condition is a `filter(...)`. Only [`parse_lenient`] sets it.
    pub filter_conditions: Vec<bool>,
    /// Parallel to `conditions`: the text between the parentheses as written (quotes kept), which a
    /// `filter(...)` needs: `filter('')` is the blank value, not an empty filter text.
    pub condition_raw: Vec<String>,
    /// The related table (and optionally its field) as written: `["Microsoft", "Sales", "Customer", "No."]`.
    pub path: Vec<String>,
    /// `None` when a where() clause is present but not understood.
    pub wheres: Option<Vec<ParsedWhere>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedWhere {
    /// `RelatedField = const(text)` or `= filter(text)` (`filter`).
    Value {
        related: String,
        text: String,
        filter: bool,
    },
    /// `RelatedField = filter('<>''''')`: a filter written as one AL string with a doubled quote in it; `text` is the
    /// string's content (`<>''`).
    QuotedFilter { related: String, text: String },
    /// `RelatedField = field(OwnField)`.
    Field { related: String, own: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Tok {
    Word(String),
    Quoted(String),
    Str(String),
    Sym(&'static str),
    /// The raw text between a balanced `(` and its `)`.
    Group(String),
}

fn lex(src: &str) -> Option<Vec<Tok>> {
    let c: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < c.len() {
        let ch = c[i];
        if ch.is_whitespace() {
            i += 1;
        } else if ch == '"' {
            let end = c[i + 1..].iter().position(|x| *x == '"')? + i + 1;
            out.push(Tok::Quoted(c[i + 1..end].iter().collect()));
            i = end + 1;
        } else if ch == '\'' {
            let mut s = String::new();
            i += 1;
            loop {
                match c.get(i)? {
                    '\'' if c.get(i + 1) == Some(&'\'') => {
                        s.push('\'');
                        i += 2;
                    }
                    '\'' => {
                        i += 1;
                        break;
                    }
                    x => {
                        s.push(*x);
                        i += 1;
                    }
                }
            }
            out.push(Tok::Str(s));
        } else if ch.is_alphanumeric() || ch == '_' {
            let start = i;
            while i < c.len() && (c[i].is_alphanumeric() || c[i] == '_') {
                i += 1;
            }
            out.push(Tok::Word(c[start..i].iter().collect()));
        } else if ch == '(' {
            // Keep a parenthesised group's text raw, quotes respected, so `const(Database::"X")`
            // and `filter(<>'')` survive as written; the structure inside is read on demand.
            let close = crate::scan::matching_paren(&c, i)?;
            out.push(Tok::Group(c[i + 1..close].iter().collect()));
            i = close + 1;
        } else if ch == '.' {
            out.push(Tok::Sym("."));
            i += 1;
        } else if ch == ',' {
            out.push(Tok::Sym(","));
            i += 1;
        } else if ch == '=' {
            out.push(Tok::Sym("="));
            i += 1;
        } else {
            return None;
        }
    }
    Some(out)
}

struct P {
    t: Vec<Tok>,
    i: usize,
    /// Accept `Field = filter(x)` conditions too (for metadata, which only names the field).
    lenient: bool,
}

impl P {
    fn word_is(&self, w: &str) -> bool {
        matches!(self.t.get(self.i), Some(Tok::Word(x)) if x.eq_ignore_ascii_case(w))
    }

    fn name(&mut self) -> Option<String> {
        let r = match self.t.get(self.i)? {
            Tok::Word(w) | Tok::Quoted(w) => w.clone(),
            _ => return None,
        };
        self.i += 1;
        Some(r)
    }

    fn group(&mut self) -> Option<String> {
        match self.t.get(self.i)? {
            Tok::Group(g) => {
                let g = g.clone();
                self.i += 1;
                Some(g)
            }
            _ => None,
        }
    }

    fn sym(&mut self, s: &str) -> Option<()> {
        match self.t.get(self.i)? {
            Tok::Sym(x) if *x == s => {
                self.i += 1;
                Some(())
            }
            _ => None,
        }
    }

    /// `const(text)` / `filter(text)` / `field(name)` after a `=`: its keyword and inner text.
    fn kind_group(&mut self) -> Option<(String, String)> {
        let kw = match self.t.get(self.i)? {
            Tok::Word(w) => w.to_ascii_lowercase(),
            _ => return None,
        };
        self.i += 1;
        Some((kw, self.group()?))
    }

    fn arm(&mut self) -> Option<ParsedArm> {
        let mut conditions = Vec::new();
        let mut filter_conditions = Vec::new();
        let mut condition_raw = Vec::new();
        if self.word_is("if") {
            self.i += 1;
            let g = self.group()?;
            for cond in crate::scan::split_slices(&g, ',') {
                let mut p = P {
                    t: lex(cond)?,
                    i: 0,
                    lenient: false,
                };
                let field = p.name()?;
                p.sym("=")?;
                let (kw, text) = p.kind_group()?;
                let filter = kw == "filter" && self.lenient;
                if (kw != "const" && !filter) || p.i != p.t.len() {
                    return None;
                }
                condition_raw.push(text.trim().to_string());
                conditions.push((field, crate::scan::unquote_relation(&text)));
                filter_conditions.push(filter);
            }
        }
        let mut path = vec![self.name()?];
        while self.sym(".").is_some() {
            path.push(self.name()?);
        }
        let mut wheres = Some(Vec::new());
        if self.word_is("where") {
            self.i += 1;
            wheres = self.group().and_then(|g| {
                crate::scan::split_slices(&g, ',')
                    .into_iter()
                    .map(|w| {
                        let mut p = P {
                            t: lex(w)?,
                            i: 0,
                            lenient: false,
                        };
                        let related = p.name()?;
                        p.sym("=")?;
                        let (kw, text) = p.kind_group()?;
                        if p.i != p.t.len() {
                            return None;
                        }
                        match kw.as_str() {
                            "filter" if whole_quoted_filter(&text).is_some() => Some(ParsedWhere::QuotedFilter {
                                related,
                                text: whole_quoted_filter(&text)?,
                            }),
                            "const" | "filter" => Some(ParsedWhere::Value {
                                related,
                                text: crate::scan::unquote_relation(&text),
                                filter: kw == "filter",
                            }),
                            "field" => Some(ParsedWhere::Field {
                                related,
                                own: crate::scan::unquote_relation(&text),
                            }),
                            _ => None,
                        }
                    })
                    .collect()
            });
        }
        Some(ParsedArm {
            conditions,
            filter_conditions,
            condition_raw,
            path,
            wheres,
        })
    }
}

/// The content of a `filter(...)` text written as one AL string that has a doubled quote in it (`'<>'''''` is `<>''`); `None`
/// for any other text, which [`crate::scan::unquote_relation`] reads. A content with `"` in it keeps the old reading.
fn whole_quoted_filter(text: &str) -> Option<String> {
    let inner = crate::queries::whole_literal(text)?;
    (inner.contains('\'') && !inner.contains('"')).then_some(inner)
}

/// The arms of a `TableRelation` value, or `None` when its shape is not understood.
pub fn parse(src: &str) -> Option<Vec<ParsedArm>> {
    parse_with(src, false)
}

/// The arms of a `TableRelation` for the Table Relations Metadata table, which also lists the
/// arms whose conditions are `Field = filter(x)`. The runtime does not understand those
/// (`parse` is `None`); `filter_conditions` marks them.
pub fn parse_lenient(src: &str) -> Option<Vec<ParsedArm>> {
    parse_with(src, true)
}

fn parse_with(src: &str, lenient: bool) -> Option<Vec<ParsedArm>> {
    let mut p = P {
        t: lex(src)?,
        i: 0,
        lenient,
    };
    let mut arms = vec![p.arm()?];
    while p.word_is("else") {
        p.i += 1;
        arms.push(p.arm()?);
    }
    (p.i == p.t.len()).then_some(arms)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arm(conds: &[(&str, &str)], path: &[&str], wheres: Option<Vec<ParsedWhere>>) -> ParsedArm {
        ParsedArm {
            conditions: conds.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect(),
            filter_conditions: conds.iter().map(|_| false).collect(),
            condition_raw: conds.iter().map(|(_, b)| b.to_string()).collect(),
            path: path.iter().map(|s| s.to_string()).collect(),
            wheres,
        }
    }

    #[test]
    fn plain_and_qualified_targets() {
        // corpus _fixtures/tables/ALTRelationWhereField.al: `"ALT Rel Where Parent"."Code"`
        assert_eq!(
            parse("\"ALT Rel Where Parent\".\"Code\""),
            Some(vec![arm(&[], &["ALT Rel Where Parent", "Code"], Some(vec![]))])
        );
        assert_eq!(parse("Location"), Some(vec![arm(&[], &["Location"], Some(vec![]))]));
        // recordref-fieldref/fieldref/TestFieldRefRelation.al:270-325: namespace-qualified targets keep every part
        assert_eq!(
            parse("Microsoft.Manufacturing.Forecast.\"Production Forecast Name\".Name"),
            Some(vec![arm(
                &[],
                &[
                    "Microsoft",
                    "Manufacturing",
                    "Forecast",
                    "Production Forecast Name",
                    "Name"
                ],
                Some(vec![])
            )])
        );
    }

    #[test]
    fn conditional_arms_with_where_clauses() {
        let src =
            "if (Kind = const(A)) \"ALT Rel Where Parent\".\"Code\" where(\"Parent Group\" = field(\"Child Group\"))
            else
            \"ALT Relation Parent B\".\"Code\"";
        assert_eq!(
            parse(src),
            Some(vec![
                arm(
                    &[("Kind", "A")],
                    &["ALT Rel Where Parent", "Code"],
                    Some(vec![ParsedWhere::Field {
                        related: "Parent Group".into(),
                        own: "Child Group".into()
                    }])
                ),
                arm(&[], &["ALT Relation Parent B", "Code"], Some(vec![])),
            ])
        );
        // else-if chains (_fixtures/tables/ALTRelationFieldClass.al `Cond On Filter Ref`)
        let chain = "if (\"Kind Filter\" = const(A)) X.Code else if (\"Kind Filter\" = const(B)) Y.Code";
        let arms = parse(chain).unwrap();
        assert_eq!(arms.len(), 2);
        assert_eq!(arms[1].conditions, vec![("Kind Filter".to_string(), "B".to_string())]);
    }

    #[test]
    fn constants_filters_and_object_references_in_where() {
        // record/CdcOwner.Table.al:67: `where("Table ID" = const(Database::"CDC Owner"))`, DvtFlowRow: `const(Date)`
        let a = &parse("\"CDC Ref Row\".\"Code\" where(\"Table ID\" = const(Database::\"CDC Owner\"))").unwrap()[0];
        assert_eq!(
            a.wheres,
            Some(vec![ParsedWhere::Value {
                related: "Table ID".into(),
                text: "Database::\"CDC Owner\"".into(),
                filter: false
            }])
        );
        let a = &parse("T.F where(A = const(1), B = filter(<>''))").unwrap()[0];
        assert_eq!(a.wheres.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn shapes_not_understood_are_none_not_guessed() {
        assert_eq!(parse("if (const(X)) A.B"), None);
        assert_eq!(parse("A.B where(C = upperlimit(D))").unwrap()[0].wheres, None);
        assert_eq!(parse("A.B unexpected"), None);
        assert_eq!(parse(""), None);
    }

    #[test]
    fn a_filter_condition_is_only_understood_by_the_metadata_parse() {
        // Base Application "Sales Line"."No.": `"Document Type" = filter(<> "Credit Memo")`.
        let src = "if (Type = const(Item), \"Document Type\" = filter(<> \"Credit Memo\")) Item where(Blocked = const(false))";
        assert_eq!(parse(src), None);
        let arms = parse_lenient(src).expect("lenient");
        assert_eq!(arms[0].conditions.len(), 2);
        assert_eq!(arms[0].filter_conditions, vec![false, true]);
        assert_eq!(arms[0].path, vec!["Item".to_string()]);
    }
}
