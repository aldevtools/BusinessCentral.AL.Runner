//! The `<Options>` of a report parameters text: its `Field` elements and the value each one gives the variable a request page
//! control shows (`apply_request_parameters`). Pinned by reports/runtime/Y68SelfClosingParams.al (corpus
//! master 0d01b0cc, BC 27.5, 28.5, 29.0) and reports/request-page/RptRequestPage.al:878-893; the rest is refused.

use super::*;
use al_builtins::describe::Describe;

/// The XML declaration a parameters string starts with: RunRequestPage writes it, and a string without it is not read as XML
/// by BC (al-runner issue 7590: BC raises an error whose text the corpus does not have).
pub(crate) const PARAMETERS_DECLARATION: &str = "<?xml version=\"1.0\" standalone=\"yes\"?>";

/// The call that hands a report its parameters string. Each rule that differs between the calls is a method here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ParamsCall {
    /// `Report.SaveAs`.
    SaveAs,
    /// `Report.Execute`.
    Execute,
    /// `Report.RunRequestPage(Id, Parameters)`: the seed of the request page.
    RunRequestPage,
}

/// What a call does with a `Field` that names no variable a control of the request page shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UnknownField {
    /// Skipped.
    Skip,
    /// Skipped through the assumption `report-saveas-unknown-field-ignored`.
    SkipAssumed,
    /// Refused.
    Refuse,
}

impl ParamsCall {
    /// The AL method, for a refusal text.
    pub(crate) fn method_name(self) -> &'static str {
        match self {
            ParamsCall::SaveAs => "Report.SaveAs",
            ParamsCall::Execute => "Report.Execute",
            ParamsCall::RunRequestPage => "Report.RunRequestPage",
        }
    }

    /// Whether a root without name and id (after the declaration, holding one `<Options>`) is this report's parameters.
    /// SaveAs: pinned (reports/layouts-saveas/AMRdlcOutput.al:153 with :350 and :372 `REQ:AMREQ42`; AMReportOutputTests.al:24
    /// with :250 `request_value_in_sheet1`). Execute and RunRequestPage: not pinned (Y64XappRunOrder.al:156 only shows the
    /// Execute runs; corpus probe ReportParametersBareRoot.al queued).
    pub(crate) fn accepts_root_without_name(self) -> bool {
        self == ParamsCall::SaveAs
    }

    /// Execute skips the name (reports/WdReportExecuteParams.al:69-77 WD_ReportExecute_UnknownField). SaveAs with a root
    /// without name and id skips it as the reporter measured (al-runner issue 7590, assumed). Everything else is refused.
    pub(crate) fn unknown_field(self, root_without_name: bool) -> UnknownField {
        match self {
            ParamsCall::Execute => UnknownField::Skip,
            ParamsCall::SaveAs if root_without_name => UnknownField::SkipAssumed,
            _ => UnknownField::Refuse,
        }
    }

    /// Execute skips a data item name the report does not have (corpus PR 886: ExecuteParamsTwiceAndViews.al:217).
    pub(crate) fn skips_unknown_data_item(self) -> bool {
        self == ParamsCall::Execute
    }

    /// Execute applies a view also where the request page hides the filters (session-system/company/Y22CopyCompany.al:301-323:
    /// report 357 "Copy Company" has `ShowFilter = false` and copies only the company its parameters' view names).
    pub(crate) fn applies_hidden_view(self) -> bool {
        self == ParamsCall::Execute
    }
}

/// The Options text of a parameters string whose root has no name and id, `None` for a string in another form. Only the form
/// the corpus and the reporter show is read: the declaration, the bare root holding exactly one `<Options>` element. A data item
/// view under such a root, or other content, is refused (not pinned).
pub(crate) fn bare_root_options(xml: &str) -> Option<Result<&str, String>> {
    let body = xml
        .strip_prefix(PARAMETERS_DECLARATION)?
        .strip_prefix("<ReportParameters>")?
        .strip_suffix("</ReportParameters>")?;
    if body.contains("<DataItems") {
        return Some(Err(
            "a data item view in a parameters string whose root has no name and id (not pinned)".into(),
        ));
    }
    let options = body
        .strip_prefix("<Options>")
        .and_then(|b| b.strip_suffix("</Options>"))
        .filter(|o| !o.contains("<Options"));
    Some(options.ok_or_else(|| {
        "a parameters string whose root has no name and id and does not hold exactly one Options element (not pinned)".into()
    }))
}

