use super::*;
use al_builtins::page_op::{PageMethod, PageOp};
use al_builtins::record_op::RecordOp;

impl Session {
    /// `TestField.SetValue`: activates the control, turns the value into what the control would
    /// have been typed with, validates the field (the table's OnValidate, then the page control's),
    /// and records a refusal in the control's ledger (corpus handlers/TestPageValidationError_Tests.al,
    /// TestPageReadOnlySetValue.al:1-30, TestPageBooleanSpelling_Tests.al).
    pub(super) fn control_set_value(&mut self, st: &St, idx: usize, args: &mut [Value]) -> Result<Value, Signal> {
        if st.borrow().lookup_setting_value {
            st.borrow_mut().lookup_value_set = true;
        }
        // A filter field (a page variable) whose OnValidate refills a temporary Worksheet: the client reads the new rows
        // (`reread_after_refill_by_validate`, RmTempRefill.al:215-:320), on a collapsed tree too (al-runner issue 6550).
        let engine = self.engine();
        let refill_held = (matches!(
            page_of(&engine, st).controls[idx].binding,
            ir::Binding::Var(_) | ir::Binding::BaseVar(_)
        ) && page_of(&engine, st).controls[idx].trigger("onvalidate").is_some()
            && self.temp_refill_shape_or_tree(st))
        .then(|| self.row_key(&self.page_rec(st)))
        .transpose()?;
        // The rows of a tree the client had expanded: putting the cut back after OnValidate may expand the parent of Rec's row.
        let expanded = st.borrow().expanded.clone();
        let r = self.control_set_value_now(st, idx, args);
        if let (Ok(_), Some(held)) = (&r, refill_held) {
            if !st.borrow().refresh && self.temp_refill_shape_or_tree(st) && self.held_rows_changed(st)? {
                self.reread_after_refill_by_validate(st, &held, expanded)?;
            }
        }
        if r.is_ok() && st.borrow().dirty && !st.borrow().draft {
            st.borrow_mut().set_value_held = true;
        }
        // The control now holds an explicit value (see `PageState::set_controls`).
        if !st.borrow().set_controls.contains(&idx) {
            st.borrow_mut().set_controls.push(idx);
        }
        r
    }

    /// The ordinals the control's own ValuesAllowed lets through, or `None` without one. Pinned for an Enum or Option control
    /// whose list names values by name or ordinal (pages/lt-y37-values-allowed/Y37ValuesAllowedSetValue.al: a ConfirmationDialog
    /// field over a global record's Enum field and an Option variable, request page Enum variables, a list's Rec field;
    /// pageext/lt-page-ext-properties/PageFieldValuesAllowed.al, a card's Rec field). Other shapes are refused.
    fn values_allowed(&self, st: &St, idx: usize, ty: &RtType) -> Result<Option<Vec<i32>>, Signal> {
        let engine = self.engine();
        let ctl = &page_of(&engine, st).controls[idx];
        let name = ctl.name.clone();
        let list = match ctl.prop("valuesallowed") {
            None => return Ok(None),
            Some(ir::PropVal::Lit(v)) => v.clone(),
            Some(_) => {
                return unsupported(format!(
                    "SetValue on control {name}, whose ValuesAllowed is computed (not pinned)"
                ));
            }
        };
        let field = self.control_field(st, idx);
        let field_prop = |n: &str| {
            field
                .as_ref()
                .is_some_and(|f| f.properties.iter().any(|(k, _)| k.eq_ignore_ascii_case(n)))
        };
        if field_prop("ValuesAllowed") {
            return unsupported(format!(
                "SetValue on control {name}, with ValuesAllowed over a field with ValuesAllowed (not pinned)"
            ));
        }
        if matches!(ctl.binding, ir::Binding::Field(_)) && self.shows_no_row(st, idx) {
            return unsupported(format!(
                "SetValue on control {name}, which has ValuesAllowed, without a row (not pinned)"
            ));
        }
        let values: Vec<(i32, String)> = match ty {
            RtType::Enum(id) if self.enum_option_list(st, idx, *id)?.is_none() => {
                engine.program.objects[*id as usize].enum_values.clone()
            }
            RtType::Option(members) if ctl.prop("optioncaption").is_none() && !field_prop("OptionCaption") => {
                members.iter().enumerate().map(|(i, m)| (i as i32, m.clone())).collect()
            }
            _ => {
                return unsupported(format!(
                    "SetValue on control {name}, which has ValuesAllowed, of a type other than an Enum or Option without captions (not pinned)"
                ));
            }
        };
        match allowed_ordinals(&list, &values) {
            Some(v) => Ok(Some(v)),
            None => unsupported(format!(
                "SetValue on control {name}, whose ValuesAllowed names no value of its type (not pinned)"
            )),
        }
    }

