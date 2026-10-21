//! Query objects (corpus query/): dataitems, columns and the query properties, resolved to the
//! IR's `Query`. The checker uses the column list for names and types, the lowering emits the
//! `Query` into the object. Anything the runtime cannot execute exactly is a *reason*: a query
//! with a reason cannot be declared as a variable (`Lowerer::rt`), so the tests that use it are
//! reported as not run instead of running with a guessed behaviour (hard rule 4).

use crate::calcformula::{CondValue, parse_conditions};
use crate::lower::{calc_const, rewrite_quotes, scalar_rt};
use crate::members::{FieldClass, MemberTable};
use crate::types::Ty;
use crate::workspace::{ObjKind, Resolution, Workspace};
use al_builtins::value::RtType;
use al_ir as ir;
use al_syntax::ast;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct QueryLayout {
    pub query: ir::Query,
    /// The type of each column, parallel to `query.columns`.
    pub column_tys: Vec<Ty>,
    /// Why the runtime cannot execute this query yet; empty = it can.
    pub reasons: Vec<String>,
}

impl QueryLayout {
    pub fn column(&self, name: &str) -> Option<usize> {
        self.query
            .columns
            .iter()
            .position(|c| c.name.eq_ignore_ascii_case(name))
    }
}

/// Object properties that do not change which rows come back or in what order. Anything else
/// (`ReadState`, ...) is refused until a corpus test pins it.
const HARMLESS_QUERY_PROPERTIES: &[&str] = &[
    "access",
    "querytype",
    "caption",
    "apipublisher",
    "apigroup",
    "apiversion",
    "entityname",
    "entitysetname",
    "entitycaption",
    "entitysetcaption",
    "usagecategory",
    "applicationarea",
    "obsoletestate",
    "obsoletereason",
    "obsoletetag",
    "description",
    "tooltip",
    "querycategory",
    "inherentpermissions",
    "inherententitlements",
    "permissions",
    // Which replica serves the read; the runtime has one store, and the test below reads rows it
    // wrote itself through a ReadOnly query (query/TestQueryDataItemTableFilterDatabaseConst.al:
    // 109-136, Base App's query 522).
    "dataaccessintent",
    // Help texts of the object: no corpus test sets them on a query, and they have no effect on the
    // rows (docs/probes-needed.md, issue 24).
    "abouttitle",
    "abouttext",
];

/// The text inside `raw` when `raw` is exactly one single-quoted AL string.
pub(crate) fn whole_literal(raw: &str) -> Option<String> {
    let t = raw.trim();
    let inner = t.strip_prefix('\'')?.strip_suffix('\'')?;
    // A quote inside must be doubled, or the literal ended early.
    let mut rest = inner;
    while let Some(i) = rest.find('\'') {
        rest = rest[i + 1..].strip_prefix('\'')?;
    }
    Some(inner.replace("''", "'"))
}

pub fn layout(ws: &Workspace, members: &MemberTable<'_>, oi: usize) -> QueryLayout {
    let obj = ws.object_ast(oi);
    let mut b = Builder {
        ws,
        members,
        file: ws.objects[oi].file,
        reasons: Vec::new(),
        items: Vec::new(),
        columns: Vec::new(),
        tys: Vec::new(),
    };
    // Procedures, variables and OnBeforeOpen run on the query they belong to
    // (query/TestQueryDataItemTableFilterDatabaseConst.al:96-107, handlers/TestManualObjectIntegrationEvent.al:69-89,
    // query/TestQueryOnBeforeOpenVariablesContracts.al:104-140). Only OnBeforeOpen exists as a trigger.
    for r in ast::routines_in(obj.syntax()) {
        if r.is_trigger() && !r.name().eq_ignore_ascii_case("OnBeforeOpen") {
            b.reasons.push(format!("query trigger {} is not supported", r.name()));
        }
    }
    let mut order_by_text = None;
    let mut top = 0;
    for p in obj.properties() {
        let name = p.name().to_ascii_lowercase();
        match name.as_str() {
            "orderby" => order_by_text = Some(p.value_text()),
            "topnumberofrows" => match p.value_text().trim().parse::<i32>() {
                Ok(n) if n >= 0 => top = n,
                _ => b.reasons.push(format!("TopNumberOfRows = {}", p.value_text())),
            },
            // ReadUncommitted reads the session's own uncommitted rows and the committed ones, as every
            // read here does (query/TestQueryReadStateContracts.al); ReadCommitted needs runtime 18.
            "readstate" if p.value_text().trim().eq_ignore_ascii_case("ReadUncommitted") => {}
            n if HARMLESS_QUERY_PROPERTIES.contains(&n) => {}
            _ => b
                .reasons
                .push(format!("query property {} is not supported yet", p.name())),
        }
    }
    let elements = obj.sections().find(|s| s.name().eq_ignore_ascii_case("elements"));
    let roots: Vec<ast::Section> = elements
        .iter()
        .flat_map(|e| e.sections())
        .filter(|s| s.name().eq_ignore_ascii_case("dataitem"))
        .collect();
    match roots.as_slice() {
        [root] => b.item(root, None),
        _ => b
            .reasons
            .push("a query needs exactly one top-level dataitem".to_string()),
    }
    let order_by = match order_by_text {
        Some(t) => b.order_by(&t),
        None => Vec::new(),
    };
    b.validate();
    QueryLayout {
        query: ir::Query {
            items: b.items,
            columns: b.columns,
            order_by,
            top,
        },
        column_tys: b.tys,
        reasons: b.reasons,
    }
}

