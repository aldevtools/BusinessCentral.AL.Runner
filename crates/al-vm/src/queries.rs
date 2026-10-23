//! Query objects: `Open`, `Read`, `Close`, filters on columns, joins, grouping and aggregation.
//!
//! A query is executed as the SQL statement BC compiles it to: a nested-loop join over the
//! dataitems (every dataitem scanned through the store, ADR-003), the filters applied as WHERE
//! conditions, grouping by the non-aggregated columns when an aggregated column is present, the
//! filters on aggregated columns applied to the groups (HAVING), then the sort and the row cap.
//! Every behaviour is pinned by a corpus test cited at the code or is refused with
//! `Signal::Unsupported`; the open questions are in docs/probes-needed.md.

use crate::Session;
use al_builtins::collation;
use al_builtins::describe::Describe;
use al_builtins::filter::{FieldFilter, FilterExpr};
use al_builtins::native::{AlError, Signal, text_of};
use al_builtins::ops;
use al_builtins::value::{QueryRow, QueryRun, QueryState, RtType, Value};
use al_data::TableDef;
use al_ir as ir;
use rust_decimal::Decimal;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::rc::Rc;
use std::sync::Arc;

type Qs = Rc<RefCell<QueryState>>;
/// One row of each dataitem's table, in dataitem order.
type JoinedRow = Vec<Vec<Value>>;
/// (table object, version) of each table a query reads.
type Stamps = Vec<(u32, u64)>;

fn unsupported<T>(what: impl Into<String>) -> Result<T, Signal> {
    Err(Signal::Unsupported(what.into()))
}

