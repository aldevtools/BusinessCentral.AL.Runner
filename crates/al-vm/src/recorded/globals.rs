//! The SingleInstance codeunits a recorded run left, and their globals, as plain data (ADR-073 decision 3).
//!
//! A global is imaged when it holds a plain value (what `value_codec` writes) or a record variable that is not temporary
//! (its state by an exhaustive destructuring of `RecordState`, [`record_image`]). An empty List or Dictionary that
//! nothing else holds is its initial value and is left out. Anything else (a temporary record, an interface, a codeunit
//! or page variable, a filled or shared collection) means the run is not recorded.
//!
//! Base Application's change log codeunit is SingleInstance and keeps the setup records it read in record globals: a
//! replay that could restore only plain values would never apply to the Global Triggers setup or the login.

use super::NotRecorded;
use super::image::{RowBytes, row_bytes, row_of};
use al_builtins::value::{Buffer, DirtyBase, FetchedRow, FieldSet, LoadState, RecordState, RowOrigin, Value};
use std::cell::RefCell;
use std::rc::Rc;

/// A record variable's state, as plain data.
#[derive(Clone, PartialEq, Debug)]
pub(super) struct RecordImage {
    table: u32,
    company: Option<String>,
    fields: RowBytes,
    filter_group: i32,
    marked_only: bool,
    read_isolation: i32,
    security_filtering: i32,
    declared_security_filtering: i32,
    declared_security_filtering_unpinned: bool,
    security_filtering_unpinned: bool,
    security_filtering_array: bool,
    key: usize,
    ascending: bool,
    field_descending: u32,
    position: Option<RowBytes>,
    auto_calc: Vec<usize>,
    load_request: Option<Vec<usize>>,
    load_buffer: Option<BufferImage>,
    load_partial: bool,
    touched: bool,
    assigned: bool,
    inserted: bool,
    dirty_base: Option<RowBytes>,
    is_dirty_base: DirtyImage,
    last_modify: Option<RowBytes>,
    failed_insert: Option<i64>,
    row_origin: Option<RowOrigin>,
    unpinned_buffer: Option<&'static str>,
}

#[derive(Clone, PartialEq, Debug)]
struct BufferImage {
    have: FieldSet,
    unsure: FieldSet,
    row: Option<String>,
    cleared: bool,
    assigned: Vec<usize>,
    missing_loaded: bool,
    fetched: Option<RowBytes>,
    init: bool,
}