    fn control_set_value_now(&mut self, st: &St, idx: usize, args: &mut [Value]) -> Result<Value, Signal> {
        // Taken first: a trigger that runs below may call SetValue itself.
        let from_variant = std::mem::take(&mut self.set_value_from_variant);
        // A subscriber to OnBeforeValidateEvent / OnAfterValidateEvent of this control would run in BC; the client does
        // not raise them yet, so a write to such a control is refused (a page that is merely opened is not).
        {
            let engine = self.engine();
            let object = st.borrow().object as usize;
            let name = page_of(&engine, st).controls[idx].name.clone();
            // The element is the control's name or the name of the field it shows.
            let field_name = self.control_field(st, idx).map(|f| f.name.clone());
            let names: Vec<&str> = std::iter::once(name.as_str()).chain(field_name.as_deref()).collect();
            for ev in ["onbeforevalidateevent", "onaftervalidateevent"] {
                // A subscriber of a declared and not compiled app, or one that names the field when the control
                // has another name, or one for every element, would run in BC and is not reached here.
                // Only a control bound to a record field is served: its events carry the page's Rec.
                let record_bound = matches!(page_of(&engine, st).controls[idx].binding, ir::Binding::Field(_));
                let unreached = engine
                    .events
                    .subscribers
                    .get(&(object, ev.to_string()))
                    .is_some_and(|v| {
                        v.iter().any(|s| {
                            s.element.is_empty()
                                || (names.iter().any(|n| s.element.eq_ignore_ascii_case(n))
                                    && (!record_bound || !s.element.eq_ignore_ascii_case(&name)))
                        })
                    });
                if unreached
                    || names
                        .iter()
                        .any(|n| engine.events.missing_for(object, ev, Some(n)).is_some())
                {
                    return unsupported(format!(
                        "page event {ev} of control {name}: a subscriber listens, and the test client does not raise it yet"
                    ));
                }
            }
        }
        // SetValue through a part whose control says Editable = false writes anyway
        // (testpart/TestPartCloseLinkEditableContracts.al:
        // Parts_EnabledReadsTheControl_EditableFalseDoesNotBlockWrites). New() and Next() onto the
        // new-row line of such a part are not pinned and stay refused.
        // A validation through the page starts on fresh table globals, also when the line is saved
        // first because the focus leaves the key control (corpus record/PWTableGlobals.al:392-405,
        // PageInsert_NewThenSetTxtThenClose_ValidationBeforeInsert: OnInsert reads Counter 0 though
        // OnNewRecord set 9).
        if st.borrow().host.is_some() {
            self.save_unsaved_host_first(st)?;
            self.save_sibling_parts(st)?;
            self.insert_blank_after_empty_last(st, idx)?;
        } else if let ir::Binding::Field(pos) = page_of(&self.engine(), st).controls[idx].binding
            && st.borrow().draft
            && self.has_source(st)
        {
            let rec = self.page_rec(st);
            let table = self.table(&rec)?;
            let key = &table.keys[0];
            let key_blank = key.iter().all(|k| {
                let v = &rec.borrow().fields[*k];
                Self::same_values(
                    std::slice::from_ref(v),
                    std::slice::from_ref(&Value::default_for(&table.fields[*k].ty)),
                )
            });
            if !key.contains(&pos) && key_blank {
                st.borrow_mut().typed_before_key = true;
            }
        }
        self.save_before_page_var_validate(st, idx)?;
        if matches!(page_of(&self.engine(), st).controls[idx].binding, ir::Binding::Field(_)) && self.has_source(st) {
            self.reset_table_globals(&self.page_rec(st));
        }
        let writes = self.write_mark();
        let inserted_by_trigger = st.borrow().new_row_inserted_by_trigger;
        let (activated, origin) = self.traced(st, |s| s.activate(st, idx));
        if inserted_by_trigger {
            return Err(self.insert_of_trigger_inserted_row(st, idx, activated)?);
        }
        if let (Err(Signal::Error(e)), Some("oninsertrecord")) = (&activated, origin.as_deref()) {
            return Err(self.insert_trigger_error(st, e, writes)?);
        }
        activated?;
        // Only an Update() requested by this call's triggers refreshes at its end.
        st.borrow_mut().refresh = false;
        let engine = self.engine();
        let page = page_of(&engine, st);
        let ctl = &page.controls[idx];
        let current = self.control_value(st, idx)?;
        // A control bound to an expression that can be written, on a page with no source table: the write itself is pinned
        // (misc/lt-probes-needed-7/PnGTempControl.al, pages/W1Controls.al:424 TempRecordControl_SetValue `set_log`, `c_after`;
        // pages/lt-page-array-path/PageArrayPathSetValue.al:91-200 ArrayElementField_*), with the type of the value it gives now
        // (as `control_type` on a page with a source table; a Decimal or a Text). The page also refreshes after a later action there (AGCR,
        // W1Controls.al:453 `buf_log`), which is not modelled: `action_invoke` refuses an action after such a write.
        let no_source_expr = matches!(ctl.binding, ir::Binding::Expr(_))
            && ctl.trigger("__sourceset").is_some()
            && !self.has_source(st)
            && !self.is_request_page(st);
        // A Decimal (the array element), a Text (the temporary record's field) and an Enum field of a global record on a
        // ConfirmationDialog (pages/lt-y37-values-allowed/Y37ValuesAllowedSetValue.al:179-189, :226) are pinned.
        let ty = self.control_type(st, idx, &current).or(match &current {
            Value::Decimal(_) if no_source_expr => Some(RtType::Decimal),
            Value::Text(_) if no_source_expr => Some(RtType::Text(None)),
            Value::Enum(id, _) if no_source_expr => Some(RtType::Enum(*id)),
            _ => None,
        });
        let Some(ty) = ty else {
            if no_source_expr {
                return unsupported(
                    "SetValue on a control bound to an expression of a page with no source table, whose value is of another kind (not pinned)",
                );
            }
            return unsupported("SetValue on a control whose type is not known");
        };
        if no_source_expr {
            st.borrow_mut().expr_set_no_source = true;
        }
        // The ordinals a control's ValuesAllowed lets through, for the shapes the corpus pins (`values_allowed`).
        let allowed = self.values_allowed(st, idx, &ty)?;
        let rec_bound = matches!(ctl.binding, ir::Binding::Field(_));
        // A SetValue on a host control while one of its parts holds a typed row that is not saved: the part saves that row
        // first, as it does when the host moves (misc/lt-probes-needed-7/PnGPartRows.al:130-163
        // PendingPartRowSavedWhenHostMoves) and when it closes (testpart/TestOkPartRow.al); for a SetValue on a Document host
        // testpart/QiHostSaveParts.al:500, :554 pin it (other hosts: assumption part-row-saved-before-host-set-value).
        self.save_parts_before_host_edit(st)?;
        self.save_host_before_part_edit(st)?;
        // OnBefore/AfterValidate of `modify` wrap the control's OnValidate on a field-bound control (pageextensiontrigger/TestModified
        // ControlValidateTriggers_Tests.al); on a variable-bound request page control OnAfterValidate alone (6651, Y64LocalProcNames.al:445).
        let ext_validate = |name: &str| -> Vec<(usize, usize)> {
            ctl.ext_triggers
                .iter()
                .filter(|(n, _, _)| n == name)
                .map(|(_, e, r)| (*e, *r))
                .collect()
        };
        let (ext_before, ext_after) = (ext_validate("onbeforevalidate"), ext_validate("onaftervalidate"));
        if !rec_bound && unpinned_ext_validate(&ext_before, &ext_after, self.is_request_page(st)) {
            return unsupported("OnBeforeValidate/OnAfterValidate on a control bound to a variable (not pinned)");
        }
        // With a table OnValidate: the table's validation (its events and OnValidate) first, then the page's
        // OnBeforeValidateEvent, OnBeforeValidate, the control's OnValidate, OnAfterValidate, the page's OnAfterValidateEvent
        // (misc/lt-probes-needed-2/PnBPages.al:185 ModifyValidateTriggers_AroundTableOnValidate `Name.OnValidate:2,E1.Name.Before,
        // E1.Name.After`; corpus writer TT's branch al-runtime-rs-staging-tt, pageextensiontrigger/TestModifiedControlValidateTriggers_Tests.al
        // TT_TableOnValidate_Order, TT_TableOnValidate_Order_WithEvents; former assumption `page-ext-validate-table-onvalidate` (pinned on corpus master since bump 1030)).
        // Pinned by pageextensiontrigger/TestModifiedControlValidateTriggers_Tests.al TT_TableOnValidate_Order, TT_TableOnValidate_Order_WithEvents (was assumption `page-ext-validate-table-onvalidate`).
        let caption = self.control_caption(st, idx)?;
        let fmt = al_builtins::pagefmt::ControlFormat {
            work_date: self.work_date_day(),
            today: Some(self.today_day_number()),
            ..self.control_format(st, idx)?
        };
        let mut arg = args.first().cloned().unwrap_or(Value::Null);
        // A Text inside a Variant on an Enum or Option control goes by ordinal digits: '1' selects the value of ordinal 1, as an
        // Integer does; ' ', '' and 'Alpha' fail to evaluate, the control keeps its value and holds no validation error. BC 27.5
        // names NavOption instead (pages/lt-be-enum-setvalue-blank/BeEnumSetValueBlank.al:532-597 table field, Enum with a blank
        // caption; the FStrict, FPlain, FOpt and V* twins). Pinned on a card with a row for digits of a value and for empty,
        // blank or all-letter texts; other texts and other shapes are refused.
        if from_variant && matches!(ty, RtType::Enum(_) | RtType::Option(_)) {
            let variant_text: Option<String> = match &arg {
                Value::Text(t) | Value::Code(t) => Some(t.to_string()),
                Value::Char(c) => char::from_u32(*c as u32).map(String::from),
                _ => None,
            };
            if let Some(t) = variant_text {
                if !(self.has_source(st) && !self.is_list(st) && !self.shows_no_row(st, idx)) {
                    return unsupported(
                        "SetValue of a Text in a Variant on an Enum or Option control of a list or of a page without a row (not pinned)",
                    );
                }
                if !t.is_empty() && t.chars().all(|c| c.is_ascii_digit()) {
                    let n: Option<i32> = t.parse().ok();
                    let is_value = match (&ty, n) {
                        (RtType::Enum(id), Some(n)) => engine.program.objects[*id as usize]
                            .enum_values
                            .iter()
                            .any(|(v, _)| i64::from(*v) == i64::from(n)),
                        (RtType::Option(members), Some(n)) => usize::try_from(n).is_ok_and(|n| n < members.len()),
                        _ => false,
                    };
                    match n {
                        Some(n) if is_value => arg = Value::Integer(n),
                        _ => {
                            return unsupported(
                                "SetValue of a Variant holding the digits of no value on an Enum or Option control (not pinned)",
                            );
                        }
                    }
                } else if t.chars().all(|c| c == ' ') || t.chars().all(|c| c.is_ascii_alphabetic()) {
                    return Err(if self.has(al_version::Gate::ConversionErrorTextsFrom28) {
                        error_code(
                            format!("The value \"{t}\" can't be evaluated into type Option."),
                            "Evaluate:Format",
                        )
                    } else {
                        error_code("The type NavOption is unknown.".to_string(), "NotSupportedType")
                    });
                } else {
                    return unsupported(
                        "SetValue of a Variant holding a Text that is neither digits, blank nor letters on an Enum or Option control (not pinned)",
                    );
                }
            }
        }
        // An Integer on an Enum control selects the value of that ordinal, also when its caption is blank (all spaces), although
        // that caption trims to '' and the empty text is refused: on a card with a row, for a control bound to a table field
        // and one bound to a page variable (pages/lt-be-enum-setvalue-blank/BeEnumSetValueBlank.al:502-528
        // SetValue_FCompat_VariantInts_FromBlank, :635 IntVars, :2366 SetValue_VCompat_VariantInts_FromBlank; controls/
        // W2ValueControls.al:Enum_VariableControl_OtherSpellings `empty`). A variable control on a list, a page without a source
        // table or without a row
        // keeps assumption testpage-setvalue-blank-enum-member (Tests-ERM ERM Budget 134922 and ERM Budget UI 134927 call
        // `Budget.GLAccCategory.SetValue(0)` on page 113); a table-field control there is not pinned and refused.
        // The blank-captioned value itself handed in (a Variant holding it, `member0` at BeEnumSetValueBlank.al:541-597) and a
        // Text of spaces (`space`, :600-611; `litSpace`) select it the same way.
        let pinned_card = self.has_source(st) && !self.is_list(st) && !self.shows_no_row(st, idx);
        // On a list, a table-field control over an Enum field with NotBlank (not the control's) takes a Text of spaces and the
        // Integer 0 for its blank-captioned value " " as a card does, and shows ' ' (BeEnumSetValueBlank.al:4021
        // QZ_NotBlankEnum_SetValueBlank_List, `notblank after space`, `notblank after ordinal 0`). Without NotBlank it is not pinned.
        let pinned_list_not_blank = rec_bound
            && self.has_source(st)
            && self.is_list(st)
            && st.borrow().host.is_none()
            && !self.shows_no_row(st, idx)
            && !matches!(
                page_of(&engine, st).controls[idx].prop("notblank"),
                Some(ir::PropVal::Lit(v)) if v.trim().eq_ignore_ascii_case("true")
            )
            && self.control_field(st, idx).is_some_and(|f| {
                f.properties
                    .iter()
                    .any(|(k, v)| k.eq_ignore_ascii_case("NotBlank") && v.trim().eq_ignore_ascii_case("true"))
            });
        let mut blank_enum = None;
        if let (RtType::Enum(id), Value::Text(t) | Value::Code(t)) = (&ty, &arg) {
            if !t.is_empty() && t.chars().all(|c| c == ' ') {
                let o = &engine.program.objects[*id as usize];
                // A value without a Caption shows its name (the `" "` member of QzNotBlankEnumList.al, BeEnumSetValueBlank.al:4021);
                // pinned in that list shape only.
                let blank = o
                    .enum_captions
                    .iter()
                    .find(|(_, c)| !c.is_empty() && c.chars().all(|c| c == ' '))
                    .map(|(v, _)| *v)
                    .or_else(|| {
                        if !pinned_list_not_blank {
                            return None;
                        }
                        o.enum_values
                            .iter()
                            .find(|(v, n)| {
                                !n.is_empty()
                                    && n.chars().all(|c| c == ' ')
                                    && !o.enum_captions.iter().any(|(w, _)| w == v)
                            })
                            .map(|(v, _)| *v)
                    });
                if let Some(v) = blank {
                    if !pinned_card && !pinned_list_not_blank {
                        return unsupported(
                            "SetValue of a blank text on an Enum control with a blank caption on a list or a page without a row (not pinned)",
                        );
                    }
                    blank_enum = Some(Value::Enum(*id, v));
                }
            }
        }
        if let (RtType::Enum(id), Value::Enum(eid, _)) = (&ty, &arg) {
            if id == eid {
                let shown = self.control_text(st, idx, &arg)?;
                // A request page's Enum variable takes it too and shows ' ' (Y37ValuesAllowedSetValue.al:247-248, :276).
                if !shown.is_empty() && shown.trim().is_empty() {
                    if !pinned_card && !(self.is_request_page(st) && !rec_bound) {
                        return unsupported(
                            "SetValue of a blank-captioned Enum value on a control of a list or of a page without a row (not pinned)",
                        );
                    }
                    blank_enum = Some(arg.clone());
                }
            }
        }
        if let (Value::Integer(_), RtType::Enum(_)) = (&arg, &ty) {
            let converted = ops::convert(arg.clone(), &ty)?;
            let shown = self.control_text(st, idx, &converted)?;
            if !shown.is_empty() && shown.trim().is_empty() {
                if !pinned_card && !pinned_list_not_blank {
                    if rec_bound {
                        return unsupported(
                            "SetValue of an Integer selecting a blank-captioned Enum value on a table-field control of a list or of a page without a row (not pinned)",
                        );
                    }
                    al_builtins::assumptions::assume(
                        al_builtins::assumptions::Assumption::TestPageSetValueBlankEnumMember,
                    );
                }
                blank_enum = Some(converted);
            }
        }
        // An Integer that is no value of the Enum is refused naming the ordinal the control held before, on a card with a row
        // (BeEnumSetValueBlank.al:523-525 `int5` from blank names '0', :742-744 from Beta names '2'; a variable control :2384).
        // Elsewhere it is not pinned.
        // An Option control does the same (:1907 from blank, :2138 from Beta; a variable control :3580).
        let mut old_ordinal = None;
        let not_a_value = match (&arg, &ty, &current) {
            (Value::Integer(n), RtType::Enum(id), Value::Enum(_, held)) => {
                let is_value = engine.program.objects[*id as usize]
                    .enum_values
                    .iter()
                    .any(|(v, _)| i64::from(*v) == i64::from(*n));
                (!is_value).then_some(*held)
            }
            (Value::Integer(n), RtType::Option(members), Value::Option(held, _)) => {
                (!usize::try_from(*n).is_ok_and(|n| n < members.len())).then_some(*held)
            }
            _ => None,
        };
        if let Some(held) = not_a_value {
            if !pinned_card {
                return unsupported(
                    "SetValue of an Integer that is no value of the Enum or Option on a list or a page without a row (not pinned)",
                );
            }
            old_ordinal = Some(held.to_string());
        }
        let text = match &arg {
            _ if old_ordinal.is_some() => old_ordinal.clone().unwrap_or_default().into(),
            Value::Text(t) | Value::Code(t) => t.clone(),
            // A Decimal handed to a text control is its Format text: 1.11, 1,234,567.5, 0, -3.5
            // (misc/lt-y14-triage/Y14Small.al:237, en-US, corpus master).
            Value::Decimal(_) if matches!(ty, RtType::Text(_)) => {
                match al_builtins::format::format_value(&arg, 0, 0, &*self) {
                    Ok(t) => t.into(),
                    Err(_) => return unsupported("SetValue of a Decimal on a text control whose Format fails"),
                }
            }
            // A whole number handed to a text or code control is its digits (WhseItemTrackingPageHandler of SCM RTAM Item Tracking-II sets
            // `Serial No.` to `TrackingQuantity2 + AssignTracking`, a Decimal); unprobed (assumption setvalue-whole-number-into-text-control).
            // Beyond 999 Format adds a thousands separator, which is not pinned for a control, so that stays refused.
            Value::Integer(_) | Value::BigInteger(_) | Value::Decimal(_)
                if matches!(ty, RtType::Text(_) | RtType::Code(_)) =>
            {
                let n = match &arg {
                    Value::Integer(i) => Some(i64::from(*i)),
                    Value::BigInteger(i) => Some(*i),
                    Value::Decimal(d) if d.fract().is_zero() => i64::try_from(d.trunc()).ok(),
                    _ => None,
                };
                match n {
                    Some(n) if n.abs() < 1000 => {
                        al_builtins::assumptions::assume(
                            al_builtins::assumptions::Assumption::SetValueWholeNumberIntoTextControl,
                        );
                        n.to_string().into()
                    }
                    _ => {
                        return unsupported(format!(
                            "SetValue of {} on a text control (how BC writes it is not pinned)",
                            arg.describe()
                        ));
                    }
                }
            }
            // An Enum handed to a text control (Analysis Types' "Item Ledger Entry Type Filter".SetValue(Enum)): the generic conversion
            // raises TypeConversion for a Variant holding an Enum, which is not what a control does; how the client writes it is not pinned.
            Value::Enum(..) if matches!(ty, RtType::Text(_) | RtType::Code(_)) => {
                return unsupported("SetValue of an Enum on a text control (how BC writes it is not pinned)");
            }
            other => {
                let converted = ops::convert(other.clone(), &ty)?;
                self.control_text(st, idx, &converted)?.into()
            }
        };
        // An Enum control that holds a refused entry shows the ordinal (see PageMethod::Value); an empty entry then is that
        // ordinal's text, which is refused as any ordinal is (pages/lt-page-enum-control/PageEnumControlCaptionless.al:107,
        // :109: `Your entry of '0'` after two refusals, on a table field of a card). Not pinned for a control bound to a
        // variable or for an Option control.
        let holds_error = st.borrow().errors.iter().any(|(i, _)| *i == idx);
        let text = if text.is_empty() && holds_error && matches!(ty, RtType::Enum(_) | RtType::Option(_)) {
            match (&ty, &current) {
                (RtType::Enum(_), Value::Enum(_, ordinal)) if rec_bound && !self.is_list(st) => {
                    ordinal.to_string().into()
                }
                _ => {
                    return unsupported(
                        "SetValue of an empty text on an Option or Enum control that holds a refused entry (not pinned here)",
                    );
                }
            }
        } else {
            text
        };
        if let RtType::Enum(id) = &ty {
            self.note_enum(*id);
        }
        let enum_list = match &ty {
            RtType::Enum(id) => self.enum_option_list(st, idx, *id)?.map(|(list, _)| list),
            _ => None,
        };
        if enum_list.is_some() && text.is_empty() {
            return unsupported(
                "SetValue of the empty text on an Enum control with an OptionCaption or a caption with a comma (not pinned)",
            );
        }
        let enum_engine = engine.clone();
        let by_text = move |id: u32, t: &str| {
            let o = &enum_engine.program.objects[id as usize];
            // The client's list takes the first entry that matches, its spaces trimmed (` with bonus` for `with bonus`,
            // misc/lt-cv-odd-leads/CvEnumCaptionComma.al:155-159; ` ` selects position 0 of ' ,c1,,,c4,c5',
            // handlers/TestPageEnumField_Tests.al:245-246), and the value at that position; an entry past the last value is
            // refused (`c4`, :240).
            if let Some(list) = &enum_list {
                let at = list.iter().position(|x| x.trim().eq_ignore_ascii_case(t))?;
                return o.enum_values.get(at).map(|(v, _)| *v);
            }
            // An enum control resolves by caption only; a value without a caption shows its name.
            // The bare member name of a captioned value is rejected, as on a real service tier
            // (handlers/TestPageEnumVariableControl_Tests.al:50-56,86-90,137-140).
            o.enum_values
                .iter()
                .find(|(v, name)| {
                    let shown = o.enum_captions.iter().find(|(c, _)| c == v).map_or(name, |(_, c)| c);
                    shown.eq_ignore_ascii_case(t)
                })
                .map(|(v, _)| *v)
        };
        // Without a row on the page there is nothing to refresh (controls/W2ValueControls.al:Enum_RecordFieldControl_*).
        let no_row = self.shows_no_row(st, idx);
        // A list shows no Refresh hint (misc/lt-probes-needed-3/PnC07TestPage.al:GetValidationError_AfterFailedValidation, ValueAssign_Number_FromText_And_Decimal);
        // a card does (handlers/TestPageErrTeardown_Tests.al, TestPageBooleanSpelling_Tests.al:210).
        let suffix = if rec_bound && !no_row && !self.is_list(st) {
            " (Select Refresh to discard errors)"
        } else {
            ""
        };
        let refuse_code = |msg: String, code: &str| -> Signal {
            let stored = format!("{msg}{suffix}");
            if let Err(why) = note_refusal(st, idx, stored.clone()) {
                return why;
            }
            error_code(
                format!(
                    "Validation error for Field: {},  Message = '{stored}'",
                    page_of(&engine, st).controls[idx].name
                ),
                code,
            )
        };
        let refuse = |_: &mut Session, msg: String| -> Signal { refuse_code(msg, "") };
        // The bounds and the length of a Text variable raise a TestValidation error
        // (handlers/TestPageVariableMinMaxContracts.al:15-16, :119).
        let refuse_bound = |msg: String| -> Signal { refuse_code(msg, "TestValidation") };
        // A blank value in a control bound to a NotBlank field is refused with a TestValidation error naming the caption
        // (pages/lt-page-presentation-properties/P119PageFields.al:P119_NotBlankPageField; Tests-SCM-Planning
        // ManufacturerCodeCannotBeBlank asserts the code). The text is pinned on a card with a row; on a list or a page
        // without a row it is not.
        // NotBlank on the page control does the same for a table field (P119PageFields.al:205-206, `Note must be filled in`), also
        // on a list, where the text has no Refresh hint (pages/lt-ap-testpage-642/ApTestPageTests.al:944, Ap17_NotBlank_List). On a
        // card without a row the entry is not checked: the row is missing (:962, Ap17_NotBlank_CardOnEmptyTable).
        let control_not_blank = matches!(
            page_of(&engine, st).controls[idx].prop("notblank"),
            Some(ir::PropVal::Lit(v)) if v.trim().eq_ignore_ascii_case("true")
        );
        // A control bound to a variable names no caption and has no Refresh hint, on a request page (pages/
        // lt-page-presentation-properties/P119ReportFields.al:158-159) and on a page without a source table (ApTestPageTests.al:977,
        // Ap17_NotBlank_PageWithoutSourceTable). On a page with a source table it is not pinned.
        if !rec_bound && control_not_blank && text.is_empty() {
            if !self.is_request_page(st) && self.has_source(st) {
                return unsupported(
                    "SetValue of a blank value on a NotBlank control bound to a variable of a page with a source table (not pinned)",
                );
            }
            return Err(refuse_bound(" must be filled in. Enter a value.".to_string()));
        }
        let field_not_blank = self.control_field(st, idx).is_some_and(|f| {
            f.properties
                .iter()
                .any(|(k, v)| k.eq_ignore_ascii_case("NotBlank") && v.trim().eq_ignore_ascii_case("true"))
        });
        // An Enum field with NotBlank on a list does not check the empty entry for NotBlank: it is refused as the same field
        // without NotBlank refuses it, an entry that is not a member (pages/lt-be-enum-setvalue-blank/BeEnumSetValueBlank.al:4021
        // QZ_NotBlankEnum_SetValueBlank_List, `Your entry of '' is not an acceptable value for 'KindNB'.`, no Refresh hint).
        let list_enum_field_not_blank =
            rec_bound && !no_row && !control_not_blank && self.is_list(st) && matches!(ty, RtType::Enum(_));
        if rec_bound
            && text.is_empty()
            && (control_not_blank || field_not_blank)
            && !(control_not_blank && no_row)
            && !list_enum_field_not_blank
        {
            if no_row || (self.is_list(st) && !control_not_blank) {
                return unsupported(
                    "SetValue of a blank value on a NotBlank control of a list or of a page without a row (the client's text is not pinned)",
                );
            }
            return Err(refuse_bound(format!("{caption} must be filled in. Enter a value.")));
        }
        // A Numeric page field takes digits only (pages/lt-page-presentation-properties/P119PageNumeric.al:P119_PageNumericSetValue:
        // letters are refused with a TestValidation error). Other non-digits are not pinned.
        if matches!(ty, RtType::Text(_) | RtType::Code(_))
            && matches!(page_of(&engine, st).controls[idx].prop("numeric"), Some(ir::PropVal::Lit(v)) if v.trim().eq_ignore_ascii_case("true"))
            && !text.chars().all(|c| c.is_ascii_digit())
        {
            if !text.chars().any(|c| c.is_ascii_alphabetic()) {
                return unsupported(
                    "SetValue on a Numeric field with a character that is neither a digit nor a letter (not pinned)",
                );
            }
            return Err(refuse_bound(format!(
                "Your entry of '{text}' is not an acceptable value for '{caption}'. Only numeric characters are allowed in field."
            )));
        }
        // CharAllowed on a page field or on the table field it shows: a Text or Code entry may hold only the characters from the first to
        // the second of the property's two characters, case sensitive (pages/lt-page-controls-aa/AaFieldProps.al:AA_CharAllowed_PageField
        // line 67 `az` and `09`, AA_CharAllowed_TableField line 100). The error names the first character outside it. The table field's
        // property restricts the page only: Validate and assignment in code accept anything (same file, `validate-upper`, `assign`).
        // Pinned on a card with a row and a two-character property only.
        if matches!(ty, RtType::Text(_) | RtType::Code(_)) {
            let page_prop = match page_of(&engine, st).controls[idx].prop("charallowed") {
                Some(ir::PropVal::Lit(v)) => Some(unquote(v)),
                _ => None,
            };
            let field_prop = self.control_field(st, idx).and_then(|f| {
                f.properties
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case("CharAllowed"))
                    .map(|(_, v)| unquote(v))
            });
            // On a list the table field's property is pinned as on a card, with no Refresh hint in the text
            // (record/lt-x-table-props/XTablePropsTests.al:928 QZ_CharAllowed_TestPage_List); the page control's property there and
            // a part are not.
            let list_pinned = rec_bound && page_prop.is_none() && st.borrow().host.is_none();
            if let Some(allowed) = page_prop.or(field_prop) {
                // Several pairs are several ranges: a character in any of them is taken, and the error lists the pairs
                // separated by commas (pages/lt-ah-pages/AhPageTests.al:176 `az,AZ,09`, :808-811 ThreeRanges, :837-838
                // SinglesAndRange `09--__`, :862 UnorderedSingles `zz55aa`, :908 TwoRangesOfLetters). An odd number of
                // characters, or a pair from a higher to a lower character, is not pinned.
                let chars: Vec<char> = allowed.chars().collect();
                let ranges: Vec<(char, char)> =
                    chars.chunks(2).filter(|p| p.len() == 2).map(|p| (p[0], p[1])).collect();
                if chars.is_empty()
                    || chars.len() % 2 != 0
                    || ranges.iter().any(|(a, b)| a > b)
                    || no_row
                    || (self.is_list(st) && !list_pinned)
                {
                    return unsupported(
                        "SetValue on a field with CharAllowed of an odd length or a backward range, on a list or on a page without a row (not pinned)",
                    );
                }
                let takes = |c: char| ranges.iter().any(|(a, b)| (*a..=*b).contains(&c));
                // A Code control upper-cases the entry first: the refused character is named upper-cased
                // (record/lt-x-table-props/XTablePropsTests.al:104, `4x6` against `09` names `X`). Where upper-casing changes
                // whether a character is taken (a range of lower-case letters), what BC does is not pinned.
                let checked: String = if matches!(ty, RtType::Code(_)) {
                    if text.chars().any(|c| takes(c) != c.to_uppercase().all(takes)) {
                        return unsupported(
                            "SetValue on a Code field with CharAllowed where upper-casing changes what is allowed (not pinned)",
                        );
                    }
                    text.to_uppercase()
                } else {
                    text.to_string()
                };
                if let Some(bad) = checked.chars().find(|c| !takes(*c)) {
                    let listed: Vec<String> = ranges.iter().map(|(a, b)| format!("{a}{b}")).collect();
                    return Err(refuse_bound(format!(
                        "The character {bad} is not valid in this field. You can specify the following characters: {}.",
                        listed.join(",")
                    )));
                }
            }
        }
        let parsed = match blank_enum {
            Some(v) => Ok(v),
            None => al_builtins::pagefmt::parse_control_text(&text, &ty, &fmt, &by_text),
        };
        let parsed = match parsed {
            Ok(v) => v,
            Err(al_builtins::pagefmt::ParseError::Invalid) => {
                // A control bound to a writable expression of a page with no source table says it too
                // (pages/lt-page-array-path/PageArrayPathSetValue.al:148 ArrayElementField_SetValueUnconvertible_IsRefused).
                let no_source_expr = {
                    let engine = self.engine();
                    let ctl = &page_of(&engine, st).controls[idx];
                    matches!(ctl.binding, ir::Binding::Expr(_))
                        && ctl.trigger("__sourceset").is_some()
                        && !self.has_source(st)
                        && !self.is_request_page(st)
                };
                if no_source_expr && !matches!(ty, RtType::Decimal) {
                    return unsupported(
                        "a refused entry on a control bound to an expression of a page with no source table, of a type other than Decimal (the text is not pinned)",
                    );
                }
                // The message names an entry of up to 19 characters whole (pages/lt-page-run-overloads-testpart/
                // P106TestFieldAsDateTime.al:61, controls/W2ValueControls.al:456); an Enum entry of 25 keeps its first and last 8 around `...`
                // (misc/lt-cv-odd-leads/CvEnumCaptionComma.al:151, `Diamond ...th bonus`). Other lengths are not pinned.
                let n = text.chars().count();
                let named: String = if n <= 19 {
                    text.to_string()
                } else if n == 25 && matches!(ty, RtType::Enum(_)) {
                    let chars: Vec<char> = text.chars().collect();
                    format!(
                        "{}...{}",
                        chars[..8].iter().collect::<String>(),
                        chars[n - 8..].iter().collect::<String>()
                    )
                } else {
                    return unsupported(format!(
                        "a refused entry of {n} characters (how the message shortens it is not pinned)"
                    ));
                };
                let mut msg = format!("Your entry of '{named}' is not an acceptable value for '{caption}'.");
                // A Date adds how the client reads one (controls/W2ValueControls.al:Date_IsoSpellings and
                // session/W5WorkDateShorthand.al, BC 27.5 and 29.0); the text holds a CR, a backslash, a CR and
                // a backslash where one would expect line breaks.
                // An Integer variable adds why (controls/W2FindFieldAssign.al:ValueAssign_IntegerControl_TextAndNonNumber).
                // A table field on a card does too (pages/lt-ah-pages/AhPageTests.al:112, pages/lt-page-runtime/P82TestField.al:296).
                // A table field on a list is not pinned.
                let card_field = rec_bound && !no_row && !self.is_list(st);
                if matches!(ty, RtType::Integer) {
                    if rec_bound && !card_field {
                        return unsupported(
                            "SetValue of a text that is not a number on an Integer field of a list or of a page without a row (the text is not pinned)",
                        );
                    }
                    msg.push_str(&format!(" The value \"{text}\" can't be evaluated into type Integer."));
                }
                // A Decimal says what was wrong (misc/lt-probes-needed-3/PnC07TestPage.al:ValueAssign_Number_FromText_And_Decimal, a list;
                // pages/lt-ah-pages/AhPageTests.al:133 and P82TestField.al:302, a card).
                if ((rec_bound && !no_row) || no_source_expr) && matches!(ty, RtType::Decimal) {
                    msg.push_str(&format!(" '{text}' is not a part of a valid decimal expression."));
                }
                // The number conversion of a table field on a card is a TestValidation error (AhPageTests.al:113, :134).
                if card_field && matches!(ty, RtType::Integer | RtType::Decimal) {
                    return Err(refuse_bound(msg));
                }
                if matches!(ty, RtType::Date) {
                    msg.push_str(&format!(
                        " The format of the Date '{text}' does not match your device's date settings.\\\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format 'yyyy-mm-dd'."
                    ));
                }
                // A DateTime adds that the date is not valid (pages/lt-page-run-overloads-testpart/P106TestFieldAsDateTime.al:
                // P106_AsDateTime_Card, an ISO text on a table field).
                if matches!(ty, RtType::DateTime) {
                    msg.push_str(" The date is not valid.");
                }
                // A control bound to a variable raises TestValidation (controls/W2ValueControls.al:Date_IsoSpellings,
                // Enum_VariableControl_*); one bound to a table field keeps the empty code.
                return Err(
                    // An Enum control raises TestValidation on a card with a row too (pages/lt-page-enum-control/
                    // PageEnumControlCaptionless.al:P161_EnumControl_SetValueSpellings_Captionless `spaceOrdinal`).
                    // A refused text on an Option control is a TestValidation error on a card with a row: the empty text (pages/lt-testpage-assertequals-option/
                    // TestPageAssertEqualsOption.al:SetValue_BlankOption_AsEmptyText) and `Nope`, `5`, `2`, `Tw` (handlers/N582TestPageEdges.al:156-186).
                    if rec_bound
                        && !no_row
                        && !matches!(ty, RtType::DateTime | RtType::Enum(_))
                        && !matches!(ty, RtType::Option(_))
                        && !self.is_list(st)
                    {
                        refuse(self, msg)
                    } else {
                        refuse_bound(msg)
                    },
                );
            }
            Err(al_builtins::pagefmt::ParseError::Unsupported) => {
                return unsupported(format!(
                    "SetValue of a {ty} control: its spelling is not modelled yet",
                    ty = ty.describe()
                ));
            }
        };
        // The entry is read before the row is looked for: a refused entry is a validation error, an acceptable one on
        // a page without a row is TestRowNotFound (controls/W2ValueControls.al:Enum_RecordFieldControl_CaptionAndNames).
        if self.shows_no_row(st, idx) {
            return Err(error_code("The row does not exist on the TestPage.", "TestRowNotFound"));
        }
        // A Text variable given too many characters (handlers/TestPageVariableMinMaxContracts.al:119).
        // A table field's overlong text is a TestValidation error too (P81PageValidateEvents.al:TestPage_SetValue_NameTooLong).
        if let (RtType::Text(Some(max)), Value::Text(t)) = (&ty, &parsed) {
            let len = Value::text_len(t);
            if len > *max as usize {
                return Err(refuse_bound(format!(
                    "The length of the string is {len}, but it must be less than or equal to {max} characters. Value: {t}"
                )));
            }
        }
        let parsed = ops::convert(parsed, &ty)?;
        // A value outside the control's ValuesAllowed is a TestValidation error; the entry it names is the text typed, and for an
        // Enum or an Integer handed over the ordinal the control holds (pages/lt-y37-values-allowed/Y37ValuesAllowedSetValue.al:
        // 214 `'3'` for Planned over Released, :217 `'Simulated'`, :222 an Option variable, :270-278 request page Enum variables,
        // :325-328 a list; a card adds the Refresh hint, pageext/lt-page-ext-properties/PageFieldValuesAllowed.al:69). The
        // control's OnValidate does not run (Y37ValuesAllowedSetValue.al:226) and Value then shows the held ordinal (:216, :224).
        if let Some(allowed) = &allowed {
            let ordinal = match &parsed {
                Value::Enum(_, o) | Value::Option(o, _) => *o,
                _ => return unsupported("SetValue on a control with ValuesAllowed whose value is no Enum or Option"),
            };
            if !allowed.contains(&ordinal) {
                let entry = match (&arg, &current) {
                    (Value::Text(_) | Value::Code(_), _) => text.to_string(),
                    (Value::Enum(..) | Value::Integer(_), Value::Enum(_, held) | Value::Option(held, _)) => {
                        held.to_string()
                    }
                    _ => {
                        return unsupported(format!(
                            "SetValue of {} outside ValuesAllowed (the entry the error names is not pinned)",
                            arg.describe()
                        ));
                    }
                };
                if self.is_list(st) && self.has_source(st) {
                    let key = self.row_key(&self.page_rec(st))?;
                    let earlier = st.borrow().values_allowed_refused_row.clone();
                    if earlier.is_some_and(|(i, k)| i == idx && !Self::same_values(&k, &key)) {
                        return unsupported(
                            "a second refused entry of a list control's ValuesAllowed on another row (what the control then shows is not pinned)",
                        );
                    }
                    st.borrow_mut().values_allowed_refused_row = Some((idx, key));
                }
                return Err(refuse_bound(format!(
                    "Your entry of '{entry}' is not an acceptable value for '{caption}'."
                )));
            }
        }
        // A table field's ValuesAllowed is checked by the client: on a Code[1] field with `'', 'A', 'N'` an entry outside the list,
        // and also the empty text although '' is listed, is a TestValidation error that lists the values with `;`; `A` is taken
        // (pages/lt-cd-testpage-leads/CdTestPageLeads.al:396-423, Item56_ValuesAllowedEmpty_SetValueEmptyAfterA,
        // Item56_ValuesAllowed_SetValueNotAllowed). Validate in code does not check it (:426-444). Pinned for a Text or Code field
        // of quoted values on a card with a row; an entry that matches only when the case is ignored, or that a Code field
        // upper-cases, and the empty text on a list without '' are not.
        if let Some(raw) = self.control_field(st, idx).and_then(|f| {
            f.properties
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("ValuesAllowed"))
                .map(|(_, v)| v.clone())
        }) {
            let values = quoted_list(&raw);
            let card_row = rec_bound && !no_row && !self.is_list(st);
            let Some(values) = values.filter(|_| card_row && matches!(ty, RtType::Text(_) | RtType::Code(_))) else {
                return unsupported(
                    "SetValue on a field with ValuesAllowed that is not a Text or Code field of quoted values on a card with a row (not pinned)",
                );
            };
            let typed = text.to_string();
            if matches!(ty, RtType::Code(_)) && typed != typed.to_uppercase() {
                return unsupported("SetValue of a lower-case entry on a Code field with ValuesAllowed (not pinned)");
            }
            let exact = values.contains(&typed);
            if !(exact && !typed.is_empty()) {
                if !exact && values.iter().any(|v| v.eq_ignore_ascii_case(&typed)) {
                    return unsupported(
                        "SetValue on a field with ValuesAllowed of an entry that differs in case only (not pinned)",
                    );
                }
                if typed.is_empty() && !exact {
                    return unsupported(
                        "SetValue of the empty text on a field whose ValuesAllowed does not list '' (not pinned)",
                    );
                }
                return Err(refuse_bound(format!(
                    "Value '{typed}' is not a valid value. The following values can be used: {}.",
                    values.join(";")
                )));
            }
        }
        // MinValue / MaxValue are enforced on a control write, not on Rec.Validate (corpus
        // handlers/TestPageFieldMinMaxValue_Tests.al:20-110). A control bound to a variable has
        // its own bounds, with the same texts; a table field's adds the "Select Refresh" suffix
        // (handlers/TestPageVariableMinMaxContracts.al:60-130). The value shows as typed for an
        // integer and with two decimals for a decimal (2.60, 0.40; more decimals are not pinned).
        {
            // The page field's own MinValue / MaxValue count on a control bound to a table field too (pages/
            // lt-page-presentation-properties/P119PageNumeric.al:P119_PageMinMax); then the value shows as the control shows it.
            let page_bound = |n: &str| match page_of(&engine, st).controls[idx].prop(n) {
                Some(ir::PropVal::Lit(v)) => Some(v.trim().to_string()),
                _ => None,
            };
            // Pinned for a Decimal field only: an Integer field with the same page properties takes 11 above a MaxValue of 10
            // without an error (handlers/TestPageValidationGetError_Tests.al:GetValidationError_IndexPastLast `setValueOutOfRange`).
            let on_the_page = self.control_field(st, idx).is_some()
                && matches!(parsed, Value::Decimal(_))
                && (page_bound("minvalue").is_some() || page_bound("maxvalue").is_some());
            let bound = |n: &str| match (
                on_the_page.then(|| page_bound(n)).flatten(),
                self.control_field(st, idx),
            ) {
                (Some(v), _) => Some(v),
                (None, Some(f)) => f
                    .properties
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case(n))
                    .map(|(_, v)| v.trim().to_string()),
                (None, None) => page_bound(n),
            };
            let num = match &parsed {
                Value::Integer(i) => Some((rust_decimal::Decimal::from(*i), i.to_string())),
                Value::BigInteger(i) => Some((rust_decimal::Decimal::from(*i), i.to_string())),
                Value::Decimal(d) if d.normalize().scale() <= 2 => Some((*d, format!("{:.2}", d))),
                _ => None,
            };
            let beyond_pinned = matches!(&parsed, Value::Decimal(d) if d.normalize().scale() > 2);
            let min = bound("minvalue");
            let max = bound("maxvalue");
            let breaks = |b: &Option<String>, below: bool| match (b, &parsed) {
                (Some(b), Value::Decimal(_) | Value::Integer(_) | Value::BigInteger(_)) => {
                    b.parse::<rust_decimal::Decimal>().ok().is_some_and(|m| {
                        let v = match &parsed {
                            Value::Integer(i) => rust_decimal::Decimal::from(*i),
                            Value::BigInteger(i) => rust_decimal::Decimal::from(*i),
                            Value::Decimal(d) => *d,
                            _ => return false,
                        };
                        if below { v < m } else { v > m }
                    })
                }
                _ => false,
            };
            let (under, over) = (breaks(&min, true), breaks(&max, false));
            if (under || over) && beyond_pinned {
                return unsupported(
                    "SetValue outside MinValue/MaxValue with more than two decimals (how BC shows the value is not pinned)",
                );
            }
            if let Some((_, shown)) = num {
                let shown = if on_the_page && (under || over) {
                    self.control_text(st, idx, &parsed)?.to_string()
                } else {
                    shown
                };
                if under {
                    let min = min.unwrap_or_default();
                    return Err(refuse_bound(format!(
                        "The value must be greater than or equal to {min}. Value: {shown}."
                    )));
                }
                if over {
                    let max = max.unwrap_or_default();
                    return Err(refuse_bound(format!(
                        "The value must be less than or equal to {max}. Value: {shown}."
                    )));
                }
            }
        }
        // A value equal to the one held still validates: OnValidate runs again (corpus
        // handlers/TestPageRuntimeContracts.al:179-194 TestPage_SetValue_OfTheValueAlreadyHeld_StillRunsOnValidate).
        let rec = if rec_bound { Some(self.page_rec(st)) } else { None };
        let before = rec.as_ref().map(|r| r.borrow().fields.clone());
        let was_dirty = st.borrow().dirty;
        // CurrPage.SaveRecord() inside the control's OnValidate must see the edit (corpus
        // handlers/TestPageSaveRecordInOnValidate_Tests.al:SaveRecordInOnValidatePersistsBeforeThePageCloses).
        // A control bound to a page variable does not edit the row: nothing for SaveRecord to write (Planning Worksheet's batch
        // name control calls SaveRecord from OnValidate on a DelayedInsert page; Tests-SCM-Planning
        // OrderTrackingForCalcRegenPlanSalesUsingForecastLFLItem).
        if rec_bound {
            st.borrow_mut().dirty = true;
        }
        let from_page_trigger = std::cell::Cell::new(false);
        st.borrow_mut().save_record_called = false;
        let was_validating = std::mem::replace(&mut st.borrow_mut().validating_control, rec_bound);
        let result = (|| -> Result<(), Signal> {
            match ctl.binding.clone() {
                ir::Binding::Field(pos) => {
                    self.ret_used = false;
                    let rec = rec.clone().expect("rec-bound");
                    // The field's OnValidate sees its own number as CurrFieldNo
                    // (handlers/TestPageCurrFieldNo_Tests.al:36-60); the page's save later sees 0 (:105-123).
                    let number = {
                        let table = rec.borrow().table as usize;
                        self.engine.tables[table]
                            .as_ref()
                            .map_or(0, |t| t.fields[pos].id as i32)
                    };
                    self.pages.curr_field_no.push((Rc::downgrade(&rec), number));
                    // A validation through the page runs the table's OnValidate on fresh table
                    // globals, and leaves the page's record with fresh globals (ADR-021 decision 8;
                    // corpus probes TableGlobals_PageFieldValidate_*, TableGlobals_PageRec_ActionGlobalsAreLostAfterPageFieldValidation).
                    self.reset_table_globals(&rec);
                    let validated = self.record_op(
                        RecordOp::Validate,
                        &rec,
                        &mut [Value::Integer(pos as i32), parsed.clone()],
                    );
                    self.reset_table_globals(&rec);
                    self.pages.curr_field_no.pop();
                    validated?;
                }
                ir::Binding::Var(_) | ir::Binding::BaseVar(_) => {
                    let (slot, owner) = ctl.binding.var_slot(ctl.owner).expect("a variable binding");
                    let inst = self.owner_instance(st, owner);
                    inst.borrow_mut().globals[slot] = parsed.clone();
                }
                // A control bound to an expression that is a place (`Totals[1]."Inv. Discount Amount"`) is written through the
                // routine `pages::expand` made (`mark_expression_write`).
                ir::Binding::Expr(_) if ctl.trigger("__sourceset").is_some() => {
                    let w = ctl.trigger("__sourceset").expect("checked");
                    mark_expression_write(st, ctl);
                    let owner = ctl.owner;
                    self.owner_routine_with(st, owner, w, vec![parsed.clone()])?;
                }
                _ => return unsupported("SetValue on a read-only control"),
            }
            // The page's events around the control's own trigger (corpus P81PageValidateEvents.al); OnBeforeValidate of `modify`
            // blocks runs after the page's OnBeforeValidateEvent (TT_TableOnValidate_Order_WithEvents on writer TT's branch).
            let element = page_of(&self.engine(), st).controls[idx].name.clone();
            if rec_bound {
                self.page_validate_event(st, "onbeforevalidateevent", &element)?;
            }
            for (e, r) in &ext_before {
                self.owner_routine(st, Some(*e), *r)?;
            }
            if let Some(r) = ctl.trigger("onvalidate") {
                if self.has_source(st) {
                    let filters = self.page_rec(st).borrow().filters.clone();
                    st.borrow_mut().filters_before_validate = Some(filters);
                }
                if let Err(e) = self.control_routine(st, idx, r) {
                    from_page_trigger.set(true);
                    return Err(e);
                }
            }
            for (e, r) in &ext_after {
                self.owner_routine(st, Some(*e), *r)?;
            }
            if rec_bound {
                self.page_validate_event(st, "onaftervalidateevent", &element)?;
            }
            Ok(())
        })();
        st.borrow_mut().validating_control = was_validating;
        // CurrPage.SaveRecord in the validation left xRec as it was; the row it stored is xRec from here on (the next SetValue's
        // OnValidate sees it: pages/lt-qj-part-propagation/QjPartPropagation.al:735 QJ_SubformSaveRecord_StoredRow and :766 _NewRow
        // `S.V.before(... x=5 ...)`).
        let pending_xrec = st.borrow_mut().xrec_after_validate.take();
        if let Some(fields) = pending_xrec {
            self.set_xrec_fields(st, fields);
        }
        match result {
            Ok(()) => {
                // The row as typed is the current row the next OnAfterGetCurrRecord sees as xRec (corpus bump 41
                // pages/lt-qj-part-propagation/QjPartPropagation.al QJ_Both_PartModifiesHost_ThenHostSave `L.Qty.V(5 x=1) ...
                // L.AGCR(10000 q=1 x.q=5)`).
                if rec_bound && !st.borrow().draft && st.borrow().cursor == Cursor::Row {
                    let typed = self.page_rec(st).borrow().fields.clone();
                    st.borrow_mut().agcr_xrec = Some(typed);
                }
                // A SetValue that goes through clears the control's refused entries (pages/lt-f3-pages/F3ValidationAddinFilter.al:
                // GetValidationError_ErrorThenCorrected, line 272: the page answers an empty text afterwards).
                st.borrow_mut().errors.retain(|(i, _)| *i != idx);
                st.borrow_mut().trigger_errors.retain(|(i, _)| *i != idx);
                if st
                    .borrow()
                    .values_allowed_refused_row
                    .as_ref()
                    .is_some_and(|(i, _)| *i == idx)
                {
                    st.borrow_mut().values_allowed_refused_row = None;
                }
                let saved = std::mem::take(&mut st.borrow_mut().save_record_called);
                let update_asked = st.borrow().refresh;
                if update_asked && st.borrow().lookup_setting_value {
                    self.defer_lookup_refresh(st);
                }
                if !(update_asked && self.host_after_part_update(st)?) {
                    self.run_refresh(st)?;
                }
                if saved && !update_asked && rec_bound {
                    self.reread_parts_after_host_save_record(st)?;
                }
                self.refresh_caption_classes(st)?;
                Ok(Value::Null)
            }
            Err(Signal::Error(e)) => {
                st.borrow_mut().refresh = false;
                st.borrow_mut().dirty = was_dirty;
                // The refused edit does not stay (corpus handlers/TestPageBooleanSpelling_Tests.al,
                // SetValue_TrueFalseSpelling_IsRefused: the stored value is untouched).
                if let (Some(rec), Some(before)) = (&rec, before) {
                    rec.borrow_mut().fields = before;
                } else if let Some((slot, owner)) = ctl.binding.var_slot(ctl.owner) {
                    let inst = self.owner_instance(st, owner);
                    inst.borrow_mut().globals[slot] = current.clone();
                } else if let (ir::Binding::Expr(_), Some(w)) = (&ctl.binding, ctl.trigger("__sourceset")) {
                    let _ = self.owner_routine_with(st, ctl.owner, w, vec![current.clone()]);
                }
                // An error of the write through an expression source (a text too long for the field) is a TestValidation
                // error, as for a variable (misc/lt-probes-needed-7/PnGTempControl.al:SetValueTooLongAndInvalid).
                // The text goes into a TestValidation error or the page's error list.
                // An error with an empty text (`Error('')`) out of a card control's own OnValidate ends the SetValue quietly: the
                // test goes on, the entry does not stay, the last error has the empty text and code NSValidateField:Dialog, and
                // the rows the trigger wrote stay (pages/lt-qi-empty-error/QiEmptyErrorPages.al:253
                // QI_EE_EmptyValidate_NoAsserterror). The same on a List and a Worksheet page opened on its own, and on a card part
                // of a card (misc/lt-np4-probes/Np4EmptyError.al:200 NP4_EE_ControlValidate_List, :224 _Worksheet, :248
                // _PartOfCard: the control shows the old value and the stored row keeps it). The same on a ListPart inside a Card
                // or a Document page (corpus PR 896, Np4EmptyError.al Y9_EE_ControlValidate_ListPartInCard,
                // Y9_EE_ControlValidate_DocumentLinePart). A list part in another host and another page type with a repeater
                // are not pinned.
                if from_page_trigger.get() && e.message.is_empty() {
                    let own = &engine.object_facts[st.borrow().object as usize].page_type;
                    let host = st.borrow().host.as_ref().map(|(h, _)| h.upgrade());
                    let pinned_list = match host {
                        None => matches!(own, Some(al_ir::PageType::List | al_ir::PageType::Worksheet)),
                        Some(Some(h)) => {
                            matches!(own, Some(al_ir::PageType::Other(o)) if o == "listpart")
                                && matches!(
                                    engine.object_facts[h.borrow().object as usize].page_type,
                                    Some(al_ir::PageType::Card | al_ir::PageType::Document)
                                )
                        }
                        Some(None) => false,
                    };
                    if self.is_list(st) && !pinned_list {
                        return unsupported(
                            "an error with an empty text out of a list control's OnValidate (what the test client does with it is not pinned)",
                        );
                    }
                    let mut e = e;
                    e.code = format!("NSValidateField:{}", e.code);
                    self.error_finals = None;
                    self.last_error = *e;
                    return Ok(Value::Null);
                }
                // Out of the table's OnValidate (or anything else than the control's own trigger) only an error with a text is
                // pinned (P81PageValidateEvents.al below); for `Error('')` the corpus pins the card control's trigger alone
                // (QiEmptyErrorPages.al:253). Corpus issue 712 queues the probe.
                // Out of the table field's OnValidate of a control bound to a field of Rec, on a control without its own OnValidate,
                // it ends the SetValue quietly as well, on a List and a Card, directly and after a Confirm answered No: the test goes
                // on, the last error has the empty text and code NSValidateField:Dialog, the control shows the old value and the
                // stored row keeps it (QiEmptyErrorPages.al:342 QZ_EE_TableValidate_ConfirmNo_List_NoAsserterror, :367 _Asserterror,
                // :390 _Direct_List_NoAsserterror, :414 _Direct_Card_NoAsserterror). The same on a ListPart inside a Card, where a
                // later edit of the control is stored (misc/lt-y13-triage/Y13PageTriage.al:350-374, corpus master).
                let page_type = |s: &St| engine.object_facts[s.borrow().object as usize].page_type.clone();
                let host = st.borrow().host.as_ref().and_then(|(h, _)| h.upgrade());
                let pinned_shape = match &host {
                    None => matches!(page_type(st), Some(al_ir::PageType::List | al_ir::PageType::Card)),
                    Some(h) => {
                        h.borrow().host.is_none()
                            && matches!(page_type(st), Some(al_ir::PageType::Other(o)) if o == "listpart")
                            && page_type(h) == Some(al_ir::PageType::Card)
                    }
                };
                if e.message.is_empty() && rec_bound && ctl.trigger("onvalidate").is_none() && pinned_shape {
                    let mut e = e;
                    e.code = format!("NSValidateField:{}", e.code);
                    self.error_finals = None;
                    self.last_error = *e;
                    return Ok(Value::Null);
                }
                // `RequestPage.Activate` in a request page control's OnValidate aborts the page: the error reaches the test as it
                // is, not as a validation error (report/lt-report-requestpage-query-members/P105RpMembers.al:171-184).
                if e.code == "ClientPageAbort:FormAbort"
                    && self.is_request_page(st)
                    && e.message.starts_with("Unhandled UI: Page ")
                {
                    return Err(Signal::Error(e));
                }
                if e.message.is_empty() {
                    return unsupported(
                        "an error with an empty text out of a table field's OnValidate during TestPage SetValue (what the test client does with it is not pinned)",
                    );
                }
                al_builtins::native::refuse_unpinned_text(&e, "A page validation error built from an error")?;
                // A control add-in call in a card control's own OnValidate (the add-in is never created in a test session) raises
                // the add-in's error with NSValidationFailed: in front of its code (lt-ac/ACControlAddIn.al:99
                // AC_ControlAddIn_ProcedureCallFromFieldTrigger, BC28PLUS). On another page type, or out of the table's OnValidate,
                // it is not pinned.
                if e.code == crate::control_add_in::NOT_CREATED {
                    if !(from_page_trigger.get() && self.is_card(st)) {
                        return Err(Signal::Error(Box::new(al_builtins::native::unpinned_error())));
                    }
                    let mut e = e;
                    e.code = format!("NSValidationFailed:{}", e.code);
                    return Err(Signal::Error(e));
                }
                if matches!(ctl.binding, ir::Binding::Expr(_)) {
                    return Err(refuse_bound(e.message));
                }
                // An error of a control's own OnValidate on a list is the bare message, with no Refresh hint (misc/lt-probes-needed-3/
                // PnC07TestPage.al:GetValidationError_AfterFailedValidation and ValueAssign_ValidationError); on a card it carries it
                // (handlers/TestPageErrTeardown_Tests.al, codeunit 60836). Whether the list or something else in that page decides is
                // not probed (docs/probes-needed.md).
                if from_page_trigger.get() && self.is_list(st) {
                    note_refusal(st, idx, e.message.clone())?;
                    st.borrow_mut().trigger_errors.push((idx, e.message.clone()));
                    return Err(error_code(
                        format!(
                            "Validation error for Field: {},  Message = '{}'",
                            page_of(&engine, st).controls[idx].name,
                            e.message
                        ),
                        "TestValidation",
                    ));
                }
                // An error of the table's validation (its trigger, the length of the text) carries TestValidation
                // (record/lt-loadfields-pagesource-validate/P81PageValidateEvents.al:TestPage_SetValue_NegativeQty_TableTriggerRaises,
                // TestPage_SetValue_NameTooLong).
                let refusal = refuse_bound(e.message.clone());
                if matches!(refusal, Signal::Error(_)) {
                    st.borrow_mut().trigger_errors.push((idx, e.message));
                }
                Err(refusal)
            }
            Err(other) => Err(other),
        }
    }

    // ---- controls -----------------------------------------------------------------------

    /// Whether control `idx` is bound to an expression that is no place (no routine writes it), on a Card without a source table
    /// that is no request page, and declares no Editable: the shape QZ_NoSourceTable_SetValue_OnExpressionControls pins
    /// (misc/lt-protected-var-abs-query-views/P146PageExprSetValue.al:506, `editable = No|No|No|Yes`).
    pub(super) fn read_only_expression_control(&self, st: &St, idx: usize) -> bool {
        let engine = self.engine();
        let ctl = &page_of(&engine, st).controls[idx];
        matches!(ctl.binding, ir::Binding::Expr(_))
            && ctl.trigger("__sourceset").is_none()
            && ctl.prop("editable").is_none()
            && !self.has_source(st)
            && !self.is_request_page(st)
            && st.borrow().host.is_none()
            && matches!(
                engine.object_facts[st.borrow().object as usize].page_type,
                Some(al_ir::PageType::Card)
            )
    }

    /// Whether control `idx` sits inside a `cuegroup` with `CuegroupLayout = Wide`.
    fn in_wide_cue_group(&self, st: &St, idx: usize) -> bool {
        let engine = self.engine();
        let page = page_of(&engine, st);
        let mut at = page.controls[idx].parent;
        while let Some(p) = at {
            if page.controls[p].kind == ir::ControlKind::CueGroup {
                return matches!(page.controls[p].prop("cuegrouplayout"), Some(ir::PropVal::Lit(v)) if v.trim().eq_ignore_ascii_case("wide"));
            }
            at = page.controls[p].parent;
        }
        false
    }

    pub(super) fn test_control_op(
        &mut self,
        method: PageOp,
        c: &ControlRef,
        args: &mut [Value],
    ) -> Result<Value, Signal> {
        self.require_open(&c.page)?;
        if c.page.borrow().link_validate_failed {
            // QjPartPropagation.al:3766 QT_ValPartSplit_Empty_Last_KeyValidateErrors `type|text = The row does not exist on the
            // TestPage.`, `type|code = TestRowNotFound`, nothing logged or stored; :3825 QT_ValPartPlain_.
            if method.method() == PageMethod::SetValue {
                return Err(error_code("The row does not exist on the TestPage.", "TestRowNotFound"));
            }
            return unsupported(format!(
                "TestField.{} on a part whose link value failed its validation as the host opened (not pinned)",
                method.method()
            ));
        }
        self.refuse_unpinned_state(&c.page)?;
        let st = &c.page;
        let m = method.method();
        let field_read = m == PageMethod::Value && args.is_empty();
        self.first_part_access(st, m, field_read)?;
        // A new host whose part shows rows of the blank key: a read of a host field and OK are pinned (PartLineBeforeHostKey.al:
        // 2116 QH_BlankRows_ReadHostControl, :1939 QH_BlankRows_OkInvoke), and Activate of a host field: of a non-key field it
        // inserts the host as without the rows, and the part then links to the new key; of the key field it does nothing
        // (pages/lt-y17-activate-new-host/Y17ActivateNewHost.al:382 QH_BlankRows_ActivateHostField, :473 _ActivateKeyControl,
        // :521 _CardHost). Nothing else.
        if st.borrow().new_host_part_rows && st.borrow().host.is_none() {
            let ok = matches!(&c.target, ControlTarget::BuiltIn(n) if n == "OK") && m == PageMethod::Invoke;
            let activate = m == PageMethod::Activate && matches!(c.target, ControlTarget::Control(_));
            if !(field_read && matches!(c.target, ControlTarget::Control(_))) && !ok && !activate {
                return unsupported(format!(
                    "TestField.{m} on a new, unsaved host whose part shows rows of the blank key (not pinned)"
                ));
            }
        }
        // After a GoToKey that moved a list without OnAfterGetCurrRecord a Value read and SetValue read nothing, and the action
        // after them reads the row twice (pages/lt-ai-list-order/QgGoToKeyNextCall.al:223 Plain_GoToKey_ThenValueRead `no_log`,
        // `qty_log` empty; :236 Plain_GoToKey_ThenSetValue_ThenAction `setvalue_log` empty; :317 _ThenValueRead_ThenAction; the Nav_
        // tests). The other field reads (properties, `As*`) read nothing more either (not pinned; kept for baseline cases 78862
        // Paging_GoToKey_Nav and 68164 TestPage_Control_Texts_GuidDurationAndBlankNumbers), and an action after them is refused
        // (`actions.rs`). Any other control call is refused.
        if m != PageMethod::Invoke && st.borrow().read_after_goto {
            // AsBoolean and AsInteger of a field, Visible, Editable and Caption of a field and Enabled of an action read
            // nothing either, and the action after them reads the row twice as after a Value read (pages/lt-wd/
            // WdGoToKeyTypedReads.al:124-247, corpus branch al-runtime-rs-staging-1030c, al-runner issue 6304). Pinned on a
            // page that is not a lookup; OK of a lookup after them is not pinned, so there they still mark the doubt.
            let kind = match &c.target {
                ControlTarget::Control(i) => Some(page_of(&self.engine(), st).controls[*i].kind.clone()),
                _ => None,
            };
            let pinned_read = match kind {
                Some(ir::ControlKind::Field) => matches!(
                    m,
                    PageMethod::AsBoolean
                        | PageMethod::AsInteger
                        | PageMethod::Visible
                        | PageMethod::Editable
                        | PageMethod::Caption
                ),
                Some(ir::ControlKind::Action) => m == PageMethod::Enabled,
                _ => false,
            };
            match m {
                PageMethod::Value => {}
                _ if pinned_read && !st.borrow().lookup => {}
                PageMethod::SetValue => st.borrow_mut().read_after_goto_set = true,
                PageMethod::AsBoolean
                | PageMethod::AsDate
                | PageMethod::AsDateTime
                | PageMethod::AsDecimal
                | PageMethod::AsInteger
                | PageMethod::AsTime
                | PageMethod::ShowMandatory
                | PageMethod::Visible
                | PageMethod::Editable
                | PageMethod::Enabled
                | PageMethod::Caption => st.borrow_mut().read_after_goto_doubt = true,
                _ => {
                    return unsupported(format!(
                        "TestField.{m} right after a GoToKey moved a list without OnAfterGetCurrRecord (whether the row is read again is not pinned)"
                    ));
                }
            }
        }
        // Only a field read may come between the refresh that settled a list and a First that reads nothing (AiListOrderTests.al:
        // 163-166, :336-338).
        if !matches!(
            m,
            PageMethod::Value
                | PageMethod::AsBoolean
                | PageMethod::AsDate
                | PageMethod::AsDateTime
                | PageMethod::AsDecimal
                | PageMethod::AsInteger
                | PageMethod::AsTime
        ) {
            st.borrow_mut().settled_first_row = false;
        }
        // A card whose OnNewRecord inserted its new row: reading a control and writing a field outside the key are pinned
        // (corpus pages/lt-cf-page-leads/CfPageLeads.al:241-289 Card_OnNewRecordInserts_OpenNewSetClose); the rest is refused.
        if st.borrow().new_row_inserted_by_trigger {
            let pinned = match (&c.target, m) {
                (ControlTarget::Control(_), PageMethod::Value) => true,
                (ControlTarget::Control(i), PageMethod::SetValue) => self.control_outside_key(st, *i),
                _ => false,
            };
            if !pinned {
                return unsupported(format!(
                    "TestField.{m} on a card whose OnNewRecord inserted the new row itself (not pinned)"
                ));
            }
        }
        let idx = match &c.target {
            ControlTarget::Control(i) => *i,
            ControlTarget::BuiltIn(name) if matches!(name.as_str(), "Edit" | "View") => {
                // On a card the built-in Edit and View switch the open page's mode in place; both stay
                // visible, and only the one the current mode does not already have is enabled (corpus
                // handlers/TestPageModeSwitchAction_Tests.al:Claim 1 and 2). A list opens its card
                // instead, like a RunObject action with RunPageOnRec, answered by a page handler (corpus
                // handlers/TestPageListViewEditAction_Tests.al:ListViewActionOpensTheCardOnTheListsCurrentRowReadOnly,
                // ListEditActionOpensTheCardOnTheListsCurrentRowEditable). Whether the action is visible or
                // enabled on a list is not pinned.
                if self.is_list(st) {
                    // Both actions are visible on a list whether or not it names a card; View is
                    // enabled, Edit only when the list names a card that can be edited; a disabled
                    // action does nothing, and so does View on a list with no card (corpus
                    // handlers/TestPageModeSwitchAction_Tests.al:
                    // PlainListWithoutCardPageIdOffersBothActionsAndEnablesOnlyView,
                    // ListWhoseCardIsReadOnlyLeavesTheEditActionVisibleButNotEnabled,
                    // ListWithAnEditableCardPageIdEnablesBothActions).
                    let edit = name == "Edit";
                    let card = self.list_card_page(st)?;
                    // A list without a card switches the mode of its own page, as a card does: opened for view,
                    // Edit is enabled and View is not; opened for edit, the other way round. A list that names a
                    // card with no row to open it for has both disabled (misc/W2ListActions.al:NoCard_WithRows,
                    // List_NoRows, BC 27.5 and 29.0).
                    let no_rows = st.borrow().cursor == Cursor::None;
                    let editable = st.borrow().mode != PageMode::View;
                    let enabled = match (edit, card) {
                        (_, None) => edit == !editable,
                        (_, Some(_)) if no_rows => false,
                        (false, _) => true,
                        (true, Some(target)) => self.card_allows_edit(target)?,
                    };
                    return match m {
                        PageMethod::Visible => Ok(Value::Boolean(true)),
                        PageMethod::Enabled => Ok(Value::Boolean(enabled)),
                        PageMethod::Invoke if !enabled || card.is_none() => Ok(Value::Null),
                        PageMethod::Invoke => self.list_card_action(st, edit),
                        other => unsupported(format!(
                            "TestAction.{other} of the built-in {name} action of a list page"
                        )),
                    };
                }
                let editable = st.borrow().mode != PageMode::View;
                let applies = (name == "Edit") == !editable;
                return match m {
                    PageMethod::Visible => Ok(Value::Boolean(true)),
                    PageMethod::Enabled => Ok(Value::Boolean(applies)),
                    PageMethod::Invoke => {
                        if applies {
                            let mode = if name == "Edit" { PageMode::Edit } else { PageMode::View };
                            st.borrow_mut().mode = mode;
                            // The parts follow the host (corpus handlers/TestPageControlEditableRoutes.al:
                            // Part_HostOpenEdit_ThenViewAction).
                            self.set_part_modes(st, mode);
                        }
                        Ok(Value::Null)
                    }
                    other => unsupported(format!("TestAction.{other}")),
                };
            }
            ControlTarget::Filter => {
                return self.test_filter_op(m, st, args);
            }
            ControlTarget::BuiltIn(name) if m == PageMethod::Invoke => {
                // OK of a lookup-mode page after a GoToKey that moved it (a list without OnAfterGetCurrRecord) reads the row once,
                // also after a Value read between them (pages/lt-ai-list-order/QgGoToKeyNextCall.al:404
                // Plain_LookupHandler_GoToKey_ThenOK `H:goto;R:R1003;H:ok`, :426 Plain_LookupHandler_GoToKey_ValueRead_ThenOK, :629 Nav_LookupHandler_GoToKey_ThenOK).
                // Other built-in actions after it are not pinned.
                if std::mem::take(&mut st.borrow_mut().read_after_goto) {
                    let doubt = {
                        let mut s = st.borrow_mut();
                        s.read_after_goto_quiet = false;
                        s.read_after_goto_set = false;
                        s.read_after_goto_prev = false;
                        std::mem::take(&mut s.read_after_goto_doubt)
                    };
                    if name != "OK" || !st.borrow().lookup || doubt {
                        return unsupported(format!(
                            "TestAction {name} after a GoToKey moved a list without OnAfterGetCurrRecord (whether the row is read again is not pinned)"
                        ));
                    }
                    let rec = self.page_rec(st);
                    if self.stored_row(&rec)?.is_some() {
                        self.populate_current(st)?;
                    }
                }
                // OK and Yes confirm, Cancel and No cancel; a refused save keeps the page open
                // (corpus testpageclose/TestPageClosingActions.al).
                // The client reads the row of a card before Cancel, as before any action (pages/lt-page-runtime/P82PageRun.al:
                // RunModal_NumberAndRecord_Cancel `OnAfterGetRecord(C);OnAfterGetRecord(C);OnQueryClosePage(LookupCancel)`); OK reads nothing
                // (RunModal_Number_OnCard).
                if name == "Cancel"
                    && self.has_source(st)
                    && !self.is_list(st)
                    && !self.is_request_page(st)
                    && st.borrow().cursor == Cursor::Row
                    && !st.borrow().draft
                    && self.declares(st, "onaftergetrecord")
                {
                    self.populate_current(st)?;
                    // A page with OnAfterGetCurrRecord runs it after that read (errors/lt-ms-refusals-129/P129Handlers.al:
                    // P129_TwoTestPageHandlers_SecondPageOpenedByAnActionOfTheFirstHandler: `card AGR 1;card AGCR 1;card OnQueryClosePage`).
                    if self.declares(st, "onaftergetcurrrecord") {
                        self.page_trigger(st, "onaftergetcurrrecord", vec![])?;
                        self.page_event(st, "onaftergetcurrrecordevent", None, None)?;
                    }
                }
                // Yes on a ConfirmationDialog whose control still holds a refused entry closes it as Cancel; the values it took stay
                // (pages/lt-y37-values-allowed/Y37ValuesAllowedSetValue.al:193-196, :225-226; without a refused entry Yes reports Yes,
                // misc/lt-probes-needed-7/PnGActionOrdinals.al:104). With an OnQueryClosePage what it sees is not pinned.
                let dialog =
                    self.engine().object_facts[st.borrow().object as usize].page_type_lower() == "confirmationdialog";
                if name == "Yes" && dialog && !st.borrow().errors.is_empty() {
                    if self.declares(st, "onqueryclosepage") {
                        return unsupported(
                            "Yes on a ConfirmationDialog with OnQueryClosePage that holds a refused entry (not pinned)",
                        );
                    }
                    return self.close_with(st, "Cancel", true, true);
                }
                return self.close_with(st, name, true, true);
            }
            // The built-in OK and Cancel are visible (report/TestProbeReportContracts.al:RequestPage_InvokeActionNotFound).
            ControlTarget::BuiltIn(_) if m == PageMethod::Visible => return Ok(Value::Boolean(true)),
            _ => return unsupported(format!("{method} on a built-in control")),
        };
        // The data items and actions of a report's request page.
        if self.is_request_page(st) {
            let kind = page_of(&self.engine(), st).controls[idx].kind.clone();
            match kind {
                ir::ControlKind::DataItem => return self.request_item_op(method, st, idx, args),
                // Visible and Enabled do not find the action either, whatever its own Visible and Enabled say
                // (report/W3ReportActions.al:186); the other members of an action are not pinned.
                ir::ControlKind::Action
                    if matches!(m, PageMethod::Invoke | PageMethod::Visible | PageMethod::Enabled) =>
                {
                    return self.request_action_not_found(st, idx);
                }
                ir::ControlKind::Action => {
                    return unsupported(format!("TestAction.{m} on an action of a request page (not pinned)"));
                }
                _ => {}
            }
        }
        // The compiler emits no call for a `Value()` whose result is thrown away: nothing is
        // evaluated, on a control that is on the page or not (corpus
        // pageapplicationarea/TestPageDiscardedFieldValueCall.al: RemovedControl_DiscardedValueCall_RaisesNothing,
        // FoundControl_DiscardedValueCall_RaisesNothing).
        if m == PageMethod::Value && args.is_empty() && !self.ret_used {
            return Ok(Value::Null);
        }
        // A control whose Visible is the literal false, or that sits in a group whose Visible is,
        // is not on the runtime page at all (corpus handlers/TestPageFieldVisibleGroup_Tests.al:1-30).
        // Neither is one that application areas removed.
        self.check_areas_pinned(st, idx)?;
        // From BC 28 a page extension's control whose source is a bare quoted name of a table extension's field is not found
        // by a TestPage read of its Value (pages/lt-wd/WdBareQuotedField.al:185-198 WD_BareNameExt_ValueReadError, `The field
        // with ID = n is not found on the page.`, TestFieldNotFound). Its other methods are not pinned (Editable() raises the
        // same error on real BC, the file's header; not asserted).
        let bare_ext = self.has(al_version::Gate::ExtControlBareQuotedSourceNotFoundFrom28)
            && page_of(&self.engine(), st).controls[idx].bare_quoted_ext_source;
        if bare_ext && !(m == PageMethod::Value && args.is_empty()) {
            return unsupported(format!(
                "TestPage.{}() of a page extension control whose source is a bare quoted name of a table extension field (not pinned)",
                m.name()
            ));
        }
        if bare_ext
            || self.control_eliminated(st, idx)
            || self.area_removed(st, idx)
            || self.action_without_behaviour(st, idx)
        {
            let engine = self.engine();
            let ctl = &page_of(&engine, st).controls[idx];
            let what = match ctl.kind {
                ir::ControlKind::Action | ir::ControlKind::SystemAction => "action",
                _ => "field",
            };
            // The id in the text is generated from the declaring object and the control name
            // (see `control_id`); it matches the ids the corpus pins for Base Application controls
            // (pageapplicationarea/TestPageApplicationAreaPrecompiledControlRemoval.al:57, :92, :124)
            // and for a part (testpart/TestTestPart.al:233). The code of a field is pinned by
            // handlers/TestPageRuntimeContracts.al:260; that of an action is TestActionNotFound
            // (handlers/TestPageLabelControlContracts.al:141-145).
            let ctl_name = ctl.name.clone();
            let owner = ctl.owner;
            let object = st.borrow().object as usize;
            let declaring = self.object_id_of(owner.unwrap_or(object) as u32);
            let id = control_id(declaring, &ctl_name);
            return Err(match what {
                "action" => error_code(
                    format!("The action with ID = {id} is not found on the page."),
                    "TestActionNotFound",
                ),
                _ => error_code(
                    format!("The field with ID = {id} is not found on the page."),
                    "TestFieldNotFound",
                ),
            });
        }
        // Only Invoke of an actionref is pinned: it runs the action it stands for. A found actionref
        // reads Visible true when neither it nor its target states a Visible (corpus
        // pageapplicationarea/TestPageApplicationAreaExtensionActionPartRemoval.al:
        // ExtActionRef_TargetEnabled_IsFound, ExtActionRef_TargetNotEnabled_IsNotFound).
        if m != PageMethod::Invoke {
            let engine = self.engine();
            let page = page_of(&engine, st);
            let ctl = &page.controls[idx];
            if let Some(action_ref) = &ctl.action_ref {
                let target_plain = match action_ref {
                    ir::ActionRef::Target(t) => page.controls[*t].prop("visible").is_none(),
                    ir::ActionRef::Unresolved(_) => false,
                };
                if m == PageMethod::Visible && target_plain && ctl.prop("visible").is_none() {
                    return Ok(Value::Boolean(true));
                }
                // handlers/TestPageRunObjectContracts.al:392-404: Visible and Enabled of an actionref answer Yes.
                let target_enabled_plain = match action_ref {
                    ir::ActionRef::Target(t) => page.controls[*t].prop("enabled").is_none(),
                    ir::ActionRef::Unresolved(_) => false,
                };
                if m == PageMethod::Enabled && target_enabled_plain && ctl.prop("enabled").is_none() {
                    return Ok(Value::Boolean(true));
                }
                return unsupported(format!("TestAction.{m} of an actionref"));
            }
        }
        match m {
            // `Ctl.Value := x` is the setter form of the property, and does what SetValue does
            // (corpus handlers/ProbeTestPageForms.al:109-131: OnValidate runs with the assigned text).
            PageMethod::Value if !args.is_empty() && self.in_cue_group(st, idx) => Ok(Value::Null),
            PageMethod::Value if !args.is_empty() => {
                let before = self.has_source(st).then(|| self.page_rec(st).borrow().fields.clone());
                let r = self.control_set_value(st, idx, args)?;
                self.resync_parts_linked_by(st, idx, before.as_deref())?;
                self.refresh_host_after_part_edit(st)?;
                Ok(r)
            }
            PageMethod::Value => {
                if self.shows_no_row(st, idx) {
                    return Ok(Value::Text(String::new().into()));
                }
                let v = self.control_value(st, idx)?;
                // A Text control in a cuegroup with CuegroupLayout = Wide reads '0' (pages/lt-ab-group-properties/ABLayouts.al:68,
                // :70, also written `wide`); a Decimal one reads its value (pages/lt-page-controls-aa/AaControls.al:422). Other
                // types there are not pinned.
                if self.in_wide_cue_group(st, idx) {
                    match &v {
                        Value::Text(_) => return Ok(Value::Text("0".into())),
                        Value::Decimal(_) => {}
                        _ => {
                            return unsupported(format!(
                                "Value of a {} control in a cuegroup with CuegroupLayout = Wide (not pinned)",
                                v.describe()
                            ));
                        }
                    }
                }
                // An Enum control that refused an entry shows the ordinal, not the caption (controls/W2ValueControls.al:
                // Enum_VariableControl_*, `value` 0 after a refused `Alpha`, BC 27.5 and 29.0).
                // An Option control does the same after the empty text was refused (pages/lt-testpage-assertequals-option/
                // TestPageAssertEqualsOption.al:SetValue_BlankOption_AsEmptyText, `afterEmpty` [1]); other refused option texts are not pinned.
                if let Value::Enum(_, ordinal) | Value::Option(ordinal, _) = &v {
                    if st.borrow().errors.iter().any(|(i, _)| *i == idx) {
                        let refused_at = st.borrow().values_allowed_refused_row.clone();
                        if let Some((_, key)) = refused_at.filter(|(i, _)| *i == idx) {
                            if !Self::same_values(&self.row_key(&self.page_rec(st))?, &key) {
                                return unsupported(
                                    "Value of a list control on another row than the one its ValuesAllowed refused an entry on (not pinned)",
                                );
                            }
                        }
                        return Ok(Value::Text(ordinal.to_string().into()));
                    }
                    // A row value outside the control's ValuesAllowed shows the ordinal on a list (pages/lt-y37-values-allowed/
                    // Y37ValuesAllowedSetValue.al:302, :322 `4` for Finished); elsewhere it is not pinned.
                    if page_of(&self.engine(), st).controls[idx]
                        .prop("valuesallowed")
                        .is_some()
                    {
                        let ty = self.control_type(st, idx, &v);
                        if let Some(ty) = ty.filter(|t| matches!(t, RtType::Enum(_) | RtType::Option(_))) {
                            let allowed = self.values_allowed(st, idx, &ty)?.unwrap_or_default();
                            if !allowed.contains(ordinal) {
                                if !self.is_list(st) {
                                    return unsupported(
                                        "Value of a control whose value is outside its ValuesAllowed, not on a list (not pinned)",
                                    );
                                }
                                return Ok(Value::Text(ordinal.to_string().into()));
                            }
                        }
                    }
                }
                let text = self.control_text(st, idx, &v)?;
                Ok(Value::Text(text.into()))
            }
            PageMethod::AsInteger => self.control_as(st, idx, &RtType::Integer),
            PageMethod::AsDecimal => self.control_as(st, idx, &RtType::Decimal),
            PageMethod::AsBoolean => self.control_as(st, idx, &RtType::Boolean),
            PageMethod::AsDate => self.control_as(st, idx, &RtType::Date),
            PageMethod::AsTime => self.control_as(st, idx, &RtType::Time),
            PageMethod::AsDateTime => self.control_as(st, idx, &RtType::DateTime),
            PageMethod::AssertEquals => self.control_assert_equals(st, idx, args),
            PageMethod::Caption => Ok(Value::Text(self.control_caption(st, idx)?.into())),
            PageMethod::OptionCount | PageMethod::GetOption => {
                // ValuesAllowed on an Enum control, or on the table field it shows, leaves only the allowed values as options, in
                // the enum's order (misc/lt-y13-triage/Y13PageTriage.al:197-243, corpus master: `Open, Released` on the control
                // and `Open, Closed` on the field, 2 options each). On an Option control, and with the property on both, it is
                // not pinned.
                let control_allowed = match page_of(&self.engine(), st).controls[idx].prop("valuesallowed") {
                    None => None,
                    Some(ir::PropVal::Lit(v)) => Some(v.clone()),
                    Some(_) => {
                        return unsupported(format!(
                            "TestField.{m} on a control with a computed ValuesAllowed (not pinned)"
                        ));
                    }
                };
                let field_allowed = self.control_field(st, idx).and_then(|f| {
                    f.properties
                        .iter()
                        .find(|(k, _)| k.eq_ignore_ascii_case("ValuesAllowed"))
                        .map(|(_, v)| v.clone())
                });
                let allowed = match (control_allowed, field_allowed) {
                    (Some(_), Some(_)) => {
                        return unsupported(format!(
                            "TestField.{m} on a control with ValuesAllowed over a field with ValuesAllowed (not pinned)"
                        ));
                    }
                    (a, b) => a.or(b),
                };
                let current = self.control_value(st, idx)?;
                let engine = self.engine();
                let members: Vec<i32> = match self.control_type(st, idx, &current) {
                    Some(RtType::Option(_)) if allowed.is_some() => {
                        return unsupported(format!(
                            "TestField.{m} on an Option control with ValuesAllowed: whether only the allowed values are options is not pinned"
                        ));
                    }
                    Some(RtType::Option(names)) => (0..names.len() as i32).collect(),
                    Some(RtType::Enum(id)) => {
                        self.note_enum(id);
                        if self.enum_option_list(st, idx, id)?.is_some() {
                            return unsupported(format!(
                                "TestField.{m} on an Enum control with an OptionCaption or a caption with a comma (not pinned)"
                            ));
                        }
                        let values = &engine.program.objects[id as usize].enum_values;
                        match &allowed {
                            None => values.iter().map(|(v, _)| *v).collect(),
                            Some(list) => match allowed_enum_values(list, values) {
                                Some(v) => v,
                                None => {
                                    return unsupported(format!(
                                        "TestField.{m} on an Enum control with ValuesAllowed = {} (only names of the enum in its order are pinned)",
                                        list.trim()
                                    ));
                                }
                            },
                        }
                    }
                    _ => return unsupported(format!("TestField.{m} on a control that is not an option")),
                };
                if m == PageMethod::OptionCount {
                    return Ok(Value::Integer(members.len() as i32));
                }
                let i = match args.first() {
                    Some(Value::Integer(i)) => *i,
                    _ => return unsupported("GetOption without an index"),
                };
                let Some(ordinal) = i
                    .checked_sub(1)
                    .and_then(|n| usize::try_from(n).ok())
                    .and_then(|n| members.get(n))
                else {
                    return unsupported(format!("GetOption({i}) outside the options (error text not pinned)"));
                };
                let v = match self.control_type(st, idx, &current) {
                    Some(RtType::Option(names)) => Value::Option(*ordinal, names.into()),
                    Some(RtType::Enum(id)) => Value::Enum(id, *ordinal),
                    _ => unreachable!(),
                };
                Ok(Value::Text(self.control_text(st, idx, &v)?.into()))
            }
            // The properties as written; a computed one is not read (docs only).
            PageMethod::HideValue | PageMethod::ShowMandatory => {
                let name = if m == PageMethod::HideValue {
                    "hidevalue"
                } else {
                    "showmandatory"
                };
                let engine = self.engine();
                match page_of(&engine, st).controls[idx].prop(name) {
                    // ShowMandatory is true when the page says nothing (handlers/TestProbePageHandlerContracts.al:
                    // TestPage_Members_ShowMandatoryHideValue_Defaults); HideValue is false.
                    // A label shows no mandatory mark (misc/lt-probes-needed-7/PnGControlKinds.al:LabelValueAndCaption `label-mandatory = No`).
                    None if matches!(page_of(&engine, st).controls[idx].binding, ir::Binding::Caption) => {
                        Ok(Value::Boolean(false))
                    }
                    // ShowMandatory without the property is Yes when another control of the page shows the same table field with
                    // ShowMandatory = true, else No (misc/lt-probes-needed-7/PnGFindField.al:HideValueAndShowMandatoryLiterals:
                    // `PlainGrpCtl` Yes next to `MandatoryCtl` on the same field, `NameCtl` No; pages/lt-page-runtime/P82TestField.al:
                    // Properties_DynamicVisible_And_ShowMandatory `no-showmandatory` No; handlers/TestProbePageHandlerContracts.al:
                    // TestPage_Members_ShowMandatoryHideValue_Defaults `name_showmandatory` Yes next to `ShowMandatoryField`). For
                    // a control bound to something else than a table field the answer is not pinned.
                    None if m == PageMethod::ShowMandatory => {
                        let page = page_of(&engine, st);
                        match &page.controls[idx].binding {
                            ir::Binding::Field(pos) => {
                                let mut any = false;
                                for c in page
                                    .controls
                                    .iter()
                                    .filter(|c| matches!(&c.binding, ir::Binding::Field(p) if p == pos))
                                {
                                    match c.prop("showmandatory") {
                                        None => {}
                                        Some(ir::PropVal::Lit(l)) => any |= lit_bool("showmandatory", l)?,
                                        Some(_) => {
                                            return unsupported(
                                                "ShowMandatory of a control next to a computed ShowMandatory on the same field",
                                            );
                                        }
                                    }
                                }
                                Ok(Value::Boolean(any))
                            }
                            _ => unsupported(
                                "ShowMandatory of a control that is no table field, without the property (not pinned)",
                            ),
                        }
                    }
                    None => Ok(Value::Boolean(false)),
                    Some(ir::PropVal::Lit(l)) => Ok(Value::Boolean(lit_bool(name, l)?)),
                    Some(_) => unsupported(format!("TestField.{m} of a computed property")),
                }
            }
            // A field of a part also answers what its part control says (see `part_field_gate`).
            PageMethod::Editable => match self.part_field_gate(st, true)? {
                Some(b) => Ok(Value::Boolean(b)),
                None if self.read_only_expression_control(st, idx) => Ok(Value::Boolean(false)),
                None => Ok(Value::Boolean(self.control_editable(st, idx)?)),
            },
            PageMethod::Enabled => match self.part_field_gate(st, false)? {
                Some(b) => Ok(Value::Boolean(b)),
                None => Ok(Value::Boolean(self.control_enabled(st, idx)?)),
            },
            PageMethod::Visible => Ok(Value::Boolean(self.control_visible(st, idx)?)),
            PageMethod::Invoke => self.action_invoke(st, idx),
            PageMethod::DrillDown | PageMethod::AssistEdit | PageMethod::Lookup => self.control_navigate(st, idx, m),
            // SetValue on a field of a cuegroup does nothing and raises nothing
            // (handlers/TestPageCueGroupFixedGridContracts.al:CueGroup_SetValue_IsSilentlyIgnored).
            PageMethod::SetValue if self.in_cue_group(st, idx) => Ok(Value::Null),
            // A control bound to an expression that is no place, on a card without a source table, is not editable, and SetValue
            // on it raises nothing and leaves the value as it was (misc/lt-protected-var-abs-query-views/P146PageExprSetValue.al:506
            // QZ_NoSourceTable_SetValue_OnExpressionControls: GetTotal(), ValA + ValB, UpperCase(PlainText)).
            PageMethod::SetValue if self.read_only_expression_control(st, idx) => Ok(Value::Null),
            PageMethod::SetValue => {
                let before = self.has_source(st).then(|| self.page_rec(st).borrow().fields.clone());
                let r = self.control_set_value(st, idx, args)?;
                self.resync_parts_linked_by(st, idx, before.as_deref())?;
                self.refresh_host_after_part_edit(st)?;
                Ok(r)
            }
            PageMethod::Activate => {
                let (activated, origin) = self.traced(st, |s| s.activate(st, idx));
                if let (Err(Signal::Error(_)), Some("oninsertrecord")) = (&activated, origin.as_deref()) {
                    return unsupported("an error in OnInsertRecord on Activate() (only SetValue is pinned)");
                }
                activated?;
                // The host is stored: its part shows the rows of the new key (Y17ActivateNewHost.al:382 `LP.OnAfterGetCurrRecord:AUTO1|0`
                // x3, then :408 part First `No`, :442 SetValue and OK as on any stored host).
                if st.borrow().new_host_part_rows && !st.borrow().draft {
                    st.borrow_mut().new_host_part_rows = false;
                }
                Ok(Value::Null)
            }
            PageMethod::ValidationErrorCount => {
                let n = st.borrow().errors.iter().filter(|(i, _)| *i == idx).count();
                Ok(Value::Integer(n as i32))
            }
            PageMethod::GetValidationError => {
                let i = match args.first() {
                    Some(Value::Integer(i)) => *i,
                    // Without an index while the control has a validation error, real BC ends the session with an
                    // uncatchable ArgumentOutOfRangeException (Windows check of the corpus, runs 39162782959 and
                    // 39162785239); the runtime has no such error to raise. Without an error the answer is not pinned.
                    None if st.borrow().errors.iter().any(|(c, _)| *c == idx) => {
                        return unsupported(
                            "TestField.GetValidationError() without an index while the control has a validation error (real BC crashes with an uncatchable ArgumentOutOfRangeException)",
                        );
                    }
                    None => {
                        return unsupported(
                            "TestField.GetValidationError() without an index on a control with no validation error (not pinned)",
                        );
                    }
                    _ => return unsupported("GetValidationError with a non-integer index"),
                };
                let s = st.borrow();
                let mine: Vec<&String> = s.errors.iter().filter(|(c, _)| *c == idx).map(|(_, t)| t).collect();
                // An index past the control's ledger escapes AL as an untrappable CLR exception
                // (corpus handlers/TestPageValidationError_Tests.al:1-40), so it is not modelled.
                match usize::try_from(i - 1).ok().and_then(|n| mine.get(n)) {
                    Some(t) => Ok(Value::Text((*t).clone().into())),
                    None => unsupported("GetValidationError past the ledger"),
                }
            }
            other => unsupported(format!("TestField.{other}")),
        }
    }
}

