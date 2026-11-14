//! A Provider chain whose Provider part shows no row, with the linked part added by a page extension outside FactBoxes
//! (al-runner issue 7392). BC shows no row in the linked part (ProviderPartOutsideFactBoxes.al:419 `h2`, with a detail row
//! of `Line Ref` 0 stored at :273);
//! the runner links it to the blank values instead, which shows the same only while no row holds them. The probe's cells run
//! under the assumption `provider-no-row-ext-chain` (corpus pull request 1384, testpage/parts/ProviderNoRowFirst.al); every
//! other shape keeps the refusal in `sync_part` (`NO_ROW_CHAIN_REFUSAL`).

use super::pinned_shape::{NextEnd, PLAIN_PROPERTIES, PinnedEnd, TestCall};
use super::*;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::filter::FieldFilter;
use al_builtins::page_op::PageMethod;

/// The refusal of a part linked through a Provider part that shows no row, outside the cells this module admits.
pub(super) const NO_ROW_CHAIN_REFUSAL: &str = "a part linked through a Provider part that shows no row, on a Provider chain declared in the base page's FactBoxes or added by a page extension outside FactBoxes (BC shows no row; not modelled)";

/// What the Card lets through after the landing: the calls ProviderNoRowFirst.al makes after it, value reads of a part's
/// control (:204-205) and the moves of a part (:202-203, :250, :288-291), while the test writes nothing to the store.
const LANDED_END: PinnedEnd = PinnedEnd {
    reason: "a call of the test on a Card whose Provider part shows no row, after the test wrote to the store, or other than a \
             value read of a part's control, First, Last or Next of a part, or Close (not pinned)",
    reads: false,
    caption: false,
    part_reads: true,
    one_action: false,
    first_ends: false,
    part_visible: false,
    part_moves: true,
    no_writes: true,
    next: NextEnd::Refused,
};

/// The globals the compiler gives every page; a page of the probe declares no other.
const IMPLICIT_PAGE_GLOBALS: [&str; 3] = ["Rec", "xRec", "CurrPage"];

/// The first field number of the system fields; the probe's own fields declare no property (:18-55).
const FIRST_SYSTEM_FIELD: u32 = 2_000_000_000;

impl Session {
    /// Whether part `child` (control `control` of `host`), whose Provider part `provider` shows no row, is in a cell
    /// ProviderNoRowFirst.al measured, as far as the pages and the calls tell (asked before any link value is built). Every
    /// fact is required:
    /// - the landing: the test opened the Card with OpenView and its first call is a GoToRecord (:192 ExtOutside_NoLines_
    ///   OpenView_First, :240 _DetailsFirstOnly, :259 _NoMove, :278 _LastNext), or with OpenEdit and a GoToKey (:301
    ///   ExtOutside_NoLines_OpenEditGoToKey_First), the host on a stored row; later syncs only under the `LANDED_END` that
    ///   landing set;
    /// - the pages as `probe_chain_pages` lists them.
    pub(super) fn provider_no_row_shape(
        &self,
        host: &St,
        control: usize,
        child: &St,
        provider: &St,
    ) -> Result<bool, Signal> {
        let landing = host.borrow().provider_no_row_landed || {
            let calls = Self::test_calls_all_succeeded(host);
            let h = host.borrow();
            h.pinned_end.is_none()
                && h.cursor == Cursor::Row
                && !h.draft
                && ((h.test_open_view() && calls.as_deref() == Some(&[TestCall::Page(PageMethod::GoToRecord)]))
                    || (h.test_open_edit() && calls.as_deref() == Some(&[TestCall::Page(PageMethod::GoToKey)])))
        };
        Ok(landing && self.probe_chain_pages(host, control, child, provider)?)
    }

