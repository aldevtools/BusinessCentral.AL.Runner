//! `GetDotNetType(x)` (pinned by tests/al-language-onprem-dotnet/lt-dotnet-interop/DotNetType*.al, 27/28/29).
//!
//! The result is a real `System.Type` object of the sidecar, so `FullName`, `Name`, `Equals` and `Format` are
//! the .NET members. The lowering decides, from the static type of the argument, what `args[1]` (the
//! specification text) says; this module settles it with the run-time value.
//!
//! - `fixed:<CLR type>`: an AL scalar variable (DotNetTypeScalars.al).
//! - `error:<text>`: a Record or a RecordRef, which BC refuses (DotNetTypeRecord.al:`record_error`, `recordref_error`).
//! - `variant`: a Variant; the kind it holds decides (DotNetTypeVariant.al).
//! - `dotnet:<declared CLR type>`: a DotNet variable; the object it holds decides (DotNetTypeOfDotNet.al).

use super::*;

const NOT_SUPPORTED: &str = "NotSupportedType";

impl Session {
    /// `System.GetDotNetType#1`: `args` is the argument value and the specification text.
    pub(crate) fn get_dotnet_type(&mut self, args: &mut [Value]) -> Result<Value, Signal> {
        let v = std::mem::replace(&mut args[0], Value::Null);
        let spec = match &args[1] {
            Value::Text(t) | Value::Code(t) => t.to_string(),
            other => return unsupported(format!("GetDotNetType with a specification of {}", other.describe())),
        };
        let (mode, rest) = spec.split_once(':').unwrap_or((spec.as_str(), ""));
        match mode {
            "fixed" => self.type_named(rest),
            "error" => al_builtins::native::raise(rest, NOT_SUPPORTED),
            "variant" => self.type_of_variant(v),
            "dotnet" => self.type_of_dotnet(v, rest),
            _ => unsupported(format!("GetDotNetType with the specification '{spec}'")),
        }
    }

    /// The `System.Type` of a Variant (DotNetTypeVariant.al, DotNetTypeRecord.al:`variant_record_error`,
    /// DotNetTypeOfDotNet.al:`variant_holding_dotnet_error`). A BigInteger in a Variant is a Decimal.
    fn type_of_variant(&mut self, v: Value) -> Result<Value, Signal> {
        let name = match &v {
            // An empty Variant (DotNetTypeVariant.al:`Empty`).
            Value::Null => "System.Object",
            Value::Integer(_) => "System.Int32",
            Value::BigInteger(_) | Value::Decimal(_) => "System.Decimal",
            Value::Boolean(_) => "System.Boolean",
            Value::Text(_) | Value::Code(_) => "System.String",
            Value::Date(_, false) | Value::DateTime(_) => "System.DateTime",
            Value::Guid(_) => "System.Guid",
            Value::Duration(_) => "System.TimeSpan",
            // BbGetDotNetType.al:287-332: a Byte and a Char are a Byte, a Time and a closing Date a DateTime, an Option
            // and an Enum an Int32.
            Value::Byte(_) | Value::Char(_) => "System.Byte",
            Value::Time(_) | Value::Date(_, true) => "System.DateTime",
            Value::Option(..) | Value::Enum(..) => "System.Int32",
            Value::Record(_) | Value::DotNet(_) => {
                return al_builtins::native::raise(
                    "GetDotNetType() does not support the type Microsoft.Dynamics.Nav.Runtime.NavVariant.",
                    NOT_SUPPORTED,
                );
            }
            other => {
                return unsupported(format!(
                    "GetDotNetType of a Variant that holds {} (not pinned)",
                    other.describe()
                ));
            }
        };
        self.type_named(name)
    }

