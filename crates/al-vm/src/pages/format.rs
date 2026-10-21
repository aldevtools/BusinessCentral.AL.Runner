use super::*;

impl Session {
    /// The value a control shows: its binding, read now.
    pub(super) fn control_value(&mut self, st: &St, idx: usize) -> Result<Value, Signal> {
        let engine = self.engine();
        match page_of(&engine, st).controls[idx].binding.clone() {
            ir::Binding::Field(pos) => {
                if st.borrow().stale {
                    return unsupported(
                        "reading a row after CurrPage.Update on a page with OnFindRecord (the row it settles on is not pinned)",
                    );
                }
                let v = self.page_rec(st).borrow().fields[pos].clone();
                // A column the platform row leaves unknown is not shown.
                if let Some(why) = v.unknown() {
                    return unsupported(why.to_string());
                }
                Ok(v)
            }
            b @ (ir::Binding::Var(_) | ir::Binding::BaseVar(_)) => {
                let (slot, owner) = b
                    .var_slot(page_of(&engine, st).controls[idx].owner)
                    .expect("a variable binding");
                // A control of a list's repeater over a page variable: BC's client shows the value the variable had when
                // OnAfterGetRecord ran for the row it stands on (each row keeps its own); the runtime keeps one value. A
                // read on a row other than the one OnAfterGetRecord ran with last is refused (per-row values not
                // modelled; Tests-ERM 134760 Empl. Ledger Entries Preview AmountFCY after First()). A row OnAfterGetRecord
                // never ran with (a new row the test typed) shows what the variable holds.
                if self.in_repeater(st, idx)
                    && self.declares(st, "onaftergetrecord")
                    && st.borrow().cursor == Cursor::Row
                    && !st.borrow().draft
                {
                    let rec = self.page_rec(st);
                    let key = self.row_key(&rec)?;
                    let s = st.borrow();
                    let ran_for_it = s.agr_rows.iter().any(|k| Self::same_key(k, &key));
                    let last = s.agr_row.as_ref().is_some_and(|k| Self::same_key(k, &key));
                    drop(s);
                    // A row other than the one OnAfterGetRecord ran with last shows what the variable held for it (pages/
                    // lt-qi-row-page-var/QiRowPageVar.al:312 QI_RV_Part_Navigation `after-action = 2 | agr 2 | cur 2`); the row
                    // OnAfterGetRecord ran with last shows the variable as it is, as before.
                    if ran_for_it && !last {
                        let s = st.borrow();
                        let shown = s
                            .row_vars
                            .iter()
                            .find(|(k, _)| Self::same_key(k, &key))
                            .and_then(|(_, v)| v.iter().find(|(c, _)| *c == idx).map(|(_, v)| v.clone()));
                        drop(s);
                        return match shown {
                            Some(v) => Ok(v),
                            None => unsupported(
                                "a page-variable control of a list row other than the one OnAfterGetRecord ran with last, with no value noted for that row (not pinned)",
                            ),
                        };
                    }
                }
                let inst = self.owner_instance(st, owner);
                let v = inst.borrow().globals[slot].clone();
                Ok(v)
            }
            ir::Binding::Expr(r) => self.control_routine(st, idx, r),
            // A label shows its caption (handlers/TestPageLabelControlContracts.al:Label_IsATestPageMember_WithItsCaption).
            ir::Binding::Caption => Ok(Value::Text(self.control_caption(st, idx)?.into())),
            ir::Binding::None => unsupported("a control without a source"),
        }
    }

    /// Whether control `idx` sits inside a repeater of its page.
    pub(super) fn in_repeater(&self, st: &St, idx: usize) -> bool {
        let engine = self.engine();
        let page = page_of(&engine, st);
        let mut at = page.controls[idx].parent;
        while let Some(p) = at {
            if page.controls[p].kind == ir::ControlKind::Repeater {
                return true;
            }
            at = page.controls[p].parent;
        }
        false
    }

    pub(super) fn control_as(&mut self, st: &St, idx: usize, to: &RtType) -> Result<Value, Signal> {
        if self.shows_no_row(st, idx) {
            return Ok(Value::default_for(to));
        }
        let v = self.control_value(st, idx)?;
        // AsDecimal of a Text control that holds no number: '', ' ' and a word raise AL:Format naming the text
        // (pages/lt-ca-odd-667/CaTestPageTests.al:90-91, :130-135). A text with digits that does not convert is not pinned.
        if let (RtType::Decimal, Value::Text(t)) = (to, &v) {
            let t = t.to_string();
            return match ops::convert(v, to) {
                Ok(d) => Ok(d),
                Err(_) if !t.chars().any(|c| c.is_ascii_digit()) => Err(error_code(
                    format!("The input string '{t}' was not in a correct format."),
                    "AL:Format",
                )),
                Err(_) => {
                    unsupported("AsDecimal of a Text control whose text has digits but is no number (not pinned)")
                }
            };
        }
        ops::convert(v, to)
    }

    /// A control bound to the page's record reads blank when the page shows no row: whatever its
    /// type, and on a field with an InitValue too (corpus testpart/BnrBlankPart.al:1-30).
    pub(super) fn shows_no_row(&self, st: &St, idx: usize) -> bool {
        let engine = self.engine();
        st.borrow().cursor == Cursor::None
            && matches!(page_of(&engine, st).controls[idx].binding, ir::Binding::Field(_))
    }

    /// The source field of a control bound to a record field.
    pub(super) fn control_field(&self, st: &St, idx: usize) -> Option<ir::Field> {
        let engine = self.engine();
        if !self.has_source(st) {
            return None;
        }
        let page = page_of(&engine, st);
        let ir::Binding::Field(pos) = page.controls[idx].binding else {
            return None;
        };
        let table = self.page_rec(st).borrow().table as usize;
        engine.program.objects[table]
            .table
            .as_ref()
            .map(|t| t.fields[pos].clone())
    }

