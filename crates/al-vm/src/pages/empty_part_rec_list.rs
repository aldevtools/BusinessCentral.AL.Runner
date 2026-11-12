//! The List-host cells of the empty ListPart Rec (`empty_part_rec.rs`): a ListPart that shows no row, on a top-level List page,
//! holds the single values of its SubPageLink on Rec, as on a Card or Document host. Al-runner issue 6924, comment 6822249976
//! (the reporter's host is a List page with Base Application page 1178 in its FactBoxes). It runs through the assumption
//! `empty-part-rec-list-host` until corpus pull request 1339 (pages/parts/EmptyPartRecLinkList.al) is pinned.
//!
//! `list_host_cell` is the allowlist: every fact the probe holds fixed is required by equality, each against the probe object
//! that holds it. A shape with any other fact keeps main's result (`suite/empty_part_rec_list.rs`
//! `empty_list_part_rec_list_host_unmeasured_shapes_stay_blank`, `empty_list_part_rec_list_host_neighbors_keep_main`). The
//! facts no probe object varies are named in the entry's text.

use super::*;
use al_builtins::page_op::PageMethod;
use al_data::TableDef;

/// Page properties of the probe's List hosts (EmptyPartRecLinkList.al page 76843 "PG EPL List ConstField" and its siblings),
/// by lower-case name with the value they hold (`None`: any value; the property names an object, a usage or a label, not a
/// behavior). No probe page has a Caption; the entry `empty-part-rec-list-host` names it as assumed alike (page 1178 has one).
const HOST_PROPS: [(&str, Option<&str>); 5] = [
    ("pagetype", Some("list")),
    ("sourcetable", None),
    ("applicationarea", None),
    ("usagecategory", None),
    ("caption", None),
];

/// Page properties of the probe's parts: page 76842 "PG EPL Part" (the first four and UsageCategory), page 76851 "PG EPL Part
/// Doc" (also DeleteAllowed and DelayedInsert, the properties of page 1178), and a Caption (see `HOST_PROPS`).
const PART_PROPS: [(&str, Option<&str>); 8] = [
    ("pagetype", Some("listpart")),
    ("sourcetable", None),
    ("insertallowed", Some("false")),
    ("applicationarea", None),
    ("usagecategory", None),
    ("deleteallowed", Some("true")),
    ("delayedinsert", Some("true")),
    ("caption", None),
];

/// The part properties every probe part sets: a ListPart without the new-row line (page 76842, as page 1178).
const PART_REQUIRED: [(&str, &str); 2] = [("pagetype", "listpart"), ("insertallowed", "false")];

/// The page triggers of a probe part, sorted: page 76842 (OnAfterGetCurrRecord), page 76851 (page 1178's set).
const PART_TRIGGERS: [&[&str]; 2] = [
    &["onaftergetcurrrecord"],
    &["onaftergetcurrrecord", "ondeleterecord", "oninit"],
];

/// The control triggers of a probe part, sorted: page 76842 (the action that reads Rec), page 76851 (and a field's OnDrillDown).
const PART_CONTROL_TRIGGERS: [&[&str]; 2] = [&["onaction"], &["onaction", "ondrilldown"]];

/// The page triggers of a probe host, sorted: page 76843 (none), page 76853 "PG EPL List Triggers".
const HOST_TRIGGERS: [&[&str]; 2] = [&[], &["onaftergetcurrrecord", "onaftergetrecord", "onopenpage"]];

/// Properties of the part control on the host (page 76843 `part(Box; ..)`). UpdatePropagation = Both (page 76854 "PG EPL List
/// DocBoth") is not observed yet.
const PART_CONTROL_PROPS: [(&str, Option<&str>); 2] = [("applicationarea", None), ("subpagelink", None)];

impl Session {
    /// Whether part `st` of `host` is a List-host cell the probe measured (see the module): the host, the part control on it,
    /// the link and the part page each hold only facts a probe cell holds.
    pub(super) fn list_host_cell(&self, st: &St, host: &St) -> Result<bool, Signal> {
        let Some(control) = st.borrow().host.as_ref().map(|(_, c)| *c) else {
            return Ok(false);
        };
        if !(self.list_host_measured(host) && self.list_landing_measured() && self.list_part_page_measured(st)) {
            return Ok(false);
        }
        let host_table = self.table(&self.page_rec(host))?;
        let part_table = self.table(&self.page_rec(st))?;
        Ok(self.list_part_control_measured(host, control, &host_table, &part_table))
    }

    /// The host: a top-level List page the test opened with TestPage.OpenEdit or OpenView, standing on a stored row, with the
    /// properties and triggers of a probe host, no page extension and no event subscriber.
    fn list_host_measured(&self, host: &St) -> bool {
        let engine = self.engine();
        let (object, top_list, opened, stored_row) = {
            let h = host.borrow();
            let object = h.object as usize;
            (
                object,
                h.host.is_none() && engine.object_facts[object].page_type == Some(al_ir::PageType::List),
                h.opened_by_test && matches!(h.mode, PageMode::Edit | PageMode::View),
                h.cursor == Cursor::Row && !h.draft,
            )
        };
        top_list
            && opened
            && stored_row
            && props_measured(&engine.program.objects[object].properties, &HOST_PROPS)
            && trigger_set_measured(&page_of(&engine, host).triggers, &HOST_TRIGGERS)
            && self.unextended_unsubscribed(host)
    }

