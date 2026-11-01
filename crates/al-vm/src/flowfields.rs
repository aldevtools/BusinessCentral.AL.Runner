//! FlowFields (`CalcFormula`), FlowFilters, `CalcFields`, `CalcSums` and
//! `SetAutoCalcFields`.
//!
//! Every behaviour here is pinned by a corpus test (cited at the code) or is
//! refused with `Signal::Unsupported`; the open questions are listed in
//! docs/probes-needed.md. Aggregates are computed over the store's scan API
//! (ADR-003): a FlowField reads the persistent table, one row at a time.

use crate::Session;
use al_builtins::describe::Describe;
use al_builtins::filter::{FieldFilter, FilterExpr, parse_filter_with};
use al_builtins::native::{AlError, Signal, text_of};
use al_builtins::ops;
use al_builtins::record_op::RecordOp;
use al_builtins::value::{RecordState, RtType, Value};
use al_data::{Scan, Store, TableDef, position};
use al_ir as ir;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::rc::Rc;

type Rec = Rc<RefCell<RecordState>>;

/// Whether the rows of `t` that pass `filters`, read in primary key order, are in the order of
/// `field`: the first row has the smallest value and the last the largest. That holds when
/// `field` is a number or date in the primary key and every key field before it is pinned by
/// one equality filter. Then min and max need one row, not the whole range.
fn key_ordered_by(t: &TableDef, filters: &[FieldFilter], field: usize) -> bool {
    let orderable = matches!(
        t.fields[field].ty,
        RtType::Integer | RtType::BigInteger | RtType::Decimal | RtType::Date | RtType::Time | RtType::DateTime
    );
    let Some(at) = t.keys[0].iter().position(|&k| k == field).filter(|_| orderable) else {
        return false;
    };
    t.keys[0][..at].iter().all(|&k| {
        let mut on_k = filters.iter().filter(|f| f.field == k);
        matches!(
            (on_k.next(), on_k.next()),
            (
                Some(FieldFilter {
                    expr: FilterExpr::Eq(_),
                    ..
                }),
                None
            )
        )
    })
}

fn unsupported<T>(what: impl Into<String>) -> Result<T, Signal> {
    Err(Signal::Unsupported(what.into()))
}

/// A runtime error with a message the corpus pins and, where it is pinned, a code.
fn error<T>(message: String) -> Result<T, Signal> {
    error_code(message, "")
}

fn error_code<T>(message: String, code: &str) -> Result<T, Signal> {
    Err(Signal::Error(Box::new(AlError {
        message,
        code: code.into(),
        call_stack: String::new(),
        ..Default::default()
    })))
}

/// BC's line break inside error texts: a carriage return and a backslash.
const BR: &str = "\\";

/// How BC names a field type in the "must have the same type" message.
fn type_name(t: &RtType) -> String {
    match t {
        RtType::Boolean => "Boolean".into(),
        RtType::Integer => "Integer".into(),
        RtType::BigInteger => "BigInteger".into(),
        RtType::Decimal => "Decimal".into(),
        RtType::Duration => "Duration".into(),
        RtType::Date => "Date".into(),
        RtType::Time => "Time".into(),
        RtType::DateTime => "DateTime".into(),
        RtType::Guid => "Guid".into(),
        RtType::Text(_) => "Text".into(),
        RtType::Code(_) => "Code".into(),
        RtType::Option(_) => "Option".into(),
        RtType::Enum(_) => "Enum".into(),
        other => other.describe(),
    }
}

/// Types that compare against each other when a filter written for one field
/// is applied to another: text with code, otherwise the same type.
fn same_kind(a: &RtType, b: &RtType) -> bool {
    use RtType as T;
    match (a, b) {
        (T::Text(_) | T::Code(_), T::Text(_) | T::Code(_)) => true,
        (T::Enum(x), T::Enum(y)) => x == y,
        _ => std::mem::discriminant(a) == std::mem::discriminant(b),
    }
}

fn number(v: &Value) -> Option<Decimal> {
    Some(match v {
        Value::Integer(i) => Decimal::from(*i),
        Value::BigInteger(i) => Decimal::from(*i),
        Value::Decimal(d) => *d,
        Value::Duration(ms) => Decimal::from(*ms),
        _ => return None,
    })
}

/// A decimal result as a value of `ty`; whole numbers only for integral
/// types. `truncate_average` (averages): an average stored in an integral FlowField is cut
/// toward zero (corpus record/TestCalcFieldsTruncateUniqueContracts.al:
/// CalcFields_AverageIntoIntegerFlowField_TruncatesTowardZero, 1.5 -> 1, -1.67 -> -1).
fn to_number(d: Decimal, ty: &RtType, truncate_average: bool) -> Result<Value, Signal> {
    let d = if truncate_average && !matches!(ty, RtType::Decimal) {
        d.trunc()
    } else {
        d
    };
    match ty {
        RtType::Duration => match d.to_i64() {
            Some(ms) if d.fract() == Decimal::ZERO => Ok(Value::Duration(ms)),
            _ => unsupported(format!("{d} as a Duration")),
        },
        // A result beyond an integral field overflows (record/TestProbeCalcFieldsContracts.al:CalcFields_BigIntegerIntoInteger).
        RtType::Integer if d.fract() == Decimal::ZERO && d.to_i32().is_none() => {
            al_builtins::native::raise("Arithmetic operation resulted in an overflow.", "AL:Overflow")
        }
        _ => ops::convert(Value::Decimal(d), ty),
    }
}

/// The field id of `SystemRowVersion` (al-sema `members.rs`).
const SYSTEM_ROW_VERSION: u32 = 2_000_000_005;

impl Session {
    /// Whether this run has the newer behaviour of `gate` (ADR-015 3.4).
    pub(crate) fn has(&self, gate: al_version::Gate) -> bool {
        al_version::gate_on(self.engine.target.as_ref(), gate)
    }

