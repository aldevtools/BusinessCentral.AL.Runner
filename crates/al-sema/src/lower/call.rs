//! Lowering of calls.
use super::*;
use al_syntax::ast::NodeText;

impl Lowerer<'_> {
    /// `Rec.SecurityFiltering()` read on a variable declared with `[SecurityFiltering(...)]` answers the declared value on
    /// BC (al-language/reports/runtime/ACAttributes.al:176-191); the runtime keeps it for a local record, a local
    /// one-dimension array of records and a global record of a codeunit, a table, a page or a report
    /// (`shared::declared_security_filtering`), so the read is refused on anything else (a global of another kind of object,
    /// a global array, an array of more dimensions: not pinned). An element `X[i]` is judged by its array `X`.
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
        let mut recv = recv;
        while recv.kind() == SyntaxKind::IndexExpr {
            let Some(base) = recv.children().next() else {
                return Ok(());
            };
            recv = base;
        }
        let attrs = match self.res(&recv) {
            Some(Res::Local(n)) => self.local_var_attrs.get(n),
            Some(Res::Global(n)) => self.global_var_attrs.get(n),
            _ => None,
        };
        if attrs.is_some_and(|a| a.iter().any(|a| a == "securityfiltering")) {
            return Err(format!(
                "SecurityFiltering() of `{}`, declared with the attribute SecurityFiltering: the declared value is not kept",
                recv.text_string().trim()
            ));
        }
        Ok(())
    }

    /// `n` names a whole array declared with `[SecurityFiltering(...)]` in an array assignment (`what`) or a CopyArray. The
    /// probe al-language/reports/runtime/SFSecurityFilteringFlow.al:429-459 reads the elements of such an array only directly
    /// (and after Clear of an element and of the array); what an element reads after the array was assigned or copied, in
    /// either direction, is not measured, so the routine is refused.
    pub(crate) fn security_filtering_array_refusal(&self, n: &SyntaxNode, what: &str) -> L<()> {
        let n = match n.kind() {
            SyntaxKind::ParenExpr => match n.children().next() {
                Some(inner) => inner,
                None => return Ok(()),
            },
            _ => n.clone(),
        };
        if n.kind() != SyntaxKind::NameRef || !matches!(self.ty_ref(&n), crate::types::Ty::Array(..)) {
            return Ok(());
        }
        let attrs = match self.res(&n) {
            Some(Res::Local(name)) => self.local_var_attrs.get(name),
            Some(Res::Global(name)) => self.global_var_attrs.get(name),
            _ => None,
        };
        if attrs.is_some_and(|a| {
            a.iter()
                .any(|a| a == "securityfiltering" || a == SECURITY_FILTERING_KEPT)
        }) {
            return Err(format!(
                "{what} of the array `{}`, declared with the attribute SecurityFiltering: what its elements read is not pinned",
                n.text_string().trim()
            ));
        }
        Ok(())
    }

    /// Whether `field` (the receiver of a Media method, `X.Layout`) is a field of a Report Layout List record.
    fn report_layout_list_field(&self, field: &SyntaxNode) -> bool {
        if field.kind() != SyntaxKind::MemberExpr {
            return false;
        }
        let Some(rec) = field.children().next() else {
            return false;
        };
        matches!(self.ty_ref(&rec), crate::types::Ty::Record { table, .. }
            if crate::types::display_name(table).eq_ignore_ascii_case("Report Layout List"))
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

    /// The receiver of a bare report method in a report's procedure (see `call`): the report's own instance for `RunModal`
    /// and `Run`, else none (the VM takes the report whose code runs). In a trigger only the bare `UseRequestPage` read
    /// (with or without parentheses) of a report is pinned, in OnInitReport, OnPreReport, OnPostReport, OnPreDataItem,
    /// OnAfterGetRecord, OnPostDataItem and the request page's OnOpenPage: it reads what the `CurrReport.` form reads there
    /// (reports/bare-report-member/UseRequestPageReadInTriggers.al:46-117, the `Three(..)` logs of :244-475; corpus
    /// reports/bare-report-member/Y36BareReportMember.al:220-282, the bare form in a procedure OnPreReport calls;
    /// al-runner issues 7413, 7414). `getter`: the call has no argument.
    fn bare_report_receiver(&self, node: &SyntaxNode, name: &str, getter: bool) -> L<Option<Box<ir::Arg>>> {
        let routine = node
            .ancestors()
            .find(|a| matches!(a.kind(), SyntaxKind::Procedure | SyntaxKind::Trigger))
            .and_then(al_syntax::ast::Routine::cast);
        let is_report = self.ws.objects[self.oi].kind == ObjKind::Report;
        if let Some(trigger) = routine.as_ref().filter(|r| r.is_trigger()) {
            // The trigger kind comes from where it is declared: the report itself, a `dataitem` or the `requestpage`.
            let section = trigger
                .syntax()
                .ancestors()
                .find_map(al_syntax::ast::Section::cast)
                .map(|sec| sec.name().to_ascii_lowercase());
            let names: &[&str] = match section.as_deref() {
                None => &["OnInitReport", "OnPreReport", "OnPostReport"],
                Some("dataitem") => &["OnPreDataItem", "OnAfterGetRecord", "OnPostDataItem"],
                Some("requestpage") => &["OnOpenPage"],
                Some(_) => &[],
            };
            let pinned = is_report
                && getter
                && name.eq_ignore_ascii_case("UseRequestPage")
                && names.iter().any(|t| trigger.name().eq_ignore_ascii_case(t));
            if !pinned {
                return Err(format!(
                    "a bare `{name}` in a trigger of the report runs while the report runs (not pinned; only the read `UseRequestPage` of a report in its run and data item triggers and its request page's OnOpenPage is)"
                ));
            }
            return Ok(None);
        }
        if routine.is_none() {
            return Err(format!(
                "a bare `{name}` in a trigger of the report runs while the report runs (not pinned)"
            ));
        }
        if !(name.eq_ignore_ascii_case("RunModal") || name.eq_ignore_ascii_case("Run")) {
            return Ok(None);
        }
        if !is_report {
            return Err(format!("a bare `{name}` in a report extension (not pinned)"));
        }
        Ok(Some(Box::new(ir::Arg::Value(ir::Expr::ThisInstance))))
    }

    /// A call resolved on `node` (the CallExpr, or the callee for a call without parentheses).
    /// The id of the implementation for a call whose argument is written as a Variant, where BC treats it apart.
    fn variant_argument_id(&self, id: &str, args: &[SyntaxNode]) -> Option<&'static str> {
        let variant = |i: usize| args.get(i).is_some_and(|a| matches!(self.ty_of(a), Ty::Variant));
        match id {
            "System.Format#1" if args.len() >= 3 && variant(0) => Some("System.FormatVariant#1"),
            // A record that reaches GetTable through a Variant loses its ChangeCompany (types/lt-cw-variant-leads/
            // CwVariantChangeCompany.al:CW_Variant_KeepsOrDropsChangeCompany `variant to recordref`).
            "RecordRef.GetTable#1" if variant(0) => Some("RecordRef.GetTableVariant#1"),
            // A Variant converts into the field as through `FieldRef.Validate` (corpus
            // record/validate/RecordValidateVariant.al; al-runner issue 6975).
            "Record.Validate#1" if args.len() == 2 && variant(1) => Some("Record.ValidateVariant#1"),
            // A Text inside a Variant goes by ordinal digits on an Enum or Option control
            // (pages/enum-setvalue-blank/BeEnumSetValueBlank.al:532-597).
            "TestField.SetValue#1" if variant(0) => Some("TestField.SetValueVariant#1"),
            _ => None,
        }
    }

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
        if callee.kind() == SyntaxKind::NameRef && callee.text_string().trim().eq_ignore_ascii_case("CopyArray") {
            for a in args.iter().take(2) {
                self.security_filtering_array_refusal(a, "CopyArray")?;
            }
        }
        // A control add-in as an argument (`BusinessChartBuffer.UpdateChart(CurrPage.BusinessChart)`, by `var` or by value)
        // refuses when the call runs, as a call of its methods does below: Base Application's chart pages make it on a path a
        // flag guards. Control add-ins are not modelled; no corpus test passes one.
        if let Some(why) = args.iter().find_map(|a| self.control_add_in(a)) {
            return Ok(ir::Expr::Refused(format!("call `{}`: {why}", node.text())));
        }
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
                self.proc_call_refusal(node, object, offset)?;
                let routine = self.routine_at.get(&(object, offset)).ok_or("procedure not found")?;
                let sig = self.request_proc_refusal(object, offset)?;
                let instance = match &receiver_node {
                    // `this.Proc()` runs on the current instance, like a bare call.
                    Some(r)
                        if r.kind() == SyntaxKind::NameRef
                            && r.text_string().trim().eq_ignore_ascii_case("this") =>
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
                    None if self.table_of[object].is_some() && self.rec_slot.is_some() => {
                        self.report_item_same_name_refusal(node)?;
                        Some(Box::new(ir::Expr::Load(ir::Place::Global(self.rec_slot.expect("checked")))))
                    }
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
                    if let Some(arg) = self.run_on_client_arg(p.by_var, &p.ty, a) {
                        out.push(arg);
                        continue;
                    }
                    if p.by_var {
                        let arg = self.by_var_arg(a)?;
                        // A `var` Text or Code parameter handed on to another `var` parameter keeps the declared length of the
                        // first caller's variable or field (text/text-methods/MaxStrLen.Codeunit.al:146-170,
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
                        let value = self.expr_into(&p.ty, a)?;
                        // A Variant handed to a by-value Text or Code parameter goes in as it is: the callee's binding converts it, and
                        // raises TypeConversion for a number, Boolean, Date, Enum or Option (types/variant/
                        // VariantNumberToTextParam.al; the text follows the session language, P147DateToText.al).
                        // The cells the matrix pins for a Variant apply first (`Variant.Into#1`: a Byte is its character).
                        if matches!(p.ty, Ty::Text(_) | Ty::Code(_)) && matches!(self.ty_ref(a), Ty::Variant) {
                            out.push(ir::Arg::Value(self.variant_into(&p.ty, value)?));
                            continue;
                        }
                        // A Variant handed to a by-value Record parameter goes in as it is too: the binding gives a RecordRef of
                        // another table the ObjectDefinitionChanged error, where an assignment raises NotSupportedOperation
                        // (streams/ArRecordProbes.al:410-436 SG_VariantRecordRef_OtherTable_AssignAndParameter `param`;
                        // al-vm `record_ref_into_record_param`). A Record is a Record of any table there as before. For a
                        // TryFunction callee the conversion stays at the call: whether it runs inside the try is not pinned.
                        if !try_call && matches!(p.ty, Ty::Record { .. }) && matches!(self.ty_ref(a), Ty::Variant) {
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
                        self.refuse_variant_argument(&p.ty, a)?;
                        // A Text or Code handed to a by-value Text or Code parameter of a TryFunction is converted by the callee's
                        // binding, inside the try: a text too long for a Code[20] parameter makes the TryFunction answer false
                        // (text/text-methods/TextAssign.Codeunit.al:240-281, LT_BU_TextAssign_LongTextToCode20Parameter). A Decimal or
                        // a BigInteger handed to an Integer parameter too: 2.5 and 3000000000 make it answer false with the conversion
                        // error (TextAssign.Codeunit.al:292-358 PYC_TryFunction_NumberArgumentConversion; types/operators-expressions/
                        // P79Enums.al:53-118 FromInteger_WithBigIntegerArgument). The other types keep the conversion at the call:
                        // whether theirs runs inside the try is not pinned.
                        if try_call
                            && ((matches!(p.ty, Ty::Text(_) | Ty::Code(_))
                                && matches!(self.ty_ref(a), Ty::Text(_) | Ty::Code(_)))
                                || (matches!(p.ty, Ty::Integer) && matches!(self.ty_ref(a), Ty::Decimal | Ty::BigInteger)))
                        {
                            out.push(ir::Arg::Value(value));
                            continue;
                        }
                        out.push(ir::Arg::Value(self.convert_arg(&p.ty, self.ty_ref(a), value)?));
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
                // call hands to the frame (text/text-methods/MaxStrLen.Codeunit.al:91-151, LT_BU_MaxStrLen_TextParameters:
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
                    // A TextConst has the length 2048 (types/variant/VaTTextConst.al:TextConst_MaxStrLen: Simple, Long, Empty and
                    // MultiLang). A Label answers 2048 too (text/textbuilder-bigtext/TxCLabel.al:Label_MaxLength).
                    Ty::Label => Ok(ir::Expr::Const(ir::Const::Integer(2048))),
                    // A Guid, null or not, answers 2147483647 (types/AjMisc.al:68-69 and :82-83).
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
                // A one-dimensional array is measured when the call runs: a parameter holds the caller's array. The array a
                // procedure returns has the length its declaration gives (al-runner issue 6658).
                if dims.len() == 1 && args.len() == 1 && arg.kind() != SyntaxKind::CallExpr {
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
                    // A dimension outside the array raises when the call runs (collections/Arrays.al:ArrayLen_Values
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
            // corpus skips it: streams/media/P102Media.al line 5).
            // `MediaSet.ExportFile` runs in the company build only (al-builtins media.rs `set_export_file`).
            Some(Res::Builtin(id)) if id == "Media.ExportFile#1" => {
                Ok(ir::Expr::Refused(format!(
                    "builtin {id}: what a media export writes to the server's file system is not pinned (docs/probes-needed.md)"
                )))
            }
            // The Layout of a Report Layout List row: its HasValue is No for a built-in layout (reports/PZReportLayoutMedia.al:51-53),
            // and its export writes the layout file of the app's package (reports/layouts-saveas/Y17LayoutExportTests.al:70,
            // corpus master). The VM looks the layout up by the row's report id and name (`Report.LayoutListExportStream#1`, the row
            // as its receiver); an empty Media field of another table exports nothing (al-builtins media.rs `media_export`).
            Some(Res::Builtin(id))
                if id == "Media.ExportStream#1"
                    && receiver_node
                        .as_ref()
                        .is_some_and(|r| self.report_layout_list_field(r)) =>
            {
                let row = receiver_node
                    .as_ref()
                    .and_then(|r| r.children().next())
                    .ok_or("the Layout field without its record")?;
                let [stream] = args else {
                    return Err("Media.ExportStream of a Report Layout List Layout without one OutStream".into());
                };
                Ok(ir::Expr::Builtin {
                    id: builtin_id("Report.LayoutListExportStream#1")?,
                    receiver: Some(Box::new(ir::Arg::Value(self.expr(&row)?))),
                    args: vec![ir::Arg::Value(self.expr(stream)?)],
                    ret_used: true,
                })
            }
            Some(Res::Builtin(id)) if id == "System.GetDotNetType#1" => self.get_dotnet_type_call(args),
            Some(Res::Builtin(id)) => {
                if al_builtins::native::lookup(&id).is_none() {
                    return Err(format!("builtin {id} is not implemented"));
                }
                let sig = self
                    .catalog
                    .by_id(&id)
                    .ok_or_else(|| format!("{id} not in catalog"))?
                    .clone();
                // Evaluate into an Option variable: a variable has no OptionCaption, so a text that is neither an ordinal nor a
                // member name is a plain rejection (P86Eval.al:Eval Option, `Bet`, `-1`, `1,0`). A field may name a member by its
                // caption, which the value does not carry, so a field keeps the id that refuses it.
                if id == "System.Format#1" || id == "System.Format#2" {
                    if let Some(why) = args.first().and_then(|a| self.decimal_field_format_unpinned(a)) {
                        return Ok(ir::Expr::Refused(why));
                    }
                }
                // `Format(Rec.Field)` of an AutoFormatType 2, 4 or 5 field without an expression takes the setup's places when it runs
                // (`auto_format_field_without_expression`); with a length or format, `decimal_field_places` decides as before.
                let without_expression = if id == "System.Format#1" && args.len() == 1 {
                    args.first().and_then(|a| self.auto_format_field_without_expression(a))
                } else {
                    None
                };
                let decimal_field_places =
                    if (id == "System.Format#1" || id == "System.Format#2") && without_expression.is_none() {
                        args.first().and_then(|a| self.decimal_field_places(a))
                    } else {
                        None
                    };
                let auto_format_field = if id == "System.Format#1" && decimal_field_places.is_none() {
                    match &without_expression {
                        Some((record, pos, kind)) => Some((record.clone(), *pos, *kind)),
                        None => args
                            .first()
                            .and_then(|a| self.auto_format_expression_field(a)),
                    }
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
                        // An element of an Option array variable reads as the variable does: an unknown name answers false
                        // and keeps the element (types/arrays/PnC14OptionArray.al:51-78,
                        // OptionArray_Evaluate_And_Out_Of_Range).
                        let var = if a.kind() == SyntaxKind::IndexExpr {
                            a.children().next()
                        } else {
                            Some(a.clone())
                        };
                        matches!(self.ty_of(a), Ty::Option(_))
                            && var.is_some_and(|v| matches!(self.res(&v), Some(Res::Local(_) | Res::Global(_))))
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
                    self.error_variant_id(args)
                } else if matches!(id.as_str(), "Record.FieldError#1" | "Record.FieldError#2") && args.len() == 2 {
                    self.field_error_variant_id(&id, args)
                } else if let Some(variant_id) = self.variant_argument_id(&id, args) {
                    variant_id.to_string()
                } else if id == "TextBuilder.Append#1"
                    && args
                        .first()
                        .is_some_and(|a| matches!(self.res(a), Some(Res::Builtin(i)) if i == "FieldRef.Value#1"))
                {
                    // `FieldRef.Value()` written directly as the argument appends the formatted value
                    // (text/textbuilder-bigtext/TextBuilderAppendFieldDirect.al).
                    "TextBuilder.AppendFieldValue#1".to_string()
                } else if id == "TextBuilder.Append#1"
                    && args.first().is_some_and(|a| {
                        matches!(self.ty_of(a), Ty::Variant)
                            && matches!(self.res(a), Some(Res::Local(_) | Res::Global(_)))
                    })
                {
                    // A Variant variable holding a number raises (format-evaluate/NumberToTextIsFormat.al:193-216);
                    // a `FieldRef.Value()` argument goes to `TextBuilder.AppendFieldValue#1`.
                    "TextBuilder.AppendVariant#1".to_string()
                } else if (id == "System.Format#1" || id == "System.Format#2") && decimal_field_places.is_some() {
                    "System.FormatDecimalField#1".to_string()
                } else if id == "System.Format#1" && auto_format_field.is_some() {
                    "System.FormatAutoFormatField#1".to_string()
                } else if id == "System.Format#1" && args.first().is_some_and(|a| self.field_table_class_column(a)) {
                    "System.FormatOptionColumn#1".to_string()
                } else {
                    // `Blob.Length` compared with 0: only the sign of the stored size is observed (issue 6896; blob_length_sign.rs).
                    Self::blob_length_emit_id(&id, node)
                };
                // `IsNull` of a `[RunOnClient]` DotNet variable: the variable can never be assigned (its constructor and
                // `Create` raise), so it is null (onprem-dotnet/lt-bg-runonclient/BgRunOnClient.al:171-176, :211-223
                // `ok Yes`; the second pin is on corpus master from PR 790).
                // `Clear` of a `[RunOnClient]` variable: it stays null (corpus master 92fdde19,
                // onprem-dotnet/lt-bg-runonclient/BgRunOnClient.al:454-480 Y16_RunOnClient_ClearAndVariant `cleared isnull=Yes`).
                if (id == "System.IsNull#1" || id == "System.Clear#1")
                    && args.len() == 1
                    && self.run_on_client_var(&args[0]).is_some()
                {
                    return Ok(ir::Expr::Const(ir::Const::Boolean(true)));
                }
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
                            && r.text_string().trim().eq_ignore_ascii_case("CurrReport") =>
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
                            && is_request_options_page(&r.text_string()) =>
                    {
                        None
                    }
                    Some(r) if sig.receiver == "Enum" && sig.name != "AsInteger" => Some(self.enum_receiver(r, &id)?),
                    // A Label is its text: the variable holds none (reads lower to the constant).
                    Some(r) if sig.instance && sig.receiver == "Label" => Some(Box::new(ir::Arg::Value(self.expr(r)?))),
                    Some(r) if sig.instance => Some(Box::new(match self.place(r) {
                        Ok(p) => ir::Arg::Ref(p),
                        Err(_) => ir::Arg::Value(self.expr(r)?),
                    })),
                    // `this.Run()` in a codeunit runs the codeunit's own instance: its globals see the run, an error is
                    // returned when the result is used, and the record form hands the record to OnRun
                    // (xmlports/shapes-properties/AjRunAndXmlPort.al:6-140, the ThisRun_* tests). The bare `Run()` / `Run(Rec)`
                    // does the same (AjRunAndXmlPort.al:170,194,215, the QZ_BareRun* tests).
                    None if sig.instance
                        && id == "Codeunit.Run#1"
                        && self.ws.objects[self.oi].kind == ObjKind::Codeunit
                        && (callee.kind() == SyntaxKind::NameRef
                            || callee.kind() == SyntaxKind::MemberExpr
                                && callee.children().next().is_some_and(|r| crate::check::is_this_keyword(&r))) =>
                    {
                        Some(Box::new(ir::Arg::Value(ir::Expr::ThisInstance)))
                    }
                    // A bare page method in a page or page extension (`Update()`, `Close()`) runs on `CurrPage`, as the
                    // `CurrPage.` form does (al-runner issue 6021; the checker resolves it in `check/calls.rs`).
                    None if sig.instance
                        && sig.receiver == "Page"
                        && callee.kind() == SyntaxKind::NameRef
                        && matches!(self.ws.objects[self.oi].kind, ObjKind::Page | ObjKind::PageExtension) =>
                    {
                        let slot = self.globals.get("currpage").ok_or("`CurrPage` is not lowered for this object")?;
                        Some(Box::new(ir::Arg::Ref(ir::Place::Global(*slot))))
                    }
                    // A bare report method in a report's or report extension's procedure is the `CurrReport.` form: the VM
                    // takes the report whose code runs (reports/bare-report-member/Y36BareReportMember.al:191-342, also
                    // on a report variable never run and from a report extension; the checker resolves only the pinned names,
                    // `check::calls::BARE_REPORT_MEMBERS`). In a trigger only the read `UseRequestPage` is pinned (`bare_report_receiver`). The bare `RunModal()` and
                    // `Run()` run the report's own instance: the triggers run once, the globals they set read back through the
                    // variable, a second call raises ObjectHasAlreadyBeenRun (reports/runtime/Y40BareReportRun.al:
                    // 214-377, corpus master ac3499d6). From a report extension they are not pinned.
                    None if sig.instance
                        && sig.receiver == "Report"
                        && callee.kind() == SyntaxKind::NameRef
                        && matches!(self.ws.objects[self.oi].kind, ObjKind::Report | ObjKind::ReportExtension) =>
                    {
                        self.bare_report_receiver(node, &sig.name, args.is_empty())?
                    }
                    // An unqualified record method in table code runs on the implicit Rec.
                    None if sig.instance && matches!(sig.receiver.as_str(), "Record" | "Query") => {
                        Some(Box::new(ir::Arg::Ref(self.implicit_rec(node)?)))
                    }
                    // A bare `Run(Page::X, ...)` in a page is the static Page.Run (the checker resolves it, `check::calls`;
                    // pages/run-modal/Y47UnqualifiedRun.al:237-325, al-runner issue 6590).
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
                        let receiver_ty = receiver_node.as_ref().map(|r| self.ty_of(r));
                        out.push(ir::Arg::Value(self.dotnet_builtin_arg(&id, p, a, &alias, receiver_ty)?));
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
                        let name = al_syntax::ast::significant_green_tokens(a)
                            .find(|t| matches!(t.kind(), SyntaxKind::Ident | SyntaxKind::QuotedIdent))
                            .map(|t| ast::unquote_ident(t.text()))
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
                    // `FilterPageBuilder.AddField(Name, Rec."Field", ..)`: the field of a record variable, passed as that record
                    // and the field's position, so that the runtime can check it is a field of the control's table
                    // (pages/filterpagebuilder/W3FilterPageBuilder.al:180, metadata/platform-tables/PlTFilterPageBuilder.al:225-230).
                    if id.starts_with("FilterPageBuilder.AddField#") && i == 1 {
                        if let Some(Res::Field { table, id: fid }) = self.res(a).cloned() {
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
                    }
                    // A field reference is passed for a parameter that names a field (`Param::names_field`); every other
                    // parameter takes the field's VALUE (`RecRef.Field(Buf."Field No.")`, `RecRef.FieldExist(Buf.Name)`).
                    match (p.names_field(), self.res(a).cloned()) {
                        // `Page.RunModal(Id, Rec, Rec."Field")` names the field by its NUMBER (the runtime checks the table has it, errors/
                        // P106RunErrors.al: a literal 999 is "supplied field number '999' cannot be found"); a field position would be wrong
                        // for a table whose numbers have gaps (Segment Line."Contact Via" is field 34, position 30).
                        (true, Some(Res::Field { table, id: fid })) if id.starts_with("Page.Run") => {
                            self.readable_field_pos(table, fid)?;
                            out.push(ir::Arg::Value(ir::Expr::Const(ir::Const::Integer(fid as i32))));
                            continue;
                        }
                        (true, Some(Res::Field { table, id: fid })) => {
                            self.named_field_pos(table, fid, &id)?;
                            let field = self.field_ref(table, fid);
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
                    // qualifies it (`ListPage."Integer Field"`, testpage/TestBCReportHandlerContracts.al:247);
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
                        self.ref_arg(&id, i, a)?
                    } else if let Some(el) = elem {
                        ir::Arg::Value(ir::Expr::Convert(self.rt(&el)?, Box::new(self.expr(a)?)))
                    // A Variant argument with a standard format number 3 to 8 raises an error that names the type
                    // 'Variant' (corpus types/decimal-integer/ScT70052.al): `System.FormatVariant#1` checks the number when it runs,
                    // a literal, a variable or an expression alike.
                    } else if id.starts_with("System.Format#") && i == 0 {
                        ir::Arg::Value(self.format_option_field(
                            args.len() == 1,
                            args.len() == 3
                                && args[1].text_string().trim() == "0"
                                && args[2].text_string().trim() == "9",
                            a,
                        )?)
                    // The substituted values of StrSubstNo, Message and Error are `Format`ted: an Option
                    // field shows its caption (corpus xmlports/PWXmlPortFormatEvaluate.al:Import_Xml_Option,
                    // `%3` of a field with OptionCaption).
                    } else if i > 0
                        && (id.starts_with("Text.StrSubstNo#")
                            || id.starts_with("Dialog.Message#")
                            || id.starts_with("Dialog.Error#"))
                    {
                        // A Decimal field that declares its format is substituted as `Format(Rec.Field)`, the version gate
                        // of an amount without an expression included (text/text-methods/RwDecimalFieldPlaces.al:83
                        // RW_StrSubstNo_DecimalField_TempRecord, :214 a row read with Get, :293 Message and Error; misc/
                        // lt-sa-unit-amount-subst/SaUnitAmountSubst.al: AutoFormatType 1, 2, 4 and 5 with a field of the row, a
                        // constant code or no expression, Message and Error too). The places are lost through any other carrier
                        // (RwDecimalFieldPlaces.al:242: a variable, `(Rec.F)`, `Rec.F + 0`, a FieldRef; SaUnitAmountSubst.al
                        // `varA2`). The shapes no corpus test measures are refused (DecimalPlaces beside another AutoFormat).
                        let unpinned = self.decimal_field_format_unpinned(a);
                        let without_expression = self.auto_format_field_without_expression(a);
                        let places = if without_expression.is_none() {
                            self.decimal_field_places(a)
                        } else {
                            None
                        };
                        let expression = if without_expression.is_none() && places.is_none() {
                            self.auto_format_expression_field(a)
                        } else {
                            None
                        };
                        match (places, without_expression.or(expression)) {
                            _ if !self.decimal_field_declares_format(a) || self.decimal_field_is_plain(a) => {
                                ir::Arg::Value(self.format_option_field(true, false, a)?)
                            }
                            _ if unpinned.is_some() => ir::Arg::Value(ir::Expr::Refused(unpinned.expect("checked"))),
                            (Some((places, mode)), _) => ir::Arg::Value(ir::Expr::Builtin {
                                id: builtin_id("System.FormatDecimalField#1")?,
                                receiver: None,
                                args: vec![
                                    ir::Arg::Value(self.expr(a)?),
                                    ir::Arg::Value(ir::Expr::Const(ir::Const::Integer(0))),
                                    ir::Arg::Value(ir::Expr::Const(ir::Const::Integer(0))),
                                    ir::Arg::Value(ir::Expr::Const(ir::Const::Text(places))),
                                    ir::Arg::Value(ir::Expr::Const(ir::Const::Integer(mode))),
                                ],
                                ret_used: true,
                            }),
                            (None, Some((record, field_no, kind))) => ir::Arg::Value(ir::Expr::Builtin {
                                id: builtin_id("System.FormatAutoFormatField#1")?,
                                receiver: None,
                                args: vec![
                                    ir::Arg::Value(self.expr(a)?),
                                    ir::Arg::Value(ir::Expr::Const(ir::Const::Integer(0))),
                                    ir::Arg::Value(ir::Expr::Const(ir::Const::Integer(0))),
                                    ir::Arg::Value(ir::Expr::Load(record)),
                                    ir::Arg::Value(ir::Expr::Const(ir::Const::Integer(field_no as i32))),
                                    ir::Arg::Value(ir::Expr::Const(ir::Const::Integer(kind))),
                                ],
                                ret_used: true,
                            }),
                            (None, None) => ir::Arg::Value(ir::Expr::Refused(
                                "StrSubstNo, Message or Error %n of a Decimal field with this DecimalPlaces or AutoFormat: its places are not pinned (docs/probes-needed.md)".into(),
                            )),
                        }
                    } else {
                        ir::Arg::Value(self.expr(a)?)
                    });
                }
                // Evaluate into an Option field reads its captions: the text goes through `System.TranslatedText#1`, refused where a
                // translation file could change them (ADR-053 step 1).
                if id == "System.Evaluate#1" && matches!(out.get(1), Some(ir::Arg::Value(_))) {
                    if let Some(owner) = args.first().and_then(|a| self.option_caption_owner(a)) {
                        let members = self.option_field_members(&args[0]).unwrap_or_default();
                        let placeholder = ir::Arg::Value(ir::Expr::Const(ir::Const::Integer(0)));
                        if let ir::Arg::Value(v) = std::mem::replace(&mut out[1], placeholder) {
                            out[1] = ir::Arg::Value(self.translated_option_text(v, owner, Some(&members))?);
                        }
                    }
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
                // A fifth argument 1: the fourth is the OptionCaptionML property text.
                // A fifth argument 2: the field declares no caption and the fourth is its member names.
                if let (true, Some((captions, kind))) = (emit_id == "System.EvaluateOptionField#1", option_captions) {
                    while out.len() < 3 {
                        out.push(ir::Arg::Value(ir::Expr::Const(ir::Const::Integer(0))));
                    }
                    out.push(ir::Arg::Value(ir::Expr::Const(ir::Const::Text(captions))));
                    match kind {
                        super::stmt::OptionCaptions::Ml => out.push(ir::Arg::Value(ir::Expr::Const(ir::Const::Integer(1)))),
                        super::stmt::OptionCaptions::None => {
                            out.push(ir::Arg::Value(ir::Expr::Const(ir::Const::Integer(2))))
                        }
                        super::stmt::OptionCaptions::Caption => {}
                    }
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
                // A sixth argument: the AutoFormatType of a field without an expression (`auto_format_field_without_expression`), else 0.
                if let (true, Some((record, pos, kind))) = (emit_id == "System.FormatAutoFormatField#1", auto_format_field) {
                    while out.len() < 3 {
                        out.push(ir::Arg::Value(ir::Expr::Const(ir::Const::Integer(0))));
                    }
                    out.push(ir::Arg::Value(ir::Expr::Load(record)));
                    out.push(ir::Arg::Value(ir::Expr::Const(ir::Const::Integer(pos as i32))));
                    out.push(ir::Arg::Value(ir::Expr::Const(ir::Const::Integer(kind))));
                }
                // The places, and when they hold (`decimal_field_places`: always, from BC 28 only, or under the measured format
                // strings only).
                if let (true, Some((places, mode))) = (emit_id == "System.FormatDecimalField#1", decimal_field_places) {
                    while out.len() < 3 {
                        out.push(ir::Arg::Value(ir::Expr::Const(ir::Const::Integer(0))));
                    }
                    out.push(ir::Arg::Value(ir::Expr::Const(ir::Const::Text(places))));
                    out.push(ir::Arg::Value(ir::Expr::Const(ir::Const::Integer(mode))));
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
            // `CurrPage.<AddIn>.Method()`: the add-in was never created, so the call raises an error when it runs (ADR-027
            // section 3 row 18; corpus testpage/ACControlAddIn.al TZ_*), and code that never reaches it runs (Base Application's
            // "Item Tracking Lines" calls its barcode add-in only after the client said it is supported).
            None if callee.kind() == SyntaxKind::MemberExpr
                && callee.children().next().and_then(|b| self.usercontrol_name(&b)).is_some() =>
            {
                let control = callee
                    .children()
                    .next()
                    .and_then(|b| self.usercontrol_name(&b))
                    .unwrap_or_default();
                Ok(self.control_add_in_call(node, control, args))
            }
            // A procedure of a control add-in called through a `ControlAddIn` variable or parameter (al-runner issue 6154: a page
            // part keeps the add-in its host page handed it, and a flag guards every call). Which add-in such a variable holds,
            // and what the call does in a test session, is not pinned: the call refuses when it runs, and code that does not
            // reach it runs.
            None if receiver_node
                .as_ref()
                .is_some_and(|r| matches!(self.ty_of(r), Ty::Builtin(ref n) if n.eq_ignore_ascii_case("ControlAddIn"))) =>
            {
                Ok(ir::Expr::Refused(format!(
                    "call `{}`: a procedure of a control add-in held by a ControlAddIn variable (not pinned)",
                    node.text()
                )))
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
    /// Windows too (reports/runtime/BvReportCharArg.al:72-116); a local procedure converts it (:160-170, the
    /// runtime's usual conversion). The same call on another codeunit was measured on the Linux tiers only (:140-158), so it
    /// is refused until the Windows check; other receivers, other number types and other Char expressions are not pinned and
    /// are refused. `None`: not such an argument.
    /// Enum metadata calls (`"Name".Names()`, `Var.Ordinals()`, `"Name".FromInteger(n)`) receive the enum itself: its object
    /// index, as an enum constant. `Ns.Sub."Name".FromInteger(n)` names it through its namespace (`Res::NamespacedEnumReceiver`).
    fn enum_receiver(&mut self, r: &SyntaxNode, id: &str) -> L<Box<ir::Arg>> {
        let object = if let Some(Res::NamespacedEnumReceiver(ei)) = self.res(r) {
            let ei = *ei;
            self.assume(al_builtins::assumptions::Assumption::NamespacedEnumReceiver);
            ei
        } else {
            let Ty::Enum(name) = self.ty_of(r) else {
                return Err(format!("{id}: the receiver is not an enum type"));
            };
            self.resolve_obj(ObjKind::Enum, &name)?
        };
        Ok(Box::new(ir::Arg::Value(ir::Expr::Const(ir::Const::Enum(
            object as u32,
            0,
        )))))
    }

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
    /// events/binding-senders/WithStatementAmbiguous.al:277-279, see `Checker::lookup_callable`).
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
    /// pins (the checker's AR1006, see `Checker::lookup_callable`; streams/CjDataset.al:215-241 pins
    /// only one procedure on each side with the same parameters). Also a bare call in a third object's code to a name a
    /// table and another app's table extension declare with the same parameters (`same_signature_in_two_objects`).
    fn report_item_same_name_refusal(&self, node: &SyntaxNode) -> L<()> {
        let path = &self.ws.files[self.file].path;
        let at: usize = node.text_range().start().into();
        match self.checked.diagnostic_at(path, at, &["AR1006"]) {
            Some(d) => Err(d.message.clone()),
            None => Ok(()),
        }
    }

    /// A call of a procedure AL code declares that the lowering refuses: two overloads take the arguments and one needs an
    /// enum conversion (the checker's AR1008, `check/enum_overload.rs`), or the procedure has a twin in a page or report
    /// extension (`twin_call_refusal`). An overload chosen by an assumed rule (AR1009) marks the assumption.
    fn proc_call_refusal(&mut self, node: &SyntaxNode, object: usize, offset: usize) -> L<()> {
        let path = &self.ws.files[self.file].path;
        let at: usize = node.text_range().start().into();
        let here = |code: &str| self.checked.diagnostic_at(path, at, &[code]);
        if let Some(d) = here("AR1008") {
            return Err(d.message.clone());
        }
        // The overload choice an enum argument made by the assumed Integer rule (`enum_overload_verdict`).
        if here("AR1009").is_some() {
            self.assume(al_builtins::assumptions::Assumption::EnumOverloadOwnType);
        }
        self.twin_call_refusal(object, offset)
    }

    /// The checker's runtime refusal of this call, if any: AL0666 (a builtin newer than the app's runtime) or AR1007
    /// (an app below the runtime floor calls a builtin AL Runner cannot clear for it; al-runner issue 5573).
    pub(crate) fn runtime_refusal(&self, node: &SyntaxNode) -> Option<String> {
        let path = &self.ws.files[self.file].path;
        let at: usize = node.text_range().start().into();
        self.checked
            .diagnostic_at(path, at, &["AL0666", "AR1007"])
            .map(|d| format!("{} {}", d.code, d.message))
    }
}

/// The [`ir::BuiltinId`] of a catalog overload id. Every catalog id has one (a test in al-builtins checks it), so
/// this fails only when `builtin_ids.rs` is stale.
impl Lowerer<'_> {
    /// How the text argument `a` of `Error` or `FieldError` is written, as far as the corpus tells the forms apart for
    /// `GetLastErrorText(true)`: a string literal; a Label or TextConst variable of the object or of the routine
    /// (errors/last-error/LastErrorTextExcludeLabel.al:28-32, lt-last-error-text-forms/
    /// LastErrorTextExcludeTextConst.al:15-17, FieldErrorTextConst.al:28); a Text or Code variable of the routine, a
    /// concatenation with `+` or a `StrSubstNo` call (LastErrorTextExcludeTextVariable.al:16-54, :129,
    /// FieldErrorTextVariable.al:43-63, LastErrorTextExcludeLabel.al:55). Anything else (a global Text variable, a field,
    /// another call that is not a procedure with a Text result) is `Other`.
    /// The catalog id of `Error(Label, ...)` or `Error(Variable, ...)`: the text `GetLastErrorText(true)` answers depends on
    /// how the text is written (al_builtins::native::ErrorTextForm); a string literal keeps `Dialog.Error#2` (corpus
    /// P86LastError.al:PlainError_Exclude).
    fn error_variant_id(&mut self, args: &[SyntaxNode]) -> String {
        use al_builtins::native::ErrorTextForm;
        if args.first().is_some_and(|a| self.is_user_proc_text(a)) {
            self.assume(al_builtins::assumptions::Assumption::ErrorTextProcedureResult);
        }
        match args.first().map_or(ErrorTextForm::Other, |a| self.error_text_form(a)) {
            ErrorTextForm::Constant => "Dialog.ErrorConstant#1",
            ErrorTextForm::Computed | ErrorTextForm::Literal => "Dialog.ErrorComputed#1",
            ErrorTextForm::Other => "Dialog.ErrorNotLiteral#1",
        }
        .to_string()
    }

    /// The catalog id of `FieldError(Field, Text)`: as for `Error`, the flag's text depends on how the text is written.
    /// Only a Label or TextConst of the object is pinned (FieldErrorTextConst.al:28); a local one stays refused.
    fn field_error_variant_id(&self, id: &str, args: &[SyntaxNode]) -> String {
        use al_builtins::native::ErrorTextForm;
        let local_label = args
            .get(1)
            .is_some_and(|a| matches!(self.res(a), Some(Res::Local(_))) && matches!(self.ty_of(a), Ty::Label));
        match args.get(1).map_or(ErrorTextForm::Other, |a| self.error_text_form(a)) {
            ErrorTextForm::Constant if local_label => id,
            ErrorTextForm::Constant => "Record.FieldErrorConstant#1",
            ErrorTextForm::Computed | ErrorTextForm::Literal => "Record.FieldErrorComputed#1",
            ErrorTextForm::Other => id,
        }
        .to_string()
    }

    /// `a` is a call of an AL procedure that returns a Text or a Code.
    fn is_user_proc_text(&self, a: &SyntaxNode) -> bool {
        a.kind() == SyntaxKind::CallExpr
            && matches!(self.res(a), Some(Res::Proc { .. }))
            && matches!(self.ty_of(a), Ty::Text(_) | Ty::Code(_))
    }

    pub(crate) fn error_text_form(&self, a: &SyntaxNode) -> al_builtins::native::ErrorTextForm {
        use al_builtins::native::ErrorTextForm;
        if a.kind() == SyntaxKind::Literal {
            return ErrorTextForm::Literal;
        }
        let ty = self.ty_of(a);
        match a.kind() {
            SyntaxKind::BinExpr
                if matches!(ty, Ty::Text(_) | Ty::Code(_))
                    && sig_tokens(a).first().is_some_and(|t| t.text() == "+") =>
            {
                return ErrorTextForm::Computed;
            }
            SyntaxKind::CallExpr => {
                let strsubstno = matches!(self.res(a), Some(Res::Builtin(i)) if i.starts_with("Text.StrSubstNo#"));
                // A procedure's Text or Code result is a value: a Label returned as text answers the redacted text
                // (errors/last-error/LastErrorTextExcludeLabel.al:510-540, :631-634, ExcludeFlag_LabelHandedOnAsText
                // `proc_true`). Only that shape is pinned (`error-text-procedure-result`).
                let user_proc_text =
                    matches!(self.res(a), Some(Res::Proc { .. })) && matches!(ty, Ty::Text(_) | Ty::Code(_));
                return if strsubstno || user_proc_text {
                    ErrorTextForm::Computed
                } else {
                    ErrorTextForm::Other
                };
            }
            _ => {}
        }
        match (self.res(a), ty) {
            // A local Label or TextConst answers as a global one does, for every argument type and property
            // (LastErrorTextExcludeLabel.al:423-540, :564-585; LastErrorTextExcludeTextConst.al:107-125).
            (Some(Res::Global(_) | Res::Local(_)), Ty::Label) => ErrorTextForm::Constant,
            (Some(Res::Local(_)), Ty::Text(_) | Ty::Code(_)) => ErrorTextForm::Computed,
            _ => ErrorTextForm::Other,
        }
    }
}

pub(super) fn builtin_id(id: &str) -> Result<ir::BuiltinId, String> {
    ir::BuiltinId::of(id).ok_or_else(|| format!("builtin {id} has no builtin id (regenerate builtin_ids.rs)"))
}

/// The parameter type name a collection element of type `t` stands for, in the spelling of the
/// builtin catalog (empty when no DotNet value may be passed for it).
pub(super) fn element_param_name(t: &Ty) -> String {
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

/// `RequestOptionsPage` or `CurrReport.RequestOptionsPage`: the request page of the report whose code is running.
fn is_request_options_page(text: &str) -> bool {
    text.trim()
        .rsplit('.')
        .next()
        .is_some_and(|last| last.eq_ignore_ascii_case("RequestOptionsPage"))
}
