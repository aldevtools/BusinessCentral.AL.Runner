//! Type inference for expressions and the diagnostics that hang off it.
use super::*;

impl<'a> Checker<'a> {
    /// AL0122 for an enum assigned to an enum of another type whose TARGET does not state
    /// `AssignmentCompatibility = true` (the source's property does not help): corpus must-not-compile
    /// 0074 and 0075, and types/PXEnumRecordRef.al:7-10 for the target that
    /// states it. An enum the front end cannot resolve is not judged. Reported for the workspace only.
    pub(crate) fn al_enum_assignment(&mut self, sc: &Scope, rhs: &SyntaxNode, target: &Ty, source: &Ty) {
        let (Ty::Enum(t), Ty::Enum(s)) = (target, source) else {
            return;
        };
        if self.ws.is_dependency(sc.oi) {
            return;
        }
        let (Some(ti), Some(si)) = (self.resolve(sc, ObjKind::Enum, t), self.resolve(sc, ObjKind::Enum, s)) else {
            return;
        };
        if ti == si || self.members[ti].assignment_compatible {
            return;
        }
        let (tn, sn) = (self.ws.objects[ti].name.clone(), self.ws.objects[si].name.clone());
        self.diag(
            sc,
            rhs,
            "AL0122",
            format!("Cannot implicitly convert type 'Enum \"{sn}\"' to 'Enum \"{tn}\"'"),
        );
    }

    /// AL0122 for the two shapes the must-not-compile corpus pins: an Integer variable assigned to a
    /// Text (case 0069, `T := I`) and a text literal of more than one character assigned to an Integer
    /// (case 0070, `I := 'text'`; a one-character literal is a Char, which an Integer takes). Other
    /// sources (a call, a Text variable) and targets (Code, BigInteger, ...) are not judged.
    pub(crate) fn al_integer_text_assignment(&mut self, sc: &Scope, rhs: &SyntaxNode, target: &Ty, source: &Ty) {
        if self.ws.is_dependency(sc.oi) {
            return;
        }
        // A Char variable minus a one-character literal is an Integer, so it cannot go into a Text; a Char variable plus a literal of
        // two or more characters is a Text, so it cannot go into an Integer (must-not-compile 1041, `T := C - 'A'`, `I := C + 'AB'`).
        // Other operators, operand shapes and targets are not judged.
        if rhs.kind() == SyntaxKind::BinExpr {
            if let Some(message) = self.char_literal_result(sc, rhs, target) {
                self.diag(sc, rhs, "AL0122", message);
            }
            return;
        }
        let message = match (target, source) {
            (Ty::Text(len), Ty::Integer) if rhs.kind() == SyntaxKind::NameRef => {
                let t = len.map_or("Text".to_string(), |n| format!("Text[{n}]"));
                format!("Cannot implicitly convert type 'Integer' to '{t}'")
            }
            // A Text variable (case 0501, `D := T`) into a Decimal; a text literal of more than one character is judged below as for an
            // Integer (case 0501, `D := '100'`). Text[30], Code and a one-character literal are not judged.
            (Ty::Decimal, Ty::Text(None)) if rhs.kind() == SyntaxKind::NameRef => {
                "Cannot implicitly convert type 'Text' to 'Decimal'".to_string()
            }
            (Ty::Integer | Ty::Decimal, Ty::Text(None)) if rhs.kind() == SyntaxKind::Literal => {
                let Some(tok) = rhs
                    .children_with_tokens()
                    .filter_map(|e| e.into_token())
                    .find(|t| t.kind() == SyntaxKind::StringLit)
                else {
                    return;
                };
                let text = tok.text();
                let inner = text
                    .get(1..text.len().saturating_sub(1))
                    .unwrap_or("")
                    .replace("''", "'");
                if inner.chars().count() <= 1 {
                    return;
                }
                format!(
                    "Cannot implicitly convert type 'Text' to '{}'",
                    if matches!(target, Ty::Decimal) {
                        "Decimal"
                    } else {
                        "Integer"
                    }
                )
            }
            _ => return,
        };
        self.diag(sc, rhs, "AL0122", message);
    }

    /// The message of AL0122 for `Char var - 'A'` into a Text or `Char var + 'AB'` into an Integer, else `None`.
    fn char_literal_result(&self, sc: &Scope, bin: &SyntaxNode, target: &Ty) -> Option<String> {
        let kids: Vec<SyntaxNode> = bin.children().collect();
        let [left, right] = kids.as_slice() else { return None };
        if left.kind() != SyntaxKind::NameRef || right.kind() != SyntaxKind::Literal {
            return None;
        }
        let r = left.text_range();
        let left_ty = self.types.get(&(sc.file, r.start().into(), r.end().into()))?;
        if !matches!(left_ty, Ty::Char) {
            return None;
        }
        let tok = right
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .find(|t| t.kind() == SyntaxKind::StringLit)?;
        let text = tok.text();
        let chars = text
            .get(1..text.len().saturating_sub(1))?
            .replace("''", "'")
            .chars()
            .count();
        match (op_token(bin).as_str(), target) {
            ("-", Ty::Text(len)) if chars == 1 => {
                let t = len.map_or("Text".to_string(), |n| format!("Text[{n}]"));
                Some(format!("Cannot implicitly convert type 'Integer' to '{t}'"))
            }
            ("+", Ty::Integer) if chars >= 2 => Some("Cannot implicitly convert type 'Text' to 'Integer'".to_string()),
            _ => None,
        }
    }

    /// AL0175 for the two shapes must-not-compile 0500 pins (corpus tests/must-not-compile/cases/
    /// 0500-char-minus-text.al:1-2): `Char - Text` and `Text - Char`, both operands variables, the Text one
    /// unbounded. A text literal with a Char compiles (an Integer), and Text[N], Code and other operands are
    /// not pinned, so they are not judged. Reported for the workspace only.
    fn al_char_minus_text(&mut self, sc: &Scope, node: &SyntaxNode, kids: &[SyntaxNode], l: &Ty, r: &Ty) {
        if op_token(node) != "-" || self.ws.is_dependency(sc.oi) || kids.len() != 2 {
            return;
        }
        if !kids.iter().all(|k| k.kind() == SyntaxKind::NameRef) {
            return;
        }
        let (a, b) = match (l, r) {
            (Ty::Char, Ty::Text(None)) => ("Char", "Text"),
            (Ty::Text(None), Ty::Char) => ("Text", "Char"),
            _ => return,
        };
        self.diag(
            sc,
            node,
            "AL0175",
            format!("Operator '-' cannot be applied to operands of type '{a}' and '{b}'"),
        );
    }

