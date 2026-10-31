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
                format!("DotNet '{alias}' has no dotnet {{ }} declaration in this app or its dependencies")
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
                && ty.is_some_and(|i| al_builtins::dotnet_sigs::is_client_capability(&self.ws.dotnet[i].decl.type_name))
            {
                return Ok(ir::Expr::Const(ir::Const::Boolean(false)));
            }
            let ctor = ty.is_some_and(|i| self.ws.dotnet[i].decl.alias.eq_ignore_ascii_case(name))
                || simple.eq_ignore_ascii_case(name);
            if ctor || al_builtins::dotnet_sigs::run_on_client_fails(name) {
                // Pinned by tests/al-language-onprem-dotnet/lt-bg-runonclient/BgRunOnClient.al TY_RunOnClient_GuiAllowedGuard_CameraCreate (was assumption `run-on-client-create`),
                // and for System.IO.Directory by Y16_RunOnClient_FileManagementDirectory (corpus master 92fdde19).
                let key = format!("{why}: {name}");
                self.run_failures.insert(
                    key.clone(),
                    (
                        al_builtins::dotnet_sigs::DOTNET_FROM_AL_NOT_SUPPORTED_TEXT.into(),
                        al_builtins::dotnet_sigs::DOTNET_FROM_AL_NOT_SUPPORTED_CODE.into(),
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
        if recv.kind() != SyntaxKind::NameRef {
            return None;
        }
        let (name, local) = match self.res(recv)? {
            Res::Local(n) => (n, true),
            Res::Global(n) if !self.in_request_page => (n, false),
            _ => return None,
        };
        let attrs = if local {
            self.local_var_attrs.get(name)
        } else {
            self.global_var_attrs.get(name)
        }?;
        attrs
            .iter()
            .any(|a| a == "runonclient")
            .then(|| format!("variable `{name}` has the attribute RunOnClient, which is not supported"))
    }

    /// The value of a `[RunOnClient]` variable read on the service tier: its slot, which only ever holds null (the
    /// variable is never assigned: `run_on_client_assign` raises or refuses, and `Clear` keeps it). The null goes to a
    /// service-tier variable (corpus master 92fdde19, onprem-dotnet/lt-bg-runonclient/BgRunOnClient.al:410-420 and :436,
    /// `client to server`), into a Variant (:454-466) and to a by-value parameter (:244-262, the Camera page's
    /// `IsAvailable(CameraProvider)`).
    pub(crate) fn run_on_client_value(&self, n: &SyntaxNode) -> Option<ir::Expr> {
        self.run_on_client_var(n)?;
        let place = match self.res(n)? {
            Res::Local(name) => ir::Place::Local(*self.locals.get(name)?),
            Res::Global(name) => ir::Place::Global(*self.globals.get(name)?),
            _ => return None,
        };
        Some(ir::Expr::Load(place))
    }

    /// `X := rhs` where `X` is declared `[RunOnClient]`: the right side is lowered first (a `Create()` or a constructor raises
    /// there), then the assignment raises or is refused when it runs (`ir::DotNetOp::ClientAssign`; corpus master 92fdde19,
    /// onprem-dotnet/lt-bg-runonclient/BgRunOnClient.al:393-409 and :441-447 for a service-tier StringBuilder).
    pub(crate) fn run_on_client_assign(&mut self, n: &SyntaxNode, kids: &[SyntaxNode], why: String) -> L<ir::Stmt> {
        let plain = sig_tokens(n).iter().any(|t| t.kind() == SyntaxKind::Assign);
        if !plain || self.run_on_client_var(&kids[1]).is_some() {
            self.run_refusals.insert(why.clone());
            return Err(why);
        }
        let rhs = self.expr(&kids[1])?;
        Ok(ir::Stmt::Expr(ir::Expr::DotNet {
            ty: None,
            receiver: Some(Box::new(rhs)),
            op: ir::DotNetOp::ClientAssign,
            args: Vec::new(),
        }))
    }

    /// `P := value` where `P` is a `var` DotNet parameter: it stands for a `[RunOnClient]` variable when the caller passed one
    /// (`ir::DotNetOp::ClientNull`), and then the assignment is that of the variable (`run_on_client_assign`).
    pub(crate) fn guard_var_dotnet_param(&self, place: &ir::Place, target_ty: &Ty, value: ir::Expr) -> ir::Expr {
        match place {
            ir::Place::Local(j)
                if self.var_params.get(*j).copied().unwrap_or(false) && matches!(target_ty, Ty::DotNet(_)) =>
            {
                ir::Expr::DotNet {
                    ty: None,
                    receiver: Some(Box::new(value)),
                    op: ir::DotNetOp::ClientAssign,
                    args: vec![ir::Arg::Ref(place.clone())],
                }
            }
            _ => value,
        }
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