/// The `Field` elements of an Options text, as (name, text). `<Field name="X" />` and `<Field name="X"></Field>` both give an
/// empty text (Y68SelfClosingParams.al:263-343: RunRequestPage writes the first form for an empty value). Anything else
/// between or after the elements (another element, a Field in another form such as single quotes or a second attribute) is
/// refused, not dropped: no corpus test pins how BC reads it.
pub(crate) fn option_fields(options: &str) -> Result<Vec<(String, String)>, String> {
    const OPEN: &str = "<Field name=\"";
    const CLOSE: &str = "</Field>";
    let unread = |rest: &str| {
        let cut: String = rest.trim().chars().take(60).collect();
        format!("a part of the parameters Options this runtime cannot read: {cut} (not pinned)")
    };
    let mut out = Vec::new();
    let mut rest = options;
    while let Some(at) = rest.find(OPEN) {
        if !rest[..at].trim().is_empty() {
            return Err(unread(rest));
        }
        let after = &rest[at + OPEN.len()..];
        let Some(q) = after.find('"') else {
            return Err(unread(rest));
        };
        let name = super::request::xml_unescape(&after[..q]);
        let tail = &after[q + 1..];
        if let Some(more) = tail.strip_prefix(" />").or_else(|| tail.strip_prefix("/>")) {
            out.push((name, String::new()));
            rest = more;
            continue;
        }
        let Some(body) = tail.strip_prefix('>') else {
            return Err(unread(rest));
        };
        let Some(end) = body.find(CLOSE) else {
            return Err(unread(rest));
        };
        if body[..end].contains('<') {
            return Err(unread(rest));
        }
        out.push((name, super::request::xml_unescape(&body[..end])));
        rest = &body[end + CLOSE.len()..];
    }
    if !rest.trim().is_empty() {
        return Err(unread(rest));
    }
    Ok(out)
}

fn is_number(text: &str) -> bool {
    let digits = text.strip_prefix('-').unwrap_or(text);
    !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
}