    /// AL0398 for Decimal constants where an Option ordinal is expected: the labels of `case O of 1.0:` (corpus
    /// tests/must-not-compile/cases/2008-case-option-decimal-label.al:1) and the list of `O in [0.0, 1.5]`
    /// (2009-option-in-decimal-list.al:1-2), `O` an Option variable. Only plain Decimal literals are judged; a
    /// Decimal variable label and `O = 1.5` compile (types/PROptionInteger.al:175-178, :308-322). In a list, only
    /// one of Decimal literals alone (the pinned shape). Reported for the workspace only.
    pub(crate) fn al_option_decimal_constants(
        &mut self,
        sc: &Scope,
        selector: &SyntaxNode,
        sel_ty: &Ty,
        consts: &[SyntaxNode],
    ) {
        if self.ws.is_dependency(sc.oi) || selector.kind() != SyntaxKind::NameRef || !matches!(sel_ty, Ty::Option(_)) {
            return;
        }
        let decimal = |n: &SyntaxNode| -> Option<String> {
            if n.kind() != SyntaxKind::Literal {
                return None;
            }
            n.children_with_tokens()
                .filter_map(|e| e.into_token())
                .find(|t| !t.kind().is_trivia())
                .filter(|t| t.kind() == SyntaxKind::Decimal)
                .map(|t| t.text().to_string())
        };
        for c in consts {
            if let Some(text) = decimal(c) {
                self.diag(
                    sc,
                    c,
                    "AL0398",
                    format!("Constant value '{text}' is outside of the valid ordinal range for this Option type"),
                );
            }
        }
    }

    /// AL0122 for the scalar pairs the must-not-compile corpus pins, where the source is a plain name (a variable,
    /// parameter or field): Decimal, Integer and Text to Boolean (0260, 0265, 0286), Date to Text (0261), Boolean,
    /// Text, Decimal and DateTime to Date (0262, 0264, 0268, 0269), Variant to an Enum (0263), a Record to an
    /// Integer (0266), a Time to an Integer (0271), a JsonToken to a JsonObject (0273); a text literal of two or
    /// more characters to a Boolean (0270); and a one-character text literal (a Char) to an Enum (0267). Other
    /// pairs are not judged.
    pub(crate) fn al_scalar_assignment(&mut self, sc: &Scope, rhs: &SyntaxNode, target: &Ty, source: &Ty) {
        if self.ws.is_dependency(sc.oi) {
            return;
        }
        let from = match rhs.kind() {
            SyntaxKind::NameRef => {
                let json = |t: &Ty, n: &str| matches!(t, Ty::Builtin(b) if b.eq_ignore_ascii_case(n));
                let pinned = matches!(
                    (target, source),
                    (Ty::Boolean, Ty::Decimal | Ty::Integer | Ty::Text(None))
                        | (Ty::Text(_), Ty::Date)
                        | (Ty::Date, Ty::Boolean | Ty::Text(_) | Ty::Decimal | Ty::DateTime)
                        | (Ty::Enum(_), Ty::Variant)
                        | (Ty::Integer, Ty::Record { .. } | Ty::Time)
                        // Corpus must-not-compile 0430, 0435, 0445, 0438, 0448 (to a Boolean), 0433, 0434, 0436, 0407
                        // (from a Boolean or a Text), 0401, 0403 (from an enum), 0412 to 0419 (to an enum). The
                        // enum pairs hold whatever `AssignmentCompatibility` says (0401, 0403: enum A states it).
                        | (Ty::Boolean, Ty::BigInteger | Ty::Option(_) | Ty::Enum(_))
                        | (Ty::Integer | Ty::BigInteger, Ty::Boolean)
                        | (Ty::Option(_), Ty::Boolean | Ty::Text(None))
                        | (Ty::Text(_) | Ty::Code(_), Ty::Enum(_))
                        | (
                            Ty::Enum(_),
                            Ty::Decimal | Ty::Code(_) | Ty::Text(_) | Ty::Boolean | Ty::Date | Ty::Char
                        )
                ) || (json(target, "JsonObject") && json(source, "JsonToken"));
                pinned.then(|| source.to_string())
            }
            SyntaxKind::Literal => {
                // An integer literal to a Boolean (corpus must-not-compile 0432).
                if matches!(target, Ty::Boolean)
                    && rhs
                        .children_with_tokens()
                        .filter_map(|e| e.into_token())
                        .any(|t| t.kind() == SyntaxKind::Integer)
                {
                    let from = "Integer".to_string();
                    self.diag(
                        sc,
                        rhs,
                        "AL0122",
                        format!("Cannot implicitly convert type '{from}' to '{target}'"),
                    );
                    return;
                }
                let chars = rhs
                    .children_with_tokens()
                    .filter_map(|e| e.into_token())
                    .find(|t| t.kind() == SyntaxKind::StringLit)
                    .map(|tok| {
                        let text = tok.text();
                        text.get(1..text.len().saturating_sub(1))
                            .unwrap_or("")
                            .replace("''", "'")
                            .chars()
                            .count()
                    });
                match (target, chars) {
                    (Ty::Enum(_), Some(1)) => Some("Char".to_string()),
                    (Ty::Boolean, Some(n)) if n > 1 => Some("Text".to_string()),
                    _ => None,
                }
            }
            _ => None,
        };
        let Some(from) = from else { return };
        self.diag(
            sc,
            rhs,
            "AL0122",
            format!("Cannot implicitly convert type '{from}' to '{target}'"),
        );
    }

    /// AL0122 for a condition that is no Boolean (corpus must-not-compile 0285 `if I then`, 0286 `while T do`):
    /// the pairs of an assignment to a Boolean.
    pub(crate) fn al_condition(&mut self, sc: &Scope, cond: &SyntaxNode, source: &Ty) {
        self.al_scalar_assignment(sc, cond, &Ty::Boolean, source);
    }

    /// AL0122 (corpus must-not-compile 0275): the result of a procedure that returns nothing is assigned. Only a
    /// bare call of a procedure of the object itself is judged, for a scalar target.
    pub(crate) fn al_no_result_assignment(&mut self, sc: &Scope, rhs: &SyntaxNode, target: &Ty, source: &Ty) {
        if self.ws.is_dependency(sc.oi)
            || rhs.kind() != SyntaxKind::CallExpr
            || *source != Ty::Unknown
            || !matches!(
                target,
                Ty::Integer
                    | Ty::BigInteger
                    | Ty::Decimal
                    | Ty::Boolean
                    | Ty::Text(_)
                    | Ty::Code(_)
                    | Ty::Date
                    | Ty::Time
                    | Ty::DateTime
                    | Ty::Guid
            )
        {
            return;
        }
        let Some(callee) = rhs.children().next().filter(|c| c.kind() == SyntaxKind::NameRef) else {
            return;
        };
        let name = name_of(&callee);
        let own = self.members[sc.self_obj].procedures_named(&name);
        // A TryFunction returns a Boolean without declaring one (corpus errors/lt-error-handling/ErTTryBasics.al:81).
        let try_function = self.ws.object_ast(sc.self_obj).routines().any(|r| {
            r.name().eq_ignore_ascii_case(&name) && r.attributes().any(|a| a.name().eq_ignore_ascii_case("TryFunction"))
        });
        if try_function || own.is_empty() || own.iter().any(|p| p.ret.is_some()) || sc_has_var(sc, &name.to_lowercase())
        {
            return;
        }
        self.diag(
            sc,
            rhs,
            "AL0122",
            format!("Cannot implicitly convert type 'None' to '{target}'"),
        );
    }

