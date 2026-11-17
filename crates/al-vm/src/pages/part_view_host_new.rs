//! `New()` on a ListPart of a host the test opened with OpenView. BC denies it: "New method failed because Insert is not
//! allowed. Page = <part>, Id = <id>.", code DB:ClientInsertDenied, and no trigger runs
//! (tests/al-language/pages/parts/PartSplitKeyInsertTiming.al:1353 Doc_OpenView_New `view|text`, `log` `[]`; :1383
//! Card_OpenView_New). The measured shape, a direct part of that page, raises that error; every other part below such a
//! page, at any depth, refuses the `New()`.

use super::pinned_shape::{PartStep, PinnedPart, TestCall};
use super::*;
use al_builtins::page_op::PageMethod;

impl Session {
    /// Before a part's `New()` leaves its row: on a part at any depth below a page the test opened with OpenView, the measured
    /// shape (a direct part of that page) raises BC's error, any other shape refuses. A part of any other page keeps the
    /// earlier path.
    pub(super) fn part_new_on_view_host(&mut self, st: &St) -> Result<(), Signal> {
        let Some(host) = st.borrow().host.as_ref().and_then(|(h, _)| h.upgrade()) else {
            return Ok(());
        };
        let root = super::shown::shown_root(&host);
        if !root.borrow().test_open_view() {
            return Ok(());
        }
        if !Rc::ptr_eq(&root, &host) || !self.view_host_new_pinned(st)? {
            return unsupported(
                "New() on a part of a page the test opened with OpenView, other than the measured shape (not pinned)",
            );
        }
        Err(self.new_denied_error(st)?)
    }

    /// The measured shape, every fact required:
    /// - the pages (`part_pages_are`): a Card (:1383) or a Document (:1353) host with no trigger whose controls are fields of Rec
    ///   and this part; a ListPart with AutoSplitKey and no DelayedInsert whose triggers are OnNewRecord and OnInsertRecord
    ///   (page 62724), controls over fields of Rec without a trigger; no page extension, table extension or subscriber;
    /// - `SubPageLink` on the primary key fields but the last, an Integer, and a SubPageView in a secondary key, ascending
    ///   (pages 62727, 62729); no SourceTableView on the part page;
    /// - the test's calls since the host opened: `GoToKey`, the part's `Last`, `Editable()` of a part control answering No, then this `New`
    ///   (:1359..:1366; value reads in between).
    fn view_host_new_pinned(&self, st: &St) -> Result<bool, Signal> {
        fn pin(host_type: ir::PageType) -> PinnedPart<'static> {
            PinnedPart {
                host_type,
                host_triggers: &[],
                part_triggers: &["onnewrecord", "oninsertrecord"],
                part_properties: &["autosplitkey"],
                part_variable_controls: false,
                part_variable_validate: false,
                part_field_validate_update: false,
                part_expression_controls: false,
                unlinked: false,
                sibling_parts: false,
                user_filter: false,
            }
        }
        use PageMethod as M;
        use PartStep::{Host, Own, OwnNo};
        use TestCall::{Other, Page};
        let calls = matches!(
            Self::part_steps(st).as_deref(),
            Some([
                Host(Page(M::GoToKey)),
                Own(Page(M::Last)),
                OwnNo(Other(M::Editable)),
                Own(Page(M::New))
            ])
        );
        let order = {
            let r = self.page_rec(st);
            let r = r.borrow();
            r.key != 0 && r.ascending
        } && self
            .object_property(st.borrow().object as usize, "SourceTableView")
            .is_none();
        Ok(calls
            && order
            && self.has_auto_split_key(st)
            && self.declares(st, "onnewrecord")
            && self.declares(st, "oninsertrecord")
            && (self.part_pages_are(st, &pin(ir::PageType::Card))?
                || self.part_pages_are(st, &pin(ir::PageType::Document))?)
            && self.part_key_is_link_then_integer(st)?)
    }
}
