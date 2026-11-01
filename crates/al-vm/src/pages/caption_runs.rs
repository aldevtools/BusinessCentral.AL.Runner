//! How many times a page open runs the CaptionClass expressions of its source table's fields with an empty record (al-runner
//! issue 5870), where the corpus pins it.

use super::*;

impl Session {
    /// The page type whose CaptionClass runs a Worksheet page shares: it runs them as a List (pages/lt-y61-page-probes/
    /// Y61CaptionClassCount.al:492 G615_Worksheet_4DifferentFields, :639 G615_Worksheet_1Field). A ListPlus with its field in a
    /// group runs them as a Card on the open, also once with the row on GoToRecord (:663 G615_ListPlus_1Field); its moves are not
    /// built, so it is `None` like every other page.
    pub(super) fn caption_runs_like(&self, st: &St) -> Option<al_ir::PageType> {
        use al_ir::PageType;
        let s = st.borrow();
        match self.engine.object_facts[s.object as usize].page_type.as_ref()? {
            PageType::Worksheet if !s.modal && matches!(s.mode, PageMode::View | PageMode::Edit) => {
                Some(PageType::List)
            }
            _ => None,
        }
    }

    /// How many times each CaptionClass expression of the source table runs with an empty record when page `st` opens: a page of
    /// its own (no host, not modal) opened with OpenView or OpenEdit. Pinned by writer SX (record/lt-sx-captionclass-count/
    /// SxCaptionClassCount.al:190-369): one expression field, Card 8 and List 7; two with the same expression text, 9 and 8 each.
    /// And by writer Y61 (pages/lt-y61-page-probes/Y61CaptionClassCount.al, three cycles alike on every major): on a List each field
    /// runs 9 times for three fields with the same text (:540), 10 for four fields with different or the same texts (:468, :516),
    /// 21 for fifteen fields with different texts (:564); a Worksheet as a List (:492 four fields, :639 one). `None` for any other shape: the counts are pinned per number of fields only.
    pub(super) fn caption_empty_runs_pinned(&self, st: &St, exprs: &[(usize, String)]) -> Option<usize> {
        use al_ir::PageType;
        let page_type = {
            let s = st.borrow();
            if s.host.is_some() || s.modal || !matches!(s.mode, PageMode::View | PageMode::Edit) {
                return None;
            }
            self.engine.object_facts[s.object as usize].page_type.clone()
        };
        let card = match page_type {
            Some(PageType::Card) => true,
            Some(PageType::List) => false,
            Some(PageType::Worksheet) => self.caption_runs_like(st)? != PageType::List,
            _ => return None,
        };
        let same_text = exprs.iter().all(|(_, e)| e.trim() == exprs[0].1.trim());
        match (card, exprs.len()) {
            (true, 1) => Some(8),
            (true, 2) if same_text => Some(9),
            (false, 1) => Some(7),
            (false, n @ (2 | 3)) if same_text => Some(6 + n),
            (false, n @ (4 | 15)) => Some(6 + n),
            _ => None,
        }
    }
}