/// Records a refused entry of control `idx` in the page's ledger. A second refusal on the control replaces its first
/// (pages/lt-ah-pages/AhPageTests.al:444-535, GVE_TwoOnOne_*: two refusals on QtyCtl count once and the later text stays,
/// and report/lt-report-requestpage-query-members/P105RpValid.al:135). Where another control's refusal came in between,
/// the order the client then keeps is not pinned: refused.
/// The ordinals a list such as `ValuesAllowed = 0, None, Partial` names: each entry a value name (ignoring case and double
/// quotes) or the ordinal of a value, in any order (pages/lt-y37-values-allowed/Y37ValuesAllowedSetValue.al:132). `None` when an
/// entry names no value.
fn allowed_ordinals(list: &str, values: &[(i32, String)]) -> Option<Vec<i32>> {
    list.split(',')
        .map(|e| {
            let e = e.trim();
            let name = e.trim_matches('"');
            values
                .iter()
                .find(|(_, n)| n.eq_ignore_ascii_case(name))
                .or_else(|| e.parse::<i32>().ok().and_then(|o| values.iter().find(|(v, _)| *v == o)))
                .map(|(v, _)| *v)
        })
        .collect()
}

/// The ordinals of `ValuesAllowed = Open, Released` of an Enum with `values` (ordinal, name): the names match the enum's values,
/// ignoring case and double quotes, in the enum's order. `None` for any other list (an ordinal, an unknown name, another order).
fn allowed_enum_values(list: &str, values: &[(i32, String)]) -> Option<Vec<i32>> {
    let mut out = Vec::new();
    let mut from = 0;
    for name in list.split(',') {
        let name = name.trim().trim_matches('"');
        let at = values[from..].iter().position(|(_, n)| n.eq_ignore_ascii_case(name))? + from;
        out.push(values[at].0);
        from = at + 1;
    }
    Some(out)
}

