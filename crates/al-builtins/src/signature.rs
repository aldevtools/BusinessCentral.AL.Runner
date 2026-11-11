//! Parsing the `syntax` line of a documented builtin into a signature.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    pub name: String,
    pub ty: String,
    pub by_var: bool,
    pub optional: bool,
    /// `Value: Any,...` — repeats zero or more times (one or more if not optional).
    pub variadic: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature {
    /// The documented type the method belongs to (`Record`, `System`, `Dialog`).
    pub receiver: String,
    pub name: String,
    /// Called on a value of the receiver type (`Rec.Insert()`), as opposed to
    /// statically (`Text.CopyStr(...)`, usually callable unqualified).
    pub instance: bool,
    pub params: Vec<Param>,
    /// Empty when the method returns nothing.
    pub ret: String,
    /// `[Ok := ]`: the return value may be ignored; ignoring it turns a failure into a runtime error.
    pub ret_optional: bool,
    /// Available from this app runtime on (`@17.0` in the catalog); `None` for
    /// everything at or below the runtime floor (ADR-015 3.3).
    pub since: Option<al_version::Runtime>,
}

/// One overload as stored in `al-surface.json`.
#[derive(Debug, serde::Deserialize)]
pub struct DocEntry {
    pub syntax: String,
    #[serde(default)]
    pub parameters: Vec<DocParam>,
    #[serde(default)]
    pub return_type: String,
    #[serde(default)]
    pub return_optional: bool,
    /// "Available or changed with runtime version X", for example `17.0.`.
    #[serde(default)]
    pub runtime_version: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct DocParam {
    #[serde(default)]
    pub description: String,
}

impl Param {
    /// The argument names a field (of the receiver, a record or a test page's filter) or a query column, so a record
    /// field written here is passed as its position, not as its value (`R.SetRange(Amount, 1)`). Exactly the
    /// parameters named `Field`, `Field1`, `Field2`, `Fields`, `FromField` or `Column` whose type is `Any` or
    /// `TestFilterField` (`tests/field_params.rs` lists every catalog parameter this selects). A parameter whose name
    /// only starts with `Field` takes a value: `RecordRef.Field(FieldName: Text)` and `FieldExist(FieldName: Text)`
    /// take the name the field holds, as they take a Text variable's value (recordref-fieldref/P107RecRefNames.al:25,
    /// :74), and `AddEntityKey(FieldId, FieldValue: Any)` takes the field's value (al-runner issue 7616).
    pub fn names_field(&self) -> bool {
        const NAMES: [&str; 6] = ["Field", "Field1", "Field2", "Fields", "FromField", "Column"];
        matches!(self.ty.as_str(), "Any" | "TestFilterField") && NAMES.iter().any(|n| self.name.eq_ignore_ascii_case(n))
    }
}

impl Signature {
    pub fn from_doc(e: &DocEntry) -> Result<Signature, String> {
        // Drop `Ret :=` / `[Ret := ]` before the call.
        let call = match e.syntax.find(":=") {
            Some(i) => e.syntax[i + 2..].trim_start_matches([' ', ']']),
            None => e.syntax.as_str(),
        }
        .trim();
        let open = call.find('(').ok_or_else(|| format!("no `(` in `{}`", e.syntax))?;
        let close = call.rfind(')').ok_or_else(|| format!("no `)` in `{}`", e.syntax))?;
        let (receiver, name) = call[..open]
            .trim()
            .rsplit_once('.')
            .ok_or_else(|| format!("no receiver in `{}`", e.syntax))?;
        let instance = e
            .parameters
            .first()
            .is_some_and(|p| p.description.starts_with("An instance of"));
        Ok(Signature {
            receiver: receiver.trim().to_string(),
            name: name.trim().to_string(),
            instance,
            params: parse_params(&call[open + 1..close])?,
            // The scrape truncates generic return types (`List of [TKey`): balance them.
            ret: {
                let r = e.return_type.trim();
                let open = r.matches('[').count().saturating_sub(r.matches(']').count());
                format!("{r}{}", "]".repeat(open))
            },
            ret_optional: e.return_optional,
            since: e.runtime_version.as_deref().and_then(tag_of),
        })
    }

