//! A `DotNet` value as the argument of a builtin parameter (ADR-024 section 2): which parameter types take one, and how.

use super::*;

impl Lowerer<'_> {
    /// The argument expression for parameter `p` of builtin `id` from `a`, whose type is the DotNet `alias` (empty for a result
    /// with no static type). `receiver_ty` is the type of the receiver of a method call.
    pub(super) fn dotnet_builtin_arg(
        &mut self,
        id: &str,
        p: &al_builtins::signature::Param,
        a: &SyntaxNode,
        alias: &str,
        receiver_ty: Option<Ty>,
    ) -> L<ir::Expr> {
        use super::call::element_param_name;
        // `IsNull(DotNet)` takes the value as it is. A `Variant` parameter takes a
        // primitive, an object or an enum as it is (`Format` of an object is its
        // `ToString()`). A scalar parameter takes a DotNet result (no static type)
        // when its run-time kind fits the parameter, as in an assignment (ADR-024
        // section 2). Anything else is refused.
        // A collection method's element parameter (`Dictionary.Add(Key: TKey, Value: TValue)`,
        // `List.Add(Value: T)`) is the element type of the receiver's declared collection.
        let pty: String = match (p.ty.as_str(), receiver_ty) {
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
            // A .NET String into the `String` overload of ErrorInfo.Create is the message
            // (tests/al-language-onprem-dotnet/lt-y51-dotnet-builtin-args/Y51DotNetBuiltinArgs.al:DotNetString_ErrorInfoCreate).
            "String" if id.starts_with("ErrorInfo.Create#") => Some(RtType::Text(None)),
            _ => None,
        };
        let value = match pty.as_str() {
            "DotNet" => self.expr(a)?,
            // CopyStream takes a DotNet stream as it is; the VM copies the bytes
            // through the host (ADR-024 section 3).
            "InStream" | "OutStream" if !p.by_var && id.starts_with("System.CopyStream#") => self.expr(a)?,
            // A MemoryStream into the InStream of these builtins: the VM reads the whole stream through the host
            // (al-vm dotnet/instream_arg.rs; Y51DotNetBuiltinArgs.al).
            "InStream"
                if !p.by_var
                    && matches!(
                        id,
                        "Media.ImportStream#1"
                            | "XmlDocument.ReadFrom#1"
                            | "XmlDocument.ReadFrom#2"
                            | "JsonObject.ReadFrom#1"
                    ) =>
            {
                self.expr(a)?
            }
            "Any" | "Variant" if !p.by_var => ir::Expr::DotNetOperand(Box::new(self.expr(a)?), ir::OperandUse::Variant),
            _ => match scalar {
                Some(rt) if !p.by_var && alias.is_empty() => ir::Expr::DotNetCoerce(
                    rt,
                    Box::new(ir::Expr::DotNetOperand(Box::new(self.expr(a)?), ir::OperandUse::Strict)),
                ),
                // A variable declared `DotNet String` into a Text or Code slot: it reads as a Text and
                // nothing as '' (tests/al-language-onprem-dotnet/lt-dotnet-interop/DotNetOperandStringEq.al),
                // then fits the slot like a DotNet result. System Application Uri.GetSegments adds one to a
                // List of [Text] (al-runner issue 6298).
                Some(rt @ RtType::Text(None))
                    if !p.by_var
                        && matches!(pty.as_str(), "Text" | "String")
                        && self.declared_clr_type(alias).as_deref() == Some("System.String") =>
                {
                    ir::Expr::DotNetCoerce(rt, Box::new(self.operand(a, ir::OperandUse::Strict)?))
                }
                _ => {
                    return Err(format!(
                        "a DotNet value passed to builtin {id}: its type is only known at run time; assign it to a typed variable first"
                    ));
                }
            },
        };
        Ok(value)
    }
}
