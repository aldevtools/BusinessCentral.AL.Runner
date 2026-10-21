//! Filter and view text: `GetFilter`, `GetFilters`, `GetView`, `SetView`.
//!
//! The corpus pins how a filter reads back for the simple shapes
//! (`'10..20'`, `'30'`, `'A'`: testfilter/TestTestFilter.al:227-350) and the view
//! grammar through the strings it feeds to `SetView`
//! (`'SORTING(Entry No.) WHERE(Entry No.=FILTER(1..3))'`:
//! recordref/TestRecordRefFilter.al:23). Anything else is refused.

use crate::Session;
use al_builtins::describe::Describe;
use al_builtins::filter::{FieldFilter, FilterExpr};
use al_builtins::native::Signal;
use al_builtins::value::{RecordState, Value};
use al_data::TableDef;
use std::cell::RefCell;
use std::rc::Rc;

type Rec = Rc<RefCell<RecordState>>;

fn unsupported<T>(what: impl Into<String>) -> Result<T, Signal> {
    Err(Signal::Unsupported(what.into()))
}

/// Characters whose presence in a text value makes `GetFilter` quote the whole value (record/
/// TestSetFilterSpecialTextContracts.al, BC 27.5), and `..`.
fn needs_quotes(s: &str) -> bool {
    s.contains(['|', '&', '<', '=', '(', ')']) || s.contains("..")
}

/// Characters in a value `GetFilter` cannot show yet: nothing pins them.
const FILTER_UNPINNED: &[char] = &['>', '*', '?', '"'];

impl Session {
    /// A filter expression over a field of type `ty`, enum members by name included.
    pub(crate) fn parse_filter_text(&self, text: &str, ty: &al_builtins::value::RtType) -> Result<FilterExpr, String> {
        let engine = self.engine.clone();
        let enums = |id: u32| crate::views::enum_filter_members(&engine.program.objects[id as usize]);
        al_builtins::filter::parse_filter_with(text, &[], ty, &enums)
    }

    /// `filter_value`; with `iso` a Date reads `yyyy-mm-dd`, as a view writes it (record/lt-record-read/RcAMisc.al:
    /// GetView_Field_Types `date`).
    fn filter_value_with(&self, v: &Value, iso: bool) -> Result<String, Signal> {
        // A blank date shows as two single quotes, like a blank text (record/lt-record-read/RcAMisc.al:
        // SetRecFilter_Composite_And_Text_Keys `dateBlank`).
        if matches!(v, Value::Date(0, false)) {
            return Ok("''".into());
        }
        // A view writes a Boolean as `true` or `false` (:GetView_Field_Types `bool`).
        if let (true, Value::Boolean(b)) = (iso, v) {
            return Ok(b.to_string());
        }
        if let (true, Value::Date(d, false)) = (iso, v) {
            if *d != 0 {
                let (y, m, day) = al_builtins::format::civil_from_day(*d);
                return Ok(format!("{y:04}-{m:02}-{day:02}"));
            }
        }
        // A date filter writes a two-digit year for 2031 to 2038 too, where Format(Date) writes four digits
        // (record/lt-date-text-forms/DateFilterTextForms.al:GetRange_DateTextForms_English `filter 02/01/31`,
        // GetRange_FourDigits_OtherWorkDate `filter 02/01/38`). The years 2039 to 2049 are not pinned.
        if let Value::Date(d, false) = v {
            let (y, m, day) = al_builtins::format::civil_from_day(*d);
            if (2031..=2038).contains(&y) {
                return Ok(format!("{m:02}/{day:02}/{:02}", y % 100));
            }
            if (2039..=2049).contains(&y) {
                return unsupported("the text of a date filter value in the years 2039 to 2049 (not pinned)");
            }
        }
        if let Value::Text(s) | Value::Code(s) = v {
            // A blank text shows as two single quotes (corpus record/PWFilterGroupOr.al: `A: ''`).
            if s.is_empty() {
                return Ok("''".into());
            }
            if needs_quotes(s) {
                if s.contains(FILTER_UNPINNED) || s.contains('\'') || s.starts_with('@') {
                    return unsupported("a filter value that needs quoting and holds more filter characters");
                }
                return Ok(format!("'{s}'"));
            }
            if s.contains(FILTER_UNPINNED) || s.starts_with('@') {
                return unsupported("a filter value that needs quoting");
            }
        }
        // A Time reads as Format gives it (events/lt-platform-triggers/PtObserve.al:Filter_DateTime_Time `>=12:00:00 PM`, Bad_Time m3);
        // a time with milliseconds is not pinned.
        if let Value::Time(t) = v {
            if *t != 0 && (t - 1) % 1000 == 0 {
                return al_builtins::format::try_format(v, 0, 0, self).map_err(Signal::Unsupported);
            }
            return unsupported("the filter text of an undefined Time or one with milliseconds (not pinned)");
        }
        // A DateTime with milliseconds shows its seconds and milliseconds, as standard format 1 writes them
        // (errors/lt-ms-refusals-130/P130DateFilters.al:211, `03/24/24 11:59:59.999 PM`; a whole minute stays `12:00 AM`).
        if let Value::DateTime(d) = v {
            if *d != 0 && (d - 1) % 1000 != 0 {
                return al_builtins::format::try_format(v, 0, 1, self).map_err(Signal::Unsupported);
            }
            // Seconds without milliseconds: not pinned.
            if *d != 0 && (d - 1) / 1000 % 60 != 0 {
                return unsupported("the filter text of a DateTime with seconds and no milliseconds (not pinned)");
            }
        }
        let text = self.message_value(v)?;
        // A value that is only blank space is shown in single quotes (corpus
        // record/TestRecordRuntimeTextsAndViews.al:Record_GetView_And_GetFilters_Texts, the blank
        // option member: `Status: <>' '`).
        // A value with a leading space too (filters/W4Filters.al:BlankOnlyFilterValues_MatchOnlyBlankRows:
        // `SetRange(Txt, ' a')` shows `Txt: ' a'`).
        if !text.is_empty() && (text.trim().is_empty() || text.starts_with(' ')) {
            return Ok(format!("'{text}'"));
        }
        Ok(text)
    }

