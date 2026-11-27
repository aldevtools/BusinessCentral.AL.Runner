//! A `DotNet` variable that holds no object, passed where its declared type is a .NET value type (al-runner issue 8268;
//! assumption `dotnet-null-value-type-argument`, corpus pull request 1623,
//! tests/al-language-onprem-dotnet/dotnet/dotnet-null-valuetype/N8268DotNetNullValueType.al). In the measured method calls
//! the null goes to .NET as the type's default value; in the measured constructors BC raises `DotNetCreate:NullReference`.
//! Every other call with a null argument declared as one of the measured types, or as a value type the probe left out, is
//! refused. A null of any other declared type keeps main's handling. Entry point: `Session::null_value_type_args`.

use al_builtins::assumptions::{Assumption, assume};
use al_builtins::value::{DotNetKind, RtType, Value};
use al_ir as ir;

use super::{MemberKind, unsupported};
use crate::{Frame, Session, Signal};

/// The declared types the probe measured.
const MEASURED: &[&str] = &[
    "System.Byte",
    "System.Int32",
    "System.Boolean",
    "System.DateTime",
    "System.TimeSpan",
    // Measured in the Regex constructor only (it raises there); no method call takes it.
    REGEX_OPTIONS,
];

/// The default value a null of a measured type goes to .NET as in a method call: `DateTime(1)` is .NET's `DateTime.MinValue`
/// (ticks 0, `value_to_wire`).
fn default_of(t: &str) -> Option<Value> {
    Some(match t {
        "System.Byte" => Value::Byte(0),
        "System.Int32" => Value::Integer(0),
        "System.Boolean" => Value::Boolean(false),
        "System.DateTime" => Value::DateTime(1),
        "System.TimeSpan" => Value::Duration(0),
        _ => return None,
    })
}

/// Value types the probe did not measure: a null declared as one of them is refused.
const UNMEASURED: &[&str] = &[
    "System.Decimal",
    "System.Int64",
    "System.Guid",
    "System.Char",
    "System.Double",
    "System.Single",
    "System.Int16",
    "System.UInt16",
    "System.UInt32",
    "System.UInt64",
    "System.SByte",
    "System.DateTimeOffset",
];

