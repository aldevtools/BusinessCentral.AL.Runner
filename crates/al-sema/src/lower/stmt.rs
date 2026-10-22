//! Lowering of routines and statements.
use super::*;

impl Lowerer<'_> {
    pub(crate) fn routine(&mut self, r: &ast::Routine) -> ir::Routine {
        #[cfg(test)]
        if r.name() == "InjectedLowerPanic" {
            panic!("injected lowering panic");
        }
        // A routine the compiler made up (a page expression) has no source.
        self.emit_pos = self.lines.is_some() && !crate::pages::is_synthetic(&r.name());
        let pos = match r.keyword_offset().filter(|_| self.emit_pos) {
            Some(k) => self.add_pos(k, k + 1).unwrap_or(ir::NO_POS),
            None => ir::NO_POS,
        };
        let end_pos = match r.end_offset().filter(|_| self.emit_pos) {
            Some(k) => self.add_pos(k, k + 1).unwrap_or(ir::NO_POS),
            None => ir::NO_POS,
        };
        let begin_pos = match r.begin_offset().filter(|_| self.emit_pos) {
            Some(k) => self.add_pos(k, k + 1).unwrap_or(ir::NO_POS),
            None => ir::NO_POS,
        };
        self.locals.clear();
        self.local_labels.clear();
        self.local_textconsts.clear();
        self.local_var_attrs.clear();
        self.tys.clear();
        self.tys_on = true;
        let mut locals = Vec::new();
        let mut by_var = Vec::new();
        let add = |lw: &mut Self, name: String, ty: Ty, locals: &mut Vec<ir::Slot>| match lw.rt(&ty) {
            Ok(rt) => {
                lw.locals.insert(name.to_lowercase(), locals.len());
                locals.push(ir::Slot { name, ty: rt });
            }
            Err(e) => {
                lw.reasons.push(format!("`{name}`: {e}"));
                lw.locals.insert(name.to_lowercase(), locals.len());
                locals.push(ir::Slot {
                    name,
                    ty: RtType::Variant,
                });
            }
        };
        for p in r.params() {
            by_var.push(p.is_var());
            let ty = p.type_ref().map(|t| Ty::from_type_ref(&t)).unwrap_or(Ty::Unknown);
            add(self, p.name(), ty, &mut locals);
        }
        let params = locals.len();
        self.var_params.clone_from(&by_var);
        self.reads_var_max_len = false;
        let ret_ty = r.return_type().map(|t| Ty::from_type_ref(&t));
        self.ret_decl = ret_ty.clone();
        let ret = match &ret_ty {
            Some(t) => match self.rt(t) {
                Ok(rt) => Some(rt),
                Err(e) => {
                    self.reasons.push(format!("return type: {e}"));
                    None
                }
            },
            None => None,
        };
        self.ret_ty = ret.clone();
        let mut ret_local = None;
        if let (Some(n), Some(t)) = (r.return_name(), &ret_ty) {
            ret_local = Some(locals.len());
            add(self, n, t.clone(), &mut locals);
        }
        let mut declared_security: Vec<(String, usize, Result<i32, String>)> = Vec::new();
        // The lowercased body text without whitespace is built only when a `[SecurityFiltering]` record variable needs it:
        // building it for every routine cost about 18 % of a cold compile (perf_budget).
        let mut body_text: Option<String> = None;
        for v in r.local_var_decls() {
            let ty = v.type_ref().map(|t| Ty::from_type_ref(&t)).unwrap_or(Ty::Unknown);
            let label = v.type_ref().as_ref().and_then(label_text);
            let refusal = v.type_ref().as_ref().and_then(textconst_refusal);
            let attrs = var_attribute_names(&v);
            let security = attrs
                .iter()
                .any(|a| a == "securityfiltering")
                .then(|| security_filtering_attr_member(&v));
            for n in v.names() {
                if let Some(m) = &security {
                    // Only a record keeps the declared value, and only one the routine does not clear or assign
                    // (what those leave is not pinned): on anything else the read stays refused.
                    if matches!(self.rt(&ty), Ok(RtType::Record(..)))
                        && !cleared_or_assigned(
                            body_text.get_or_insert_with(|| {
                                r.body()
                                    .map(|b| {
                                        b.text()
                                            .to_string()
                                            .to_lowercase()
                                            .chars()
                                            .filter(|c| !c.is_whitespace())
                                            .collect()
                                    })
                                    .unwrap_or_default()
                            }),
                            &n,
                        )
                    {
                        declared_security.push((n.clone(), locals.len(), m.clone()));
                    }
                }
                if !attrs.is_empty() {
                    let mut attrs = attrs.clone();
                    if declared_security.last().is_some_and(|(d, ..)| *d == n) {
                        attrs.retain(|a| a != "securityfiltering");
                    }
                    self.local_var_attrs.insert(n.to_lowercase(), attrs);
                }
                if let Some(l) = &label {
                    self.local_labels.insert(n.to_lowercase(), l.clone());
                }
                if is_textconst(v.type_ref().as_ref()) {
                    self.local_textconsts.insert(n.to_lowercase());
                }
                if let Some(why) = refusal {
                    self.reasons.push(format!("`{n}`: {why}"));
                }
                add(self, n, ty.clone(), &mut locals);
            }
        }
        // An InStream handed back (the named return value or a `var` parameter) that the routine created on a local
        // variable (a Temp Blob, a temporary record's Blob field) reads as empty once the local is out of scope
        // (streams/lt-dc-stream-leads/DcInStreamScope.al:107-112, :125-126). The runtime keeps the bytes: refused.
        let is_instream = |t: Option<al_syntax::ast::TypeRef>| {
            t.is_some_and(|t| t.syntax().text().to_string().trim().eq_ignore_ascii_case("instream"))
        };
        let mut handed: Vec<String> = r
            .params()
            .filter(|p| p.is_var() && is_instream(p.type_ref()))
            .map(|p| p.name().to_lowercase())
            .collect();
        if let Some(n) = r.return_name().filter(|_| is_instream(r.return_type())) {
            handed.push(n.to_lowercase());
        }
        if !handed.is_empty() {
            let local_names: Vec<String> = r
                .local_var_decls()
                .flat_map(|v| v.names())
                .map(|n| n.to_lowercase())
                .collect();
            if let Some(why) = r
                .body()
                .and_then(|b| self.stream_of_a_local_handed_back(&b, &handed, &local_names))
            {
                self.reasons.push(why);
            }
        }
        // OnQueryClosePage with a Boolean return that ends without giving a value lets the page close. Not pinned (the corpus
        // triggers all `exit(true)` or `exit(false)`); Item Tracking Lines' ends with `Result` untouched when the availability
        // check finds nothing to warn about, and Microsoft's tests close it (Tests-SCM, about 1,100 tests; assumption
        // query-close-page-falls-through-as-true). A named return value starts unassigned: a read of it before an assignment is
        // refused, and the trigger's answer is Yes when it ends without an `exit(value)` and without an assignment.
        let qcp =
            r.is_trigger() && r.name().eq_ignore_ascii_case("onqueryclosepage") && matches!(ret, Some(RtType::Boolean));
        if qcp {
            if let Some(l) = ret_local {
                let flag = locals.len();
                locals.push(ir::Slot {
                    name: "__assigned".into(),
                    ty: RtType::Boolean,
                });
                self.qcp_ret = Some((l, flag));
            }
        }
        self.hidden_base = locals.len();
        self.hidden_locals.clear();
        let mut body = match r.body() {
            Some(b) => self.stmts_of(&b),
            None => Vec::new(),
        };
        // `--tdd` (ADR-051 decision 10): a routine with a variable of a stub object instantiates the object.
        if self.checked.tdd_decls.as_ref().is_some_and(|d| !d.objects.is_empty()) {
            let marks = self.tdd_object_marks(r);
            body.splice(0..0, marks);
        }
        if !declared_security.is_empty() {
            let prologue = self.declared_security_filtering(&declared_security, &locals);
            body.splice(0..0, prologue);
        }
        locals.append(&mut self.hidden_locals);
        if qcp {
            // A Boolean OnQueryClosePage that reaches its end without a value lets the page close
            // (pages/lt-query-close-page-result/QcrNamedBranches.al:OnQueryClosePage_NamedTrueIfFlagFlagFalseHandlerOk_Observed and its siblings).
            let yes = ir::Expr::Const(ir::Const::Boolean(true));
            match self.qcp_ret.take() {
                Some((l, flag)) => {
                    let guard = vec![ir::Stmt::If(
                        ir::Expr::Unary(ir::UnOp::Not, Box::new(ir::Expr::Load(ir::Place::Local(flag)))),
                        vec![ir::Stmt::Assign(ir::Place::Local(l), yes.clone())],
                        Vec::new(),
                    )];
                    body = guard_exits(body, &guard);
                    body.extend(guard);
                }
                None => {
                    body.push(ir::Stmt::Exit(Some(yes)));
                }
            }
        }
        // The service tier implements a `[Native]` procedure; its AL body is empty. Running that body
        // would answer the default value of the return type, which is not what the platform answers.
        // A few have an implementation here (`platform_native`, pinned by the corpus); the others are refused.
        let native = (body.is_empty() && r.attributes().any(|a| a.name().eq_ignore_ascii_case("native")))
            .then(|| {
                let types: Vec<String> = r
                    .params()
                    .map(|p| p.type_ref().map(|t| t.type_name().to_lowercase()).unwrap_or_default())
                    .collect();
                al_builtins::platform_native::find(&self.ws.objects[self.oi].name, &r.name(), &types)
            })
            .flatten()
            .and_then(|entry| ir::BuiltinId::of(entry.id));
        if let Some(id) = native {
            let call = ir::Expr::Builtin {
                id,
                receiver: None,
                args: (0..params)
                    .map(|i| ir::Arg::Value(ir::Expr::Load(ir::Place::Local(i))))
                    .collect(),
                ret_used: ret_ty.is_some(),
            };
            body = vec![if ret_ty.is_some() {
                ir::Stmt::Exit(Some(call))
            } else {
                ir::Stmt::Expr(call)
            }];
        } else if body.is_empty() && r.attributes().any(|a| a.name().eq_ignore_ascii_case("native")) {
            self.reasons.push(format!(
                "[Native] procedure `{}` is implemented by the platform and has no AL body (what it answers is not pinned)",
                r.name()
            ));
        }
        let expr_tys = self.take_table(&body);
        self.tys_on = false;
        let field = r
            .syntax()
            .ancestors()
            .filter_map(ast::Section::cast)
            .find(|s| s.name().eq_ignore_ascii_case("field"))
            .and_then(|s| {
                let id: u32 = s.args().first()?.parse().ok()?;
                self.table_of[self.oi].and_then(|t| self.field_pos(t, id))
            });
        let container = r.syntax().ancestors().find_map(ast::Section::cast).map(|sec| {
            let arg = sec.args().first().cloned().unwrap_or_default();
            (
                sec.name().to_ascii_lowercase(),
                arg.trim().trim_matches('"').to_string(),
            )
        });
        ir::Routine {
            name: r.name(),
            field,
            container,
            is_trigger: r.is_trigger(),
            attributes: self.attributes_of(r),
            params,
            by_var,
            locals,
            ret,
            ret_local,
            body: body.into(),
            pos,
            end_pos,
            begin_pos,
            expr_tys,
            reads_var_max_len: self.reads_var_max_len,
        }
    }

    /// The first argument of `Format`. An Option field with an `OptionCaption` formats as the
    /// caption, not the member name (corpus fieldref/TestPrecompiledFieldOptionCaption.al:47-55,
    /// a Base Application table whose caption differs from its member). The value keeps its
    /// ordinal and takes the caption list. Other Format forms on such a field are not pinned.
    /// `number9`: the call is `Format(x, 0, 9)`.
    ///
    /// A member named through the field, `Format(Row.Opt::Two)`, shows its caption too (misc/lt-probes-needed-2/
    /// PnBRecordMisc.al:96, :113: `Dos`).
    pub(crate) fn format_option_field(&mut self, plain: bool, number9: bool, arg: &SyntaxNode) -> L<ir::Expr> {
        let value = self.expr(arg)?;
        let member = arg.kind() == SyntaxKind::ScopeExpr;
        let field_node = if member {
            match arg.children().next() {
                Some(lhs) => lhs,
                None => return Ok(value),
            }
        } else {
            arg.clone()
        };
        let Some(Res::Field { table, id: field }) = self.res(&field_node).cloned() else {
            return Ok(value);
        };
        let Some(f) = self.members[table].fields.iter().find(|f| f.id == field) else {
            return Ok(value);
        };
        // OptionCaptionML gives the captions of the session language (record/lt-x-table-props/XTablePropsTests.al:219-228):
        // the value takes them when it runs.
        if let (Ty::Option(_), Some((_, ml))) = (
            &f.ty,
            f.properties
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("OptionCaptionML")),
        ) {
            if member && !plain && !number9 {
                return Err(format!(
                    "Format of member {} of option field {} with a length or format number (only the plain form and `0, 9` are pinned)",
                    arg.text(),
                    f.name
                ));
            }
            let ml = ml.clone();
            return Ok(ir::Expr::Builtin {
                id: super::call::builtin_id("System.OptionCaptionML#1")?,
                receiver: None,
                args: vec![
                    ir::Arg::Value(value),
                    ir::Arg::Value(ir::Expr::Const(ir::Const::Text(ml))),
                ],
                ret_used: true,
            });
        }
        let (Ty::Option(names), Some((_, text))) = (
            &f.ty,
            f.properties
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("OptionCaption")),
        ) else {
            return Ok(value);
        };
        if al_syntax::ast::text_over_max_length(text) {
            return Err(format!(
                "Format of option field {} whose OptionCaption is longer than its MaxLength (whether it is cut is not pinned)",
                f.name
            ));
        }
        let captions: Vec<String> = match crate::members::first_string_literal(text) {
            Some(list) => list.split(',').map(str::to_string).collect(),
            None => return Err(format!("Format of option field {} with OptionCaption `{text}`", f.name)),
        };
        if &captions == names {
            return Ok(value);
        }
        // Every form of Format reads the caption list (text/TestProbeTextDateFormatContracts.al:
        // Format_OptionFieldAndOptionVariable): standard formats 2 and 9 the ordinal, a length cuts or pads the caption.
        // A member named through the field is pinned in the plain form (PnBRecordMisc.al:96) and with format number 9, which
        // gives the ordinal (misc/lt-probes-needed-5/PnE10Formats.al:96-97, :127).
        if member && !plain && !number9 {
            return Err(format!(
                "Format of member {} of option field {} with a length or format number (only the plain form and `0, 9` are pinned)",
                arg.text(),
                f.name
            ));
        }
        if captions.len() != names.len() {
            return Err(format!(
                "Format of option field {} whose OptionCaption has another number of entries than its members is not pinned",
                f.name
            ));
        }
        Ok(ir::Expr::Convert(RtType::Option(captions), Box::new(value)))
    }

    /// Evaluate into an Option field with OptionCaptionML: which captions it accepts is not pinned, so it is refused.
    pub(crate) fn refuse_evaluate_option_caption_ml(&mut self, arg: &SyntaxNode) -> L<()> {
        let Some(Res::Field { table, id: field }) = self.res(arg).cloned() else {
            return Ok(());
        };
        let Some(f) = self.members[table].fields.iter().find(|f| f.id == field) else {
            return Ok(());
        };
        if matches!(f.ty, Ty::Option(_))
            && f.properties
                .iter()
                .any(|(k, _)| k.eq_ignore_ascii_case("OptionCaptionML"))
        {
            return Err(format!(
                "Evaluate into option field {} with OptionCaptionML (which captions it takes is not pinned)",
                f.name
            ));
        }
        if matches!(f.ty, Ty::Option(_))
            && f.properties
                .iter()
                .any(|(k, v)| k.eq_ignore_ascii_case("OptionCaption") && al_syntax::ast::text_over_max_length(v))
        {
            return Err(format!(
                "Evaluate into option field {} whose OptionCaption is longer than its MaxLength (not pinned)",
                f.name
            ));
        }
        Ok(())
    }

    /// The OptionCaption entries of an Option table field passed as `arg`, as a comma list, when the field has a literal
    /// OptionCaption of as many entries as members that differs from them. `None` for any other argument.
    pub(crate) fn option_field_captions(&mut self, arg: &SyntaxNode) -> Option<String> {
        let Some(Res::Field { table, id: field }) = self.res(arg).cloned() else {
            return None;
        };
        let f = self.members[table].fields.iter().find(|f| f.id == field)?;
        let (Ty::Option(names), Some((_, text))) = (
            &f.ty,
            f.properties
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("OptionCaption")),
        ) else {
            return None;
        };
        let list = crate::members::first_string_literal(text)?;
        let captions: Vec<&str> = list.split(',').collect();
        (captions.len() == names.len() && captions.iter().zip(names).any(|(c, n)| c != n)).then(|| list.to_string())
    }

    /// The places `min:max` a Decimal table field declares for `Format`: its `DecimalPlaces`, else the precision of its
    /// `AutoFormatExpression` (type 10), else the 2:2 of a blank-expression amount (type 1) or 2:5 of a unit amount (type 2)
    /// (pages/lt-page-autoformat-lookups/P113AutoFormat.al:P113_AF_TableFieldFormatIsPlain, pages/lt-page-presentation-properties/
    /// P119Table.al:P119_DecimalPlaces). An amount (type 1) without an `AutoFormatExpression` gives 2:2 from BC 28 only (the
    /// `true` beside it: `Gate::AutoFormatAmountFieldRoundsFrom28`, types/lt-at-partner-gaps/AtPrecisionStandardFormat2.al:
    /// 684-723); a unit amount (type 2) without one keeps every digit (:684 `1,234.5678`). `None` for any other argument or field.
    pub(crate) fn decimal_field_places(&mut self, arg: &SyntaxNode) -> Option<(String, bool)> {
        let Some(Res::Field { table, id: field }) = self.res(arg).cloned() else {
            return None;
        };
        let f = self.members[table].fields.iter().find(|f| f.id == field)?;
        if !matches!(f.ty, Ty::Decimal) {
            return None;
        }
        let prop = |n: &str| {
            f.properties
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(n))
                .map(|(_, v)| v.trim().to_string())
        };
        let digits = |p: &str| p.split(':').all(|x| x.trim().parse::<u32>().is_ok());
        if let Some(p) = prop("DecimalPlaces").filter(|p| digits(p)) {
            return Some((p.replace(' ', ""), false));
        }
        let expr = prop("AutoFormatExpression");
        match prop("AutoFormatType").as_deref() {
            Some("10") => {
                let text = expr?;
                let rest = text.split("<Precision,").nth(1)?;
                let p = rest.split('>').next()?.trim().to_string();
                digits(&p).then_some((p, false))
            }
            Some("1") if expr.as_deref() == Some("''") => Some(("2:2".into(), false)),
            Some("1") if expr.is_none() => Some(("2:2".into(), true)),
            Some("2") if expr.as_deref() == Some("''") => Some(("2:5".into(), false)),
            _ => None,
        }
    }

    /// Whether `arg` is a Decimal table field whose AutoFormatExpression is an expression (not a text): `Format` of it in
    /// code applies the field's AutoFormat, evaluated with the record when it runs (types/lt-auto-format-currency/
    /// QiTableFieldAutoFormat.al:400-418 QI_TableFieldExpression_FormatInCode). Gives the place of the record and the
    /// field's number.
    pub(crate) fn auto_format_expression_field(&mut self, arg: &SyntaxNode) -> Option<(ir::Place, usize)> {
        let Some(Res::Field { table, id: field }) = self.res(arg).cloned() else {
            return None;
        };
        let f = self.members[table].fields.iter().find(|f| f.id == field)?;
        if !matches!(f.ty, Ty::Decimal) {
            return None;
        }
        let expr = f
            .properties
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("AutoFormatExpression"))
            .map(|(_, v)| v.trim().to_string())?;
        if expr.starts_with('\'') {
            return None;
        }
        match self.place(arg).ok()? {
            ir::Place::Field(base, _) => Some((*base, field as usize)),
            _ => None,
        }
    }

    pub(crate) fn attributes_of(&self, r: &ast::Routine) -> Vec<ir::Attribute> {
        r.attributes()
            .map(|a| ir::Attribute {
                text: a.syntax().text().to_string(),
                name: a.name(),
                args: a
                    .args()
                    .map(|l| l.children().filter_map(|e| self.constant(&e).ok()).collect())
                    .unwrap_or_default(),
            })
            .collect()
    }

    /// A routine of a dependency app that nothing reached: its name and attributes (so event
    /// subscribers stay registered), no body, and a reason that refuses anything that runs it.
    pub(crate) fn stub(&mut self, r: &ast::Routine) -> ir::Routine {
        self.refused(
            r,
            "not compiled: no code reached it while compiling the workspace's dependencies (ADR-013)".into(),
        )
    }

    /// A routine without a body, and `reason` that refuses anything that runs it.
    pub(crate) fn refused(&mut self, r: &ast::Routine, reason: String) -> ir::Routine {
        self.reasons.push(reason);
        ir::Routine {
            name: r.name(),
            field: None,
            container: None,
            is_trigger: r.is_trigger(),
            attributes: self.attributes_of(r),
            params: 0,
            by_var: Vec::new(),
            locals: Vec::new(),
            ret: None,
            ret_local: None,
            body: Vec::new().into(),
            pos: ir::NO_POS,
            end_pos: ir::NO_POS,
            begin_pos: ir::NO_POS,
            expr_tys: Vec::new(),
            reads_var_max_len: false,
        }
    }

    /// Records the position of the code at `start..end` of the file being lowered and returns its id.
    pub(crate) fn add_pos(&mut self, start: usize, end: usize) -> Option<u32> {
        let lines = self.lines.as_ref()?;
        let (line, col) = lines.locate(start);
        let (end_line, _) = lines.locate(end.saturating_sub(1).max(start));
        let p = ir::SrcPos {
            file: lines.file,
            line,
            col,
            end_line,
            object: self.oi as u32,
            routine: self.routine_idx as u32,
        };
        self.positions.push(p);
        Some(self.positions.len() as u32 - 1)
    }

    /// The marker for statement `n`, when the routine being lowered has positions.
    pub(crate) fn mark(&mut self, n: &SyntaxNode) -> Option<ir::Stmt> {
        if !self.emit_pos {
            return None;
        }
        let (start, end) = significant_span(n)?;
        self.add_pos(start, end).map(ir::Stmt::Pos)
    }

    /// Statement children of a block-like node, flattened.
    pub(crate) fn stmts_of(&mut self, n: &SyntaxNode) -> Vec<ir::Stmt> {
        let mut out = Vec::new();
        for c in n.children().filter(|c| is_stmt(c.kind())) {
            out.extend(self.stmt(&c));
        }
        out
    }

    pub(crate) fn stmt_or_empty(&mut self, n: Option<&SyntaxNode>) -> Vec<ir::Stmt> {
        n.map(|n| self.stmt(n)).unwrap_or_default()
    }

    /// Notes that the lowering of the statement being lowered rests on assumption `a`: the statement is
    /// preceded by `Stmt::Assume(a)`, so a test that runs it is reported as flagged.
    // No lowering-level assumption is registered at the moment; the next one calls this.
    #[allow(dead_code)]
    pub(crate) fn assume(&mut self, a: al_builtins::assumptions::Assumption) {
        if let Some(frame) = self.assumed.last_mut() {
            if !frame.contains(&a) {
                frame.push(a);
            }
        }
    }

    /// `--tdd` (ADR-051 decision 10): a value written to a stub Text or Code field is refused when it is longer than the
    /// length the stub is declared with, as the real length is not known yet.
    pub(crate) fn tdd_fit(&self, target: &SyntaxNode, target_ty: &Ty, value: ir::Expr) -> ir::Expr {
        let Some(d) = self.checked.tdd_decls.as_ref().filter(|d| !d.fields.is_empty()) else {
            return value;
        };
        if !matches!(target_ty, Ty::Text(_) | Ty::Code(_)) {
            return value;
        }
        let Some(Res::Field { table, id }) = self.res(target).cloned() else {
            return value;
        };
        if !d.fields.contains_key(&(table, id)) {
            return value;
        }
        let name = self.members[table]
            .fields
            .iter()
            .find(|f| f.id == id)
            .map(|f| f.name.clone())
            .unwrap_or_default();
        ir::Expr::TddFit {
            max: crate::check::tdd_decl::STUB_TEXT_LENGTH,
            why: crate::check::tdd_decl::stub_field_reason(&name),
            inner: Box::new(value),
        }
    }

    /// `--tdd`: a `Stmt::TddMark` for each stub object a parameter or local of `r` has as its type.
    fn tdd_object_marks(&self, r: &ast::Routine) -> Vec<ir::Stmt> {
        let Some(d) = self.checked.tdd_decls.as_ref() else {
            return Vec::new();
        };
        let tys = r
            .params()
            .filter_map(|p| p.type_ref())
            .chain(r.local_var_decls().filter_map(|v| v.type_ref()))
            .map(|t| Ty::from_type_ref(&t));
        if d.objects.is_empty() {
            return Vec::new();
        }
        let mut out: Vec<String> = Vec::new();
        for t in tys {
            if let Some(sig) = self.tdd_object_sig(&t) {
                if !out.contains(&sig) {
                    out.push(sig);
                }
            }
        }
        out.into_iter().map(ir::Stmt::TddMark).collect()
    }

    /// `--tdd`: the signature of the stub object a variable of type `t` is an instance of, if it is one.
    pub(crate) fn tdd_object_sig(&self, t: &Ty) -> Option<String> {
        let d = self.checked.tdd_decls.as_ref().filter(|d| !d.objects.is_empty())?;
        let (kind, name) = match t {
            Ty::Record { table, .. } => (ObjKind::Table, table),
            Ty::Object(k, n) => (*k, n),
            Ty::Enum(n) => (ObjKind::Enum, n),
            _ => return None,
        };
        match self.ws.resolve_ref(self.file, kind, &[name.to_string()]) {
            Resolution::Found(oi) => d.objects.get(&oi).cloned(),
            _ => None,
        }
    }

    pub(crate) fn stmt(&mut self, n: &SyntaxNode) -> Vec<ir::Stmt> {
        self.tdd_marked.borrow_mut().push(Vec::new());
        self.assumed.push(Vec::new());
        let mark = self.tys.len();
        let mut out = self.stmt_plain(n);
        if self.tys_on && n.kind() != SyntaxKind::Block {
            self.seal_stmts(mark, &out);
        }
        let marks = self.assumed.pop().unwrap_or_default();
        let tdd = self.tdd_marked.borrow_mut().pop().unwrap_or_default();
        if (!marks.is_empty() || !tdd.is_empty()) && !out.is_empty() {
            // After the position marker, which belongs to the statement that follows it.
            let at = usize::from(matches!(out.first(), Some(ir::Stmt::Pos(_))));
            let n_marks = marks.len();
            for (i, a) in marks.into_iter().enumerate() {
                out.insert(at + i, ir::Stmt::Assume(a));
            }
            for (i, s) in tdd.into_iter().enumerate() {
                out.insert(at + n_marks + i, ir::Stmt::TddMark(s));
            }
        }
        out
    }

    /// A call or other expression used as a statement. A try function called as a statement behaves like a normal method
    /// (doc: TryFunction attribute), so its errors propagate.
    /// `continue;` in an object that declares a procedure named Continue calls the procedure; the loop goes on with the
    /// rest of the body (corpus types/PUContinue.al:166-180, Continue_WithProcedureNamedContinue, codeunit 82150 at
    /// :221-272: `123|calls=1`). Only a procedure of the object itself without parameters or return value is pinned.
    fn continue_stmt(&mut self) -> L<ir::Stmt> {
        let found = self.members[self.oi]
            .procedures
            .iter()
            .filter(|p| p.name.eq_ignore_ascii_case("continue"))
            .map(|p| (p.offset, p.declared_in, p.params.is_empty() && p.ret.is_none()))
            .collect::<Vec<_>>();
        match found.as_slice() {
            [] => Ok(ir::Stmt::Continue),
            [(offset, declared_in, true)] if *declared_in == self.oi => {
                let routine = self.routine_at.get(&(self.oi, *offset)).ok_or("procedure not found")?;
                let try_call = self
                    .routine_nodes
                    .get(self.oi)
                    .get(routine)
                    .is_some_and(|r| r.attributes().any(|a| a.name().eq_ignore_ascii_case("tryfunction")));
                if try_call {
                    return Err("`continue;` with a TryFunction named Continue (not pinned)".into());
                }
                self.declared_in_both_proc(self.oi, *offset)?;
                self.calls.push((self.oi, routine));
                Ok(ir::Stmt::Expr(ir::Expr::Call {
                    object: self.oi,
                    routine,
                    instance: None,
                    args: Vec::new(),
                    try_call: false,
                }))
            }
            _ => Err("`continue;` where a procedure named Continue takes parameters, returns a value, is overloaded or comes from another object (not pinned)".into()),
        }
    }

    fn expr_stmt(&mut self, e: &SyntaxNode) -> L<ir::Stmt> {
        self.expr(e).map(|e| match e {
            ir::Expr::Builtin { id, receiver, args, .. } => ir::Stmt::Expr(ir::Expr::Builtin {
                id,
                receiver,
                args,
                ret_used: false,
            }),
            ir::Expr::Call {
                object,
                routine,
                instance,
                args,
                ..
            } => ir::Stmt::Expr(ir::Expr::Call {
                object,
                routine,
                instance,
                args,
                try_call: false,
            }),
            e => ir::Stmt::Expr(e),
        })
    }

    pub(crate) fn stmt_plain(&mut self, n: &SyntaxNode) -> Vec<ir::Stmt> {
        use SyntaxKind::*;
        let kids: Vec<SyntaxNode> = n.children().collect();
        // A statement the parser recovered from an error can lack its parts (an `if` with no condition). The file's
        // tests are not run (the parse error is their reason), but its routines are still lowered: refuse, never index
        // past the parts.
        let (parts, what) = match n.kind() {
            ExprStmt => (1, "a call statement"),
            IfStmt => (1, "an if statement"),
            WhileStmt => (1, "a while statement"),
            RepeatStmt => (1, "a repeat statement"),
            CaseStmt => (1, "a case statement"),
            WithStmt => (1, "a with statement"),
            AssignStmt => (2, "an assignment"),
            ForeachStmt => (2, "a foreach statement"),
            ForStmt => (3, "a for statement"),
            _ => (0, ""),
        };
        let r = match n.kind() {
            Block => return self.stmts_of(n),
            _ if kids.len() < parts => Err(format!("{what} without its parts (the parser did not accept it)")),
            ExprStmt => {
                // A call that passes `CurrFieldNo` to a `var` parameter: the callee works on a local that holds its value, and
                // what the local holds afterwards becomes the record's CurrFieldNo (record/lt-curr-field-no/CurrFieldNoVarParam.al:
                // 103-106, :117; a copy of it is not the property, :128).
                let outer = self.curr_field_args.replace(Vec::new());
                let lowered = self.expr_stmt(&kids[0]);
                let args = std::mem::replace(&mut self.curr_field_args, outer).unwrap_or_default();
                match lowered {
                    Ok(call) if !args.is_empty() => {
                        let mut out: Vec<ir::Stmt> = self.mark(n).into_iter().collect();
                        let builtin = |id: &str, rec: usize, args: Vec<ir::Arg>, used: bool| -> L<ir::Expr> {
                            Ok(ir::Expr::Builtin {
                                id: super::call::builtin_id(id)?,
                                receiver: Some(Box::new(ir::Arg::Ref(ir::Place::Global(rec)))),
                                args,
                                ret_used: used,
                            })
                        };
                        let mut after = Vec::new();
                        for &(tmp, rec) in &args {
                            match (
                                builtin("Record.CurrFieldNo#1", rec, Vec::new(), true),
                                builtin(
                                    "Record.SetCurrFieldNo#1",
                                    rec,
                                    vec![ir::Arg::Value(ir::Expr::Load(ir::Place::Local(tmp)))],
                                    false,
                                ),
                            ) {
                                (Ok(get), Ok(set)) => {
                                    out.push(ir::Stmt::Assign(ir::Place::Local(tmp), get));
                                    after.push(ir::Stmt::Expr(set));
                                }
                                (Err(e), _) | (_, Err(e)) => {
                                    self.reasons.push(e);
                                    return Vec::new();
                                }
                            }
                        }
                        out.push(call);
                        out.extend(after);
                        return out;
                    }
                    other => other,
                }
            }
            AssignStmt => self.assign(n, &kids),
            // See ForeachStmt: in a foreach nested in another loop the shape is refused.
            IfStmt if self.loops.last() == Some(&true) && continue_else(n) => {
                Err("`if ... then continue else ...` in a foreach nested in another loop (microsoft/AL issue 8339, not pinned)".into())
            }
            IfStmt => {
                let else_at = sig_tokens(n)
                    .iter()
                    .find(|t| t.text().eq_ignore_ascii_case("else"))
                    .map(|t| t.text_range().start());
                let cond = self.condition(&kids[0]);
                let (mut then_s, mut else_s) = (Vec::new(), Vec::new());
                for k in kids.iter().skip(1) {
                    let lowered = self.stmt(k);
                    if else_at.is_some_and(|e| k.text_range().start() > e) {
                        else_s.extend(lowered);
                    } else {
                        then_s.extend(lowered);
                    }
                }
                cond.map(|c| ir::Stmt::If(c, then_s, else_s))
            }
            WhileStmt => {
                let cond = self.condition(&kids[0]);
                self.loops.push(false);
                let body = self.stmt_or_empty(kids.get(1));
                self.loops.pop();
                cond.map(|c| ir::Stmt::While(c, body))
            }
            RepeatStmt => {
                let (until, body_nodes) = kids.split_last().expect("repeat has until");
                let mut body = Vec::new();
                self.loops.push(false);
                for b in body_nodes {
                    body.extend(self.stmt(b));
                }
                self.loops.pop();
                self.condition(until).map(|u| ir::Stmt::Repeat(body, u))
            }
            ForStmt => {
                let down = sig_tokens(n).iter().any(|t| t.text().eq_ignore_ascii_case("downto"));
                let var = self.place(&kids[0]);
                let from = self.operand(&kids[1], ir::OperandUse::Value);
                let to = self.operand(&kids[2], ir::OperandUse::Value);
                self.loops.push(false);
                let body = self.stmt_or_empty(kids.get(3));
                self.loops.pop();
                (|| {
                    Ok(ir::Stmt::For {
                        var: var?,
                        from: from?,
                        to: to?,
                        down,
                        body,
                    })
                })()
            }
            ForeachStmt => {
                let var = self.place(&kids[0]);
                let coll = self.expr(&kids[1]);
                // `if ... then continue else ...` as the body of a foreach, or anywhere in a foreach nested in another loop: BC
                // skips the else branch in the nested shape (corpus types/PUContinue.al:182-218, Continue_ForeachInFor_IfContinueElse:
                // `seen` is empty; microsoft/AL issue 8339), which no rule explains yet. Only a block in a foreach that is not
                // nested is pinned to behave as continue does elsewhere (:206-212, `A.C.`). Refused until a probe pins it.
                // `loops` notes per enclosing loop whether it is such a nested foreach.
                let nested = !self.loops.is_empty();
                let direct = kids.get(2).is_some_and(continue_else);
                self.loops.push(nested);
                let body = self.stmt_or_empty(kids.get(2));
                self.loops.pop();
                (|| {
                    if direct {
                        return Err("`if ... then continue else ...` as the body of a foreach (microsoft/AL issue 8339, not pinned)".into());
                    }
                    Ok(ir::Stmt::Foreach {
                        var: var?,
                        collection: coll?,
                        body,
                    })
                })()
            }
            CaseStmt => self.case(&kids),
            // `with Rec do Stmt`: the names the checker took from the record read and write that record
            // (corpus events/lt-global-triggers-statements/P114WithStatement.al,
            // events/lt-with-statement-scopes/WithStatementNested.al). The record is read as a place each time.
            WithStmt => {
                let start = usize::from(n.text_range().start());
                let place = match self.ty_ref(&kids[0]) {
                    Ty::Record { .. } => self.place(&kids[0]),
                    other => Err(format!("with on a value of type {other} is not lowered (only records)")),
                };
                match place {
                    Ok(p) => {
                        self.with_places.insert(start, p);
                        let body = self.stmt_or_empty(kids.get(1));
                        self.with_places.remove(&start);
                        return body;
                    }
                    Err(e) => Err(e),
                }
            }
            ExitStmt => match kids.first() {
                None => Ok(ir::Stmt::Exit(None)),
                Some(e) => self.expr(e).and_then(|v| {
                    if let Some(decl) = self.ret_decl.clone() {
                        self.refuse_textconst_into_variant(&decl, e)?;
                    }
                    // A DotNet result must fit the return type exactly (ADR-014). An interface
                    // result takes what an interface variable takes: a codeunit that implements
                    // it, an enum, or an interface it extends (codeunit/TestInterfaceRuntimeContracts.al:
                    // Interface_VarParameterAndReturnValue_CarryTheReference, `exit(Impl)`).
                    if let Some(decl @ Ty::Object(ObjKind::Interface, _)) = self.ret_decl.clone() {
                        return self
                            .convert_to(&decl, self.ty_ref(e), v)
                            .map(|v| ir::Stmt::Exit(Some(v)));
                    }
                    Ok(ir::Stmt::Exit(Some(match (self.ty_ref(e), &self.ret_ty) {
                        (Ty::DotNet(a), Some(rt)) if a.is_empty() && !matches!(rt, RtType::DotNet(_)) => {
                            ir::Expr::DotNetCoerce(rt.clone(), Box::new(v))
                        }
                        _ => v,
                    })))
                }),
            },
            BreakStmt => Ok(ir::Stmt::Break),
            ContinueStmt => self.continue_stmt(),
            AssertErrorStmt => Ok(ir::Stmt::AssertError(self.stmt_or_empty(kids.first()))),
            other => Err(format!("statement {other:?} is not lowered yet")),
        };
        match r {
            Ok(s) => match self.mark(n) {
                Some(m) => vec![m, s],
                None => vec![s],
            },
            // A statement that uses a variable whose attribute is not supported refuses when it runs, not the whole
            // routine: Base Application declares such variables in routines whose other paths run (`var_attr_refusal`).
            Err(e) if self.run_refusals.contains(&e) => {
                let s = ir::Stmt::Expr(ir::Expr::Refused(e));
                match self.mark(n) {
                    Some(m) => vec![m, s],
                    None => vec![s],
                }
            }
            Err(e) => {
                self.reasons.push(e);
                Vec::new()
            }
        }
    }

    pub(crate) fn assign(&mut self, n: &SyntaxNode, kids: &[SyntaxNode]) -> L<ir::Stmt> {
        // `CurrFieldNo := FieldNo(X)` in table code sets the field number of the implicit Rec. Base Application's
        // `SelltoContactLookup` does it before it validates a field. The corpus pins reading CurrFieldNo
        // (handlers/TestPageCurrFieldNo_Tests.al); the assignment is not pinned (docs/probes-needed.md).
        if kids[0].kind() == SyntaxKind::NameRef
            && matches!(self.res(&kids[0]), Some(Res::Global(g)) if g == "currfieldno")
            && !self.globals.contains_key("currfieldno")
        {
            if !sig_tokens(n).iter().any(|t| t.kind() == SyntaxKind::Assign) {
                return Err("a compound assignment to CurrFieldNo is not lowered (not pinned)".into());
            }
            let rec = self
                .rec_slot
                .ok_or("CurrFieldNo outside code that has an implicit Rec")?;
            let value = self.expr(&kids[1])?;
            let value = self.convert_to(&Ty::Integer, self.ty_ref(&kids[1]), value)?;
            return Ok(ir::Stmt::Expr(ir::Expr::Builtin {
                id: ir::BuiltinId::of("Record.SetCurrFieldNo#1").ok_or("Record.SetCurrFieldNo#1 has no builtin id")?,
                receiver: Some(Box::new(ir::Arg::Ref(ir::Place::Global(rec)))),
                args: vec![ir::Arg::Value(value)],
                ret_used: false,
            }));
        }
        // `X.Property := value` on a DotNet value (ADR-014).
        if let Some(Res::DotNetMember(name)) = self.res(&kids[0]).cloned() {
            if kids[0].kind() == SyntaxKind::MemberExpr {
                if sig_tokens(n).iter().any(|t| {
                    matches!(
                        t.kind(),
                        SyntaxKind::PlusAssign
                            | SyntaxKind::MinusAssign
                            | SyntaxKind::StarAssign
                            | SyntaxKind::SlashAssign
                    )
                }) {
                    return Err("compound assignment to a DotNet property is not lowered yet".into());
                }
                let value = self.dotnet_arg(&kids[1])?;
                return self
                    .dotnet_member(&kids[0], &kids[0], &name, ir::DotNetOp::Set(name.clone()), vec![value])
                    .map(ir::Stmt::Expr);
            }
        }
        // `Page.Control.SetValue := x` is the call `SetValue(x)` (Microsoft's Tests-ERM write it so, ERM Pmt. Tolerance VAT
        // Appln.): a member without parentheses on a test page control that the checker leaves unresolved.
        if kids[0].kind() == SyntaxKind::MemberExpr
            && self.res(&kids[0]).is_none()
            && matches!(kids[0].children().next().map(|b| self.ty_ref(&b)), Some(Ty::Builtin(t)) if t.as_ref() == "TestField")
            && member_ident(&kids[0]).is_some_and(|m| m.eq_ignore_ascii_case("SetValue"))
            && sig_tokens(n).iter().any(|t| t.kind() == SyntaxKind::Assign)
        {
            let recv = kids[0].children().next().ok_or("member base")?;
            let receiver = match self.place(&recv) {
                Ok(p) => ir::Arg::Ref(p),
                Err(_) => ir::Arg::Value(self.expr(&recv)?),
            };
            let rhs = self.expr(&kids[1])?;
            return Ok(ir::Stmt::Expr(ir::Expr::Builtin {
                id: super::call::builtin_id("TestField.SetValue#1")?,
                receiver: Some(Box::new(receiver)),
                args: vec![ir::Arg::Value(rhs)],
                ret_used: false,
            }));
        }
        // `Part.PAGE.Proc := value`: the checker picked the one-parameter procedure and recorded it on the target.
        if kids[0].kind() == SyntaxKind::MemberExpr
            && matches!(self.res(&kids[0]), Some(Res::Proc { .. }))
            && sig_tokens(n).iter().any(|t| t.kind() == SyntaxKind::Assign)
        {
            return self.call(&kids[0], &kids[0], &kids[1..2]).map(ir::Stmt::Expr);
        }
        // `Proc := value` calls a procedure of this object that takes one value parameter (Base Application's Service
        // Line writes `SetHideWarrantyWarning := true`; corpus codeunit/lt-procedure-assignment/ProcedureAssignmentOneParameter.al). The checker leaves the
        // name unresolved (it looks for a call with no arguments), so the procedure is found here by its name.
        if kids[0].kind() == SyntaxKind::NameRef
            && self.res(&kids[0]).is_none()
            && sig_tokens(n).iter().any(|t| t.kind() == SyntaxKind::Assign)
        {
            let name = kids[0].text().to_string();
            let name = name.trim().trim_matches('"');
            let found = self.members[self.oi]
                .procedures
                .iter()
                .filter(|p| p.name.eq_ignore_ascii_case(name))
                .map(|p| (p.offset, p.params.clone()))
                .collect::<Vec<_>>();
            if let [(offset, params)] = found.as_slice() {
                if params.len() == 1 && !params[0].by_var {
                    let routine = self.routine_at.get(&(self.oi, *offset)).ok_or("procedure not found")?;
                    self.declared_in_both_proc(self.oi, *offset)?;
                    let value = self.expr(&kids[1])?;
                    let value = self.convert_to(&params[0].ty, self.ty_ref(&kids[1]), value)?;
                    self.calls.push((self.oi, routine));
                    let try_call = self
                        .routine_nodes
                        .get(self.oi)
                        .get(routine)
                        .is_some_and(|r| r.attributes().any(|a| a.name().eq_ignore_ascii_case("tryfunction")));
                    return Ok(ir::Stmt::Expr(ir::Expr::Call {
                        object: self.oi,
                        routine,
                        instance: None,
                        args: vec![ir::Arg::Value(value)],
                        try_call,
                    }));
                }
            }
        }
        // `FieldRef.Value := x` is the setter form of the `Value` method
        // (corpus recordref/TestRecordRefCRUD.al:20-23): not a place, a builtin call.
        if let Some(Res::Builtin(id)) = self.res(&kids[0]).cloned() {
            let sig = self
                .catalog
                .by_id(&id)
                .filter(|s| is_property_setter(&s.receiver, &s.name))
                .cloned();
            // The member without arguments resolves to the getter; where the setter is an overload of its own
            // (`XmlReadOptions.PreserveWhitespace#2`), the assignment calls that one.
            let (id, sig) = match sig.filter(|s| s.params.is_empty()).and_then(|s| {
                self.catalog
                    .overloads(&s.receiver, &s.name)
                    .into_iter()
                    .find(|o| o.params.len() == 1)
                    .cloned()
            }) {
                Some(setter) => (self.catalog.id(&setter), Some(setter)),
                None => (
                    id.clone(),
                    self.catalog
                        .by_id(&id)
                        .filter(|s| is_property_setter(&s.receiver, &s.name))
                        .cloned(),
                ),
            };
            let member = kids[0].kind() == SyntaxKind::MemberExpr;
            // Without a receiver the member is a global function (`WorkDate := d`) or a Record
            // member of the implicit Rec of table, page and dataitem code (`FilterGroup := 2`).
            let implicit = !member
                && sig.as_ref().is_some_and(|s| {
                    !s.instance
                        || (s.receiver == "Record" && (self.rec_slot.is_some() || self.supplied_by_with(&kids[0])))
                });
            if let Some(sig) = sig.filter(|_| member || implicit) {
                // A test page control's setter form is run by the test client, not a native builtin.
                if al_builtins::native::lookup(&id).is_none() && !id.starts_with("TestField.") {
                    return Err(match al_builtins::native::refusal(&id) {
                        Some(why) => format!("builtin {id}: {why} (docs/probes-needed.md)"),
                        None => format!("builtin {id} is not implemented"),
                    });
                }
                let compound = sig_tokens(n).iter().any(|t| {
                    matches!(
                        t.kind(),
                        SyntaxKind::PlusAssign
                            | SyntaxKind::MinusAssign
                            | SyntaxKind::StarAssign
                            | SyntaxKind::SlashAssign
                    )
                });
                if compound {
                    return Err("compound assignment to a setter-form builtin is not lowered yet".into());
                }
                let receiver = if member {
                    let recv = kids[0].children().next().ok_or("member base")?;
                    // `CurrReport` is the running report: the VM takes it when a report builtin has
                    // no receiver (as for the call form). A static member (`System.WorkDate`)
                    // has none.
                    let current_report = sig.receiver == "Report"
                        && matches!(
                            self.ws.objects[self.oi].kind,
                            ObjKind::Report | ObjKind::ReportExtension
                        )
                        && recv.text().to_string().trim().eq_ignore_ascii_case("CurrReport");
                    // `RequestOptionsPage` is the request page of the running report: no variable to take.
                    let request_page = sig.receiver == "RequestPage"
                        && matches!(
                            self.ws.objects[self.oi].kind,
                            ObjKind::Report | ObjKind::ReportExtension
                        )
                        && recv
                            .text()
                            .to_string()
                            .trim()
                            .eq_ignore_ascii_case("RequestOptionsPage");
                    if !sig.instance || current_report || request_page {
                        None
                    } else {
                        Some(Box::new(match self.place(&recv) {
                            Ok(p) => ir::Arg::Ref(p),
                            Err(_) => ir::Arg::Value(self.expr(&recv)?),
                        }))
                    }
                } else if sig.instance {
                    Some(Box::new(ir::Arg::Ref(self.implicit_rec(&kids[0])?)))
                } else {
                    None
                };
                let rhs = self.expr(&kids[1])?;
                return Ok(ir::Stmt::Expr(ir::Expr::Builtin {
                    id: super::call::builtin_id(&id)?,
                    receiver,
                    args: vec![ir::Arg::Value(rhs)],
                    ret_used: false,
                }));
            }
        }
        // `GlobalLanguage := Id` is the other spelling of `GlobalLanguage(Id)` (System Application's
        // "Reten. Pol. Telemetry Impl." writes it so; the setter is pinned by
        // session/TestSessionSettings.al:534): a builtin call, not a place.
        if let Some(Res::Builtin(id)) = self.res(&kids[0]).cloned() {
            if id == "System.GlobalLanguage#1" && kids[0].kind() != SyntaxKind::MemberExpr {
                if sig_tokens(n).iter().any(|t| {
                    t.kind() != SyntaxKind::Assign
                        && matches!(
                            t.kind(),
                            SyntaxKind::PlusAssign
                                | SyntaxKind::MinusAssign
                                | SyntaxKind::StarAssign
                                | SyntaxKind::SlashAssign
                        )
                }) {
                    return Err("compound assignment to GlobalLanguage is not lowered yet".into());
                }
                let rhs = self.expr(&kids[1])?;
                return Ok(ir::Stmt::Expr(ir::Expr::Builtin {
                    id: super::call::builtin_id(&id)?,
                    receiver: None,
                    args: vec![ir::Arg::Value(rhs)],
                    ret_used: false,
                }));
            }
        }
        let place = self.place(&kids[0])?;
        let target_ty = self.ty_of(&kids[0]);
        let rhs = self.expr(&kids[1])?;
        let rhs = self.tdd_fit(&kids[0], &target_ty, rhs);
        let op = sig_tokens(n).into_iter().find(|t| {
            matches!(
                t.kind(),
                SyntaxKind::Assign
                    | SyntaxKind::PlusAssign
                    | SyntaxKind::MinusAssign
                    | SyntaxKind::StarAssign
                    | SyntaxKind::SlashAssign
            )
        });
        let op_kind = op.map(|t| t.kind());
        let value = match op_kind {
            Some(SyntaxKind::PlusAssign) => {
                ir::Expr::Binary(ir::BinOp::Add, Box::new(ir::Expr::Load(place.clone())), Box::new(rhs))
            }
            Some(SyntaxKind::MinusAssign) => {
                ir::Expr::Binary(ir::BinOp::Sub, Box::new(ir::Expr::Load(place.clone())), Box::new(rhs))
            }
            Some(SyntaxKind::StarAssign) => {
                ir::Expr::Binary(ir::BinOp::Mul, Box::new(ir::Expr::Load(place.clone())), Box::new(rhs))
            }
            Some(SyntaxKind::SlashAssign) => {
                ir::Expr::Binary(ir::BinOp::Div, Box::new(ir::Expr::Load(place.clone())), Box::new(rhs))
            }
            _ => rhs,
        };
        // A `FieldRef.Value()` into a record's Text or Code field stays unconverted here: the field write turns a number into its Format text.
        let variant_into_text_field = matches!(place, ir::Place::Field(..))
            && matches!(target_ty, Ty::Text(_) | Ty::Code(_))
            // `FieldRef.Value()` is typed Unknown by the checker: its number goes into the field as its Format text
            // (types/lt-variant-to-text-fields/VariantFieldRefBigIntToText.al:TextField_FromFieldRefValue_Numbers), where a Variant holding
            // a number raises TypeConversion (VariantNumberToField.al:TextField_FromVariantInteger_Result and siblings).
            && matches!(self.ty_ref(&kids[1]), Ty::Unknown)
            && matches!(op_kind, Some(SyntaxKind::Assign) | None);
        // A Variant holding an Integer or a Date assigned to a Text variable raises TypeConversion (corpus
        // types/lt-variant/VaTCompare.al:Concat_After_Conversion, af117 and af121); the conversion into a field, a parameter
        // or a returned value is another path (VariantNumberToField.al; assumption date-to-text-is-format).
        let variant_into_text_variable = matches!(
            place,
            ir::Place::Local(_) | ir::Place::Global(_) | ir::Place::BaseGlobal(_)
        ) && matches!(target_ty, Ty::Text(_) | Ty::Code(_))
            && matches!(self.ty_ref(&kids[1]), Ty::Variant)
            && matches!(op_kind, Some(SyntaxKind::Assign) | None)
            && matches!(&value, ir::Expr::Load(p) if matches!(p, ir::Place::Local(_) | ir::Place::Global(_) | ir::Place::BaseGlobal(_)));
        if variant_into_text_variable {
            let ir::Expr::Load(src_place) = value.clone() else {
                unreachable!()
            };
            let is = |id: &str| -> L<ir::Expr> {
                Ok(ir::Expr::Builtin {
                    id: super::call::builtin_id(id)?,
                    receiver: Some(Box::new(ir::Arg::Ref(src_place.clone()))),
                    args: Vec::new(),
                    ret_used: true,
                })
            };
            // A Date, Time or DateTime in a Variant raises into a Text and a Code variable alike
            // (report/lt-requestpage-savevalues-date-text-setvalue/P147DateToText.al:297-399, QO_*Variant_EachConversion; the
            // NavCode text: types/lt-variant-to-text-fields/VariantNumberToField.al). Into a Code variable nothing else is settled.
            let or = |a: ir::Expr, b: ir::Expr| ir::Expr::Binary(ir::BinOp::Or, Box::new(a), Box::new(b));
            let dates = or(
                or(is("Variant.IsDate#1")?, is("Variant.IsTime#1")?),
                is("Variant.IsDateTime#1")?,
            );
            let (cond, kind) = if matches!(target_ty, Ty::Code(_)) {
                (dates, "NavCode")
            } else {
                // An Enum or Option in a Variant raises too (VariantEnumOptionToText.al: `text variable`).
                let cond = or(or(is("Variant.IsInteger#1")?, dates), is("Variant.IsOption#1")?);
                (cond, "NavText")
            };
            let converted = self.convert_to(&target_ty, self.ty_ref(&kids[1]), value)?;
            let guarded = ir::Expr::Ternary(
                Box::new(cond),
                Box::new(ir::Expr::Fail {
                    message: format!(
                        "The expression Microsoft.Dynamics.Nav.Runtime.NavIndirectValue cannot be type-converted to a Microsoft.Dynamics.Nav.Runtime.{kind} value."
                    ),
                    code: "TypeConversion".into(),
                }),
                Box::new(converted),
            );
            return Ok(ir::Stmt::Assign(place, guarded));
        }
        // A Variant variable holding an Integer, a BigInteger or a Decimal assigned to a Text or Code field raises the same
        // TypeConversion as into a variable (types/lt-variant-to-text-fields/VariantNumberToField.al:TextField_FromVariantInteger_Result,
        // _BigInteger_, _Decimal_, CodeField_FromVariantNumbers_Result, TextField_FromVariantNumber_TooLong); a call result such as
        // `FieldRef.Value()` converts to its Format text (VariantFieldRefBigIntToText.al:TextField_FromFieldRefValue_Numbers).
        // Other contents of the Variant keep the unconverted path.
        if matches!(place, ir::Place::Field(..))
            && matches!(target_ty, Ty::Text(_) | Ty::Code(_))
            && matches!(op_kind, Some(SyntaxKind::Assign) | None)
            && matches!(self.ty_ref(&kids[1]), Ty::Variant)
            && matches!(&value, ir::Expr::Load(p) if matches!(p, ir::Place::Local(_) | ir::Place::Global(_) | ir::Place::BaseGlobal(_)))
        {
            let ir::Expr::Load(src_place) = value.clone() else {
                unreachable!()
            };
            let is = |id: &str| -> L<ir::Expr> {
                Ok(ir::Expr::Builtin {
                    id: super::call::builtin_id(id)?,
                    receiver: Some(Box::new(ir::Arg::Ref(src_place.clone()))),
                    args: Vec::new(),
                    ret_used: true,
                })
            };
            let or = |a: ir::Expr, b: ir::Expr| ir::Expr::Binary(ir::BinOp::Or, Box::new(a), Box::new(b));
            let cond = or(
                or(
                    or(is("Variant.IsInteger#1")?, is("Variant.IsBigInteger#1")?),
                    is("Variant.IsDecimal#1")?,
                ),
                // An Enum or Option in a Variant raises too (types/lt-variant-to-text-fields/VariantEnumOptionToText.al: `text field`, `code field`).
                // A Date as well, as into a Text parameter (report/lt-requestpage-savevalues-date-text-setvalue/P147DateToText.al:89-105).
                or(is("Variant.IsOption#1")?, is("Variant.IsDate#1")?),
            );
            let kind = if matches!(target_ty, Ty::Code(_)) {
                "NavCode"
            } else {
                "NavText"
            };
            let guarded = ir::Expr::Ternary(
                Box::new(cond),
                Box::new(ir::Expr::Fail {
                    message: format!(
                        "The expression Microsoft.Dynamics.Nav.Runtime.NavIndirectValue cannot be type-converted to a Microsoft.Dynamics.Nav.Runtime.{kind} value."
                    ),
                    code: "TypeConversion".into(),
                }),
                Box::new(value),
            );
            return Ok(ir::Stmt::Assign(place, guarded));
        }
        // `B := A` of two interface variables raises when A holds no implementation (codeunit/lt-enums-interfaces/OjT71047.al:
        // 250-251). A Variant into an Integer raises TypeConversion:Overflow for a BigInteger or Duration beyond its range
        // (types/lt-variant/VaTConvert.al:183-184; types/lt-filter-enum-relation-duration-guid/D_Duration.al:226).
        if matches!(op_kind, Some(SyntaxKind::Assign) | None) && !variant_into_text_field {
            let checked = match (&target_ty, self.ty_ref(&kids[1])) {
                (Ty::Object(ObjKind::Interface, _), Ty::Object(ObjKind::Interface, _)) => {
                    Some("Interface.AssertAssigned#1")
                }
                (Ty::Integer, Ty::Variant) => Some("Variant.IntoInteger#1"),
                // A Variant into an Option variable or field: a BigInteger beyond Integer raises TypeConversion:Overflow
                // (types/lt-bigint-enum-option/BigIntToEnumOption.al:353-383 QO_Variant_ToOptionVariable,
                // QO_Variant_ToOptionField_Assign); the value then converts as any value does.
                (Ty::Option(_), Ty::Variant) => Some("Variant.IntoOption#1"),
                _ => None,
            };
            if let Some(id) = checked {
                let value = match id {
                    "Interface.AssertAssigned#1" => self.convert_to(&target_ty, self.ty_ref(&kids[1]), value)?,
                    "Variant.IntoOption#1" => {
                        let checked = ir::Expr::Builtin {
                            id: super::call::builtin_id(id)?,
                            receiver: None,
                            args: vec![ir::Arg::Value(value)],
                            ret_used: true,
                        };
                        let value = self.convert_to(&target_ty, &Ty::Variant, checked)?;
                        return Ok(ir::Stmt::Assign(place, value));
                    }
                    _ => value,
                };
                return Ok(ir::Stmt::Assign(
                    place,
                    ir::Expr::Builtin {
                        id: super::call::builtin_id(id)?,
                        receiver: None,
                        args: vec![ir::Arg::Value(value)],
                        ret_used: true,
                    },
                ));
            }
        }
        self.refuse_textconst_into_variant(&target_ty, &kids[1])?;
        // A Byte assigned to a character of a Text or Code goes in unconverted: the runtime maps a byte from 0x80 up to the
        // character Format shows for it (al-language-onprem/types/ByteFormat.al:13-31, `Interp::write_char`).
        let byte_into_text_char = matches!(op_kind, Some(SyntaxKind::Assign) | None)
            && matches!(self.ty_ref(&kids[1]), Ty::Byte)
            && kids[0].kind() == SyntaxKind::IndexExpr
            && kids[0]
                .children()
                .next()
                .is_some_and(|b| matches!(self.ty_ref(&b), Ty::Text(_) | Ty::Code(_)));
        let value = match op_kind {
            _ if variant_into_text_field || byte_into_text_char => value,
            Some(SyntaxKind::Assign) | None => self.convert_to(&target_ty, self.ty_ref(&kids[1]), value)?,
            Some(_) => ir::Expr::Convert(self.rt(&target_ty)?, Box::new(value)),
        };
        if let Some((l, flag)) = self.qcp_ret {
            if place == ir::Place::Local(l) {
                // An assignment to the named return value of an OnQueryClosePage notes that it has one.
                return Ok(ir::Stmt::If(
                    ir::Expr::Const(ir::Const::Boolean(true)),
                    vec![
                        ir::Stmt::Assign(place, value),
                        ir::Stmt::Assign(ir::Place::Local(flag), ir::Expr::Const(ir::Const::Boolean(true))),
                    ],
                    Vec::new(),
                ));
            }
        }
        Ok(ir::Stmt::Assign(place, value))
    }

    pub(crate) fn case(&mut self, kids: &[SyntaxNode]) -> L<ir::Stmt> {
        let scrutinee = self.operand(&kids[0], ir::OperandUse::Value)?;
        // A text literal label of a Code selector is converted to the selector's Code: `'abc'` matches the Code `ABC` before the
        // label `'ABC'` does, and a label longer than the Code raises StringLengthExceeded with its text in upper case
        // (types/TestCaseContracts.al:441-466, BN_Case_CodeSelectorAndLabels). A Text selector compares by case (:445-448).
        // A Text variable label and the bounds of a range (text literals or Text variables) are converted the same way
        // (types/TestCaseContracts.al:509-545, PYC_Case_CodeSelector_VariableAndRangeLabels). A Code label whose length can
        // exceed the selector's is not pinned and is refused; other labels keep the plain comparison.
        let code_selector = match self.ty_of(&kids[0]) {
            t @ Ty::Code(_) => Some(t),
            _ => None,
        };
        let option_selector = matches!(self.ty_of(&kids[0]), Ty::Option(_));
        let mut branches = Vec::new();
        let mut otherwise = Vec::new();
        for k in &kids[1..] {
            match k.kind() {
                SyntaxKind::CaseBranch => {
                    let parts: Vec<SyntaxNode> = k.children().collect();
                    let mut labels = Vec::new();
                    let mut body = Vec::new();
                    for p in &parts {
                        if is_stmt(p.kind()) {
                            body.extend(self.stmt(p));
                        } else if let Some(code) = &code_selector {
                            labels.push(self.code_case_label(code, p)?);
                        } else if option_selector && matches!(self.ty_of(p), Ty::Decimal) {
                            labels.push(self.option_case_decimal_label(p)?);
                        } else {
                            labels.push(self.set_elem(p)?);
                        }
                    }
                    branches.push((labels, body));
                }
                SyntaxKind::CaseElse => otherwise = self.stmts_of(k),
                _ => {}
            }
        }
        // `case <Variant> of <label>:` raises when the first label is compared, whatever the Variant holds and whatever the label is
        // (types/lt-variant-unassigned/VariantUnassignedEnum.al: Unassigned_Variant_CaseMatches, EnumZero_, EnumOne_, IntegerZero_,
        // ClearedVariant_CaseMatches, Unassigned_Variant_CaseMatches_InsideProcedure). The scrutinee is evaluated first. The corpus asserts the text only (GetLastErrorText),
        // so the error code is left empty.
        if matches!(self.ty_of(&kids[0]), Ty::Variant) && branches.iter().any(|(labels, _)| !labels.is_empty()) {
            return Ok(ir::Stmt::Case {
                scrutinee,
                branches: vec![(
                    vec![ir::SetElem::One(ir::Expr::Fail {
                        message: "Unable to compare operands of type NavVariant with NavVariant.".into(),
                        code: String::new(),
                    })],
                    Vec::new(),
                )],
                otherwise: Vec::new(),
            });
        }
        Ok(ir::Stmt::Case {
            scrutinee,
            branches,
            otherwise,
        })
    }

    /// A Decimal label of a `case` with an Option selector: a Decimal variable that holds a whole number compares with the
    /// ordinal (`case O of D:` with `D = 1` matches `O::Quantity`, types/PROptionInteger.al:308-322
    /// QS_Case_OptionSelector_DecimalVariableLabel). A fraction is not pinned (the corpus notes an overflow error from the
    /// Linux container for `D = 1.5`, :319-320) and is refused when it runs, and so is a value outside Integer. A label that is
    /// not a plain name is refused (it would be evaluated twice); a Decimal literal does not compile (AL0398).
    fn option_case_decimal_label(&mut self, n: &SyntaxNode) -> L<ir::SetElem> {
        let e = self.expr(n)?;
        if !matches!(e, ir::Expr::Load(_)) {
            return Err("a case label of Decimal type other than a variable on an Option selector (not pinned)".into());
        }
        let bin = |op, a: ir::Expr, b: ir::Expr| ir::Expr::Binary(op, Box::new(a), Box::new(b));
        let int = |i: i32| ir::Expr::Const(ir::Const::Integer(i));
        let whole = bin(ir::BinOp::Eq, bin(ir::BinOp::Mod, e.clone(), int(1)), int(0));
        let in_range = bin(
            ir::BinOp::And,
            bin(ir::BinOp::Ge, e.clone(), int(i32::MIN)),
            bin(ir::BinOp::Le, e.clone(), int(i32::MAX)),
        );
        Ok(ir::SetElem::One(ir::Expr::Ternary(
            Box::new(bin(ir::BinOp::And, whole, in_range)),
            Box::new(e),
            Box::new(ir::Expr::Refused(
                "a case label of an Option holding a Decimal with a fraction or outside Integer (not pinned)".into(),
            )),
        )))
    }

    /// One label of a `case` with a Code selector (see `case`).
    fn code_case_label(&mut self, code: &Ty, n: &SyntaxNode) -> L<ir::SetElem> {
        let bound = |this: &mut Self, c: &SyntaxNode| -> L<ir::Expr> {
            let e = this.expr(c)?;
            if super::expr::is_string_literal(c) {
                return this.convert_to(code, &Ty::Text(None), e);
            }
            match (this.ty_of(c), code) {
                (t @ Ty::Text(_), _) => this.convert_to(code, &t, e),
                (Ty::Code(Some(n)), Ty::Code(Some(m))) if n <= *m => Ok(e),
                (Ty::Code(_), Ty::Code(None)) => Ok(e),
                (t @ Ty::Code(_), _) => Err(format!(
                    "a case label of type {t} with a selector of type {code}: whether the label is converted to the selector's length is not pinned"
                )),
                _ => Ok(e),
            }
        };
        if n.kind() != SyntaxKind::RangeExpr {
            return Ok(ir::SetElem::One(bound(self, n)?));
        }
        let dotdot = sig_tokens(n)
            .into_iter()
            .find(|t| t.kind() == SyntaxKind::DotDot)
            .expect("range has ..")
            .text_range()
            .start();
        let mut lo = None;
        let mut hi = None;
        for c in n.children() {
            let e = bound(self, &c)?;
            if c.text_range().start() < dotdot {
                lo = Some(e);
            } else {
                hi = Some(e);
            }
        }
        Ok(ir::SetElem::Range(lo, hi))
    }

    pub(crate) fn set_elem(&mut self, n: &SyntaxNode) -> L<ir::SetElem> {
        if n.kind() != SyntaxKind::RangeExpr {
            return Ok(ir::SetElem::One(self.expr(n)?));
        }
        let dotdot = sig_tokens(n)
            .into_iter()
            .find(|t| t.kind() == SyntaxKind::DotDot)
            .expect("range has ..")
            .text_range()
            .start();
        let mut lo = None;
        let mut hi = None;
        for c in n.children() {
            let e = self.expr(&c)?;
            if c.text_range().start() < dotdot {
                lo = Some(e);
            } else {
                hi = Some(e);
            }
        }
        Ok(ir::SetElem::Range(lo, hi))
    }

    /// The argument for a `var` parameter: the variable it names. Base Application passes the implicit
    /// `CurrFieldNo` to event publishers that declare `var CallingFieldNo: Integer` (`Sales Header`,
    /// `OnBeforeShouldSearchForCustomerByName`). `CurrFieldNo` is no variable: in a call statement the callee gets a
    /// local that holds its value, and what it assigns comes back to `CurrFieldNo` after the call (record/lt-curr-field-no/
    /// CurrFieldNoVarParam.al:103-106, :117). In a call inside an expression it is refused.
    pub(crate) fn by_var_arg(&mut self, a: &SyntaxNode) -> L<ir::Arg> {
        // `this` in a report, passed to a `var` parameter of the report's type: the parameter holds the running
        // report (see `expr` for the assumption).
        if a.kind() == SyntaxKind::NameRef
            && crate::check::is_this_keyword(a)
            && self.ws.objects[self.oi].kind == ObjKind::Report
            && !self.globals.contains_key("this")
        {
            return Ok(ir::Arg::Value(self.expr(a)?));
        }
        match self.place(a) {
            Ok(p) => Ok(ir::Arg::Ref(p)),
            Err(_)
                if a.kind() == SyntaxKind::NameRef
                    && a.text().to_string().trim().eq_ignore_ascii_case("CurrFieldNo") =>
            {
                let rec = self
                    .rec_slot
                    .ok_or("CurrFieldNo outside code that has an implicit Rec")?;
                let Some(args) = self.curr_field_args.as_mut() else {
                    return Err(
                        "CurrFieldNo passed to a var parameter of a call inside an expression is not lowered (the write-back is pinned for a call statement only)"
                            .into(),
                    );
                };
                let tmp = self.hidden_base + self.hidden_locals.len();
                args.push((tmp, rec));
                self.hidden_locals.push(ir::Slot {
                    name: "__currfieldno".into(),
                    ty: RtType::Integer,
                });
                Ok(ir::Arg::Ref(ir::Place::Local(tmp)))
            }
            Err(e) => Err(e),
        }
    }

    /// A local record declared with `[SecurityFiltering(x)]` starts with x: `Rec.SecurityFiltering()` answers it
    /// (al-language/lt-ac/ACAttributes.al:176-179, :187-190) and, with no security filter in force, it reads the same
    /// rows (:184-186). The routine starts by setting it. What `Clear`, `ClearAll` or an assignment of the whole
    /// variable leaves is not pinned: for a variable the routine names that way (`cleared_or_assigned`), and on anything
    /// but a record, the read of the value stays refused (`security_filtering_read_refusal`).
    fn declared_security_filtering(
        &mut self,
        declared: &[(String, usize, Result<i32, String>)],
        locals: &[ir::Slot],
    ) -> Vec<ir::Stmt> {
        let mut out = Vec::new();
        let id = match super::call::builtin_id("Record.SecurityFiltering#1") {
            Ok(id) => id,
            Err(e) => {
                self.reasons.push(e);
                return out;
            }
        };
        let members: Vec<String> = al_builtins::sysenum::option_members("SecurityFilter")
            .expect("known")
            .iter()
            .map(|m| m.to_string())
            .collect();
        for (name, slot, member) in declared {
            let member = match member {
                Ok(m) => *m,
                Err(e) => {
                    self.reasons.push(format!("`{name}`: {e}"));
                    continue;
                }
            };
            debug_assert!(matches!(locals[*slot].ty, RtType::Record(..)));
            out.push(ir::Stmt::Expr(ir::Expr::Builtin {
                id,
                receiver: Some(Box::new(ir::Arg::Ref(ir::Place::Local(*slot)))),
                args: vec![ir::Arg::Value(ir::Expr::Const(ir::Const::Option(
                    member,
                    members.clone(),
                )))],
                ret_used: false,
            }));
        }
        out
    }

    /// Why a use of the variable `name` is refused for an attribute of its declaration: `[RunOnClient]` (BC raises
    /// DotNetFromAlNotSupported, al-language-onprem-dotnet/lt-ac/AcDnAttributes.al:145-146) and `[WithEvents]` (the
    /// `trigger Variable::Event` handlers run when the object raises the event, AcDnAttributes.al:50, :86) are not
    /// implemented, and running the variable without them would be silently wrong.
    pub(crate) fn var_attr_refusal(&self, name: &str, local: bool) -> Option<String> {
        let attrs = if local {
            self.local_var_attrs.get(name)
        } else {
            self.global_var_attrs.get(name)
        }?;
        attrs.iter().find_map(|a| match a.as_str() {
            "runonclient" => Some(format!(
                "variable `{name}` has the attribute RunOnClient, which is not supported"
            )),
            "withevents" => Some(format!(
                "variable `{name}` has the attribute WithEvents: its event triggers are not supported"
            )),
            _ => None,
        })
    }

    pub(crate) fn place(&mut self, n: &SyntaxNode) -> L<ir::Place> {
        match n.kind() {
            SyntaxKind::ParenExpr => self.place(&n.children().next().ok_or("empty parentheses")?),
            SyntaxKind::IndexExpr => {
                let kids: Vec<SyntaxNode> = n.children().collect();
                // `A[x] [y]` on a multi-dimensional array is `A[x, y]`: the chain becomes one index (misc/lt-chained-array-index/
                // ChainedArrayIndex.al:P168_Chained_TwoDimensions_ReadWrite, P168_Chained_ThreeDimensionsAndText).
                if kids[0].kind() == SyntaxKind::IndexExpr && matches!(self.ty_ref(&kids[0]), Ty::Array(..)) {
                    if let ir::Place::Index(root, mut idx) = self.place(&kids[0])? {
                        for k in &kids[1..] {
                            idx.push(self.expr(k)?);
                        }
                        return Ok(ir::Place::Index(root, idx));
                    }
                }
                let base = self.place(&kids[0])?;
                let idx = kids[1..].iter().map(|k| self.expr(k)).collect::<L<Vec<_>>>()?;
                Ok(ir::Place::Index(Box::new(base), idx))
            }
            _ => match self.res(n) {
                Some(Res::TddStub(site)) if site.kind != crate::check::tdd::TddKind::Proc => {
                    Err(crate::check::tdd_decl::site_reason(site))
                }
                Some(Res::Local(name)) if self.var_attr_refusal(name, true).is_some() => {
                    let why = self.var_attr_refusal(name, true).unwrap_or_default();
                    self.run_refusals.insert(why.clone());
                    Err(why)
                }
                Some(Res::Global(name)) if !self.in_request_page && self.var_attr_refusal(name, false).is_some() => {
                    let why = self.var_attr_refusal(name, false).unwrap_or_default();
                    self.run_refusals.insert(why.clone());
                    Err(why)
                }
                Some(Res::Local(name)) => self
                    .locals
                    .get(name)
                    .map(|i| ir::Place::Local(*i))
                    .ok_or_else(|| format!("local `{name}`")),
                // An extension variable named like a base variable is its own (corpus
                // handlers/TestPageExtensionBaseAccessContracts.al:20, Shadow).
                Some(Res::Global(name)) => self
                    .in_request_page
                    .then(|| self.request_page_globals.get(name))
                    .flatten()
                    .or_else(|| self.globals.get(name))
                    .map(|i| ir::Place::Global(*i))
                    .ok_or_else(|| format!("implicit variable `{name}` is not lowered yet")),
                Some(Res::BaseGlobal(name)) => {
                    let name = name.clone();
                    let (slot, _, _) = self.base_var(&name)?;
                    Ok(ir::Place::BaseGlobal(slot))
                }
                Some(Res::Field { table, id }) => {
                    let (table, id) = (*table, *id);
                    // A temporary record of a platform table is an ordinary temporary table: the
                    // columns the platform provider pins do not matter (record/
                    // TestPlatformTablesWritableContracts.al:27-48).
                    let temporary = n.kind() == SyntaxKind::MemberExpr
                        && n.children()
                            .next()
                            .is_some_and(|b| matches!(self.ty_ref(&b), Ty::Record { temporary: true, .. }));
                    if temporary {
                        self.field_pos(table, id).ok_or_else(|| "field position".to_string())?;
                    } else {
                        self.readable_field_pos(table, id)?;
                        self.coverage_test_id_read(n, table, id)?;
                        self.coverage_lookup_method_id_read(n, table, id)?;
                    }
                    let field = self.field_ref(table, id);
                    let base = match n.kind() {
                        SyntaxKind::MemberExpr => self.place(&n.children().next().ok_or("member base")?)?,
                        _ => self.implicit_rec(n)?,
                    };
                    Ok(ir::Place::Field(Box::new(base), field))
                }
                _ if n.kind() == SyntaxKind::NameRef && n.text().to_string().trim().eq_ignore_ascii_case("this") => Err(
                    "`this` passed by reference (`BindSubscription(this)`) is not lowered: no corpus test passes the running instance as an argument"
                        .into(),
                ),
                _ => Err(self
                    .base_member(n)
                    .or_else(|| self.control_add_in(n))
                    .unwrap_or_else(|| format!("`{}` is not assignable", n.text()))),
            },
        }
    }
}

