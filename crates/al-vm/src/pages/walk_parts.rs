//! What the parts of a host read while a GoToKey or GoToRecord walks the host's rows from the first one to the row it finds
//! (`Session::page_go_to_key` in `rows.rs`, `go_to_key_current_only` in `gotokey_find.rs`): for each row the walk passes whose lines they do not already show, the parts read
//! their rows again in two shapes. Every other host keeps the reads it had: its parts read only at the row the walk finds.

use super::*;
use al_builtins::page_op::PageMethod;

/// A host whose parts read again at each row the walk passes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum WalkParts {
    /// A Document host (pages/parts/QjPartPropagation.al:1183 QL_Both_GoToKey_Counts `log-gotokey-h2-again` = H1 with the
    /// part's reads, then H2 with them; :1210 QL_Default_GoToKey_Counts).
    DocumentHost,
    /// A top-level List host whose ListPart showed no row and landed in a cell of a host with the page triggers of probe 1378, at
    /// a GoToRecord that is the test's first call since the open (`Session::passed_row_cell`).
    TriggerCellList,
}

impl Session {
    /// Whether the parts of `st` read again at each row the walk passes, and in which shape (see the module).
    pub(super) fn walk_reads_parts(&self, st: &St) -> Option<WalkParts> {
        let document = {
            let object = st.borrow().object as usize;
            self.engine.object_facts[object].page_type == Some(al_ir::PageType::Document)
        } && !self.is_list(st)
            && st.borrow().host.is_none()
            && !st.borrow().parts.is_empty();
        if document {
            return Some(WalkParts::DocumentHost);
        }
        self.trigger_cell_walk(st).then_some(WalkParts::TriggerCellList)
    }

    /// The walk of `st` passes a row whose lines its parts do not show: in the shape `walk`, they read their rows there. On the
    /// List host of `WalkParts::TriggerCellList` only where the part lands in a cell there (`passed_row_cell`); elsewhere it
    /// reads nothing, as before. Sets `walk_synced_parts` when the parts read.
    pub(super) fn sync_parts_passed(&mut self, st: &St, walk: Option<WalkParts>) -> Result<(), Signal> {
        let synced = match walk {
            Some(WalkParts::DocumentHost) => self.sync_parts(st).map(|()| true)?,
            Some(WalkParts::TriggerCellList) => {
                self.during_host_move(HostMove::passing(PageMethod::GoToRecord), |s| {
                    let cell = s.passed_row_cell(st)?;
                    if cell {
                        s.sync_parts(st)?;
                    }
                    Ok(cell)
                })?
            }
            None => false,
        };
        if synced {
            st.borrow_mut().walk_synced_parts = true;
        }
        Ok(())
    }

    /// A list with OnAfterGetCurrRecord alone among its row triggers steps from its first row to the key one row at a time
    /// (`go_to_key_current_only`): at each row it passes, the part of a `WalkParts::TriggerCellList` host reads again
    /// (`sync_parts_passed`); every other host's parts read nothing there, as before.
    pub(super) fn sync_parts_stepped(&mut self, st: &St) -> Result<(), Signal> {
        let walk = self.trigger_cell_walk(st).then_some(WalkParts::TriggerCellList);
        self.sync_parts_passed(st, walk)
    }
}
