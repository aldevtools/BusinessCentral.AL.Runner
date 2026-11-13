//! `Record.SetFilter` whose whole filter text has an empty side of `|` (al-runner issue 7764 item 2): BC raises Filter:CSide.
//! The parser stops such a text with `filter::EMPTY_TERM`, which main refuses; at that refusal (`op_inner.rs`) only the measured
//! shapes ([`empty_or_side`]) on the measured record ([`Session::measured_record`]) raise the error, every other call keeps the
//! refusal.

use super::*;
use al_builtins::assumptions::{Assumption, assume};

/// Which side of the `|` BC names in the error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    Left,
    Right,
}

/// Whether the shape is pinned by the pinned corpus (no assumption is marked) or measured only by the open probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Evidence {
    Pinned,
    Probe,
}

/// The measured cells, the whole filter text by equality. `V` is one literal value with no placeholder argument (letters and
/// digits on a Text or Code field, no longer than the field; digits that read as an Integer on an Integer field), `%1` one
/// placeholder with exactly one argument (a non-empty Text no longer than the field on a Text field, an Integer on an Integer
/// field).
///
/// - Probe (corpus PR 1382, record/filters/SetFilterEmptyTerm.al): Text `%1|` and `|%1` with `'A'` (:123), `A|`, `|A`, `A||B`,
///   `|` (:142); Code `%1|` with `'A'` (:161), `A|`, `|A`, `A||B` (:177); Integer `%1|` and `|%1` with `1`, `1||2` (:193);
///   Text `%1|` with a Text literal argument (:222). `V|` and `V||V` name the right side, `|V` and `|` the left side. Code
///   `|%1` was not measured.
/// - Pinned: Integer `|10` and `10|` (record/crud/RcAFilters.al:376, :378), Text `||` (RcAFilters.al:648, the left side).
fn empty_or_side(text: &str, args: &[Value], ty: &RtType) -> Option<(Side, Evidence)> {
    let (text_len, int_field) = match ty {
        RtType::Text(len) | RtType::Code(len) => (*len, false),
        RtType::Integer => (None, true),
        _ => return None,
    };
    let fits = |v: &str| text_len.is_some_and(|n| v.chars().count() <= n as usize);
    if int_field && args.is_empty() && text == "|10" {
        return Some((Side::Left, Evidence::Pinned));
    }
    if int_field && args.is_empty() && text == "10|" {
        return Some((Side::Right, Evidence::Pinned));
    }
    if matches!(ty, RtType::Text(_)) && args.is_empty() && text == "||" {
        return Some((Side::Left, Evidence::Pinned));
    }
    let literal = |v: &str| {
        !v.is_empty()
            && if int_field {
                v.bytes().all(|b| b.is_ascii_digit()) && v.parse::<i32>().is_ok()
            } else {
                v.bytes().all(|b| b.is_ascii_alphanumeric()) && fits(v)
            }
    };
    let side = match args {
        [] => match text.split('|').collect::<Vec<_>>()[..] {
            [v, ""] if literal(v) => Side::Right,
            ["", v] if literal(v) => Side::Left,
            [a, "", b] if literal(a) && literal(b) => Side::Right,
            ["", ""] if matches!(ty, RtType::Text(_)) => Side::Left,
            _ => return None,
        },
        [arg] => {
            let filled = match arg {
                Value::Text(s) | Value::Code(s) => !int_field && !s.is_empty() && fits(s),
                Value::Integer(_) => int_field,
                _ => false,
            };
            let leading_measured = matches!(ty, RtType::Text(_) | RtType::Integer);
            match text {
                "%1|" if filled => Side::Right,
                "|%1" if filled && leading_measured => Side::Left,
                _ => return None,
            }
        }
        _ => return None,
    };
    Some((side, Evidence::Probe))
}