/// The name after the last dot of a member expression.
fn member_ident(n: &SyntaxNode) -> Option<String> {
    let text = n.text().to_string();
    let last = text.rsplit('.').next()?.trim().trim_matches('"');
    (!last.is_empty()).then(|| last.to_string())
}

/// `body` with `guard` in front of every `exit` that gives no value (the named return value of an OnQueryClosePage).
fn guard_exits(body: Vec<ir::Stmt>, guard: &[ir::Stmt]) -> Vec<ir::Stmt> {
    use ir::Stmt as S;
    let mut out = Vec::with_capacity(body.len());
    for s in body {
        match s {
            S::Exit(None) => {
                out.extend(guard.iter().cloned());
                out.push(S::Exit(None));
            }
            S::If(c, a, b) => out.push(S::If(c, guard_exits(a, guard), guard_exits(b, guard))),
            S::While(c, b) => out.push(S::While(c, guard_exits(b, guard))),
            S::Repeat(b, c) => out.push(S::Repeat(guard_exits(b, guard), c)),
            S::AssertError(b) => out.push(S::AssertError(guard_exits(b, guard))),
            S::For {
                var,
                from,
                to,
                down,
                body,
            } => out.push(S::For {
                var,
                from,
                to,
                down,
                body: guard_exits(body, guard),
            }),
            S::Foreach { var, collection, body } => out.push(S::Foreach {
                var,
                collection,
                body: guard_exits(body, guard),
            }),
            S::Case {
                scrutinee,
                branches,
                otherwise,
            } => out.push(S::Case {
                scrutinee,
                branches: branches.into_iter().map(|(e, b)| (e, guard_exits(b, guard))).collect(),
                otherwise: guard_exits(otherwise, guard),
            }),
            other => out.push(other),
        }
    }
    out
}

