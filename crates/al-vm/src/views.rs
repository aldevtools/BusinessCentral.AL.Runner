//! Filter and view text: `GetFilter`, `GetFilters`, `GetView`, `SetView`.
//!
//! The corpus pins how a filter reads back for the simple shapes
//! (`'10..20'`, `'30'`, `'A'`: testpage/filters/TestTestFilter.al:227-350) and the view
//! grammar through the strings it feeds to `SetView`
//! (`'SORTING(Entry No.) WHERE(Entry No.=FILTER(1..3))'`:
//! recordref-fieldref/recordref/TestRecordRefFilter.al:23). Anything else is refused.

use crate::Session;
use al_builtins::describe::Describe;
use al_builtins::filter::{FieldFilter, FilterExpr, needs_quotes};
use al_builtins::native::{Signal, unsupported};
use al_builtins::value::{RecordState, RtType, Value};
use al_data::TableDef;
use std::cell::RefCell;
use std::rc::Rc;

type Rec = Rc<RefCell<RecordState>>;

/// `=` before a literal text that `filter_value_with` quotes for its `*` or `?` alone, so it does not read as a pattern.
fn equal_sign(v: &Value) -> &'static str {
    match v {
        Value::Text(s) | Value::Code(s)
            if s.contains(['*', '?']) && !s.contains('"') && !s.starts_with('@') && !needs_quotes(s) =>
        {
            "="
        }
        _ => "",
    }
}

impl Session {
    /// `enum_filter_members` of an enum id in the session language, for `filter::parse_filter_with`.
    pub(crate) fn enum_members_fn(&self) -> impl Fn(u32) -> Result<Vec<(i32, String)>, String> + use<> {
        let engine = self.engine.clone();
        let language = al_builtins::format::FormatCtx::language(self);
        move |id: u32| crate::views::enum_filter_members(&engine.program.objects[id as usize], language)
    }

    /// A filter expression over a field of type `ty`, enum members by name included.
    pub(crate) fn parse_filter_text(&self, text: &str, ty: &al_builtins::value::RtType) -> Result<FilterExpr, String> {
        let engine = self.engine.clone();
        let language = al_builtins::format::FormatCtx::language(self);
        let enums = |id: u32| crate::views::enum_filter_members(&engine.program.objects[id as usize], language);
        al_builtins::filter::parse_filter_with(text, &[], ty, &enums)
    }