    /// AL0122 (corpus must-not-compile 0324): a codeunit variable is assigned to an interface variable and the codeunit
    /// (a source object of the workspace) implements neither the interface nor an interface that extends it. An
    /// implemented interface that does not resolve ends the check.
    pub(crate) fn al_interface_assignment(&mut self, sc: &Scope, rhs: &SyntaxNode, target: &Ty, source: &Ty) {
        let (Ty::Object(ObjKind::Interface, iname), Ty::Object(ObjKind::Codeunit, cname)) = (target, source) else {
            return;
        };
        if rhs.kind() != SyntaxKind::NameRef || self.ws.is_dependency(sc.oi) {
            return;
        }
        let (Some(ii), Some(ci)) = (
            self.resolve(sc, ObjKind::Interface, iname),
            self.resolve(sc, ObjKind::Codeunit, cname),
        ) else {
            return;
        };
        if self.ws.is_dependency(ci) {
            return;
        }
        let family = self.ws.interface_family(ii);
        let file = self.ws.objects[ci].file;
        for parts in &self.ws.objects[ci].implements {
            match self.resolve_parts(file, ObjKind::Interface, parts) {
                Some(j) if family.contains(&j) => return,
                Some(_) => {}
                None => return,
            }
        }
        self.diag(
            sc,
            rhs,
            "AL0122",
            format!(
                "Cannot implicitly convert type '{}' to '{}'",
                capitalized_kind(source),
                capitalized_kind(target)
            ),
        );
    }

    /// AL0161 (corpus must-not-compile 0356, 0358, 0359): `C.Name` on a codeunit variable of another codeunit, where `Name`
    /// is only declared `local` or is a global variable (never reachable through a variable).
    pub(crate) fn al_codeunit_access(&mut self, sc: &Scope, node: &SyntaxNode, base: &Ty, name: &str) {
        // A local procedure of a table, called through a record of it (corpus must-not-compile 0359). Judged on the
        // table's own source, so a procedure of a table extension is never taken for it.
        if let Ty::Record { table, .. } = base {
            if self.ws.is_dependency(sc.oi) {
                return;
            }
            let Some(ti) = self.resolve(sc, ObjKind::Table, table) else {
                return;
            };
            if ti == sc.oi || ti == sc.self_obj || self.ws.is_dependency(ti) {
                return;
            }
            let ast = self.ws.object_ast(ti);
            let mut named = ast
                .routines()
                .filter(|r| r.name().eq_ignore_ascii_case(name))
                .peekable();
            if named.peek().is_some() && named.all(|r| r.is_local()) {
                self.diag(
                    sc,
                    node,
                    "AL0161",
                    format!("'{name}()' is inaccessible due to its protection level"),
                );
            }
            return;
        }
        let Ty::Object(ObjKind::Codeunit, cname) = base else {
            return;
        };
        if self.ws.is_dependency(sc.oi) {
            return;
        }
        let Some(ci) = self.resolve(sc, ObjKind::Codeunit, cname) else {
            return;
        };
        if ci == sc.oi || ci == sc.self_obj {
            return;
        }
        let procs = self.members[ci].procedures_named(name);
        let shown = if !procs.is_empty() {
            if !procs.iter().all(|p| p.local) {
                return;
            }
            format!("{name}()")
        } else if crate::base_access::declares_var(&self.ws.object_ast(ci), name) {
            name.to_string()
        } else {
            return;
        };
        self.diag(
            sc,
            node,
            "AL0161",
            format!("'{shown}' is inaccessible due to its protection level"),
        );
    }

    /// Infers `node` as a value, calling a parameterless callable.
    pub(crate) fn ty(&mut self, sc: &mut Scope, node: &SyntaxNode) -> Ty {
        let e = self.infer(sc, node);
        self.ty_of_inferred(sc, node, e)
    }

    /// The target of `Proc := value` where `Proc` is a procedure of the workspace that takes one parameter and none takes
    /// none: AL reads it as the call `Proc(value)` (Base Application's Service Statistics writes
    /// `CurrPage.SubForm.PAGE.SetParentControl := PAGE::"Service Statistics"`). The call is picked with the value's type
    /// and recorded on the target; the type of the target is unknown. Assumption `procedure-assign-is-call`.
    pub(crate) fn assign_target_ty(&mut self, sc: &mut Scope, target: &SyntaxNode, rhs: &SyntaxNode) -> Ty {
        let e = self.infer(sc, target);
        if let E::Callable(c) = &e {
            let visible: Vec<&Cand> = c.iter().filter(|c| c.hidden.is_none()).collect();
            if !visible.is_empty()
                && visible
                    .iter()
                    .all(|c| matches!(c.target, Res::Proc { .. }) && c.min == 1 && c.max >= 1)
            {
                let c = c.clone();
                let arg = self.ty(sc, rhs);
                let t = self.pick(sc, target, &c, &[arg], &name_of(target));
                let _ = t;
                let r = target.text_range();
                self.types
                    .insert((sc.file, r.start().into(), r.end().into()), Ty::Unknown);
                return Ty::Unknown;
            }
        }
        self.ty_of_inferred(sc, target, e)
    }

    fn ty_of_inferred(&mut self, sc: &mut Scope, node: &SyntaxNode, e: E) -> Ty {
        let t = match e {
            E::Val(t) => t,
            E::Callable(c) => self.pick(sc, node, &c, &[], &name_of(node)),
            E::PartialObj(k, parts) => {
                // AL0118 (corpus must-not-compile 0222, 0223): `Codeunit::"X"` names no codeunit. Only the plain
                // one-segment name of a codeunit is judged (a namespace may qualify it; other object kinds can
                // be platform objects the workspace does not carry).
                if k == ObjKind::Codeunit
                    && parts.len() == 1
                    && parts[0].parse::<u32>().is_err()
                    && node.kind() == SyntaxKind::ScopeExpr
                    && !self.ws.names_object(sc.file, k, &parts)
                {
                    if let Some(n) = node.children().nth(1).filter(|n| n.kind() == SyntaxKind::NameRef) {
                        if !self.ws.is_dependency(sc.oi) {
                            self.diag(
                                sc,
                                node,
                                "AL0118",
                                format!("The name '{}' does not exist in the current context.", n.text()),
                            );
                        }
                    }
                }
                self.diag(
                    sc,
                    node,
                    "AR0002",
                    format!("unresolved {} `{}`", k.label(), parts.join(".")),
                );
                Ty::Integer
            }
            _ => Ty::Unknown,
        };
        let r = node.text_range();
        self.types
            .insert((sc.file, r.start().into(), r.end().into()), t.clone());
        t
    }