/// The member index of `SecurityFilter` named by `[SecurityFiltering(SecurityFilter::X)]`.
fn security_filtering_attr_member(v: &ast::VarDecl) -> Result<i32, String> {
    let attr = v
        .attributes()
        .into_iter()
        .find(|a| a.name().eq_ignore_ascii_case("SecurityFiltering"))
        .ok_or("no SecurityFiltering attribute")?;
    let text = attr.args().map(|a| a.text().to_string()).unwrap_or_default();
    let inner = text.trim().trim_start_matches('(').trim_end_matches(')').trim();
    let member = inner.rsplit("::").next().unwrap_or("").trim().trim_matches('"');
    let members = al_builtins::sysenum::option_members("SecurityFilter").expect("known");
    members
        .iter()
        .position(|m| m.eq_ignore_ascii_case(member))
        .map(|i| i as i32)
        .ok_or_else(|| format!("the attribute SecurityFiltering({inner}) is not understood"))
}

/// Whether the routine body (lower case, without white space) does something to the variable `name` whose effect on
/// the declared SecurityFiltering value is not pinned: `Clear(name)`, `ClearAll`, `name :=`, passing it as an argument
/// to any call (a callee may clear or assign it through a `var` parameter), or `Reset`, `Copy` or `TransferFields` on
/// it. A match inside a string or a comment only makes the check stricter.
fn cleared_or_assigned(body: &str, name: &str) -> bool {
    let n = name.to_lowercase();
    let quoted = format!("\"{n}\"");
    body.contains("clearall")
        || [&n, &quoted].iter().any(|v| {
            body.contains(&format!("{v}:="))
                || body.contains(&format!("({v},"))
                || body.contains(&format!("({v})"))
                || body.contains(&format!(",{v}"))
                || ["reset(", "copy(", "transferfields("]
                    .iter()
                    .any(|m| body.contains(&format!("{v}.{m}")))
        })
}

