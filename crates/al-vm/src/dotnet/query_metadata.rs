//! `Microsoft.Dynamics.Nav.Client.BusinessChart.QueryMetadataReader` and its `QueryFields` (ADR-014 section 1): the reader
//! System Application's Web Service Management builds over a query's Application Object Metadata (Base Application's
//! ODataUtility.GetTenantWebServiceMetadata) to name the Tenant Web Service Columns of a published query.
//!
//! `FromStream` takes only an InStream over the Metadata Blob of a query's Application Object Metadata row (al-vm
//! `platform/objects.rs`; the Blob holds the query's id under the mark `stream::QUERY_METADATA`). `Fields()` lists one
//! entry per column in query order: `FieldNo` the source field (0 for a Method = Count column), `TableNo` the data item's
//! table, `FieldName` the column name (tests/al-language-onprem-dotnet/lt-re-dotnet/ReQueryMetadataShapes.al:99-145; one
//! data item: lt-qx-dotnet/QxDotNetDemoShapes.al:150-182; Sum, Average, Min, Max, ReverseSign and FlowField columns:
//! ReQueryMetadataShapes.al:171-182).
//!
//! Refused: any other stream, a query with a filter element, a column name that is not a plain identifier (none is
//! pinned), `Item` out of range, every other member.

use super::registry::{MemberKind, Pin, RegistryMember, RegistryType};
use crate::Session;
use al_builtins::dotnet_sigs::{QUERY_FIELDS, QUERY_FIELDS_LIST, QUERY_METADATA_READER};
use al_builtins::library_kind::LibraryKind;
use al_builtins::native::Signal;
use al_builtins::value::{DotNetKind, DotNetObject, LibraryValue, Value};
use std::cell::RefCell;
use std::rc::Rc;

const PIN: &str = "tests/al-language-onprem-dotnet/lt-re-dotnet/ReQueryMetadataShapes.al:99-145, :171-182; \
    lt-qx-dotnet/QxDotNetDemoShapes.al:150-182";

fn unsupported<T>(why: impl Into<String>) -> Result<T, Signal> {
    Err(Signal::Unsupported(why.into()))
}

fn object(type_name: &str, state: Vec<Value>) -> Value {
    Value::DotNet(Rc::new(DotNetObject::new(DotNetKind::Internal {
        type_name: type_name.into(),
        state: RefCell::new(state),
    })))
}

fn this_state<'a>(this: Option<&'a Rc<DotNetObject>>, type_name: &str) -> Result<&'a RefCell<Vec<Value>>, Signal> {
    match this.map(|o| &o.kind) {
        Some(DotNetKind::Internal { type_name: t, state }) if t == type_name => Ok(state),
        _ => unsupported(format!(
            "DotNet {type_name} member on something that is not a {type_name} the runtime made"
        )),
    }
}

fn no_args(args: &[Value], what: &str) -> Result<(), Signal> {
    if args.is_empty() {
        Ok(())
    } else {
        unsupported(format!("DotNet {what} takes no arguments here"))
    }
}

/// The query id the stream's Blob holds, if the stream reads the Metadata of a query's Application Object Metadata row.
fn query_id(stream: &Value) -> Option<i32> {
    let Value::Library(LibraryKind::InStream, l) = stream else {
        return None;
    };
    let l = l.borrow();
    let LibraryValue::InStream(s) = &*l else { return None };
    let buf = s.live_blob()?.borrow();
    if buf.render_pending.as_deref() != Some(al_builtins::stream::QUERY_METADATA) {
        return None;
    }
    Some(i32::from_le_bytes(buf.bytes.get(..4)?.try_into().ok()?))
}