/// The values of a property such as `ValuesAllowed = '', 'A', 'N'`, unquoted; `None` when one is not a quoted text.
fn quoted_list(raw: &str) -> Option<Vec<String>> {
    let mut out = Vec::new();
    let mut chars = raw.trim().chars().peekable();
    loop {
        while chars.peek().is_some_and(|c| c.is_whitespace()) {
            chars.next();
        }
        if chars.next()? != '\'' {
            return None;
        }
        let mut v = String::new();
        loop {
            match chars.next()? {
                '\'' if chars.peek() == Some(&'\'') => {
                    chars.next();
                    v.push('\'');
                }
                '\'' => break,
                c => v.push(c),
            }
        }
        out.push(v);
        while chars.peek().is_some_and(|c| c.is_whitespace()) {
            chars.next();
        }
        match chars.next() {
            None => return Some(out),
            Some(',') => {}
            Some(_) => return None,
        }
    }
}

fn note_refusal(st: &St, idx: usize, text: String) -> Result<(), Signal> {
    let mut s = st.borrow_mut();
    match s.errors.iter().position(|(i, _)| *i == idx) {
        None => s.errors.push((idx, text)),
        Some(at) if at + 1 == s.errors.len() && s.errors.iter().filter(|(i, _)| *i == idx).count() == 1 => {
            s.errors[at].1 = text;
            s.trigger_errors.retain(|(i, _)| *i != idx);
        }
        Some(_) => {
            return Err(Signal::Unsupported(
                "a second refusal on a control after another control's refusal (the order of the errors is not pinned)"
                    .into(),
            ));
        }
    }
    Ok(())
}