    pub(crate) fn filter_text(&self, e: &FilterExpr) -> Result<String, Signal> {
        self.filter_text_with(e, false)
    }

    /// `filter_text`; with `iso` the dates are written as a view writes them.
    pub(crate) fn filter_text_with(&self, e: &FilterExpr, iso: bool) -> Result<String, Signal> {
        Ok(match e {
            FilterExpr::Range(Some(a), Some(b)) => {
                if al_builtins::ops::compare(a, b).ok() == Some(std::cmp::Ordering::Equal) {
                    self.filter_value_with(a, iso)?
                } else {
                    format!(
                        "{}..{}",
                        self.filter_value_with(a, iso)?,
                        self.filter_value_with(b, iso)?
                    )
                }
            }
            FilterExpr::Range(Some(a), None) => format!("{}..", self.filter_value_with(a, iso)?),
            FilterExpr::Range(None, Some(b)) => format!("..{}", self.filter_value_with(b, iso)?),
            FilterExpr::Range(None, None) => return unsupported("an unbounded range filter"),
            FilterExpr::Eq(v) => self.filter_value_with(v, iso)?,
            FilterExpr::RangeSame(v) => {
                let t = self.filter_value_with(v, iso)?;
                format!("{t}..{t}")
            }
            // `<>%1` with the text `*` reads back quoted (record/lt-bl-wildcard-getfilter/WildcardGetFilter.al:196, :199).
            FilterExpr::Ne(Value::Text(s) | Value::Code(s)) if &**s == "*" => "<>'*'".to_string(),
            FilterExpr::Ne(v) => format!("<>{}", self.filter_value_with(v, iso)?),
            FilterExpr::Lt(v) => format!("<{}", self.filter_value_with(v, iso)?),
            FilterExpr::Le(v) => format!("<={}", self.filter_value_with(v, iso)?),
            FilterExpr::Gt(v) => format!(">{}", self.filter_value_with(v, iso)?),
            FilterExpr::Ge(v) => format!(">={}", self.filter_value_with(v, iso)?),
            FilterExpr::Like(p, ci) if needs_quotes(p) && !p.contains(['*', '?', '\'', '>']) => {
                format!("'{}{p}'", if *ci { "@" } else { "" })
            }
            FilterExpr::Like(p, ci) => format!("{}{p}", if *ci { "@" } else { "" }),
            // record/lt-bl-wildcard-getfilter/WildcardGetFilter.al:125-171: `<>*`, `<>A*`, `<>@a*` read back as written.
            FilterExpr::NotLike(p, ci) => format!("<>{}{p}", if *ci { "@" } else { "" }),
            FilterExpr::Or(xs) => xs
                .iter()
                .map(|x| self.filter_text_with(x, iso))
                .collect::<Result<Vec<_>, _>>()?
                .join("|"),
            FilterExpr::And(xs) => {
                if xs.iter().any(|x| matches!(x, FilterExpr::Or(_))) {
                    return unsupported("an AND of OR filters");
                }
                xs.iter()
                    .map(|x| self.filter_text_with(x, iso))
                    .collect::<Result<Vec<_>, _>>()?
                    .join("&")
            }
        })
    }