    /// Why a field cannot be handed to `CalcFields`, or `Ok` when it is a
    /// FlowField with a formula this runtime can calculate.
    pub(crate) fn check_calc_field(&mut self, table: usize, pos: usize) -> Result<(), Signal> {
        let engine = self.engine.clone();
        let schema = engine.program.objects[table].table.as_ref().expect("table");
        let f = &schema.fields[pos];
        let tcap = self.table_caption(table);
        // Corpus record/TestCalcFieldsFieldClass.Codeunit.al:76,92,108: only a FlowField
        // (or a Blob) is accepted, and the refusal names the field and its table.
        if f.stored || f.flow_filter {
            if let (true, RtType::Library(n)) = (f.stored, &f.ty) {
                return unsupported(format!(
                    "CalcFields on a {n} field (streams and media are not implemented yet)"
                ));
            }
            // Captions, not names, and code DB:Ok (corpus record/TestCalcFieldsTruncateUniqueContracts.al:
            // CalcFields_OnANormalField_UsesTheFieldAndTableCaptions_CodeDbOk).
            let fcap = self.field_caption(&self.engine.tables[table].clone().expect("table"), pos);
            return error_code(
                format!("The {fcap} field in the {tcap} table must be a FlowField."),
                "DB:Ok",
            );
        }
        if let Some(reason) = &f.calc_refusal {
            return unsupported(reason.clone());
        }
        // Corpus record/TestCalcFieldsFieldClass.Codeunit.al:124.
        let Some(calc) = &f.calc else {
            return error(format!(
                "You must define a CalcFormula for the {} FlowField in the {tcap} table.",
                f.name
            ));
        };
        let src = &engine.program.objects[calc.table].table.as_ref().expect("table").fields;
        match calc.method {
            ir::CalcMethod::Sum | ir::CalcMethod::Average => {
                let s = &src[calc.field.expect("sum and average have a field")];
                if !matches!(
                    s.ty,
                    RtType::Decimal | RtType::Integer | RtType::BigInteger | RtType::Duration
                ) {
                    return unsupported(format!("sum or average over a {} field", type_name(&s.ty)));
                }
                // Corpus record/TestCalcFormulaValidationTests.Codeunit.al:96-130: the FlowField
                // and its source must have the very same type, refused when calculated.
                // Gate FlowFieldSumIntegerIntoBigInteger (BC 29, same file :205,227): a sum of an
                // Integer source into a BigInteger FlowField is accepted.
                let relaxed = self.has(al_version::Gate::FlowFieldSumIntegerIntoBigInteger);
                let integer_pair = |a: &RtType, b: &RtType| {
                    matches!(
                        (a, b),
                        (RtType::BigInteger, RtType::Integer) | (RtType::Integer, RtType::BigInteger)
                    )
                };
                if std::mem::discriminant(&f.ty) != std::mem::discriminant(&s.ty)
                    // BC 29 accepts the Integer/BigInteger pair in both directions, for a sum and an average
                    // (record/TestProbeCalcFieldsContracts.al:CalcFields_BigIntegerIntoInteger, BC29PLUS).
                    && !(relaxed && integer_pair(&f.ty, &s.ty))
                {
                    let stcap = self.table_caption(calc.table);
                    // Layout and code: corpus record/TestCalcFieldsTruncateUniqueContracts.al:
                    // CalcFields_TypeMismatch_FullTextAndCode.
                    return error_code(
                        format!(
                            "The following fields must have the same type:{BR}{BR}Field: {} <-- {}{BR}Table: {tcap} <-- {stcap}{BR}Type: {} <-- {}",
                            f.name,
                            s.name,
                            type_name(&f.ty),
                            type_name(&s.ty)
                        ),
                        "DB:TypeMisMatch",
                    );
                }
            }
            ir::CalcMethod::Min | ir::CalcMethod::Max | ir::CalcMethod::Lookup => {
                let s = &src[calc.field.expect("has a field")];
                // Gate FlowFieldMinMaxOverRowVersionRefused (BC 29): the aggregate comes back from
                // SQL as a timestamp and BC refuses it. lookup() still calculates. Corpus
                // record/TestCalcFormulaSystemFieldsTests.Codeunit.al:298-318,353-361 pin the
                // sentence and the Field, Type and SQL type fragments; the whole text, its backslash line
                // breaks and the code are pinned by calcfields/W4CalcFields.al:134-135 and
                // misc/lt-probes-needed-1/PnACalcFieldsBC29.al:225-243.
                if calc.method != ir::CalcMethod::Lookup
                    && s.id == SYSTEM_ROW_VERSION
                    && self.has(al_version::Gate::FlowFieldMinMaxOverRowVersionRefused)
                {
                    return error_code(
                        format!(
                            "Table content in the {tcap} table on SQL Server contains a value that is not compatible with the corresponding data type that is defined for the table.{BR}{BR}Field: {}{BR}Type: {}{BR}SQL type: timestamp{BR}{BR}The table design may have been changed outside Business Central.",
                            f.name,
                            type_name(&f.ty)
                        ),
                        "NDBCS:IncompatibleSqlType",
                    );
                }
                // A lookup into an Enum field from the field of another enum is allowed (record/lt-flowfield-lookup-enum/
                // FlowFieldLookupOtherEnum.al; Microsoft's VAT Return Period).
                let enum_pair = matches!((&f.ty, &s.ty), (RtType::Enum(_), RtType::Enum(_)));
                // A lookup or a max of an Enum field into an Option FlowField (al-runner issue 6481; corpus writer Y20,
                // record/lt-flowfield-lookup-enum/Y20FlowFieldLookupEnumIntoOption.al). A min is not pinned.
                let enum_into_option = matches!((&f.ty, &s.ty), (RtType::Option(_), RtType::Enum(_)))
                    && calc.method != ir::CalcMethod::Min;
                if !same_kind(&f.ty, &s.ty)
                    && !(enum_pair && calc.method == ir::CalcMethod::Lookup)
                    && !enum_into_option
                {
                    return unsupported("min, max or lookup into a field of another type");
                }
            }
            ir::CalcMethod::Count => {
                if !matches!(f.ty, RtType::Integer) {
                    return unsupported("count into a field that is not an Integer");
                }
                // Gate FlowFieldCountFilterBigIntegerFromIntegerFrom29: before BC 29 a filter that links a BigInteger
                // source field to an Integer field (`Big = field(Id)`) is refused when calculated, source first
                // (calcfields/lt-bm-flowfield-conversions/BmFlowFieldConversions.al:166-186). Only this pair and only
                // a count are pinned.
                if !self.has(al_version::Gate::FlowFieldCountFilterBigIntegerFromIntegerFrom29) {
                    for c in &calc.conds {
                        let ir::CalcValue::Field(h) = c.value else { continue };
                        let (sf, hf) = (&src[c.field], &schema.fields[h]);
                        if matches!((&sf.ty, &hf.ty), (RtType::BigInteger, RtType::Integer)) {
                            let stcap = self.table_caption(calc.table);
                            return error_code(
                                format!(
                                    "The following fields must have the same type:{BR}{BR}Field: {} <-- {}{BR}Table: {stcap} <-- {tcap}{BR}Type: {} <-- {}",
                                    sf.name,
                                    hf.name,
                                    type_name(&sf.ty),
                                    type_name(&hf.ty)
                                ),
                                "DB:TypeMisMatch",
                            );
                        }
                    }
                }
            }
            ir::CalcMethod::Exist => {
                if !matches!(f.ty, RtType::Boolean) {
                    return unsupported("exist into a field that is not a Boolean");
                }
            }
        }
        Ok(())
    }