impl Session {
    /// The Filter:CSide error of `Record.SetFilter(field, text, args..)`, called from AL code (`from_al`), whose `text` the parser
    /// stopped with `filter::EMPTY_TERM`, when `text` is a measured shape of [`empty_or_side`] on the measured record; `None`
    /// keeps main's refusal.
    pub(crate) fn setfilter_empty_or_side(
        &self,
        from_al: bool,
        t: &TableDef,
        pos: usize,
        rec: &Rec,
        args: &[Value],
    ) -> Option<Signal> {
        let [_, Value::Text(text) | Value::Code(text), values @ ..] = args else {
            return None;
        };
        let field = t.fields.get(pos)?;
        let (side, evidence) = empty_or_side(text, values, &field.ty)?;
        if !(from_al && self.measured_record(t, pos, rec)) {
            return None;
        }
        if evidence == Evidence::Probe {
            assume(Assumption::SetfilterEmptyOrSide);
        }
        let reason = match side {
            Side::Left => al_builtins::filter::LEFT_OF_OR_EMPTY,
            Side::Right => al_builtins::filter::RIGHT_OF_OR_EMPTY,
        };
        Some(self.filter_error(t, pos, text, reason))
    }

    /// The record and field both the pins and the probe used (RcAObjects.al:16-31 table 71500; SetFilterEmptyTerm.al:15-24 table
    /// 71907; the calls RcAFilters.al:361-378 and SetFilterEmptyTerm.al:98-104): filter group 0 of a record that is not temporary
    /// and holds no filter, an English session, a table numbered below 2000000000 that states
    /// no TableType (or Normal) and whose caption is its name, a Normal field with no TableRelation whose caption is its name, and
    /// on an Integer field no MakeIntFilter subscriber.
    fn measured_record(&self, t: &TableDef, pos: usize, rec: &Rec) -> bool {
        let (group, temporary, filtered) = {
            let r = rec.borrow();
            (r.filter_group, r.temporary, !r.filters.is_empty())
        };
        let object = &self.engine.program.objects[t.object as usize];
        let field = &t.fields[pos];
        group == 0
            && !temporary
            && !filtered
            && self.global_language == 1033
            && object.id.is_some_and(|id| id < 2_000_000_000)
            && matches!(
                self.engine.object_facts[t.object as usize].table_type,
                None | Some(al_ir::TableType::Normal)
            )
            && self.engine.field_facts[t.object as usize][pos].class == al_ir::FieldClass::Normal
            && object
                .table
                .as_ref()
                .and_then(|s| s.fields.get(pos))
                .is_some_and(|f| !f.relation)
            && self.caption(t) == t.name
            && self.field_caption(t, pos) == field.name
            && match field.ty {
                RtType::Integer => !self
                    .engine
                    .events
                    .is_subscribed(crate::events::FILTER_HELPER_TRIGGERS, "makeintfilter"),
                _ => true,
            }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text_ty() -> RtType {
        RtType::Text(Some(50))
    }

    #[test]
    fn only_the_measured_shapes_match() {
        let a = [Value::Text("A".into())];
        assert_eq!(
            empty_or_side("A|", &[], &text_ty()),
            Some((Side::Right, Evidence::Probe))
        );
        assert_eq!(
            empty_or_side("%1|", &a, &text_ty()),
            Some((Side::Right, Evidence::Probe))
        );
        assert_eq!(
            empty_or_side("||", &[], &text_ty()),
            Some((Side::Left, Evidence::Pinned))
        );
        assert_eq!(
            empty_or_side("|10", &[], &RtType::Integer),
            Some((Side::Left, Evidence::Pinned))
        );
        for (t, args) in [
            ("A|B||C", &[][..]),
            (" A|", &[]),
            ("A|", &a),
            ("%1|", &[]),
            ("%1||", &a),
            ("|A|", &[]),
        ] {
            assert_eq!(empty_or_side(t, args, &text_ty()), None, "{t}");
        }
        assert_eq!(empty_or_side("||", &[], &RtType::Code(Some(20))), None);
        assert_eq!(empty_or_side("A|", &[], &RtType::Integer), None);
        assert_eq!(empty_or_side("1|", &[], &RtType::Decimal), None);
        assert_eq!(empty_or_side("%1|", &[Value::Text("".into())], &text_ty()), None);
        assert_eq!(empty_or_side("2147483648|", &[], &RtType::Integer), None);
        assert_eq!(
            empty_or_side("2147483647|", &[], &RtType::Integer),
            Some((Side::Right, Evidence::Probe))
        );
        assert_eq!(empty_or_side("ABCD|", &[], &RtType::Text(Some(3))), None);
        assert_eq!(empty_or_side("|%1", &a, &RtType::Code(Some(20))), None);
        assert_eq!(
            empty_or_side("%1|", &a, &RtType::Code(Some(20))),
            Some((Side::Right, Evidence::Probe))
        );
    }
}