    /// The AL compiler's own diagnostics for a name that resolved to nothing (`alcodes.rs`). Both
    /// are reported for the code of the workspace only, never for a dependency.
    pub(crate) fn al_unresolved_name(&mut self, sc: &Scope, node: &SyntaxNode) {
        if self.ws.is_dependency(sc.oi) {
            return;
        }
        let name = name_of(node);
        let kind = self.ws.objects[sc.oi].kind;
        // AL0161 (corpus must-not-compile 0001, 0002; handlers/TestPageExtensionBaseAccessContracts.al,
        // header): a page extension names a base page variable that is not `protected`, or calls a
        // `local` base procedure. The extension's own variables and parameters were looked up first.
        let base_kind = match kind {
            ObjKind::PageExtension => Some(ObjKind::Page),
            ObjKind::ReportExtension => Some(ObjKind::Report),
            _ => None,
        };
        if let Some(base_kind) = base_kind {
            if let Some(b) = self.ws.extension_target(sc.oi) {
                if self.ws.objects[b].kind == base_kind {
                    let base = self.ws.object_ast(b);
                    let is_var = crate::base_access::declares_var(&base, &name);
                    let is_local_proc = base
                        .routines()
                        .any(|r| r.name().eq_ignore_ascii_case(&name) && r.is_local());
                    if is_var || is_local_proc {
                        let shown = if is_var { name.clone() } else { format!("{name}()") };
                        self.diag(
                            sc,
                            node,
                            "AL0161",
                            format!("'{shown}' is inaccessible due to its protection level"),
                        );
                        return;
                    }
                }
            }
        }
        // AL0118 (corpus must-not-compile 0005, 0030 to 0033, 0059). A call of a bare name that nothing can
        // supply:
        // - in a codeunit without `TableNo` (no implicit record): no procedure of the codeunit and no system
        //   function;
        // - in a table, a page, a report or a page extension: the lookup above also tried the implicit
        //   record, the base object and the system functions, but not every implicit member of those is
        //   modelled, so the name must also be a method name that no type of the catalog has;
        // - the name `Rec` (not a call) in a codeunit without `TableNo`.
        let is_callee =
            node.parent().is_some_and(|p| p.kind() == SyntaxKind::CallExpr) && node.prev_sibling().is_none();
        // A bare name that is not a call (corpus must-not-compile 0200, 0205 to 0210): no variable, label,
        // field, data item or column of the object, no system variable and no function of the platform. In a
        // codeunit without an implicit record it is judged wherever it stands; in the other objects only as
        // the target of `:=` in a trigger or procedure of the object itself (not in a data item, control
        // or field trigger, whose implicit members are not all modelled).
        let bare_value = !is_callee
            && !SYSTEM_VARIABLES.iter().any(|s| s.eq_ignore_ascii_case(&name))
            && !self.catalog.knows_name(&name)
            && !self.has_named_section(sc.oi, &name)
            && !self.sibling_extension_declares(sc.oi, &name);
        let assigned =
            node.parent().is_some_and(|p| p.kind() == SyntaxKind::AssignStmt) && node.prev_sibling().is_none();
        let object_level = !node.ancestors().any(|a| a.kind() == SyntaxKind::Section);
        // System variables the object type does not have (corpus must-not-compile 0351 to 0355): `CurrPage`,
        // `CurrReport`, `CurrXmlPort`, `CurrQuery` and `xRec` in a codeunit without `TableNo`, the first four in a table,
        // `Rec` in the object-level code of a report without a request page source table.
        let lower = name.to_ascii_lowercase();
        let absent_system_variable = match kind {
            ObjKind::Codeunit => {
                sc.rec.is_none()
                    && matches!(
                        lower.as_str(),
                        "currpage" | "currreport" | "currxmlport" | "currquery" | "xrec"
                    )
            }
            ObjKind::Table | ObjKind::TableExtension => {
                matches!(lower.as_str(), "currpage" | "currreport" | "currxmlport" | "currquery")
            }
            ObjKind::Report => lower == "rec" && sc.rec.is_none() && object_level,
            _ => false,
        } && !is_callee;
        let judged = absent_system_variable
            || match kind {
                // A bare call can also be a method of the Codeunit type called on the running instance: `Run()`
                // runs the codeunit itself (it compiles in BC; NP Retail's scanner import codeunits call it).
                ObjKind::Codeunit => {
                    sc.rec.is_none()
                        && ((is_callee
                            && self.catalog.overloads("Codeunit", &name).is_empty()
                            && self.catalog.hidden("Codeunit", &name).is_empty())
                            || name.eq_ignore_ascii_case("Rec")
                            || bare_value)
                }
                ObjKind::Table | ObjKind::Page | ObjKind::Report | ObjKind::PageExtension => {
                    (is_callee && !self.catalog.knows_name(&name) && !self.sibling_extension_declares(sc.oi, &name))
                        || (bare_value && assigned && object_level && kind != ObjKind::PageExtension)
                }
                ObjKind::Query | ObjKind::XmlPort => bare_value && assigned && object_level,
                _ => false,
            };
        if judged {
            self.diag(
                sc,
                node,
                "AL0118",
                format!("The name '{name}' does not exist in the current context."),
            );
        }
    }

    /// Whether another extension of the object that extension `oi` extends declares a procedure `name`. The
    /// extensions of one page merge, so a page extension calls a public or `internal` procedure of a sibling
    /// (Microsoft's Tests-TestLibraries UsersPBTTest.PageExt.al calls RefreshFactboxes of Base Application's
    /// UsersPerfFactboxes.PageExt.al).
    pub(crate) fn sibling_extension_declares(&self, oi: usize, name: &str) -> bool {
        let Some(base) = self.ws.extension_target(oi) else {
            return false;
        };
        self.ws.extensions_of(base).iter().filter(|&&e| e != oi).any(|&e| {
            self.ws
                .object_ast(e)
                .routines()
                .any(|r| r.name().eq_ignore_ascii_case(name))
        })
    }

    /// AL0132 (corpus must-not-compile 0060, 0061): `R.Name` or `R.Name(...)` on a `Record` variable
    /// whose table the workspace declares, where `Name` is no field of the table, no procedure of it and
    /// no method of `Record` in any version of the catalog. A table that an extension adds fields to is
    /// not judged (the fields of an extension that was not loaded are out of reach).
    pub(crate) fn al_unknown_record_member(&mut self, sc: &Scope, node: &SyntaxNode, base: &Ty, name: &str) {
        let Ty::Record { table, .. } = base else { return };
        if self.ws.is_dependency(sc.oi) {
            return;
        }
        let Some(ti) = self.resolve(sc, ObjKind::Table, table) else {
            return;
        };
        if !self.ws.extensions_of(ti).is_empty()
            || !self.catalog.overloads("Record", name).is_empty()
            || !self.catalog.hidden("Record", name).is_empty()
        {
            return;
        }
        self.diag(
            sc,
            node,
            "AL0132",
            format!("'Record \"{table}\"' does not contain a definition for '{name}'"),
        );
    }

    /// AL0132 for a method that a library type does not have (corpus must-not-compile 0107 `Length` on a Text, 0224
    /// `Contain` on a Text, 0225 `Length` on a List, 0226 `Size` on a Dictionary, 0228 `Put` on a JsonObject,
    /// 0229 `Send2` on an HttpClient, 0230 `Fieldd` on a RecordRef, 0233 `Invert` on a Boolean, 0234 `Size` on a
    /// Text, 0235 `GetName` on a JsonToken, 0236 `Add` on a TextBuilder). Only these types are judged, and only
    /// when the catalog has no method of that name for the type in any runtime version: the catalog is not
    /// complete enough for the others.
    pub(crate) fn al_unknown_builtin_member(&mut self, sc: &Scope, node: &SyntaxNode, base: &Ty, name: &str) {
        const JUDGED: &[&str] = &["JsonObject", "JsonToken", "HttpClient", "RecordRef", "TextBuilder"];
        let shown = match base {
            Ty::Text(_) => "Text".to_string(),
            Ty::Code(_) => "Code".to_string(),
            Ty::Boolean | Ty::List(_) | Ty::Dictionary(..) => base.to_string(),
            Ty::Builtin(n) if JUDGED.iter().any(|j| j.eq_ignore_ascii_case(n)) => base.to_string(),
            _ => return,
        };
        let Some(recv) = receiver_of(base) else { return };
        if self.ws.is_dependency(sc.oi)
            || !self.catalog.overloads(&recv, name).is_empty()
            || !self.catalog.hidden(&recv, name).is_empty()
        {
            return;
        }
        self.diag(
            sc,
            node,
            "AL0132",
            format!("'{shown}' does not contain a definition for '{name}'"),
        );
    }