    fn table_caption(&self, table: usize) -> String {
        let t = self.engine.tables[table].clone().expect("table");
        self.caption(&t)
    }

    fn is_blob_field(&self, table: usize, p: usize) -> bool {
        let f = &self.engine.program.objects[table].table.as_ref().expect("table").fields[p];
        f.stored && f.ty == RtType::Library("Blob".into())
    }

    /// `CalcFields`: every named field is validated before any is calculated,
    /// and a refused call changes nothing (corpus
    /// record/TestCalcFormulaValidationTests.Codeunit.al:211,
    /// record/TestCalcFieldsFieldClass.Codeunit.al:204).
    pub(crate) fn calc_fields(&mut self, rec: &Rec, all: &[usize]) -> Result<(), Signal> {
        let table = rec.borrow().table as usize;
        // Blob fields are not FlowFields: CalcFields loads their content (streams/TestBlob.al).
        let is_blob = |p: usize| {
            let f = &self.engine.program.objects[table].table.as_ref().expect("table").fields[p];
            f.stored && f.ty == RtType::Library("Blob".into())
        };
        let (blobs, positions): (Vec<usize>, Vec<usize>) = all.iter().partition(|p| is_blob(**p));
        let positions = &positions[..];
        for &p in positions {
            self.check_calc_field(table, p)?;
        }
        self.load_blobs(rec, &blobs)?;
        let mut out = Vec::new();
        for &p in positions {
            out.push((p, self.calc_value(rec, p, &mut Vec::new())?));
        }
        // A FlowField on a temporary record reads the persistent source table (corpus
        // record/TestCalcFieldsTruncateUniqueContracts.al:CalcFields_OnATemporaryRecord_ReadsThePersistentSourceTable).
        let mut r = rec.borrow_mut();
        for (p, v) in out {
            r.fields[p] = v;
        }
        Ok(())
    }

    /// `Validate` on the FlowField at `pos` (record/lt-flowfield-write/DiFwWriteTests.al). A Count, Exist or Lookup FlowField
    /// raises FlowFieldValidation before OnValidate runs (DiFwWriteTests.al:284, :439, :693-694). A Decimal Sum FlowField
    /// inserts the difference between `value` and the current sum into the source table, then holds `value`
    /// (DiFwWriteTests.al:812-819, :293). Every other shape is refused: it is not pinned.
    pub(crate) fn validate_flowfield(
        &mut self,
        rec: &Rec,
        t: &TableDef,
        pos: usize,
        value: Option<&Value>,
    ) -> Result<(), Signal> {
        let engine = self.engine.clone();
        let table = t.object as usize;
        let schema = engine.program.objects[table].table.as_ref().expect("table");
        let f = &schema.fields[pos];
        let Some(calc) = f.calc.as_ref() else {
            return unsupported(format!(
                "Validate on the FlowField {} without a formula the runtime calculates",
                f.name
            ));
        };
        use ir::CalcMethod as M;
        let decimal_sum = calc.method == M::Sum && f.ty == RtType::Decimal;
        let typed_error = matches!(
            (calc.method, &f.ty),
            (M::Count, RtType::Integer) | (M::Exist, RtType::Boolean) | (M::Lookup, RtType::Text(_) | RtType::Code(_))
        );
        if !decimal_sum && !typed_error {
            return unsupported(format!(
                "Validate on the FlowField {} of type {} with this CalcFormula method (not pinned)",
                f.name,
                f.ty.describe()
            ));
        }
        if typed_error {
            // The text names the field and the table; whether by name or caption is not pinned where they differ.
            let (field, caption) = (f.name.clone(), self.field_caption(t, pos));
            let table_caption = self.caption(t);
            if field != caption || t.name != table_caption {
                return unsupported("the FlowFieldValidation error of a field or table with a caption (not pinned)");
            }
            return Err(crate::transactions::al_error_code(
                &format!(
                    "The type of the {field} FlowField in the {} table must be Decimal.",
                    t.name
                ),
                "FlowFieldValidation",
            ));
        }
        // The one-argument Validate takes the value the variable holds: 0 without CalcFields, the sum after it, or an assigned
        // value (DiFwWriteTests.al:860-926, QB_ValidateOneArgFlowFieldWithOnValidate).
        let held = rec.borrow().fields[pos].clone();
        let new = ops::convert(value.cloned().unwrap_or(held), &f.ty)?;
        let refuse = |why: &str| unsupported(format!("Validate on the Sum FlowField {}: {why} (not pinned)", f.name));
        if calc.negate {
            return refuse("a negated sum");
        }
        if rec.borrow().temporary || rec.borrow().company.is_some() {
            return refuse("a temporary record or another company");
        }
        if f.relation {
            return refuse("a FlowField with a relation");
        }
        let src_t = self.engine.tables[calc.table].clone().expect("table");
        if src_t.keys[0].len() != 1 || src_t.fields[src_t.keys[0][0]].ty != RtType::Integer {
            return refuse("a source table whose key is not one Integer field");
        }
        let _ = new;
        Ok(())
    }

