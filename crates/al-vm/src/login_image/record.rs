//! A record buffer (al-builtins `RecordState`) in the arena, field by field. The state is read
//! field by field, never cloned: `Clone` blanks `own_read`, `probed_source`, `globals` and the dirty base.

use super::NotImaged;
use super::arena::{Frozen, NodeId};
use super::freeze::{Holder, Walker};
use super::plain::PlainFilter;
use super::thaw::Maker;
use al_builtins::value::{
    Buffer, DirtyBase, FetchedRow, FieldSet, LoadState, OwnRead, OwnReadAt, ProbedSource, RecordState, RowOrigin,
    TableGlobals, Value,
};

/// What a record buffer holds. `crm` (`true` means no image) and `run_page_link` (a page's state: `Some` means no image)
/// are not kept.
#[derive(Debug)]
pub(crate) struct RecordNode {
    table: u32,
    temporary: bool,
    /// The node of its temporary table.
    temp: Option<NodeId>,
    company: Option<String>,
    change_company_called: bool,
    fields: Vec<Frozen>,
    filters: Vec<PlainFilter>,
    decimal_cut_filters: Vec<PlainFilter>,
    filter_group: i32,
    marks: Vec<Vec<Frozen>>,
    marked_only: bool,
    read_isolation: i32,
    security_filtering: i32,
    declared_security_filtering: i32,
    declared_security_filtering_unpinned: bool,
    security_filtering_unpinned: bool,
    security_filtering_array: bool,
    key: usize,
    adhoc: Option<(usize, Vec<usize>)>,
    ascending: bool,
    field_descending: u32,
    position: Option<Vec<Frozen>>,
    auto_calc: Vec<usize>,
    load: LoadNode,
    touched: bool,
    assigned: bool,
    inserted: bool,
    dirty_base: Option<Vec<Frozen>>,
    is_dirty_base: DirtyNode,
    last_modify: Option<Vec<Frozen>>,
    failed_insert: Option<i64>,
    row_origin: Option<RowOrigin>,
    own_read: Option<OwnReadNode>,
    probed_source: (u8, u64),
    /// The node of its table globals set (`None`: unset).
    globals: Option<NodeId>,
    unpinned_buffer: Option<&'static str>,
    view_assumption: Option<al_builtins::assumptions::Assumption>,
    auto_calc_dropped: bool,
}

