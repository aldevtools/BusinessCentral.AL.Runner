//! The row a lookup page opens on when the field already holds a value
//! (pages/lt-y80-lookup-holds-value/Y80LookupHoldsValue.al).

use super::*;

/// The most rows of the related table the held-value lookup is pinned for (the probe table has four).
const PINNED_RELATED_ROWS: usize = 4;

impl Session {
    /// The related row a lookup over a held value opens on: the first row whose related field (`pos`, `None`: the first primary
    /// key field) equals the held value, text compared without case (Y80_LK_Card_HoldsLowerCase, _AltHoldsMiddle). A value that is
    /// no row, or a blank one, opens on the first row like a blank field (Y80_LK_Card_HoldsNoRow); that is `None`. A related table
    /// of more than four rows is refused: how much of it the page reads around the row is not pinned.
    pub(super) fn lookup_start_row(
        &mut self,
        related: &St,
        held: &Value,
        pos: Option<usize>,
    ) -> Result<Option<Vec<Value>>, Signal> {
        if !self.is_list(related) {
            return unsupported(
                "Lookup over a TableRelation from a field that already holds a value, through a page that is not a list (not pinned)",
            );
        }
        let rec = self.page_rec(related);
        let pos = match pos {
            Some(p) => p,
            None => self.table(&rec)?.keys[0][0],
        };
        let rows = self.view_rows(&rec, PINNED_RELATED_ROWS + 1)?;
        if rows.len() > PINNED_RELATED_ROWS {
            return unsupported(
                "Lookup over a TableRelation from a field that already holds a value, onto a table of more than four rows (how much of the table the page reads around the row is not pinned)",
            );
        }
        let held_text = match held {
            Value::Text(t) | Value::Code(t) => Some(t.to_string().to_lowercase()),
            _ => None,
        };
        Ok(rows.into_iter().find(|row| match (&held_text, &row[pos]) {
            (Some(h), Value::Text(t) | Value::Code(t)) => &t.to_string().to_lowercase() == h,
            (None, v) => Self::same_values(std::slice::from_ref(v), std::slice::from_ref(held)),
            _ => false,
        }))
    }

    /// What a lookup page does before it opens, over a field of a card or list: OnOpenPage runs once more, and for a held value that
    /// names a row the page opens on it, after its own OnOpenPage a first OnAfterGetRecord reads that row (`lead_read`) (Y80LookupHoldsValue.al `handler log` of every case:
    /// `open();open();R:G2;R:G2;R:G3;R:G4;R:G2;R:G1;R:G2;R:G2;C:G2`, a blank or unmatched value `open();open();R:G1;R:G2;...`). The page
    /// then opens on that row (`lookup_held_row`, `handed_or_group_list_on_row`).
    pub(super) fn lookup_pre_open(
        &mut self,
        related: &St,
        handed: &Rec,
        start: Option<Vec<Value>>,
    ) -> Result<(), Signal> {
        if self.declares(related, "onopenpage") {
            self.page_trigger(related, "onopenpage", vec![])?;
        }
        let Some(fields) = start else { return Ok(()) };
        handed.borrow_mut().fields = fields.clone();
        related.borrow_mut().lookup_held_row = true;
        Ok(())
    }

    /// The read of the held row that precedes the rows of the open (see `lookup_pre_open`).
    pub(super) fn lead_read_of_held_row(&mut self, st: &St) -> Result<(), Signal> {
        if self.declares(st, "onaftergetrecord") {
            self.page_trigger(st, "onaftergetrecord", vec![])?;
        }
        Ok(())
    }
}