    /// After the OnValidate of a Decimal Sum FlowField (`validate_flowfield` checked it): inserts the difference between
    /// `target` and the current sum into the source table, with Insert(true): the source table's OnInsert runs after the
    /// FlowField's OnValidate (pages/lt-cf-page-leads/CfPageLeads.al:EditableFlowField_SetValue, `Amount.OnValidate:50/30,Entry.OnInsert`).
    pub(crate) fn write_sum_difference(
        &mut self,
        rec: &Rec,
        t: &TableDef,
        pos: usize,
        target: Value,
    ) -> Result<(), Signal> {
        let engine = self.engine.clone();
        let table = t.object as usize;
        let schema = engine.program.objects[table].table.as_ref().expect("table");
        let f = &schema.fields[pos];
        let calc = f.calc.as_ref().expect("checked by validate_flowfield");
        let refuse = |why: &str| unsupported(format!("Validate on the Sum FlowField {}: {why} (not pinned)", f.name));
        let src_t = self.engine.tables[calc.table].clone().expect("table");
        let new = target;
        // The values of the new row: the where-fields that name a normal field of the record take its value; one that names a
        // FlowFilter takes the filter's value or the start of its range, and nothing when no filter is set (DiFwWriteTests.al:812, :818).
        let mut sets: Vec<(usize, Value)> = Vec::new();
        for c in &calc.conds {
            let sty = &src_t.fields[c.field].ty;
            let ir::CalcValue::Field(q) = &c.value else {
                return refuse("a where-condition other than field()");
            };
            let pf = &schema.fields[*q];
            if pf.flow_filter {
                let filter = rec
                    .borrow()
                    .filters
                    .iter()
                    .filter(|x| x.field == *q)
                    .map(|x| x.expr.clone())
                    .collect::<Vec<_>>();
                match filter.as_slice() {
                    [] => {}
                    [FilterExpr::Eq(v)] | [FilterExpr::Range(Some(v), Some(_))] => {
                        sets.push((c.field, ops::convert(v.clone(), sty)?))
                    }
                    _ => return refuse("a FlowFilter that is not one value or a closed range"),
                }
            } else if pf.stored {
                sets.push((c.field, ops::convert(rec.borrow().fields[*q].clone(), sty)?));
            } else {
                return refuse("a where-condition on a FlowField");
            }
        }
        let current = {
            self.check_calc_field(table, pos)?;
            self.calc_value(rec, pos, &mut Vec::new())?
        };
        let (Value::Decimal(cur), Value::Decimal(target)) = (&current, &new) else {
            return refuse("a sum that is not a Decimal");
        };
        let diff = *target - *cur;
        // No row when the value is the sum already (DiFwWriteTests.al:816 `p1 same src`).
        if !diff.is_zero() {
            let Some(at) = calc.field else {
                return refuse("a sum without a field");
            };
            let cx = self.cx_for(rec, &src_t)?;
            let key = src_t.keys[0][0];
            let last = self
                .store
                .next(
                    &cx,
                    &src_t,
                    &Scan {
                        key: 0,
                        filters: &[],
                        ascending: false,
                        after: None,
                    },
                )
                .map_err(|e| Signal::Unsupported(format!("store error {e}")))?;
            // The next entry number after the last row (DiFwWriteTests.al:812 `101` after `100`); an empty table is not pinned.
            let Some(Value::Integer(last)) = last.map(|r| r[key].clone()) else {
                return refuse("an empty source table");
            };
            let Value::Record(row) = self.default_value(&RtType::Record(calc.table as u32, false)) else {
                return refuse("a source record");
            };
            {
                let mut r = row.borrow_mut();
                r.fields[key] = Value::Integer(last + 1);
                for (p, v) in sets {
                    r.fields[p] = v;
                }
                r.fields[at] = ops::convert(Value::Decimal(diff), &src_t.fields[at].ty)?;
            }
            self.record_op(RecordOp::Insert, &row, &mut [Value::Boolean(true)])?;
        }
        rec.borrow_mut().fields[pos] = new;
        Ok(())
    }

    /// Calculates the FlowField at `pos` of `rec` from the table as it is now.
    /// Whether the source table holds a row for the FlowField of `rec`, under the formula's filters. A sum, average, min or max
    /// over no row is NULL in SQL and `CalcFields` shows 0; a query that averages, takes the minimum or the maximum of such a
    /// column leaves the NULL out (corpus query/lt-query-flowfield-methods/QueryFlowFieldAvgMinMax.al:
    /// Average_NoFlowFilter_GroupedByGroupCode `B|10`, Min_NoFlowFilter_GroupedByGroupCode, Min_FlowFilterRange_GroupedByGroupCode).
    pub(crate) fn calc_has_source_rows(&mut self, rec: &Rec, pos: usize) -> Result<bool, Signal> {
        let engine = self.engine.clone();
        let table = rec.borrow().table as usize;
        let f = &engine.program.objects[table].table.as_ref().expect("table").fields[pos];
        let Some(calc) = f.calc.as_ref() else { return Ok(true) };
        if !matches!(
            calc.method,
            ir::CalcMethod::Sum | ir::CalcMethod::Average | ir::CalcMethod::Min | ir::CalcMethod::Max
        ) {
            return Ok(true);
        }
        let filters = self.calc_filters(rec, calc, &mut Vec::new())?;
        let src_t = self.engine.tables[calc.table].clone().expect("table");
        if self.permissions.is_some() {
            self.permission_gate_flowfield(calc.table)?;
        }
        let cx = self.cx_for(rec, &src_t)?;
        let n = self
            .store
            .count(&cx, &src_t, &filters)
            .map_err(|e| Signal::Unsupported(format!("store error {e}")))?;
        Ok(n > 0)
    }

    pub(crate) fn calc_value(&mut self, rec: &Rec, pos: usize, stack: &mut Vec<usize>) -> Result<Value, Signal> {
        // Corpus record/TestCalcFormulaFlowFieldValueTests.Codeunit.al:176-221: a formula
        // that needs itself, directly or through another FlowField, is a recursion error.
        if stack.contains(&pos) {
            // Real BC runs out of stack: record/TestCalcFieldsTruncateUniqueContracts.al:
            // CalcFields_SelfReferencingFormula_FullTextAndCode.
            return error_code(
                "There is insufficient memory to execute this function. This can be caused by recursive function calls. Contact your system administrator.".into(),
                "StackOverflow",
            );
        }
        stack.push(pos);
        let r = self.calc_value_inner(rec, pos, stack);
        stack.pop();
        r
    }