    /// How a control spells its value: the control's own properties, then the source field's.
    pub(super) fn control_format(&mut self, st: &St, idx: usize) -> Result<ControlFormat, Signal> {
        let engine = self.engine();
        let page = page_of(&engine, st);
        let ctl = &page.controls[idx];
        let field = self.control_field(st, idx);
        let from_field = |n: &str| {
            field.as_ref().and_then(|f| {
                f.properties
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case(n))
                    .map(|(_, v)| v.clone())
            })
        };
        let lit = |n: &str| match ctl.prop(n) {
            Some(ir::PropVal::Lit(v)) => Some(v.clone()),
            _ => None,
        };
        // A table field's AutoFormatExpression that is an expression, read through a control that names no expression of its
        // own: a field of the row is evaluated with the control's row (types/lt-auto-format-currency/QiTableFieldAutoFormat.al;
        // control expressions: autoformat/PWAutoFormatTypes.al:19-22, AutoFormatCard.Page.al:53; literal field expressions:
        // pages/lt-page-autoformat-lookups/P113AutoFormat.al). A procedure call and any other expression are refused.
        // `field_expression` is its text.
        let mut field_expression = None;
        if ctl.prop("autoformatexpression").is_none() {
            if let Some(f) = field.as_ref() {
                let text = from_field("autoformatexpression");
                if text.as_deref().is_some_and(|e| !is_text_literal(e)) {
                    // BC runs an expression that calls a procedure when the page fetches the row (open, Next,
                    // GoToRecord), not when the text is read (types/lt-auto-format-currency/QiTableFieldAutoFormat.al:421
                    // QI_Type10Procedure_Card_Calls `after-open` fmt();fmt(A);, `after-t10-read` empty). Evaluated on read,
                    // its side effects and errors would come at the wrong time, so it is refused.
                    if text.as_deref().is_some_and(|e| e.contains('(')) {
                        return unsupported(format!(
                            "a control over table field {} whose AutoFormatExpression calls a procedure (BC runs it when the row is fetched; not modelled)",
                            f.name
                        ));
                    }
                    if let Some(why) = f.auto_format_refusal.clone() {
                        return unsupported(why);
                    }
                    let Some((object, routine)) = f.auto_format_expression else {
                        return unsupported(
                            "a control over a table field whose AutoFormatExpression is an expression that did not compile",
                        );
                    };
                    // A field of the row (`Rec."Currency Code"`, `"Currency Code"`) is pinned on a Card and a List, read on every
                    // row it moves to (QiTableFieldAutoFormat.al:272 QI_TableFieldExpression_Card_ShowCurrencyNever, :300
                    // QI_TableFieldExpression_List_ShowCurrencyNever: First, Next, Previous). Any other expression is not.
                    if !is_field_reference(text.as_deref().unwrap_or_default()) {
                        return unsupported(format!(
                            "a control over table field {} whose AutoFormatExpression is neither a field of the row nor a text (not pinned)",
                            f.name
                        ));
                    }
                    let rec = self.page_rec(st);
                    field_expression = match self.call_on_record(object, routine, rec, None, Vec::new())?.0 {
                        Value::Text(t) | Value::Code(t) => Some(t.into_string()),
                        other => {
                            return unsupported(format!(
                                "the AutoFormatExpression of field {} gave a {}",
                                f.name,
                                other.describe()
                            ));
                        }
                    };
                }
            }
        }
        // The field's AutoFormatExpression: what its expression gave, else its literal text.
        let field_afe = field_expression.or_else(|| from_field("autoformatexpression").map(|e| unquote(&e)));
        // The precision of the table field's AutoFormatExpression beats the control's DecimalPlaces when the control names no
        // expression of its own (pages/lt-page-autoformat-lookups/P113AutoFormat.al: PDec `DecimalPlaces = 1 : 1` over a field
        // with `<Precision,3:3>` shows three places).
        let field_precision = if ctl.prop("autoformatexpression").is_none() {
            field_afe.as_deref().and_then(pagefmt::parse_precision)
        } else {
            None
        };
        let mut f = ControlFormat {
            decimal_places: field_precision.or_else(|| {
                lit("decimalplaces")
                    .or_else(|| from_field("decimalplaces"))
                    .and_then(|p| pagefmt::parse_decimal_places(&p))
            }),
            ..Default::default()
        };
        // An auto-format expression carries a precision: `<Precision,4:4><Standard Format,0>`
        // (corpus autoformat/TestAutoFormat.al, TestAutoFormatExpressionTiming.al).
        if f.decimal_places.is_none() {
            let expr = match ctl.prop("autoformatexpression") {
                Some(ir::PropVal::Lit(v)) => Some(unquote(v)),
                Some(routine) => match self.prop_routine_value(st, idx, routine)? {
                    Some(Value::Text(t) | Value::Code(t)) => Some(t.into_string()),
                    _ => None,
                },
                None => None,
            };
            f.decimal_places = expr.as_deref().and_then(pagefmt::parse_precision);
            // The table field's own expression applies when the control has none (pages/lt-page-autoformat-lookups/
            // P113AutoFormat.al: T10 `<Precision,3:3>` shows 1,234.500, PovT0 with type 0 keeps the field's 3 places).
            if f.decimal_places.is_none() && expr.is_none() {
                f.decimal_places = field_afe.as_deref().and_then(pagefmt::parse_precision);
            }
            // AutoFormatType 1, 2, 4 and 5 (on the control, else on the table field): the places of an
            // amount or a unit amount, from the currency the expression names
            // (autoformat/PWAutoFormatTypes.al, see `auto_format_places`).
            if f.decimal_places.is_none() {
                let kind = lit("autoformattype").or_else(|| from_field("autoformattype"));
                if let Some(kind) = kind
                    .as_deref()
                    .map(str::trim)
                    .filter(|k| matches!(*k, "1" | "2" | "4" | "5"))
                {
                    let expr = expr.or_else(|| field_afe.clone()).unwrap_or_default();
                    f.decimal_places = Some(self.auto_format_places(kind, &expr)?);
                }
            }
        }
        // ClosingDates on the control (pages/lt-page-presentation-properties/P119ReportFields.al:174,
        // pageext/lt-z-modify-controls/ZModifyControls.al:290) or on the source field (record/lt-x-table-props/
        // XTablePropsTests.al:523-524). A control that turns it off over a field that turns it on is not pinned.
        let closing_ctl = lit("closingdates").map(|v| is_true(&v));
        let closing_field = from_field("closingdates").is_some_and(|v| is_true(&v));
        if closing_ctl == Some(false) && closing_field {
            if let Value::Date(_, true) = self.control_value(st, idx)? {
                return unsupported("ClosingDates = false on a control whose field sets ClosingDates (not pinned)");
            }
        }
        f.closing_dates = closing_ctl.unwrap_or(closing_field);
        f.blank_zero = lit("blankzero")
            .or_else(|| from_field("blankzero"))
            .is_some_and(|v| is_true(&v));
        // A Decimal FlowFilter field shows nothing while it holds zero (handlers/N582TestPageEdges.al:400,
        // BR_Editable_FlowFilterControl `card edit amtfilter value` ''; :428 after SetValue('7') it shows 7.00). Pinned for the
        // unset field only; SetValue('0') and other number types are docs/probes-needed.md.
        if field.as_ref().is_some_and(|f| {
            matches!(f.ty, RtType::Decimal)
                && f.properties
                    .iter()
                    .any(|(k, v)| k.eq_ignore_ascii_case("FieldClass") && v.trim().eq_ignore_ascii_case("FlowFilter"))
        }) {
            if st.borrow().set_controls.contains(&idx) {
                // Only the unset field is pinned: after an explicit SetValue a zero's text is not.
                if matches!(self.control_value(st, idx)?, Value::Decimal(d) if d.is_zero()) {
                    return unsupported(
                        "the text of a Decimal FlowFilter control after SetValue gave it zero (not pinned)",
                    );
                }
            } else {
                f.blank_zero = true;
            }
        }
        if let Some(b) = lit("blanknumbers").or_else(|| from_field("blanknumbers")) {
            f.blank = Blank::parse(&b).unwrap_or_default();
        }
        f.option_captions = lit("optioncaption").map(|c| unquote(&c).split(',').map(str::to_string).collect());
        // OptionCaptionML on the control, else on the table field: the captions of the language the page opened in, over the
        // table field's OptionCaption. A page opened in 1033 keeps One, Two, Three after GlobalLanguage(1031) and (1036)
        // (pages/lt-page-controls-aa/AaFieldProps.al:AA_OptionCaptionML_PageField line 179); one opened in 1031 shows Zwei
        // (pageext/lt-z-modify-controls/ZModifyControls.al:322-360 on the control, record/lt-x-table-props/
        // XTablePropsTests.al:229-230 on the field); 1036 falls back to ENU. Other languages are refused.
        if f.option_captions.is_none() {
            if let Some(ml) = lit("optioncaptionml").or_else(|| from_field("optioncaptionml")) {
                let Some(language) = st.borrow().open_language else {
                    return unsupported("OptionCaptionML on a page that has not opened");
                };
                let list = al_builtins::format::ml_text_in_language(&ml, language)
                    .map_err(|e| Signal::Unsupported(format!("OptionCaptionML on a page field {e}")))?;
                f.option_captions = Some(list.split(',').map(str::to_string).collect());
            }
        }
        if f.option_captions.is_none() {
            f.option_captions =
                from_field("optioncaption").map(|c| unquote(&c).split(',').map(str::to_string).collect());
        }
        if f.option_captions.is_none() {
            // Without captions an option shows its member names, a blank member as nothing.
            f.option_captions = from_field("optionmembers").map(|c| {
                unquote(&c)
                    .split(',')
                    .map(|m| m.trim().trim_matches('"').to_string())
                    .collect()
            });
        }
        Ok(f)
    }

    /// The option list of an Enum control where it is not the enum's captions one by one, with whether it is the control's own
    /// OptionCaption. The client keeps one comma-separated list and the value at position i of the enum (declaration order) stands
    /// for entry i:
    /// - the control's OptionCaption: on an enum with ordinals 0, 1, 4, 5 and `' ,c1,,,c4,c5'` the values show ` `, `c1`, `4`, `5`
    ///   (an empty entry shows the ordinal) and SetValue takes ` ` and `c1` only (handlers/TestPageEnumField_Tests.al:197-205,
    ///   :235-246);
    /// - the enum's captions joined, when one holds a comma: `Diamond Level, with bonus` is two entries, so `with bonus` selects the
    ///   value at position 2 and shows as ` with bonus` (misc/lt-cv-odd-leads/CvEnumCaptionComma.al:150-174).
    ///
    /// Both are pinned on a control bound to a table field; OptionCaptionML or a table field's OptionCaption on an Enum, and a control
    /// bound to a variable, are refused. `Ok(None)`: the captions one by one.
    pub(super) fn enum_option_list(&self, st: &St, idx: usize, id: u32) -> Result<Option<(Vec<String>, bool)>, Signal> {
        let engine = self.engine();
        let ctl = &page_of(&engine, st).controls[idx];
        let field = self.control_field(st, idx);
        let field_has = |n: &str| {
            field
                .as_ref()
                .is_some_and(|f| f.properties.iter().any(|(k, _)| k.eq_ignore_ascii_case(n)))
        };
        if ctl.prop("optioncaptionml").is_some() || field_has("optioncaptionml") || field_has("optioncaption") {
            return unsupported("OptionCaptionML, or a table field's OptionCaption, on an Enum control (not pinned)");
        }
        let rec_bound = matches!(ctl.binding, ir::Binding::Field(_));
        if let Some(prop) = ctl.prop("optioncaption") {
            let ir::PropVal::Lit(v) = prop else {
                return unsupported("an OptionCaption that is not a literal on an Enum control");
            };
            if !rec_bound {
                return unsupported("OptionCaption on an Enum control bound to a variable (not pinned)");
            }
            return Ok(Some((unquote(v).split(',').map(str::to_string).collect(), true)));
        }
        let o = &engine.program.objects[id as usize];
        let shown: Vec<&str> = o
            .enum_values
            .iter()
            .map(|(v, name)| {
                o.enum_captions
                    .iter()
                    .find(|(c, _)| c == v)
                    .map_or(name.as_str(), |(_, c)| c.as_str())
            })
            .collect();
        if !shown.iter().any(|c| c.contains(',')) {
            return Ok(None);
        }
        if !rec_bound {
            return unsupported("an Enum control bound to a variable whose captions hold a comma (not pinned)");
        }
        Ok(Some((shown.join(",").split(',').map(str::to_string).collect(), false)))
    }

    /// The text an Enum control with an option list ([`Self::enum_option_list`]) shows for `ordinal`.
    fn enum_list_text(&self, id: u32, ordinal: i32, list: &[String], from_control: bool) -> Result<String, Signal> {
        let engine = self.engine();
        let at = engine.program.objects[id as usize]
            .enum_values
            .iter()
            .position(|(v, _)| *v == ordinal);
        match at.and_then(|p| list.get(p)) {
            // An empty OptionCaption entry shows the ordinal (handlers/TestPageEnumField_Tests.al:204-205).
            Some(e) if e.is_empty() && from_control => Ok(ordinal.to_string()),
            Some(e) if e.is_empty() => unsupported("an empty entry of an Enum's comma-split captions (not pinned)"),
            Some(e) => Ok(e.clone()),
            None => unsupported("an Enum value past the end of its control's option list (not pinned)"),
        }
    }

    /// The places a decimal control with `AutoFormatType` 1, 2, 4 or 5 shows (corpus autoformat/PWAutoFormatTypes.al,
    /// BC 27.5; the setup does not matter in any of the three setups probed, so General Ledger Setup is not read):
    /// 1 and 4 are amounts, 2 and 5 unit amounts. A blank expression gives 2:2 for an amount and 2:5 for a unit
    /// amount (:98-175). A currency code that exists gives the currency's "Amount Decimal Places" or
    /// "Unit-Amount Decimal Places" (:177-195), both empty gives 2:2 (:197-217), and a code that is no
    /// currency gives the places of General Ledger Setup: 2:5 for all four under a setup of 2:5 and 2:5 (:263-290), 2:2 for
    /// an amount and 2:5 for a unit amount under 2:2 and 2:5 (types/lt-auto-format-currency/QiTableFieldAutoFormat.al:272
    /// `nocur_t1` 1,234.57, `nocur_t2` 1,234.56789). A currency with only one of the two places empty, and a code that is
    /// no currency without a General Ledger Setup row or with its places empty, are not pinned.
    /// `Format` in code of the field numbered `field_no` of `record`, a Decimal table field whose AutoFormatExpression is an expression: BC
    /// applies the field's AutoFormat outside a page too (types/lt-auto-format-currency/QiTableFieldAutoFormat.al:400-418
    /// QI_TableFieldExpression_FormatInCode: a field of the row for types 1 and 2, the places of its currency; a procedure for
    /// type 10, the precision of the format it answers). The lowering sends only those shapes here.
    pub(crate) fn auto_format_field_places_of(&mut self, record: &Value, field_no: usize) -> Result<String, Signal> {
        let Value::Record(rec) = record else {
            return unsupported("Format of an AutoFormat field of something that is no record");
        };
        let engine = self.engine();
        let table = rec.borrow().table as usize;
        let Some(f) = engine.program.objects[table]
            .table
            .as_ref()
            .and_then(|t| t.fields.iter().find(|f| f.id as usize == field_no))
            .cloned()
        else {
            return unsupported("Format of an AutoFormat field the table does not have");
        };
        if let Some(why) = f.auto_format_refusal.clone() {
            return unsupported(why);
        }
        let Some((object, routine)) = f.auto_format_expression else {
            return unsupported(format!(
                "Format of field {} whose AutoFormatExpression is an expression that did not compile",
                f.name
            ));
        };
        let kind = f
            .properties
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("AutoFormatType"))
            .map(|(_, v)| v.trim().to_string())
            .unwrap_or_default();
        let text = f
            .properties
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("AutoFormatExpression"))
            .map(|(_, v)| v.trim().to_string())
            .unwrap_or_default();
        let pinned = match kind.as_str() {
            "1" | "2" => is_field_reference(&text),
            "10" => text.contains('('),
            _ => false,
        };
        if !pinned {
            return unsupported(format!(
                "Format in code of field {} with AutoFormatType {kind} and the AutoFormatExpression `{text}` (only a field of the row for types 1 and 2 and a procedure for type 10 are pinned)",
                f.name
            ));
        }
        let probe = Rc::new(RefCell::new(rec.borrow().clone()));
        let expr = match self.call_on_record(object, routine, probe, None, Vec::new())?.0 {
            Value::Text(t) | Value::Code(t) => t.into_string(),
            other => {
                return unsupported(format!(
                    "the AutoFormatExpression of field {} gave a {}",
                    f.name,
                    other.describe()
                ));
            }
        };
        let (min, max) = if kind == "10" {
            match pagefmt::parse_precision(&expr) {
                Some(p) => p,
                None => {
                    return unsupported(format!(
                        "Format of field {} whose AutoFormatExpression of type 10 gave `{expr}` (only a precision is pinned)",
                        f.name
                    ));
                }
            }
        } else {
            self.auto_format_places(&kind, &expr)?
        };
        Ok(format!("{min}:{max}"))
    }

    pub(super) fn auto_format_places(&mut self, kind: &str, expr: &str) -> Result<(u32, u32), Signal> {
        let unit = matches!(kind, "2" | "5");
        if expr.trim().is_empty() {
            return Ok(if unit { (2, 5) } else { (2, 2) });
        }
        let engine = self.engine();
        let Some(oi) = engine
            .program
            .objects
            .iter()
            .position(|o| o.id == Some(4) && o.table.is_some())
        else {
            return unsupported(format!(
                "AutoFormatType {kind} with the currency `{expr}`: the Currency table is not in the program (not pinned)"
            ));
        };
        let Some(t) = engine.tables[oi].clone() else {
            return unsupported("AutoFormatType with a currency: the Currency table did not compile");
        };
        let field = |name: &str| t.fields.iter().position(|f| f.name.eq_ignore_ascii_case(name));
        let (Some(amount_at), Some(unit_at)) = (field("Amount Decimal Places"), field("Unit-Amount Decimal Places"))
        else {
            return unsupported("AutoFormatType with a currency: the Currency table has no decimal places fields");
        };
        let row = match ops::convert(Value::Text(expr.to_string().into()), &t.fields[0].ty) {
            Ok(key) => self
                .store
                .get(&self.cx, &t, &[key])
                .map_err(|e| Signal::Unsupported(format!("store error {e}")))?,
            Err(_) => None,
        };
        let Some(row) = row else {
            return self.setup_places(unit, expr);
        };
        let text = |at: usize| {
            al_builtins::native::text_of(&row[at])
                .unwrap_or_default()
                .trim()
                .to_string()
        };
        let (amount, unit_places) = (text(amount_at), text(unit_at));
        if amount.is_empty() && unit_places.is_empty() {
            return Ok((2, 2));
        }
        let wanted = if unit { unit_places } else { amount };
        match pagefmt::parse_decimal_places(&wanted) {
            Some(p) if !wanted.is_empty() => Ok(p),
            _ => unsupported(format!(
                "AutoFormatType {kind} with the currency `{expr}`, whose places are `{wanted}` (one of the two empty is not pinned)"
            )),
        }
    }

    /// The places of General Ledger Setup (table 98) for an amount or a unit amount whose currency code `expr` is no
    /// currency (see [`Self::auto_format_places`]). A missing table, row or empty places are refused.
    fn setup_places(&mut self, unit: bool, expr: &str) -> Result<(u32, u32), Signal> {
        let refused = |why: &str| {
            unsupported(format!(
                "AutoFormatType with the currency `{expr}`, which is no currency: {why} (not pinned)"
            ))
        };
        let engine = self.engine();
        let Some(oi) = engine
            .program
            .objects
            .iter()
            .position(|o| o.id == Some(98) && o.table.is_some())
        else {
            return refused("the General Ledger Setup table is not in the program");
        };
        let Some(t) = engine.tables[oi].clone() else {
            return refused("the General Ledger Setup table did not compile");
        };
        let name = if unit {
            "Unit-Amount Decimal Places"
        } else {
            "Amount Decimal Places"
        };
        let Some(at) = t.fields.iter().position(|f| f.name.eq_ignore_ascii_case(name)) else {
            return refused("General Ledger Setup has no decimal places fields");
        };
        let Ok(key) = ops::convert(Value::Text(String::new().into()), &t.fields[0].ty) else {
            return refused("the key of General Ledger Setup is not a text");
        };
        let row = self
            .store
            .get(&self.cx, &t, &[key])
            .map_err(|e| Signal::Unsupported(format!("store error {e}")))?;
        let Some(row) = row else {
            return refused("General Ledger Setup has no row");
        };
        let wanted = al_builtins::native::text_of(&row[at])
            .unwrap_or_default()
            .trim()
            .to_string();
        match pagefmt::parse_decimal_places(&wanted) {
            Some(p) if !wanted.is_empty() => Ok(p),
            _ => refused(&format!("General Ledger Setup's {name} is `{wanted}`")),
        }
    }

    pub(super) fn control_text(&mut self, st: &St, idx: usize, v: &Value) -> Result<String, Signal> {
        // A Media control shows the id of its media, a MediaSet control the id of the first media
        // of its set; lowercase, no braces, empty when there is none
        // (handlers/TestPageFieldFormats_Tests.al:28, :628-640, TestPageFieldFormatsMediaFilled_Tests.al).
        if let (Some(RtType::Library(n)), Value::Guid(id)) = (self.control_type(st, idx, v), v) {
            let shown = if n.eq_ignore_ascii_case("Media") {
                Some(*id)
            } else if n.eq_ignore_ascii_case("MediaSet") {
                self.media.sets.borrow().get(id).and_then(|m| m.first().copied())
            } else {
                None
            };
            if n.eq_ignore_ascii_case("Media") || n.eq_ignore_ascii_case("MediaSet") {
                return Ok(match shown {
                    Some(g) if g != [0; 16] => al_builtins::pagefmt::guid_text(&g)
                        .trim_matches(['{', '}'])
                        .to_lowercase(),
                    _ => String::new(),
                });
            }
        }
        if let Value::Library(name, _) = v {
            return unsupported(format!("control text of a {name} value"));
        }
        // A control with HideValue = true shows no value (handlers/TestProbePageHandlerContracts.al:
        // TestPage_Members_ShowMandatoryHideValue_Defaults, `hidevalue_field_value` is empty).
        {
            let engine = self.engine();
            // A label keeps showing its caption (pages/lt-page-presentation-properties/P119LabelHideValue.al:P119_HideValue `label-value`).
            let is_label = matches!(page_of(&engine, st).controls[idx].binding, ir::Binding::Caption);
            // HideValue may name a variable: `HideValue = HideName` set in OnAfterGetRecord hides the value of that row
            // (pages/lt-page-presentation-properties/P119LabelHideValue.al:P119_HideValue `name-value`).
            if !is_label
                && page_of(&engine, st).controls[idx].prop("hidevalue").is_some()
                && self.prop_bool_now(st, idx, "hidevalue", false)?
            {
                return Ok(String::new());
            }
        }
        let f = self.control_format(st, idx)?;
        let engine = self.engine();
        let caption = move |id: u32, ordinal: i32| {
            let o = &engine.program.objects[id as usize];
            o.enum_captions
                .iter()
                .find(|(v, _)| *v == ordinal)
                .or_else(|| o.enum_values.iter().find(|(v, _)| *v == ordinal))
                .map(|(_, c)| c.clone())
                .unwrap_or_else(|| ordinal.to_string())
        };
        let mut listed = None;
        if let Value::Enum(id, ordinal) = v {
            self.note_enum(*id);
            if let Some((list, from_control)) = self.enum_option_list(st, idx, *id)? {
                listed = Some(self.enum_list_text(*id, *ordinal, &list, from_control)?);
            }
        }
        let mut text = match listed {
            Some(t) => t,
            None => pagefmt::control_text(v, &f, &caption),
        };
        // ExtendedDatatype = Ratio shows a percentage: zero is `0%` (pages/lt-page-presentation-properties/P119PageFields.al:P119_FieldState); another
        // value is not pinned.
        {
            let ext = self
                .control_field(st, idx)
                .and_then(|fl| {
                    fl.properties
                        .iter()
                        .find(|(k, _)| k.eq_ignore_ascii_case("ExtendedDatatype"))
                        .map(|(_, v)| v.clone())
                })
                .or_else(
                    || match page_of(&self.engine(), st).controls[idx].prop("extendeddatatype") {
                        Some(ir::PropVal::Lit(l)) => Some(l.clone()),
                        _ => None,
                    },
                );
            // ExtendedDatatype = Masked shows ten bullets for a text of 11 characters (pages/lt-page-presentation-properties/
            // P119PageFields.al:P119_SetValues `secret`). Whether the count is fixed or the length capped at ten is not pinned,
            // so a shorter text is refused; the empty text shows empty (`secret-value`).
            if ext.as_ref().is_some_and(|e| e.trim().eq_ignore_ascii_case("masked")) {
                if let Value::Text(t) | Value::Code(t) = v {
                    let n = t.chars().count();
                    if n >= 10 {
                        text = "\u{2022}".repeat(10);
                    } else if n > 0 {
                        return unsupported(
                            "a Masked control that shows a text shorter than ten characters (not pinned)",
                        );
                    }
                }
            }
            if ext.is_some_and(|e| e.trim().eq_ignore_ascii_case("ratio")) {
                match v {
                    Value::Decimal(d) if d.is_zero() => text = "0%".to_string(),
                    Value::Decimal(_) => {
                        return unsupported("a Ratio control that shows a value other than zero (not pinned)");
                    }
                    _ => {}
                }
            }
        }
        // A DateTime control writes a plain space before AM / PM on every version (the corpus re-pinned it for the newer
        // service image: pages/lt-page-runtime/P82TestField.al `dt-value`, pages/lt-page-run-overloads-testpart/
        // P106TestFieldAsDateTime.al:P106_AsDateTime_Values; the older pin with a narrow no-break space is gone).
        Ok(text)
    }

    /// The type of the value a control holds.
    pub(super) fn control_type(&self, st: &St, idx: usize, v: &Value) -> Option<RtType> {
        if let Some(f) = self.control_field(st, idx) {
            return Some(f.ty);
        }
        let engine = self.engine();
        let ctl = &page_of(&engine, st).controls[idx];
        if let Some((slot, owner)) = ctl.binding.var_slot(ctl.owner) {
            let object = owner.unwrap_or(st.borrow().object as usize);
            return Some(engine.program.objects[object].globals[slot].ty.clone());
        }
        // A control bound to an expression has the type of the value the expression gives now (its routine returns a
        // Variant); only the plain kinds, and only when the expression can be written.
        if matches!(ctl.binding, ir::Binding::Expr(_))
            && ctl.trigger("__sourceset").is_some()
            && (self.has_source(st) || self.is_request_page(st))
        {
            return match v {
                Value::Decimal(_) => Some(RtType::Decimal),
                Value::Integer(_) => Some(RtType::Integer),
                Value::BigInteger(_) => Some(RtType::BigInteger),
                Value::Boolean(_) => Some(RtType::Boolean),
                Value::Date(..) => Some(RtType::Date),
                Value::Text(_) => Some(RtType::Text(None)),
                Value::Code(_) => Some(RtType::Code(None)),
                _ => None,
            };
        }
        None
    }

    /// corpus handlers/TestPageDecimalAssertEquals_Tests.al:85, TestPageVariableControlErrorName.al:89:
    /// `AssertEquals for Field: <control> Expected = '<expected>', Actual = '<actual>'`.
    pub(super) fn control_assert_equals(&mut self, st: &St, idx: usize, args: &mut [Value]) -> Result<Value, Signal> {
        let actual = self.control_value(st, idx)?;
        let actual_text = if self.shows_no_row(st, idx) {
            String::new()
        } else {
            self.control_text(st, idx, &actual)?
        };
        let expected = args.first().cloned().unwrap_or(Value::Null);
        // A text is compared as written; any other value is converted to the field's type and
        // spelled through the control (corpus handlers/TestPageFieldFormats_Tests.al:1-40).
        let expected_text = match &expected {
            Value::Text(t) | Value::Code(t) => t.clone(),
            other => {
                let converted = match self.control_type(st, idx, &actual) {
                    Some(RtType::Text(_) | RtType::Code(_)) if matches!(other, Value::Enum(..)) => {
                        return unsupported(
                            "AssertEquals of an Enum on a text control (how BC writes it is not pinned)",
                        );
                    }
                    // A number on a Text or Code control is spelled and compared with the control's text: an Integer in plain digits
                    // (handlers/N582TestPageEdges.al:195-247: 10, 1234567, -5, -1234; '1,234,567' differs), a Decimal as Format shows it, with
                    // group separators from 1000 on and no trailing zeros (same file :250-330: 1,000, 1,234.5, -1,234.5, 999.99, 1.5, 0), and
                    // 0, 5, 6, BigInteger 5, Decimal 5.5 and 5.0 against '5.5', '5', '05' (pages/lt-testpage-assertequals-option/
                    // TestPageAssertEqualsOption.al:AssertEquals_TextControl_NumbersAndText, AssertEquals_CodeControl_NumbersAndText). A BigInteger
                    // outside 0 to 9 is not pinned. A Boolean goes through ops::convert (pinned in the same tests).
                    Some(RtType::Text(_) | RtType::Code(_))
                        if matches!(other, Value::Integer(_) | Value::BigInteger(_) | Value::Decimal(_)) =>
                    {
                        match other {
                            Value::Integer(n) => Value::Text(n.to_string().into()),
                            Value::BigInteger(n) => {
                                if !(0..10).contains(n) {
                                    al_builtins::assumptions::assume(
                                        al_builtins::assumptions::Assumption::TestPageAssertEqualsIntegerOnTextControl,
                                    );
                                }
                                Value::Text(n.to_string().into())
                            }
                            Value::Decimal(d) => Value::Text(
                                al_builtins::format::try_format(&Value::Decimal(d.normalize()), 0, 0, self)
                                    .map_err(Signal::Unsupported)?
                                    .into(),
                            ),
                            _ => unreachable!(),
                        }
                    }
                    Some(rt) => ops::convert(other.clone(), &rt)?,
                    None => other.clone(),
                };
                self.control_text(st, idx, &converted)?.into()
            }
        };
        // A text on a Code control is compared in upper case: 'abc' against 'ABC' passes
        // (pages/lt-testpage-assertequals-option/TestPageAssertEqualsOption.al:167, :179). The error shows the expected
        // text as written ('Wrong' against an empty Code control, handlers/TestRequestPageControlErrorName.al:281, :397).
        let on_code = matches!(&expected, Value::Text(_) | Value::Code(_))
            && matches!(self.control_type(st, idx, &actual), Some(RtType::Code(_)));
        if on_code && expected_text.to_uppercase() == actual_text {
            return Ok(Value::Null);
        }
        if expected_text != actual_text {
            let engine = self.engine();
            let name = page_of(&engine, st).controls[idx].name.clone();
            // misc/lt-probes-needed-7/PnGTempControl.al:AssertEqualsThroughTheControls and pages/lt-page-runtime/P82TestField.al:
            // the error code is TestAssertEquals.
            return Err(error_code(
                format!("AssertEquals for Field: {name} Expected = '{expected_text}', Actual = '{actual_text}'"),
                "TestAssertEquals",
            ));
        }
        Ok(Value::Null)
    }

    /// The control's caption: its own `CaptionClass` or `Caption`, else the source field's `CaptionClass` or `Caption`,
    /// else its name.
    pub(super) fn control_caption(&mut self, st: &St, idx: usize) -> Result<String, Signal> {
        let engine = self.engine();
        let ctl = &page_of(&engine, st).controls[idx];
        // CaptionClass `3,<text>` is a caption the platform resolves to <text>
        // (corpus autoformat/TestAutoFormatExpressionTiming.al, WorkingCaptionClass_OpensAndReadsItsCaption).
        let class = match ctl.prop("captionclass") {
            Some(ir::PropVal::Lit(c)) => Some(unquote(c)),
            Some(routine) => match self.prop_routine_value(st, idx, routine)? {
                Some(Value::Text(t) | Value::Code(t)) => Some(t.into_string()),
                _ => None,
            },
            None => None,
        };
        if let Some(c) = class {
            // The page resolved it when it filled; a read takes that caption and raises no event
            // (misc/W2CaptionClass.al:113-116 claimed, :133-136 unclaimed, :170-178 an expression).
            let cached = st
                .borrow()
                .control_captions
                .iter()
                .find(|(i, t, _)| *i == idx && *t == c)
                .map(|(_, _, caption)| caption.clone());
            if let Some(caption) = cached {
                return Ok(caption);
            }
            // A class text that changed since the page last filled: whether BC resolves it at the read is not pinned.
            if raises_resolve(&c) && st.borrow().control_captions.iter().any(|(i, _, _)| *i == idx) {
                return unsupported(
                    "Caption() of a CaptionClass expression that changed since the page filled (whether BC resolves it again is not pinned)",
                );
            }
            return self.caption_class_text(st, &c);
        }
        let own_caption = match ctl.prop("caption") {
            Some(ir::PropVal::Lit(c)) => Some(unquote(c)),
            _ => None,
        };
        let field = self.control_field(st, idx);
        let field_prop = |n: &str| {
            field.as_ref().and_then(|f| {
                f.properties
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case(n))
                    .map(|(_, v)| unquote(v))
            })
        };
        // The source field's CaptionClass beats its Caption (pageext/lt-page-ext-properties/TableExtCaptionClass.al:69) and is
        // resolved when the caption is read, not when the page opens (record/lt-x-table-props/XTablePropsTests.al:483-487).
        // Whether a Caption on the control beats it is not pinned.
        if let Some(why) = field.as_ref().and_then(|f| f.caption_class_refusal.clone()) {
            return unsupported(why);
        }
        // A CaptionClass of the source field that is an expression ran when the page filled (`fill_field_caption_expressions`);
        // the read takes that caption and runs nothing (record/lt-qd-captionclass-areas/QdCaptionClassAreas.al:422-481
        // Page_ControlCaptions, `exprId_again`; pageext/lt-page-ext-properties/TableExtCaptionClass.al:124-149).
        if let Some(raw) = field.as_ref().and_then(|f| {
            f.properties
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("captionclass"))
                .map(|(_, v)| v.trim().to_string())
        }) {
            if crate::recmeta::string_literal(&raw).is_none() {
                if own_caption.is_some() {
                    return unsupported(
                        "a control Caption over its source field's CaptionClass (which one BC shows is not pinned)",
                    );
                }
                return match st.borrow().field_captions.iter().find(|(i, _)| *i == idx) {
                    Some((_, caption)) => Ok(caption.clone()),
                    None => unsupported(
                        "Caption() of a control over a field whose CaptionClass is an expression, before the page filled a row",
                    ),
                };
            }
        }
        if let Some(c) = field_prop("captionclass") {
            if own_caption.is_some() {
                return unsupported(
                    "a control Caption over its source field's CaptionClass (which one BC shows is not pinned)",
                );
            }
            if raises_resolve(&c) {
                // One read is pinned (XTablePropsTests.al:483-487); whether a second read raises the event again is not.
                if st.borrow().control_captions.iter().any(|(i, _, _)| *i == idx) {
                    return unsupported(
                        "a second Caption() of a field CaptionClass that raises OnResolveCaptionClass (whether BC raises it again is not pinned)",
                    );
                }
                let caption = self.caption_class_text(st, &c)?;
                st.borrow_mut().control_captions.push((idx, c, caption.clone()));
                return Ok(caption);
            }
            return self.caption_class_text(st, &c);
        }
        if let Some(c) = own_caption {
            return Ok(c);
        }
        if let Some(c) = field_prop("caption") {
            return Ok(c);
        }
        if let Some(f) = field {
            return Ok(f.name);
        }
        Ok(ctl.name.clone())
    }

    /// Resolve a control's CaptionClass as the page fills and keep the caption for `Caption()`. A class text the control
    /// already resolved raises no event again: after a SetValue only the expression whose text changed is resolved
    /// (misc/W2CaptionClass.al:92 at open, :174-178 after SetValue).
    pub(super) fn fill_caption_class(&mut self, st: &St, idx: usize, text: &str) -> Result<(), Signal> {
        if st
            .borrow()
            .control_captions
            .iter()
            .any(|(i, t, _)| *i == idx && t == text)
        {
            // A later fill with the same class text keeps the caption and raises no OnResolveCaptionClass: the session keeps
            // what it translated (types/PSCaptionClass.al:94-115; misc/W2CaptionClass.al:174-178 pins that a changed text is
            // resolved again).
            return Ok(());
        }
        let caption = self.caption_class_text(st, text)?;
        let mut s = st.borrow_mut();
        s.control_captions.retain(|(i, _, _)| *i != idx);
        s.control_captions.push((idx, text.to_string(), caption));
        Ok(())
    }

    /// After a SetValue the client evaluates the CaptionClass expressions again and resolves the ones whose text changed
    /// (misc/W2CaptionClass.al:ExpressionFromPageVariable_FollowsTheVariable, `set_events`).
    pub(super) fn refresh_caption_classes(&mut self, st: &St) -> Result<(), Signal> {
        if !st.borrow().open {
            return Ok(());
        }
        let engine = self.engine();
        let page = page_of(&engine, st);
        for (idx, c) in page.controls.iter().enumerate() {
            let Some(routine @ (ir::PropVal::Routine(_) | ir::PropVal::Ext(..))) = c.prop("captionclass") else {
                continue;
            };
            if let Some(Value::Text(t) | Value::Code(t)) = self.prop_routine_value(st, idx, routine)? {
                if !t.starts_with("3,") {
                    self.fill_caption_class(st, idx, &t)?;
                }
            }
        }
        Ok(())
    }

    /// A boolean control property, evaluated now: a literal, a routine, or `default`.
    pub(super) fn prop_bool_now(&mut self, st: &St, idx: usize, name: &str, default: bool) -> Result<bool, Signal> {
        let engine = self.engine();
        match page_of(&engine, st).controls[idx].prop(name) {
            None => Ok(default),
            Some(ir::PropVal::Lit(v)) => lit_bool(name, v),
            Some(routine) => match self.prop_routine_value(st, idx, routine)? {
                Some(v) => ops::truthy(&v),
                None => Ok(default),
            },
        }
    }

    /// The caption of the page in force: what `CurrPage.Caption := x` set, else its `Caption`
    /// property, else the page name.
    pub(super) fn page_caption(&self, st: &Rc<RefCell<PageState>>) -> String {
        if let Some(c) = &st.borrow().caption {
            return c.clone();
        }
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let o = &engine.program.objects[object];
        o.properties
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Caption"))
            .map(|(_, v)| unquote(v))
            // A page with CaptionML only shows its ENU text (types/lt-caption-before-run/CaptionReadMl.al:P182b_Ml_BeforeRunAfterRunModal,
            // `[LT ML Caption]` before, during and after the run). A CaptionML without an ENU entry keeps the page name.
            .or_else(|| {
                let (_, v) = o.properties.iter().find(|(k, _)| k.eq_ignore_ascii_case("CaptionML"))?;
                let at = v.to_ascii_uppercase().find("ENU")?;
                let rest = v[at + 3..].trim_start().strip_prefix('=')?.trim_start();
                let rest = rest.strip_prefix('\'')?;
                Some(rest[..rest.find('\'')?].to_string())
            })
            .unwrap_or_else(|| o.name.clone())
    }

    /// The caption a TestPage shows: the page caption, then ` - ` and the values of the data caption fields of the row, joined
    /// by ` ∙ `, empty ones left out; a new record has `New - ` in front. A list shows its caption alone. The fields are the
    /// page's `DataCaptionFields` when it names some, else the table's. Pinned by pages/lt-page-autoformat-lookups/
    /// P113DataCaption.al (P113_DC_CardCaption, _CardNewRecord, _PageLevelDataCaptionFields, _EmptyDataCaptionFields, _ListCaption,
    /// _EmptyTable). A `DataCaptionExpression` is refused: the expression is not evaluated here.
    pub(super) fn test_page_caption(&mut self, st: &St) -> Result<String, Signal> {
        let base = self.page_caption(st);
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let has_prop = |name: &str| {
            engine.program.objects[object]
                .properties
                .iter()
                .any(|(k, _)| k.eq_ignore_ascii_case(name))
        };
        // A request page with a source table shows its caption alone on a stored row, whatever DataCaptionFields or
        // DataCaptionExpression say (report/lt-y-request-page/YRequestPageTests.al:84, :157, :206, :241, :255, :365, :387). On a new
        // row it is not pinned.
        if self.is_request_page(st) {
            if st.borrow().draft {
                return unsupported("the caption of a request page on a new row (not pinned)");
            }
            return Ok(base);
        }
        if has_prop("DataCaptionExpression") {
            return unsupported("the caption of a page with DataCaptionExpression (the expression is not evaluated)");
        }
        // A caption the page assigned still gets the record's values behind it (pages/lt-page-runtime/P82CurrPage.al:
        // Caption_ReadAndAssign: `Changed caption - A`); on no row it stands alone (handlers/TestPageCaption_Tests.al).
        if !self.has_source(st) {
            return Ok(base);
        }
        if self.is_list(st) {
            // BC adds the filter on a data caption field to a list's caption (al-runner issue 5593: `Probe Bin List -
            // Z1` on BC 29 for a filter Zone Code = Z1, the bare caption for a filter on another field). No corpus test
            // pins it yet (docs/probes-needed.md "User issues (lane-user-trust)"), so such a caption is refused.
            let rec = self.page_rec(st);
            let table = self.table(&rec)?;
            // The fields the page's or the table's DataCaptionFields name, else the primary key.
            let names: Vec<String> = [object, table.object as usize]
                .iter()
                .filter_map(|&o| {
                    engine.program.objects[o]
                        .properties
                        .iter()
                        .find(|(k, _)| k.eq_ignore_ascii_case("DataCaptionFields"))
                        .map(|(_, v)| v.clone())
                })
                .filter(|v| !v.trim().is_empty())
                .collect();
            let caption_fields: Vec<usize> = if names.is_empty() {
                table.keys.first().cloned().unwrap_or_default()
            } else {
                names
                    .iter()
                    .flat_map(|v| v.split(','))
                    .filter_map(|n| {
                        let n = n.trim().trim_matches('"');
                        table.fields.iter().position(|f| f.name.eq_ignore_ascii_case(n))
                    })
                    .collect()
            };
            if rec.borrow().filters.iter().any(|f| caption_fields.contains(&f.field)) {
                return unsupported(
                    "the caption of a list whose record has a filter on a data caption field (not pinned, al-runner issue 5593)",
                );
            }
            return Ok(base);
        }
        let (cursor, draft) = {
            let s = st.borrow();
            (s.cursor, s.draft || s.mode == PageMode::New)
        };
        if cursor == Cursor::None {
            return Ok(base);
        }
        let rec = self.page_rec(st);
        let table = self.table(&rec)?;
        let names_of = |props: &[(String, String)]| -> Option<String> {
            props
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("DataCaptionFields"))
                .map(|(_, v)| v.clone())
                .filter(|v| !v.trim().is_empty())
        };
        // The page's own DataCaptionFields (empty or naming a field) does not change the fields: the table's are shown
        // (P113_DC_PageLevelDataCaptionFields: `DataCaptionFields = Name` shows `C1 ∙ Alpha`).
        let names = names_of(&engine.program.objects[table.object as usize].properties);
        let mut parts = Vec::new();
        // Without DataCaptionFields the caption shows the primary key (pages/lt-page-runtime/P82RunModal*.al:RunModal_Number_OnCard
        // `LT P82 Card Caption - A`, P117 `LT P117 Card - 1`).
        let names = names.or_else(|| {
            let key: Vec<String> = table
                .keys
                .first()?
                .iter()
                .map(|&i| table.fields[i].name.clone())
                .collect();
            Some(key.join(","))
        });
        if let Some(names) = names {
            for name in names.split(',') {
                let name = name.trim().trim_matches('"');
                let Some(i) = table.fields.iter().position(|f| f.name.eq_ignore_ascii_case(name)) else {
                    return unsupported(format!("DataCaptionFields names `{name}`, not a field of the table"));
                };
                let v = rec.borrow().fields[i].clone();
                if let Some(why) = v.unknown() {
                    return unsupported(why.to_string());
                }
                let text = pagefmt::control_text(&v, &pagefmt::ControlFormat::default(), &|_, o| o.to_string());
                if !text.is_empty() {
                    parts.push(text);
                }
            }
        }
        let mut caption = base;
        if !parts.is_empty() {
            caption = format!("{caption} - {}", parts.join(" \u{2219} "));
        }
        if draft {
            caption = format!("New - {caption}");
        }
        Ok(caption)
    }

    pub(super) fn page_editable(&self, st: &St) -> bool {
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let props = &engine.program.objects[object].properties;
        // A list handed over in lookup mode is read-only, page-variable controls included
        // (corpus handlers/TestPageControlEditableRoutes.al:446 List_RunModal_LookupMode).
        st.borrow().mode != PageMode::View
            && !(st.borrow().editable_off && self.open_lock(st) != Some(PageMode::New))
            && !(st.borrow().lookup && self.is_list_page(st))
            && !props
                .iter()
                .any(|(k, v)| k.eq_ignore_ascii_case("Editable") && v.trim().eq_ignore_ascii_case("false"))
    }

    /// The open mode, when the page is read-only only because its OnOpenPage called `CurrPage.Editable(false)`. What that
    /// does depends on the mode: OpenEdit locks every control (handlers/TestPageControlEditableRoutes.al:553); OpenView
    /// leaves a page-variable control editable (pages/lt-cd-testpage-leads/CdTestPageLeads.al:95, Item49_LockCard_OpenView);
    /// OpenNew keeps the page editable and locks the Rec controls (CdTestPageLeads.al:117-118, Item49_LockCard_OpenNew).
    pub(super) fn open_lock(&self, st: &St) -> Option<PageMode> {
        let s = st.borrow();
        (s.editable_off && s.editable_off_in_open).then_some(s.mode)
    }

    /// Editable and Enabled follow the page state live (corpus
    /// handlers/TestPageControlPropertyLiveness_Tests.al:73, TestPageEditable_Tests.al).
    pub(super) fn control_editable(&mut self, st: &St, idx: usize) -> Result<bool, Signal> {
        // A control bound to a page variable stays editable on a read-only page (corpus
        // handlers/TestPageReadOnlySetValue.al:365 OpenView_PageVariableControl_StaysEditable).
        let engine = self.engine();
        let var_bound = matches!(
            page_of(&engine, st).controls[idx].binding,
            ir::Binding::Var(_) | ir::Binding::BaseVar(_)
        );
        let open_lock = self.open_lock(st);
        if var_bound && open_lock == Some(PageMode::New) {
            return unsupported(
                "Editable() of a page-variable control on a page opened with OpenNew whose OnOpenPage called \
                 CurrPage.Editable(false) (not pinned)",
            );
        }
        let rec_bound = !var_bound
            || (st.borrow().lookup && self.is_list_page(st))
            || (st.borrow().editable_off && open_lock != Some(PageMode::View))
            // A request page that says `Editable = false` makes its variable-bound fields read No too
            // (handlers/PSRequestPageEditable.al Editable_RequestPageEditableFalse).
            || (self.is_request_page(st) && !self.page_editable(st));
        // A label is never editable (handlers/TestPageLabelControlContracts.al). A cuegroup with
        // `Editable = false` makes its fields read No (handlers/TestPageCueGroupFixedGridContracts.al:
        // CueGroup_SetValue_IsSilentlyIgnored).
        if matches!(page_of(&engine, st).controls[idx].binding, ir::Binding::Caption) {
            return Ok(false);
        }
        let mut cue = page_of(&engine, st).controls[idx].parent;
        while let Some(p) = cue {
            // A group (a grid is one) with `Editable = false` does the same (pages/lt-page-editability-width-lookups/P115GridLabel.al:
            // P115_GridAndLabel_Editable `ro-grid-field-editable = No`).
            if matches!(
                page_of(&engine, st).controls[p].kind,
                ir::ControlKind::CueGroup | ir::ControlKind::Group
            ) && !self.prop_bool_now(st, p, "editable", true)?
            {
                return Ok(false);
            }
            cue = page_of(&engine, st).controls[p].parent;
        }
        // A page that opened for edit over an empty view shows no row, and its fields read No (pages/lt-page-types-empty-table/
        // PageTypesEmptyTable.al:Card_OpenEdit_EmptyTable `codeEditable`).
        if st.borrow().blank_on_open
            && st.borrow().cursor == Cursor::None
            && matches!(page_of(&engine, st).controls[idx].binding, ir::Binding::Field(_))
        {
            return Ok(false);
        }
        // ModifyAllowed = false makes the fields of a stored row read-only (pages/lt-page-autoformat-lookups/P113ModifyAllowed.al:
        // P113_MA_CardEditable: the page is editable, its controls are not; a new row, which can be inserted, is).
        if matches!(page_of(&engine, st).controls[idx].binding, ir::Binding::Field(_))
            && st.borrow().cursor == Cursor::Row
            && !st.borrow().draft
            && flag_denied(&engine, st.borrow().object as usize, "ModifyAllowed")?
        {
            return Ok(false);
        }
        // A field whose source table field says `Editable = false` and whose control declares no Editable of its
        // own is not editable (pinned for a stored field: pages/lt-page-field-editable-source/PageFieldEditableSource.al:
        // Card_OpenEdit_FieldEditable `lockedNoProp`; a control with its own Editable = true is editable there
        // `lockedEditableTrue`). A FlowField is pinned too (below); a FlowFilter field is not
        // (assumption page-field-follows-source-field-editable-false).
        if page_of(&engine, st).controls[idx].prop("editable").is_none() {
            if let Some(f) = self.control_field(st, idx) {
                if f.properties
                    .iter()
                    .any(|(k, v)| k.eq_ignore_ascii_case("Editable") && v.trim().eq_ignore_ascii_case("false"))
                {
                    // A FlowField is pinned (handlers/N582TestPageEdges.al:324 N582_Editable_FlowFieldNotEditable: Editable()
                    // No on a card, a list and a view page); a FlowFilter field is the assumption.
                    let flow_field = f.properties.iter().any(|(k, v)| {
                        k.eq_ignore_ascii_case("FieldClass") && v.trim().eq_ignore_ascii_case("FlowField")
                    });
                    if !f.stored && !flow_field {
                        al_builtins::assumptions::assume(
                            al_builtins::assumptions::Assumption::PageFieldFollowsSourceFieldEditableFalse,
                        );
                    }
                    return Ok(false);
                }
            }
        }
        let page_editable = self.page_editable(st) && !(rec_bound && open_lock == Some(PageMode::New));
        Ok((!rec_bound || page_editable) && self.prop_bool_now(st, idx, "editable", true)?)
    }

    /// Whether control `idx` sits inside a `cuegroup`.
    pub(super) fn in_cue_group(&self, st: &St, idx: usize) -> bool {
        let engine = self.engine();
        let page = page_of(&engine, st);
        let mut at = page.controls[idx].parent;
        while let Some(p) = at {
            if page.controls[p].kind == ir::ControlKind::CueGroup {
                return true;
            }
            at = page.controls[p].parent;
        }
        false
    }

    pub(super) fn control_enabled(&mut self, st: &St, idx: usize) -> Result<bool, Signal> {
        // A field in a group with `Enabled = false` reads No on a request page (misc/lt-object-properties-runtime-effect/
        // ReportRequestPageGroupEditable.al:P194_ReportGroup_EditableEnabled, `disabled_enabled = No`); on another page it is not pinned.
        if self.is_request_page(st) {
            let engine = self.engine();
            let mut at = page_of(&engine, st).controls[idx].parent;
            while let Some(p) = at {
                if page_of(&engine, st).controls[p].kind == ir::ControlKind::Group
                    && !self.prop_bool_now(st, p, "enabled", true)?
                {
                    return Ok(false);
                }
                at = page_of(&engine, st).controls[p].parent;
            }
        }
        self.prop_bool_now(st, idx, "enabled", true)
    }

    pub(super) fn control_visible(&mut self, st: &St, idx: usize) -> Result<bool, Signal> {
        let engine = self.engine();
        let page = page_of(&engine, st);
        // Own Visible is the open-time value; a group's and a part's are live (a part: pages/lt-page-filtergroup-assumptions/
        // P99InvisiblePart.al:P99_FlagTrueInOnAfterGetCurrRecord) and so are the groups around the
        // control (corpus handlers/TestPageControlPropertyLiveness_Tests.al:56). An action's is live too: after an action set
        // its variable to false it reads No, with or without CurrPage.Update, while a field keeps its open-time value
        // (pages/lt-cd-testpage-leads/CdTestPageLeads.al:251-253, :267, Item51_VisibleVariables_*).
        let own = if matches!(
            page.controls[idx].kind,
            ir::ControlKind::Group | ir::ControlKind::CueGroup | ir::ControlKind::Part | ir::ControlKind::Action
        ) {
            self.prop_bool_now(st, idx, "visible", true)?
        } else {
            st.borrow().frozen_visible.get(idx).copied().unwrap_or(true)
        };
        let mut visible = own;
        let mut parent = page.controls[idx].parent;
        while let Some(p) = parent {
            visible &= self.prop_bool_now(st, p, "visible", true)?;
            parent = page.controls[p].parent;
        }
        Ok(visible)
    }
}

/// The text of the `ENU = '...'` entry of a `...ML` property value.
/// The class text goes to OnResolveCaptionClass (see `caption_class_text`): not area 3, not a numeric or empty area, and with a comma.
pub(super) fn raises_resolve(text: &str) -> bool {
    match text.split_once(',') {
        Some((area, _)) => area != "3" && !area.is_empty() && area.trim().parse::<u32>().is_err(),
        None => false,
    }
}

/// Whether an AutoFormatExpression is a plain field of the row: `Rec."Currency Code"`, `"Currency Code"`, `Rec.Code`, `Code`.
fn is_field_reference(text: &str) -> bool {
    let t = text.trim();
    let t = match t.get(..4) {
        Some(p) if p.eq_ignore_ascii_case("rec.") => &t[4..],
        _ => t,
    };
    if let Some(inner) = t.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
        return !inner.is_empty() && !inner.contains('"');
    }
    let mut chars = t.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}