    /// The `System.Type` of what a DotNet variable holds, which is `GetType()` of it
    /// (DotNetTypeOfDotNet.al:`equals_gettype=Yes`). `declared` is the CLR type the variable is declared as.
    fn type_of_dotnet(&mut self, v: Value, declared: &str) -> Result<Value, Signal> {
        match &v {
            Value::DotNet(o) => match &o.kind {
                DotNetKind::Host { handle, clr_type } => self.type_call(DotNetTarget::Instance(*handle), clr_type),
                DotNetKind::Enum {
                    clr_type,
                    name,
                    ordinal,
                } => {
                    let wire = DotNetValue::Enum {
                        ty: clr_type.clone(),
                        name: name.clone(),
                        ordinal: *ordinal,
                    };
                    self.type_call(DotNetTarget::Value(wire), clr_type)
                }
                DotNetKind::Internal { type_name, .. } => {
                    unsupported(format!("GetDotNetType of a runtime-provided {type_name}"))
                }
            },
            Value::Null => self.type_of_unassigned(declared),
            // A primitive the variable holds (a `DotNet String` that got a Text).
            other => match boxed_clr_type(other) {
                Some(name) => self.type_named(name),
                None => unsupported(format!(
                    "GetDotNetType of a DotNet variable that holds {}",
                    other.describe()
                )),
            },
        }
    }

    /// A DotNet variable that holds nothing (tests/al-language-onprem-dotnet/lt-bb-dotnet/BbGetDotNetType.al:233-278, 364, 373):
    /// a `System.String` answers System.Object; `System.Object`, `System.Char`, `System.Byte`, `System.Int32`,
    /// `System.Text.StringBuilder`, `System.Type` and `System.Exception` answer their own type. Other declared types are not pinned.
    /// (`X.GetType()` of the unassigned Object then raises: `dotnet_apply`.)
    fn type_of_unassigned(&mut self, declared: &str) -> Result<Value, Signal> {
        match declared {
            "System.String" => self.type_named("System.Object"),
            "System.Object"
            | "System.Char"
            | "System.Byte"
            | "System.Int32"
            | "System.Text.StringBuilder"
            | "System.Type"
            | "System.Exception" => self.type_named(declared),
            "" => unsupported("GetDotNetType of an unassigned DotNet value whose declared type is not known"),
            other => unsupported(format!(
                "GetDotNetType of an unassigned DotNet variable declared as {other} (not pinned; docs/probes-needed.md)"
            )),
        }
    }

    /// `System.Type.GetType(name)` for a type of mscorlib.
    fn type_named(&mut self, name: &str) -> Result<Value, Signal> {
        self.dotnet_ready("System.Type")?;
        let decl = DotNetType {
            assembly: "mscorlib".into(),
            version: None,
            culture: None,
            public_key_token: None,
            name: "System.Type".into(),
        };
        let arg = DotNetArg {
            value: DotNetValue::Str(name.to_string()),
            by_ref: false,
        };
        let session = self.dotnet.session.as_mut().expect("opened by dotnet_ready");
        let reply = session.invoke(DotNetTarget::Static(decl), "GetType", vec![arg]);
        self.type_reply(reply, "System.Type.GetType")
    }

    /// `GetType()` on an object the host holds, or on an enum value it boxes.
    fn type_call(&mut self, target: DotNetTarget, clr_type: &str) -> Result<Value, Signal> {
        self.dotnet_ready(clr_type)?;
        let session = self.dotnet.session.as_mut().expect("opened by dotnet_ready");
        let reply = session.invoke(target, "GetType", Vec::new());
        self.type_reply(reply, &format!("{clr_type}.GetType"))
    }

    pub(super) fn type_reply(
        &mut self,
        reply: Result<DotNetReply, DotNetFailure>,
        shown: &str,
    ) -> Result<Value, Signal> {
        match reply {
            Ok(r) => {
                self.dotnet.null_string = matches!(r.value, DotNetValue::NullString);
                self.wire_to_value(r.value).map_err(|e| prefix(e, shown))
            }
            Err(DotNetFailure::Refused(why)) => unsupported(format!("DotNet {shown}: {why}")),
            Err(DotNetFailure::Exception(e)) => unsupported(format!(
                "DotNet {shown} raised the .NET exception {}: {}; the text AL would see is not pinned (ADR-014 section 7)",
                e.clr_type, e.message
            )),
        }
    }
}