    fn calc_value_inner(&mut self, rec: &Rec, pos: usize, stack: &mut Vec<usize>) -> Result<Value, Signal> {
        let engine = self.engine.clone();
        let table = rec.borrow().table as usize;
        let f = &engine.program.objects[table].table.as_ref().expect("table").fields[pos];
        let calc = f.calc.as_ref().expect("checked by check_calc_field");
        let filters = self.calc_filters(rec, calc, stack)?;
        let src_t = self.engine.tables[calc.table].clone().expect("table");
        self.unseeded_read(&src_t, rec.borrow().temporary, "a FlowField")?;
        // ADR-062: a FlowField reads its source table indirectly (CpPermLeads.al:29-63).
        if self.permissions.is_some() {
            self.permission_gate_flowfield(calc.table)?;
        }
        let cx = self.cx_for(rec, &src_t)?;
        let store_err = |e: al_data::StoreError| Signal::Unsupported(format!("store error {e}"));
        use ir::CalcMethod as M;
        // A condition on a FlowField of the source table compares each source row's calculated value: count, exist and
        // lookup are pinned (record/lt-calcformula-date-literal/CalcFormulaDateLiteral.al:76-95, :243-280
        // EqualsEmptyDate_Chained_GroupOverFlowField: filter(true) on an exist, filter(0) and filter(> 0) on a count), and so
        // are sum, -sum, average, min and max over the rows that pass (record/lt-y17-sum-where-flowfield/
        // Y17SumWhereFlowField.al:197, :227; a condition FlowField with an unset FlowFilter in its own formula filters
        // nothing and a condition on a sum FlowField works the same, :303). A sign on another method is not pinned.
        let src_fields = &engine.program.objects[calc.table].table.as_ref().expect("table").fields;
        let (stored, flow): (Vec<FieldFilter>, Vec<FieldFilter>) = filters
            .iter()
            .cloned()
            .partition(|c| src_fields[c.field].stored || src_fields[c.field].flow_filter);
        if !flow.is_empty() {
            if calc.negate && calc.method != M::Sum {
                return unsupported(
                    "a signed CalcFormula other than sum with a condition on a FlowField of its source table (not pinned)",
                );
            }
            let mut rows = Vec::new();
            self.each_row(&cx, &src_t, &stored, |row| {
                rows.push(row.to_vec());
                Ok(())
            })?;
            let mut matched: Vec<Vec<Value>> = Vec::new();
            for row in rows {
                let Value::Record(probe) = self.default_value(&RtType::Record(calc.table as u32, false)) else {
                    return unsupported("a source record of a FlowField");
                };
                {
                    let mut p = probe.borrow_mut();
                    p.fields = row.clone();
                    p.company = rec.borrow().company.clone();
                }
                let mut pass = true;
                for c in &flow {
                    self.check_calc_field(calc.table, c.field)?;
                    let v = self.calc_value(&probe, c.field, &mut Vec::new())?;
                    if !c.expr.matches(&v) {
                        pass = false;
                        break;
                    }
                }
                if pass {
                    matched.push(row);
                    if matches!(calc.method, M::Exist | M::Lookup) {
                        break;
                    }
                }
            }
            let value = match calc.method {
                M::Count => Value::Integer(matched.len() as i32),
                M::Exist => Value::Boolean(!matched.is_empty()),
                M::Lookup => match matched.first() {
                    Some(row) => row[calc.field.expect("field")].clone(),
                    None => self.default_value(&f.ty),
                },
                M::Sum | M::Average => {
                    let at = calc.field.expect("field");
                    let mut sum = Decimal::ZERO;
                    for row in &matched {
                        sum += number(&row[at]).ok_or_else(|| non_number(&row[at]))?;
                    }
                    // Over no row both are 0 (Y17SumWhereFlowField.al:227, header C).
                    let mut d = match (calc.method, matched.len()) {
                        (M::Sum, _) | (_, 0) => sum,
                        (_, n) => sum / Decimal::from(n as i64),
                    };
                    if calc.negate {
                        d = -d;
                    }
                    return to_number(d, &f.ty, calc.method == M::Average);
                }
                M::Min | M::Max => {
                    let at = calc.field.expect("field");
                    let mut best: Option<Value> = None;
                    for row in &matched {
                        let v = &row[at];
                        best = match best.take() {
                            None => Some(v.clone()),
                            Some(b) => match (calc.method, ops::compare(v, &b)?) {
                                (M::Min, Ordering::Less) | (M::Max, Ordering::Greater) => Some(v.clone()),
                                (_, _) => Some(b),
                            },
                        };
                    }
                    match best {
                        Some(v) => v,
                        None => self.default_value(&f.ty),
                    }
                }
            };
            return ops::convert(value, &f.ty);
        }
        let value = match calc.method {
            M::Count => {
                let n = self.store.count(&cx, &src_t, &filters).map_err(store_err)? as i32;
                Value::Integer(if calc.negate { -n } else { n })
            }
            M::Exist => {
                let found = self
                    .store
                    .next(
                        &cx,
                        &src_t,
                        &Scan {
                            key: 0,
                            filters: &filters,
                            ascending: true,
                            after: None,
                        },
                    )
                    .map_err(store_err)?
                    .is_some();
                // A leading '-' on an exist is a logical NOT (corpus
                // record/TestCalcFormulaSignFiltersTests.Codeunit.al:196).
                Value::Boolean(found != calc.negate)
            }
            M::Sum | M::Average => {
                let at = calc.field.expect("field");
                let (mut sum, mut n) = (Decimal::ZERO, 0i64);
                self.each_row(&cx, &src_t, &filters, |row| {
                    sum += number(&row[at]).ok_or_else(|| non_number(&row[at]))?;
                    n += 1;
                    Ok(())
                })?;
                let mut d = if calc.method == M::Sum {
                    sum
                } else if n == 0 {
                    Decimal::ZERO
                } else {
                    sum / Decimal::from(n)
                };
                if calc.negate {
                    d = -d;
                }
                // Corpus record/TestCalcFormulaMinMaxAvgTests.Codeunit.al:115: an Integer
                // average of 5.25 or 0.25 is stored as 5 and 0; only fractions below one
                // half are pinned (truncation and rounding agree there).
                to_number(d, &f.ty, calc.method == M::Average)?
            }
            M::Min | M::Max | M::Lookup => {
                let at = calc.field.expect("field");
                if calc.negate {
                    return unsupported("a sign on min, max or lookup");
                }
                // Lookup reads the first matching row in primary key order (corpus
                // record/TestCalcFieldsTruncateUniqueContracts.al:
                // CalcFields_Lookup_OverSeveralRows_ReadsTheFirstInPrimaryKeyOrder), so it
                // needs one row. When the primary key already orders the rows by the field, so
                // do min (the first row) and max (the last row): no need to walk millions of
                // rows of a computed table such as Date
                // (corpus record/TestDateVirtualTableFlowField.al:94-110).
                let first_only = calc.method == M::Lookup;
                let ends = !first_only && key_ordered_by(&src_t, &filters, at);
                let mut best: Option<Value> = None;
                if first_only || ends {
                    let ascending = calc.method != M::Max;
                    let row = self
                        .store
                        .next(
                            &cx,
                            &src_t,
                            &Scan {
                                key: 0,
                                filters: &filters,
                                ascending,
                                after: None,
                            },
                        )
                        .map_err(store_err)?;
                    best = row.map(|r| r[at].clone());
                } else {
                    self.each_row(&cx, &src_t, &filters, |row| {
                        let v = &row[at];
                        best = match best.take() {
                            None => Some(v.clone()),
                            Some(b) => match (calc.method, ops::compare(v, &b)?) {
                                (M::Min, Ordering::Less) | (M::Max, Ordering::Greater) => Some(v.clone()),
                                (_, _) => Some(b),
                            },
                        };
                        Ok(())
                    })?;
                }
                // Before BC 29 a max of an Enum field into an Option FlowField keeps the Enum member, and no row gives the
                // Enum's first member; from BC 29 it is an Option ordinal (Y20FlowFieldLookupEnumIntoOption.al:189-198,
                // `#if BC29PLUS`; al-runner issue 6481).
                let src_ty = self.engine.program.objects[calc.table].table.as_ref().expect("table").fields[at].ty.clone();
                if calc.method == M::Max
                    && matches!((&f.ty, &src_ty), (RtType::Option(_), RtType::Enum(_)))
                    && !self.has(al_version::Gate::ProbedValuesDifferFrom29)
                {
                    return Ok(match best {
                        Some(v) => v,
                        None => self.default_value(&src_ty),
                    });
                }
                // Nothing matched: the type default (corpus
                // record/TestCalcFormulaMinMaxAvgTests.Codeunit.al:250).
                match best {
                    Some(v) => v,
                    None => self.default_value(&f.ty),
                }
            }
        };
        // A lookup into an Enum field of another enum keeps the looked-up member as it is: `Zero` stays `Zero` and `Five`
        // (an ordinal the FlowField's enum lacks) stays `Five` (corpus record/lt-flowfield-lookup-enum/
        // FlowFieldLookupOtherEnum.al: Lookup_MembersPresentInBoth_Result, Lookup_MemberOnlyInSource_Result).
        if matches!((&value, &f.ty), (Value::Enum(from, _), RtType::Enum(to)) if from != to) {
            return Ok(value);
        }
        // A lookup of an Enum field into an Option FlowField keeps the Enum member too: Format shows the Enum's name, also for an
        // ordinal the Option lacks (Y20FlowFieldLookupEnumIntoOption.al:115, :136; al-runner issue 6481).
        if calc.method == ir::CalcMethod::Lookup && matches!((&value, &f.ty), (Value::Enum(..), RtType::Option(_))) {
            return Ok(value);
        }
        // A value longer than the FlowField: a lookup of a Text source into a shorter Text FlowField keeps the whole value,
        // with no error and no truncation (calcfields/lt-bm-flowfield-conversions/BmFlowFieldConversions.al:201-217,
        // BM_LookupFlowField_ShorterThanSource `short_30_chars`, fields at :50-58). Any other pair (Code, min or max) is
        // not pinned.
        match ops::convert(value.clone(), &f.ty) {
            Err(Signal::Error(e)) if e.code == "StringLengthExceeded" => {
                let source_text = calc.field.is_some_and(|at| {
                    matches!(
                        self.engine.program.objects[calc.table]
                            .table
                            .as_ref()
                            .expect("table")
                            .fields[at]
                            .ty,
                        RtType::Text(_)
                    )
                });
                if calc.method == ir::CalcMethod::Lookup && matches!(f.ty, RtType::Text(_)) && source_text {
                    return Ok(value);
                }
                unsupported(
                    "a min, max or lookup FlowField value longer than the field (only a Text lookup into Text is pinned)",
                )
            }
            other => other,
        }
    }