    /// The AR1002 text. On a builtin type it says the name is in no version of the
    /// catalog, and how far the docs behind the catalog reach (ADR-015 3.3 point 5).
    pub(crate) fn no_member(&self, name: &str, t: &Ty) -> String {
        let mut m = format!("no member `{name}` on {t}");
        if receiver_of(t).is_some() {
            m.push_str(&format!(
                "; not known to the catalog (docs for runtime up to {})",
                self.catalog.newest_documented_runtime()
            ));
        }
        m
    }

    pub(crate) fn infer(&mut self, sc: &mut Scope, node: &SyntaxNode) -> E {
        use SyntaxKind::*;
        let kids: Vec<SyntaxNode> = node.children().collect();
        match node.kind() {
            Literal => E::Val(literal_ty(node)),
            ParenExpr => E::Val(kids.first().map(|k| self.ty(sc, k)).unwrap_or(Ty::Unknown)),
            NameRef => self.name(sc, node),
            PrefixExpr => {
                let t = kids.first().map(|k| self.ty(sc, k)).unwrap_or(Ty::Unknown);
                let op = first_token(node).to_ascii_lowercase();
                if op == "not" {
                    if let Some(k) = kids.first() {
                        self.tdd_settle(sc, k, &Ty::Boolean);
                    }
                }
                E::Val(if op == "not" {
                    Ty::Boolean
                } else if matches!(t, Ty::DotNet(_)) {
                    // A DotNet operand is settled when it runs (ADR-024).
                    Ty::DotNet("".into())
                } else {
                    t
                })
            }
            BinExpr if matches!(op_token(node).as_str(), "is" | "as") => {
                // The right operand is an interface name, not a value: it is not resolved as a
                // name here. `x is I` is a Boolean, `x as I` has the type of interface I.
                if let Some(k) = kids.first() {
                    self.ty(sc, k);
                }
                if op_token(node) == "is" {
                    E::Val(Ty::Boolean)
                } else {
                    E::Val(kids.get(1).map_or(Ty::Unknown, |n| {
                        Ty::Object(ObjKind::Interface, type_name_text(n).into())
                    }))
                }
            }
            BinExpr => {
                let l = kids.first().map(|k| self.ty(sc, k)).unwrap_or(Ty::Unknown);
                let r = kids.get(1).map(|k| self.ty(sc, k)).unwrap_or(Ty::Unknown);
                self.al_char_minus_text(sc, node, kids.as_slice(), &l, &r);
                let op = op_token(node);
                if op == "in"
                    && let [a, set] = kids.as_slice()
                    && set.kind() == SyntaxKind::SetExpr
                {
                    let elems: Vec<SyntaxNode> = set.children().collect();
                    if elems.iter().all(|e| e.kind() == SyntaxKind::Literal) {
                        self.al_option_decimal_constants(sc, a, &l, &elems);
                    }
                }
                if let [a, b] = kids.as_slice() {
                    self.tdd_settle_operands(sc, &op, a, b, &l, &r);
                }
                if char_and_one_char_literal(&op, kids.as_slice(), &l, &r) {
                    return E::Val(Ty::Integer);
                }
                E::Val(binary_ty(&op, &l, &r))
            }
            TernaryExpr => {
                if let Some(c) = kids.first() {
                    self.ty(sc, c);
                }
                let t = kids.get(1).map(|k| self.ty(sc, k)).unwrap_or(Ty::Unknown);
                if let Some(e) = kids.get(2) {
                    self.ty(sc, e);
                }
                E::Val(t)
            }
            SetExpr | RangeExpr => {
                for k in &kids {
                    self.ty(sc, k);
                }
                E::Val(Ty::Unknown)
            }
            IndexExpr => {
                let base = kids.first().map(|k| self.ty(sc, k)).unwrap_or(Ty::Unknown);
                for k in kids.iter().skip(1) {
                    self.ty(sc, k);
                }
                E::Val(match base {
                    // Fewer indices than dimensions: a row, indexed further by a chained `[y]` (`A[x] [y]`).
                    Ty::Array(dims, el) if kids.len() - 1 < dims.len() => {
                        Ty::Array(dims[kids.len() - 1..].to_vec(), el)
                    }
                    Ty::Array(_, el) => *el,
                    Ty::Text(_) | Ty::Code(_) => Ty::Char,
                    _ => Ty::Unknown,
                })
            }
            CallExpr => {
                let callee = &kids[0];
                let args: Vec<SyntaxNode> = kids.get(1).map(|a| a.children().collect()).unwrap_or_default();
                let target = match (callee.kind() == NameRef)
                    .then(|| self.lookup_callable(sc, node, &name_of(callee)))
                    .flatten()
                {
                    Some(c) => E::Callable(c),
                    None => self.infer(sc, callee),
                };
                // The receiver whose fields bare-name arguments may refer to.
                let receiver = (callee.kind() == MemberExpr)
                    .then(|| callee.children().next())
                    .flatten()
                    .and_then(|b| {
                        let r = b.text_range();
                        self.types.get(&(sc.file, r.start().into(), r.end().into())).cloned()
                    });
                let cands: Vec<&Cand> = match &target {
                    E::Callable(c) => c.iter().filter(|c| c.hidden.is_none()).collect(),
                    _ => Vec::new(),
                };
                let unknown_callee = matches!(target, E::Val(Ty::Unknown));
                let mut arg_tys = Vec::with_capacity(args.len());
                self.reach_static_run(sc, callee, &args);
                for (i, a) in args.iter().enumerate() {
                    // On a receiver outside the workspace, a bare name may be one of
                    // its fields: resolve it if we can, but never report it.
                    if unknown_callee && a.kind() == NameRef {
                        let t = match self.lookup_name(sc, a) {
                            Some(E::Val(t)) => t,
                            _ => Ty::Unknown,
                        };
                        let r = a.text_range();
                        self.types
                            .insert((sc.file, r.start().into(), r.end().into()), t.clone());
                        arg_tys.push(t);
                        continue;
                    }
                    let field_pos = cands.iter().any(|c| c.is_field_pos(i));
                    // `Page.FindFirstField(Code, 'x')`: a bare name in a field position of a test page
                    // method is a control of that page (corpus
                    // handlers/TestPagePlatformPageSourceTable_Tests.al:66).
                    if field_pos && a.kind() == NameRef {
                        if let Some(t @ (Ty::TestPage(_) | Ty::TestRequestPage(_) | Ty::TestPart(_))) = receiver.clone()
                        {
                            // A variable of that name wins; `Code` is also a type name, so the
                            // name lookup in general is not asked.
                            let var = name_of(a).to_lowercase();
                            if !sc.vars.iter().any(|f| f.contains_key(&var)) {
                                if let Some(E::Val(ct)) = self.page_control_member(sc, &t, &name_of(a)) {
                                    let r = a.text_range();
                                    self.types
                                        .insert((sc.file, r.start().into(), r.end().into()), ct.clone());
                                    arg_tys.push(ct);
                                    continue;
                                }
                            }
                        }
                    }
                    if field_pos && a.kind() == NameRef {
                        if let Some(t) = receiver.as_ref().and_then(|r| self.field_of(sc, r, &name_of(a))) {
                            let r = a.text_range();
                            self.types
                                .insert((sc.file, r.start().into(), r.end().into()), t.clone());
                            if let Some(r) = receiver.as_ref().and_then(|r| self.field_res(sc, r, &name_of(a))) {
                                if matches!(r, Res::QueryColumn { .. }) {
                                    self.record(sc, a, r);
                                }
                            }
                            // A `TestPage.Filter` field is a field of the page's source table
                            // (corpus testfilter/TestTestFilter.al:1-30).
                            let table_idx = match &receiver {
                                Some(Ty::Record { table, .. } | Ty::TestFilter(table)) => {
                                    self.resolve(sc, ObjKind::Table, table)
                                }
                                _ => None,
                            };
                            if let Some(ti) = table_idx {
                                if let Some(f) = self.members[ti].field(&name_of(a)) {
                                    let id = f.id;
                                    // `--tdd`: `CalcFields` of a stub field is refused (ADR-051 decision 10).
                                    if !self.tdd_calcfields_stub(sc, a, &kids[0], ti, id) {
                                        self.record(sc, a, Res::Field { table: ti, id });
                                    }
                                }
                            }
                            arg_tys.push(t);
                            continue;
                        }
                        // `--tdd` (ADR-051 decision 10): the field argument of `Rec.Validate(F, ...)`,
                        // `Rec.SetRange(F, ...)`, `Rec.TestField(F, ...)` and the other record methods that name a
                        // field names one the table lacks: a stub field site, as `Rec.F` is. Its type comes from the
                        // value arguments (`tdd_settle_field_args`).
                        if super::tdd::field_arg_method(&member_name(callee), i)
                            && self.tdd_field_arg(sc, a, &kids[0], receiver.as_ref())
                        {
                            arg_tys.push(Ty::Unknown);
                            continue;
                        }
                    }
                    arg_tys.push(self.ty(sc, a));
                }
                if unknown_callee && self.tdd_call(sc, node, callee, &arg_tys) {
                    return E::Val(Ty::Unknown);
                }
                match target {
                    E::Callable(c) if self.tdd_apps.is_some() && self.tdd_overload(sc, node, callee, &c, &arg_tys) => {
                        E::Val(Ty::Unknown)
                    }
                    E::Callable(c) => {
                        let ret = self.pick(sc, node, &c, &arg_tys, &member_name(callee));
                        if self.tdd_apps.is_some() {
                            self.tdd_settle_field_args(sc, callee, &args, &arg_tys);
                            self.tdd_settle_args(sc, node, callee, &c, &args, &arg_tys);
                        }
                        E::Val(ret)
                    }
                    E::Val(Ty::Unknown) => E::Val(Ty::Unknown),
                    E::Val(t) => {
                        self.diag(
                            sc,
                            callee,
                            "AR1004",
                            format!("`{}` of type {t} is not callable", callee.text()),
                        );
                        E::Val(Ty::Unknown)
                    }
                    _ => E::Val(Ty::Unknown),
                }
            }
            MemberExpr => {
                // `this.Member` in a codeunit, report, page, xmlport or query: the object's own procedure or global
                // variable (Microsoft Learn "this keyword": a self-reference in every object type; no corpus test
                // outside a codeunit, docs/probes-needed.md). It
                // resolves like the bare name does, without the implicit record's members.
                if is_this_keyword(&kids[0])
                    && matches!(
                        self.ws.objects[sc.oi].kind,
                        ObjKind::Codeunit
                            | ObjKind::Report
                            | ObjKind::Page
                            | ObjKind::XmlPort
                            | ObjKind::Query
                            | ObjKind::ReportExtension
                            | ObjKind::PageExtension
                    )
                    && !sc_has_var(sc, "this")
                {
                    if let Some(e) = kids.get(1).and_then(|m| self.this_member(sc, node, m)) {
                        return e;
                    }
                }
                let base = self.infer(sc, &kids[0]);
                let mname = kids.get(1).map(name_of).unwrap_or_default();
                if let E::PartialObj(k, mut parts) = base {
                    parts.push(mname);
                    return match self.ws.resolve_ref(sc.file, k, &parts) {
                        // `Enum::Ns.Name` names the enum type, as `Enum::Name` does.
                        Resolution::Found(i) if k == ObjKind::Enum => {
                            self.touch(i);
                            let qualified = self.ws.objects[i].name.clone();
                            E::TypeName(Ty::Enum(qualified.into()))
                        }
                        Resolution::Found(i) => {
                            self.reach_named_object(sc, node, k, i);
                            let id = self.ws.objects[i].id.unwrap_or(0) as i32;
                            self.record(sc, node, Res::ObjectId(id));
                            E::Val(Ty::Integer)
                        }
                        _ => match (k, self.ws.unprovided_platform_table_id(sc.file, &parts)) {
                            (ObjKind::Table, Some(id)) => {
                                self.record(sc, node, Res::ObjectId(id));
                                E::Val(Ty::Integer)
                            }
                            _ => E::PartialObj(k, parts),
                        },
                    };
                }
                let base_ty = match base {
                    E::Val(t) => t,
                    E::Callable(c) => self.pick(sc, &kids[0], &c, &[], &member_name(&kids[0])),
                    E::TypeName(t) => {
                        // Static members of a type: `"ALT Status".FromInteger(1)`. Lowering reads
                        // the enum behind the type name from the receiver's recorded type.
                        let r = kids[0].text_range();
                        self.types
                            .insert((sc.file, r.start().into(), r.end().into()), t.clone());
                        return match self.static_members(&t, &mname) {
                            Some(c) => E::Callable(c),
                            None => E::Val(Ty::Unknown),
                        };
                    }
                    E::ObjKeyword(k) => {
                        // `Codeunit.Run(...)`, `Page.RunModal(...)`, `Database.Commit()`.
                        let recv = match k {
                            ObjKind::Table => "Database",
                            other => kind_receiver(other),
                        };
                        let c = self.catalog_cands(recv, &mname, false, &Ty::Unknown);
                        return if c.is_empty() {
                            self.diag(sc, node, "AR1002", self.no_member(&mname, &Ty::Builtin(recv.into())));
                            E::Val(Ty::Unknown)
                        } else {
                            E::Callable(c)
                        };
                    }
                    E::EnumKeyword | E::PartialObj(..) => return E::Val(Ty::Unknown),
                };
                let r = kids[0].text_range();
                self.types
                    .insert((sc.file, r.start().into(), r.end().into()), base_ty.clone());
                // `CurrPage.Update(true)` is the page's method even when the page has an action or control of
                // that name (System Application's "Schedule Feature Data Update" has an action `Update`): a
                // control is not called. Real BC compiles that code; no corpus test names a control like a method.
                if let Ty::Object(ObjKind::Page, page) = &base_ty {
                    if node.parent().is_some_and(|p| p.kind() == CallExpr)
                        && node.prev_sibling().is_none()
                        && self
                            .resolve(sc, ObjKind::Page, page)
                            .is_some_and(|oi| self.named_section(oi, &mname).is_some())
                    {
                        if let Some(recv) = receiver_of(&base_ty) {
                            let c = self.catalog_cands(&recv, &mname, true, &base_ty);
                            if !c.is_empty() {
                                return E::Callable(c);
                            }
                        }
                    }
                }
                // `Q.Open()` is the query's method even when the query has a column named Open (Base
                // Application's "Summarized Stock By Item Trkg." has one): a column is not called.
                // Compiles in real BC; no corpus test names a column like a method (docs/probes-needed.md).
                if matches!(base_ty, Ty::Object(ObjKind::Query, _))
                    && node.parent().is_some_and(|p| p.kind() == CallExpr)
                    && node.prev_sibling().is_none()
                    && self.field_res(sc, &base_ty, &mname).is_some()
                {
                    if let Some(recv) = receiver_of(&base_ty) {
                        let c = self.catalog_cands(&recv, &mname, true, &base_ty);
                        if !c.is_empty() {
                            return E::Callable(c);
                        }
                    }
                }
                if let Some(r) = self.field_res(sc, &base_ty, &mname) {
                    self.record(sc, node, r);
                }
                // `Host.OpenView.Invoke()` is the control called OpenView, not the built-in method:
                // a member that is itself the receiver of another member can only be a control
                // (corpus handlers/TestPageActionRunPageMode.al: actions named OpenView and OpenEdit).
                // `Part.Page` is the page the part shows, also when that page has a control called Page (Base
                // Application's Purch. Invoice Subform has a group "Page"): real BC compiles
                // `CurrPage.PurchLines.Page.ForceTotalsCalculation()`; no corpus test names a control Page.
                if matches!(base_ty, Ty::TestPage(_) | Ty::TestRequestPage(_) | Ty::TestPart(_))
                    && !(matches!(base_ty, Ty::TestPart(_)) && mname.eq_ignore_ascii_case("Page"))
                    && node
                        .parent()
                        .is_some_and(|p| p.kind() == MemberExpr && p.children().next().as_ref() == Some(node))
                {
                    if let Some(e) = self.page_control_member(sc, &base_ty, &mname) {
                        return e;
                    }
                }
                self.al_codeunit_access(sc, node, &base_ty, &mname);
                match self.member(sc, &base_ty, &mname) {
                    Some(e) => e,
                    None if self.tdd_member(sc, node, &kids[0], &base_ty, &mname) => E::Val(Ty::Unknown),
                    None if self.tdd_field(sc, node, &kids[0], &base_ty, &mname) => E::Val(Ty::Unknown),
                    None => {
                        self.diag(sc, node, "AR1002", self.no_member(&mname, &base_ty));
                        self.al_unknown_record_member(sc, node, &base_ty, &mname);
                        self.al_unknown_builtin_member(sc, node, &base_ty, &mname);
                        E::Val(Ty::Unknown)
                    }
                }
            }
            ScopeExpr => {
                let rhs = kids.get(1).map(name_of).unwrap_or_default();
                let base = if kids[0].kind() == NameRef {
                    match self.lookup_name(sc, &kids[0]) {
                        Some(e) => e,
                        None => {
                            // `"Some Enum"::Value` where the enum is outside the workspace.
                            self.diag(sc, node, "AR0002", format!("unresolved enum `{}`", name_of(&kids[0])));
                            return E::Val(Ty::Unknown);
                        }
                    }
                } else if let Some(e) = self.namespaced_enum(sc, &kids[0]) {
                    // `Ns.Sub."Enum"::Value`: the enum named through its namespace.
                    e
                } else {
                    self.infer(sc, &kids[0])
                };
                match base {
                    E::ObjKeyword(k) => match self.resolve_quiet(sc.file, k, &rhs) {
                        Some(i) => {
                            // `Database::"X"`, `Page::"X"`: the object's id. A codeunit named so can run.
                            self.reach_named_object(sc, node, k, i);
                            let id = self.ws.objects[i].id.unwrap_or(0) as i32;
                            self.record(sc, node, Res::ObjectId(id));
                            E::Val(Ty::Integer)
                        }
                        None => match (k, self.ws.unprovided_platform_table_id(sc.file, &split_qualified(&rhs))) {
                            // A platform table nothing provides: its id is still a constant.
                            (ObjKind::Table, Some(id)) => {
                                self.record(sc, node, Res::ObjectId(id));
                                E::Val(Ty::Integer)
                            }
                            _ => E::PartialObj(k, vec![rhs]),
                        },
                    },
                    E::PartialObj(k, parts) => E::PartialObj(k, parts),
                    E::EnumKeyword => match self.resolve(sc, ObjKind::Enum, &rhs) {
                        Some(i) => E::TypeName(Ty::Enum(self.ws.objects[i].name.as_str().into())),
                        // `Enum::Acme.Docs."Kind"`: the first segment is a namespace; the next
                        // `.` step resolves the whole name (the member branch above).
                        None if node.parent().is_some_and(|p| p.kind() == MemberExpr) => {
                            E::PartialObj(ObjKind::Enum, vec![rhs])
                        }
                        None => {
                            self.diag(sc, node, "AR0002", format!("unresolved enum `{rhs}`"));
                            E::TypeName(Ty::Unknown)
                        }
                    },
                    E::TypeName(t) | E::Val(t) => {
                        // `Kinds::Purchase` with `Kinds: array[3] of Option ...` names a member of
                        // the element's option (Microsoft's SCM Supply Planning tests).
                        // The member is then a plain Integer, not an Option: `Format(Kinds::Released)` is `1`
                        // (misc/lt-probes-needed-3/PnC14OptionArray.al:OptionArray_MemberOnArrayName).
                        let (t, via_array) = match t {
                            Ty::Array(_, el) if matches!(*el, Ty::Option(_)) => (*el, true),
                            t => (t, false),
                        };
                        // `Mode::Decompress` on a `DotNet CompressionMode` variable: a static member of the
                        // .NET type (a .NET enum's member), read like `Mode.Decompress`.
                        if matches!(&t, Ty::DotNet(alias) if !alias.is_empty()) {
                            self.record(sc, node, Res::DotNetMember(rhs.clone()));
                        }
                        self.scope_member(sc, node, &t, &rhs);
                        E::Val(if via_array { Ty::Integer } else { t })
                    }
                    // `Rec.ReadIsolation::ReadUncommitted`: the property name is also the way to name the
                    // members of the option it takes (Base Application writes it).
                    E::Callable(_) if system_option_of_property(&kids[0], &rhs).is_some() => {
                        let (enum_name, i) = system_option_of_property(&kids[0], &rhs).expect("checked");
                        self.record(sc, node, Res::OptionValue(i as i32));
                        E::Val(Ty::Builtin(enum_name.into()))
                    }
                    // `RecRef.SecurityFiltering::Ignored`: the member of the option a call answers
                    // (Base Application's "Find Record Management"). Its type is the option's.
                    E::Callable(c) => {
                        let t = self.pick(sc, &kids[0], &c, &[], &rhs);
                        self.scope_member(sc, node, &t, &rhs);
                        E::Val(t)
                    }
                }
            }
            _ => E::Val(Ty::Unknown),
        }
    }