    /// `filter_value`; with `iso` a Date reads `yyyy-mm-dd`, as a view writes it (record/crud/RcAMisc.al:
    /// GetView_Field_Types `date`).
    fn filter_value_with(&self, v: &Value, iso: bool) -> Result<String, Signal> {
        // An enum value with a `CaptionML` shows its caption in the session language, as Format does
        // (format-evaluate/EnumCaptionReaders.al ECR_FilterTexts_1033, _2067), when the filter was set under that language
        // (`Session::enum_ml_filter_check`, called by the readers of a filter entry).
        // A blank date shows as two single quotes, like a blank text (record/crud/RcAMisc.al:
        // SetRecFilter_Composite_And_Text_Keys `dateBlank`).
        // A blank DateTime too, in a view and in GetFilters (record/views/Y71SetViewDateTime.al:105-134
        // `>''&..2025-11-01T23:59:59Z`, `''..''|...`, `Dt: >''&..11/01/25 11:59:59 PM`).
        if matches!(v, Value::Date(0, false) | Value::DateTime(0)) {
            return Ok("''".into());
        }
        // An empty DateFormula and an empty RecordId show as two single quotes in GetFilter(s), set by SetRange or by a filter
        // text (recordref-fieldref/fieldref/FieldRefFilterEmptyText.al:139-150, :172-189; record/filters/
        // FilterEmptyDateFormula.al:109-118; al-runner issue 7437). In a view they are written as before.
        if !iso {
            let empty = match v {
                Value::DateFormula(f) => f.is_empty(),
                Value::Library(al_builtins::library_kind::LibraryKind::RecordId, _) => {
                    self.message_value(v)?.is_empty()
                }
                _ => false,
            };
            if empty {
                return Ok("''".into());
            }
        }
        // The zero Duration in GetFilter(s) (recordref-fieldref/fieldref/TestFieldRefFilter.al FieldRef_SetFilter_Zero_Duration, corpus
        // PR 1248: `''`, `<>''`, `''..41 days 16 hours`); in a view it is not pinned.
        if matches!(v, Value::Duration(0)) {
            if iso {
                return unsupported("the view text of the zero Duration (not pinned)");
            }
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
        // A view writes a DateTime in UTC as `yyyy-mm-ddThh:mm:ssZ`, with the milliseconds after a dot without trailing zeros
        // (`2025-01-15T17:45:10.5Z`) (P136Filter.al UN_DateTimeFilter_* `view=` types/P136Filter.al:447-586).
        if let (true, Value::DateTime(d)) = (iso, v) {
            if *d != 0 {
                let t = d - 1;
                let (day, ms) = (t.div_euclid(86_400_000), t.rem_euclid(86_400_000));
                let (y, m, dd) = al_builtins::format::civil_from_day((day + 1) as i32);
                let (h, mi, sec, f) = (ms / 3_600_000, ms / 60_000 % 60, ms / 1000 % 60, ms % 1000);
                let frac = if f == 0 {
                    String::new()
                } else {
                    format!(".{f:03}").trim_end_matches('0').to_string()
                };
                return Ok(format!("{y:04}-{m:02}-{dd:02}T{h:02}:{mi:02}:{sec:02}{frac}Z"));
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
                // `*` and `?` stay in a quoted value (W4Filters.al SetFilterPercent1_WildcardTogetherWithOperators); a double quote
                // next to an operator is not pinned.
                if s.contains('"') || s.starts_with('@') {
                    return unsupported("a filter value that needs quoting and holds more filter characters");
                }
                // record/TestFilterQuotedLiterals.al:95-105: an apostrophe in a quoted value is doubled.
                return Ok(format!("'{}'", s.replace('\'', "''")));
            }
            // The text `@` alone, from `SetFilter(F, '%1', '@')`, reads back quoted (format-evaluate/Y14Small.al:309, corpus
            // master).
            if &**s == "@" {
                return Ok("'@'".into());
            }
            // A double quote is shown as it is (W4Filters.al SetFilterPercent1_GreaterThanAndDoubleQuote `x"y`, `"y`, `"`,
            // SetFilterPercent1_WildcardTogetherWithOperators `*"*`).
            if s.starts_with('@') {
                return unsupported("a filter value that needs quoting");
            }
            // A literal text with `*` or `?` (from `=%1`, `>%1`, `<=%1`, a range end, SetRange, or `*` and `?` alone) is shown in
            // single quotes (record/filters/PlaceholderWildcardValue.al:249-298, :300-325 `'A*B'..AXB`, :401-419; al-runner issue
            // 7460; `*` alone: WildcardGetFilter.al:196, W4Filters.al `star_alone`). With an apostrophe it is not pinned.
            if s.contains(['*', '?']) && !s.contains('"') {
                // In a view the text is not pinned (except `*` alone, as before).
                if iso && &**s != "*" {
                    return unsupported("the view text of a literal filter value with a wildcard (not pinned)");
                }
                if s.contains('\'') {
                    return unsupported("a filter value with a wildcard and an apostrophe (not pinned)");
                }
                return Ok(format!("'{s}'"));
            }
        }
        // A Time reads as Format gives it (events/platform-triggers/PtObserve.al:Filter_DateTime_Time `>=12:00:00 PM`, Bad_Time m3);
        // a time with milliseconds is not pinned.
        if let Value::Time(t) = v {
            if *t != 0 && (t - 1) % 1000 == 0 {
                return al_builtins::format::try_format(v, 0, 0, self).map_err(Signal::unsupported);
            }
            // A time with milliseconds shows them as standard format 1 writes it (`12:00:00.123 PM`, `>= 9:05:07.5 AM`), and a
            // view writes it on a 24-hour clock as format 9 does (`T=FILTER(23:59:59.999)`) (events/table-events/
            // BdFilterEvents.al:BdFilter_TimeMilliseconds, BdFilter_TimeMillisecondsView).
            if *t != 0 {
                let standard = if iso { 9 } else { 1 };
                return al_builtins::format::try_format(v, 0, standard, self).map_err(Signal::unsupported);
            }
            // The undefined Time shows as `''` in GetFilter(s), given as `''`, as `%1` or by SetRange (corpus
            // master 3eb812ed, record/filters/TimeFilterEmptyText.al:113-126). In a view it is not pinned.
            if !iso {
                return Ok("''".into());
            }
            return unsupported("the view text of an undefined Time (not pinned)");
        }
        // A DateTime with milliseconds shows its seconds and milliseconds, as standard format 1 writes them
        // (record/filters/P130DateFilters.al:211, `03/24/24 11:59:59.999 PM`; a whole minute stays `12:00 AM`).
        if let Value::DateTime(d) = v {
            if *d != 0 && (d - 1) % 1000 != 0 {
                return al_builtins::format::try_format(v, 0, 1, self).map_err(Signal::unsupported);
            }
            // The last second of a day, the end of a date-alone filter, shows its seconds (`01/15/25 11:59:59 PM`, P136Filter.al
            // UN_DateTimeFilter_DateAlone types/P136Filter.al:447-586).
            if *d != 0 && (d - 1) % 86_400_000 == 86_399_000 {
                return al_builtins::format::try_format(v, 0, 1, self).map_err(Signal::unsupported);
            }
            // Other seconds without milliseconds: not pinned.
            if *d != 0 && (d - 1) / 1000 % 60 != 0 {
                return unsupported("the filter text of a DateTime with seconds and no milliseconds (not pinned)");
            }
        }
        let text = self.message_value(v)?;
        // A value that is only blank space is shown in single quotes (corpus
        // record/TestRecordRuntimeTextsAndViews.al:Record_GetView_And_GetFilters_Texts, the blank
        // option member: `Status: <>' '`).
        // A value with a leading space too (record/filters/W4Filters.al:BlankOnlyFilterValues_MatchOnlyBlankRows:
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
                    // SetRange of one wildcard text reads back as `='A*B'` (PlaceholderWildcardValue.al:413-417, :622-626).
                    format!("{}{}", equal_sign(a), self.filter_value_with(a, iso)?)
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
            // A literal wildcard text is shown after `=`: the text `*` from `SetFilter(F, '%1', '*')` as `='*'` (record/filters/
            // W4Filters.al SetFilterPercent1_WildcardTogetherWithOperators `star_alone`), `?` and `=%1` with `A*B` as `='?'` and
            // `='A*B'` (PlaceholderWildcardValue.al:236, :249-262).
            FilterExpr::Eq(v) => format!("{}{}", equal_sign(v), self.filter_value_with(v, iso)?),
            // The blank DateTime of a quoted empty text alone (see the parser) shows `''..''` in GetFilter(s) and in a view
            // (record/filters/TimeFilterEmptyText.al:161 `empty`; record/views/Y71SetViewDateTime.al:125-127 dt_empty_or).
            FilterExpr::RangeSame(Value::DateTime(0)) => "''..''".to_string(),
            FilterExpr::RangeSame(v) => {
                let t = self.filter_value_with(v, iso)?;
                format!("{t}..{t}")
            }
            // `<>%1` with the text `*` or `?` reads back quoted (record/filters/WildcardGetFilter.al:196, :199;
            // PlaceholderWildcardValue.al:226), as `filter_value_with` writes a literal wildcard text.
            FilterExpr::Ne(v) => format!("<>{}", self.filter_value_with(v, iso)?),
            FilterExpr::Lt(v) => format!("<{}", self.filter_value_with(v, iso)?),
            FilterExpr::Le(v) => format!("<={}", self.filter_value_with(v, iso)?),
            FilterExpr::Gt(v) => format!(">{}", self.filter_value_with(v, iso)?),
            FilterExpr::Ge(v) => format!(">={}", self.filter_value_with(v, iso)?),
            // With `>` too: `'@a>b@x.com'` (format-evaluate/Y14Small.al:309, corpus master).
            FilterExpr::Like(p, ci) if needs_quotes(p) && !p.contains('\'') => {
                format!("'{}{p}'", if *ci { "@" } else { "" })
            }
            // `@''` reads back as written (corpus master record/crud/RcAFilters.al:1011, :1026 `at two apos`).
            FilterExpr::Like(p, true) if p.is_empty() => "@''".to_string(),
            FilterExpr::Like(p, ci) => format!("{}{p}", if *ci { "@" } else { "" }),
            // record/filters/WildcardGetFilter.al:125-171: `<>*`, `<>A*`, `<>@a*` read back as written.
            FilterExpr::NotLike(p, ci) => format!("<>{}{p}", if *ci { "@" } else { "" }),
            // record/filters/WcInvalidFilterExpr.al:84-111: `@` and `<>@` read back as written.
            FilterExpr::AtSignAlone(negated) => if *negated { "<>@" } else { "@" }.to_string(),
            FilterExpr::EmptyViewValue => "==".to_string(),
            FilterExpr::UnpinnedMatch(inner, _) => self.filter_text_with(inner, iso)?,
            FilterExpr::UnpinnedText(_) => {
                return unsupported(
                    "the text of a filter with a wildcard text written as a literal value (not pinned)",
                );
            }
            FilterExpr::NumCode(inner) => self.filter_text_with(inner, iso)?,
            // The parts of an OR list each show as alone: a quoted empty text on a DateTime field as `''..''` (record/views/
            // Y71SetViewDateTime.al:125-127 dt_empty_or, :207-209 sca_empty_or), the blank DateTime given as %1 as `''`
            // (record/filters/TimeFilterEmptyText.al:169 `empty or arg`); a Date stays `''` (Y71SetViewDateTime.al:289-291).
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
            .map(|f| {
                self.enum_ml_filter_check(&table, f)?;
                self.filter_text(&self.with_option_captions(&table, pos, &f.expr))
            })
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

    /// The end of `CopyFilter(FromField, Target.ToField)`: `copied` (already moved to the target's group) replace the target field's
    /// filters, and the target forgets the row it holds (record/views/R6807ViewFind.al, Rec_copyFilterFieldStored,
    /// Rec_copyFilterFieldFlowZero).
    pub(crate) fn put_copied_filters(dst: &Rec, to: usize, to_group: i32, copied: Vec<FieldFilter>) {
        let mut d = dst.borrow_mut();
        d.position = None;
        d.filters.retain(|f| !(f.field == to && f.group == to_group));
        for mut f in copied {
            f.field = to;
            f.group = to_group;
            d.filters.push(f);
        }
    }

    /// `Target.CopyFilters(Source)`: the source's filters replace the target's; the target forgets the row it holds, so the next
    /// Find tests it against them like after SetRange (record/views/R6807ViewFind.al, Rec_copyStored, Rec_copyFlowZero).
    pub(crate) fn copy_filters(rec: &Rec, args: &[Value]) -> Result<Value, Signal> {
        let Some(Value::Record(src)) = args.first() else {
            return unsupported(format!("CopyFilters from {}", args.first().describe()));
        };
        if !Rc::ptr_eq(src, rec) {
            let filters = src.borrow().filters.clone();
            let mut r = rec.borrow_mut();
            r.filters = filters;
            r.position = None;
        }
        Ok(Value::Null)
    }

    /// `FilterGroup([Group])` of a record or a RecordRef: the group it sets, or the current one without an
    /// argument (corpus record/TestRecordRuntimeTextsAndViews.al:Record_FilterGroup_ReturnsTheGroupJustSet_...).
    pub(crate) fn filter_group(rec: &Rec, args: &[Value]) -> Result<Value, Signal> {
        let mut r = rec.borrow_mut();
        if let Some(v) = args.first() {
            r.filter_group = match v {
                // The OR group: filters on different fields are alternatives (corpus record/PWFilterGroupOr.al). Another negative
                // group is set and read back like any other (record/filters/P68F_FilterGroup.al
                // RecordRef_Setter_Negative_And_Large: -100 and -2147483647); a filter set in it is refused in `record_op_inner`.
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
        // A record on MarkedOnly names it first (testpage/P128SelectionFilter.al `marked A only`,
        // lt-ms-refusals-135/P135SelectionFilter.al:357 `filters=[Marked: Yes]` with no other filter).
        if rec.borrow().marked_only {
            parts.push("Marked: Yes".to_string());
        }
        for f in Self::filters_in_field_order(rec) {
            self.enum_ml_filter_check(t, &f)?;
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
    /// pages/filterpagebuilder/TestFilterPageBuilder.al:FilterPageBuilder_GetView_CaptionsFlag_RendersAnOptionFilterDifferently).
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
        // (record/keys-sorting/RecordSortAscendingViews.al:P161_SetAscending_Single_AscendingMethodAndTies `back.view`).
        let key_bits = if fields >= 32 { u32::MAX } else { (1u32 << fields) - 1 };
        let mask_all = mask;
        let mask = mask & key_bits;
        let only_ties_descending = mask == 0 && mask_all != 0;
        if mask != 0 && !(fields == 1 && mask == 1) {
            // One direction per field of the key: `ORDER(Descending,Ascending)` (record/W4SortKeys.al:
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
                // The numeric form of a record writes an Option filter as its ordinal (record/keys-sorting/
                // Y43SetAscendingFlowField.al:234 `WHERE(Field1=1(1))`), and an Enum filter too, from a Record and a RecordRef
                // (corpus master format-evaluate/EnumCaptionReaders.al:1530-1551, ECR_RecordRefSetViewEnumName_GetViewFalseWritesTheOrdinal).
                let numeric_option =
                    !use_names && record_form && matches!(t.fields[f.field].ty, RtType::Option(_) | RtType::Enum(_));
                let text = if option_ordinals || numeric_option {
                    // A Date or DateTime value takes the XML form here too (events/platform-triggers/PtObserve.al:600
                    // `WHERE(Field5=1(2020-01-01..))`); a Boolean of this form is not pinned and keeps its session form.
                    let iso = !matches!(t.fields[f.field].ty, RtType::Boolean);
                    self.filter_text_with(&ordinals(&f.expr), iso)?
                } else {
                    self.enum_ml_filter_check(t, f)?;
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
    /// `GetView` writes; a name wins over a caption (record/views/SetViewByCaption.al: SetView_CaptionInWhere
    /// `name_beta_is_also_a_caption`, SetView_CaptionInSorting).
    pub(crate) fn view_field(&self, t: &TableDef, name: &str) -> Result<usize, Signal> {
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
        self.set_view_in(rec, t, view, None, false, false, true)
    }

    /// `set_view`, writing the filters of the view to filter group `group` (default: the current
    /// one). With `keep_unnamed`, a view that names no SORTING keeps the record's key and one that
    /// names no ORDER keeps its direction (a part page's SourceTableView after the part control's
    /// SubPageView, corpus testpage/TestPartOwnSortVsSubPageView.al:1-45). With `order_needs_key`,
    /// an ORDER without a SORTING is ignored (a part's SubPageView, corpus
    /// testpage/TestPartSubPageViewSorting.al:SubPageViewOrderOnly_KeepsPrimaryKeyOrder). `runtime` is a
    /// view given to SetView at run time; otherwise it is an object property (SourceTableView, SubPageView, ...),
    /// which reads a quoted Text/Code operator differently (see the WHERE arm).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn set_view_in(
        &mut self,
        rec: &Rec,
        t: &TableDef,
        view: &str,
        group: Option<i32>,
        keep_unnamed: bool,
        order_needs_key: bool,
        runtime: bool,
    ) -> Result<(), Signal> {
        let mut key = None;
        let mut adhoc: Option<Vec<usize>> = None;
        let mut ascending = None;
        // ORDER with one direction per SORTING field (true = descending) and the number of SORTING fields.
        let mut order_dirs: Option<Vec<bool>> = None;
        let mut sorting_len = 0usize;
        let mut filters: Vec<FieldFilter> = Vec::new();
        let group = group.unwrap_or(rec.borrow().filter_group);
        for (word, inner) in view_parts(view).ok_or_else(|| invalid_view(view))? {
            match word.as_str() {
                "VERSION" => {}
                "SORTING" => {
                    let Some(wanted) = self.view_sorting_fields(t, inner, view, runtime)? else {
                        key = Some(0);
                        adhoc = None;
                        sorting_len = 1;
                        continue;
                    };
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
                    let items = split_top(inner, ',');
                    let alone = items.len() == 1;
                    for item in items {
                        let (field, cond) = item
                            .split_once('=')
                            .ok_or_else(|| Signal::unsupported(format!("view condition `{item}`")))?;
                        let Some(pos) = self.view_where_field(t, field.trim(), cond, runtime, alone)? else {
                            continue;
                        };
                        let cond = cond.trim();
                        let open = cond
                            .find('(')
                            .ok_or_else(|| Signal::unsupported(format!("view condition `{item}`")))?;
                        let kind = cond[..open].trim().to_ascii_uppercase();
                        let (value, tail) = balanced(&cond[open..])
                            .ok_or_else(|| Signal::unsupported(format!("view condition `{item}`")))?;
                        if !tail.trim().is_empty() || !matches!(kind.as_str(), "FILTER" | "CONST" | "0" | "1") {
                            return unsupported(format!("view condition `{item}`"));
                        }
                        let ty = t.fields[pos].ty.clone();
                        // `const("ALT Status"::Active)` in an object property reads as the member
                        // (record/qualified-enum-filters/TestQualifiedEnumFilters.al:SourceTableView_QualifiedEnumConst_OpensActiveRow,
                        // al-runner issue 7189). A view given to SetView is text, not AL: no qualifier there.
                        let stripped = match &ty {
                            al_builtins::value::RtType::Enum(id) if !runtime => {
                                let name = &self.engine.program.objects[*id as usize].name;
                                match al_builtins::enum_qualifier::strip_enum_qualifier(value, name) {
                                    std::borrow::Cow::Borrowed(v) if v.contains("::") => self.cross_enum_member(v, *id),
                                    stripped => stripped,
                                }
                            }
                            _ => std::borrow::Cow::Borrowed(value),
                        };
                        // `<> "Enum"::Member`, `Database::"Table"`, `Page::"Page"` and two of them joined by `|` in an object
                        // property (pages/link-filter/LinkEnumQualifiedAndObjectConstants.al, al-runner issues 7405, 7411; a page's
                        // SourceTableView: pages/view-filters/ViewConstBlankCode.al:719, :755).
                        let stripped = match (!runtime && stripped.contains("::"))
                            .then(|| self.view_qualified_operands(&stripped, &ty))
                            .flatten()
                        {
                            Some(resolved) => std::borrow::Cow::Owned(resolved),
                            None => stripped,
                        };
                        let value: &str = &stripped;
                        // `filter('>10')` is an AL text literal: the filter is what it holds, operators included
                        // (testpage/PnGFixtures.al:666 `SubPageView = where("Line No." = filter('>10'))`, opened by
                        // PnGPartTriggers.al:DisabledPartControlEnabled). A literal with a quote inside is not stripped.
                        let value = value
                            .trim()
                            .strip_prefix('\'')
                            .and_then(|v| v.strip_suffix('\''))
                            // In a view given to SetView, a quoted value of a Text/Code field is the literal value (as
                            // GetView writes it): FILTER('>B') selects `>B`, FILTER(>B) compares. In an object property
                            // it compares, as above (record/filters/UcQuotedValues.al: SetView_FilterGreaterB,
                            // PageAndQuery_FilterGreaterB, BC 27.5-29.0).
                            .filter(|_| {
                                !runtime
                                    || !matches!(
                                        ty,
                                        al_builtins::value::RtType::Text(_) | al_builtins::value::RtType::Code(_)
                                    )
                            })
                            // Only a literal that starts with a comparison operator (`'>10'`, `'>=2'`) is read as a filter; a quoted name such as a
                            // member with parentheses keeps its quotes (record/TestCalcFormulaQuotedFilterTests.Codeunit.al:60308).
                            .filter(|v| !v.contains('\'') && !v.contains('"') && v.starts_with(['<', '>']))
                            .unwrap_or(value);
                        // `const(0D)` on a Date field is the blank date, in every view (Y76ViewBlankDate.al:280
                        // Y76_Action_RunPageView_Const0D, :293 Y76_Part_SubPageView `const part`).
                        let const_blank_date = !runtime
                            && kind == "CONST"
                            && matches!(ty, al_builtins::value::RtType::Date)
                            && value.eq_ignore_ascii_case("0d");
                        let (kind, value) = if const_blank_date {
                            ("FILTER".to_string(), "")
                        } else {
                            (kind, value)
                        };
                        let empty = self.empty_view_value(&kind, &ty, value, runtime)?;
                        // `filter('')` on a Date field reaches here as an empty value: it selects the blank date, as `filter(0D)`
                        // (`''` here) does, in a top-level page's SourceTableView (testpage/CdTestPageLeads.al:842
                        // SH_SourceTableView_DateFilterBlank_Rows; Item58_SourceTableView_Filter0D at :521 for filter(0D)) and in an
                        // action's RunPageView, a part's SubPageView, a report's, XMLport's and query's view (pages/view-filters/
                        // Y76ViewBlankDate.al:252 Y76_Action_RunPageView_Blank, :293 Y76_Part_SubPageView, :325, :346, :368).
                        let value = if empty.is_none()
                            && kind == "FILTER"
                            && matches!(ty, al_builtins::value::RtType::Date)
                            && value.trim().is_empty()
                        {
                            "''"
                        } else {
                            value
                        };
                        // A Date text reads as SetFilter reads it in the session's language: `010120..` is 01/01/20..
                        // (events/platform-triggers/PtObserve.al:Filter_View).
                        let date_ctx = self.date_filter_context_for(value);
                        let database = self.view_database_const(value, &ty, runtime)?;
                        let value = database.as_deref().unwrap_or(value);
                        let expr = match empty {
                            Some(e) => e,
                            None => {
                                al_builtins::filter::with_date_context(date_ctx, || self.parse_filter_text(value, &ty))
                                    .map_err(|e| Signal::unsupported(format!("SetView: {e}")))?
                            }
                        };
                        filters.retain(|f| f.field != pos);
                        filters.push(FieldFilter {
                            lang: al_builtins::filter::filter_language(),
                            field: pos,
                            expr,
                            group,
                        });
                    }
                }
                other => return unsupported(format!("view part {other}")),
            }
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
        // A view set after a Get or a Find makes the record forget that it holds its row, as SetRange and SetFilter do: Find()
        // then tests the row against the new filters, on a stored field and on a FlowField, Record and RecordRef alike
        // (record/views/R6807ViewFind.al, Rec_svStored, Rec_svFlowZero, Ref_svStored, Ref_svFlowZero,
        // Ref_viewRoundTripFlowZero).
        r.position = None;
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
            // (record/keys-sorting/SetViewSortingNonKey.al:SetView_SortingNonKeyField_Descending, SetView_SortingFieldLists).
            self.ret_used = false;
            let mut args: Vec<Value> = wanted.iter().map(|p| Value::Integer(*p as i32)).collect();
            self.record_op(al_builtins::record_op::RecordOp::SetCurrentKey, rec, &mut args)?;
        }
        Ok(())
    }
}

/// The parts of a view, `WORD(body)` one after the other, each word upper-cased with the text between its brackets (brackets
/// inside quotes do not count, see [`balanced`]). `None` when the view does not read so. The one reader of a view's
/// structure: `set_view_in`, [`view_clauses`] and the page metadata view (`platform/pages.rs`) all read through it.
pub(crate) fn view_parts(view: &str) -> Option<Vec<(String, &str)>> {
    let mut out = Vec::new();
    let mut rest = view.trim();
    while !rest.is_empty() {
        let open = rest.find('(')?;
        let (inner, after) = balanced(&rest[open..])?;
        out.push((rest[..open].trim().to_ascii_uppercase(), inner));
        rest = after.trim();
    }
    Some(out)
}

/// Which of SORTING, ORDER and WHERE a view has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ViewClauses {
    pub(crate) sorting: bool,
    pub(crate) order: bool,
    pub(crate) filters: bool,
}

/// The clauses `view` names, read by [`view_parts`], never by searching the text: a field name or a value that holds one of
/// the words (`"Order No."`, `"Sorting Group"`, `filter(<>'sorting')`) names no clause (AL Runner issue 7613). A view that
/// does not read as SORTING, ORDER, WHERE and VERSION parts counts as having all three, so every caller takes its careful
/// path (a refusal, or what the view sets), and `set_view_in` rejects the view itself.
pub(crate) fn view_clauses(view: &str) -> ViewClauses {
    let all = ViewClauses {
        sorting: true,
        order: true,
        filters: true,
    };
    let Some(parts) = view_parts(view) else { return all };
    let mut out = ViewClauses {
        sorting: false,
        order: false,
        filters: false,
    };
    for (word, _) in parts {
        match word.as_str() {
            "SORTING" => out.sorting = true,
            "ORDER" => out.order = true,
            "WHERE" => out.filters = true,
            "VERSION" => {}
            _ => return all,
        }
    }
    out
}

/// BC's error for a view that does not parse: a carriage return and a backslash join the
/// sentence to the view text (record/TestRecordViewCopyMarkContracts.al:130,
/// SetView_ReplacesTheFilters_KeepsTheDirection_AndRejectsGarbage).
pub(crate) fn invalid_view(view: &str) -> Signal {
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
        FilterExpr::AtSignAlone(n) => FilterExpr::AtSignAlone(*n),
        FilterExpr::EmptyViewValue => FilterExpr::EmptyViewValue,
        FilterExpr::UnpinnedMatch(..) | FilterExpr::UnpinnedText(_) => e.clone(),
        FilterExpr::NumCode(e) => FilterExpr::NumCode(e.clone()),
    }
}

/// A view written as an AL property (`DataItemTableView`, `SourceTableView`, `SubPageView`) in the
/// text form `SetView` reads: a double-quoted name inside a condition's value is the AL quoting of an
/// identifier, which the filter grammar writes with single quotes. Corpus
/// record/TestCalcFormulaQuotedFilterTests.Codeunit.al:189-215 (the oracle passes a DataItemTableView
/// `filter(<> "Initial Entry")` and `filter("Payment Discount (VAT Excl.)")`) and
/// testpage/TestPageSourceTableView.al:169-190 (a SourceTableView `const("G/L")`). Field names keep their
/// quotes (`field_by_name` takes either).
pub(crate) fn property_view(raw: &str) -> String {
    property_view_with(raw, true)
}

/// A page's `SourceTableView` as [`page_source_view`] reads it.
pub(crate) struct PageSourceView {
    /// The view to apply, the `const('')` conditions that set no filter taken out.
    pub view: String,
    /// The fields of a `const` on an Enum field (held as typed values).
    pub enum_consts: Vec<usize>,
    /// The fields of a `const(0D)` or `filter(0D)` on a Date field.
    pub blank_dates: Vec<usize>,
    /// The fields whose view filter `Filter.GetFilter` reads as the member's number.
    pub number_fields: Vec<usize>,
}

/// A member name alone: no filter operator, quote or qualifier.
fn is_plain_member(value: &str) -> bool {
    !value.is_empty() && value.chars().all(|c| c.is_alphanumeric() || c == '_')
}

/// A page's `SourceTableView` (already through [`property_view`]) as the page applies it, and the fields it names with a
/// `const` on an Enum field (the page holds those as typed values: a TestPage's `Filter.GetFilter` reads the ordinal, pages/
/// lt-ce-page-run/CePageRun.al SetTableView_FilterOnAnotherField `Kind=0`).
/// - `const(0D)` and `filter(0D)` on a Date field filter on the blank date (the third list: how the filter reads back is not
///   pinned, so the page refuses every read of it) (testpage/CdTestPageLeads.al:508
///   Item58_SourceTableView_Const0D, :521 Item58_SourceTableView_Filter0D: the rows with 0D show).
/// - `const('')` on a Text field sets no filter (CdTestPageLeads.al:539 Item58_SourceTableView_ConstEmptyTextAndEnum: a row
///   whose field is `x` shows), and on a Code field too (al-runner issue 7371). On a field of another type nothing pins it, so
///   it is refused.
pub(crate) fn page_source_view(t: &TableDef, view: &str) -> Result<PageSourceView, Signal> {
    let mut parts = Vec::new();
    let mut enum_consts = Vec::new();
    let mut blank_dates = Vec::new();
    let mut number_fields = Vec::new();
    let mut rest = view.trim();
    while !rest.is_empty() {
        let (Some(open), Some((inner, after))) = (rest.find('('), rest.find('(').and_then(|o| balanced(&rest[o..])))
        else {
            return Ok(PageSourceView {
                view: view.to_string(),
                enum_consts,
                blank_dates,
                number_fields,
            });
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
                    // `filter('')` (written `FILTER()` here) on a Code field selects the blank value, and `Filter.GetFilter` reads `''`
                    // (pages/view-filters/ViewConstBlankCode.al:559 SourceTableView_C20Filter_RowsAndFilters).
                    RtType::Code(_) if kind == "FILTER" && value.is_empty() => {
                        items.push(format!("{field}={kind}(''){tail}"));
                    }
                    // `const('')` on a Code[20] or Code[10] field sets no filter either, alone, twice, next to a value const, an
                    // enum const, an Option const or a sorting() (ViewConstBlankCode.al:454, :524, :629, :699, :804, :489, :839).
                    _ if kind == "CONST" && value == "''" => {
                        if !matches!(ty, RtType::Text(_) | RtType::Code(_)) {
                            return unsupported(format!(
                                "SourceTableView const('') on a field of type {} (pinned for a Text or Code field only)",
                                ty.describe()
                            ));
                        }
                    }
                    RtType::Enum(_) if kind == "CONST" => {
                        enum_consts.push(pos);
                        items.push(item.to_string());
                    }
                    // `filter(Order)` of one Enum member and `const(Item)` of an Option member read back as the member's number
                    // in `Filter.GetFilter` (pages/view-filters/ViewConstBlankCode.al:490 SourceTableView_Reported_RowsAndFilters
                    // `Doc Type=[1]`, `Kind=[1]`). Other filter texts keep main's reading.
                    RtType::Enum(_) if kind == "FILTER" && is_plain_member(value) => {
                        number_fields.push(pos);
                        items.push(item.to_string());
                    }
                    RtType::Option(_) if kind == "CONST" && is_plain_member(value) => {
                        number_fields.push(pos);
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
    Ok(PageSourceView {
        view: parts.join(" "),
        enum_consts,
        blank_dates,
        number_fields,
    })
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
                            // literal (testpage/TestPageSourceTableView.al, testpage/PnGFixtures.al:666, 1036).
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

/// `(a(b)c)rest` → (`a(b)c`, `rest`). Parentheses inside a quoted value or name do not count: GetView quotes
/// a value such as `Diana's (Meals` as `'Diana''s (Meals'` (apostrophes doubled, record/TestFilterQuotedLiterals.al:95-105),
/// and SetView must read it back. A view whose quotes do not pair is scanned as before, quotes ignored.
fn balanced(s: &str) -> Option<(&str, &str)> {
    balanced_with(s, true).or_else(|| balanced_with(s, false))
}

fn balanced_with(s: &str, quotes: bool) -> Option<(&str, &str)> {
    let mut depth = 0usize;
    let mut quote = None;
    for (i, c) in s.char_indices() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), _) => {}
            (None, '\'' | '"') if quotes => quote = Some(c),
            (None, '(') => depth += 1,
            (None, ')') => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some((&s[1..i], &s[i + 1..]));
                }
            }
            _ => {}
        }
    }
    None
}

/// `s` split at each `sep` outside parentheses and quotes (see `balanced`).
fn split_top(s: &str, sep: char) -> Vec<&str> {
    split_top_with(s, sep, true).unwrap_or_else(|| split_top_with(s, sep, false).unwrap_or_else(|| vec![s]))
}

fn split_top_with(s: &str, sep: char, quotes: bool) -> Option<Vec<&str>> {
    let mut out = Vec::new();
    let (mut depth, mut start, mut quote) = (0i32, 0usize, None);
    for (i, c) in s.char_indices() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), _) => {}
            (None, '\'' | '"') if quotes => quote = Some(c),
            (None, '(') => depth += 1,
            (None, ')') => depth -= 1,
            (None, c) if c == sep && depth == 0 => {
                out.push(&s[start..i]);
                start = i + c.len_utf8();
            }
            _ => {}
        }
    }
    out.push(&s[start..]);
    quote.is_none().then_some(out)
}