    /// `e` with the Option values of field `pos` carrying the field's OptionCaption entries: `GetFilter` and `GetFilters`
    /// show the caption (record/TestProbeRecordRefContracts.al:Filters_GetFiltersText, `option_filter` is `Second`).
    pub(crate) fn with_option_captions(&self, t: &TableDef, pos: usize, e: &FilterExpr) -> FilterExpr {
        let Ok(Some((false, members))) = self.members_of_field(t, pos) else {
            return e.clone();
        };
        let captions: std::sync::Arc<[String]> = members.iter().map(|m| m.2.clone()).collect::<Vec<_>>().into();
        let map = |v: &Value| match v {
            Value::Option(o, _) => Value::Option(*o, captions.clone()),
            other => other.clone(),
        };
        match e {
            FilterExpr::Range(a, b) => FilterExpr::Range(a.as_ref().map(map), b.as_ref().map(map)),
            FilterExpr::Eq(v) => FilterExpr::Eq(map(v)),
            FilterExpr::RangeSame(v) => FilterExpr::RangeSame(map(v)),
            FilterExpr::Ne(v) => FilterExpr::Ne(map(v)),
            FilterExpr::Lt(v) => FilterExpr::Lt(map(v)),
            FilterExpr::Le(v) => FilterExpr::Le(map(v)),
            FilterExpr::Gt(v) => FilterExpr::Gt(map(v)),
            FilterExpr::Ge(v) => FilterExpr::Ge(map(v)),
            FilterExpr::Or(xs) => FilterExpr::Or(xs.iter().map(|x| self.with_option_captions(t, pos, x)).collect()),
            FilterExpr::And(xs) => FilterExpr::And(xs.iter().map(|x| self.with_option_captions(t, pos, x)).collect()),
            other => other.clone(),
        }
    }

    /// The filter on field `pos` as `GetFilter` reads it back ('' when there is none).
    pub(crate) fn field_filter_text(&self, rec: &Rec, pos: usize) -> Result<String, Signal> {
        let table = self.table(rec)?;
        let r = rec.borrow();
        let parts = r
            .filters
            .iter()
            .filter(|f| f.field == pos && f.group == r.filter_group)
            .map(|f| self.filter_text(&self.with_option_captions(&table, pos, &f.expr)))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(parts.join(""))
    }

    /// The filters of the current filter group, in field order.
    fn filters_in_field_order(rec: &Rec) -> Vec<FieldFilter> {
        let r = rec.borrow();
        let mut v: Vec<FieldFilter> = r
            .filters
            .iter()
            .filter(|f| f.group == r.filter_group)
            .cloned()
            .collect();
        v.sort_by_key(|f| f.field);
        v
    }

    /// `FilterGroup([Group])` of a record or a RecordRef: the group it sets, or the current one without an
    /// argument (corpus record/TestRecordRuntimeTextsAndViews.al:Record_FilterGroup_ReturnsTheGroupJustSet_...).
    pub(crate) fn filter_group(rec: &Rec, args: &[Value]) -> Result<Value, Signal> {
        let mut r = rec.borrow_mut();
        if let Some(v) = args.first() {
            r.filter_group = match v {
                // The OR group: filters on different fields are alternatives (corpus
                // record/PWFilterGroupOr.al). Other negative groups are not pinned.
                Value::Integer(n) if *n < 0 && *n != al_builtins::filter::OR_GROUP => {
                    return unsupported(format!("FilterGroup({n}) (only -1, the OR group, is pinned)"));
                }
                Value::Integer(n) => *n,
                other => return unsupported(format!("FilterGroup({})", other.describe())),
            };
        }
        Ok(Value::Integer(r.filter_group))
    }

    /// Whether the current filter group has a filter (`HasFilter`).
    pub(crate) fn has_filter(rec: &Rec) -> bool {
        !Self::filters_in_field_order(rec).is_empty()
    }

    /// `GetFilters`: `Caption: filter` pairs joined by `, ` (docs/probes-needed.md).
    pub(crate) fn get_filters(&self, rec: &Rec, t: &TableDef) -> Result<String, Signal> {
        let mut parts = Vec::new();
        // A record on MarkedOnly names it first (errors/lt-ms-refusals-128/P128SelectionFilter.al `marked A only`,
        // lt-ms-refusals-135/P135SelectionFilter.al:357 `filters=[Marked: Yes]` with no other filter).
        if rec.borrow().marked_only {
            parts.push("Marked: Yes".to_string());
        }
        for f in Self::filters_in_field_order(rec) {
            parts.push(format!(
                "{}: {}",
                self.field_caption(t, f.field),
                self.filter_text(&self.with_option_captions(t, f.field, &f.expr))?
            ));
        }
        Ok(parts.join(", "))
    }

    /// `GetView(UseNames)`: `SORTING(keyfields) [ORDER(Descending)] [WHERE(f=FILTER(x),...)]`.
    /// Record and RecordRef views start with `VERSION(1) ` (corpus
    /// record/TestRecordRuntimeTextsAndViews.al:Record_GetView_And_GetFilters_Texts).
    pub(crate) fn get_view(&self, rec: &Rec, t: &TableDef, use_names: bool) -> Result<String, Signal> {
        Ok(format!(
            "VERSION(1) {}",
            self.get_view_with(rec, t, use_names, false, true)?
        ))
    }