struct Builder<'a> {
    ws: &'a Workspace,
    members: &'a MemberTable<'a>,
    file: usize,
    reasons: Vec<String>,
    items: Vec<ir::QueryItem>,
    columns: Vec<ir::QueryColumn>,
    tys: Vec<Ty>,
}

impl Builder<'_> {
    fn reason(&mut self, r: impl Into<String>) {
        self.reasons.push(r.into());
    }

    /// Position of the stored field `name` of table object `table`.
    fn stored_field(&mut self, table: usize, name: &str, what: &str) -> Option<usize> {
        let Some(pos) = self.members[table]
            .fields
            .iter()
            .position(|f| f.name.eq_ignore_ascii_case(name))
        else {
            self.reason(format!("{what}: `{name}` is not a field of the table"));
            return None;
        };
        let f = &self.members[table].fields[pos];
        if f.class != FieldClass::Normal {
            self.reason(format!("{what}: `{name}` is not a stored field"));
            return None;
        }
        Some(pos)
    }

    fn item(&mut self, sec: &ast::Section, parent: Option<usize>) {
        let args = sec.args();
        let name = args.first().cloned().unwrap_or_default();
        let table_name = crate::types::section_object_name(sec, 1);
        let table = match table_name.as_ref() {
            Some(t) => match self
                .ws
                .resolve_ref(self.file, ObjKind::Table, &crate::types::qualified_parts(t))
            {
                Resolution::Found(i) => i,
                _ => {
                    self.reason(format!(
                        "dataitem {name}: {}",
                        self.ws.outside_workspace(ObjKind::Table, t)
                    ));
                    usize::MAX
                }
            },
            None => {
                self.reason(format!("dataitem {name} has no table"));
                usize::MAX
            }
        };
        let idx = self.items.len();
        self.items.push(ir::QueryItem {
            name: name.clone(),
            table,
            parent,
            left_outer: parent.is_some(),
            right_outer: false,
            link: Vec::new(),
            link_items: Vec::new(),
            table_filter: Vec::new(),
        });
        let mut cross = false;
        for p in sec.properties() {
            let value = p.value_text();
            match p.name().to_ascii_lowercase().as_str() {
                "dataitemlink" => match parent {
                    Some(par) => self.link(idx, par, &value),
                    None => self.reason(format!("dataitem {name}: DataItemLink on the top-level dataitem")),
                },
                "sqljointype" => match value.trim().to_ascii_lowercase().as_str() {
                    // Corpus query/TestQueryJoin.al:79,117: InnerJoin drops the parent without a
                    // child, LeftOuterJoin keeps it.
                    "innerjoin" => self.items[idx].left_outer = false,
                    "leftouterjoin" => self.items[idx].left_outer = true,
                    // query/TestQueryCrossJoinContracts.al:162-226: the product of both tables, an
                    // inner join whose every inner row matches (no rows when the inner table is empty).
                    "crossjoin" if parent.is_some() => {
                        self.items[idx].left_outer = false;
                        cross = true;
                    }
                    // query/lt-outer-joins/P71JoinNone.al:Join_Right_NoFilter, Join_Full_NoFilter: a right join keeps the child rows
                    // no parent links to, a full join keeps those and the parents without a child.
                    "rightouterjoin" if parent.is_some() => {
                        self.items[idx].left_outer = false;
                        self.items[idx].right_outer = true;
                    }
                    "fullouterjoin" if parent.is_some() => {
                        self.items[idx].left_outer = true;
                        self.items[idx].right_outer = true;
                    }
                    other => self.reason(format!("dataitem {name}: SqlJoinType = {other} is not supported yet")),
                },
                "dataitemtablefilter" => {
                    if table != usize::MAX {
                        let conds = self.table_filter(table, &value);
                        self.items[idx].table_filter = conds;
                    }
                }
                other => self.reason(format!("dataitem {name}: property {other} is not supported yet")),
            }
        }
        if cross && !self.items[idx].link.is_empty() {
            self.reason(format!("dataitem {name}: CrossJoin with a DataItemLink (not pinned)"));
        } else if parent.is_some() && !cross && self.items[idx].link.is_empty() {
            self.reason(format!("dataitem {name} is nested without a DataItemLink"));
        }
        let mut nested = 0;
        for m in sec.sections() {
            match m.name().to_ascii_lowercase().as_str() {
                "dataitem" => {
                    nested += 1;
                    if nested > 1 {
                        self.reason(format!(
                            "dataitem {name} has several nested dataitems (the join of siblings is not supported yet)"
                        ));
                    } else {
                        self.item(&m, Some(idx));
                    }
                }
                "column" | "filter" => self.column(&m, idx),
                other => self.reason(format!("dataitem {name}: element {other} is not supported yet")),
            }
        }
    }

    /// `DataItemLink = "Child Field" = Parent."Parent Field", ...`. The field on the right may belong to an
    /// item further out than the parent: the link then reads that item's current row (corpus branch
    /// al-runtime-rs-staging, query/PXQueryOuterLink.al:6-11, probed on BC 27.5). Each part reads the item it
    /// names; parts naming two different items are an assumption (docs/probes-needed.md).
    fn link(&mut self, idx: usize, parent: usize, text: &str) {
        let table = self.items[idx].table;
        if table == usize::MAX {
            return;
        }
        let mut items = Vec::new();
        for part in crate::scan::split_query(text, ',') {
            let Some((l, r)) = split_once_top(&part, '=') else {
                self.reason(format!("DataItemLink `{part}` is not understood"));
                continue;
            };
            let Some((who, field)) = split_once_top(r.trim(), '.') else {
                self.reason(format!("DataItemLink `{part}` does not name a parent field"));
                continue;
            };
            // The enclosing items, nearest first.
            let mut anc = Some(parent);
            let mut found = None;
            while let Some(a) = anc {
                if crate::scan::unquote_double(&who).eq_ignore_ascii_case(&self.items[a].name) {
                    found = Some(a);
                    break;
                }
                anc = self.items[a].parent;
            }
            let Some(target) = found else {
                self.reason(format!(
                    "DataItemLink `{part}` links to {who}, which is no enclosing dataitem of {}",
                    self.items[idx].name
                ));
                continue;
            };
            let ttable = self.items[target].table;
            if ttable == usize::MAX {
                return;
            }
            let own = self.stored_field(table, &crate::scan::unquote_double(&l), "DataItemLink");
            let theirs = self.stored_field(ttable, &crate::scan::unquote_double(&field), "DataItemLink");
            if let (Some(o), Some(t)) = (own, theirs) {
                self.items[idx].link.push((o, t));
                items.push(target);
            }
        }
        if items.iter().any(|&t| t != parent) {
            self.items[idx].link_items = items;
        }
    }

    /// `DataItemTableFilter = Status = const(Open), Amount = filter(> 100)`.
    fn table_filter(&mut self, table: usize, text: &str) -> Vec<ir::CalcCond> {
        let conds = match parse_conditions(text) {
            Ok(c) => c,
            Err(e) => {
                self.reason(format!("DataItemTableFilter {e}"));
                return Vec::new();
            }
        };
        let mut out: Vec<ir::CalcCond> = Vec::new();
        for c in conds {
            let Some(pos) = self.stored_field(table, &c.field, "DataItemTableFilter") else {
                continue;
            };
            let ty = self.members[table].fields[pos].ty.clone();
            let value = match &c.value {
                // Corpus query/TestQueryDataItemTableFilterDatabaseConst.al:159-190: a Database::
                // const is the object id.
                CondValue::Const(raw) => match calc_const(self.ws, self.members, self.file, raw, &ty) {
                    Ok(k) => ir::CalcValue::Const(k),
                    Err(e) => {
                        self.reason(format!("DataItemTableFilter: {e}"));
                        continue;
                    }
                },
                // `filter('>=2')`: a filter written as one AL string is that filter's text
                // (query/TestQueryCrossJoinContracts.al:187-198 filters Id >= 2 and Qty > 5 this way).
                CondValue::Filter(raw) => ir::CalcValue::Filter(match whole_literal(raw) {
                    Some(inner) => inner,
                    None => rewrite_quotes(raw),
                }),
                _ => {
                    self.reason("DataItemTableFilter with field(...) is not supported yet");
                    continue;
                }
            };
            // Two conditions on one field: how BC combines them is not pinned.
            if out.iter().any(|o| o.field == pos) {
                self.reason(format!(
                    "DataItemTableFilter has two conditions on `{}` (their combination is not pinned yet)",
                    c.field
                ));
                continue;
            }
            out.push(ir::CalcCond { field: pos, value });
        }
        out
    }

    fn column(&mut self, sec: &ast::Section, item: usize) {
        let filter_element = sec.name().eq_ignore_ascii_case("filter");
        let args = sec.args();
        let name = args.first().cloned().unwrap_or_default();
        let table = self.items[item].table;
        let mut method = None;
        let mut reverse_sign = false;
        let mut caption = None;
        let mut filter_text = None;
        for p in sec.properties() {
            let v = p.value_text();
            match p.name().to_ascii_lowercase().as_str() {
                "method" => {
                    method = match v.trim().to_ascii_lowercase().as_str() {
                        "sum" => Some(ir::QueryMethod::Sum),
                        "count" => Some(ir::QueryMethod::Count),
                        "average" => Some(ir::QueryMethod::Average),
                        "min" => Some(ir::QueryMethod::Min),
                        "max" => Some(ir::QueryMethod::Max),
                        other => {
                            self.reason(format!("column {name}: Method = {other} is not supported yet"));
                            None
                        }
                    }
                }
                "reversesign" => reverse_sign = v.trim().eq_ignore_ascii_case("true"),
                "caption" => caption = single_quoted(&v),
                "columnfilter" => filter_text = Some(v),
                "description" | "tooltip" => {}
                other => self.reason(format!("column {name}: property {other} is not supported yet")),
            }
        }
        // The source field. A `Count` column has none.
        let source = match args.get(1) {
            Some(f) if table != usize::MAX => {
                let f = crate::scan::unquote_double(f);
                match self.members[table]
                    .fields
                    .iter()
                    .position(|x| x.name.eq_ignore_ascii_case(&f))
                {
                    Some(p) => Some(p),
                    None => {
                        self.reason(format!("column {name}: `{f}` is not a field of the table"));
                        None
                    }
                }
            }
            _ => None,
        };
        let mut ty = Ty::Unknown;
        if let Some(pos) = source {
            let f = &self.members[table].fields[pos];
            let (class, fty) = (f.class.clone(), f.ty.clone());
            match (&class, filter_element) {
                (FieldClass::Normal, _) | (FieldClass::FlowFilter, true) => {}
                (FieldClass::FlowField, false) if method.is_none() => {}
                // Sum, Average, Min and Max add up the values calculated per row; Base Application's "Cust. Ledg. Entry
                // Remain. Amt." sums "Remaining Amt. (LCY)" (assumption query-method-over-flowfield).
                (FieldClass::FlowField, false)
                    if matches!(
                        method,
                        Some(ir::QueryMethod::Sum | ir::QueryMethod::Average | ir::QueryMethod::Min | ir::QueryMethod::Max)
                    ) => {}
                // Base Application's "Vend. Ledg. Entry Remain. Amt." sums a FlowField; the corpus pins
                // a FlowField column only without a method (query/TestQueryJoinFlowFieldGroupBy.al).
                (FieldClass::FlowField, false) => self.reason(format!(
                    "column {name}: Method over a FlowField: whether BC sums the calculated values per row is not pinned (docs/probes-needed.md)"
                )),
                _ => self.reason(format!(
                    "column {name}: a {class:?} field in this position (or with Method) is not supported yet"
                )),
            }
            ty = fty;
        }
        match method {
            Some(ir::QueryMethod::Count) => {
                if source.is_some() {
                    self.reason(format!("column {name}: Count over a field is not supported yet"));
                }
                ty = Ty::Integer;
            }
            Some(ir::QueryMethod::Sum | ir::QueryMethod::Average) => {
                // Decimal sources (corpus query/TestQueryAggregation.al) and Integer sources keep
                // their type: Sum of an Integer column is an Integer and its Average is truncated
                // (query/TestQueryRuntimeContracts.al:Query_Average_IntegerColumnTruncates_...,
                // :Query_ScalarAggregates_OverNoRows_...). BigInteger and Duration sources are legal
                // AL (AL0347) but their column type is not pinned.
                if ty != Ty::Decimal && ty != Ty::Integer {
                    self.reason(format!(
                        "column {name}: Sum and Average over a field that is not Decimal or Integer"
                    ));
                }
            }
            Some(ir::QueryMethod::Min | ir::QueryMethod::Max) | None => {
                if source.is_none() {
                    self.reason(format!("column {name} has no data source"));
                }
            }
        }
        if reverse_sign && method.is_some() && method != Some(ir::QueryMethod::Sum) {
            self.reason(format!("column {name}: ReverseSign with a Method other than Sum"));
        }
        if filter_element && (method.is_some() || reverse_sign) {
            self.reason(format!("filter {name} with Method or ReverseSign"));
        }
        if reverse_sign && !matches!(ty, Ty::Decimal | Ty::Integer | Ty::BigInteger) {
            self.reason(format!("column {name}: ReverseSign on a non-number"));
        }
        let rt = match scalar_rt(self.ws, self.file, &ty) {
            Some(Ok(rt)) => rt,
            Some(Err(e)) => {
                self.reason(format!("column {name}: {e}"));
                RtType::Variant
            }
            // A RecordId column passes the stored value through (Base Application's "Workflow Instance" query has one and no
            // code reads it); no corpus test pins a RecordId column, so a Method, a ColumnFilter or a ReverseSign on it stays refused
            // (assumption query-column-record-id).
            None if ty == Ty::Builtin("RecordId".into())
                && source.is_some()
                && method.is_none()
                && filter_text.is_none() =>
            {
                RtType::Library(al_builtins::library_kind::LibraryKind::RecordId)
            }
            None => {
                if ty != Ty::Unknown {
                    self.reason(format!("column {name}: a column of type {ty}"));
                }
                RtType::Variant
            }
        };
        let static_filter = filter_text.and_then(|t| self.column_filter(&name, &ty, &t));
        self.columns.push(ir::QueryColumn {
            name,
            caption,
            item,
            field: source,
            filter_element,
            method,
            reverse_sign,
            ty: rt,
            static_filter,
        });
        self.tys.push(ty);
    }

    /// `ColumnFilter = Name = const('C1')` / `Name = filter(> 0)`: one condition on the column itself.
    fn column_filter(&mut self, column: &str, ty: &Ty, text: &str) -> Option<ir::CalcValue> {
        let conds = match parse_conditions(text) {
            Ok(c) => c,
            Err(e) => {
                self.reason(format!("ColumnFilter {e}"));
                return None;
            }
        };
        let [c] = conds.as_slice() else {
            self.reason(format!("column {column}: ColumnFilter with several conditions"));
            return None;
        };
        if !c.field.eq_ignore_ascii_case(column) {
            self.reason(format!(
                "column {column}: ColumnFilter on `{}`, which is not the column itself",
                c.field
            ));
            return None;
        }
        match &c.value {
            CondValue::Const(raw) => match calc_const(self.ws, self.members, self.file, raw, ty) {
                Ok(k) => Some(ir::CalcValue::Const(k)),
                Err(e) => {
                    self.reason(format!("ColumnFilter: {e}"));
                    None
                }
            },
            CondValue::Filter(raw) => Some(ir::CalcValue::Filter(rewrite_quotes(raw))),
            _ => {
                self.reason("ColumnFilter with field(...) is not supported yet");
                None
            }
        }
    }

    /// `OrderBy = ascending(A), descending(B)`.
    fn order_by(&mut self, text: &str) -> Vec<(usize, bool)> {
        let mut out = Vec::new();
        for part in crate::scan::split_query(text, ',') {
            let part = part.trim();
            let lower = part.to_ascii_lowercase();
            let (asc, rest) = if let Some(r) = lower.strip_prefix("ascending") {
                (true, &part[part.len() - r.len()..])
            } else if let Some(r) = lower.strip_prefix("descending") {
                (false, &part[part.len() - r.len()..])
            } else {
                self.reason(format!("OrderBy `{part}` is not understood"));
                continue;
            };
            let rest = rest.trim();
            let Some(inner) = rest.strip_prefix('(').and_then(|r| r.strip_suffix(')')) else {
                self.reason(format!("OrderBy `{part}` is not understood"));
                continue;
            };
            // `ascending(A, B)` names several columns in one direction (Base Application's "Item
            // Application Entries Inb."), as `ascending(A), ascending(B)`.
            for col in crate::scan::split_query(inner, ',') {
                let col = crate::scan::unquote_double(&col);
                match self.columns.iter().position(|c| c.name.eq_ignore_ascii_case(&col)) {
                    Some(i) if !self.columns[i].filter_element => out.push((i, asc)),
                    _ => self.reason(format!("OrderBy names `{col}`, which is not a column of the dataset")),
                }
            }
        }
        out
    }

    /// Combinations whose result the corpus does not pin.
    fn validate(&mut self) {
        // Aggregates over a left outer join skip the missing side: sum 0 and count 0 for a parent
        // without children (query/TestQueryOuterJoinDescendingSaveAsContracts.al).
        // Groups without an OrderBy (query/TestQueryAggregation.al always orders its groups) have no
        // pinned order, but that only matters when a run produces several: the VM refuses then
        // (`Session::query_rows`), so a query like Base App's 1302 can be declared and used.
        // A FlowField on the optional side of an outer join would be calculated on the default row.
        for c in &self.columns {
            let item = &self.items[c.item];
            let Some(field) = c.field else { continue };
            if item.left_outer && item.parent.is_some() && item.table != usize::MAX {
                let f = &self.members[item.table].fields[field];
                if f.class != FieldClass::Normal {
                    let r = format!(
                        "column {} reads a FlowField of an outer-joined dataitem (not pinned)",
                        c.name
                    );
                    self.reasons.push(r);
                }
            }
        }
        // A static filter or table filter on the optional side of an outer join acts after the join: a parent row
        // whose joined row is missing or fails the filter is gone, so the join is an inner join in effect
        // (corpus query/lt-outer-joins/P71JoinStatic.al: Join_Left_StaticActive gives 2 of 3 items,
        // Join_Left_StaticZoneBlank gives none; P71FilterCombined.al for a run time filter on top).
        // A ColumnFilter on a Count column is a filter on the groups (HAVING), not on the joined rows: the join
        // stays outer and `const(0)` keeps the parents without children
        // (errors/lt-ms-refusals-133/P133QueryOuterFilters.al:321-323, :361-362, default join and LeftOuterJoin).
        // Other aggregates with a ColumnFilter on the optional side are not pinned.
        for c in &self.columns {
            let item = &self.items[c.item];
            if item.left_outer
                && item.parent.is_some()
                && c.static_filter.is_some()
                && c.method.is_some_and(|m| m != ir::QueryMethod::Count)
            {
                self.reasons.push(format!(
                    "column {} has a ColumnFilter on an aggregate other than Count of an outer-joined dataitem (not pinned)",
                    c.name
                ));
            }
        }
        let filtered: Vec<bool> = self
            .items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                !item.table_filter.is_empty()
                    || self
                        .columns
                        .iter()
                        .any(|c| c.item == i && c.static_filter.is_some() && c.method.is_none())
            })
            .collect();
        for (item, filtered) in self.items.iter_mut().zip(filtered) {
            if item.left_outer && item.parent.is_some() && filtered {
                item.left_outer = false;
            }
        }
    }
}

