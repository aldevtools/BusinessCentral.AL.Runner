//! Candidates and whether an argument type fits a parameter type.
use super::*;

pub(crate) fn proc_cand(p: &ProcSig) -> Cand {
    Cand {
        min: p.params.len(),
        max: p.params.len(),
        ret: p.ret.clone().unwrap_or(Ty::Unknown),
        target: Res::Proc {
            object: p.declared_in,
            offset: p.offset,
        },
        param_tys: p.params.iter().map(|x| x.ty.to_string()).collect(),
        fields: Vec::new(),
        rest_field: false,
        hidden: None,
        with_off: None,
    }
}

/// Whether an argument of type `arg` fits a parameter declared as `param`
/// (catalog spelling or `Ty` display). Only used to break arity ties.
pub(crate) fn fits(arg: &Ty, param: &str) -> bool {
    // A record fits a parameter declared on a table only when it is that table
    // (System Application overloads `ValidateRetentionPeriod` on two record types).
    if let Ty::Record { .. } = arg {
        if param.to_ascii_lowercase().starts_with("record \"") {
            return arg.to_string().eq_ignore_ascii_case(param);
        }
    }
    let p = param.to_ascii_lowercase();
    let p = p.split(['[', ' ']).next().unwrap_or("");
    match arg {
        Ty::Unknown => true,
        Ty::Integer | Ty::BigInteger | Ty::Decimal | Ty::Byte | Ty::Char => {
            matches!(
                p,
                "integer" | "biginteger" | "decimal" | "byte" | "char" | "any" | "joker" | "variant"
            )
        }
        Ty::Text(_) | Ty::Code(_) | Ty::Label => {
            matches!(p, "text" | "code" | "string" | "any" | "joker" | "variant" | "label")
        }
        Ty::Boolean => matches!(p, "boolean" | "any" | "joker" | "variant"),
        // A .NET result whose type is known only when the code runs (ADR-024) fits a parameter of
        // a scalar type, which it is settled to, and a Variant; not an object parameter such as
        // ErrorInfo (`Error(Service.LastError.Message)` is the Text overload).
        Ty::DotNet(alias) if alias.is_empty() => matches!(
            p,
            "text"
                | "code"
                | "integer"
                | "biginteger"
                | "decimal"
                | "boolean"
                | "byte"
                | "char"
                | "date"
                | "time"
                | "datetime"
                | "duration"
                | "guid"
                | "any"
                | "joker"
                | "variant"
                | "dotnet"
        ),
        other => {
            if p == "any" || p == "joker" || p == "variant" {
                return true;
            }
            let arg = other.to_string().to_ascii_lowercase();
            // A parameter that names an object (`Record "Price Asset"`) fits only an argument of
            // that object: overloads that differ in the table are told apart by it.
            if param.contains('"') {
                return arg == param.to_ascii_lowercase();
            }
            // A `var` list or dictionary takes exactly its own element types: overloads that differ in
            // them (`GetValues(var List of [JsonValue])` and `(var List of [JsonObject])`) are told
            // apart by the argument's. Text and Code are one kind and their length does not count.
            // A built-in's `List of [T]` names a placeholder, not an element type.
            if matches!(other, Ty::List(_) | Ty::Dictionary(..)) && param.contains('[') && !param.contains("[T") {
                return generic_shape(&arg) == generic_shape(param);
            }
            arg.starts_with(p)
        }
    }
}

/// Whether an argument fits a parameter of a procedure AL code declares, where two overloads compete (misc/lt-inferred-behaviors/
/// P69Overloads.al:Overload_IntegerVsVariant, Overload_TextVsVariant): a BigInteger or a Decimal does not take the Integer overload
/// (the Variant one wins), a Char takes the Text overload over the Variant one. Other pairs fit as `fits` says.
pub(crate) fn fits_proc(arg: &Ty, param: &str) -> bool {
    let p = param.to_ascii_lowercase();
    let p = p.split(['[', ' ']).next().unwrap_or("");
    match (arg, p) {
        (Ty::BigInteger | Ty::Decimal, "integer") => false,
        (Ty::Char, "text") => true,
        // An Option takes the Integer overload over the Variant one (`option_var`).
        (Ty::Option(_), "integer") => true,
        // An Integer converts to an Option parameter (codeunits/CwInterfaceLeads.al:143-167 `CW_Overload_OptionVsEnum`:
        // `P(1)` and `P(I)` take `P(O: Option A,B,C)`).
        (Ty::Integer, "option") => true,
        _ => fits(arg, param),
    }
}