/// `if <cond> then continue else <stmt>`, the then branch a bare `continue` or a block with a `continue` of its own.
fn continue_else(n: &SyntaxNode) -> bool {
    if n.kind() != SyntaxKind::IfStmt {
        return false;
    }
    let Some(else_at) = sig_tokens(n)
        .iter()
        .find(|t| t.text().eq_ignore_ascii_case("else"))
        .map(|t| t.text_range().start())
    else {
        return false;
    };
    n.children()
        .skip(1)
        .filter(|k| k.text_range().start() < else_at)
        .any(|k| match k.kind() {
            SyntaxKind::ContinueStmt => true,
            SyntaxKind::Block => k.children().any(|c| c.kind() == SyntaxKind::ContinueStmt),
            _ => false,
        })
}

impl Lowerer<'_> {
    /// The refusal for a routine whose body runs `<receiver>.CreateInStream(<name>, ...)` (a call in the syntax tree, so
    /// comments and strings never match) with `<name>` resolving to one of `handed` (an InStream the caller gets back) and
    /// the receiver's root name resolving to one of `locals` (a variable declared in the routine's own `var` section).
    fn stream_of_a_local_handed_back(&self, body: &SyntaxNode, handed: &[String], locals: &[String]) -> Option<String> {
        let local_named = |n: &SyntaxNode, names: &[String]| match self.res(n) {
            Some(Res::Local(v)) => names.iter().any(|h| h.eq_ignore_ascii_case(v)),
            _ => false,
        };
        for call in body.descendants().filter(|n| n.kind() == SyntaxKind::CallExpr) {
            let mut kids = call.children();
            let (Some(callee), Some(args)) = (kids.next(), kids.find(|k| k.kind() == SyntaxKind::ArgList)) else {
                continue;
            };
            if callee.kind() != SyntaxKind::MemberExpr {
                continue;
            }
            let mut parts = callee.children();
            let (Some(recv), Some(member)) = (parts.next(), parts.next()) else {
                continue;
            };
            if !member.text().to_string().trim().eq_ignore_ascii_case("CreateInStream") {
                continue;
            }
            let Some(first) = args.children().next() else {
                continue;
            };
            if first.kind() != SyntaxKind::NameRef || !local_named(&first, handed) {
                continue;
            }
            let mut root = recv;
            while matches!(
                root.kind(),
                SyntaxKind::MemberExpr | SyntaxKind::IndexExpr | SyntaxKind::CallExpr | SyntaxKind::ParenExpr
            ) {
                match root.children().next() {
                    Some(c) => root = c,
                    None => break,
                }
            }
            if root.kind() == SyntaxKind::NameRef && local_named(&root, locals) {
                return Some(format!(
                    "an InStream handed back to the caller is created on the local variable `{}`: BC reads it as empty once \
                     the local is out of scope, the runtime keeps the bytes",
                    root.text().to_string().trim()
                ));
            }
        }
        None
    }
}