    /// Inverse of [`Signature::to_line`].
    pub fn from_line(line: &str) -> Result<Signature, String> {
        let bad = || format!("malformed catalog line `{line}`");
        let (head, rest) = line.split_once(" (").ok_or_else(bad)?;
        let (qual, kind) = head.rsplit_once(' ').ok_or_else(bad)?;
        let (receiver, name) = qual.rsplit_once('.').ok_or_else(bad)?;
        let close = rest.rfind(')').ok_or_else(bad)?;
        let (params_s, tail) = (&rest[..close], rest[close + 1..].trim());
        let (tail, since) = match tail.rsplit_once('@') {
            Some((t, v)) => (t.trim_end(), Some(al_version::Runtime::parse(v).map_err(|_| bad())?)),
            None => (tail, None),
        };
        let (ret, ret_optional) = match tail.strip_prefix("-> ") {
            Some(r) => match r.strip_suffix('?') {
                Some(r) => (r.to_string(), true),
                None => (r.to_string(), false),
            },
            None if tail.is_empty() => (String::new(), false),
            None => return Err(bad()),
        };
        let mut params = Vec::new();
        let mut depth = 0;
        let mut cur = String::new();
        for c in params_s.chars().chain(std::iter::once(',')) {
            match c {
                '[' => depth += 1,
                ']' => depth -= 1,
                ',' if depth == 0 => {
                    let p = cur.trim();
                    if !p.is_empty() {
                        let (optional, p) = p.strip_prefix('?').map_or((false, p), |p| (true, p));
                        let (by_var, p) = p.strip_prefix("var ").map_or((false, p), |p| (true, p));
                        let (variadic, p) = p.strip_suffix('*').map_or((false, p), |p| (true, p));
                        let (n, t) = p.split_once(": ").ok_or_else(bad)?;
                        params.push(Param {
                            name: n.into(),
                            ty: t.into(),
                            by_var,
                            optional,
                            variadic,
                        });
                    }
                    cur.clear();
                    continue;
                }
                _ => {}
            }
            cur.push(c);
        }
        Ok(Signature {
            receiver: receiver.into(),
            name: name.into(),
            instance: match kind {
                "inst" => true,
                "static" => false,
                _ => return Err(bad()),
            },
            params,
            ret,
            ret_optional,
            since,
        })
    }