    /// Whether part `child` of `host`, which passed `provider_no_row_shape`, may be linked with `link` (the blank values): no
    /// stored row of the part's table passes it (the probe's table holds none, so the blank link shows no row there). The first
    /// time it holds, the Card takes `LANDED_END`, which refuses every call after a write to the store.
    pub(super) fn provider_no_row_rows(&mut self, host: &St, child: &St, link: &[FieldFilter]) -> Result<bool, Signal> {
        let rec = self.page_rec(child);
        if self.stored_rows_passing(&rec, link.to_vec())? != 0 {
            return Ok(false);
        }
        assume(Assumption::ProviderNoRowExtChain);
        let mut h = host.borrow_mut();
        if !h.provider_no_row_landed {
            h.provider_no_row_landed = true;
            h.pinned_end = Some(LANDED_END);
        }
        Ok(true)
    }

    /// The pages of ProviderNoRowFirst.al (:18-55 the tables, :57 the Card, :78 the Provider part, :97 the linked part, :147
    /// the extension):
    /// - the host: a top-level Card with no property but the plain ones, no trigger, no routine, no variable, no subscriber,
    ///   exactly one page extension; its controls are fields of Rec with exactly the property ApplicationArea (:68) and
    ///   exactly two parts, no control trigger and no property a `modify` block replaced, none in FactBoxes;
    /// - the extension: no property, no trigger, no routine, no variable; it adds only part `control`, right after the
    ///   Provider part (`addafter`, :151);
    /// - the Provider part control: from the page itself, no Provider, one `field(..)` entry (:72); part `control`: Provider =
    ///   that part, two `field(..)` entries (:157); each entry joins the leading fields of the part's primary key, in order, to
    ///   the leading fields of the primary key of the page it links to, in order, two fields of one type; no property but
    ///   ApplicationArea, SubPageLink and Provider;
    /// - both part pages: ListParts with Editable = false and no other property but the plain ones, no trigger, no routine,
    ///   no variable, no extension, no subscriber; their controls are one repeater and in it fields of Rec with exactly the
    ///   property ApplicationArea and no trigger (:85-94, :104-113);
    /// - the source tables of the three pages: no property, no routine, no table extension, no subscriber, and their own
    ///   fields declare no property.
    fn probe_chain_pages(&self, host: &St, control: usize, child: &St, provider: &St) -> Result<bool, Signal> {
        let table_of = |p: &St| self.table(&self.page_rec(p)).map(|t| t.object as usize);
        let tables = [table_of(host)?, table_of(provider)?, table_of(child)?];
        let engine = self.engine();
        let program = &engine.program;
        let object = host.borrow().object as usize;
        let page = page_of(&engine, host);
        let page_type = |s: &St| engine.object_facts[s.borrow().object as usize].page_type.clone();
        let [Some(host_table), Some(provider_table), Some(child_table)] =
            tables.map(|t| program.objects[t].table.as_ref())
        else {
            return Ok(false);
        };
        let Some(provider_idx) = page.controls[control].part_link.as_ref().and_then(|l| l.provider) else {
            return Ok(false);
        };
        let Some(&ext) = page.extensions.first() else {
            return Ok(false);
        };
        let card = host.borrow().host.is_none()
            && page_type(host) == Some(ir::PageType::Card)
            && page.triggers.is_empty()
            && page.ext_triggers.is_empty()
            && page.extensions == [ext]
            && declares_no_code(&program.objects[object])
            && declares_no_code(&program.objects[ext])
            && program.objects[ext].properties.is_empty()
            && !engine.events.any_subscriber_of(object)
            && program.objects[object]
                .properties
                .iter()
                .all(|(k, _)| PLAIN_PROPERTIES.contains(&k.to_ascii_lowercase().as_str()));
        let link_props = |c: &ir::Control| {
            c.props.iter().all(|(k, v)| {
                matches!(v, ir::PropVal::Lit(_))
                    && ["applicationarea", "subpagelink", "provider"].contains(&k.to_ascii_lowercase().as_str())
            })
        };
        let mut parts = 0;
        let controls = page.controls.iter().enumerate().all(|(i, c)| {
            let plain = c.triggers.is_empty() && c.ext_triggers.is_empty() && c.ext_replaced.is_empty() && !c.factbox;
            match c.kind {
                ir::ControlKind::Field => {
                    plain && c.owner.is_none() && matches!(c.binding, ir::Binding::Field(_)) && only_application_area(c)
                }
                ir::ControlKind::Part => {
                    parts += 1;
                    let Some(l) = c.part_link.as_ref() else {
                        return false;
                    };
                    plain
                        && link_props(c)
                        && if i == provider_idx {
                            c.owner.is_none()
                                && l.provider.is_none()
                                && l.entries.len() == 1
                                && link_on_keys(l, provider_table, host_table)
                        } else {
                            i == control
                                && control == provider_idx + 1
                                && c.owner == Some(ext)
                                && l.provider == Some(provider_idx)
                                && l.entries.len() == 2
                                && link_on_keys(l, child_table, provider_table)
                        }
                }
                _ => false,
            }
        });
        if !(card && controls && parts == 2) {
            return Ok(false);
        }
        let part_pages = [provider, child].iter().all(|p| {
            let o = p.borrow().object as usize;
            matches!(page_type(p), Some(ir::PageType::Other(t)) if t == "listpart")
                && declares_no_code(&program.objects[o])
                && one_repeater_of_fields(&page_of(&engine, p).controls)
        });
        let tables_plain = tables.iter().all(|&t| {
            let o = &program.objects[t];
            o.routines.is_empty()
                && o.properties.is_empty()
                && o.table.as_ref().is_some_and(|x| {
                    x.extensions.is_empty()
                        && x.fields
                            .iter()
                            .filter(|f| f.id < FIRST_SYSTEM_FIELD)
                            .all(|f| f.properties.is_empty())
                })
                && !engine.events.any_subscriber_of(t)
        });
        drop(engine);
        if !(part_pages && tables_plain) {
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
        Ok(true)
    }
}

/// Whether object `o` declares no routine (no trigger, procedure or expression property) and no variable of its own.
fn declares_no_code(o: &ir::Object) -> bool {
    o.routines.is_empty()
        && o.globals
            .iter()
            .all(|g| IMPLICIT_PAGE_GLOBALS.iter().any(|n| g.name.eq_ignore_ascii_case(n)))
}

/// Whether control `c` declares exactly the property ApplicationArea, as a literal.
fn only_application_area(c: &ir::Control) -> bool {
    matches!(c.props.as_slice(), [(k, ir::PropVal::Lit(_))] if k.eq_ignore_ascii_case("applicationarea"))
}

/// Whether `controls` are one repeater with no property and no trigger, and in it fields of Rec with exactly the property
/// ApplicationArea and no trigger.
fn one_repeater_of_fields(controls: &[ir::Control]) -> bool {
    let Some((repeater, fields)) = controls.split_first() else {
        return false;
    };
    let bare = |c: &ir::Control| c.triggers.is_empty() && c.ext_triggers.is_empty() && c.ext_replaced.is_empty();
    repeater.kind == ir::ControlKind::Repeater
        && repeater.parent.is_none()
        && repeater.props.is_empty()
        && bare(repeater)
        && !fields.is_empty()
        && fields.iter().all(|c| {
            c.kind == ir::ControlKind::Field
                && c.parent == Some(0)
                && matches!(c.binding, ir::Binding::Field(_))
                && only_application_area(c)
                && bare(c)
        })
}

/// Whether every entry of `l` is a `field(..)` entry that joins the leading field of `part`'s primary key at its position to
/// the field of `source`'s primary key at the same position, both of one type (:72 Code[20] to Code[20]; :157 Code[20] to
/// Code[20], Integer to Integer).
fn link_on_keys(l: &ir::PartLink, part: &ir::Table, source: &ir::Table) -> bool {
    l.entries.iter().enumerate().all(|(i, e)| match e {
        ir::LinkEntry::Field(pf, hf) => {
            part.keys.first().and_then(|k| k.get(i)) == Some(pf)
                && source.keys.first().and_then(|k| k.get(i)) == Some(hf)
                && part.fields[*pf].ty == source.fields[*hf].ty
        }
        _ => false,
    })
}