    /// The source-table filters of a formula's where-conditions for `rec`.
    fn calc_filters(
        &mut self,
        rec: &Rec,
        calc: &ir::CalcFormula,
        stack: &mut Vec<usize>,
    ) -> Result<Vec<FieldFilter>, Signal> {
        for a in &calc.assumes {
            al_builtins::assumptions::assume(*a);
        }
        let engine = self.engine.clone();
        let table = rec.borrow().table as usize;
        let own = &engine.program.objects[table].table.as_ref().expect("table").fields;
        let src = &engine.program.objects[calc.table].table.as_ref().expect("table").fields;
        let enums = |id: u32| crate::views::enum_filter_members(&engine.program.objects[id as usize]);
        let parse = |text: &str, ty: &RtType| {
            parse_filter_with(text, &[], ty, &enums)
                .map_err(|e| Signal::Unsupported(format!("CalcFormula filter: {e}")))
        };
        // The flow filter the caller set on field `q`, if any.
        let flow = |q: usize| {
            rec.borrow()
                .filters
                .iter()
                .find(|x| x.field == q)
                .map(|x| x.expr.clone())
        };
        // (source field, filter, replaces other conditions on that field)
        let mut parts: Vec<(usize, FilterExpr, bool)> = Vec::new();
        for c in &calc.conds {
            let sty = &src[c.field].ty;
            match &c.value {
                ir::CalcValue::Const(k) => {
                    parts.push((c.field, FilterExpr::Eq(ops::convert(crate::constant(k), sty)?), false));
                }
                ir::CalcValue::Filter(text) => parts.push((c.field, parse(text, sty)?, false)),
                ir::CalcValue::Field(q) => {
                    let pf = &own[*q];
                    if pf.flow_filter {
                        // Corpus record/TestCalcFormulaFlowFiltersTests.Codeunit.al:107: the
                        // caller's whole filter applies; none set means no constraint.
                        if let Some(e) = flow(*q) {
                            if !same_kind(&pf.ty, sty) {
                                return unsupported("a flow filter of another type than the source field");
                            }
                            parts.push((c.field, e, false));
                        }
                    } else if !pf.stored {
                        // Corpus record/TestCalcFormulaFlowFieldValueTests.Codeunit.al:112: a
                        // FlowField on the other side is calculated first, not read from the buffer.
                        self.check_calc_field(table, *q)?;
                        let v = self.calc_value(rec, *q, stack)?;
                        parts.push((c.field, FilterExpr::Eq(ops::convert(v, sty)?), false));
                    } else {
                        let v = rec.borrow().fields[*q].clone();
                        parts.push((c.field, FilterExpr::Eq(ops::convert(v, sty)?), false));
                    }
                }
                ir::CalcValue::UpperLimit(q) => {
                    if !own[*q].flow_filter {
                        return unsupported("upperlimit() of a field that is not a flow filter");
                    }
                    // Corpus record/TestCalcFormulaFlowFiltersTests.Codeunit.al:144: only the
                    // upper bound of the caller's range; none set means none.
                    match flow(*q) {
                        None => {}
                        Some(FilterExpr::Range(_, Some(hi))) => {
                            parts.push((c.field, FilterExpr::Range(None, Some(hi)), false));
                        }
                        // A filter of one value is its own upper limit, and so is `<=` a value (UpperLimitSingleValue.al:
                        // UpperLimit_OneValue `setfilter_format_d`, `setfilter_le_d`).
                        Some(FilterExpr::Eq(hi) | FilterExpr::Le(hi)) => {
                            parts.push((c.field, FilterExpr::Range(None, Some(hi)), false));
                        }
                        // An open end, `<`, `>`, `<>` and several values are more than one range (UpperLimitSingleValue.al:
                        // UpperLimit_OneValue `setfilter_lt_d`, UpperLimit_OpenAndEmpty). The code is DB:NotIntervalFilter
                        // (calcfields/lt-upperlimit-error-code/UpperLimitErrorCode.al:107-111,131).
                        Some(
                            e @ (FilterExpr::Range(Some(_), None)
                            | FilterExpr::Lt(_)
                            | FilterExpr::Gt(_)
                            | FilterExpr::Ne(_)
                            | FilterExpr::Or(_)),
                        ) => {
                            let text = self.filter_text(&e)?;
                            let t = self.table(rec)?;
                            return Err(crate::transactions::al_error_code(
                                &format!(
                                    "The filter expression \"{text}\" applied to the field {} in the {} table results in more than one range.",
                                    self.field_caption(&t, *q),
                                    self.caption(&t)
                                ),
                                "DB:NotIntervalFilter",
                            ));
                        }
                        Some(_) => return unsupported("upperlimit() of a filter that is not a range"),
                    }
                }
                ir::CalcValue::FieldFilter(q) => {
                    let v = rec.borrow().fields[*q].clone();
                    let Some(text) = text_of(&v) else {
                        return unsupported("field(filter()) of a field that is not text");
                    };
                    // Corpus record/TestCalcFormulaFlowFiltersTests.Codeunit.al:194-230: the value is
                    // read as a filter expression, blank imposes nothing, and a filter takes over
                    // from a link on the same source field.
                    if !text.trim().is_empty() {
                        parts.push((c.field, parse(&text, sty)?, true));
                    }
                }
            }
        }
        let overridden: Vec<usize> = parts.iter().filter(|p| p.2).map(|p| p.0).collect();
        Ok(parts
            .into_iter()
            .filter(|(field, _, is_override)| *is_override || !overridden.contains(field))
            .map(|(field, expr, _)| FieldFilter { field, expr, group: 0 })
            .collect())
    }