    /// `get_view`; with `option_ordinals` an Option or Enum filter value is written as its
    /// ordinal (a FilterPageBuilder's `GetView(Name, false)`, corpus
    /// filterpage/TestFilterPageBuilder.al:FilterPageBuilder_GetView_CaptionsFlag_RendersAnOptionFilterDifferently).
    pub(crate) fn get_view_with(
        &self,
        rec: &Rec,
        t: &TableDef,
        use_names: bool,
        option_ordinals: bool,
        record_form: bool,
    ) -> Result<String, Signal> {
        let name = |pos: usize| {
            if use_names {
                // The caption stands for the name (record/TestProbeRecordRefContracts.al:GetTable_CopiesBufferFiltersKeyDirection:
                // `Identifier=FILTER(1..5)` for field Id with Caption 'Identifier').
                self.field_caption(t, pos)
            } else {
                format!("Field{}", t.fields[pos].id)
            }
        };
        let (key, ascending, mask) = {
            let r = rec.borrow();
            (r.key, r.ascending && r.field_descending == 0, r.field_descending)
        };
        let mut out = format!(
            "SORTING({})",
            t.keys[key].iter().map(|&p| name(p)).collect::<Vec<_>>().join(",")
        );
        let fields = t.keys[key].len();
        // Only the key's own fields show in the view. When just the primary key fields after the key run descending
        // (`Ascending(false)`, then `SetAscending(Field, true)`), the view is the plain ascending sorting
        // (record/lt-record-sort-ascending/RecordSortAscendingViews.al:P161_SetAscending_Single_AscendingMethodAndTies `back.view`).
        let key_bits = if fields >= 32 { u32::MAX } else { (1u32 << fields) - 1 };
        let mask_all = mask;
        let mask = mask & key_bits;
        let only_ties_descending = mask == 0 && mask_all != 0;
        if mask != 0 && !(fields == 1 && mask == 1) {
            // One direction per field of the key: `ORDER(Descending,Ascending)` (records/W4SortKeys.al:
            // SetAscending_TwoFieldKey_OneFieldAtATime). Every field descending, a field of the primary key's part, or the
            // numeric form is not pinned.
            if mask == key_bits {
                // Every field descending reads as the one direction (SetAscendingEdges.al:GetView_AllKeyFieldsDescending).
                out.push_str(if use_names || !record_form {
                    " ORDER(Descending)"
                } else {
                    " ORDER(1)"
                });
            } else {
                // The numeric form of a record: 1 descending, 0 ascending (SetAscendingEdges.al:GetView_MixedDirections).
                let dirs: Vec<&str> = (0..fields)
                    .map(|i| match (mask & (1 << i) != 0, use_names || !record_form) {
                        (true, true) => "Descending",
                        (false, true) => "Ascending",
                        (true, false) => "1",
                        (false, false) => "0",
                    })
                    .collect();
                out.push_str(&format!(" ORDER({})", dirs.join(",")));
            }
        } else if !ascending && !only_ties_descending {
            out.push_str(if use_names || !record_form {
                " ORDER(Descending)"
            } else {
                " ORDER(1)"
            });
        }
        let filters = Self::filters_in_field_order(rec);
        if !filters.is_empty() {
            let mut items = Vec::new();
            for f in &filters {
                let text = if option_ordinals {
                    self.filter_text(&ordinals(&f.expr))?
                } else {
                    self.filter_text_with(&f.expr, true)?
                };
                // The numeric form writes the kind as a number: FILTER is 1, CONST is 0.
                items.push(if use_names || !record_form {
                    format!("{}=FILTER({text})", name(f.field))
                } else {
                    format!("{}=1({text})", name(f.field))
                });
            }
            out.push_str(&format!(" WHERE({})", items.join(",")));
        }
        Ok(out)
    }

    /// The field a view names: by name or `Field<n>`; failing that by caption, which is what
    /// `GetView` writes; a name wins over a caption (record/lt-setview-caption/SetViewByCaption.al: SetView_CaptionInWhere
    /// `name_beta_is_also_a_caption`, SetView_CaptionInSorting).
    fn view_field(&self, t: &TableDef, name: &str) -> Result<usize, Signal> {
        if let Some(p) = field_by_name(t, name) {
            return Ok(p);
        }
        let wanted = name.trim_matches('"');
        match (0..t.fields.len()).find(|&p| self.field_caption(t, p).eq_ignore_ascii_case(wanted)) {
            Some(p) => Ok(p),
            None => unsupported(format!("view field `{wanted}`")),
        }
    }

    /// `SetView`: replaces the key, the order and all filters by those of the view.
    pub(crate) fn set_view(&mut self, rec: &Rec, t: &TableDef, view: &str) -> Result<(), Signal> {
        self.set_view_in(rec, t, view, None, false, false)
    }