/// A view names a field by its name or, in the numeric form, as `Field<n>` in any case (`field2`, `fIeLd4`:
/// record/views/Y51SetViewShapes.al:118-131, SetView_LowercaseFieldN).
fn field_by_name(t: &TableDef, name: &str) -> Option<usize> {
    let name = name.trim_matches('"');
    if let Some(p) = t.fields.iter().position(|f| f.name.eq_ignore_ascii_case(name)) {
        return Some(p);
    }
    name.get(..5)
        .filter(|p| p.eq_ignore_ascii_case("field"))
        .and_then(|_| name[5..].parse::<u32>().ok())
        .and_then(|id| t.fields.iter().position(|f| f.id == id))
}

/// The members of an enum for a text filter: the captions first, then the names (see `filter::parse_filter_with`). A value
/// with a `CaptionML` is matched by its caption in the session language `language`, not by another language's caption
/// (format-evaluate/EnumCaptionReaders.al ECR_FilterWordsStatus_1033, _2067, ECR_FilterWordsMixed_*, ECR_ViewWords_*).
pub(crate) fn enum_filter_members(o: &al_ir::Object, language: i32) -> Result<Vec<(i32, String)>, String> {
    let mut out: Vec<(i32, String)> = o.enum_captions.iter().filter(|(_, c)| !c.is_empty()).cloned().collect();
    for (v, ml) in &o.enum_captions_ml {
        // A value with both a Caption and a CaptionML is not pinned (as `Session::enum_ml_captions`).
        if out.iter().any(|(w, _)| w == v) {
            return Err(format!(
                "the caption of value {v} of enum {}: it declares a Caption and a CaptionML (not pinned)",
                o.name
            ));
        }
        let text = al_builtins::format::ml_text_in_language(ml, language)
            .map_err(|e| format!("the CaptionML of enum {} {e}", o.name))?;
        out.push((*v, text));
    }
    out.extend(o.enum_values.iter().cloned());
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::{ViewClauses, property_view, view_clauses};

    #[test]
    fn the_clauses_of_a_view_come_from_its_parts_not_from_the_words_in_its_names_and_values() {
        let c = |sorting, order, filters| ViewClauses {
            sorting,
            order,
            filters,
        };
        assert_eq!(view_clauses(""), c(false, false, false));
        assert_eq!(
            view_clauses("sorting(Seq) where(\"Order No.\" = filter(<>''))"),
            c(true, false, true)
        );
        assert_eq!(
            view_clauses("where(\"Sorting Group\" = const(1))"),
            c(false, false, true)
        );
        assert_eq!(
            view_clauses("where(Txt = filter('order(x) sorting(y)'))"),
            c(false, false, true)
        );
        assert_eq!(view_clauses("sorting(\"Where Used\")"), c(true, false, false));
        assert_eq!(view_clauses("SORTING(Field1) ORDER(Descending)"), c(true, true, false));
        assert_eq!(
            view_clauses("VERSION(1) Sorting (Field2) Where(Field1=1(2))"),
            c(true, false, true)
        );
        // A view that does not read as parts counts as having every clause.
        assert_eq!(view_clauses("sorting(Seq"), c(true, true, true));
        assert_eq!(view_clauses("garbage(1)"), c(true, true, true));
    }

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