impl Session {
    /// The value the option `name` with the text `text` gives a variable that holds `current`.
    pub(crate) fn request_option_value(&self, name: &str, text: &str, current: &Value) -> Result<Value, Signal> {
        let refuse = || unsupported(format!("the parameter {name} = {text} (this form is not pinned)"));
        // An empty text sets the variable to its blank value, whatever the request page's OnInit or OnInitReport set: Text
        // and Code '', Date, Time and DateTime undefined, Integer, Decimal and BigInteger 0, Boolean No, an Option its first
        // member and an Enum the value 0 (Y68SelfClosingParams.al:263-343, :373-405).
        if text.is_empty() {
            return match current {
                Value::Text(_) => Ok(Value::Text("".into())),
                Value::Code(_) => Ok(Value::Code("".into())),
                Value::Date(..) => Ok(Value::Date(0, false)),
                Value::Time(_) => Ok(Value::Time(0)),
                Value::DateTime(_) => Ok(Value::DateTime(0)),
                Value::Integer(_) => Ok(Value::Integer(0)),
                Value::BigInteger(_) => Ok(Value::BigInteger(0)),
                Value::Decimal(_) => Ok(Value::default_for(&RtType::Decimal)),
                Value::Boolean(_) => Ok(Value::Boolean(false)),
                Value::Option(_, m) => Ok(Value::Option(0, m.clone())),
                // Only an enum that has the value 0 is pinned (its first value, `First`).
                Value::Enum(id, _)
                    if al_builtins::native::Ctx::enum_members(self, *id)
                        .is_some_and(|m| m.first().is_some_and(|(n, _)| *n == 0)) =>
                {
                    Ok(Value::Enum(*id, 0))
                }
                other => unsupported(format!(
                    "an empty parameter {name} for a control holding {}",
                    other.describe()
                )),
            };
        }
        // The forms RunRequestPage writes (Y68SelfClosingParams.al:345-371 `filled all`, :446-470 `seeded returned`).
        match current {
            Value::Text(_) => Ok(Value::Text(text.into())),
            // A Code is uppercased (`xy` gives `XY`).
            Value::Code(_) => Ok(Value::Code(text.to_uppercase().as_str().into())),
            Value::Boolean(_) if text == "true" || text == "false" => Ok(Value::Boolean(text == "true")),
            // `yyyy-MM-dd` (also RptRequestPage.al:878-893 RunRequestPage_ThenExecute_AppliesValues).
            Value::Date(..) => match al_builtins::iso::parse_date(text) {
                Some(d) if text.len() == 10 => Ok(Value::Date(d, false)),
                _ => refuse(),
            },
            // `HH:mm:ss`.
            Value::Time(_) => match al_builtins::iso::parse_time(text) {
                Some(t) if text.len() == 8 => Ok(Value::Time(t)),
                _ => refuse(),
            },
            // `yyyy-MM-ddTHH:mm:ssZ`.
            Value::DateTime(_) => match al_builtins::iso::parse_datetime(text) {
                Some(t) if text.len() == 20 && text.ends_with('Z') => Ok(Value::DateTime(t)),
                _ => refuse(),
            },
            Value::Integer(_) => match text.parse::<i32>() {
                Ok(n) if is_number(text) => Ok(Value::Integer(n)),
                // A text of letters fails the Execute with the text `NavType` and the code Argument (`filled bad int`).
                _ if text.bytes().all(|b| b.is_ascii_alphabetic()) => Err(Signal::Error(Box::new(AlError {
                    message: "NavType".into(),
                    code: "Argument".into(),
                    ..Default::default()
                }))),
                _ => refuse(),
            },
            Value::BigInteger(_) => match text.parse::<i64>() {
                Ok(n) if is_number(text) => Ok(Value::BigInteger(n)),
                _ => refuse(),
            },
            // `0.75`, `2.5`: digits with an optional `.` part.
            Value::Decimal(_) => {
                let (whole, frac) = text.split_once('.').unwrap_or((text, "0"));
                if !is_number(whole) || !frac.bytes().all(|b| b.is_ascii_digit()) || frac.is_empty() {
                    return refuse();
                }
                ops::convert(Value::Text(text.into()), &RtType::Decimal)
            }
            // An Option by its index; a member name fails the Execute with the Evaluate error from BC 28 and
            // `The type NavOption is unknown.` before (`filled by name`, guarded BC28PLUS).
            Value::Option(_, m) => match text.parse::<i32>() {
                Ok(n) if is_number(text) && n >= 0 && (n as usize) < m.len() => Ok(Value::Option(n, m.clone())),
                Ok(_) => refuse(),
                Err(_) if m.iter().any(|x| x == text) => {
                    let bc28 = self.engine.target.as_ref().map_or(29, |t| t.bc.major) >= 28;
                    let (message, code) = if bc28 {
                        (
                            format!("The value \"{text}\" can't be evaluated into type Option."),
                            "Evaluate:Format",
                        )
                    } else {
                        ("The type NavOption is unknown.".to_string(), "NotSupportedType")
                    };
                    Err(Signal::Error(Box::new(AlError {
                        message,
                        code: code.into(),
                        ..Default::default()
                    })))
                }
                Err(_) => refuse(),
            },
            // An Enum by its ordinal, one of its values.
            Value::Enum(id, _) => match text.parse::<i32>() {
                Ok(n)
                    if is_number(text)
                        && al_builtins::native::Ctx::enum_members(self, *id)
                            .is_some_and(|m| m.iter().any(|(o, _)| *o == n)) =>
                {
                    Ok(Value::Enum(*id, n))
                }
                _ => refuse(),
            },
            Value::Boolean(_) => refuse(),
            other => unsupported(format!(
                "a parameter for the control {name} holding {}",
                other.describe()
            )),
        }
    }
}