    /// `set_view`, writing the filters of the view to filter group `group` (default: the current
    /// one). With `keep_unnamed`, a view that names no SORTING keeps the record's key and one that
    /// names no ORDER keeps its direction (a part page's SourceTableView after the part control's
    /// SubPageView, corpus handlers/TestPartOwnSortVsSubPageView.al:1-45). With `order_needs_key`,
    /// an ORDER without a SORTING is ignored (a part's SubPageView, corpus
    /// handlers/TestPartSubPageViewSorting.al:SubPageViewOrderOnly_KeepsPrimaryKeyOrder).
    pub(crate) fn set_view_in(
        &mut self,
        rec: &Rec,
        t: &TableDef,
        view: &str,
        group: Option<i32>,
        keep_unnamed: bool,
        order_needs_key: bool,
    ) -> Result<(), Signal> {
        let mut key = None;
        let mut adhoc: Option<Vec<usize>> = None;
        let mut ascending = None;
        // ORDER with one direction per SORTING field (true = descending) and the number of SORTING fields.
        let mut order_dirs: Option<Vec<bool>> = None;
        let mut sorting_len = 0usize;
        let mut filters: Vec<FieldFilter> = Vec::new();
        let group = group.unwrap_or(rec.borrow().filter_group);
        let mut rest = view.trim();
        while !rest.is_empty() {
            let open = rest.find('(').ok_or_else(|| invalid_view(view))?;
            let word = rest[..open].trim().to_ascii_uppercase();
            let (inner, after) = balanced(&rest[open..]).ok_or_else(|| invalid_view(view))?;
            match word.as_str() {
                "VERSION" => {}
                "SORTING" => {
                    let wanted = inner
                        .split(',')
                        .map(|n| self.view_field(t, n.trim()))
                        .collect::<Result<Vec<_>, _>>()?;
                    sorting_len = wanted.len();
                    // Fields no key starts with order the rows by themselves, as SetCurrentKey does (a view that
                    // GetView wrote for such a key sets it back). Not pinned for SetView; Microsoft's tests set views
                    // of ad hoc keys (assumption set-view-sorting-without-a-key).
                    match t
                        .keys
                        .iter()
                        .position(|k| k.len() >= wanted.len() && k[..wanted.len()] == wanted[..])
                    {
                        Some(k) => key = Some(k),
                        None => adhoc = Some(wanted),
                    }
                }
                "ORDER" => {
                    let mut dirs = Vec::new();
                    for item in inner.split(',') {
                        match item.trim().to_ascii_lowercase().as_str() {
                            "ascending" | "0" => dirs.push(false),
                            "descending" | "1" => dirs.push(true),
                            other => return unsupported(format!("view ORDER({other})")),
                        }
                    }
                    ascending = Some(!dirs[0]);
                    order_dirs = Some(dirs);
                }
                "WHERE" => {
                    for item in split_top(inner, ',') {
                        let (field, cond) = item
                            .split_once('=')
                            .ok_or_else(|| Signal::Unsupported(format!("view condition `{item}`")))?;
                        let pos = self.view_field(t, field.trim())?;
                        let cond = cond.trim();
                        let open = cond
                            .find('(')
                            .ok_or_else(|| Signal::Unsupported(format!("view condition `{item}`")))?;
                        let kind = cond[..open].trim().to_ascii_uppercase();
                        let (value, tail) = balanced(&cond[open..])
                            .ok_or_else(|| Signal::Unsupported(format!("view condition `{item}`")))?;
                        if !tail.trim().is_empty() || !matches!(kind.as_str(), "FILTER" | "CONST" | "0" | "1") {
                            return unsupported(format!("view condition `{item}`"));
                        }
                        let ty = t.fields[pos].ty.clone();
                        // `filter('>10')` is an AL text literal: the filter is what it holds, operators included
                        // (misc/lt-probes-needed-7/PnGFixtures.al:666 `SubPageView = where("Line No." = filter('>10'))`, opened by
                        // PnGPartTriggers.al:DisabledPartControlEnabled). A literal with a quote inside is not stripped.
                        let value = value
                            .trim()
                            .strip_prefix('\'')
                            .and_then(|v| v.strip_suffix('\''))
                            // Only a literal that starts with a comparison operator (`'>10'`, `'>=2'`) is read as a filter; a quoted name such as a
                            // member with parentheses keeps its quotes (record/TestCalcFormulaQuotedFilterTests.Codeunit.al:60308).
                            .filter(|v| !v.contains('\'') && !v.contains('"') && v.starts_with(['<', '>']))
                            .unwrap_or(value);
                        // A Date text reads as SetFilter reads it in the session's language: `010120..` is 01/01/20..
                        // (events/lt-platform-triggers/PtObserve.al:Filter_View).
                        let date_ctx = al_builtins::filter::DateFilterContext {
                            today: self.today_day_number(),
                            work_date: self.work_date_day().unwrap_or_else(|| self.today_day_number()),
                            language: self.global_language,
                        };
                        let expr =
                            al_builtins::filter::with_date_context(date_ctx, || self.parse_filter_text(value, &ty))
                                .map_err(|e| Signal::Unsupported(format!("SetView: {e}")))?;
                        filters.retain(|f| f.field != pos);
                        filters.push(FieldFilter {
                            field: pos,
                            expr,
                            group,
                        });
                    }
                }
                other => return unsupported(format!("view part {other}")),
            }
            rest = after.trim();
        }
        // One direction, or one per SORTING field (SetAscendingEdges.al:SetView_OrderClauses); any other count is an error.
        let mut field_mask = 0u32;
        if let Some(dirs) = &order_dirs {
            if dirs.len() > 1 {
                if dirs.len() != sorting_len {
                    return Err(crate::transactions::al_error_code(
                        "The number of ascending/descending specifications in the ORDER clause must either be 1 or the same as number of fields specified in the SORTING clause.",
                        "DB:Ok",
                    ));
                }
                // Every field the same way is one direction (SetAscendingEdges.al:`sorting_order_desc_desc`). Mixed
                // directions are pinned only for a SORTING that is a whole key.
                if dirs.iter().all(|d| *d) {
                    ascending = Some(false);
                } else if dirs.iter().all(|d| !*d) {
                    ascending = Some(true);
                } else if key.is_some_and(|k| t.keys[k].len() == sorting_len) && sorting_len < 32 {
                    ascending = Some(true);
                    for (i, d) in dirs.iter().enumerate() {
                        if *d {
                            field_mask |= 1 << i;
                        }
                    }
                } else {
                    return unsupported(
                        "view ORDER with mixed directions on a SORTING that is not a whole key (not pinned)",
                    );
                }
            }
        }
        let mut r = rec.borrow_mut();
        if order_dirs.is_some() {
            r.field_descending = field_mask;
        }
        // The view replaces the filters of the current group; other groups stay.
        r.filters.retain(|f| f.group != group);
        r.filters.extend(filters);
        if order_needs_key && key.is_none() {
            ascending = None;
        }
        if key.is_some() || !keep_unnamed {
            r.key = key.unwrap_or(0);
        }
        // A view that names no ORDER keeps the direction
        // (record/TestRecordViewCopyMarkContracts.al:130, SetView_ReplacesTheFilters_KeepsTheDirection_AndRejectsGarbage).
        if let Some(a) = ascending {
            r.ascending = a;
        }
        drop(r);
        if let Some(wanted) = adhoc {
            // SORTING(...) that no key starts with orders by those fields, then by the primary key, as SetCurrentKey does
            // (record/lt-setview-sorting-nonkey/SetViewSortingNonKey.al:SetView_SortingNonKeyField_Descending, SetView_SortingFieldLists).
            self.ret_used = false;
            let mut args: Vec<Value> = wanted.iter().map(|p| Value::Integer(*p as i32)).collect();
            self.record_op(al_builtins::record_op::RecordOp::SetCurrentKey, rec, &mut args)?;
        }
        Ok(())
    }
}