#[derive(Clone, PartialEq, Debug)]
enum DirtyImage {
    Unknown(&'static str),
    Fresh,
    Synced,
    Row {
        base: RowBytes,
        alt: Option<RowBytes>,
        dirty_only: bool,
        stale_store: bool,
    },
}

fn opt_row(v: &Option<Vec<Value>>, what: &'static str) -> Result<Option<RowBytes>, NotRecorded> {
    v.as_ref()
        .map(|r| row_bytes(r).ok_or(NotRecorded::Value(what)))
        .transpose()
}

/// The image of a record variable, or why it has none: a temporary or CRM record, filters, marks, an ad-hoc key,
/// table globals that hold an instance, or a page link are not imaged.
pub(super) fn record_image(r: &RecordState) -> Result<RecordImage, NotRecorded> {
    let RecordState {
        table,
        temporary,
        temp_id,
        company,
        crm,
        fields,
        filters,
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
        globals,
        unpinned_buffer,
        run_page_link,
    } = r;
    let plain = !temporary
        && temp_id.is_none()
        && !crm
        && filters.is_empty()
        && marks.is_empty()
        && adhoc.is_none()
        && !globals.holds_instances()
        && run_page_link.is_none();
    if !plain {
        return Err(NotRecorded::Value(
            "a record global that is temporary, filtered, marked or linked",
        ));
    }
    let LoadState {
        request,
        buffer,
        partial,
    } = load;
    let load_buffer = match buffer {
        None => None,
        Some(Buffer {
            have,
            unsure,
            row,
            cleared,
            assigned,
            missing_loaded,
            fetched: FetchedRow(fetched),
            init,
        }) => Some(BufferImage {
            have: have.clone(),
            unsure: unsure.clone(),
            row: row.clone(),
            cleared: *cleared,
            assigned: assigned.clone(),
            missing_loaded: *missing_loaded,
            fetched: fetched
                .as_ref()
                .map(|f| row_bytes(f).ok_or(NotRecorded::Value("a record global's fetched row")))
                .transpose()?,
            init: *init,
        }),
    };
    let is_dirty_base = match is_dirty_base {
        DirtyBase::Unknown(why) => DirtyImage::Unknown(why),
        DirtyBase::Fresh => DirtyImage::Fresh,
        DirtyBase::Synced => DirtyImage::Synced,
        DirtyBase::Row {
            base,
            alt,
            dirty_only,
            stale_store,
        } => DirtyImage::Row {
            base: row_bytes(base).ok_or(NotRecorded::Value("a record global's IsDirty base"))?,
            alt: opt_row(alt, "a record global's IsDirty base")?,
            dirty_only: *dirty_only,
            stale_store: *stale_store,
        },
    };
    Ok(RecordImage {
        table: *table,
        company: company.clone(),
        fields: row_bytes(fields).ok_or(NotRecorded::Value("a record global's fields"))?,
        filter_group: *filter_group,
        marked_only: *marked_only,
        read_isolation: *read_isolation,
        security_filtering: *security_filtering,
        declared_security_filtering: *declared_security_filtering,
        declared_security_filtering_unpinned: *declared_security_filtering_unpinned,
        security_filtering_unpinned: *security_filtering_unpinned,
        security_filtering_array: *security_filtering_array,
        key: *key,
        ascending: *ascending,
        field_descending: *field_descending,
        position: opt_row(position, "a record global's position")?,
        auto_calc: auto_calc.clone(),
        load_request: request.clone(),
        load_buffer,
        load_partial: *partial,
        touched: *touched,
        assigned: *assigned,
        inserted: *inserted,
        dirty_base: opt_row(dirty_base, "a record global's dirty base")?,
        is_dirty_base,
        last_modify: opt_row(last_modify, "a record global's last Modify")?,
        failed_insert: *failed_insert,
        row_origin: *row_origin,
        unpinned_buffer: *unpinned_buffer,
    })
}

/// Puts `i` into `r`, a fresh record variable of the same declaration (its table globals and the parts the image
/// requires empty stay as the declaration made them).
pub(super) fn restore_record(r: &mut RecordState, i: &RecordImage) {
    r.table = i.table;
    r.company = i.company.clone();
    r.fields = row_of(&i.fields);
    r.filter_group = i.filter_group;
    r.marked_only = i.marked_only;
    r.read_isolation = i.read_isolation;
    r.security_filtering = i.security_filtering;
    r.declared_security_filtering = i.declared_security_filtering;
    r.declared_security_filtering_unpinned = i.declared_security_filtering_unpinned;
    r.security_filtering_unpinned = i.security_filtering_unpinned;
    r.security_filtering_array = i.security_filtering_array;
    r.key = i.key;
    r.ascending = i.ascending;
    r.field_descending = i.field_descending;
    r.position = i.position.as_deref().map(row_of);
    r.auto_calc = i.auto_calc.clone();
    r.load = LoadState {
        request: i.load_request.clone(),
        buffer: i.load_buffer.as_ref().map(|b| Buffer {
            have: b.have.clone(),
            unsure: b.unsure.clone(),
            row: b.row.clone(),
            cleared: b.cleared,
            assigned: b.assigned.clone(),
            missing_loaded: b.missing_loaded,
            fetched: FetchedRow(b.fetched.as_deref().map(|f| Rc::new(row_of(f)))),
            init: b.init,
        }),
        partial: i.load_partial,
    };
    r.touched = i.touched;
    r.assigned = i.assigned;
    r.inserted = i.inserted;
    r.dirty_base = i.dirty_base.as_deref().map(row_of);
    r.is_dirty_base = match &i.is_dirty_base {
        DirtyImage::Unknown(why) => DirtyBase::Unknown(why),
        DirtyImage::Fresh => DirtyBase::Fresh,
        DirtyImage::Synced => DirtyBase::Synced,
        DirtyImage::Row {
            base,
            alt,
            dirty_only,
            stale_store,
        } => DirtyBase::Row {
            base: row_of(base),
            alt: alt.as_deref().map(row_of),
            dirty_only: *dirty_only,
            stale_store: *stale_store,
        },
    };
    r.last_modify = i.last_modify.as_deref().map(row_of);
    r.failed_insert = i.failed_insert;
    r.row_origin = i.row_origin;
    r.unpinned_buffer = i.unpinned_buffer;
}

#[derive(Clone, PartialEq, Debug)]
enum GlobalImage {
    Plain(Vec<u8>),
    Record(Box<RecordImage>),
}

/// One SingleInstance codeunit's instance.
#[derive(Clone, PartialEq, Debug)]
pub(super) struct SingleImage {
    object: usize,
    /// The globals that do not hold their initial value, by slot.
    globals: Vec<(usize, GlobalImage)>,
    initialized: bool,
    rerun: bool,
    born: bool,
    single: bool,
}

/// The image of one global, `None` when it holds its initial empty collection.
fn global_image(v: &Value) -> Result<Option<GlobalImage>, NotRecorded> {
    match v {
        Value::Record(r) if Rc::strong_count(r) == 1 => {
            Ok(Some(GlobalImage::Record(Box::new(record_image(&r.borrow())?))))
        }
        Value::List(l) if Rc::strong_count(l) == 1 && l.borrow().is_empty() => Ok(None),
        Value::Dictionary(d) if Rc::strong_count(d) == 1 && d.borrow().is_empty() => Ok(None),
        v => {
            let mut out = Vec::new();
            if al_builtins::value_codec::encode(v, &mut out) {
                Ok(Some(GlobalImage::Plain(out)))
            } else {
                Err(NotRecorded::Value(
                    "a SingleInstance global that is not a plain value or a record",
                ))
            }
        }
    }
}

/// The SingleInstance codeunits of the session, sorted by object, or why one has no image.
pub(super) fn singles_image(
    singletons: &std::collections::HashMap<usize, Rc<RefCell<al_builtins::value::Instance>>>,
) -> Result<Vec<SingleImage>, NotRecorded> {
    let mut out = Vec::with_capacity(singletons.len());
    for (&object, inst) in singletons {
        let al_builtins::value::Instance {
            object: _, // the key of the map
            globals,
            initialized,
            rerun,
            base,
            exts,
            xp_text,
            born,
            single,
        } = &*inst.borrow();
        if base.is_some() || !exts.is_empty() || !xp_text.is_empty() {
            return Err(NotRecorded::Value("a SingleInstance codeunit with linked instances"));
        }
        let mut images = Vec::new();
        for (slot, v) in globals.iter().enumerate() {
            if let Some(i) = global_image(v)? {
                images.push((slot, i));
            }
        }
        out.push(SingleImage {
            object,
            globals: images,
            initialized: *initialized,
            rerun: *rerun,
            born: born.0,
            single: *single,
        });
    }
    out.sort_by_key(|s| s.object);
    Ok(out)
}

/// Makes the session's SingleInstance codeunits those of `images`: each a fresh instance with its imaged globals put
/// in. The session had none (the key).
pub(super) fn restore_singles(s: &mut crate::Session, images: &[SingleImage]) {
    for i in images {
        let inst = s.instance(i.object);
        {
            let mut b = inst.borrow_mut();
            b.initialized = i.initialized;
            b.rerun = i.rerun;
            b.born = al_builtins::value::Born(i.born);
            b.single = i.single;
            for (slot, g) in &i.globals {
                match g {
                    GlobalImage::Plain(bytes) => {
                        let mut r = al_builtins::value_codec::Reader::new(bytes);
                        b.globals[*slot] = al_builtins::value_codec::decode(&mut r).expect("bytes `encode` wrote");
                    }
                    GlobalImage::Record(img) => match &b.globals[*slot] {
                        Value::Record(r) => restore_record(&mut r.borrow_mut(), img),
                        _ => unreachable!("the slot of a record global starts with a record"),
                    },
                }
            }
        }
        s.singletons.insert(i.object, inst);
    }
}