    /// Calls `f` on every row of `t` passing `filters`, in primary key order.
    pub(crate) fn each_row(
        &mut self,
        cx: &al_data::AccessContext,
        t: &TableDef,
        filters: &[FieldFilter],
        f: impl FnMut(&[Value]) -> Result<(), Signal>,
    ) -> Result<(), Signal> {
        self.each_row_of(None, cx, t, filters, f)
    }

    /// `each_row` over the rows `rec` reads: a temporary record has a table of its own (`store_for`), so its sums
    /// are over those rows and not over the permanent table of the same number.
    pub(crate) fn each_row_of(
        &mut self,
        rec: Option<&Rec>,
        cx: &al_data::AccessContext,
        t: &TableDef,
        filters: &[FieldFilter],
        mut f: impl FnMut(&[Value]) -> Result<(), Signal>,
    ) -> Result<(), Signal> {
        let mut after: Option<Vec<Value>> = None;
        loop {
            let store = match rec {
                Some(r) => self.store_for(r),
                None => &mut self.store,
            };
            let next = store
                .next(
                    cx,
                    t,
                    &Scan {
                        key: 0,
                        filters,
                        ascending: true,
                        after: after.as_deref(),
                    },
                )
                .map_err(|e| Signal::Unsupported(format!("store error {e}")))?;
            let Some(row) = next else { return Ok(()) };
            f(&row)?;
            after = Some(position(t, 0, &row));
        }
    }

    /// `CalcSums`: the sum of stored number fields over the record's own
    /// filters (corpus record/TestRecordFlowField.al:109-160).
    ///
    /// `duration` allows a Duration field: only `FieldRef.CalcSum` pins it (FieldRefCalcSumTypes), `Record.CalcSums` of
    /// a Duration is refused until the corpus pins it (issue 630 item 3).
    pub(crate) fn calc_sums(&mut self, rec: &Rec, positions: &[usize], duration: bool) -> Result<(), Signal> {
        let engine = self.engine.clone();
        let table = rec.borrow().table as usize;
        let t = self.engine.tables[table].clone().expect("table");
        let fields = &engine.program.objects[table].table.as_ref().expect("table").fields;
        for &p in positions {
            let f = &fields[p];
            if !f.stored
                || !matches!(f.ty, RtType::Decimal | RtType::Integer | RtType::BigInteger)
                    && !(duration && f.ty == RtType::Duration)
            {
                return unsupported(format!(
                    "CalcSums on {} (only stored number fields are implemented)",
                    f.name
                ));
            }
        }
        let (filters, flow_field_filters) = self.split_filters(rec);
        let mut sums = vec![Decimal::ZERO; positions.len()];
        let cx = self.cx_for(rec, &t)?;
        let mut add = |row: &[Value]| -> Result<(), Signal> {
            for (s, &p) in sums.iter_mut().zip(positions) {
                *s += number(&row[p]).ok_or_else(|| non_number(&row[p]))?;
            }
            Ok(())
        };
        if flow_field_filters.is_empty() {
            self.each_row_of(Some(rec), &cx, &t, &filters, &mut add)?;
        } else {
            // The sum is over the rows whose FlowField value passes the filter (record/lt-calcsums-flowfield-filter/CalcSumsFlowFieldFilter.al:
            // CalcSums_FilterFlowFieldNotZero_Result, CalcSums_FlowFieldRangeAndStoredFilter_Result, CalcSums_SetRangeFlowFieldZero_Result).
            let mut rows = Vec::new();
            self.each_row_of(Some(rec), &cx, &t, &filters, |row| {
                rows.push(row.to_vec());
                Ok(())
            })?;
            for row in rows {
                if self.passes_flow_field_filters(rec, &row, &flow_field_filters)? {
                    add(&row)?;
                }
            }
        }
        // The sums land in the record's fields only once all are known.
        let mut out = Vec::new();
        for (s, &p) in sums.into_iter().zip(positions) {
            out.push((p, to_number(s, &fields[p].ty, false)?));
        }
        let mut r = rec.borrow_mut();
        for (p, v) in out {
            r.fields[p] = v;
        }
        Ok(())
    }