/// BC's error for a view that does not parse: a carriage return and a backslash join the
/// sentence to the view text (record/TestRecordViewCopyMarkContracts.al:130,
/// SetView_ReplacesTheFilters_KeepsTheDirection_AndRejectsGarbage).
fn invalid_view(view: &str) -> Signal {
    Signal::Error(Box::new(al_builtins::native::AlError {
        message: format!("Invalid expression of type: table view.{}{view}", crate::records::BR),
        code: "DB:Ok".into(),
        call_stack: String::new(),
        ..Default::default()
    }))
}

/// `e` with every Option and Enum value replaced by its ordinal.
fn ordinals(e: &FilterExpr) -> FilterExpr {
    let v = |x: &Value| match x {
        Value::Option(o, _) | Value::Enum(_, o) => Value::Integer(*o),
        other => other.clone(),
    };
    match e {
        FilterExpr::Range(a, b) => FilterExpr::Range(a.as_ref().map(v), b.as_ref().map(v)),
        FilterExpr::Eq(x) => FilterExpr::Eq(v(x)),
        FilterExpr::RangeSame(x) => FilterExpr::RangeSame(v(x)),
        FilterExpr::Ne(x) => FilterExpr::Ne(v(x)),
        FilterExpr::Lt(x) => FilterExpr::Lt(v(x)),
        FilterExpr::Le(x) => FilterExpr::Le(v(x)),
        FilterExpr::Gt(x) => FilterExpr::Gt(v(x)),
        FilterExpr::Ge(x) => FilterExpr::Ge(v(x)),
        FilterExpr::Or(xs) => FilterExpr::Or(xs.iter().map(ordinals).collect()),
        FilterExpr::And(xs) => FilterExpr::And(xs.iter().map(ordinals).collect()),
        FilterExpr::Like(p, ci) => FilterExpr::Like(p.clone(), *ci),
        FilterExpr::NotLike(p, ci) => FilterExpr::NotLike(p.clone(), *ci),
    }
}

/// A view written as an AL property (`DataItemTableView`, `SourceTableView`, `SubPageView`) in the
/// text form `SetView` reads: a double-quoted name inside a condition's value is the AL quoting of an
/// identifier, which the filter grammar writes with single quotes. Corpus
/// record/TestCalcFormulaQuotedFilterTests.Codeunit.al:189-215 (the oracle passes a DataItemTableView
/// `filter(<> "Initial Entry")` and `filter("Payment Discount (VAT Excl.)")`) and
/// handlers/TestPageSourceTableView.al:169-190 (a SourceTableView `const("G/L")`). Field names keep their
/// quotes (`field_by_name` takes either).
pub(crate) fn property_view(raw: &str) -> String {
    property_view_with(raw, true)
}