    /// `Ns.Sub."Enum"` before `::`: a chain of names that resolves to an enum through its namespace.
    pub(crate) fn namespaced_enum(&self, sc: &Scope, node: &SyntaxNode) -> Option<E> {
        fn parts(n: &SyntaxNode, out: &mut Vec<String>) -> bool {
            match n.kind() {
                SyntaxKind::NameRef => {
                    out.push(name_of(n));
                    true
                }
                SyntaxKind::MemberExpr => {
                    let mut kids = n.children();
                    let (Some(base), Some(member), None) = (kids.next(), kids.next(), kids.next()) else {
                        return false;
                    };
                    parts(&base, out) && {
                        out.push(name_of(&member));
                        true
                    }
                }
                _ => false,
            }
        }
        if node.kind() != SyntaxKind::MemberExpr {
            return None;
        }
        let mut names = Vec::new();
        if !parts(node, &mut names) {
            return None;
        }
        match self.ws.resolve_ref(sc.file, ObjKind::Enum, &names) {
            Resolution::Found(i) => {
                self.touch(i);
                Some(E::TypeName(Ty::Enum(self.ws.objects[i].name.as_str().into())))
            }
            _ => None,
        }
    }

    /// `T::Member` where `T` is the type of the left side: records the enum value or option member.
    pub(crate) fn scope_member(&mut self, sc: &mut Scope, node: &SyntaxNode, t: &Ty, rhs: &str) {
        match t {
            Ty::Enum(name) => {
                if let Some(ei) = self.resolve(sc, ObjKind::Enum, name) {
                    if let Some((v, _)) = self.members[ei]
                        .enum_values
                        .iter()
                        .find(|(_, n)| n.eq_ignore_ascii_case(rhs))
                    {
                        let value = *v as i32;
                        self.record(sc, node, Res::EnumValue { object: ei, value });
                    } else if self.tdd_enum_value(sc, node, ei, rhs) {
                        // `--tdd` (ADR-051 decision 10): the next compile declares the value.
                    } else if !self.ws.is_dependency(sc.oi) && !self.ws.is_dependency(ei) {
                        // AL0132 (corpus must-not-compile 0204): a value the enum, with the values of its
                        // extensions, does not have. Only an enum of the workspace is judged.
                        let shown = crate::alcodes::quoted_if_needed(&self.ws.objects[ei].name);
                        self.diag(
                            sc,
                            node,
                            "AL0132",
                            format!("'{shown}' does not contain a definition for '{rhs}'"),
                        );
                    }
                }
            }
            Ty::Option(members) => {
                if let Some(i) = members.iter().position(|m| m.eq_ignore_ascii_case(rhs)) {
                    self.record(sc, node, Res::OptionValue(i as i32));
                }
            }
            // `FieldClass::Normal`: platform option types.
            Ty::Builtin(n) => {
                if let Some(i) =
                    al_builtins::sysenum::members(n).and_then(|m| m.iter().position(|m| m.eq_ignore_ascii_case(rhs)))
                {
                    self.record(sc, node, Res::OptionValue(i as i32));
                }
                // AL0132 (corpus must-not-compile 0010 to 0013): members the compiler does not have.
                if !self.ws.is_dependency(sc.oi)
                    && ABSENT_ENUM_MEMBERS
                        .iter()
                        .any(|(e, ms)| e.eq_ignore_ascii_case(n) && ms.iter().any(|m| m.eq_ignore_ascii_case(rhs)))
                {
                    self.diag(
                        sc,
                        node,
                        "AL0132",
                        format!("'{n}' does not contain a definition for '{rhs}'"),
                    );
                }
            }
            _ => {}
        }
    }