    /// Splits a record's filters into those the store can evaluate (stored
    /// fields) and those on FlowFields. Filters on FlowFilter fields are
    /// dropped: they hold no data and never constrain their own table (corpus
    /// record/TestCalcFormulaFlowFiltersTests.Codeunit.al:258).
    pub(crate) fn split_filters(&self, rec: &Rec) -> (Vec<FieldFilter>, Vec<FieldFilter>) {
        let r = rec.borrow();
        let fields = &self.engine.program.objects[r.table as usize]
            .table
            .as_ref()
            .expect("table")
            .fields;
        let (mut stored, mut flow_fields) = (Vec::new(), Vec::new());
        for f in &r.filters {
            let schema = &fields[f.field];
            if schema.stored {
                stored.push(f.clone());
            } else if !schema.flow_filter {
                flow_fields.push(f.clone());
            }
        }
        // A database table compares its Time and DateTime columns with the filter values rounded as the columns hold them
        // (errors/lt-ms-refusals-130/P130DateFilters.al:206-227).
        if !r.temporary {
            if let Some(t) = self.engine.tables[r.table as usize].as_ref() {
                crate::sqltime::round_filters(t, &mut stored);
            }
        }
        (stored, flow_fields)
    }

    /// Whether a row passes the filters set on FlowFields: each is checked
    /// against the field's calculated value for that row (corpus
    /// record/TestRecordFlowFieldFilterVisitor.al:30,49).
    pub(crate) fn passes_flow_field_filters(
        &mut self,
        rec: &Rec,
        row: &[Value],
        filters: &[FieldFilter],
    ) -> Result<bool, Signal> {
        let table = rec.borrow().table as usize;
        let probe = {
            let r = rec.borrow();
            let mut p = r.clone();
            p.fields = row.to_vec();
            p.temporary = false;
            p.auto_calc.clear();
            Rc::new(RefCell::new(p))
        };
        for f in filters {
            self.check_calc_field(table, f.field)?;
            let v = self.calc_value(&probe, f.field, &mut Vec::new())?;
            if !f.expr.matches(&v) {
                return Ok(false);
            }
        }
        Ok(true)
    }

    pub(crate) fn flow_op(&mut self, method: RecordOp, rec: &Rec, args: &[Value]) -> Result<Value, Signal> {
        let positions: Vec<usize> = args
            .iter()
            .map(|a| match a {
                Value::Integer(p) => Ok(*p as usize),
                other => unsupported(format!("field argument {}", other.describe())),
            })
            .collect::<Result<_, _>>()?;
        match method {
            RecordOp::CalcFields => {
                self.calc_fields(rec, &positions)?;
            }
            RecordOp::CalcSums => {
                self.calc_sums(rec, &positions, false)?;
            }
            RecordOp::SetAutoCalcFields => {
                let table = rec.borrow().table as usize;
                for &p in &positions {
                    // A Blob is accepted and loaded by every read (corpus
                    // misc/lt-probes-needed-1/PnABlobLength.al:83 Length_InMemoryAndFindVariants:
                    // `findfirst-autocalc` 5 against `findfirst-unloaded` 11).
                    if self.is_blob_field(table, p) {
                        continue;
                    }
                    self.check_calc_field(table, p)?;
                }
                // A call adds its fields to the set, and a call without fields empties it
                // (record/lt-record-write/RcBFlowFields.al:SetAutoCalcFields_Reset_And_Replace `replaced`, `cleared`).
                let mut r = rec.borrow_mut();
                if positions.is_empty() {
                    r.auto_calc.clear();
                } else {
                    for p in positions {
                        if !r.auto_calc.contains(&p) {
                            r.auto_calc.push(p);
                        }
                    }
                }
            }
            other => return unsupported(format!("Record.{other}")),
        }
        Ok(Value::Boolean(true))
    }

    /// After a row is read into `rec`: the fields of `SetAutoCalcFields`.
    pub(crate) fn auto_calculate(&mut self, rec: &Rec) -> Result<(), Signal> {
        let auto = rec.borrow().auto_calc.clone();
        if auto.is_empty() {
            return Ok(());
        }
        // A temporary record loads an auto-calculated Blob like a stored row (record/lt-temporary-blob/TemporaryBlobAutoCalc.al:
        // Temporary_SetAutoCalcFields_ThenFind).
        self.calc_fields(rec, &auto)
    }
}

/// Why a Sum cannot add a value: a column a platform row leaves unknown, or not a number.
fn non_number(v: &Value) -> Signal {
    Signal::Unsupported(
        v.unknown()
            .map_or_else(|| "sum over a non-number".to_string(), str::to_string),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use al_data::FieldDef;

    fn table(keys: Vec<usize>, types: &[RtType]) -> TableDef {
        TableDef {
            object: 0,
            name: "T".into(),
            fields: types
                .iter()
                .enumerate()
                .map(|(i, ty)| FieldDef {
                    id: i as u32 + 1,
                    name: format!("F{i}"),
                    ty: ty.clone(),
                    stored: true,
                    code_storage: Default::default(),
                })
                .collect(),
            keys: vec![keys],
            per_company: false,
        }
    }

    fn eq(field: usize, v: i32) -> FieldFilter {
        FieldFilter {
            field,
            expr: FilterExpr::Eq(Value::Integer(v)),
            group: 0,
        }
    }

    fn range(field: usize) -> FieldFilter {
        FieldFilter {
            field,
            expr: FilterExpr::Range(Some(Value::Integer(1)), None),
            group: 0,
        }
    }

    #[test]
    fn rows_come_in_the_order_of_the_first_key_field() {
        let t = table(vec![0, 1], &[RtType::Integer, RtType::Date]);
        assert!(key_ordered_by(&t, &[], 0));
        assert!(key_ordered_by(&t, &[range(0)], 0));
    }

    #[test]
    fn a_later_key_field_orders_the_rows_only_when_the_earlier_ones_are_pinned() {
        // Date's key: (Period Type, Period Start). Corpus record/TestDateVirtualTableFlowField.al:140.
        let t = table(vec![0, 1], &[RtType::Integer, RtType::Date]);
        assert!(!key_ordered_by(&t, &[], 1));
        assert!(!key_ordered_by(&t, &[range(0)], 1));
        assert!(key_ordered_by(&t, &[eq(0, 0)], 1));
        // Two filters on the pinned field are not one pin.
        assert!(!key_ordered_by(&t, &[eq(0, 0), range(0)], 1));
        // A filter on the ordered field itself does not matter.
        assert!(key_ordered_by(&t, &[eq(0, 0), range(1)], 1));
    }

    #[test]
    fn only_numbers_and_dates_sort_the_same_as_the_comparison_a_formula_uses() {
        let t = table(vec![0], &[RtType::Code(Some(20)), RtType::Integer]);
        assert!(!key_ordered_by(&t, &[], 0));
        // A field outside the primary key says nothing about the order.
        let t = table(vec![0], &[RtType::Integer, RtType::Integer]);
        assert!(!key_ordered_by(&t, &[], 1));
    }
}
