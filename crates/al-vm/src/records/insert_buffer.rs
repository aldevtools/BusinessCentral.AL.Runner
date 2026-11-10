//! The insert buffer of a table and a duplicate-key Insert that is still pending (al-runner issue 7407).

use super::*;
use crate::state::TxWriteKind;

/// Microsoft Learn on `BufferedInsertEnabled`: "up to 5 rows will be buffered in the table queue before they are inserted".
const BUFFER_ROWS: usize = 5;

impl Session {
    /// The refusal for an asserterror body that ended with a duplicate-key Insert pending, when that Insert was the 5th or a
    /// later insert into its table since the table was last read, written otherwise or committed. A duplicate that BC finds while
    /// the insert buffer is sent may raise inside the asserterror, so the runner's answer, no error, is not known to be right.
    /// The corpus pins the deferred error only for fewer inserts in a row (errors/last-error/InsertDuplicateKeyExcludeFlag.al,
    /// record/TestRecordRuntimeTextsAndViews.al); the probe for the rest is corpus pull request 1279 (docs/probes-needed.md,
    /// `duplicate-insert-buffer-flush`).
    pub(crate) fn pending_duplicate_in_full_buffer(&self) -> Option<String> {
        let (object, _) = self.tx.deferred_insert.as_ref()?;
        let table = *object as u32;
        // The pending duplicate counts as an insert; the writes since the last Modify, Delete or Rename of the table are inserts.
        let mut inserts = 1;
        for w in self.tx.writes.iter().rev() {
            if w.table != table {
                continue;
            }
            if w.kind != TxWriteKind::Insert {
                break;
            }
            inserts += 1;
        }
        (inserts >= BUFFER_ROWS).then(|| {
            format!(
                "an asserterror body that ends with a pending duplicate-key Insert into {} after {} inserts in one transaction: whether BC raises it inside the asserterror once the insert buffer is full is not pinned",
                self.engine.program.objects[*object].name,
                inserts - 1
            )
        })
    }
}
