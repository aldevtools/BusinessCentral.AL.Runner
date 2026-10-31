//! Lowering of the arguments and members of `DotNet` values (ADR-014).
use super::*;

impl Lowerer<'_> {
    /// An argument of a DotNet member: a plain variable goes by place, so a by-reference .NET
    /// parameter can write it back (ADR-014 section 4); anything else by value.
    pub(crate) fn dotnet_arg(&mut self, a: &SyntaxNode) -> L<ir::Arg> {
        let is_label = match self.res(a) {
            Some(Res::Local(n)) => self.local_labels.contains_key(n),
            Some(Res::Global(n)) => self.global_labels.contains_key(n),
            _ => false,
        };
        let variable = matches!(self.res(a), Some(Res::Local(_) | Res::Global(_) | Res::Field { .. }));
        if !is_label
            && variable
            && matches!(
                a.kind(),
                SyntaxKind::NameRef | SyntaxKind::MemberExpr | SyntaxKind::IndexExpr
            )
        {
            if let Ok(p) = self.place(a) {
                return Ok(ir::Arg::Ref(p));
            }
        }
        Ok(ir::Arg::Value(self.expr(a)?))
    }

    /// A member of a DotNet value (ADR-014): `callee` is the member expression, `op` says what to
    /// do to it. A call named like the receiver's alias is the constructor.
    pub(crate) fn dotnet_member(
        &mut self,
        _node: &SyntaxNode,
        callee: &SyntaxNode,
        name: &str,
        op: ir::DotNetOp,
        args: Vec<ir::Arg>,
    ) -> L<ir::Expr> {
        let recv = callee
            .children()
            .next()
            .filter(|_| callee.kind() == SyntaxKind::MemberExpr)
            .ok_or("a DotNet member without a receiver is not lowered")?;
        let Ty::DotNet(alias) = self.ty_of(&recv) else {
            return Err(format!("`{name}` is not a member of a DotNet value"));
        };
        let ty = if alias.is_empty() {
            None
        } else {
            Some(self.ws.resolve_dotnet(self.file, &alias).ok_or_else(|| {
                format!(
                    "DotNet '{}' has no dotnet {{ }} declaration in this app or its dependencies",
                    crate::types::display_name(&alias)
                )
            })?)
        };
        // A `[RunOnClient]` variable: BC runs its calls on the client and a test session has none, so the constructor, a
        // client feature's `Create` and a static call raise DotNetFromAlNotSupported
        // (onprem-dotnet/lt-bg-runonclient/BgRunOnClient.al:122-150 for a constructor and Environment.MachineName,
        // :186-209 and :226-236 for `Create` and a constructor in a local variable; lt-ac/AcDnAttributes.al:131-147 for a
        // global one). Only these members are pinned; any other member of such a variable stays refused.
        if let (Some(why), ir::DotNetOp::Call(_)) = (self.run_on_client_var(&recv), &op) {
            let simple = ty
                .map(|i| {
                    self.ws.dotnet[i]
                        .decl
                        .type_name
                        .rsplit('.')
                        .next()
                        .unwrap_or("")
                        .to_string()
                })
                .unwrap_or_default();
            let simple = simple.split('`').next().unwrap_or("").to_string();
            // The client features' static IsAvailable() answers No in a test session
            // (lt-bg-runonclient/BgRunOnClient.al:99-120, the seven types of the probe; measured on Windows).
            if name.eq_ignore_ascii_case("IsAvailable")
                && args.is_empty()
                && ty.is_some_and(|i| {
                    let t = self.ws.dotnet[i].decl.type_name.as_str();
                    t.starts_with("Microsoft.Dynamics.Nav.Client.Capabilities.")
                        || t == "Microsoft.Dynamics.Nav.Client.PageNotifier"
                })
            {
                return Ok(ir::Expr::Const(ir::Const::Boolean(false)));
            }
            let ctor = ty.is_some_and(|i| self.ws.dotnet[i].decl.alias.eq_ignore_ascii_case(name))
                || simple.eq_ignore_ascii_case(name);
            if ctor || name.eq_ignore_ascii_case("Create") || name.eq_ignore_ascii_case("MachineName") {
                // Pinned by tests/al-language-onprem-dotnet/lt-bg-runonclient/BgRunOnClient.al TY_RunOnClient_GuiAllowedGuard_CameraCreate (was assumption `run-on-client-create`).
                let key = format!("{why}: {name}");
                self.run_failures.insert(
                    key.clone(),
                    (
                        "The operation cannot complete because it uses a feature that is not supported (.NET Framework Interoperability.)".into(),
                        "DotNetFromAlNotSupported:DotNetFromAlNotSupported".into(),
                    ),
                );
                return Err(key);
            }
        }
        if let (Some(i), ir::DotNetOp::Call(_)) = (ty, &op) {
            // The constructor is named like the alias or like the CLR type: the probe app declares
            // alias `DnStringBuilder` for `System.Text.StringBuilder` and calls `Builder.StringBuilder()`
            // (tests/al-language-onprem-dotnet/TestDotNetValueKindsContracts.al, codeunit 61400).
            let decl = &self.ws.dotnet[i].decl;
            let simple = decl.type_name.rsplit('.').next().unwrap_or("");
            let simple = simple.split('`').next().unwrap_or(simple);
            if decl.alias.eq_ignore_ascii_case(name) || simple.eq_ignore_ascii_case(name) {
                return Ok(ir::Expr::DotNet {
                    ty,
                    receiver: None,
                    op: ir::DotNetOp::Construct,
                    args,
                });
            }
        }
        Ok(ir::Expr::DotNet {
            ty,
            receiver: Some(Box::new(self.expr(&recv)?)),
            op,
            args,
        })
    }

    /// The reason text when `recv` is a variable declared `[RunOnClient]`.
    pub(crate) fn run_on_client_var(&self, recv: &SyntaxNode) -> Option<String> {
        let (name, local) = match self.res(recv)? {
            Res::Local(n) => (n, true),
            Res::Global(n) => (n, false),
            _ => return None,
        };
        self.var_attr_refusal(name, local).filter(|w| w.contains("RunOnClient"))
    }

    /// `GetDotNetType(x)` (tests/al-language-onprem-dotnet/lt-dotnet-interop/DotNetType*.al). The static type of the argument
    /// says what the VM does with its value (`al-vm/src/dotnet/gettype.rs`): the second argument is that decision as text.
    pub(crate) fn get_dotnet_type_call(&mut self, args: &[SyntaxNode]) -> L<ir::Expr> {
        let [arg] = args else {
            return Err("GetDotNetType takes one argument".into());
        };
        let fixed = |clr: &str| format!("fixed:{clr}");
        let (spec, with_value) = match self.ty_of(arg) {
            // DotNetTypeScalars.al: an Option is an Int32, a Code and a Text a String, a Date and a Time a DateTime.
            Ty::Integer | Ty::Option(_) => (fixed("System.Int32"), true),
            // An AL Enum is an Int32 (lt-bb-dotnet/BbGetDotNetType.al:341), pinned for Enum "Sales Document Type" only.
            Ty::Enum(name) if name.eq_ignore_ascii_case("Sales Document Type") => (fixed("System.Int32"), true),
            Ty::Enum(name) => {
                return Err(format!(
                    "GetDotNetType of an Enum other than \"Sales Document Type\" (Enum \"{name}\") is not pinned (docs/probes-needed.md)"
                ));
            }
            // lt-bb-dotnet/BbGetDotNetType.al:352-353.
            Ty::DateFormula => (
                "error:GetDotNetType() does not support the type Microsoft.Dynamics.Nav.Runtime.NavDateFormula."
                    .to_string(),
                false,
            ),
            Ty::BigInteger => (fixed("System.Int64"), true),
            Ty::Decimal => (fixed("System.Decimal"), true),
            Ty::Boolean => (fixed("System.Boolean"), true),
            Ty::Byte => (fixed("System.Byte"), true),
            Ty::Char => (fixed("System.Char"), true),
            Ty::Text(_) | Ty::Code(_) => (fixed("System.String"), true),
            Ty::Date | Ty::Time | Ty::DateTime => (fixed("System.DateTime"), true),
            Ty::Duration => (fixed("System.TimeSpan"), true),
            Ty::Guid => (fixed("System.Guid"), true),
            // DotNetTypeRecord.al:`record_error`, `temporary_error`, `recordref_error`.
            Ty::Record { .. } => (
                "error:GetDotNetType() does not support the type <record class>.".to_string(),
                false,
            ),
            Ty::Builtin(b) if b.eq_ignore_ascii_case("RecordRef") => (
                "error:GetDotNetType() does not support the type Microsoft.Dynamics.Nav.Runtime.NavRecordRef."
                    .to_string(),
                false,
            ),
            Ty::Variant => ("variant".to_string(), true),
            Ty::DotNet(alias) => (
                format!("dotnet:{}", self.declared_clr_type(&alias).unwrap_or_default()),
                true,
            ),
            other => {
                return Err(format!(
                    "GetDotNetType of a {other} is not pinned (docs/probes-needed.md)"
                ));
            }
        };
        let value = if with_value {
            self.expr(arg)?
        } else {
            ir::Expr::Const(ir::Const::Boolean(false))
        };
        Ok(ir::Expr::Builtin {
            id: super::call::builtin_id("System.GetDotNetType#1")?,
            receiver: None,
            args: vec![
                ir::Arg::Value(value),
                ir::Arg::Value(ir::Expr::Const(ir::Const::Text(spec))),
            ],
            ret_used: true,
        })
    }
}