fn split_once_top(text: &str, sep: char) -> Option<(String, String)> {
    let parts = crate::scan::split_query(text, sep);
    if parts.len() < 2 {
        return None;
    }
    let first = parts[0].clone();
    let rest = text[first.len() + sep.len_utf8()..].to_string();
    Some((first, rest))
}

/// A property value that is exactly one single-quoted AL string: its text.
fn single_quoted(v: &str) -> Option<String> {
    let t = v.trim();
    let inner = t.strip_prefix('\'')?.strip_suffix('\'')?;
    // An inner quote must be doubled; anything else is more than one literal.
    let mut chars = inner.chars().peekable();
    let mut out = String::new();
    while let Some(c) = chars.next() {
        if c == '\'' {
            if chars.next() != Some('\'') {
                return None;
            }
            out.push('\'');
        } else {
            out.push(c);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::AppSource;

    const TABLES: &str = r#"
table 50000 "T" { fields { field(1; "No."; Code[20]) { } field(2; Amount; Decimal) { } field(3; Qty; Integer) { } field(5; Big; BigInteger) { }
    field(4; Total; Decimal) { FieldClass = FlowField; CalcFormula = sum(U.Amount where("No." = field("No."))); } }
    keys { key(PK; "No.") { Clustered = true; } } }
table 50001 "U" { fields { field(1; "No."; Code[20]) { } field(2; Amount; Decimal) { } }
    keys { key(PK; "No.") { Clustered = true; } } }
"#;

    fn layout_of(query: &str) -> QueryLayout {
        let src = format!("{TABLES}\n{query}");
        let ws = Workspace::load(
            vec![AppSource {
                id: "a".into(),
                name: "a".into(),
                dependencies: vec![],
                files: vec![("f.al".into(), src)],
            }],
            &Default::default(),
        );
        let members = crate::members::build(&ws);
        let oi = ws
            .objects
            .iter()
            .position(|o| o.kind == ObjKind::Query)
            .expect("a query");
        layout(&ws, &members, oi)
    }

    // Corpus pins a FlowField column without a method (query/TestQueryJoinFlowFieldGroupBy.al, QffHeaderFlowField);
    // an aggregate over a FlowField (Base Application's "Cust. Ledg. Entry Remain. Amt.") is in no corpus test: it is
    // accepted, assumed to add the per-row values (query-method-over-flowfield), and Count over one stays refused.
    #[test]
    fn an_aggregate_over_a_flow_field_is_accepted_as_assumed() {
        let l = layout_of("query 50100 Q { elements { dataitem(T; T) { column(S; Total) { Method = Sum; } } } }");
        assert!(l.reasons.is_empty(), "{:?}", l.reasons);
        let l = layout_of("query 50100 Q { elements { dataitem(T; T) { column(S; Total) { } } } }");
        assert!(l.reasons.is_empty(), "{:?}", l.reasons);
        let l = layout_of("query 50100 Q { elements { dataitem(T; T) { column(S; Total) { Method = Count; } } } }");
        assert!(!l.reasons.is_empty());
    }

    // corpus query/QJOrderSum.Query.al: the columns, their types and the OrderBy.
    #[test]
    fn columns_types_and_order_are_resolved() {
        let l = layout_of(
            "query 50100 Q { OrderBy = descending(No); TopNumberOfRows = 3; elements { dataitem(T; T) {
                column(No; \"No.\") { Caption = 'The number'; }
                column(Sum; Amount) { Method = Sum; }
                column(Cnt) { Method = Count; }
                filter(QtyFilter; Qty) { ColumnFilter = QtyFilter = filter(1..5); }
            } } }",
        );
        assert_eq!(l.reasons, Vec::<String>::new());
        assert_eq!(
            l.column_tys,
            vec![Ty::Code(Some(20)), Ty::Decimal, Ty::Integer, Ty::Integer]
        );
        assert_eq!(l.query.order_by, vec![(0, false)]);
        assert_eq!(l.query.top, 3);
        assert_eq!(l.query.columns[0].caption.as_deref(), Some("The number"));
        assert!(l.query.columns[3].filter_element);
        assert_eq!(
            l.query.columns[3].static_filter,
            Some(ir::CalcValue::Filter("1..5".into()))
        );
        assert_eq!(l.column("SUM"), Some(1));
    }

    // Base Application's "Item Application Entries Inb.": `OrderBy = ascending(A, B, C, D)` names several
    // columns in one direction, which reads as `ascending(A), ascending(B), ...` (the order by several
    // columns in turn is docs/probes-needed.md "Rows are ordered by the OrderBy columns").
    #[test]
    fn one_direction_may_name_several_columns() {
        let l = layout_of(
            "query 50100 Q { OrderBy = ascending(Sum, No), descending(Cnt); elements { dataitem(T; T) {
                column(No; \"No.\") { }
                column(Sum; Amount) { }
                column(Cnt; Qty) { }
            } } }",
        );
        assert_eq!(l.reasons, Vec::<String>::new());
        assert_eq!(l.query.order_by, vec![(1, true), (0, true), (2, false)]);
    }

    // corpus query/QjCustOrdersImplicitJoin.Query.al: a nested dataitem without SqlJoinType is a
    // left outer join (TestQueryJoinDefaultType.al:69); DataItemLink may name several fields
    // (TestQueryDataItemLinkMultiField.al).
    #[test]
    fn joins_default_to_left_outer_and_links_name_every_field() {
        let l = layout_of(
            "query 50100 Q { elements { dataitem(A; T) { column(No; \"No.\") { }
                dataitem(B; U) { DataItemLink = \"No.\" = A.\"No.\", Amount = A.Amount; column(X; Amount) { } } } } }",
        );
        assert_eq!(l.reasons, Vec::<String>::new());
        assert!(!l.query.items[0].left_outer);
        assert!(l.query.items[1].left_outer);
        assert_eq!(l.query.items[1].link, vec![(0, 0), (1, 1)]);
    }

    // Unpinned shapes are reasons, not guesses (hard rule 4).
    #[test]
    fn unsupported_shapes_are_refused_with_a_reason() {
        let has = |q: &str, needle: &str| {
            let l = layout_of(q);
            assert!(
                l.reasons.iter().any(|r| r.contains(needle)),
                "{needle}: {:?}",
                l.reasons
            );
        };
        // Sibling dataitems.
        has(
            "query 50100 Q { elements { dataitem(A; T) { column(No; \"No.\") { }
                dataitem(B; U) { DataItemLink = \"No.\" = A.\"No.\"; column(X; Amount) { } }
                dataitem(C; U) { DataItemLink = \"No.\" = A.\"No.\"; column(Y; Amount) { } } } } }",
            "several nested dataitems",
        );
        // A join type that does not exist.
        has(
            "query 50100 Q { elements { dataitem(A; T) { column(No; \"No.\") { }
                dataitem(B; U) { DataItemLink = \"No.\" = A.\"No.\"; SqlJoinType = SidewaysJoin; column(X; Amount) { } } } } }",
            "sidewaysjoin",
        );
        // Right and full outer joins keep the child rows no parent links to (query/lt-outer-joins/P71JoinNone.al).
        for (join, left, right) in [("RightOuterJoin", false, true), ("FullOuterJoin", true, true)] {
            let l = layout_of(&format!(
                "query 50100 Q {{ elements {{ dataitem(A; T) {{ column(No; \"No.\") {{ }}
                dataitem(B; U) {{ DataItemLink = \"No.\" = A.\"No.\"; SqlJoinType = {join}; column(X; Amount) {{ }} }} }} }} }}"
            ));
            assert_eq!(l.reasons, Vec::<String>::new());
            assert_eq!(
                (l.query.items[1].left_outer, l.query.items[1].right_outer),
                (left, right)
            );
        }
        // Query code: procedures, variables and an OnBeforeOpen run (query/TestQueryOnBeforeOpenVariablesContracts.al);
        // another trigger does not exist.
        assert!(
            layout_of(
                "query 50100 Q { elements { dataitem(A; T) { column(No; \"No.\") { } } } var N: Integer; trigger OnBeforeOpen() begin N += 1; end; procedure P() begin end; }"
            )
            .reasons
            .is_empty()
        );
        has(
            "query 50100 Q { elements { dataitem(A; T) { column(No; \"No.\") { } } } trigger OnAfterOpen() begin end; }",
            "OnAfterOpen",
        );
        // Grouped aggregate without an OrderBy is refused by the VM when a run yields several groups.
        assert!(
            layout_of(
                "query 50100 Q { elements { dataitem(A; T) { column(No; \"No.\") { } column(S; Amount) { Method = Sum; } } } }"
            )
            .reasons
            .is_empty()
        );
        // Sum over a BigInteger field (an Integer field is pinned: query/TestQueryRuntimeContracts.al).
        has(
            "query 50100 Q { elements { dataitem(A; T) { column(S; Big) { Method = Sum; } } } }",
            "not Decimal or Integer",
        );
        // A table outside the workspace.
        has(
            "query 50100 Q { elements { dataitem(A; Nowhere) { column(No; \"No.\") { } } } }",
            "outside the workspace",
        );
    }

    /// A ColumnFilter on a Count column of the optional side keeps the join outer
    /// (errors/lt-ms-refusals-133/P133QueryOuterFilters.al:321-323); on another aggregate it is not pinned.
    #[test]
    fn a_count_column_filter_keeps_the_join_outer_and_other_aggregates_are_refused() {
        let l = layout_of(
            "query 50100 Q { elements { dataitem(A; T) { column(No; \"No.\") { }
                dataitem(B; U) { DataItemLink = \"No.\" = A.\"No.\"; column(N) { Method = Count; ColumnFilter = N = const(0); } } } } }",
        );
        assert_eq!(l.reasons, Vec::<String>::new());
        assert!(l.query.items[1].left_outer);
        let l = layout_of(
            "query 50100 Q { elements { dataitem(A; T) { column(No; \"No.\") { }
                dataitem(B; U) { DataItemLink = \"No.\" = A.\"No.\"; column(S; Amount) { Method = Sum; ColumnFilter = S = const(0); } } } } }",
        );
        assert!(
            l.reasons
                .iter()
                .any(|r| r.contains("ColumnFilter on an aggregate other than Count")),
            "{:?}",
            l.reasons
        );
    }

    /// A static filter on the optional side of an outer join acts after the join, so the join is an inner join in effect
    /// (query/lt-outer-joins/P71JoinStatic.al).
    #[test]
    fn a_static_filter_on_the_optional_side_makes_the_join_inner() {
        let l = layout_of(
            "query 50100 Q { elements { dataitem(A; T) { column(No; \"No.\") { }
                dataitem(B; U) { DataItemLink = \"No.\" = A.\"No.\"; DataItemTableFilter = Amount = filter(>0); column(X; Amount) { } } } } }",
        );
        assert_eq!(l.reasons, Vec::<String>::new());
        assert!(!l.query.items[1].left_outer);
    }

    #[test]
    fn splitting_respects_quotes_and_parentheses() {
        assert_eq!(
            crate::scan::split_query("\"A,B\" = X.\"C\", D = X.E", ','),
            vec!["\"A,B\" = X.\"C\"".to_string(), " D = X.E".to_string()]
        );
        assert_eq!(
            split_once_top("Customer.\"No.\"", '.'),
            Some(("Customer".to_string(), "\"No.\"".to_string()))
        );
        assert_eq!(crate::scan::unquote_double(" \"No.\" "), "No.");
    }

    #[test]
    fn captions_are_single_quoted_literals() {
        assert_eq!(
            single_quoted("'Column; with semicolon'").as_deref(),
            Some("Column; with semicolon")
        );
        assert_eq!(
            single_quoted("'\"Quoted\" column'").as_deref(),
            Some("\"Quoted\" column")
        );
        assert_eq!(single_quoted("'It''s'").as_deref(), Some("It's"));
        assert_eq!(single_quoted("'A' 'B'"), None);
        assert_eq!(single_quoted("Label"), None);
    }
}