    pub(crate) fn name(&mut self, sc: &mut Scope, node: &SyntaxNode) -> E {
        match self.lookup_name(sc, node) {
            Some(e) => e,
            None if self.tdd_bare(sc, node) => E::Val(Ty::Unknown),
            None if self.tdd_bare_field(sc, node) => E::Val(Ty::Unknown),
            None => {
                self.diag(sc, node, "AR1000", format!("unresolved name `{}`", name_of(node)));
                self.al_unresolved_name(sc, node);
                E::Val(Ty::Unknown)
            }
        }
    }

    /// `Codeunit::"X"`, `Page::"X"`, ...: the object's id is used. A codeunit named so can run. A test
    /// hands `Report::"X"` to a helper that runs it
    /// (handlers/TestReportDatasetHelpLink.al:RenderDataset(Report::"G/L Register")), so the call that
    /// runs it names no constant: code of the workspace that passes a report's id to a call may run it. A
    /// dependency's mention (a report id in a setup table) does not.
    pub(crate) fn reach_named_object(&self, sc: &Scope, node: &SyntaxNode, k: ObjKind, i: usize) {
        if k == ObjKind::Codeunit {
            self.reach_run(i);
        } else {
            self.touch_schema(i);
        }
        // Only as an argument of a call; `I := Report::"X"` just names the id (check/demand.rs pins that).
        let is_argument = node.parent().is_some_and(|p| p.kind() == SyntaxKind::ArgList);
        if k == ObjKind::Report && is_argument && !self.ws.is_dependency(sc.oi) {
            self.touch_full(i);
        }
    }

