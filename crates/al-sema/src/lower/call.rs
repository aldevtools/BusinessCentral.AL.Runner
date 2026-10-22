//! Lowering of calls.
use super::*;

impl Lowerer<'_> {
    /// `Rec.SecurityFiltering()` read on a variable declared with `[SecurityFiltering(...)]` answers the declared value on
    /// BC (al-language/lt-ac/ACAttributes.al:176-191); the runtime keeps it only for a local record
    /// (`Lowerer::declared_security_filtering`), so the read is refused on anything else.
    fn security_filtering_read_refusal(&self, callee: &SyntaxNode, args: &[SyntaxNode]) -> L<()> {
        if !args.is_empty() || callee.kind() != SyntaxKind::MemberExpr {
            return Ok(());
        }
        let mut kids = callee.children();
        let (Some(recv), Some(member)) = (kids.next(), kids.next()) else {
            return Ok(());
        };
        if !member
            .text()
            .to_string()
            .trim()
            .eq_ignore_ascii_case("SecurityFiltering")
        {
            return Ok(());
        }
        let attrs = match self.res(&recv) {
            Some(Res::Local(n)) => self.local_var_attrs.get(n),
            Some(Res::Global(n)) => self.global_var_attrs.get(n),
            _ => None,
        };
        if attrs.is_some_and(|a| a.iter().any(|a| a == "securityfiltering")) {
            return Err(format!(
                "SecurityFiltering() of `{}`, declared with the attribute SecurityFiltering: the declared value is not kept",
                recv.text().to_string().trim()
            ));
        }
        Ok(())
    }

    /// `--tdd` (ADR-051 decision 4): a stub site becomes `ir::Expr::TddStub`; one whose return type is not known is
    /// refused with the reason the checker gave.
    fn tdd_stub(&mut self, site: &crate::check::tdd::TddSite, args: &[SyntaxNode]) -> L<ir::Expr> {
        use crate::check::tdd::TddRet;
        let ty = match &site.ret {
            TddRet::Refused(why) => return Err(why.clone()),
            TddRet::Pending => {
                return Err(format!(
                    "--tdd: cannot infer the return type of {} from its use",
                    site.shown()
                ));
            }
            TddRet::NoValue => None,
            TddRet::Value(t) => Some(self.rt(t)?),
        };
        let stub = crate::check::tdd::signature(self.ws, site).ok_or("--tdd: a stub without a signature")?;
        let args = args.iter().map(|a| self.expr(a)).collect::<L<Vec<_>>>()?;
        Ok(ir::Expr::TddStub { stub, args, ty })
    }

    /// A call resolved on `node` (the CallExpr, or the callee for a call without parentheses).
    pub(crate) fn call(&mut self, node: &SyntaxNode, callee: &SyntaxNode, args: &[SyntaxNode]) -> L<ir::Expr> {
        if let Some(Res::TddStub(site)) = self.res(node).cloned() {
            return self.tdd_stub(&site, args);
        }
        if let Some(Res::DotNetMember(name)) = self.res(node).cloned() {
            let args = args.iter().map(|a| self.dotnet_arg(a)).collect::<L<Vec<_>>>()?;
            let op = if node.kind() == SyntaxKind::CallExpr {
                ir::DotNetOp::Call(name.clone())
            } else {
                ir::DotNetOp::Get(name.clone())
            };
            return self.dotnet_member(node, callee, &name, op, args);
        }
        self.security_filtering_read_refusal(callee, args)?;
        // `this.Proc(...)` calls on the running instance, as the bare call does.
        let receiver_node = (callee.kind() == SyntaxKind::MemberExpr)
            .then(|| callee.children().next())
            .flatten()
            .filter(|r| !crate::check::is_this_keyword(r));
        if let Some(Res::Builtin(id)) = self.res(node) {
            self.coverage_test_id_call(id, receiver_node.as_ref(), args)?;
        }
        match self.res(node).cloned() {
            Some(Res::Proc { object, offset }) if self.ws.objects[object].kind == ObjKind::Interface => {
                self.interface_call(object, offset, receiver_node.as_ref(), args)
            }
            Some(Res::Proc { object, offset }) => {
                let routine = self.routine_at.get(&(object, offset)).ok_or("procedure not found")?;
                let sig = self.members[object]
                    .procedures
                    .iter()
                    .find(|p| p.offset == offset)
                    .ok_or("procedure signature")?
                    .clone();
                let instance = match &receiver_node {
                    // `this.Proc()` runs on the current instance, like a bare call.
                    Some(r)
                        if r.kind() == SyntaxKind::NameRef
                            && r.text().to_string().trim().eq_ignore_ascii_case("this") =>
                    {
                        if object != self.oi {
                            return Err("`this` calls a procedure of another object".into());
                        }
                        None
                    }
                    Some(r)
                        if matches!(
                            self.ty_of(r),
                            Ty::Object(
                                ObjKind::Codeunit | ObjKind::Page | ObjKind::XmlPort | ObjKind::Report | ObjKind::Query,
                                _
                            ) | Ty::Record { .. }
                        ) =>
                    {
                        Some(Box::new(self.expr(r)?))
                    }
                    Some(_) => return Err("procedure calls on this receiver are not lowered yet".into()),
                    // A procedure of the record of a `with` statement runs on that record.
                    None if self.supplied_by_with(node) => Some(Box::new(ir::Expr::Load(self.implicit_rec(node)?))),
                    None if object == self.oi => {
                        self.declared_in_both_proc(object, offset)?;
                        self.bare_is_dirty_unpinned(node, object, offset)?;
                        self.report_item_same_name_refusal(node)?;
                        None
                    }
                    // A page or report extension calling a procedure of the page or report it extends.
                    None if self.base_object().is_some_and(|(b, _, _)| b == object) => {
                        self.base_call_allowed(object, routine)?;
                        Some(Box::new(ir::Expr::BaseInstance))
                    }
                    // A table extension calling its table's procedure: on the same Rec.
                    None if self.table_of[object].is_some() && self.rec_slot.is_some() => Some(Box::new(
                        ir::Expr::Load(ir::Place::Global(self.rec_slot.expect("checked"))),
                    )),
                    None => return Err("implicit calls into another object are not lowered yet".into()),
                };
                // The routine node is the one `routine_at` numbered, so no search by offset.
                let try_call = self
                    .routine_nodes
                    .get(object)
                    .get(routine)
                    .is_some_and(|r| r.attributes().any(|a| a.name().eq_ignore_ascii_case("tryfunction")));
                let receiver_kind = match &receiver_node {
                    Some(r) if instance.is_some() => match self.ty_of(r) {
                        Ty::Object(k, _) => Some(k),
                        Ty::Record { .. } => Some(ObjKind::Table),
                        _ => None,
                    },
                    _ => None,
                };
                let mut out = Vec::new();
                for (i, a) in args.iter().enumerate() {
                    let p = sig.params.get(i).ok_or("too many arguments")?;
                    if p.by_var {
                        let arg = self.by_var_arg(a)?;
                        // A `var` Text or Code parameter handed on to another `var` parameter keeps the declared length of the
                        // first caller's variable or field (text/lt-text-functions/MaxStrLen.Codeunit.al:146-170,
                        // PYC_MaxStrLen_VarParameterHops; to an event's subscribers too, :172-201): the routine takes the
                        // lengths from its caller to hand them on.
                        if matches!(p.ty, Ty::Text(_) | Ty::Code(_))
                            && let ir::Arg::Ref(ir::Place::Local(j)) = &arg
                            && self.var_params.get(*j).copied().unwrap_or(false)
                            && matches!(self.ty_of(a), Ty::Text(_) | Ty::Code(_))
                        {
                            self.reads_var_max_len = true;
                        }
                        out.push(arg);
                    } else {
                        if let Some(kind) = receiver_kind {
                            if let Some(fail) = self.char_into_number_of_other_object(kind, &p.ty, a)? {
                                out.push(ir::Arg::Value(fail));
                                continue;
                            }
                        }
                        let value = self.expr(a)?;
                        // A Variant handed to a by-value Text or Code parameter goes in as it is: the callee's binding converts it, and
                        // raises TypeConversion for a number, Boolean, Date, Enum or Option (types/lt-variant-to-text-fields/
                        // VariantNumberToTextParam.al; the text follows the session language, P147DateToText.al).
                        if matches!(p.ty, Ty::Text(_) | Ty::Code(_)) && matches!(self.ty_ref(a), Ty::Variant) {
                            out.push(ir::Arg::Value(value));
                            continue;
                        }
                        // A Notification handed to a by-value parameter is the caller's own (the binding in al-vm lib.rs shares it too): no copy.
                        let is_notification =
                            |t: &Ty| matches!(t, Ty::Builtin(n) if n.eq_ignore_ascii_case("notification"));
                        if is_notification(&p.ty) && is_notification(self.ty_ref(a)) {
                            out.push(ir::Arg::Value(value));
                            continue;
                        }
                        self.refuse_textconst_into_variant(&p.ty, a)?;
                        // A Text or Code handed to a by-value Text or Code parameter of a TryFunction is converted by the callee's
                        // binding, inside the try: a text too long for a Code[20] parameter makes the TryFunction answer false
                        // (text/lt-text-functions/TextAssign.Codeunit.al:240-281, LT_BU_TextAssign_LongTextToCode20Parameter). The other
                        // types keep the conversion at the call: whether theirs runs inside the try is not pinned.
                        if try_call
                            && matches!(p.ty, Ty::Text(_) | Ty::Code(_))
                            && matches!(self.ty_ref(a), Ty::Text(_) | Ty::Code(_))
                        {
                            out.push(ir::Arg::Value(value));
                            continue;
                        }
                        out.push(ir::Arg::Value(self.convert_to(&p.ty, self.ty_ref(a), value)?));
                    }
                }
                self.calls.push((object, routine));
                Ok(ir::Expr::Call {
                    object,
                    routine,
                    instance,
                    args: out,
                    try_call,
                })
            }
            // MaxStrLen reads the declared length of its argument's type, which values do not
            // carry (text/TestTextExtended.al MaxStrLen_*: 50 for Text[50], 2147483647 for Text).
            Some(Res::Builtin(id)) if id.starts_with("Text.MaxStrLen#") => {
                let arg = args.first().ok_or("MaxStrLen without an argument")?;
                // A `var` Text or Code parameter reports the declared length of the caller's variable or field, which the
                // call hands to the frame (text/lt-text-functions/MaxStrLen.Codeunit.al:91-151, LT_BU_MaxStrLen_TextParameters:
                // `var text with field text100 = 100`, `var text30 with field text100 = 100`, `var code30 with code20 = 20`).
                if arg.kind() == SyntaxKind::NameRef
                    && matches!(self.ty_of(arg), Ty::Text(_) | Ty::Code(_))
                    && let Ok(ir::Place::Local(i)) = self.place(arg)
                    && self.var_params.get(i).copied().unwrap_or(false)
                {
                    self.reads_var_max_len = true;
                    return Ok(ir::Expr::Builtin {
                        id: builtin_id("Text.MaxStrLenOfVarParameter#1")?,
                        receiver: None,
                        args: vec![ir::Arg::Ref(ir::Place::Local(i))],
                        ret_used: true,
                    });
                }
                match self.ty_of(arg) {
                    Ty::Text(Some(n)) | Ty::Code(Some(n)) => Ok(ir::Expr::Const(ir::Const::Integer(n as i32))),
                    Ty::Text(None) | Ty::Code(None) => Ok(ir::Expr::Const(ir::Const::Integer(i32::MAX))),
                    // A TextConst has the length 2048 (types/lt-variant/VaTTextConst.al:TextConst_MaxStrLen: Simple, Long, Empty and
                    // MultiLang). A Label answers 2048 too (text/lt-label-textbuilder/TxCLabel.al:Label_MaxLength).
                    Ty::Label => Ok(ir::Expr::Const(ir::Const::Integer(2048))),
                    // A Guid, null or not, answers 2147483647 (types/lt-aj-probes/AjMisc.al:68-69 and :82-83).
                    Ty::Guid => Ok(ir::Expr::Const(ir::Const::Integer(i32::MAX))),
                    other => Err(format!("MaxStrLen of a value of type {other}")),
                }
            }
            // ArrayLen reads the shape of its argument's type, which array values do not keep
            // (streams/TestArrayStreamContracts.al ArrayLen_*).
            Some(Res::Builtin(id)) if id.starts_with("System.ArrayLen#") => {
                let arg = args.first().ok_or("ArrayLen without an argument")?;
                let Ty::Array(dims, _) = self.ty_of(arg) else {
                    return Err("ArrayLen of something that is not an array variable".into());
                };
                // A one-dimensional array is measured when the call runs: a parameter holds the caller's array.
                if dims.len() == 1 && args.len() == 1 {
                    return Ok(ir::Expr::Builtin {
                        id: builtin_id(&id)?,
                        receiver: None,
                        args: vec![ir::Arg::Ref(self.place(arg)?)],
                        ret_used: true,
                    });
                }
                let dim = match args.get(1) {
                    Some(d) => match self.expr(d)? {
                        ir::Expr::Const(ir::Const::Integer(k)) => Some(k),
                        _ => return Err("ArrayLen with a dimension that is not a constant".into()),
                    },
                    None if dims.len() == 1 => Some(1),
                    // Without a dimension the total number of elements
                    // (streams/TestStreamsRuntimeContracts.al:ArrayLen_MultiDimensional_WithAndWithoutDimension).
                    None => {
                        let total: u32 = dims.iter().product();
                        return Ok(ir::Expr::Const(ir::Const::Integer(total as i32)));
                    }
                };
                match dim.and_then(|k| usize::try_from(k - 1).ok()).and_then(|k| dims.get(k)) {
                    Some(n) => Ok(ir::Expr::Const(ir::Const::Integer(*n as i32))),
                    // A dimension outside the array raises when the call runs (misc/lt-system-functions/Arrays.al:ArrayLen_Values
                    // `dim 3 of 2d`, `dim 0`); the call carries the dimension and the number of dimensions.
                    None => Ok(ir::Expr::Builtin {
                        id: builtin_id(&id)?,
                        receiver: None,
                        args: vec![
                            ir::Arg::Value(ir::Expr::Const(ir::Const::Integer(dim.unwrap_or(0)))),
                            ir::Arg::Value(ir::Expr::Const(ir::Const::Integer(dims.len() as i32))),
                        ],
                        ret_used: true,
                    }),
                }
            }
            // Refused when it runs, so code that never reaches it still runs: Base Application's Item Picture page exports in an
            // action's trigger, which a test that only opens an Item Card never invokes. What the call writes is not pinned (the
            // corpus skips it: streams/lt-stream-read-write-media/P102Media.al line 5).
            Some(Res::Builtin(id)) if matches!(id.as_str(), "Media.ExportFile#1" | "MediaSet.ExportFile#1") => {
                Ok(ir::Expr::Refused(format!(
                    "builtin {id}: what a media export writes to the server's file system is not pinned (docs/probes-needed.md)"
                )))
            }
            Some(Res::Builtin(id)) if id == "System.GetDotNetType#1" => self.get_dotnet_type_call(args),
            Some(Res::Builtin(id)) => {
                if al_builtins::native::lookup(&id).is_none() {
                    return Err(match al_builtins::native::refusal(&id) {
                        Some(why) => format!("builtin {id}: {why} (docs/probes-needed.md)"),
                        None => format!("builtin {id} is not implemented"),
                    });
                }
                let sig = self
                    .catalog
                    .by_id(&id)
                    .ok_or_else(|| format!("{id} not in catalog"))?
                    .clone();
                // Evaluate into an Option variable: a variable has no OptionCaption, so a text that is neither an ordinal nor a
                // member name is a plain rejection (P86Eval.al:Eval Option, `Bet`, `-1`, `1,0`). A field may name a member by its
                // caption, which the value does not carry, so a field keeps the id that refuses it.
                let decimal_field_places = if id == "System.Format#1" || id == "System.Format#2" {
                    args.first().and_then(|a| self.decimal_field_places(a))
                } else {
                    None
                };
                let auto_format_field = if id == "System.Format#1" && decimal_field_places.is_none() {
                    args.first().and_then(|a| self.auto_format_expression_field(a))
                } else {
                    None
                };
                let option_captions = if id == "System.Evaluate#1" {
                    if let Some(a) = args.first() {
                        self.refuse_evaluate_option_caption_ml(a)?;
                    }
                    args.first().and_then(|a| self.option_field_captions(a))
                } else {
                    None
                };
                let emit_id = if option_captions.is_some() {
                    "System.EvaluateOptionField#1".to_string()
                } else if id == "System.Evaluate#1"
                    && args.first().is_some_and(|a| {
                        matches!(self.ty_of(a), Ty::Option(_))
                            && matches!(self.res(a), Some(Res::Local(_) | Res::Global(_)))
                    })
                {
                    "System.EvaluateVariableOption#1".to_string()
                } else if id == "System.Evaluate#1"
                    && args
                        .first()
                        .is_some_and(|a| matches!(self.ty_of(a), Ty::Text(Some(_)) | Ty::Code(Some(_))))
                {
                    "System.EvaluateBounded#1".to_string()
                } else if id == "Dialog.Error#2" && args.first().is_some_and(|a| a.kind() != SyntaxKind::Literal) {
                    // `Error(Label, ...)` and `Error(Variable, ...)`: only a string literal has its text for
                    // `GetLastErrorText(true)` pinned (corpus P86LastError.al:PlainError_Exclude).
                    "Dialog.ErrorNotLiteral#1".to_string()
                } else if id == "System.Format#1"
                    && args.len() >= 3
                    && args.first().is_some_and(|a| matches!(self.ty_of(a), Ty::Variant))
                {
                    "System.FormatVariant#1".to_string()
                } else if id == "TextBuilder.Append#1"
                    && args
                        .first()
                        .is_some_and(|a| matches!(self.res(a), Some(Res::Builtin(i)) if i == "FieldRef.Value#1"))
                {
                    // `FieldRef.Value()` written directly as the argument appends the formatted value
                    // (text/lt-textbuilder-field-direct/TextBuilderAppendFieldDirect.al).
                    "TextBuilder.AppendFieldValue#1".to_string()
                } else if id == "TextBuilder.Append#1"
                    && args.first().is_some_and(|a| {
                        matches!(self.ty_of(a), Ty::Variant)
                            && matches!(self.res(a), Some(Res::Local(_) | Res::Global(_)))
                    })
                {
                    // A Variant variable holding a number raises (types/lt-number-to-text/NumberToTextIsFormat.al:193-216);
                    // a `FieldRef.Value()` argument goes to `TextBuilder.AppendFieldValue#1`.
                    "TextBuilder.AppendVariant#1".to_string()
                } else if id == "RecordRef.GetTable#1"
                    && args.first().is_some_and(|a| matches!(self.ty_of(a), Ty::Variant))
                {
                    // A record that reaches GetTable through a Variant loses its ChangeCompany (types/lt-cw-variant-leads/
                    // CwVariantChangeCompany.al:CW_Variant_KeepsOrDropsChangeCompany `variant to recordref`).
                    "RecordRef.GetTableVariant#1".to_string()
                } else if id == "TestField.SetValue#1"
                    && args.first().is_some_and(|a| matches!(self.ty_of(a), Ty::Variant))
                {
                    // A Text inside a Variant goes by ordinal digits on an Enum or Option control
                    // (pages/lt-be-enum-setvalue-blank/BeEnumSetValueBlank.al:532-597).
                    "TestField.SetValueVariant#1".to_string()
                } else if (id == "System.Format#1" || id == "System.Format#2") && decimal_field_places.is_some() {
                    "System.FormatDecimalField#1".to_string()
                } else if id == "System.Format#1" && auto_format_field.is_some() {
                    "System.FormatAutoFormatField#1".to_string()
                } else {
                    id.clone()
                };
                let receiver = match &receiver_node {
                    // `CurrReport` is the report whose code is running: the VM takes the running
                    // instance when a report builtin has no receiver.
                    Some(r)
                        if sig.instance
                            && sig.receiver == "Report"
                            && matches!(
                                self.ws.objects[self.oi].kind,
                                ObjKind::Report | ObjKind::ReportExtension
                            )
                            && r.text().to_string().trim().eq_ignore_ascii_case("CurrReport") =>
                    {
                        None
                    }
                    // `RequestOptionsPage` is the request page of the report whose code is running; it has
                    // no variable to take as a receiver.
                    Some(r)
                        if sig.instance
                            && sig.receiver == "RequestPage"
                            && matches!(
                                self.ws.objects[self.oi].kind,
                                ObjKind::Report | ObjKind::ReportExtension
                            )
                            && r.text().to_string().trim().eq_ignore_ascii_case("RequestOptionsPage") =>
                    {
                        None
                    }
                    // Enum metadata calls (`"Name".Names()`, `Var.Ordinals()`, `"Name".FromInteger(n)`)
                    // receive the enum itself: its object index, as an enum constant.
                    Some(r) if sig.receiver == "Enum" && sig.name != "AsInteger" => {
                        let Ty::Enum(name) = self.ty_of(r) else {
                            return Err(format!("{id}: the receiver is not an enum type"));
                        };
                        let object = self.resolve_obj(ObjKind::Enum, &name)?;
                        Some(Box::new(ir::Arg::Value(ir::Expr::Const(ir::Const::Enum(
                            object as u32,
                            0,
                        )))))
                    }
                    // A Label is its text: the variable holds none (reads lower to the constant).
                    Some(r) if sig.instance && sig.receiver == "Label" => Some(Box::new(ir::Arg::Value(self.expr(r)?))),
                    Some(r) if sig.instance => Some(Box::new(match self.place(r) {
                        Ok(p) => ir::Arg::Ref(p),
                        Err(_) => ir::Arg::Value(self.expr(r)?),
                    })),
                    // `this.Run()` in a codeunit runs the codeunit's own instance: its globals see the run, an error is
                    // returned when the result is used, and the record form hands the record to OnRun
                    // (xmlport/lt-aj-xmlport/AjRunAndXmlPort.al:6-140, the ThisRun_* tests).
                    None if sig.instance
                        && id == "Codeunit.Run#1"
                        && self.ws.objects[self.oi].kind == ObjKind::Codeunit
                        && callee.kind() == SyntaxKind::MemberExpr
                        && callee.children().next().is_some_and(|r| crate::check::is_this_keyword(&r)) =>
                    {
                        Some(Box::new(ir::Arg::Value(ir::Expr::ThisInstance)))
                    }
                    // An unqualified record method in table code runs on the implicit Rec.
                    None if sig.instance && matches!(sig.receiver.as_str(), "Record" | "Query") => {
                        Some(Box::new(ir::Arg::Ref(self.implicit_rec(node)?)))
                    }
                    _ => None,
                };
                let mut out = Vec::new();
                for (i, a) in args.iter().enumerate() {
                    let p = sig
                        .params
                        .get(i)
                        .or_else(|| sig.params.last().filter(|p| p.variadic))
                        .ok_or("too many arguments")?;
                    // `Clear(X)` of a declared DotNet variable goes by place like any other variable.
                    let clears_dotnet =
                        id == "System.Clear#1" && matches!(self.ty_ref(a), Ty::DotNet(al) if !al.is_empty());
                    if let (false, Ty::DotNet(alias)) = (clears_dotnet, self.ty_of(a)) {
                        // `IsNull(DotNet)` takes the value as it is. A `Variant` parameter takes a
                        // primitive, an object or an enum as it is (`Format` of an object is its
                        // `ToString()`). A scalar parameter takes a DotNet result (no static type)
                        // when its run-time kind fits the parameter, as in an assignment (ADR-024
                        // section 2). Anything else is refused.
                        // A collection method's element parameter (`Dictionary.Add(Key: TKey, Value: TValue)`,
                        // `List.Add(Value: T)`) is the element type of the receiver's declared collection.
                        let pty: String = match (p.ty.as_str(), receiver_node.as_ref().map(|r| self.ty_of(r))) {
                            ("TKey", Some(Ty::Dictionary(k, _))) => element_param_name(&k),
                            ("TValue", Some(Ty::Dictionary(_, v))) => element_param_name(&v),
                            ("T", Some(Ty::List(e))) => element_param_name(&e),
                            _ => p.ty.clone(),
                        };
                        let scalar = match pty.as_str() {
                            "Text" | "Code" => Some(RtType::Text(None)),
                            "Integer" => Some(RtType::Integer),
                            "BigInteger" => Some(RtType::BigInteger),
                            "Decimal" => Some(RtType::Decimal),
                            "Boolean" => Some(RtType::Boolean),
                            "Char" => Some(RtType::Char),
                            "Byte" => Some(RtType::Byte),
                            "Date" => Some(RtType::Date),
                            "Time" => Some(RtType::Time),
                            "DateTime" => Some(RtType::DateTime),
                            "Duration" => Some(RtType::Duration),
                            "Guid" => Some(RtType::Guid),
                            // A .NET String passed to SecretStrSubstNo is the secret's content; another kind is
                            // refused when it runs, as `fit` takes only a Text into a Text slot
                            // (tests/al-language-onprem-dotnet/TestDotNetValueKindsContracts.al:463).
                            "SecretText" if id.starts_with("SecretText.SecretStrSubstNo#") => Some(RtType::Text(None)),
                            _ => None,
                        };
                        let value = match pty.as_str() {
                            "DotNet" => self.expr(a)?,
                            // CopyStream takes a DotNet stream as it is; the VM copies the bytes
                            // through the host (ADR-024 section 3).
                            "InStream" | "OutStream" if !p.by_var && id.starts_with("System.CopyStream#") => {
                                self.expr(a)?
                            }
                            "Any" | "Variant" if !p.by_var => {
                                ir::Expr::DotNetOperand(Box::new(self.expr(a)?), ir::OperandUse::Variant)
                            }
                            _ => match scalar {
                                Some(rt) if !p.by_var && alias.is_empty() => ir::Expr::DotNetCoerce(
                                    rt,
                                    Box::new(ir::Expr::DotNetOperand(Box::new(self.expr(a)?), ir::OperandUse::Strict)),
                                ),
                                _ => {
                                    return Err(format!(
                                        "a DotNet value passed to builtin {id}: its type is only known at run time; assign it to a typed variable first"
                                    ));
                                }
                            },
                        };
                        out.push(ir::Arg::Value(value));
                        continue;
                    }
                    // A bare control name in a field position of a test page method.
                    if a.kind() == SyntaxKind::NameRef
                        && self.res(a).is_none()
                        && matches!(self.ty_ref(a), Ty::Builtin(b) if &**b == "TestField")
                        && receiver_node.as_ref().is_some_and(|r| {
                            matches!(
                                self.ty_of(r),
                                Ty::TestPage(_) | Ty::TestRequestPage(_) | Ty::TestPart(_)
                            )
                        })
                    {
                        let name = a
                            .children_with_tokens()
                            .filter_map(|e| e.into_token())
                            .find(|t| matches!(t.kind(), SyntaxKind::Ident | SyntaxKind::QuotedIdent))
                            .map(|t| ast::ident_text(&t))
                            .ok_or("control name")?;
                        let recv = receiver_node.clone().expect("checked");
                        out.push(ir::Arg::Value(self.control_named(&recv, &name)?));
                        continue;
                    }
                    // `Source.CopyFilter(FromField, Target."ToField")`: the target is a field of another
                    // record variable, passed as that record and the field's position.
                    if id == "Record.CopyFilter#1" && i == 1 {
                        let Some(Res::Field { table, id: fid }) = self.res(a).cloned() else {
                            return Err("CopyFilter to something that is not a field of a record".into());
                        };
                        self.readable_field_pos(table, fid)?;
                        let field = self.field_ref(table, fid);
                        let base = match a.kind() {
                            SyntaxKind::MemberExpr => self.place(&a.children().next().ok_or("member base")?)?,
                            _ => self.implicit_rec(a)?,
                        };
                        out.push(ir::Arg::Ref(base));
                        out.push(ir::Arg::Value(ir::Expr::Const(ir::Const::FieldPos(field as u32))));
                        continue;
                    }
                    // A parameter named Field... that is an Integer takes the number as a VALUE (`RecRef.Field(Buf."Field No.")`,
                    // `RecRef.FieldExist(R.Number)`); a field reference is passed for a parameter that names a field.
                    let field_param = (p.name.to_ascii_lowercase().starts_with("field") && p.ty != "Integer")
                        || p.name.eq_ignore_ascii_case("FromField")
                        || p.name.to_ascii_lowercase().starts_with("column");
                    match (field_param, self.res(a).cloned()) {
                        // `Page.RunModal(Id, Rec, Rec."Field")` names the field by its NUMBER (the runtime checks the table has it, errors/
                        // P106RunErrors.al: a literal 999 is "supplied field number '999' cannot be found"); a field position would be wrong
                        // for a table whose numbers have gaps (Segment Line."Contact Via" is field 34, position 30).
                        (true, Some(Res::Field { table, id: fid })) if id.starts_with("Page.Run") => {
                            self.readable_field_pos(table, fid)?;
                            out.push(ir::Arg::Value(ir::Expr::Const(ir::Const::Integer(fid as i32))));
                            continue;
                        }
                        (true, Some(Res::Field { table, id })) => {
                            self.readable_field_pos(table, id)?;
                            let field = self.field_ref(table, id);
                            out.push(ir::Arg::Value(ir::Expr::Const(ir::Const::FieldPos(field as u32))));
                            continue;
                        }
                        // A query column is named by its position in the query's column list.
                        // The query is the table; the column's position in its list is the field number.
                        (true, Some(Res::QueryColumn { query, column })) => {
                            let field = self.field_ref(query, column as u32);
                            out.push(ir::Arg::Value(ir::Expr::Const(ir::Const::FieldPos(field as u32))));
                            continue;
                        }
                        _ => {}
                    }
                    // `Page.FindFirstField(Name, v)` names the control bare. The corpus only
                    // qualifies it (`ListPage."Integer Field"`, handlers/TestBCReportHandlerContracts.al:247);
                    // what a bare name means (the receiver's controls first?) is not pinned.
                    if p.ty == "TestField"
                        && a.kind() != SyntaxKind::MemberExpr
                        && !matches!(self.res(a), Some(Res::Local(_) | Res::Global(_)))
                    {
                        return Err(format!(
                            "the TestField argument `{}` is a bare control name: only the qualified form is pinned by a corpus test",
                            a.text()
                        ));
                    }
                    // Generic parameters convert to the collection's element types.
                    let elem = match (p.ty.as_str(), receiver_node.as_ref().map(|r| self.ty_of(r))) {
                        ("T", Some(Ty::List(el))) => Some(*el),
                        ("TKey", Some(Ty::Dictionary(k, _))) => Some(*k),
                        ("TValue", Some(Ty::Dictionary(_, v))) => Some(*v),
                        _ => None,
                    };
                    // CopyArray writes into its destination in place, so both arrays are passed as places: the destination can be a
                    // row of a two-dimensional array (a copy that is written back), and the source's dimensions are checked at run time.
                    out.push(if p.by_var || (id == "System.CopyArray#1" && i < 2) {
                        ir::Arg::Ref(self.place(a)?)
                    } else if let Some(el) = elem {
                        ir::Arg::Value(ir::Expr::Convert(self.rt(&el)?, Box::new(self.expr(a)?)))
                    // A Variant argument with a standard format number 3 to 8 raises an error that names the type
                    // 'Variant' (corpus types/lt-scalars/ScT70052.al): `System.FormatVariant#1` checks the number when it runs,
                    // a literal, a variable or an expression alike.
                    } else if id.starts_with("System.Format#") && i == 0 {
                        ir::Arg::Value(self.format_option_field(
                            args.len() == 1,
                            args.len() == 3
                                && args[1].text().to_string().trim() == "0"
                                && args[2].text().to_string().trim() == "9",
                            a,
                        )?)
                    // The substituted values of StrSubstNo, Message and Error are `Format`ted: an Option
                    // field shows its caption (corpus xmlport/PWXmlPortFormatEvaluate.al:Import_Xml_Option,
                    // `%3` of a field with OptionCaption).
                    } else if i > 0
                        && (id.starts_with("Text.StrSubstNo#")
                            || id.starts_with("Dialog.Message#")
                            || id.starts_with("Dialog.Error#"))
                    {
                        ir::Arg::Value(self.format_option_field(true, false, a)?)
                    } else {
                        ir::Arg::Value(self.expr(a)?)
                    });
                }
                // `--tdd` (ADR-051 decision 10): `Validate(StubField, Value)` checks the stub length as an assignment does.
                if id.starts_with("Record.Validate#") && args.len() == 2 && out.len() == 2 {
                    let ty = self.ty_of(&args[0]);
                    if let ir::Arg::Value(v) = out.pop().expect("two arguments") {
                        let v = self.tdd_fit(&args[0], &ty, v);
                        out.push(ir::Arg::Value(v));
                    }
                }
                // The OptionCaption of an Option field goes along as a fourth argument.
                if let (true, Some(captions)) = (emit_id == "System.EvaluateOptionField#1", option_captions) {
                    while out.len() < 3 {
                        out.push(ir::Arg::Value(ir::Expr::Const(ir::Const::Integer(0))));
                    }
                    out.push(ir::Arg::Value(ir::Expr::Const(ir::Const::Text(captions))));
                }
                // The declared length of a Text or Code variable goes along as a fourth argument.
                if emit_id == "System.EvaluateBounded#1" {
                    if let Some(Ty::Text(Some(n)) | Ty::Code(Some(n))) = args.first().map(|a| self.ty_of(a)) {
                        while out.len() < 3 {
                            out.push(ir::Arg::Value(ir::Expr::Const(ir::Const::Integer(0))));
                        }
                        out.push(ir::Arg::Value(ir::Expr::Const(ir::Const::Integer(n as i32))));
                    }
                }
                // The record and the field's position go along: the places come from the field's AutoFormatExpression when it runs.
                if let (true, Some((record, pos))) = (emit_id == "System.FormatAutoFormatField#1", auto_format_field) {
                    while out.len() < 3 {
                        out.push(ir::Arg::Value(ir::Expr::Const(ir::Const::Integer(0))));
                    }
                    out.push(ir::Arg::Value(ir::Expr::Load(record)));
                    out.push(ir::Arg::Value(ir::Expr::Const(ir::Const::Integer(pos as i32))));
                }
                // The places, and whether they hold only from BC 28 (an amount field without AutoFormatExpression).
                if let (true, Some((places, gated))) = (emit_id == "System.FormatDecimalField#1", decimal_field_places) {
                    while out.len() < 3 {
                        out.push(ir::Arg::Value(ir::Expr::Const(ir::Const::Integer(0))));
                    }
                    out.push(ir::Arg::Value(ir::Expr::Const(ir::Const::Text(places))));
                    out.push(ir::Arg::Value(ir::Expr::Const(ir::Const::Boolean(gated))));
                }
                Ok(ir::Expr::Builtin {
                    id: builtin_id(&emit_id)?,
                    receiver,
                    args: out,
                    ret_used: true,
                })
            }
            // The checker refused a builtin of a newer runtime (ADR-015 3.3): say why.
            None if self.runtime_refusal(node).is_some() => Err(self.runtime_refusal(node).unwrap_or_default()),
            // `Window.Open(..)` in a report extension, where `Window` is a variable of the base report.
            None if self.base_member(callee).is_some() => Err(self.base_member(callee).unwrap_or_default()),
            // `CurrPage.<AddIn>.Method()`: refused when it runs, so code that never reaches it still runs
            // (Base Application's "Item Tracking Lines" calls its barcode add-in only after the client said it
            // is supported). Control add-ins are not modelled; no corpus test calls one.
            None if callee.kind() == SyntaxKind::MemberExpr
                && callee.children().next().and_then(|b| self.control_add_in(&b)).is_some() =>
            {
                let why = callee
                    .children()
                    .next()
                    .and_then(|b| self.control_add_in(&b))
                    .unwrap_or_default();
                Ok(ir::Expr::Refused(format!("call `{}`: {why}", node.text())))
            }
            // `--tdd`: `Target.Missing().Insert()`, a stub site used as a receiver, says why it is refused.
            None if receiver_node.as_ref().is_some_and(|r| {
                matches!(self.res(r), Some(Res::TddStub(s)) if matches!(s.ret, crate::check::tdd::TddRet::Refused(_)))
            }) =>
            {
                match receiver_node.as_ref().and_then(|r| self.res(r)) {
                    Some(Res::TddStub(site)) => match &site.ret {
                        crate::check::tdd::TddRet::Refused(why) => Err(why.clone()),
                        _ => unreachable!("checked"),
                    },
                    _ => unreachable!("checked"),
                }
            }
            None => Err(format!("call `{}` has no callee the checker resolved", node.text())),
            Some(r) => Err(format!(
                "call `{}` to {} is not lowered yet",
                node.text(),
                match r {
                    Res::Local(_) | Res::Global(_) => "a variable",
                    Res::Field { .. } | Res::QueryColumn { .. } => "a field",
                    Res::EnumValue { .. } | Res::OptionValue(_) => "an enum or option value",
                    Res::ObjectId(_) => "an object id",
                    _ => "this kind of callee",
                }
            )),
        }
    }

    /// A Char argument for a by-value number parameter of a procedure called on a variable of another object (`receiver`
    /// is its kind). A Char variable handed to an Integer parameter of a report procedure fails with 'Unable to convert from
    /// System.Char to System.Int32.' (code Conversion), with and without a report extension that adds an overload, measured on
    /// Windows too (report/lt-bv-report-char-arg/BvReportCharArg.al:87-131); a local procedure converts it (:160-170, the
    /// runtime's usual conversion). The same call on another codeunit was measured on the Linux tiers only (:140-158), so it
    /// is refused until the Windows check; other receivers, other number types and other Char expressions are not pinned and
    /// are refused. `None`: not such an argument.
    fn char_into_number_of_other_object(&self, receiver: ObjKind, param: &Ty, arg: &SyntaxNode) -> L<Option<ir::Expr>> {
        if !matches!(self.ty_ref(arg), Ty::Char)
            || !matches!(
                param,
                Ty::Integer | Ty::BigInteger | Ty::Decimal | Ty::Byte | Ty::Duration
            )
        {
            return Ok(None);
        }
        let variable =
            arg.kind() == SyntaxKind::NameRef && matches!(self.res(arg), Some(Res::Local(_) | Res::Global(_)));
        match receiver {
            ObjKind::Report if variable && matches!(param, Ty::Integer) => Ok(Some(ir::Expr::Fail {
                message: "Unable to convert from System.Char to System.Int32.".into(),
                code: "Conversion".into(),
            })),
            ObjKind::Codeunit if variable && matches!(param, Ty::Integer) => Err(
                "a Char variable into an Integer parameter of another codeunit (measured on Linux only, Windows check queued)".into(),
            ),
            _ => Err(format!(
                "a Char into a {param} parameter of a {receiver} procedure called on a variable: the conversion is not pinned"
            )),
        }
    }

    /// A bare call of a codeunit's own IsDirty in a codeunit with TableNo, outside its OnRun: whether the call is the
    /// codeunit's procedure or Record.IsDirty of the implicit Rec is not pinned (only OnRun is:
    /// events/lt-with-statement-scopes/WithStatementAmbiguous.al:277-279, see `Checker::lookup_callable`).
    fn bare_is_dirty_unpinned(&self, node: &SyntaxNode, object: usize, offset: usize) -> L<()> {
        if self.ws.objects[object].kind != ObjKind::Codeunit || self.rec_slot.is_none() {
            return Ok(());
        }
        let named_is_dirty = self.members[object]
            .procedures
            .iter()
            .any(|p| p.offset == offset && p.name.eq_ignore_ascii_case("IsDirty"));
        if !named_is_dirty {
            return Ok(());
        }
        let in_on_run = node
            .ancestors()
            .find_map(ast::Routine::cast)
            .is_some_and(|r| r.is_trigger() && r.name().eq_ignore_ascii_case("OnRun"));
        if in_on_run {
            return Ok(());
        }
        Err("a bare IsDirty() outside OnRun of a codeunit with TableNo: its own procedure or Record.IsDirty is not pinned".into())
    }

    /// A call in a report data item to a name the report and the item's table both declare, in a shape no corpus test
    /// pins (the checker's AR1006, see `Checker::lookup_callable`; report/lt-cj-report-leads/CjDataset.al:215-241 pins
    /// only one procedure on each side with the same parameters).
    fn report_item_same_name_refusal(&self, node: &SyntaxNode) -> L<()> {
        let path = &self.ws.files[self.file].path;
        let at: usize = node.text_range().start().into();
        match self
            .checked
            .diagnostics
            .iter()
            .find(|d| d.code == "AR1006" && d.offset == at && &d.file == path)
        {
            Some(d) => Err(d.message.clone()),
            None => Ok(()),
        }
    }

    /// The checker's runtime refusal of this call, if any: AL0666 (a builtin newer than the app's runtime) or AR1007
    /// (an app below the runtime floor calls a builtin AL Runner cannot clear for it; al-runner issue 5573).
    pub(crate) fn runtime_refusal(&self, node: &SyntaxNode) -> Option<String> {
        let path = &self.ws.files[self.file].path;
        let at: usize = node.text_range().start().into();
        self.checked
            .diagnostics
            .iter()
            .find(|d| matches!(d.code, "AL0666" | "AR1007") && d.offset == at && &d.file == path)
            .map(|d| format!("{} {}", d.code, d.message))
    }
}

/// The [`ir::BuiltinId`] of a catalog overload id. Every catalog id has one (a test in al-builtins checks it), so
/// this fails only when `builtin_ids.rs` is stale.
pub(super) fn builtin_id(id: &str) -> Result<ir::BuiltinId, String> {
    ir::BuiltinId::of(id).ok_or_else(|| format!("builtin {id} has no builtin id (regenerate builtin_ids.rs)"))
}

/// The parameter type name a collection element of type `t` stands for, in the spelling of the
/// builtin catalog (empty when no DotNet value may be passed for it).
fn element_param_name(t: &Ty) -> String {
    match t {
        Ty::Integer => "Integer",
        Ty::BigInteger => "BigInteger",
        Ty::Decimal => "Decimal",
        Ty::Boolean => "Boolean",
        Ty::Char => "Char",
        Ty::Byte => "Byte",
        Ty::Text(_) => "Text",
        Ty::Code(_) => "Code",
        Ty::Date => "Date",
        Ty::Time => "Time",
        Ty::DateTime => "DateTime",
        Ty::Duration => "Duration",
        Ty::Guid => "Guid",
        Ty::Variant => "Variant",
        Ty::DotNet(_) => "DotNet",
        _ => "",
    }
    .to_string()
}
