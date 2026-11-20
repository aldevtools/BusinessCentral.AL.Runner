//! GoToKey on a list that opened on the last row of its descending view in a key other than the primary key, the rows above it
//! read up (`read_rows_up_from_the_landed_last_row`; al-runner issues 6363 and 8172).

use super::*;
use al_builtins::record_op::RecordOp;

/// The measured cells of rows held by the open read and rows above the top row held (corpus
/// testpage/WdModalGoToKeyDescSecondary.al): 30 rows hold 21 with 9 above (:146 D01, :165 D30), 5 rows hold all 5 with none
/// above (:184 D02).
const MEASURED_HELD_ABOVE: [(usize, usize); 2] = [(21, 9), (5, 0)];

/// Whether the open read held `held` rows with `above` rows above them, a cell the corpus measures.
fn held_and_above_measured(held: usize, above: usize) -> bool {
    MEASURED_HELD_ABOVE.contains(&(held, above))
}

impl Session {
    /// GoToKey on a list that opened as `read_rows_up_from_the_landed_last_row` models, the cursor still on the last row: the client
    /// reads from the top row it holds up to the row before the first of the view, then moves from the first row down one row at a
    /// time, each move reading the row three times and running OnAfterGetCurrRecord, until the key (WdModalGoToKeyDescSecondary.al:146
    /// `GoToKey log` `R:D21,...,R:D29,R:D30,R:D30,R:D30,C:D30,R:D29,R:D29,R:D29,C:D29,...,C:D01` for D01 of 30 rows, :165 _First
    /// for D30, :184 _FiveRows for D02 of 5 rows `R:D05,R:D05,R:D05,C:D05,...,C:D02`). Pinned for a key the view has, in the cells
    /// of `MEASURED_HELD_ABOVE`; other counts of rows held and above, a key the view does not have, and a second GoToKey or a move
    /// after it are refused.
    pub(super) fn go_to_key_from_the_last_row_read_up(&mut self, st: &St, want: &[Value]) -> Result<Value, Signal> {
        let rec = self.page_rec(st);
        let here = self.row_key(&rec)?;
        let on_last = {
            let s = st.borrow();
            s.read_up_from_last == ReadUpFromLast::Opened
                && s.cursor == Cursor::Row
                && !s.draft
                && !s.stale
                && s.window.last().is_some_and(|w| Self::same_key(&w.key, &here))
        };
        if !on_last {
            return unsupported(
                "GoToKey on a list that opened on the last row of its descending view in a key other than the primary key, after a move or a GoToKey (not pinned)",
            );
        }
        let up = self.rows_up_from_the_top_held(st, &rec)?;
        let held = st.borrow().window.len();
        if !held_and_above_measured(held, up.len() - 1) {
            return unsupported(
                "GoToKey on a list that opened on the last row of its descending view, with a number of rows held and rows above them that no case measured (not pinned)",
            );
        }
        let table = self.table(&rec)?;
        let mut rows: Vec<crate::pages::WindowRow> = up
            .iter()
            .rev()
            .map(|f| crate::pages::WindowRow {
                key: table.primary_key(f),
                stored: f.clone(),
                shown: f.clone(),
            })
            .collect();
        rows.extend(st.borrow().window.iter().skip(1).cloned());
        let Some(target) = rows.iter().position(|w| Self::same_key(&w.key, want)) else {
            return unsupported(
                "GoToKey for a key the view does not have, on a list that opened on the last row of its descending view (not pinned)",
            );
        };
        // The reads up to the row before the first one.
        for f in up.iter().take(up.len() - 1) {
            rec.borrow_mut().fields = f.clone();
            self.populate_current(st)?;
        }
        {
            let mut s = st.borrow_mut();
            s.window = rows;
            s.window_from_start = true;
            s.read_up_from_last = ReadUpFromLast::WalkedFromFirst;
        }
        for i in 0..=target {
            let r = self.read_client_row_in(st, i, true);
            self.torn_down_on_error(st, r)?;
        }
        Ok(Value::Boolean(true))
    }

    /// The fields of the top row held and of each row above it up to the first of the view, read on a copy of the record; no
    /// trigger runs. More rows above than a window holds are refused.
    fn rows_up_from_the_top_held(
        &mut self,
        st: &St,
        rec: &Rc<RefCell<RecordState>>,
    ) -> Result<Vec<Vec<Value>>, Signal> {
        let top = st.borrow().window[0].key.clone();
        let probe = Rc::new(RefCell::new(rec.borrow().clone()));
        self.ret_used = true;
        if !matches!(
            self.record_op(RecordOp::Get, &probe, &mut top.clone())?,
            Value::Boolean(true)
        ) {
            return unsupported("GoToKey on a list whose top row held is gone from the table (not pinned)");
        }
        let mut up = vec![probe.borrow().fields.clone()];
        loop {
            self.ret_used = true;
            if !matches!(self.record_op(RecordOp::Next, &probe, &mut [Value::Integer(-1)])?, Value::Integer(n) if n != 0)
            {
                return Ok(up);
            }
            if up.len() > Self::WINDOW {
                return unsupported(
                    "GoToKey on a list that opened on the last row of its descending view, with more rows above the rows it holds than a window (not pinned)",
                );
            }
            up.push(probe.borrow().fields.clone());
        }
    }
}
