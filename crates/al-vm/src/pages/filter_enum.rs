//! TestFilter.GetFilter of an Enum field, the forms a typed filter takes (corpus master pages/view-filters/
//! Y76TestFilterEnum.al, enum with ordinals 0, 1, 2, 4, 7):
//! - a range reads as its ordinals, open ends too (`Second..` = `1..`, `..Fifth` = `..4`, :122 and :147);
//! - `>`, `>=`, `<`, `<=` read as the list of the members they keep, by name (`>Second` = `Third|Fifth|Eighth`, :147);
//! - a list with a range in it reads every part as an ordinal (`First..Third|Eighth` = `0..2|7`, `First|Third..Fifth` =
//!   `0|2..4`, :179); a list of single values keeps the names (`First|Third|Fifth`), but a SourceTableView list reads as
//!   ordinals (`First|Third|Fifth` = `0|2|4`, Y76_SourceTableView_Range);
//! - `<>` parts joined by `&` read as ordinals (`<>0&<>1&<>2`, :179).

use super::*;
use al_builtins::filter::{FieldFilter, FilterExpr};

/// The group of a SourceTableView's filters.
const VIEW_GROUP: i32 = 2;

impl Session {
    /// The text of `expr` on the Enum `id`, or `None` for a form this module does not read (the default text applies).
    pub(super) fn enum_filter_text(&self, id: u32, expr: &FilterExpr, group: i32) -> Result<Option<String>, Signal> {
        let ord = |v: &Value| match v {
            Value::Enum(_, o) => Some(*o),
            _ => None,
        };
        let name = |o: i32| self.filter_text(&FilterExpr::Eq(Value::Enum(id, o)));
        Ok(match expr {
            FilterExpr::Range(Some(a), None) => ord(a).map(|x| format!("{x}..")),
            FilterExpr::Range(None, Some(b)) => ord(b).map(|y| format!("..{y}")),
            FilterExpr::Gt(v) | FilterExpr::Ge(v) | FilterExpr::Lt(v) | FilterExpr::Le(v) => {
                let Some(x) = ord(v) else { return Ok(None) };
                let keep = |o: i32| match expr {
                    FilterExpr::Gt(_) => o > x,
                    FilterExpr::Ge(_) => o >= x,
                    FilterExpr::Lt(_) => o < x,
                    _ => o <= x,
                };
                let mut members: Vec<i32> = self.engine.program.objects[id as usize]
                    .enum_values
                    .iter()
                    .map(|(o, _)| *o)
                    .filter(|o| keep(*o))
                    .collect();
                members.sort_unstable();
                if members.is_empty() {
                    return unsupported("TestFilter.GetFilter of an Enum comparison that keeps no member (not pinned)");
                }
                Some(members.into_iter().map(name).collect::<Result<Vec<_>, _>>()?.join("|"))
            }
            FilterExpr::Or(parts) => {
                let mut texts = Vec::new();
                let mut has_range = false;
                for p in parts {
                    match p {
                        FilterExpr::Eq(v) => match ord(v) {
                            Some(x) => texts.push(x.to_string()),
                            None => return Ok(None),
                        },
                        FilterExpr::Range(Some(a), Some(b)) => match (ord(a), ord(b)) {
                            (Some(x), Some(y)) if x == y => texts.push(x.to_string()),
                            (Some(x), Some(y)) => {
                                has_range = true;
                                texts.push(format!("{x}..{y}"));
                            }
                            _ => return Ok(None),
                        },
                        _ => return Ok(None),
                    }
                }
                (has_range || group == VIEW_GROUP).then(|| texts.join("|"))
            }
            FilterExpr::And(parts) => {
                let ords: Option<Vec<i32>> = parts
                    .iter()
                    .map(|p| match p {
                        FilterExpr::Ne(v) => ord(v),
                        _ => None,
                    })
                    .collect();
                ords.map(|os| os.iter().map(|o| format!("<>{o}")).collect::<Vec<_>>().join("&"))
            }
            _ => None,
        })
    }

    /// A part's link filter (filter group 4) on an Option or Enum field reads as the option numbers: a list of values
    /// `0|1|2`, one value `2`, `<>1` (corpus pages/link-quoted-member/Y42LinkQuotedMember.al:335, :379, :401, :423). The same
    /// when the part's OnOpenPage set the same values on the field first (:357 Lq_Option_PartAlsoSetsFilter, `0|1|2`); a
    /// first filter of other values is not pinned and keeps the default text. `None`: the default text applies.
    pub(super) fn part_link_ordinals(
        filters: &[FieldFilter],
        pos: usize,
        first: Option<&FieldFilter>,
    ) -> Option<String> {
        let link = filters
            .iter()
            .find(|f| f.field == pos && f.group == super::parts::LINK_GROUP)?;
        let text = Self::ordinals(&link.expr)?;
        let first = first?;
        (first.group == link.group || Self::ordinals(&first.expr).as_ref() == Some(&text)).then_some(text)
    }

    /// `expr` written with option numbers, for a list of single values, one value or `<>` one value.
    fn ordinals(expr: &FilterExpr) -> Option<String> {
        let ord = |v: &Value| match v {
            Value::Enum(_, o) | Value::Option(o, _) => Some(o.to_string()),
            _ => None,
        };
        let one = |e: &FilterExpr| match e {
            FilterExpr::Eq(v) => ord(v),
            FilterExpr::Range(Some(a), Some(b)) => ord(a).filter(|x| ord(b).as_ref() == Some(x)),
            _ => None,
        };
        match expr {
            FilterExpr::Ne(v) => ord(v).map(|o| format!("<>{o}")),
            FilterExpr::Or(parts) => parts.iter().map(one).collect::<Option<Vec<_>>>().map(|v| v.join("|")),
            e => one(e),
        }
    }
}