/// Marks assumption page-expression-control-setvalue for a write through a control's expression source, but where it is
/// pinned: a TestField.Lookup on the page's own control bound to a field of a page global record writes that field, then
/// runs the control's OnValidate (pages/lt-sc-outside-pages/ScLookupGlobalRecordField.al: Value() shows R2 and the saved
/// setting row holds it; al-runner issue 6519).
fn mark_expression_write(st: &St, ctl: &ir::Control) {
    let pinned = ctl.owner.is_none() && ctl.var_field.is_some() && st.borrow().lookup_setting_value;
    if !pinned {
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::PageExpressionControlSetValue);
    }
}

/// Whether the `modify` triggers `before` (OnBeforeValidate) and `after` (OnAfterValidate) of a control bound to a variable are a
/// shape nothing pins: any OnBeforeValidate, and OnAfterValidate outside a request page (al-runner issue 6651; corpus branch
/// al-runtime-rs-staging-y64 85d86e7b, reportext/lt-y64-local-proc-names/Y64LocalProcNames.al:445 `ext rp MyProc<-ext aftervalidate`
/// after `base MyProc<-base validate`).
fn unpinned_ext_validate(before: &[(usize, usize)], after: &[(usize, usize)], request_page: bool) -> bool {
    !before.is_empty() || (!after.is_empty() && !request_page)
}
