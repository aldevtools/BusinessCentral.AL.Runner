//! A `DotNet` variable that holds no object, passed where its declared type is a .NET value type (al-runner issue 8268;
//! assumption `dotnet-null-value-type-argument`, corpus pull request 1623,
//! tests/al-language-onprem-dotnet/dotnet/dotnet-null-valuetype/N8268DotNetNullValueType.al). In the measured method calls
//! the null goes to .NET as the type's default value; in the measured constructors BC raises `DotNetCreate:NullReference`.
//! Each call is served only on the measured BC majors and with its null in the place the probe passed it (`Where`). Every
//! other call or place with a null argument declared as one of the measured types, or as a value type the probe left out,
//! is refused. A null of any other declared type keeps main's handling. Entry point: `Session::null_value_type_args`.

use al_builtins::assumptions::{Assumption, assume};
use al_builtins::value::{DotNetKind, RtType, Value};
use al_ir as ir;

use super::{MemberKind, unsupported};
use crate::{Frame, Session, Signal};

/// A declared type the probe measured: one with a default below, or `RegexOptions`, measured in the Regex constructor
/// only (it raises there; no method call takes it).
fn is_measured(t: &str) -> bool {
    default_of(t).is_some() || t == REGEX_OPTIONS
}

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

/// One argument of a measured call: a null declared as this type, an Integer or Text value, or a value held by a variable
/// declared as this DotNet type (an enum member for `RegexOptions`).
#[derive(Clone, Copy)]
enum A {
    Null(&'static str),
    Int,
    Text,
    DotNet(&'static str),
}

/// Where the probe passed the null of a call: a local variable of a routine that is no `[TryFunction]` (the test itself), or
/// a by-value parameter of a `[TryFunction]` (N8268DotNetNullValueType.al:337-407).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Where {
    Direct,
    TryParam,
}

const DATE_TIME: &str = "System.DateTime";
const TIME_SPAN: &str = "System.TimeSpan";

const REGEX: &str = "System.Text.RegularExpressions.Regex";
const REGEX_OPTIONS: &str = "System.Text.RegularExpressions.RegexOptions";

/// The measured method calls (type, member, place of the null, arguments) and the probe line of each.
const METHODS: &[(&str, &str, Where, &[A])] = &[
    (
        "System.Convert",
        "ToString",
        Where::Direct,
        &[A::Null("System.Byte"), A::Int],
    ), // :167
    (
        "System.Convert",
        "ToString",
        Where::TryParam,
        &[A::Null("System.Int32"), A::Int],
    ), // :194, :337
    (
        "System.Convert",
        "ToInt32",
        Where::TryParam,
        &[A::Null("System.Boolean")],
    ), // :211, :346
    (
        DATE_TIME,
        "Compare",
        Where::TryParam,
        &[A::Null(DATE_TIME), A::DotNet(DATE_TIME)],
    ), // :227, :355
    (DATE_TIME, "Add", Where::TryParam, &[A::Null(TIME_SPAN)]), // :243, :365
    (TIME_SPAN, "Add", Where::TryParam, &[A::Null(TIME_SPAN)]), // :259, :377
    (
        "System.Text.StringBuilder",
        "Remove",
        Where::TryParam,
        &[A::Null("System.Int32"), A::Int],
    ), // :275, :389
];

/// The measured constructors (type, place of the nulls, arguments) and the probe line of each.
const CONSTRUCTORS: &[(&str, Where, &[A])] = &[
    (
        "System.DateTimeOffset",
        Where::Direct,
        &[A::DotNet(DATE_TIME), A::Null(TIME_SPAN)],
    ), // :48
    (
        REGEX,
        Where::Direct,
        &[A::Text, A::DotNet(REGEX_OPTIONS), A::Null(TIME_SPAN)],
    ), // :86
    (
        REGEX,
        Where::TryParam,
        &[A::Text, A::Null(REGEX_OPTIONS), A::DotNet(TIME_SPAN)],
    ), // :106, :329
    (
        REGEX,
        Where::TryParam,
        &[A::Text, A::Null(REGEX_OPTIONS), A::Null(TIME_SPAN)],
    ), // :129, :329
    (
        "System.Text.StringBuilder",
        Where::TryParam,
        &[A::Null("System.Int32"), A::Int],
    ), // :293, :400
];

/// A null argument: its declared DotNet type, and the measured place it sits in, if any.
struct NullArg {
    ty: String,
    place: Option<Where>,
}

impl Session {
    /// The declared DotNet type of the variable or parameter argument `i` is held by, if it is one.
    fn dotnet_type_of(&self, places: &[Option<ir::Place>], i: usize, f: &Frame) -> Option<&str> {
        let RtType::DotNet(t) = self.place_type(places.get(i)?.as_ref()?, f) else {
            return None;
        };
        Some(self.engine.program.dotnet_types[t as usize].type_name.as_str())
    }

    /// The null argument `i`, when it is a null held by a variable or parameter declared as a DotNet type.
    fn null_arg(&self, values: &[Value], places: &[Option<ir::Place>], i: usize, f: &Frame) -> Option<NullArg> {
        if !matches!(values[i], Value::Null) {
            return None;
        }
        let ty = self.dotnet_type_of(places, i, f)?.to_string();
        let r = &self.engine.program.objects[f.object].routines[f.routine];
        let is_try = self.engine.routine_facts(f.object, f.routine).is_try;
        let place = match (&places[i], is_try) {
            (Some(ir::Place::Local(l)), false) if *l >= r.params => Some(Where::Direct),
            (Some(ir::Place::Local(l)), true) if *l < r.params && r.by_var.get(*l) == Some(&false) => {
                Some(Where::TryParam)
            }
            _ => None,
        };
        Some(NullArg { ty, place })
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
        if !nulls.iter().flatten().any(|n| is_measured(&n.ty)) {
            return Ok(());
        }
        let fits = |place: Where, pattern: &[A]| {
            pattern.len() == values.len()
                && (0..values.len()).all(|i| match pattern[i] {
                    A::Null(t) => nulls[i].as_ref().is_some_and(|n| n.place == Some(place) && n.ty == t),
                    A::Int => matches!(values[i], Value::Integer(_)),
                    A::Text => matches!(values[i], Value::Text(_)),
                    A::DotNet(t) => {
                        self.dotnet_type_of(places, i, f) == Some(t)
                            && match &values[i] {
                                Value::DateTime(_) => t == DATE_TIME,
                                Value::Duration(_) => t == TIME_SPAN,
                                Value::DotNet(o) => {
                                    matches!(&o.kind, DotNetKind::Enum { clr_type, .. } if clr_type == t)
                                }
                                _ => false,
                            }
                    }
                })
        };
        // The probe ran on the corpus CI legs of these majors only (N8268DotNetNullValueType.al:15).
        let major = self.engine.target.as_ref().map(|t| t.bc.major);
        let measured_major = matches!(major, Some(27..=29));
        let method = measured_major
            && kind == MemberKind::Call
            && METHODS
                .iter()
                .any(|(t, m, w, p)| *t == clr_type && *m == member && fits(*w, p));
        let construct = measured_major
            && kind == MemberKind::Construct
            && CONSTRUCTORS.iter().any(|(t, w, p)| *t == clr_type && fits(*w, p));
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