/// `DirtyBase`, variant by variant.
#[derive(Debug)]
enum DirtyNode {
    Unknown(&'static str),
    Fresh,
    Synced,
    Row {
        base: Vec<Frozen>,
        alt: Option<Vec<Frozen>>,
        dirty_only: bool,
        stale_store: bool,
    },
}

/// `LoadState` with its `Buffer`, field by field; the fetched row is a node (its `Rc` may be shared).
#[derive(Debug)]
struct LoadNode {
    request: Option<Vec<usize>>,
    buffer: Option<BufferNode>,
    partial: bool,
}

#[derive(Debug)]
struct BufferNode {
    have: FieldSet,
    unsure: FieldSet,
    row: Option<String>,
    cleared: bool,
    assigned: Vec<usize>,
    missing_loaded: bool,
    fetched: Option<NodeId>,
    init: bool,
}

/// `OwnReadAt`, with the store's change count as the fact the render compares: whether it is the record's store's count
/// now (state_render.rs `own_read`). The thaw writes the thawn store's count where it is; else the old number, which no
/// later count equals (a count is handed out once in the process).
#[derive(Debug)]
struct OwnReadNode {
    got: bool,
    cleared: bool,
    system_id: Frozen,
    version: Option<i64>,
    commits: u64,
    store_changes: u64,
    current: bool,
}

impl RecordNode {
    /// The node of the record's temporary table.
    pub(crate) fn temp(&self) -> Option<NodeId> {
        self.temp
    }
}

impl Walker<'_> {
    /// A record buffer. Refused: a CRM table's record, a record a page linked (`run_page_link`), a temporary table whose
    /// table is a by-value copy's or holds rows not measured (`TempId`), a filter value with no plain form.
    pub(crate) fn record(&mut self, r: &RecordState, path: &str) -> Result<RecordNode, NotImaged> {
        let RecordState {
            table,
            temporary,
            temp_id,
            company,
            change_company_called,
            crm,
            fields,
            filters,
            decimal_cut_filters,
            filter_group,
            marks,
            marked_only,
            read_isolation,
            security_filtering,
            declared_security_filtering,
            declared_security_filtering_unpinned,
            security_filtering_unpinned,
            security_filtering_array,
            key,
            adhoc,
            ascending,
            field_descending,
            position,
            auto_calc,
            load,
            touched,
            assigned,
            inserted,
            dirty_base,
            is_dirty_base,
            last_modify,
            failed_insert,
            row_origin,
            own_read,
            probed_source,
            globals,
            unpinned_buffer,
            view_assumption,
            run_page_link,
            rolled_read,
            auto_calc_dropped,
        } = r;
        let refuse = |what: &str| NotImaged::Value {
            path: path.to_string(),
            what: what.to_string(),
        };
        if *crm {
            return Err(refuse("Record of a CRM table"));
        }
        if run_page_link.is_some() {
            return Err(refuse("Record with a page link"));
        }
        // The variable's own read of a row holding a rolled Time (al-vm `records/time_rollover.rs`) is not imaged.
        if rolled_read.0.is_some() {
            return Err(refuse("Record holding its own read of a rolled Time"));
        }
        let temp = match temp_id.as_ref().map(|t| (t, t.image_parts())) {
            Some((_, p)) if p.by_value_copy => return Err(refuse("temporary Record of a by-value copy's table")),
            Some((_, p)) if p.rows_unmeasured => return Err(refuse("temporary Record whose rows are not measured")),
            Some((t, _)) => Some(self.handle(Holder::TempTable(t.clone()), 1, format!("{path} temporary table"))),
            None => None,
        };
        let mut plain_filters = Vec::with_capacity(filters.len());
        for f in filters {
            plain_filters.push(PlainFilter::of(f).ok_or_else(|| super::freeze::no_form(path, "record filter"))?);
        }
        let mut plain_cut_filters = Vec::with_capacity(decimal_cut_filters.len());
        for f in decimal_cut_filters {
            plain_cut_filters.push(PlainFilter::of(f).ok_or_else(|| super::freeze::no_form(path, "record filter"))?);
        }
        let mut frozen_marks = Vec::with_capacity(marks.len());
        for m in marks {
            frozen_marks.push(self.field_values(m, &format!("{path} mark"))?);
        }
        let rows = |w: &mut Self, v: &Option<Vec<Value>>, what: &str| {
            v.as_deref()
                .map(|v| w.field_values(v, &format!("{path} {what}")))
                .transpose()
        };
        let position = rows(self, position, "position")?;
        let dirty_base = rows(self, dirty_base, "dirty base")?;
        let last_modify = rows(self, last_modify, "last modify")?;
        let is_dirty_base = self.dirty(is_dirty_base, path)?;
        let load = self.load(load, path)?;
        let own_read = match &own_read.0 {
            Some(m) => Some(self.own_read(m, temp_id.as_ref().map(|t| t.id()), path)?),
            None => None,
        };
        let globals =
            (!globals.is_unset()).then(|| self.handle(Holder::Globals(globals.share()), 1, format!("{path} globals")));
        Ok(RecordNode {
            table: *table,
            temporary: *temporary,
            temp,
            company: company.clone(),
            change_company_called: *change_company_called,
            fields: self.field_values(fields, path)?,
            filters: plain_filters,
            decimal_cut_filters: plain_cut_filters,
            filter_group: *filter_group,
            marks: frozen_marks,
            marked_only: *marked_only,
            read_isolation: *read_isolation,
            security_filtering: *security_filtering,
            declared_security_filtering: *declared_security_filtering,
            declared_security_filtering_unpinned: *declared_security_filtering_unpinned,
            security_filtering_unpinned: *security_filtering_unpinned,
            security_filtering_array: *security_filtering_array,
            key: *key,
            adhoc: adhoc.clone(),
            ascending: *ascending,
            field_descending: *field_descending,
            position,
            auto_calc: auto_calc.clone(),
            load,
            touched: *touched,
            assigned: *assigned,
            inserted: *inserted,
            dirty_base,
            is_dirty_base,
            last_modify,
            failed_insert: *failed_insert,
            row_origin: *row_origin,
            own_read,
            probed_source: (probed_source.0, probed_source.1),
            globals,
            unpinned_buffer: *unpinned_buffer,
            view_assumption: *view_assumption,
            auto_calc_dropped: *auto_calc_dropped,
        })
    }