    /// The part lands as the host opens (EmptyPartRecLinkList.al :347), after a GoToRecord that left the row (:379), after one
    /// that stayed on it (a List with one row and the first of two rows, EmptyPartRecLinkList.al
    /// EmptyPart_ListOpenEdit_OneHead_GoToSameRow_FactBox_ActionReadsRec, EmptyPart_ListOpenEdit_GoToFirstRow_FactBox_ActionReadsRec)
    /// or after Next (:434).
    fn list_landing_measured(&self) -> bool {
        self.part_open_running
            || matches!(
                self.host_move_running,
                Some(HostMove {
                    method: PageMethod::GoToRecord | PageMethod::Next,
                    ..
                })
            )
    }

    /// The part control: the List page's own (no page extension), with the properties of a probe part control, no Provider and
    /// the Provider of no other part, and a link of `const(..)`, `filter(..)` and `field(..)` entries of the probe's types:
    /// a const or filter on an Integer field, a field link from a Code field to a Code field, none on the last key field.
    fn list_part_control_measured(
        &self,
        host: &St,
        control: usize,
        host_table: &TableDef,
        part_table: &TableDef,
    ) -> bool {
        let engine = self.engine();
        let page = page_of(&engine, host);
        let c = &page.controls[control];
        let Some(link) = c.part_link.as_ref() else {
            return false;
        };
        let props = c.props.iter().all(|(k, v)| {
            PART_CONTROL_PROPS.iter().any(|(name, want)| {
                k == name
                    && match (want, v) {
                        (None, ir::PropVal::Lit(_)) => true,
                        (Some(want), ir::PropVal::Lit(v)) => v.trim().eq_ignore_ascii_case(want),
                        _ => false,
                    }
            })
        });
        let provider_of_none = page
            .controls
            .iter()
            .all(|o| o.part_link.as_ref().and_then(|l| l.provider) != Some(control));
        let short_of_last_key = |f: usize| {
            part_table.keys[0]
                .iter()
                .position(|k| *k == f)
                .is_some_and(|i| i + 1 < part_table.keys[0].len())
        };
        let integer = |f: usize| part_table.fields[f].ty == al_builtins::value::RtType::Integer;
        let code = |ty: &al_builtins::value::RtType| matches!(ty, al_builtins::value::RtType::Code(_));
        let entries = link.entries.iter().all(|e| match e {
            ir::LinkEntry::Filter(f, _) => integer(*f) && short_of_last_key(*f),
            ir::LinkEntry::Field(f, hf) => {
                code(&part_table.fields[*f].ty) && code(&host_table.fields[*hf].ty) && short_of_last_key(*f)
            }
            ir::LinkEntry::FieldFilter(..) => false,
        });
        c.owner.is_none() && props && link.view.is_none() && link.provider.is_none() && provider_of_none && entries
    }

    /// The part page: a ListPart on a stored table with the properties and triggers of a probe part, no page extension and no
    /// event subscriber.
    fn list_part_page_measured(&self, st: &St) -> bool {
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let page = page_of(&engine, st);
        let mut control_triggers: Vec<&str> = page
            .controls
            .iter()
            .flat_map(|c| c.triggers.iter().map(|(n, _)| n.as_str()))
            .collect();
        control_triggers.sort_unstable();
        control_triggers.dedup();
        let props = &engine.program.objects[object].properties;
        props_measured(props, &PART_PROPS)
            && PART_REQUIRED.iter().all(|(name, want)| {
                props
                    .iter()
                    .any(|(k, v)| k.eq_ignore_ascii_case(name) && v.trim().eq_ignore_ascii_case(want))
            })
            && trigger_set_measured(&page.triggers, &PART_TRIGGERS)
            && PART_CONTROL_TRIGGERS.contains(&control_triggers.as_slice())
            && self.unextended_unsubscribed(st)
    }
}

/// Every property in `props` is one of `allowed`, with the value it names (case-insensitive).
fn props_measured(props: &[(String, String)], allowed: &[(&str, Option<&str>)]) -> bool {
    props.iter().all(|(k, v)| {
        allowed
            .iter()
            .any(|(name, want)| k.eq_ignore_ascii_case(name) && want.is_none_or(|w| v.trim().eq_ignore_ascii_case(w)))
    })
}

/// The page's trigger names, sorted, are exactly one of the `measured` sets.
fn trigger_set_measured(triggers: &[(String, usize)], measured: &[&[&str]]) -> bool {
    let mut names: Vec<&str> = triggers.iter().map(|(n, _)| n.as_str()).collect();
    names.sort_unstable();
    measured.contains(&names.as_slice())
}