/// One argument of a measured call: a null declared as this type, or a value of this AL kind.
#[derive(Clone, Copy)]
enum A {
    Null(&'static str),
    Int,
    Text,
    DateTime,
    DotNetEnum(&'static str),
    TimeSpan,
}

const REGEX: &str = "System.Text.RegularExpressions.Regex";
const REGEX_OPTIONS: &str = "System.Text.RegularExpressions.RegexOptions";

/// The measured method calls (type, member, arguments) and the probe line of each.
const METHODS: &[(&str, &str, &[A])] = &[
    ("System.Convert", "ToString", &[A::Null("System.Byte"), A::Int]), // :167
    ("System.Convert", "ToString", &[A::Null("System.Int32"), A::Int]), // :194
    ("System.Convert", "ToInt32", &[A::Null("System.Boolean")]),       // :211
    ("System.DateTime", "Compare", &[A::Null("System.DateTime"), A::DateTime]), // :227
    ("System.DateTime", "Add", &[A::Null("System.TimeSpan")]),         // :243
    ("System.TimeSpan", "Add", &[A::Null("System.TimeSpan")]),         // :259
    (
        "System.Text.StringBuilder",
        "Remove",
        &[A::Null("System.Int32"), A::Int],
    ), // :275
];

/// The measured constructors (type, arguments) and the probe line of each.
const CONSTRUCTORS: &[(&str, &[A])] = &[
    ("System.DateTimeOffset", &[A::DateTime, A::Null("System.TimeSpan")]), // :48
    (
        REGEX,
        &[A::Text, A::DotNetEnum(REGEX_OPTIONS), A::Null("System.TimeSpan")],
    ), // :86
    (REGEX, &[A::Text, A::Null(REGEX_OPTIONS), A::TimeSpan]),              // :106
    (REGEX, &[A::Text, A::Null(REGEX_OPTIONS), A::Null("System.TimeSpan")]), // :129
    ("System.Text.StringBuilder", &[A::Null("System.Int32"), A::Int]),     // :293
];

/// A null argument: its declared DotNet type, and whether it sits where the probe measured one: a local variable or a
/// by-value parameter (N8268DotNetNullValueType.al passes the null both ways).
struct NullArg {
    ty: String,
    measured_place: bool,
}

impl Session {
    /// The null argument `i`, when it is a null held by a variable or parameter declared as a DotNet type.
    fn null_arg(&self, values: &[Value], places: &[Option<ir::Place>], i: usize, f: &Frame) -> Option<NullArg> {
        if !matches!(values[i], Value::Null) {
            return None;
        }
        let p = places.get(i)?.as_ref()?;
        let RtType::DotNet(t) = self.place_type(p, f) else {
            return None;
        };
        let r = &self.engine.program.objects[f.object].routines[f.routine];
        let measured_place = match p {
            ir::Place::Local(l) => *l >= r.params || r.by_var.get(*l) == Some(&false),
            _ => false,
        };
        Some(NullArg {
            ty: self.engine.program.dotnet_types[t as usize].type_name.clone(),
            measured_place,
        })
    }

    /// See the module header: puts the default values in place of the nulls of a measured method call. `Err` raises the
    /// constructor error or refuses; a call with no null of a listed value type is left as it is.
    pub(super) fn null_value_type_args(
        &mut self,
        kind: MemberKind,
        clr_type: &str,
        member: &str,
        values: &mut [Value],
        places: &[Option<ir::Place>],
        f: &Frame,
    ) -> Result<(), Signal> {
        let nulls: Vec<Option<NullArg>> = (0..values.len()).map(|i| self.null_arg(values, places, i, f)).collect();
        if let Some(n) = nulls.iter().flatten().find(|n| UNMEASURED.contains(&n.ty.as_str())) {
            return unsupported(format!("DotNet {member}: a null {} argument (not measured)", n.ty));
        }
        if !nulls.iter().flatten().any(|n| MEASURED.contains(&n.ty.as_str())) {
            return Ok(());
        }
        let fits = |pattern: &[A]| {
            pattern.len() == values.len()
                && pattern
                    .iter()
                    .zip(values.iter())
                    .zip(&nulls)
                    .all(|((a, v), n)| match *a {
                        A::Null(t) => n.as_ref().is_some_and(|n| n.measured_place && n.ty == t),
                        A::Int => matches!(v, Value::Integer(_)),
                        A::Text => matches!(v, Value::Text(_)),
                        A::DateTime => matches!(v, Value::DateTime(_)),
                        A::TimeSpan => matches!(v, Value::Duration(_)),
                        A::DotNetEnum(t) => matches!(v, Value::DotNet(o)
                        if matches!(&o.kind, DotNetKind::Enum { clr_type, .. } if clr_type == t)),
                    })
        };
        let method = kind == MemberKind::Call
            && METHODS
                .iter()
                .any(|(t, m, p)| *t == clr_type && *m == member && fits(p));
        let major = self.engine.target.as_ref().map(|t| t.bc.major);
        let construct = kind == MemberKind::Construct
            && matches!(major, Some(27..=29))
            && CONSTRUCTORS.iter().any(|(t, p)| *t == clr_type && fits(p));
        if method {
            assume(Assumption::DotnetNullValueTypeArgument);
            for (v, n) in values.iter_mut().zip(&nulls) {
                if let Some(d) = n.as_ref().and_then(|n| default_of(&n.ty)) {
                    *v = d;
                }
            }
            return Ok(());
        }
        if !construct {
            return unsupported(format!(
                "DotNet {member} of {clr_type}: a null argument declared as a .NET value type in a call or place the probe did not measure"
            ));
        }
        assume(Assumption::DotnetNullValueTypeArgument);
        // The texts of N8268DotNetNullValueType.al:52-56 (BC 28PLUS / BC 27), :90-94 for Regex.
        let assembly = match (major == Some(27), clr_type == REGEX) {
            (false, false) => "mscorlib, Version=4.0.0.0, Culture=neutral, PublicKeyToken=b77a5c561934e089",
            (false, true) => "System, Version=4.0.0.0, Culture=neutral, PublicKeyToken=b77a5c561934e089",
            (true, false) => "System.Runtime, Version=10.0.0.0, Culture=neutral, PublicKeyToken=b03f5f7f11d50a3a",
            (true, true) => {
                "System.Text.RegularExpressions, Version=10.0.0.0, Culture=neutral, PublicKeyToken=b03f5f7f11d50a3a"
            }
        };
        al_builtins::native::raise(
            format!(
                "Cannot create an instance of the following .NET Framework object: assembly {assembly}, type {clr_type}."
            ),
            "DotNetCreate:NullReference",
        )
    }
}