/// A `List of [..]` or `Dictionary of [..]` spelled for comparing element types: lower case, no
/// lengths, Code as Text.
pub(crate) fn generic_shape(t: &str) -> String {
    let mut out = String::new();
    let mut depth_len = false;
    for c in t.to_ascii_lowercase().chars() {
        match c {
            '[' if out.ends_with("text") || out.ends_with("code") => depth_len = true,
            ']' if depth_len => depth_len = false,
            _ if depth_len => {}
            _ => out.push(c),
        }
    }
    out.replace("code", "text")
}

pub(crate) fn is_field_param(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.starts_with("field") || n == "fromfield" || n.starts_with("column")
}

pub(crate) fn sig_cand(s: &Signature, recv: &Ty, id: String) -> Cand {
    let min = s.params.iter().filter(|p| !p.optional && !p.variadic).count()
        + s.params.iter().filter(|p| !p.optional && p.variadic).count();
    let max = if s.params.iter().any(|p| p.variadic) {
        usize::MAX
    } else {
        s.params.len()
    };
    let fields = s.params.iter().map(|p| is_field_param(&p.name)).collect();
    let rest_field = s.params.iter().any(|p| p.variadic && is_field_param(&p.name));
    Cand {
        target: Res::Builtin(id),
        param_tys: s.params.iter().map(|p| p.ty.clone()).collect(),
        min,
        max,
        // `Enum::"X".FromInteger(n)` is a value of enum X; the catalog's return type is "Any".
        ret: match recv {
            Ty::Enum(n) if !n.is_empty() && s.receiver == "Enum" && s.name == "FromInteger" => recv.clone(),
            _ => catalog_ty(&s.ret, recv),
        },
        fields,
        rest_field,
        hidden: None,
        with_off: None,
    }
}

/// A candidate that only a newer runtime has.
pub(crate) fn hidden_cand(s: &Signature, recv: &Ty, id: String) -> Cand {
    Cand {
        hidden: s.since,
        ..sig_cand(s, recv, id)
    }
}

/// A catalog type name as a `Ty`, with `T`/`TKey`/`TValue` bound from the receiver.
pub(crate) fn catalog_ty(s: &str, recv: &Ty) -> Ty {
    let s = s.trim();
    if s.is_empty() || s.eq_ignore_ascii_case("Any") || s.eq_ignore_ascii_case("Joker") {
        return Ty::Unknown;
    }
    // Catalog spellings repeat on every call site of a method; parse each once.
    thread_local! {
        static PARSED: std::cell::RefCell<HashMap<String, Option<Ty>>> = Default::default();
    }
    let parsed = PARSED.with(|c| {
        c.borrow_mut()
            .entry(s.to_string())
            .or_insert_with(|| al_syntax::parse_type(s).map(|tr| Ty::from_type_ref(&tr)))
            .clone()
    });
    match parsed {
        Some(t) => subst(t, recv),
        None => Ty::Unknown,
    }
}

pub(crate) fn subst(t: Ty, recv: &Ty) -> Ty {
    match (&t, recv) {
        (Ty::Builtin(n), Ty::List(el)) if &**n == "T" => (**el).clone(),
        (Ty::Builtin(n), Ty::Dictionary(k, _)) if &**n == "TKey" => (**k).clone(),
        (Ty::Builtin(n), Ty::Dictionary(_, v)) if &**n == "TValue" => (**v).clone(),
        (Ty::List(el), _) => Ty::List(Box::new(subst((**el).clone(), recv))),
        (Ty::Dictionary(k, v), _) => Ty::Dictionary(
            Box::new(subst((**k).clone(), recv)),
            Box::new(subst((**v).clone(), recv)),
        ),
        _ => t,
    }
}