    /// One catalog line: `Record.SetFilter inst (Field: Any, String: Text, ?Value: Any*) -> Boolean?`
    pub fn to_line(&self) -> String {
        let params: Vec<String> = self
            .params
            .iter()
            .map(|p| {
                format!(
                    "{}{}{}: {}{}",
                    if p.optional { "?" } else { "" },
                    if p.by_var { "var " } else { "" },
                    p.name,
                    p.ty,
                    if p.variadic { "*" } else { "" }
                )
            })
            .collect();
        let mut line = format!(
            "{}.{} {} ({})",
            self.receiver,
            self.name,
            if self.instance { "inst" } else { "static" },
            params.join(", ")
        );
        if !self.ret.is_empty() {
            line.push_str(&format!(" -> {}{}", self.ret, if self.ret_optional { "?" } else { "" }));
        }
        if let Some(r) = self.since {
            line.push_str(&format!(" @{r}"));
        }
        line
    }
}

/// The catalog tag for a docs "available or changed with runtime version" value:
/// only versions above the runtime floor are tagged (ADR-015 3.3). The docs
/// write `17.0.` with a trailing dot; an unreadable value is no tag.
fn tag_of(runtime_version: &str) -> Option<al_version::Runtime> {
    let r = al_version::Runtime::parse(runtime_version.trim().trim_end_matches('.')).ok()?;
    (r > al_version::Runtime::floor()).then_some(r)
}

/// Parses the text between the parentheses of a syntax line.
pub fn parse_params(s: &str) -> Result<Vec<Param>, String> {
    let b: Vec<char> = s.chars().collect();
    let mut i = 0;
    let mut opt_depth = 0usize;
    let mut out = Vec::new();
    loop {
        // Separators and optional-group brackets between parameters.
        while i < b.len() && (b[i].is_whitespace() || b[i] == ',' || b[i] == '[' || b[i] == ']') {
            match b[i] {
                '[' => opt_depth += 1,
                ']' => {
                    opt_depth = opt_depth
                        .checked_sub(1)
                        .ok_or_else(|| format!("unbalanced `]` in `{s}`"))?
                }
                _ => {}
            }
            i += 1;
        }
        if i >= b.len() {
            break;
        }
        // [var] name ':' type [',...']
        let start = i;
        while i < b.len() && b[i] != ':' && b[i] != ',' && b[i] != ']' {
            i += 1;
        }
        if i >= b.len() || b[i] != ':' {
            return Err(format!("expected `name: type` at `{}`", &s[char_off(&b, start)..]));
        }
        let head: String = b[start..i].iter().collect();
        let head = head.trim();
        let (by_var, name) = match head.strip_prefix("var ") {
            Some(n) => (true, n.trim()),
            None => (false, head),
        };
        let name = name.rsplit('.').next().unwrap_or(name).to_string();
        i += 1;
        let ty_start = i;
        let mut ty_depth = 0usize;
        while i < b.len() {
            match b[i] {
                // A bracket right after `of` opens type arguments, not an optional group.
                '[' if ty_depth > 0 || b[ty_start..i].iter().collect::<String>().trim_end().ends_with(" of") => {
                    ty_depth += 1
                }
                ']' if ty_depth > 0 => ty_depth -= 1,
                ',' | ']' | '[' if ty_depth == 0 => break,
                _ => {}
            }
            i += 1;
        }
        let ty: String = b[ty_start..i].iter().collect();
        let mut variadic = false;
        let rest: String = b[i..].iter().collect();
        if rest.starts_with(",...") {
            variadic = true;
            i += 4;
        }
        out.push(Param {
            name,
            ty: ty.trim().to_string(),
            by_var,
            optional: opt_depth > 0,
            variadic,
        });
    }
    if opt_depth != 0 {
        return Err(format!("unbalanced `[` in `{s}`"));
    }
    Ok(out)
}

fn char_off(b: &[char], i: usize) -> usize {
    b[..i].iter().map(|c| c.len_utf8()).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(name: &str, ty: &str) -> Param {
        Param {
            name: name.into(),
            ty: ty.into(),
            by_var: false,
            optional: false,
            variadic: false,
        }
    }
    fn opt(mut x: Param) -> Param {
        x.optional = true;
        x
    }
    fn var(mut x: Param) -> Param {
        x.by_var = true;
        x
    }
    fn many(mut x: Param) -> Param {
        x.variadic = true;
        x
    }

    #[test]
    fn empty() {
        assert_eq!(parse_params("").unwrap(), vec![]);
    }

    #[test]
    fn required_and_trailing_optional_groups() {
        assert_eq!(
            parse_params("Field: Any [, FromValue: Any] [, ToValue: Any]").unwrap(),
            vec![p("Field", "Any"), opt(p("FromValue", "Any")), opt(p("ToValue", "Any"))]
        );
    }

    #[test]
    fn only_optional() {
        assert_eq!(
            parse_params("[PageParameters: Text]").unwrap(),
            vec![opt(p("PageParameters", "Text"))]
        );
    }

    #[test]
    fn var_parameters() {
        assert_eq!(
            parse_params("WriteOptions: XmlWriteOptions, var Text: Text").unwrap(),
            vec![p("WriteOptions", "XmlWriteOptions"), var(p("Text", "Text"))]
        );
    }

    #[test]
    fn variadic_forms() {
        assert_eq!(
            parse_params("Field: Any, String: Text [, Value: Any,...]").unwrap(),
            vec![p("Field", "Any"), p("String", "Text"), opt(many(p("Value", "Any")))]
        );
        assert_eq!(parse_params("Fields: Any,...").unwrap(), vec![many(p("Fields", "Any"))]);
        assert_eq!(
            parse_params("[Value: Any,...]").unwrap(),
            vec![opt(many(p("Value", "Any")))]
        );
        assert_eq!(
            parse_params("Field1: Any [, Field2: Any,...]").unwrap(),
            vec![p("Field1", "Any"), opt(many(p("Field2", "Any")))]
        );
        assert_eq!(
            parse_params("String: Text [, var Variable1: Any,...]").unwrap(),
            vec![p("String", "Text"), opt(var(many(p("Variable1", "Any"))))]
        );
    }

    #[test]
    fn brackets_inside_a_type_are_not_optional_markers() {
        assert_eq!(
            parse_params("Values: List of [Char] [, StartIndex: Integer]").unwrap(),
            vec![p("Values", "List of [Char]"), opt(p("StartIndex", "Integer"))]
        );
        assert_eq!(
            parse_params("D: Dictionary of [Text, List of [Integer]]").unwrap(),
            vec![p("D", "Dictionary of [Text, List of [Integer]]")]
        );
    }

    #[test]
    fn dotted_parameter_name_keeps_the_last_segment() {
        assert_eq!(
            parse_params("FromField: Any, Record.ToField: Any").unwrap(),
            vec![p("FromField", "Any"), p("ToField", "Any")]
        );
    }

    fn doc(syntax: &str, first_param_desc: &str, ret: &str, ret_optional: bool) -> DocEntry {
        DocEntry {
            syntax: syntax.into(),
            parameters: vec![DocParam {
                description: first_param_desc.into(),
            }],
            return_type: ret.into(),
            return_optional: ret_optional,
            runtime_version: None,
        }
    }

    #[test]
    fn instance_method_with_optional_return() {
        let s = Signature::from_doc(&doc(
            "[Ok := ]  Record.SetFilter(Field: Any, String: Text [, Value: Any,...])",
            "An instance of the [Record](record-data-type.md) data type.",
            "Boolean",
            true,
        ))
        .unwrap();
        assert_eq!(
            (s.receiver.as_str(), s.name.as_str(), s.instance),
            ("Record", "SetFilter", true)
        );
        assert_eq!(s.params.len(), 3);
        assert_eq!(
            s.to_line(),
            "Record.SetFilter inst (Field: Any, String: Text, ?Value: Any*) -> Boolean?"
        );
    }

    #[test]
    fn static_method_with_multi_word_return_name() {
        let s = Signature::from_doc(&doc(
            "Enum with integer value :=   Enum.FromInteger(Value: Integer)",
            "The integer value.",
            "Enum",
            false,
        ))
        .unwrap();
        assert!(!s.instance);
        assert_eq!(s.to_line(), "Enum.FromInteger static (Value: Integer) -> Enum");
    }

    #[test]
    fn procedure_without_return_and_var_param() {
        let s = Signature::from_doc(&doc(
            "Dialog.Open(String: Text [, var Variable1: Any,...])",
            "",
            "",
            false,
        ))
        .unwrap();
        assert_eq!(s.to_line(), "Dialog.Open static (String: Text, ?var Variable1: Any*)");
    }

    #[test]
    fn line_round_trips() {
        for l in [
            "Record.SetFilter inst (Field: Any, String: Text, ?Value: Any*) -> Boolean?",
            "Dialog.Open inst (String: Text, ?var Variable1: Any*)",
            "Database.Commit static ()",
            "Text.IndexOfAny inst (Values: List of [Char], ?StartIndex: Integer) -> Integer",
            "Session.LogMessage static (EventId: Text, CustomDimensions: Dictionary of [Text, Text])",
        ] {
            assert_eq!(Signature::from_line(l).unwrap().to_line(), l);
        }
        let s = Signature::from_line("Dialog.Open inst (String: Text, ?var Variable1: Any*)").unwrap();
        assert_eq!(
            s.params[1],
            Param {
                name: "Variable1".into(),
                ty: "Any".into(),
                by_var: true,
                optional: true,
                variadic: true
            }
        );
    }

    #[test]
    fn syntax_without_receiver_is_an_error() {
        assert!(Signature::from_doc(&doc("Message(Text: Text)", "", "", false)).is_err());
    }

    #[test]
    fn malformed_is_an_error() {
        assert!(parse_params("Field Any").is_err());
        assert!(parse_params("[Field: Any").is_err());
    }
}