    fn dirty(&mut self, d: &DirtyBase, path: &str) -> Result<DirtyNode, NotImaged> {
        Ok(match d {
            DirtyBase::Unknown(why) => DirtyNode::Unknown(why),
            DirtyBase::Fresh => DirtyNode::Fresh,
            DirtyBase::Synced => DirtyNode::Synced,
            DirtyBase::Row {
                base,
                alt,
                dirty_only,
                stale_store,
            } => DirtyNode::Row {
                base: self.field_values(base, &format!("{path} dirty base row"))?,
                alt: (alt.as_deref())
                    .map(|a| self.field_values(a, &format!("{path} dirty base alt")))
                    .transpose()?,
                dirty_only: *dirty_only,
                stale_store: *stale_store,
            },
        })
    }

    fn load(&mut self, l: &LoadState, path: &str) -> Result<LoadNode, NotImaged> {
        let LoadState {
            request,
            buffer,
            partial,
        } = l;
        let buffer = buffer.as_ref().map(|b| {
            let Buffer {
                have,
                unsure,
                row,
                cleared,
                assigned,
                missing_loaded,
                fetched,
                init,
            } = b;
            BufferNode {
                have: have.clone(),
                unsure: unsure.clone(),
                row: row.clone(),
                cleared: *cleared,
                assigned: assigned.clone(),
                missing_loaded: *missing_loaded,
                fetched: (fetched.0.as_ref())
                    .map(|r| self.handle(Holder::Row(r.clone()), 1, format!("{path} fetched row"))),
                init: *init,
            }
        });
        Ok(LoadNode {
            request: request.clone(),
            buffer,
            partial: *partial,
        })
    }

    /// `temp`: the id of the record's temporary table, `None` for the database.
    fn own_read(&mut self, m: &OwnReadAt, temp: Option<u64>, path: &str) -> Result<OwnReadNode, NotImaged> {
        let OwnReadAt {
            got,
            cleared,
            system_id,
            version,
            commits,
            store_changes,
        } = m;
        let store = match temp {
            Some(id) => self.session.temps.get(&id).map_or(0, |s| s.change_counter()),
            None => self.session.store.change_counter(),
        };
        Ok(OwnReadNode {
            got: *got,
            cleared: *cleared,
            system_id: self
                .field_values(std::slice::from_ref(system_id), &format!("{path} own read"))?
                .remove(0),
            version: *version,
            commits: *commits,
            store_changes: *store_changes,
            current: *store_changes == store,
        })
    }
}