thread_local! {
    /// Set when a sort compared texts that have no pinned collation order.
    static COLLATION_FAILED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// `ColumnNo`: the FNV-1a 32 hash of the column's name in UTF-16LE, read as a signed 32-bit
/// number, without its sign (misc/lt-probes-needed-1/PnAQueryFiltersColumnNo.al:143-150 for
/// four columns and a filter element, :168-175 for seven aggregated columns). The one hash whose
/// absolute value does not fit (`i32::MIN`) is not pinned and is refused.
fn column_no(name: &str) -> Result<i32, Signal> {
    let mut h: u32 = 0x811c_9dc5;
    for b in name.encode_utf16().flat_map(u16::to_le_bytes) {
        h = (h ^ u32::from(b)).wrapping_mul(0x0100_0193);
    }
    match (h as i32).checked_abs() {
        Some(n) => Ok(n),
        None => unsupported(format!(
            "ColumnNo of column {name}: its hash is -2147483648 (not pinned)"
        )),
    }
}

fn arg_col(args: &[Value], i: usize) -> Result<usize, Signal> {
    match args.get(i) {
        Some(Value::Integer(p)) => Ok(*p as usize),
        other => unsupported(format!("column argument {}", other.describe())),
    }
}

/// Compares two sort keys: the first `dirs.len()` entries in the given directions, the rest
/// ascending.
fn key_cmp(a: &[Value], b: &[Value], dirs: &[bool]) -> Ordering {
    for (i, (x, y)) in a.iter().zip(b).enumerate() {
        // Text sorts and groups by the database collation (case sensitive), not by code point
        // (query/TestQueryRuntimeContracts.al:Query_OrderByText_UsesTheDatabaseCollation_NotOrdinalOrder).
        let o = match (text_of(x), text_of(y)) {
            (Some(s), Some(t)) if matches!(x, Value::Text(_) | Value::Code(_)) => match collation::compare(&s, &t) {
                Ok(o) => o,
                // A character without a weight: the caller refuses the result.
                Err(_) => {
                    COLLATION_FAILED.with(|f| f.set(true));
                    s.cmp(&t)
                }
            },
            _ => ops::compare(x, y).unwrap_or(Ordering::Equal),
        };
        let o = if dirs.get(i).copied().unwrap_or(true) {
            o
        } else {
            o.reverse()
        };
        if o != Ordering::Equal {
            return o;
        }
    }
    Ordering::Equal
}

fn negate(v: Value) -> Result<Value, Signal> {
    Ok(match v {
        Value::Decimal(d) => Value::Decimal(-d),
        Value::Integer(i) => Value::Integer(-i),
        Value::BigInteger(i) => Value::BigInteger(-i),
        other => return unsupported(format!("ReverseSign on {}", other.describe())),
    })
}

/// The filters of one execution, split by where they apply.
struct Plan {
    /// WHERE filters on stored fields, per dataitem.
    scan: Vec<Vec<FieldFilter>>,
    /// Flow filters (`filter()` elements over FlowFilter fields), per dataitem.
    flow: Vec<Vec<FieldFilter>>,
    /// Filters on non-aggregated FlowField columns, checked on the projected value.
    post: Vec<(usize, FilterExpr)>,
    /// Filters on aggregated columns, checked on the group.
    having: Vec<(usize, FilterExpr)>,
    /// Filters on stored fields of a left-outer-joined dataitem: (dataitem, field, filter). They
    /// apply after the join, to the values of the joined row, which for an unmatched parent are the
    /// defaults (query/TestQueryOuterJoinDescendingSaveAsContracts.al:
    /// a filter drops the unmatched parents unless it admits 0).
    outer: Vec<(usize, usize, FilterExpr)>,
}

/// The row of a dataitem with no match under a left outer join.
const NO_MATCH: Vec<Value> = Vec::new();

impl Session {
    /// An instance of query `object` whose hidden `__query` slot holds `q`, so a procedure's
    /// unqualified `SetRange` and the like act on the query the caller holds
    /// (query/TestQueryDataItemTableFilterDatabaseConst.al:96-136).
    pub(crate) fn query_instance(
        &mut self,
        object: usize,
        q: std::rc::Rc<std::cell::RefCell<al_builtins::value::QueryState>>,
    ) -> std::rc::Rc<std::cell::RefCell<al_builtins::value::Instance>> {
        if let Some(inst) = q.borrow().instance.clone() {
            return inst;
        }
        // Made once per query variable: its variables live as long as it does (`Clear` makes a new
        // state without an instance). The hidden slot points back at the state.
        let inst = self.instance(object);
        if let Some(slot) = self.engine.program.objects[object].rec_slot {
            inst.borrow_mut().globals[slot] = Value::Query(q.clone());
        }
        q.borrow_mut().instance = Some(inst.clone());
        inst
    }

    pub(crate) fn query_op(
        &mut self,
        method: &str,
        recv: Option<&mut Value>,
        args: &mut [Value],
    ) -> Result<Value, Signal> {
        if method == "SaveAsCsvStatic" {
            return self.query_save_csv_static(args);
        }
        let qs: Qs = match recv {
            Some(Value::Query(q)) => q.clone(),
            other => return unsupported(format!("Query.{method} on {}", other.describe())),
        };
        let engine = self.engine.clone();
        let object = qs.borrow().object as usize;
        let def = engine.program.objects[object].query.as_ref().expect("a query object");
        if qs.borrow().in_before_open {
            match method {
                // CurrQuery.Open and CurrQuery.Close inside OnBeforeOpen raise; the trigger does not run again
                // (query/lt-ch-query-leads/ChQueryLeads.al:259-290, OnBeforeOpen_CallsOpen / _CallsClose).
                "Open" | "Close" => {
                    return Err(Signal::Error(Box::new(al_builtins::native::AlError {
                        message:
                            "It is not possible to execute the current query from within the 'OnBeforeOpen' trigger."
                                .into(),
                        code: "QueryIllegalCallInTrigger".into(),
                        ..Default::default()
                    })));
                }
                // SaveAsXml there returns false after writing bytes the corpus only counts
                // (query/lt-ch-query-leads/ChQueryLeads.al:271-278): their content is not pinned.
                "SaveAsXml" | "SaveAsJson" | "SaveAsCsv" => {
                    return unsupported(format!(
                        "CurrQuery.{method} inside OnBeforeOpen (the bytes it writes are not pinned)"
                    ));
                }
                // Read on a query opened for the first time returns false (ChQueryLeads.al:294-301); on a query
                // that was open before, it is not pinned.
                "Read" if qs.borrow().run.is_some() => {
                    return unsupported(
                        "CurrQuery.Read inside OnBeforeOpen of a query that was already open (not pinned)",
                    );
                }
                _ => {}
            }
        }
        match method {
            "Open" => {
                // Opening an open query is not an error (query/TestQueryRuntimeContracts.al:
                // Query_StateMisuse_DoesNotThrow); once rows were read what it does is not pinned.
                if qs.borrow().run.as_ref().is_some_and(|r| r.started) {
                    // After a filter changed mid-read it runs again with the new filters
                    // (misc/lt-probes-needed-1/PnAQueryOpenRead.al:125-170, SetRangeAndOpenAfterRead: `next-entry` 4).
                    // After the last row (with or without a filter set since) it runs again too: not pinned
                    // (assumption query-closed-after-last-row).
                    let filters_changed = qs.borrow().run.as_ref().is_some_and(|r| r.filters_changed);
                    if Self::query_read_to_the_end(&qs) {
                        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::QueryClosedAfterLastRow);
                    } else if !filters_changed {
                        return unsupported("Open on a query that is already open and was read from (not pinned)");
                    }
                    qs.borrow_mut().run = None;
                }
                // OnBeforeOpen runs inside Open, after the filters set so far, and a filter it sets
                // applies to this read (query/TestQueryOnBeforeOpenVariablesContracts.al:104-140).
                if let Some(ri) = engine.program.objects[object]
                    .routines
                    .iter()
                    .position(|r| r.is_trigger && r.name.eq_ignore_ascii_case("OnBeforeOpen"))
                {
                    let inst = self.query_instance(object, qs.clone());
                    qs.borrow_mut().in_before_open = true;
                    let r = self.call(object, ri, inst, Vec::new());
                    qs.borrow_mut().in_before_open = false;
                    r?;
                }
                // Open does not run the query; the first Read does.
                qs.borrow_mut().run = Some(QueryRun {
                    started: false,
                    filters_changed: false,
                    rows: Vec::new(),
                    next: 0,
                    current: None,
                    last_key: None,
                    returned: 0,
                    stamps: Vec::new(),
                });
                Ok(Value::Boolean(true))
            }
            "Read" => self.query_read(def, &qs),
            "Close" => {
                qs.borrow_mut().run = None;
                Ok(Value::Null)
            }
            "SetRange" | "SetFilter" => {
                let col = arg_col(args, 0)?;
                // A filter set after Open and before the first Read is in force when the query runs.
                // After a Read that did not reach the end the filter is set, and the next Read raises until the query is
                // opened again (query/lt-query-setrange-after-read/QuerySetRangeAfterRead.al:52-77, SetRange_AfterFirstRead;
                // errors/lt-ms-refusals-131/P131QuerySetRangeAfterRead.al:118-145, SetFilter).
                // The same after the last Read returned false: the filter is set and the next Read raises
                // (errors/lt-ms-refusals-131/P131QuerySetRangeAfterRead.al:154-170, P131_Query_SetRangeAfterLastRead_AndAfterClose).
                if let Some(run) = qs.borrow_mut().run.as_mut().filter(|r| r.started) {
                    run.filters_changed = true;
                }
                let ty = def.columns[col].ty.clone();
                let expr = if method == "SetRange" {
                    match args.get(1) {
                        // `SetRange(Column)` removes the filter.
                        None => None,
                        Some(from) => {
                            let from = ops::convert(from.clone(), &ty)?;
                            let to = match args.get(2) {
                                Some(v) => ops::convert(v.clone(), &ty)?,
                                None => from.clone(),
                            };
                            Some(FilterExpr::Range(Some(from), Some(to)))
                        }
                    }
                } else if text_of(args.get(1).unwrap_or(&Value::Null)).is_some_and(|t| t.is_empty()) && args.len() <= 2
                {
                    // An empty SetFilter clears the filter
                    // (query/TestQueryRuntimeContracts.al:Query_GetFilters_TextForSingleAndCombinedFilters).
                    None
                } else {
                    let text = text_of(args.get(1).unwrap_or(&Value::Null)).unwrap_or_default();
                    let engine = self.engine.clone();
                    let enums = |id: u32| crate::views::enum_filter_members(&engine.program.objects[id as usize]);
                    Some(
                        al_builtins::filter::parse_filter_with(&text, &args[2..], &ty, &enums)
                            .map_err(|e| Signal::Unsupported(format!("SetFilter: {e}")))?,
                    )
                };
                let mut q = qs.borrow_mut();
                if let Some(run) = q.run.as_mut().filter(|r| !r.started) {
                    run.filters_changed = true;
                }
                let before = q.filters.len();
                q.filters.retain(|(c, _)| *c != col);
                if q.filters.len() != before {
                    q.filter_replaced = true;
                }
                if let Some(e) = expr {
                    q.filters.push((col, e));
                }
                Ok(Value::Null)
            }
            "GetFilter" => {
                let col = arg_col(args, 0)?;
                let q = qs.borrow();
                match q.filters.iter().find(|(c, _)| *c == col) {
                    Some((_, e)) => self.filter_text(e).map(|s| Value::Text(s.into())),
                    None if def.columns[col].static_filter.is_some() => {
                        unsupported("GetFilter of a column with a static ColumnFilter (not pinned)")
                    }
                    None => Ok(Value::Text(String::new().into())),
                }
            }
            "GetFilters" => {
                // `Caption: filter` pairs joined by `, `
                // (query/TestQueryRuntimeContracts.al:Query_GetFilters_TextForSingleAndCombinedFilters).
                let q = qs.borrow();
                let mut parts = Vec::new();
                for (c, e) in self.filters_in_text_order(def, &q.filters, q.filter_replaced)? {
                    let caption = self.column_caption(def, c)?;
                    parts.push(format!("{caption}: {}", self.filter_text(&e)?));
                }
                Ok(Value::Text(parts.join(", ").into()))
            }
            "ColumnName" => Ok(Value::Text(def.columns[arg_col(args, 0)?].name.clone().into())),
            "ColumnCaption" => self
                .column_caption(def, arg_col(args, 0)?)
                .map(|s| Value::Text(s.into())),
            // A hash of the column name, not its position (see `column_no`).
            "ColumnNo" => column_no(&def.columns[arg_col(args, 0)?].name).map(Value::Integer),
            "TopNumberOfRows" => match args.first() {
                None => Ok(Value::Integer(qs.borrow().top.unwrap_or(def.top))),
                Some(n) => {
                    let n = match n {
                        Value::Integer(n) => *n,
                        other => return unsupported(format!("TopNumberOfRows({})", other.describe())),
                    };
                    if n < 0 {
                        // Corpus query/TestQueryTopProperty.al:178.
                        return Err(Signal::Error(Box::new(AlError {
                            message: "Top number of rows cannot be negative".into(),
                            code: String::new(),
                            call_stack: String::new(),
                            ..Default::default()
                        })));
                    }
                    if qs.borrow().run.is_some() {
                        return unsupported("TopNumberOfRows on an open query (not pinned)");
                    }
                    qs.borrow_mut().top = Some(n);
                    Ok(Value::Null)
                }
            },
            "SecurityFiltering" => {
                let members = al_builtins::sysenum::option_members("SecurityFilter").expect("known");
                match args.first() {
                    // Corpus query/TestQueryObject.al:382-390.
                    Some(Value::Option(i, _)) => {
                        if members[*i as usize].eq_ignore_ascii_case("Validated") {
                            // query/TestQueryRuntimeContracts.al:Query_SecurityFiltering_DefaultAndValidated
                            return Err(Signal::Error(Box::new(AlError {
                                message: "SECURITYFILTERING cannot be set to \"Validated\" on a Query.".into(),
                                code: "Argument".into(),
                                call_stack: String::new(),
                                ..Default::default()
                            })));
                        }
                        qs.borrow_mut().security_filtering = Some(*i);
                        Ok(Value::Null)
                    }
                    // The default is Filtered (same test).
                    None => {
                        let i = match qs.borrow().security_filtering {
                            Some(i) => i,
                            None => members
                                .iter()
                                .position(|m| m.eq_ignore_ascii_case("Filtered"))
                                .expect("a member") as i32,
                        };
                        Ok(Value::Option(i, members.into()))
                    }
                    Some(other) => unsupported(format!("SecurityFiltering({})", other.describe())),
                }
            }
            "SaveAsXml" | "SaveAsJson" => self.query_save_as(def, &qs, method, args),
            "SaveAsCsv" => {
                let filters = qs.borrow().filters.clone();
                let top = qs.borrow().top.unwrap_or(def.top);
                self.query_save_csv(def, &filters, top, args)
            }
            other => unsupported(format!("Query.{other}")),
        }
    }

    /// `SaveAsXml(OutStream)` writes the rows as a .NET DataSet, `SaveAsJson(OutStream)` as an array
    /// of objects keyed by column name. Both run the query with the filters set now, whether it is
    /// open or not (query/TestQueryOuterJoinDescendingSaveAsContracts.al:Query_SaveAsXml_And_SaveAsJson_Output,
    /// corpus: . Pinned for Integer columns; the lines of the XML are
    /// not pinned (the probe trims and joins them), the bytes are UTF-8 without a byte order mark whatever the
    /// stream's text encoding (not pinned).
    fn query_save_as(&mut self, def: &ir::Query, qs: &Qs, method: &str, args: &[Value]) -> Result<Value, Signal> {
        let Some(Value::Library(_, h)) = args.first() else {
            return unsupported(format!("{method} with {}", args.first().describe()));
        };
        let blob = match &*h.borrow() {
            al_builtins::value::LibraryValue::OutStream(blob, _) => blob.clone(),
            _ => return unsupported(format!("{method} to something that is not an OutStream")),
        };
        let cols: Vec<usize> = (0..def.columns.len())
            .filter(|&c| !def.columns[c].filter_element)
            .collect();
        let json = method == "SaveAsJson";
        for &c in &cols {
            if !matches!(def.columns[c].ty, RtType::Integer) {
                return unsupported(format!(
                    "{method} of the column {} of type {} (only Integer columns are pinned)",
                    def.columns[c].name,
                    def.columns[c].ty.describe()
                ));
            }
        }
        let filters = qs.borrow().filters.clone();
        let (rows, _) = self.query_rows(def, &filters)?;
        let top = qs.borrow().top.unwrap_or(def.top);
        let rows: Vec<&QueryRow> = rows
            .iter()
            .take(if top > 0 { top as usize } else { usize::MAX })
            .collect();
        let cell = |r: &QueryRow, c: usize| match &r.columns[c] {
            Value::Integer(i) => i.to_string(),
            _ => String::new(),
        };
        let mut out = String::new();
        if json {
            out.push('[');
            for (i, r) in rows.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push('{');
                for (k, &c) in cols.iter().enumerate() {
                    if k > 0 {
                        out.push(',');
                    }
                    out.push_str(&format!("\"{}\":{}", def.columns[c].name, cell(r, c)));
                }
                out.push('}');
            }
            out.push(']');
        } else {
            out.push_str(r#"<?xml version="1.0" standalone="yes"?><DataSet><xs:schema id="NewDataSet" xmlns="" xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:msdata="urn:schemas-microsoft-com:xml-msdata"><xs:element name="NewDataSet" msdata:IsDataSet="true" msdata:MainDataTable="Result" msdata:Locale=""><xs:complexType><xs:choice minOccurs="0" maxOccurs="unbounded"><xs:element name="Result" msdata:Locale=""><xs:complexType><xs:sequence>"#);
            for &c in &cols {
                out.push_str(&format!(
                    r#"<xs:element name="{}" type="xs:int" minOccurs="0" />"#,
                    def.columns[c].name
                ));
            }
            out.push_str(
                "</xs:sequence></xs:complexType></xs:element></xs:choice></xs:complexType></xs:element></xs:schema>",
            );
            for r in &rows {
                out.push_str("<Result>");
                for &c in &cols {
                    let n = &def.columns[c].name;
                    out.push_str(&format!("<{n}>{}</{n}>", cell(r, c)));
                }
                out.push_str("</Result>");
            }
            out.push_str("</DataSet>");
        }
        al_builtins::stream::write_bytes(&blob, out.as_bytes());
        Ok(Value::Boolean(true))
    }

    /// `Query.SaveAsCsv(Number, OutStream)`: the query of that id with no filters. An id no query has writes nothing and
    /// raises nothing (xmlport/lt-xmlport-query-datatransfer/XpTQuerySave.al:SaveAsCsv_UnknownQueryId).
    fn query_save_csv_static(&mut self, args: &[Value]) -> Result<Value, Signal> {
        let id = match args.first() {
            Some(Value::Integer(n)) => *n,
            other => return unsupported(format!("Query.SaveAsCsv with {}", other.describe())),
        };
        let engine = self.engine.clone();
        let Some(object) = engine
            .program
            .objects
            .iter()
            .position(|o| o.is_kind(al_ir::ObjKind::Query) && o.id == u32::try_from(id).ok())
        else {
            return Ok(Value::Boolean(false));
        };
        let def = engine.program.objects[object].query.as_ref().expect("a query object");
        self.query_save_csv(def, &[], def.top, &args[1..])
    }

    /// The rows of a query as CSV: the column names, then a row per line, each line ended by a line feed. A text with a comma or a
    /// quote is quoted, a Boolean reads True or False, (xmlport/lt-xmlport-query-datatransfer/XpTQuerySave.al).
    /// Other column types are not pinned.
    fn query_save_csv(
        &mut self,
        def: &ir::Query,
        filters: &[(usize, FilterExpr)],
        top: i32,
        args: &[Value],
    ) -> Result<Value, Signal> {
        if args.len() > 1 {
            return unsupported("Query.SaveAsCsv with a format or a format argument (not pinned)");
        }
        let Some(Value::Library(_, h)) = args.first() else {
            return unsupported(format!("Query.SaveAsCsv with {}", args.first().describe()));
        };
        let blob = match &*h.borrow() {
            al_builtins::value::LibraryValue::OutStream(blob, _) => blob.clone(),
            _ => return unsupported("Query.SaveAsCsv to something that is not an OutStream"),
        };
        let cols: Vec<usize> = (0..def.columns.len())
            .filter(|&c| !def.columns[c].filter_element)
            .collect();
        for &c in &cols {
            if !matches!(
                def.columns[c].ty,
                RtType::Integer
                    | RtType::BigInteger
                    | RtType::Decimal
                    | RtType::Boolean
                    | RtType::Text(_)
                    | RtType::Code(_)
            ) {
                return unsupported(format!(
                    "Query.SaveAsCsv of the column {} of type {} (not pinned)",
                    def.columns[c].name,
                    def.columns[c].ty.describe()
                ));
            }
        }
        let (rows, _) = self.query_rows(def, filters)?;
        // The file of query/W7QueryLifecycle.al:SaveAsCsv_NonIntegerColumns (a Date column and a text with an accent) is 83 bytes
        // where this writes 81; which of the two accounts for the difference is not pinned, so neither is written.
        if rows.iter().any(|r| {
            r.columns
                .iter()
                .any(|v| matches!(v, Value::Text(s) | Value::Code(s) if !s.is_ascii()))
        }) {
            return unsupported("Query.SaveAsCsv of a text beyond ASCII (not pinned)");
        }
        let mut out = String::new();
        let quote = |s: &str| {
            if s.contains(',') || s.contains('"') {
                format!("\"{}\"", s.replace('"', "\"\""))
            } else {
                s.to_string()
            }
        };
        out.push_str(
            &cols
                .iter()
                .map(|&c| quote(&def.columns[c].name))
                .collect::<Vec<_>>()
                .join(","),
        );
        out.push_str("\r\n");
        for r in rows.iter().take(if top > 0 { top as usize } else { usize::MAX }) {
            let cells: Vec<String> = cols
                .iter()
                .map(|&c| match &r.columns[c] {
                    Value::Integer(i) => i.to_string(),
                    Value::BigInteger(i) => i.to_string(),
                    Value::Decimal(d) => d.normalize().to_string(),
                    Value::Boolean(b) => (if *b { "True" } else { "False" }).to_string(),
                    Value::Text(s) | Value::Code(s) => quote(s),
                    _ => String::new(),
                })
                .collect();
            out.push_str(&cells.join(","));
            out.push_str("\r\n");
        }
        al_builtins::stream::write_bytes(&blob, out.as_bytes());
        Ok(Value::Boolean(true))
    }

    /// The value of column `column` in the row `Read` returned last.
    pub(crate) fn query_column(&mut self, q: Value, column: usize) -> Result<Value, Signal> {
        let Value::Query(qs) = q else {
            return unsupported(format!("a query column of {}", q.describe()));
        };
        let qs = qs.borrow();
        match qs.run.as_ref().and_then(|r| r.current.as_ref()) {
            Some(row) => Ok(row[column].clone()),
            None => {
                // Before Open, after Open and before the first Read, and after the last row alike
                // (query/TestQueryRuntimeContracts.al:Query_ColumnRead_WithoutARow_ThrowsQueryNoDataSet).
                let def = self.engine.program.objects[qs.object as usize]
                    .query
                    .as_ref()
                    .expect("a query object");
                Err(Signal::Error(Box::new(AlError {
                    message: format!(
                        "You have not yet retrieved a data row for this query. The value for column '{}' can only be accessed after a row is retrieved with a call to the READ function.",
                        def.columns[column].name
                    ),
                    code: "QueryNoDataSet".into(),
                    call_stack: String::new(),
                    ..Default::default()
                })))
            }
        }
    }

    /// `ColumnCaption`: the `Caption` property, else the caption of the source field.
    fn column_caption(&self, def: &ir::Query, col: usize) -> Result<String, Signal> {
        let c = &def.columns[col];
        if let Some(caption) = &c.caption {
            return Ok(caption.clone());
        }
        let Some(field) = c.field else {
            return unsupported("the caption of a Count column without a Caption property (not pinned)");
        };
        let t = self.engine.tables[def.items[c.item].table].clone().expect("table");
        match self.field_caption_written(&t, field)? {
            crate::recmeta::WrittenCaption::Text(caption) => Ok(caption),
            crate::recmeta::WrittenCaption::Class(class) => unsupported(format!(
                "the caption of query column {} whose field's CaptionClass `{class}` the platform translates (not pinned for a query column)",
                c.name
            )),
            crate::recmeta::WrittenCaption::Expr(expr) => unsupported(format!(
                "the caption of query column {} whose field's CaptionClass is the expression `{}` (not pinned)",
                c.name,
                expr.trim()
            )),
        }
    }

    /// The filters in the order `GetFilters` prints them.
    ///
    /// Filters on columns only come in column order, whatever the order they were set in
    /// (query/W7QueryFilters.al: GetFilters_*, all 13 orders over seven columns, BC 27.5 and 29.0).
    ///
    /// With a filter element among them the order depends on the order of setting. Each filter is
    /// placed when it is set: a column goes before the first column already filtered that comes
    /// after it in column order (filter elements do not count), else last; the filter element goes
    /// before the first filter already set whose field number is higher, else last. This rule
    /// matches every case of misc/lt-probes-needed-1/PnAQueryFiltersColumnNo.al:59-118 (filter
    /// element on field 2 between columns on fields 1 and 3) and
    /// query/TestQueryRuntimeContracts.al:219-268 (same shape, other query). Outside that shape it
    /// is not pinned and is refused: more than one filter element, filters on several dataitems or
    /// on aggregated columns, columns whose column order differs from their field order, a field
    /// filtered twice, or a filter set again or removed (`filter_replaced`).
    fn filters_in_text_order(
        &self,
        def: &ir::Query,
        filters: &[(usize, FilterExpr)],
        filter_replaced: bool,
    ) -> Result<Vec<(usize, FilterExpr)>, Signal> {
        if !filters.iter().any(|(c, _)| def.columns[*c].filter_element) {
            let mut out = filters.to_vec();
            out.sort_by_key(|(c, _)| *c);
            return Ok(out);
        }
        let refuse = |why: &str| unsupported(format!("GetFilters with a filter element: {why} (not pinned)"));
        if filter_replaced {
            return refuse("a filter was set again or removed");
        }
        if filters.iter().filter(|(c, _)| def.columns[*c].filter_element).count() > 1 {
            return refuse("more than one filter element");
        }
        let item = def.columns[filters[0].0].item;
        let mut field_no = Vec::with_capacity(filters.len());
        for (c, _) in filters {
            let col = &def.columns[*c];
            let Some(field) = col.field.filter(|_| col.item == item && col.method.is_none()) else {
                return refuse("a filter on another dataitem or on an aggregated column");
            };
            let table = def.items[item].table;
            let id = self.engine.program.objects[table].table.as_ref().expect("table").fields[field].id;
            if field_no.contains(&id) {
                return refuse("one field filtered twice");
            }
            field_no.push(id);
        }
        let mut columns: Vec<(usize, u32)> = filters
            .iter()
            .zip(&field_no)
            .filter(|((c, _), _)| !def.columns[*c].filter_element)
            .map(|((c, _), n)| (*c, *n))
            .collect();
        columns.sort_by_key(|(c, _)| *c);
        if columns.windows(2).any(|w| w[0].1 > w[1].1) {
            return refuse("columns whose column order differs from their field order");
        }
        // Positions in `filters` (set order), placed one by one.
        let mut placed: Vec<usize> = Vec::with_capacity(filters.len());
        for (i, (c, _)) in filters.iter().enumerate() {
            let at = if def.columns[*c].filter_element {
                placed.iter().position(|&j| field_no[j] > field_no[i])
            } else {
                placed
                    .iter()
                    .position(|&j| !def.columns[filters[j].0].filter_element && filters[j].0 > *c)
            };
            placed.insert(at.unwrap_or(placed.len()), i);
        }
        Ok(placed.into_iter().map(|i| filters[i].clone()).collect())
    }

    /// Whether the query was read to its last row (its last Read returned false).
    fn query_read_to_the_end(qs: &Qs) -> bool {
        qs.borrow()
            .run
            .as_ref()
            .is_some_and(|r| r.started && r.current.is_none() && r.next >= r.rows.len())
    }

    fn query_read(&mut self, def: &ir::Query, qs: &Qs) -> Result<Value, Signal> {
        // Read on a query that is not open returns false
        // (query/TestQueryRuntimeContracts.al:Query_StateMisuse_DoesNotThrow).
        // With the result unused the statement raises (xmlport/lt-xmlport-query-datatransfer/XpTQueryBasic.al:Read_BeforeOpen, Read_AfterClose).
        if qs.borrow().run.is_none() {
            if self.ret_used {
                return Ok(Value::Boolean(false));
            }
            return Err(Signal::Error(Box::new(AlError {
                message: "You have not yet retrieved a dataset for this query. The READ function can only be called after the dataset is retrieved with a call to the OPEN function.".into(),
                code: "QueryNoDataSet".into(),
                call_stack: String::new(),
                ..Default::default()
            })));
        }
        if qs.borrow().run.as_ref().is_some_and(|r| r.filters_changed) {
            return Err(Signal::Error(Box::new(AlError {
                message: "The READ function is called on a query after query metadata has been changed by one of the following: SETFILTER, SETRANGE, SECURITYFILTERING, or TOPNUMBEROFROWS. After metadata changes, you must reopen the query by calling the OPEN function before calling the READ function.".into(),
                code: "Query".into(),
                call_stack: String::new(),
                ..Default::default()
            })));
        }
        if !qs.borrow().run.as_ref().expect("open").started {
            let filters = qs.borrow().filters.clone();
            let (rows, stamps) = self.query_rows(def, &filters)?;
            let mut q = qs.borrow_mut();
            let run = q.run.as_mut().expect("open");
            run.rows = rows;
            run.stamps = stamps;
            run.started = true;
        }
        // A write to a table the query reads, made between two Reads, invalidates the result set;
        // Read then reads again and resumes after the last row it returned, in the query's order
        // (corpus query/TestQueryReadAfterWrite.al:102-179).
        let now = self.query_stamps(def);
        if qs.borrow().run.as_ref().expect("open").stamps != now {
            if def.columns.iter().any(|c| c.method.is_some()) {
                return unsupported("a write to a table an open aggregated query reads (the re-read is not pinned)");
            }
            let filters = qs.borrow().filters.clone();
            let (mut rows, stamps) = self.query_rows(def, &filters)?;
            let dirs: Vec<bool> = def.order_by.iter().map(|(_, asc)| *asc).collect();
            let mut q = qs.borrow_mut();
            let run = q.run.as_mut().expect("open");
            if let Some(last) = &run.last_key {
                rows.retain(|r| key_cmp(&r.key, last, &dirs) == Ordering::Greater);
            }
            run.rows = rows;
            run.next = 0;
            run.stamps = stamps;
        }
        let mut q = qs.borrow_mut();
        let top = q.top.unwrap_or(def.top);
        let run = q.run.as_mut().expect("open");
        // The cap counts the rows returned since Open, so a re-read returns only the rows still owed
        // (corpus query/TestQueryOrderAfterWrite.al:127-149, TestQueryTopProperty.al:94-108).
        if (top > 0 && run.returned >= top as usize) || run.next >= run.rows.len() {
            run.current = None;
            return Ok(Value::Boolean(false));
        }
        let row = run.rows[run.next].clone();
        run.next += 1;
        run.returned += 1;
        run.current = Some(row.columns);
        run.last_key = Some(row.key);
        Ok(Value::Boolean(true))
    }

    /// The version of every table the query reads.
    fn query_stamps(&self, def: &ir::Query) -> Stamps {
        let mut out = Stamps::new();
        for item in &def.items {
            let t = self.engine.tables[item.table].as_ref().expect("table");
            if !out.iter().any(|(o, _)| *o == t.object) {
                out.push((t.object, self.store.table_stamp(&self.cx, t)));
            }
        }
        out
    }

    /// Runs the query: its rows in order, and the table versions they were read at.
    fn query_rows(
        &mut self,
        def: &ir::Query,
        runtime: &[(usize, FilterExpr)],
    ) -> Result<(Vec<QueryRow>, Stamps), Signal> {
        let stamps = self.query_stamps(def);
        COLLATION_FAILED.with(|f| f.set(false));
        let plan = self.query_plan(def, runtime)?;
        // The joined rows: one row of each dataitem's table.
        let mut joined: Vec<JoinedRow> = Vec::new();
        self.expand(def, &plan, 0, &mut Vec::new(), &mut joined)?;
        self.expand_unlinked(def, &plan, &mut joined)?;
        if !plan.outer.is_empty() {
            let mut kept = Vec::new();
            for j in joined {
                let mut ok = true;
                for (item, field, expr) in &plan.outer {
                    let row = self.filled(def.items[*item].table, &j[*item]);
                    if let Some(why) = row[*field].unknown() {
                        return unsupported(why.to_string());
                    }
                    ok &= expr.matches(&row[*field]);
                }
                if ok {
                    kept.push(j);
                }
            }
            joined = kept;
        }
        let aggregated = def.columns.iter().any(|c| c.method.is_some());
        let dirs: Vec<bool> = def.order_by.iter().map(|(_, asc)| *asc).collect();
        let mut rows = if aggregated {
            self.group_rows(def, &plan, joined)?
        } else {
            let mut rows = Vec::new();
            for j in joined {
                let columns = self.project(def, &plan, &j)?;
                if plan.post.iter().all(|(c, e)| e.matches(&columns[*c])) {
                    // Order by the OrderBy columns, then the primary key of each dataitem.
                    let mut key: Vec<Value> = def.order_by.iter().map(|(c, _)| columns[*c].clone()).collect();
                    for (item, row) in def.items.iter().zip(&j) {
                        let row = self.filled(item.table, row);
                        let t = self.engine.tables[item.table].as_ref().expect("table");
                        key.extend(t.primary_key(&row));
                    }
                    rows.push(QueryRow { columns, key });
                }
            }
            rows
        };
        // Groups with no OrderBy come back sorted by their grouping columns in the order the query
        // declares them (corpus query/PWQueryContracts.al:165-178: grouping by Grp2 then Grp1 gives
        // `A 1`, `A 2`, `B 2`, `C 1`), which `group_rows` already does; grouping columns that start
        // with the root key give that key's order (query/TestQueryCrossJoinContracts.al:202-213).
        rows.sort_by(|a, b| key_cmp(&a.key, &b.key, &dirs));
        if COLLATION_FAILED.with(|f| f.replace(false)) {
            return unsupported(
                "sorting or grouping text with characters whose database collation order is not pinned",
            );
        }
        // Rows that tie on every OrderBy column and still differ are ordered by the primary key
        // ascending (corpus query/TestQueryOrderAfterWrite.al:92-104, ascending only), or in no
        // pinned way: grouped rows. Ties under a descending OrderBy still ascend by primary key
        // (query/TestQueryOuterJoinDescendingSaveAsContracts.al, corpus: .
        let k = dirs.len();
        if aggregated && k > 0 {
            for w in rows.windows(2) {
                if key_cmp(&w[0].key[..k], &w[1].key[..k], &dirs) == Ordering::Equal {
                    // Microsoft's own queries do this (Create Pick's CalcQtyInWhseEntries orders by one Option column
                    // and groups by ten): the groups stay in the order of their grouping columns, which
                    // `group_rows` produced and the stable sort kept. Not pinned: an assumption.
                    al_builtins::assumptions::assume(
                        al_builtins::assumptions::Assumption::GroupedQueryTiesInGroupingOrder,
                    );
                    break;
                }
            }
        }
        Ok((rows, stamps))
    }

    /// Sorts every filter of the query to the place it applies.
    fn query_plan(&mut self, def: &ir::Query, runtime: &[(usize, FilterExpr)]) -> Result<Plan, Signal> {
        let engine = self.engine.clone();
        let mut plan = Plan {
            scan: vec![Vec::new(); def.items.len()],
            flow: vec![Vec::new(); def.items.len()],
            post: Vec::new(),
            having: Vec::new(),
            outer: Vec::new(),
        };
        for (i, item) in def.items.iter().enumerate() {
            let fields = &engine.program.objects[item.table].table.as_ref().expect("table").fields;
            for c in &item.table_filter {
                let ty = &fields[c.field].ty;
                let expr = match &c.value {
                    ir::CalcValue::Const(k) => FilterExpr::Eq(ops::convert(crate::constant(k), ty)?),
                    ir::CalcValue::Filter(text) => self.parse_query_filter(text, ty)?,
                    _ => return unsupported("a table filter condition of this form"),
                };
                plan.scan[i].push(FieldFilter {
                    field: c.field,
                    expr,
                    group: 0,
                });
            }
        }
        let has_right = def.items.iter().any(|i| i.right_outer);
        // The table filter of a dataitem other than the right-joined one acts on the joined rows too: a Full join with a filter on
        // the parent drops the child rows no kept parent links to (P71JoinParent.al:Parent_Full_GrpG1).
        if has_right {
            for i in 0..def.items.len() {
                if !def.items[i].right_outer {
                    for f in std::mem::take(&mut plan.scan[i]) {
                        plan.outer.push((i, f.field, f.expr));
                    }
                }
            }
        }
        for (ci, c) in def.columns.iter().enumerate() {
            // A runtime filter on a column replaces the column's static ColumnFilter (corpus
            // query/TestQueryColumnFilter.al:62-100); a table filter combines with either
            // (query/TestQueryDataItemTableFilter.al:137-229).
            let expr = match runtime.iter().find(|(r, _)| *r == ci) {
                Some((_, e)) => Some(e.clone()),
                None => match &c.static_filter {
                    Some(ir::CalcValue::Const(k)) => Some(FilterExpr::Eq(ops::convert(crate::constant(k), &c.ty)?)),
                    Some(ir::CalcValue::Filter(text)) => Some(self.parse_query_filter(text, &c.ty)?),
                    Some(_) => return unsupported("a column filter of this form"),
                    None => None,
                },
            };
            let Some(expr) = expr else { continue };
            let item = &def.items[c.item];
            if c.reverse_sign {
                return unsupported(
                    "a filter on a ReverseSign column (whether it applies before the sign flips is not pinned)",
                );
            }
            if c.method.is_some() {
                plan.having.push((ci, expr));
                continue;
            }
            let field = c.field.expect("a column over a field");
            let schema = &engine.program.objects[item.table].table.as_ref().expect("table").fields[field];
            if c.filter_element && schema.flow_filter {
                // Corpus query/TestQueryFlowFieldColumn.al:150-200: it sets the flow filter of the
                // FlowFields of the dataitem; it never constrains the rows.
                plan.flow[c.item].push(FieldFilter { field, expr, group: 0 });
            } else if schema.stored && ((item.left_outer && item.parent.is_some()) || has_right) {
                // With a right or full outer join in the query a filter acts on the joined rows: a root row filtered before the join
                // would turn the child rows it linked into unlinked ones (query/lt-outer-joins/P71FilterRightFull.al).
                plan.outer.push((c.item, field, expr));
            } else if schema.stored {
                plan.scan[c.item].push(FieldFilter { field, expr, group: 0 });
            } else {
                // A filter() element over a FlowField is pinned for a query of one data item without a Method column
                // (query/lt-cg-query-leads/CgQueryLeads.al:759,792): with a join or a grouping, which rows it is calculated
                // on is not pinned.
                if c.filter_element && (def.items.len() > 1 || def.columns.iter().any(|c| c.method.is_some())) {
                    return unsupported(
                        "a filter() element over a FlowField in a query with a join or a Method column (not pinned)",
                    );
                }
                plan.post.push((ci, expr));
            }
        }
        Ok(plan)
    }

    fn parse_query_filter(&self, text: &str, ty: &RtType) -> Result<FilterExpr, Signal> {
        self.parse_filter_text(text, ty)
            .map_err(|e| Signal::Unsupported(format!("query filter: {e}")))
    }

    /// Nested-loop join: every row of dataitem `level` that passes its filters and links to the
    /// rows chosen above it, then the next dataitem.
    fn expand(
        &mut self,
        def: &ir::Query,
        plan: &Plan,
        level: usize,
        chosen: &mut Vec<Vec<Value>>,
        out: &mut Vec<JoinedRow>,
    ) -> Result<(), Signal> {
        self.expand_to(def, plan, level, def.items.len(), chosen, out)
    }

    /// [`Self::expand`] down to the dataitems before `stop`.
    fn expand_to(
        &mut self,
        def: &ir::Query,
        plan: &Plan,
        level: usize,
        stop: usize,
        chosen: &mut Vec<Vec<Value>>,
        out: &mut Vec<JoinedRow>,
    ) -> Result<(), Signal> {
        if level == stop {
            out.push(chosen.clone());
            return Ok(());
        }
        let item = &def.items[level];
        let t: Arc<TableDef> = self.engine.tables[item.table].clone().expect("table");
        let mut filters = plan.scan[level].clone();
        // A parent (or the item the link reads) that had no match has nothing to link to.
        let parent_missing =
            item.parent.is_some_and(|p| chosen[p].is_empty()) || item.link_items.iter().any(|&a| chosen[a].is_empty());
        if let Some(parent) = item.parent.filter(|_| !parent_missing) {
            // The link reads the parent's row, or an item further out (corpus branch al-runtime-rs-staging,
            // query/PXQueryOuterLink.al:6-11). Corpus query/TestQueryDataItemLinkMultiField.al: every
            // equality of the link holds.
            // Parts of one DataItemLink may name different enclosing data items: each part joins to its item
            // (query/lt-query-links-relations-httpstream/MixedLinkInnerLeft.al:P100_Inner, P100_GrandOnly, P100_InnerColumnFilterOnA, P100_InnerColumnFilterOnC).
            for (k, (own, theirs)) in item.link.iter().enumerate() {
                let linked = item.link_items.get(k).copied().unwrap_or(parent);
                let v = ops::convert(chosen[linked][*theirs].clone(), &t.fields[*own].ty)?;
                filters.push(FieldFilter {
                    field: *own,
                    expr: FilterExpr::Eq(v),
                    group: 0,
                });
            }
        }
        let mut rows: Vec<Vec<Value>> = Vec::new();
        if !parent_missing {
            let cx = self.cx.clone();
            self.each_row(&cx, &t, &filters, |row| {
                rows.push(row.to_vec());
                Ok(())
            })?;
        }
        if rows.is_empty() && item.left_outer && item.parent.is_some() {
            // Corpus query/TestQueryJoin.al:117-145: the unmatched parent is kept, the child's
            // columns hold their defaults (`NO_MATCH` stands for the row, see `row_of`).
            rows.push(NO_MATCH);
        }
        for row in rows {
            chosen.push(row);
            self.expand_to(def, plan, level + 1, stop, chosen, out)?;
            chosen.pop();
        }
        Ok(())
    }

    /// The rows a right or full outer join adds: the rows of the dataitem that no row of the joined dataitems above links to, with
    /// those at their defaults (query/lt-outer-joins/P71JoinNone.al:Join_Right_NoFilter, Join_Full_NoFilter; P71ShapeLocFromItem.al:
    /// Shape_Full_ZoneN, with the dataitem below an inner join).
    fn expand_unlinked(&mut self, def: &ir::Query, plan: &Plan, out: &mut Vec<JoinedRow>) -> Result<(), Signal> {
        let rights: Vec<usize> = (0..def.items.len()).filter(|&i| def.items[i].right_outer).collect();
        let Some(&level) = rights.first() else {
            return Ok(());
        };
        if rights.len() != 1 || def.items[level].parent != Some(level - 1) || level + 1 != def.items.len() {
            return unsupported(
                "a right or full outer join that is not the innermost of a chain of dataitems (not pinned)",
            );
        }
        let item = &def.items[level];
        if !item.link_items.is_empty() {
            return unsupported("a right or full outer join whose link reads a dataitem further out (not pinned)");
        }
        let mut prefix: Vec<JoinedRow> = Vec::new();
        self.expand_to(def, plan, 0, level, &mut Vec::new(), &mut prefix)?;
        let ct: Arc<TableDef> = self.engine.tables[item.table].clone().expect("table");
        let cx = self.cx.clone();
        let mut children: Vec<Vec<Value>> = Vec::new();
        self.each_row(&cx, &ct, &plan.scan[level], |row| {
            children.push(row.to_vec());
            Ok(())
        })?;
        for c in children {
            let mut linked = false;
            for p in &prefix {
                let parent = &p[level - 1];
                if parent.is_empty() {
                    continue;
                }
                let mut all = true;
                for (own, theirs) in &item.link {
                    let v = ops::convert(parent[*theirs].clone(), &ct.fields[*own].ty)?;
                    all &= ops::compare(&v, &c[*own]).ok() == Some(Ordering::Equal);
                }
                if all {
                    linked = true;
                    break;
                }
            }
            if !linked {
                let mut row = vec![NO_MATCH; level];
                row.push(c);
                out.push(row);
            }
        }
        Ok(())
    }

    /// The column values of one joined row. Aggregated columns and filter elements are `Null` here.
    fn project(&mut self, def: &ir::Query, plan: &Plan, joined: &[Vec<Value>]) -> Result<Vec<Value>, Signal> {
        let engine = self.engine.clone();
        let mut out = Vec::with_capacity(def.columns.len());
        for (ci, c) in def.columns.iter().enumerate() {
            // A filter() element over a FlowField is calculated for its filter only (`plan.post`).
            if c.method.is_some() || (c.filter_element && !plan.post.iter().any(|(p, _)| *p == ci)) {
                out.push(Value::Null);
                continue;
            }
            let item = &def.items[c.item];
            let field = c.field.expect("a column over a field");
            if matches!(c.ty, RtType::Library(al_builtins::library_kind::LibraryKind::RecordId)) {
                al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::QueryColumnRecordId);
            }
            let row = &self.filled(item.table, &joined[c.item]);
            let schema = &engine.program.objects[item.table].table.as_ref().expect("table").fields[field];
            let mut v = if schema.stored {
                // A column a platform row leaves unknown cannot be a column of the result.
                if let Some(why) = row[field].unknown() {
                    return unsupported(why.to_string());
                }
                row[field].clone()
            } else {
                // A FlowField is calculated per joined row, with the flow filters of its dataitem
                // (corpus query/TestQueryFlowFieldColumn.al:37-200, query/TestQueryJoinFlowFieldGroupBy.al).
                self.check_calc_field(item.table, field)?;
                let probe = self.probe(item.table, row, &plan.flow[c.item]);
                self.calc_value(&probe, field, &mut Vec::new())?
            };
            if c.reverse_sign {
                // Corpus query/TestQueryReverseSign.al:36-77.
                v = negate(v)?;
            }
            out.push(v);
        }
        Ok(out)
    }

    /// The row of a dataitem in a joined row: the defaults when the dataitem had no match.
    fn filled(&mut self, table: usize, row: &[Value]) -> Vec<Value> {
        if row.is_empty() {
            let t = self.engine.tables[table].clone().expect("table");
            t.fields.iter().map(|f| self.default_value(&f.ty)).collect()
        } else {
            row.to_vec()
        }
    }

    fn probe(
        &mut self,
        table: usize,
        row: &[Value],
        flow: &[FieldFilter],
    ) -> Rc<RefCell<al_builtins::value::RecordState>> {
        let Value::Record(rc) = self.default_value(&RtType::Record(table as u32, false)) else {
            unreachable!("a record default");
        };
        {
            let mut r = rc.borrow_mut();
            r.fields = row.to_vec();
            r.filters = flow.to_vec();
            r.temporary = false;
            r.temp_id = None;
        }
        rc
    }

    /// The values a column's aggregate reads, one per joined row that has the dataitem. A FlowField is calculated per row
    /// with the flow filters of its dataitem, as `project` does (Method over a FlowField: corpus
    /// query/lt-query-flowfield-methods/QueryFlowFieldAvgMinMax.al).
    fn aggregated_values(
        &mut self,
        def: &ir::Query,
        plan: &Plan,
        c: &ir::QueryColumn,
        rows: &[JoinedRow],
    ) -> Result<Vec<Value>, Signal> {
        let field = c.field.expect("a column over a field");
        let item = &def.items[c.item];
        let stored = self.engine.program.objects[item.table]
            .table
            .as_ref()
            .expect("table")
            .fields[field]
            .stored;
        let mut out = Vec::new();
        for j in rows.iter().filter(|j| !j[c.item].is_empty()) {
            if stored {
                out.push(j[c.item][field].clone());
            } else {
                let row = self.filled(item.table, &j[c.item]);
                self.check_calc_field(item.table, field)?;
                let probe = self.probe(item.table, &row, &plan.flow[c.item]);
                // A FlowField over no source row is NULL to an average, a minimum and a maximum (QueryFlowFieldAvgMinMax.al).
                if matches!(
                    c.method,
                    Some(ir::QueryMethod::Average | ir::QueryMethod::Min | ir::QueryMethod::Max)
                ) && !self.calc_has_source_rows(&probe, field)?
                {
                    continue;
                }
                out.push(self.calc_value(&probe, field, &mut Vec::new())?);
            }
        }
        Ok(out)
    }

    /// Groups the joined rows by the non-aggregated columns and aggregates the rest.
    fn group_rows(&mut self, def: &ir::Query, plan: &Plan, joined: Vec<JoinedRow>) -> Result<Vec<QueryRow>, Signal> {
        use ir::QueryMethod as M;
        let group_cols: Vec<usize> = def
            .columns
            .iter()
            .enumerate()
            .filter(|(_, c)| c.method.is_none() && !c.filter_element)
            .map(|(i, _)| i)
            .collect();
        // (group key, the projected rows and their source rows)
        let mut keyed: Vec<(Vec<Value>, JoinedRow)> = Vec::new();
        for j in joined {
            let columns = self.project(def, plan, &j)?;
            if !plan.post.iter().all(|(c, e)| e.matches(&columns[*c])) {
                continue;
            }
            let key = group_cols.iter().map(|c| columns[*c].clone()).collect();
            keyed.push((key, j));
        }
        // Rows with the same group columns form one group (SQL's GROUP BY).
        keyed.sort_by(|a, b| key_cmp(&a.0, &b.0, &[]));
        let mut groups: Vec<(Vec<Value>, Vec<JoinedRow>)> = Vec::new();
        for (key, j) in keyed {
            match groups.last_mut() {
                Some((k, rows)) if key_cmp(k, &key, &[]) == Ordering::Equal => rows.push(j),
                _ => groups.push((key, vec![j])),
            }
        }
        // Corpus query/TestQueryAggregation.al:102-160: without a grouping column the whole table
        // is one group, even when it is empty.
        if groups.is_empty() && group_cols.is_empty() {
            groups.push((Vec::new(), Vec::new()));
        }
        let mut out = Vec::new();
        for (key, rows) in groups {
            let mut columns = vec![Value::Null; def.columns.len()];
            for (c, v) in group_cols.iter().zip(&key) {
                columns[*c] = v.clone();
            }
            for (ci, c) in def.columns.iter().enumerate() {
                let Some(method) = c.method else { continue };
                // A dataitem that had no match under a left outer join adds no row to its aggregates
                // (query/TestQueryOuterJoinDescendingSaveAsContracts.al: `2/sum0/cnt0`).
                let all: Vec<Value> = if method == M::Count {
                    Vec::new()
                } else {
                    self.aggregated_values(def, plan, c, &rows)?
                };
                let values = || all.clone();
                let mut v = match method {
                    M::Count => Value::Integer(rows.iter().filter(|j| !j[c.item].is_empty()).count() as i32),
                    M::Sum | M::Average => {
                        let vs = values();
                        let mut sum = Decimal::ZERO;
                        for x in &vs {
                            match x {
                                Value::Decimal(d) => sum += d,
                                Value::Integer(i) => sum += Decimal::from(*i),
                                Value::BigInteger(i) => sum += Decimal::from(*i),
                                Value::Duration(i) => sum += Decimal::from(*i),
                                other => return unsupported(format!("Sum over {}", other.describe())),
                            }
                        }
                        // Over no rows both give 0 (query/TestQueryRuntimeContracts.al:
                        // Query_ScalarAggregates_OverNoRows_ReturnOneRowOfZeros).
                        let total = if method == M::Sum || vs.is_empty() {
                            sum
                        } else {
                            let avg = sum / Decimal::from(vs.len() as i64);
                            match &c.ty {
                                // An Integer column averages to an Integer, truncated: 1.33 and 1.5
                                // both give 1 (Query_Average_IntegerColumnTruncates_...).
                                RtType::Decimal => {
                                    // A Decimal column keeps 18 places less the digits before the point:
                                    // 1.33333333333333333 (17 places), and -0.333333333333333333 (18) for the
                                    // average of -2, 0 and 1 (xmlport/lt-xmlport-query-datatransfer/XpTQueryAggregates.al:
                                    // Group_NegativeAndZero). Whether BC rounds or cuts the last place is not pinned: refuse where they differ.
                                    let int_digits = if avg.trunc().is_zero() {
                                        0
                                    } else {
                                        avg.trunc().abs().to_string().len() as u32
                                    };
                                    let places = 18u32.saturating_sub(int_digits);
                                    let cut =
                                        avg.round_dp_with_strategy(places, rust_decimal::RoundingStrategy::ToZero);
                                    let rounded = avg.round_dp_with_strategy(
                                        places,
                                        rust_decimal::RoundingStrategy::MidpointAwayFromZero,
                                    );
                                    if cut != rounded {
                                        return unsupported(
                                            "an Average of a Decimal column that needs rounding at the last kept place (not pinned)",
                                        );
                                    }
                                    cut
                                }
                                _ => avg.trunc(),
                            }
                        };
                        match &c.ty {
                            RtType::Decimal => Value::Decimal(total),
                            RtType::Integer => {
                                match i32::try_from(total.trunc().mantissa() / 10i128.pow(total.trunc().scale())) {
                                    Ok(i) => Value::Integer(i),
                                    Err(_) => return unsupported("a Sum that overflows an Integer column"),
                                }
                            }
                            RtType::BigInteger => {
                                match i64::try_from(total.trunc().mantissa() / 10i128.pow(total.trunc().scale())) {
                                    Ok(i) => Value::BigInteger(i),
                                    Err(_) => return unsupported("a Sum that overflows a BigInteger column"),
                                }
                            }
                            RtType::Duration => {
                                match i64::try_from(total.trunc().mantissa() / 10i128.pow(total.trunc().scale())) {
                                    Ok(i) => Value::Duration(i),
                                    Err(_) => return unsupported("a Sum that overflows a Duration column"),
                                }
                            }
                            other => return unsupported(format!("Sum or Average of a {} column", other.describe())),
                        }
                    }
                    M::Min | M::Max => {
                        let vs = values();
                        let mut best: Option<Value> = None;
                        for x in vs {
                            best = Some(match best {
                                None => x,
                                Some(b) => {
                                    let o = ops::compare(&x, &b)?;
                                    if (method == M::Min && o == Ordering::Less)
                                        || (method == M::Max && o == Ordering::Greater)
                                    {
                                        x
                                    } else {
                                        b
                                    }
                                }
                            });
                        }
                        match best {
                            Some(b) => b,
                            // Over no rows: 0 (the type's default).
                            None => self.default_value(&c.ty),
                        }
                    }
                };
                if c.reverse_sign {
                    // Corpus query/TestQueryReverseSign.al:79-125: the group total is negated.
                    v = negate(v)?;
                }
                columns[ci] = v;
            }
            // HAVING: the filters on aggregated columns see the group's result (corpus
            // query/TestQueryAggregation.al:182-221, TestQueryColumnFilter.al:36-100).
            if !plan.having.iter().all(|(c, e)| e.matches(&columns[*c])) {
                continue;
            }
            // Order by the OrderBy columns, then the remaining grouping columns.
            let mut sort: Vec<Value> = def.order_by.iter().map(|(c, _)| columns[*c].clone()).collect();
            sort.extend(key);
            out.push(QueryRow { columns, key: sort });
        }
        Ok(out)
    }
}