/// A page's `SourceTableView` (already through [`property_view`]) as the page applies it, and the fields it names with a
/// `const` on an Enum field (the page holds those as typed values: a TestPage's `Filter.GetFilter` reads the ordinal, pages/
/// lt-ce-page-run/CePageRun.al SetTableView_FilterOnAnotherField `Kind=0`).
/// - `const(0D)` and `filter(0D)` on a Date field filter on the blank date (the third list: how the filter reads back is not
///   pinned, so the page refuses every read of it) (pages/lt-cd-testpage-leads/CdTestPageLeads.al:508
///   Item58_SourceTableView_Const0D, :521 Item58_SourceTableView_Filter0D: the rows with 0D show).
/// - `const('')` on a Text field sets no filter (CdTestPageLeads.al:539 Item58_SourceTableView_ConstEmptyTextAndEnum: a row
///   whose field is `x` shows). On a field of another type nothing pins it, so it is refused.
pub(crate) fn page_source_view(t: &TableDef, view: &str) -> Result<(String, Vec<usize>, Vec<usize>), Signal> {
    let mut parts = Vec::new();
    let mut enum_consts = Vec::new();
    let mut blank_dates = Vec::new();
    let mut rest = view.trim();
    while !rest.is_empty() {
        let (Some(open), Some((inner, after))) = (rest.find('('), rest.find('(').and_then(|o| balanced(&rest[o..])))
        else {
            return Ok((view.to_string(), enum_consts, blank_dates));
        };
        let word = &rest[..open];
        if word.trim().eq_ignore_ascii_case("where") {
            let mut items = Vec::new();
            for item in split_top(inner, ',') {
                let parsed = item.split_once('=').and_then(|(field, cond)| {
                    let o = cond.find('(')?;
                    let (value, tail) = balanced(&cond[o..])?;
                    let pos = field_by_name(t, field.trim())?;
                    Some((field, pos, cond[..o].trim().to_ascii_uppercase(), value.trim(), tail))
                });
                let Some((field, pos, kind, value, tail)) = parsed else {
                    items.push(item.to_string());
                    continue;
                };
                let ty = &t.fields[pos].ty;
                use al_builtins::value::RtType;
                match ty {
                    RtType::Date if value.eq_ignore_ascii_case("0d") && matches!(kind.as_str(), "CONST" | "FILTER") => {
                        blank_dates.push(pos);
                        items.push(format!("{field}={kind}(''){tail}"));
                    }
                    _ if kind == "CONST" && value == "''" => {
                        if !matches!(ty, RtType::Text(_)) {
                            return unsupported(format!(
                                "SourceTableView const('') on a field of type {} (pinned for a Text field only)",
                                ty.describe()
                            ));
                        }
                    }
                    RtType::Enum(_) if kind == "CONST" => {
                        enum_consts.push(pos);
                        items.push(item.to_string());
                    }
                    _ => items.push(item.to_string()),
                }
            }
            if !items.is_empty() {
                parts.push(format!("{word}({})", items.join(",")));
            }
        } else {
            parts.push(format!("{word}({inner})"));
        }
        rest = after.trim();
    }
    Ok((parts.join(" "), enum_consts, blank_dates))
}

/// The conditions of a view (already through [`property_view`]) that filter a Date field with `const(0D)` or
/// `filter(0D)`: (field, `CONST` or `FILTER`).
pub(crate) fn blank_date_conditions(t: &TableDef, view: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut rest = view.trim();
    while let (Some(open), Some((inner, after))) = (rest.find('('), rest.find('(').and_then(|o| balanced(&rest[o..]))) {
        if rest[..open].trim().eq_ignore_ascii_case("where") {
            for item in split_top(inner, ',') {
                let parsed = item.split_once('=').and_then(|(field, cond)| {
                    let o = cond.find('(')?;
                    let (value, _) = balanced(&cond[o..])?;
                    let pos = field_by_name(t, field.trim())?;
                    Some((pos, cond[..o].trim().to_ascii_uppercase(), value.trim().to_string()))
                });
                if let Some((pos, kind, value)) = parsed {
                    if matches!(t.fields[pos].ty, al_builtins::value::RtType::Date) && value.eq_ignore_ascii_case("0d")
                    {
                        out.push((pos, kind));
                    }
                }
            }
        }
        rest = after.trim();
    }
    out
}

/// [`property_view`] for a part control's `SubPageView`: a quoted filter stays as written, so a view like `filter('>10')` is
/// refused.
pub(crate) fn sub_page_view_text(raw: &str) -> String {
    property_view_with(raw, false)
}