fn plain_identifier(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn from_stream(s: &mut Session, _: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    let Some(id) = args.first().filter(|_| args.len() == 1).and_then(query_id) else {
        return unsupported(format!(
            "DotNet {QUERY_METADATA_READER}.FromStream: only the Metadata of a query's Application Object Metadata row is served"
        ));
    };
    let engine = s.engine.clone();
    let program = &engine.program;
    let Some(def) = program
        .objects
        .iter()
        .find(|o| o.kind == al_ir::ObjKind::Query && o.id == Some(id as u32))
        .and_then(|o| o.query.as_ref())
    else {
        return unsupported(format!(
            "DotNet {QUERY_METADATA_READER}.FromStream: query {id} is not compiled"
        ));
    };
    let mut entries = Vec::new();
    for c in &def.columns {
        if c.filter_element {
            return unsupported(format!(
                "DotNet {QUERY_METADATA_READER}.Fields of query {id}: a filter element (not pinned)"
            ));
        }
        if !plain_identifier(&c.name) {
            return unsupported(format!(
                "DotNet {QUERY_METADATA_READER}.Fields of query {id}: the FieldName of column '{}' (only plain names are pinned)",
                c.name
            ));
        }
        let item = &def.items[c.item];
        let table = &program.objects[item.table];
        // Sum, Average, Min and Max columns carry their source field like a plain column, Count has 0; ReverseSign and
        // FlowField columns are plain entries (lt-re-dotnet/ReQueryMetadataShapes.al:171-182, one and nested data items).
        let field_no = match (c.method, c.field) {
            (Some(al_ir::QueryMethod::Count), _) => 0,
            (_, Some(f)) => table.table.as_ref().expect("a data item's table").fields[f].id as i32,
            (_, None) => {
                return unsupported(format!(
                    "DotNet {QUERY_METADATA_READER}.Fields of query {id}: column {} has no source field",
                    c.name
                ));
            }
        };
        entries.push(object(
            QUERY_FIELDS,
            vec![
                Value::Integer(field_no),
                Value::Integer(table.id.unwrap_or(0) as i32),
                Value::Text(c.name.clone().into()),
            ],
        ));
    }
    Ok(object(QUERY_METADATA_READER, vec![object(QUERY_FIELDS_LIST, entries)]))
}

fn fields(_: &mut Session, this: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    no_args(args, &format!("{QUERY_METADATA_READER}.Fields"))?;
    let state = this_state(this, QUERY_METADATA_READER)?;
    Ok(state.borrow()[0].clone())
}

fn count(_: &mut Session, this: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    no_args(args, "Count")?;
    Ok(Value::Integer(
        this_state(this, QUERY_FIELDS_LIST)?.borrow().len() as i32
    ))
}

fn item(_: &mut Session, this: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    let state = this_state(this, QUERY_FIELDS_LIST)?;
    let at = match args {
        [Value::Integer(i)] => *i as i64,
        [Value::BigInteger(i)] => *i,
        _ => return unsupported("DotNet query fields Item takes one index here"),
    };
    match usize::try_from(at).ok().and_then(|i| state.borrow().get(i).cloned()) {
        Some(v) => Ok(v),
        None => unsupported("DotNet query fields Item out of range (the exception is not pinned)"),
    }
}

fn field(this: Option<&Rc<DotNetObject>>, args: &[Value], at: usize, name: &str) -> Result<Value, Signal> {
    no_args(args, &format!("{QUERY_FIELDS}.{name}"))?;
    Ok(this_state(this, QUERY_FIELDS)?.borrow()[at].clone())
}

fn field_no(_: &mut Session, this: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    field(this, args, 0, "FieldNo")
}

fn table_no(_: &mut Session, this: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    field(this, args, 1, "TableNo")
}

fn field_name(_: &mut Session, this: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    field(this, args, 2, "FieldName")
}

fn member(name: &str, f: super::registry::MemberFn) -> RegistryMember {
    RegistryMember {
        name: name.into(),
        kind: MemberKind::Call,
        pin: Pin::Corpus(PIN.into()),
        call: f,
        call_ref: None,
    }
}

pub fn types() -> Vec<RegistryType> {
    vec![
        RegistryType {
            clr_name: QUERY_METADATA_READER.into(),
            members: vec![member("FromStream", from_stream), member("Fields", fields)],
        },
        RegistryType {
            clr_name: QUERY_FIELDS_LIST.into(),
            members: vec![member("Count", count), member("Item", item)],
        },
        RegistryType {
            clr_name: QUERY_FIELDS.into(),
            members: vec![
                member("FieldNo", field_no),
                member("TableNo", table_no),
                member("FieldName", field_name),
            ],
        },
    ]
}