    /// `Page.Run(Page::"X")`, `Report.Run(Report::"X")`, `XmlPort.Import(XmlPort::"X", ...)`: the
    /// object named in the first argument runs, so its code is reached (not just its shape).
    pub(crate) fn reach_static_run(&self, sc: &Scope, callee: &SyntaxNode, args: &[SyntaxNode]) {
        if self.demand.borrow().is_none() || callee.kind() != SyntaxKind::MemberExpr {
            return;
        }
        let mut kids = callee.children();
        let (Some(base), Some(_)) = (kids.next(), kids.next()) else {
            return;
        };
        let kind = match base.text().to_string().trim().to_ascii_lowercase().as_str() {
            "page" => ObjKind::Page,
            "report" => ObjKind::Report,
            "xmlport" => ObjKind::XmlPort,
            _ => return,
        };
        let Some(first) = args.first().filter(|a| a.kind() == SyntaxKind::ScopeExpr) else {
            return;
        };
        // `Report::"X"`, `Report::Ns."X"`: the object's name in the form types carry.
        let mut kids = first.children();
        let (Some(lhs), Some(rhs)) = (kids.next(), kids.next()) else {
            return;
        };
        if !lhs
            .text()
            .to_string()
            .trim()
            .eq_ignore_ascii_case(base.text().to_string().trim())
        {
            return;
        }
        fn parts(n: &SyntaxNode, out: &mut Vec<String>) {
            if n.kind() == SyntaxKind::MemberExpr {
                for k in n.children() {
                    parts(&k, out);
                }
            } else {
                out.push(name_of(n));
            }
        }
        let mut p = Vec::new();
        parts(&rhs, &mut p);
        let name = p.join(crate::types::QUAL_SEP);
        if let Some(oi) = self.resolve_at(sc.file, kind, &name) {
            self.touch_full(oi);
        }
    }
}

/// A type of an object as the compiler shows it: `Codeunit "X"`, where `Ty` shows `codeunit "X"`.
fn capitalized_kind(t: &Ty) -> String {
    let s = t.to_string();
    let mut c = s.chars();
    c.next()
        .map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str())
}
