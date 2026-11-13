//! `Record.SetFilter` whose whole filter text has an empty side of `|` (al-runner issue 7764 item 2): BC raises Filter:CSide
//! before it reads a value. Only the measured shapes take this path ([`empty_or_side`]); every other text goes on to the parser
//! and keeps its old outcome.

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

/// The measured cells, the whole filter text by equality. `V` is one literal value (letters and digits on a Text or Code field,
/// digits on an Integer field) with no placeholder argument, `%1` one placeholder with exactly one argument that is not empty.
///
/// - Probe (corpus PR 1382, record/filters/SetFilterEmptyTerm.al, BC 27.5, 28.5 and 29.0): Text `%1|` and `|%1` with `'A'`
///   (:123), `A|`, `|A`, `A||B`, `|` (:142); Code `%1|` with `'A'` (:161), `A|`, `|A`, `A||B` (:177); Integer `%1|` and `|%1`
///   with `1`, `1||2` (:193); Text `%1|` with a Text literal argument (:222). `V|` and `V||V` name the right side, `|V` and
///   `|` the left side.
/// - Pinned: Integer `|10` and `10|` (record/crud/RcAFilters.al:376, :378), Text `||` (RcAFilters.al:648, the left side).
fn empty_or_side(text: &str, args: &[Value], ty: &RtType) -> Option<(Side, Evidence)> {
    let text_field = matches!(ty, RtType::Text(_) | RtType::Code(_));
    let int_field = matches!(ty, RtType::Integer);
    if !text_field && !int_field {
        return None;
    }
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
                v.bytes().all(|b| b.is_ascii_digit())
            } else {
                v.bytes().all(|b| b.is_ascii_alphanumeric())
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
                Value::Text(s) | Value::Code(s) => text_field && !s.is_empty(),
                Value::Integer(_) => int_field,
                _ => false,
            };
            match text {
                "%1|" if filled => Side::Right,
                "|%1" if filled => Side::Left,
                _ => return None,
            }
        }
        _ => return None,
    };
    Some((side, Evidence::Probe))
}

impl Session {
    /// The Filter:CSide error of `Record.SetFilter(field, text, args..)` (called from AL code) when `text` is a measured shape
    /// of [`empty_or_side`]; `None` sends the call on. The measured calls ran in filter group 0, in an English session, with no
    /// MakeIntFilter subscriber on an Integer field.
    pub(crate) fn setfilter_empty_or_side(&mut self, rec: &Rec, args: &[Value]) -> Result<Option<Signal>, Signal> {
        let [Value::Integer(pos), filter, values @ ..] = args else {
            return Ok(None);
        };
        let (Value::Text(text) | Value::Code(text)) = filter else {
            return Ok(None);
        };
        if rec.borrow().filter_group != 0 || self.global_language != 1033 {
            return Ok(None);
        }
        let t = self.table(rec)?;
        let pos = *pos as usize;
        let Some(field) = t.fields.get(pos) else {
            return Ok(None);
        };
        let Some((side, evidence)) = empty_or_side(text, values, &field.ty) else {
            return Ok(None);
        };
        if matches!(field.ty, RtType::Integer)
            && self
                .engine
                .events
                .is_subscribed(crate::events::FILTER_HELPER_TRIGGERS, "makeintfilter")
        {
            return Ok(None);
        }
        if evidence == Evidence::Probe {
            assume(Assumption::SetfilterEmptyOrSide);
        }
        let side = match side {
            Side::Left => "left",
            Side::Right => "right",
        };
        Ok(Some(crate::transactions::al_error_code(
            &format!(
                "The filter \"{text}\" is not valid for the {} field on the {} table. The {side} side of '|' operators cannot be empty.",
                self.field_caption(&t, pos),
                self.caption(&t)
            ),
            "Filter:CSide",
        )))
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
    }
}