fn property_view_with(raw: &str, unquote_filter: bool) -> String {
    let mut parts = Vec::new();
    let mut rest = raw.trim();
    while !rest.is_empty() {
        let (Some(open), Some((inner, after))) = (rest.find('('), rest.find('(').and_then(|o| balanced(&rest[o..])))
        else {
            // Not a view this rewrite understands: left for `set_view` to reject.
            return raw.to_string();
        };
        let word = &rest[..open];
        if word.trim().eq_ignore_ascii_case("where") {
            let items: Vec<String> = split_top(inner, ',')
                .into_iter()
                .map(|item| match item.split_once('=') {
                    Some((field, cond)) => match (cond.find('('), cond.find('(').and_then(|o| balanced(&cond[o..]))) {
                        (Some(o), Some((value, tail))) => {
                            // The AL string of `filter('>10')` is the filter text `>10`; in a `const` it stays a quoted
                            // literal (handlers/TestPageSourceTableView.al, misc/lt-probes-needed-7/PnGFixtures.al:666, 1036).
                            let value = value.trim();
                            let is_filter = unquote_filter && cond[..o].trim().eq_ignore_ascii_case("filter");
                            let whole = value.len() >= 2
                                && value.starts_with('\'')
                                && value.ends_with('\'')
                                && !value[1..value.len() - 1].replace("''", "").contains('\'');
                            if is_filter && whole {
                                format!(
                                    "{field}={}({}){tail}",
                                    &cond[..o],
                                    value[1..value.len() - 1].replace("''", "'")
                                )
                            } else {
                                format!("{field}={}({}){tail}", &cond[..o], rewrite_al_quotes(value))
                            }
                        }
                        _ => item.to_string(),
                    },
                    None => item.to_string(),
                })
                .collect();
            parts.push(format!("{word}({})", items.join(",")));
        } else {
            parts.push(format!("{word}({inner})"));
        }
        rest = after.trim();
    }
    parts.join(" ")
}

/// AL's `"name"` as the filter literal `'name'`; a single-quoted literal is kept as it is.
fn rewrite_al_quotes(raw: &str) -> String {
    let mut out = String::new();
    let mut chars = raw.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                out.push('\'');
                for n in chars.by_ref() {
                    match n {
                        '"' => break,
                        '\'' => out.push_str("''"),
                        n => out.push(n),
                    }
                }
                out.push('\'');
            }
            '\'' => {
                out.push('\'');
                while let Some(n) = chars.next() {
                    out.push(n);
                    if n == '\'' {
                        if chars.peek() == Some(&'\'') {
                            out.push(chars.next().expect("peeked"));
                        } else {
                            break;
                        }
                    }
                }
            }
            c => out.push(c),
        }
    }
    out
}

/// `(a(b)c)rest` → (`a(b)c`, `rest`).
fn balanced(s: &str) -> Option<(&str, &str)> {
    let mut depth = 0usize;
    for (i, c) in s.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some((&s[1..i], &s[i + 1..]));
                }
            }
            _ => {}
        }
    }
    None
}

fn split_top(s: &str, sep: char) -> Vec<&str> {
    let mut out = Vec::new();
    let (mut depth, mut start) = (0i32, 0usize);
    for (i, c) in s.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            c if c == sep && depth == 0 => {
                out.push(&s[start..i]);
                start = i + c.len_utf8();
            }
            _ => {}
        }
    }
    out.push(&s[start..]);
    out
}

/// A view names a field by its name or, in the numeric form, as `Field<n>`.
fn field_by_name(t: &TableDef, name: &str) -> Option<usize> {
    let name = name.trim_matches('"');
    if let Some(p) = t.fields.iter().position(|f| f.name.eq_ignore_ascii_case(name)) {
        return Some(p);
    }
    name.strip_prefix("Field")
        .and_then(|n| n.parse::<u32>().ok())
        .and_then(|id| t.fields.iter().position(|f| f.id == id))
}

/// The members of an enum for a text filter: the captions first, then the names (see `filter::parse_filter_with`).
pub(crate) fn enum_filter_members(o: &al_ir::Object) -> Vec<(i32, String)> {
    let mut out: Vec<(i32, String)> = o.enum_captions.iter().filter(|(_, c)| !c.is_empty()).cloned().collect();
    out.extend(o.enum_values.iter().cloned());
    out
}

#[cfg(test)]
mod tests {
    use super::property_view;

    #[test]
    fn a_property_view_reads_al_quoted_names_in_values_as_filter_literals() {
        // corpus record/TestCalcFormulaQuotedFilterNegatedView.Report.al:18 and
        // TestCalcFormulaQuotedFilterParenView.Report.al:17 (the oracle passes
        // record/TestCalcFormulaQuotedFilterTests.Codeunit.al:189-215).
        assert_eq!(
            property_view("sorting(\"Entry No.\") where(\"Entry Type\" = filter(<> \"Initial Entry\"))"),
            "sorting(\"Entry No.\") where(\"Entry Type\" = filter(<> 'Initial Entry'))"
        );
        assert_eq!(
            property_view("where(\"Document Type\" = const(\"Credit Memo\"), Kind = const(Item))"),
            "where(\"Document Type\" = const('Credit Memo'), Kind = const(Item))"
        );
        assert_eq!(
            property_view("sorting(\"No.\") order(descending)"),
            "sorting(\"No.\") order(descending)"
        );
    }
}
