//! A Provider chain whose Provider part shows no row, with the linked part added by a page extension outside FactBoxes
//! (al-runner issue 7392). BC shows no row in the linked part (ProviderPartOutsideFactBoxes.al `h2`, corpus pull request 1324);
//! the runner links it to the blank values instead, which shows the same only while no row holds them. The probe's cells run
//! under the assumption `provider-no-row-ext-chain` (corpus pull request 1384, testpage/parts/ProviderNoRowFirst.al); every
//! other shape keeps the refusal in `sync_part`.

use super::pinned_shape::{NextEnd, PLAIN_PROPERTIES, PinnedEnd, TestCall};
use super::*;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::filter::FieldFilter;
use al_builtins::page_op::PageMethod;

/// What the Card lets through after the landing: the calls ProviderNoRowFirst.al makes after it.
const LANDED_END: PinnedEnd = PinnedEnd {
    reason: "a call of the test on a Card whose Provider part shows no row, other than a value read, First, Last or Next of a \
             part, or Close (not pinned)",
    reads: true,
    one_action: false,
    first_ends: false,
    part_visible: false,
    part_moves: true,
    next: NextEnd::Refused,
};

impl Session {
    /// Whether part `child` (control `control` of `host`), whose Provider part `provider` shows no row, may be linked with `link` (the blank
    /// values) because the cell is one ProviderNoRowFirst.al measured. Every fact is required:
    /// - the landing: the test opened the Card with OpenView and its first call is a GoToRecord (:192 ExtOutside_NoLines_
    ///   OpenView_First, :240 _DetailsFirstOnly, :259 _NoMove, :278 _LastNext), or with OpenEdit and a GoToKey (:301
    ///   ExtOutside_NoLines_OpenEditGoToKey_First), the host on a stored row; later syncs only under the `LANDED_END` that
    ///   landing set;
    /// - the pages as `probe_chain_pages` lists them;
    /// - no stored row of the part's table passes `link` (the probe's table holds none, so the blank link shows no row there).
    pub(super) fn provider_no_row_ext_chain(
        &mut self,
        host: &St,
        control: usize,
        child: &St,
        provider: &St,
        link: &[FieldFilter],
    ) -> Result<bool, Signal> {
        let landed = host.borrow().provider_no_row_landed;
        let landing = landed || {
            let calls = Self::test_calls_all_succeeded(host);
            let h = host.borrow();
            h.pinned_end.is_none()
                && h.cursor == Cursor::Row
                && !h.draft
                && ((h.test_open_view() && calls.as_deref() == Some(&[TestCall::Page(PageMethod::GoToRecord)]))
                    || (h.test_open_edit() && calls.as_deref() == Some(&[TestCall::Page(PageMethod::GoToKey)])))
        };
        if !(landing && self.probe_chain_pages(host, control, child, provider)?) {
            return Ok(false);
        }
        let rec = self.page_rec(child);
        if self.stored_rows_passing(&rec, link.to_vec())? != 0 {
            return Ok(false);
        }
        assume(Assumption::ProviderNoRowExtChain);
        if !landed {
            let mut h = host.borrow_mut();
            h.provider_no_row_landed = true;
            h.pinned_end = Some(LANDED_END);
        }
        Ok(true)
    }

    /// The pages of ProviderNoRowFirst.al (:57 the Card, :78 the Provider part, :97 the linked part, :147 the extension):
    /// - the host: a top-level Card with no property but the plain ones, no trigger, no subscriber, exactly one page
    ///   extension, which declares no trigger; its controls are fields of Rec and exactly two parts, all with literal properties
    ///   and no trigger, the Provider part and fields from the page itself, part `control` from the extension, neither in
    ///   FactBoxes;
    /// - the Provider part control: one `field(..)` link entry, no Provider; part `control`: Provider = that part, two
    ///   `field(..)` entries; both with no property but ApplicationArea, SubPageLink and Provider;
    /// - both part pages: ListParts with Editable = false and no other property but the plain ones, no trigger, no extension,
    ///   no subscriber;
    /// - the source tables of the three pages: no routine, no table extension, no subscriber.
    fn probe_chain_pages(&self, host: &St, control: usize, child: &St, provider: &St) -> Result<bool, Signal> {
        let engine = self.engine();
        let object = host.borrow().object as usize;
        let page = page_of(&engine, host);
        let page_type = |s: &St| engine.object_facts[s.borrow().object as usize].page_type.clone();
        let card = host.borrow().host.is_none()
            && page_type(host) == Some(ir::PageType::Card)
            && page.triggers.is_empty()
            && page.ext_triggers.is_empty()
            && page.extensions.len() == 1
            && !engine.events.any_subscriber_of(object)
            && engine.program.objects[object]
                .properties
                .iter()
                .all(|(k, _)| PLAIN_PROPERTIES.contains(&k.to_ascii_lowercase().as_str()));
        let Some(provider_idx) = page.controls[control].part_link.as_ref().and_then(|l| l.provider) else {
            return Ok(false);
        };
        let ext = page.extensions.first().copied();
        let link_props = |c: &ir::Control| {
            c.props.iter().all(|(k, v)| {
                matches!(v, ir::PropVal::Lit(_))
                    && ["applicationarea", "subpagelink", "provider"].contains(&k.to_ascii_lowercase().as_str())
            })
        };
        let fields_only = |l: &ir::PartLink, n: usize| {
            l.entries.len() == n && l.entries.iter().all(|e| matches!(e, ir::LinkEntry::Field(..)))
        };
        let mut parts = 0;
        let controls = page.controls.iter().enumerate().all(|(i, c)| {
            let plain = c.triggers.is_empty() && c.ext_triggers.is_empty() && !c.factbox;
            match c.kind {
                ir::ControlKind::Field => {
                    plain
                        && c.owner.is_none()
                        && matches!(c.binding, ir::Binding::Field(_))
                        && c.props.iter().all(|(_, v)| matches!(v, ir::PropVal::Lit(_)))
                }
                ir::ControlKind::Part => {
                    parts += 1;
                    let Some(l) = c.part_link.as_ref() else {
                        return false;
                    };
                    plain
                        && link_props(c)
                        && if i == provider_idx {
                            c.owner.is_none() && l.provider.is_none() && fields_only(l, 1)
                        } else {
                            i == control
                                && ext.is_some()
                                && c.owner == ext
                                && l.provider == Some(provider_idx)
                                && fields_only(l, 2)
                        }
                }
                _ => false,
            }
        });
        if !(card && controls && parts == 2) {
            return Ok(false);
        }
        let list_parts = [provider, child]
            .iter()
            .all(|p| matches!(page_type(p), Some(ir::PageType::Other(t)) if t == "listpart"));
        drop(engine);
        if !list_parts {
            return Ok(false);
        }
        for p in [provider, child] {
            let part_object = p.borrow().object as usize;
            if !(self.page_declares_within(p, &[], &["editable"])
                && flag_denied(&self.engine(), part_object, "Editable")?)
            {
                return Ok(false);
            }
        }
        for p in [host, provider, child] {
            let table = self.table(&self.page_rec(p))?.object as usize;
            let engine = self.engine();
            let t = &engine.program.objects[table];
            if !(t.routines.is_empty()
                && t.table.as_ref().is_some_and(|x| x.extensions.is_empty())
                && !engine.events.any_subscriber_of(table))
            {
                return Ok(false);
            }
        }
        Ok(true)
    }
}