impl Maker<'_> {
    /// The record buffer of `n`, every handle it names made already (temporary tables with their stores, rows, Blobs,
    /// table globals sets).
    pub(crate) fn record(&self, n: &RecordNode) -> RecordState {
        let RecordNode {
            table,
            temporary,
            temp,
            company,
            change_company_called,
            fields,
            filters,
            decimal_cut_filters,
            filter_group,
            marks,
            marked_only,
            read_isolation,
            security_filtering,
            declared_security_filtering,
            declared_security_filtering_unpinned,
            security_filtering_unpinned,
            security_filtering_array,
            key,
            adhoc,
            ascending,
            field_descending,
            position,
            auto_calc,
            load,
            touched,
            assigned,
            inserted,
            dirty_base,
            is_dirty_base,
            last_modify,
            failed_insert,
            row_origin,
            own_read,
            probed_source,
            globals,
            unpinned_buffer,
            view_assumption,
            auto_calc_dropped,
        } = n;
        let temp_id = temp.map(|t| self.temp_id(t));
        let rows = |v: &Option<Vec<Frozen>>| v.as_ref().map(|v| self.values(v));
        RecordState {
            table: *table,
            temporary: *temporary,
            company: company.clone(),
            change_company_called: *change_company_called,
            crm: false,
            // A buffer that shared a table's initial values is copied on its first write: an owned one reads the same.
            fields: self.values(fields).into(),
            filters: filters.iter().map(PlainFilter::filter).collect(),
            decimal_cut_filters: decimal_cut_filters.iter().map(PlainFilter::filter).collect(),
            filter_group: *filter_group,
            marks: marks.iter().map(|m| self.values(m)).collect(),
            marked_only: *marked_only,
            read_isolation: *read_isolation,
            security_filtering: *security_filtering,
            declared_security_filtering: *declared_security_filtering,
            declared_security_filtering_unpinned: *declared_security_filtering_unpinned,
            security_filtering_unpinned: *security_filtering_unpinned,
            security_filtering_array: *security_filtering_array,
            key: *key,
            adhoc: adhoc.clone(),
            ascending: *ascending,
            field_descending: *field_descending,
            position: rows(position),
            auto_calc: auto_calc.clone(),
            load: self.load(load),
            touched: *touched,
            assigned: *assigned,
            inserted: *inserted,
            dirty_base: rows(dirty_base),
            is_dirty_base: match is_dirty_base {
                DirtyNode::Unknown(why) => DirtyBase::Unknown(why),
                DirtyNode::Fresh => DirtyBase::Fresh,
                DirtyNode::Synced => DirtyBase::Synced,
                DirtyNode::Row {
                    base,
                    alt,
                    dirty_only,
                    stale_store,
                } => DirtyBase::Row {
                    base: self.values(base),
                    alt: alt.as_ref().map(|a| self.values(a)),
                    dirty_only: *dirty_only,
                    stale_store: *stale_store,
                },
            },
            last_modify: rows(last_modify),
            failed_insert: *failed_insert,
            row_origin: *row_origin,
            own_read: OwnRead(own_read.as_ref().map(|m| self.own_read(m, temp_id.as_ref()))),
            probed_source: ProbedSource(probed_source.0, probed_source.1),
            globals: globals.map_or_else(TableGlobals::default, |g| self.globals(g).share()),
            unpinned_buffer: *unpinned_buffer,
            view_assumption: *view_assumption,
            run_page_link: None,
            rolled_read: Default::default(),
            auto_calc_dropped: *auto_calc_dropped,
            temp_id,
        }
    }

    fn load(&self, l: &LoadNode) -> LoadState {
        LoadState {
            request: l.request.clone(),
            buffer: l.buffer.as_ref().map(|b| Buffer {
                have: b.have.clone(),
                unsure: b.unsure.clone(),
                row: b.row.clone(),
                cleared: b.cleared,
                assigned: b.assigned.clone(),
                missing_loaded: b.missing_loaded,
                fetched: FetchedRow(b.fetched.map(|r| self.row(r))),
                init: b.init,
            }),
            partial: l.partial,
        }
    }

    fn own_read(&self, m: &OwnReadNode, temp: Option<&al_builtins::value::TempId>) -> OwnReadAt {
        let store = match temp {
            Some(t) => self.session_temp_counter(t.id()),
            None => self.store_counter,
        };
        OwnReadAt {
            got: m.got,
            cleared: m.cleared,
            system_id: self.value(&m.system_id),
            version: m.version,
            commits: m.commits,
            store_changes: if m.current { store } else { m.store_changes },
        }
    }
}
