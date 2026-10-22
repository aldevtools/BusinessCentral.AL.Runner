//! Pages for the test client (ADR-012): the source-level step that turns
//! expression-valued control properties into routines, and the layout of a
//! page's control tree.
//!
//! `Visible = not HideIt;` is AL code, so it must be resolved and lowered like
//! any other. Rather than teach the checker a second expression entry point,
//! `expand` appends one synthetic procedure per such property to the page
//! object before the file is parsed for good, named from the property and its
//! offset in the original text (`__visible_123`). Original offsets do not move,
//! because the text is only added at the end of an object.

use al_syntax::ast;
use std::collections::BTreeSet;

/// What a synthetic property routine returns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropType {
    Boolean,
    Text,
    Variant,
}

impl PropType {
    fn al(self) -> &'static str {
        match self {
            PropType::Boolean => "Boolean",
            PropType::Text => "Text",
            PropType::Variant => "Variant",
        }
    }
}

/// Section names whose properties belong to a page control.
const CONTROL_SECTIONS: &[&str] = &[
    "field",
    "group",
    "repeater",
    "part",
    "action",
    "systemaction",
    "modify",
    "label",
];

/// The type a property evaluates to, when it is one the client evaluates.
pub fn dynamic_type(prop: &str) -> Option<PropType> {
    match prop.to_ascii_lowercase().as_str() {
        // HideValue may name a variable (pages/lt-page-presentation-properties/P119LabelHideValue.al:74).
        "visible" | "editable" | "enabled" | "hidevalue" => Some(PropType::Boolean),
        "autoformatexpression" | "captionclass" => Some(PropType::Text),
        _ => None,
    }
}

/// Whether a property value is a literal rather than an expression.
pub fn is_literal(ty: PropType, value: &str) -> bool {
    let v = value.trim();
    match ty {
        PropType::Boolean => v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("false"),
        PropType::Text => v.starts_with('\'') && v.ends_with('\'') && v.len() >= 2 && !v[1..v.len() - 1].contains('\''),
        PropType::Variant => false,
    }
}

/// Why BC cannot run a Boolean control expression (`Visible`, `Editable`, `Enabled`, `HideValue`) the way the runtime
/// would: BC compiles these on the client when the page opens ("dynamic control expression"), with a grammar of its own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientExpr {
    /// The client compile fails on every measured BC version: the page is refused.
    Refused(String),
    /// A BigInteger literal (`0L`): the client compile fails on BC 27 (`Not a valid number`) and runs from BC 28
    /// (pages/lt-cc-page-leads/CcPageLeads.al:801-808, Expr_BigIntegerLiteral_Enabled, `#if BC28PLUS`).
    BigIntegerLiteral(String),
}

/// The name of the procedure when `value` is a call with no arguments and nothing else (`IsAllowed()`, `"Is Allowed"()`).
pub fn bare_call_name(value: &str) -> Option<String> {
    let v = value.trim();
    let head = v.strip_suffix(')')?.trim_end().strip_suffix('(')?.trim_end();
    let ok = if let Some(q) = head.strip_prefix('"') {
        q.strip_suffix('"').is_some_and(|n| !n.is_empty() && !n.contains('"'))
    } else {
        head.chars().next().is_some_and(|c| c.is_alphabetic() || c == '_')
            && head.chars().all(|c| c.is_alphanumeric() || c == '_')
    };
    ok.then(|| head.trim_matches('"').to_string())
}

/// What the client compile of Boolean control expression `value` does that the runtime cannot follow, or `None`.
///
/// Pinned failures (pages/lt-cc-page-leads/CcPageLeads.al): a member of a procedure's result (`GetRow().Flag`, :764,
/// "The type of function 'GetRow' is unknown."), a date literal (`Rec.D = 0D`, :846, "Not a valid number."), a member of a
/// global record (`GlobalRow.Grp = 2`, :880, "The identifier 'GlobalRow.Grp' could not be found."). The error texts carry an
/// index and a rewritten expression whose rule is not pinned, so these are refused, not raised. A call, a time or datetime
/// literal and a member of anything but `Rec`, `xRec` or `this` are refused with them: their client compile is not pinned
/// (Base Application and the System Application use none of them).
pub fn client_expression(value: &str) -> Option<ClientExpr> {
    let b: Vec<char> = value.chars().collect();
    let mut i = 0;
    let mut prev_word: Option<String> = None;
    let mut big = None;
    let refuse = |what: &str| {
        Some(ClientExpr::Refused(format!(
            "{what} in the dynamic control expression `{}` (BC compiles it on the client when the page opens; not pinned)",
            value.trim()
        )))
    };
    while i < b.len() {
        let c = b[i];
        if c == '\'' {
            i += 1;
            while i < b.len() {
                if b[i] == '\'' {
                    if b.get(i + 1) == Some(&'\'') {
                        i += 2;
                        continue;
                    }
                    break;
                }
                i += 1;
            }
            i += 1;
            prev_word = None;
            continue;
        }
        let word = if c == '"' {
            let start = i;
            i += 1;
            while i < b.len() && b[i] != '"' {
                i += 1;
            }
            i += 1;
            Some(b[start..i.min(b.len())].iter().collect::<String>())
        } else if c.is_ascii_digit() {
            let start = i;
            while i < b.len()
                && (b[i].is_ascii_alphanumeric() || b[i] == '.' && b.get(i + 1).is_some_and(|d| d.is_ascii_digit()))
            {
                i += 1;
            }
            let lit: String = b[start..i].iter().collect::<String>().to_ascii_lowercase();
            if lit.ends_with('d') || lit.ends_with('t') {
                return refuse("a date or time literal");
            }
            if lit.ends_with('l') {
                big = Some(ClientExpr::BigIntegerLiteral(format!(
                    "a BigInteger literal in the dynamic control expression `{}`: a BC version without gate BigIntegerLiteralInControlExpressionFrom28 fails to compile it when the page opens \
                     (pages/lt-cc-page-leads/CcPageLeads.al:801-808; the error text's index rule is not pinned)",
                    value.trim()
                )));
            }
            prev_word = None;
            continue;
        } else if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < b.len() && (b[i].is_alphanumeric() || b[i] == '_') {
                i += 1;
            }
            Some(b[start..i].iter().collect::<String>())
        } else {
            None
        };
        if let Some(w) = word {
            prev_word = Some(w);
            continue;
        }
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        match c {
            '(' => {
                if let Some(w) = &prev_word {
                    if !["and", "or", "not", "xor", "div", "mod", "in"].contains(&w.to_ascii_lowercase().as_str()) {
                        return refuse("a call");
                    }
                }
            }
            '.' if b.get(i + 1) != Some(&'.') => match &prev_word {
                Some(w) if ["rec", "xrec", "this"].contains(&w.trim_matches('"').to_ascii_lowercase().as_str()) => {}
                _ => return refuse("a member access"),
            },
            ':' if b.get(i + 1) == Some(&':') => {
                // `Rec.Status::Open`, `"Enum"::Value`: the value name is a word, not a member.
                i += 2;
                while i < b.len() && b[i].is_whitespace() {
                    i += 1;
                }
                if i < b.len() && b[i] == '"' {
                    i += 1;
                    while i < b.len() && b[i] != '"' {
                        i += 1;
                    }
                    i += 1;
                } else {
                    while i < b.len() && (b[i].is_alphanumeric() || b[i] == '_') {
                        i += 1;
                    }
                }
                prev_word = None;
                continue;
            }
            _ => {}
        }
        prev_word = None;
        i += 1;
    }
    big
}

/// Name of the synthetic routine for property `prop` of the node at `offset`
/// (counted from the start of its object, so inserting text into an earlier
/// object of the same file does not rename it).
pub fn synthetic_name(prop: &str, offset: usize) -> String {
    format!("__{}_{offset}", prop.to_ascii_lowercase())
}

/// Whether `name` is the name of a routine made by `expand`.
pub fn is_synthetic(name: &str) -> bool {
    name.strip_prefix("__")
        .and_then(|r| r.rsplit_once('_'))
        .is_some_and(|(prop, off)| !prop.is_empty() && !off.is_empty() && off.chars().all(|c| c.is_ascii_digit()))
}

/// How a field control's source expression binds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// `Rec.Field`
    RecField(String),
    /// A global variable of the page.
    Var(String),
    /// Anything else: evaluated by a synthetic routine, read-only.
    Expr,
}

/// Classifies the source expression `text` of a field control.
pub fn classify_source(text: &str, globals: &BTreeSet<String>) -> Source {
    let t = text.trim();
    if let Some(rest) = t.strip_prefix("Rec.").or_else(|| t.strip_prefix("rec.")) {
        let name = rest.trim();
        let unq = name.strip_prefix('"').and_then(|n| n.strip_suffix('"'));
        match unq {
            Some(n) if !n.contains('"') => return Source::RecField(n.to_string()),
            None if name.chars().all(|c| c.is_alphanumeric() || c == '_') && !name.is_empty() => {
                return Source::RecField(name.to_string());
            }
            _ => {}
        }
    }
    let bare = t.strip_prefix('"').and_then(|n| n.strip_suffix('"')).unwrap_or(t);
    if globals.contains(&bare.to_lowercase()) {
        return Source::Var(bare.to_string());
    }
    Source::Expr
}

/// Whether `text` names a place AL can assign to: names (plain or quoted) joined by dots, each with indices of digits or
/// names (`Totals[1]."Inv. Discount Amount"`); no call, no operator, no literal.
pub fn assignable_path(text: &str) -> bool {
    let b: Vec<char> = text.trim().chars().collect();
    let mut i = 0;
    let name = |i: &mut usize| -> bool {
        if *i < b.len() && b[*i] == '"' {
            let start = *i + 1;
            let mut j = start;
            while j < b.len() && b[j] != '"' {
                j += 1;
            }
            if j >= b.len() || j == start {
                return false;
            }
            *i = j + 1;
            return true;
        }
        let start = *i;
        while *i < b.len() && (b[*i].is_alphanumeric() || b[*i] == '_') {
            *i += 1;
        }
        *i > start && !b[start].is_ascii_digit()
    };
    loop {
        if !name(&mut i) {
            return false;
        }
        while i < b.len() && b[i] == '[' {
            i += 1;
            let start = i;
            while i < b.len() && b[i] != ']' {
                if !(b[i].is_alphanumeric() || b[i] == '_') {
                    return false;
                }
                i += 1;
            }
            if i >= b.len() || i == start {
                return false;
            }
            i += 1;
        }
        if i == b.len() {
            return true;
        }
        if b[i] != '.' {
            return false;
        }
        i += 1;
    }
}

/// Whether `text` is one bare name, quoted or not (`ProdOrderNo`, `"Prod. Order No."`): the only form in which
/// a control of an extension can name a `protected var` of its base object.
pub fn plain_name(text: &str) -> Option<String> {
    let t = text.trim();
    if let Some(inner) = t.strip_prefix('"').and_then(|n| n.strip_suffix('"')) {
        return (!inner.is_empty() && !inner.contains('"')).then(|| inner.to_string());
    }
    (!t.is_empty()
        && t.chars().all(|c| c.is_alphanumeric() || c == '_')
        && !t.starts_with(|c: char| c.is_ascii_digit()))
    .then(|| t.to_string())
}

/// Global variable names (lower case) declared in `obj`.
pub fn global_names(obj: &ast::Object) -> BTreeSet<String> {
    obj.var_decls()
        .flat_map(|v| v.names())
        .map(|n| n.to_lowercase())
        .collect()
}

/// A control's section and the properties and source of it that need routines:
/// (property name, type, offset used in the name, expression text).
///
/// `extension_routines` are the routines of an extension object (empty for any other object): a field of an extension
/// whose source is a bare name that is none of them and no global of the extension gets no routine, because it
/// names a variable of the base object (corpus handlers/TestRequestPageControlErrorName.al, Ext5803Handler:
/// `field(...; ResetProdOrderCosting)` in Base App's "Mfg. Reset Cost Is Adjusted"). Lowering binds it there or refuses.
fn needed(
    section: &ast::Section,
    base: usize,
    globals: &BTreeSet<String>,
    extension_routines: Option<&BTreeSet<String>>,
) -> Vec<(String, PropType, usize, String)> {
    let mut out = Vec::new();
    for p in section.properties() {
        let name = p.name();
        let Some(ty) = dynamic_type(&name) else { continue };
        let value = p.value_text();
        if !is_literal(ty, &value) {
            out.push((
                name.to_ascii_lowercase(),
                ty,
                usize::from(p.syntax().text_range().start()) - base,
                value,
            ));
        }
    }
    if section.name().eq_ignore_ascii_case("field") {
        let args = section.args_raw();
        if let Some(src) = args.get(1) {
            let names_a_base_variable = extension_routines
                .is_some_and(|routines| plain_name(src).is_some_and(|n| !routines.contains(&n.to_lowercase())));
            if classify_source(src, globals) == Source::Expr && !names_a_base_variable {
                out.push((
                    "source".into(),
                    PropType::Variant,
                    usize::from(section.syntax().text_range().start()) - base,
                    src.clone(),
                ));
            }
        }
    }
    out
}

/// The table field sections of a table or table extension whose `CaptionClass` is an expression: (field name as written
/// in the section, the name of the synthetic routine that evaluates it). A `field` section names its field second, a
/// `modify` block first. The routine returns Text and runs with the record as Rec (FieldCaption, a page control's caption;
/// pageext/lt-page-ext-properties/TableExtCaptionClass.al:56-74).
pub fn field_caption_class_routines(obj: &ast::Object) -> Vec<(String, String, String)> {
    field_expression_routines(obj, "CaptionClass")
}

/// The table field sections of a table or table extension whose `AutoFormatExpression` is an expression, as
/// [`field_caption_class_routines`]. A page control over the field that names no expression of its own formats its value
/// with the routine's result, run with the control's row as Rec (a field of the row: types/lt-auto-format-currency/QiTableFieldAutoFormat.al).
pub fn field_auto_format_routines(obj: &ast::Object) -> Vec<(String, String, String)> {
    field_expression_routines(obj, "AutoFormatExpression")
}

/// The table field sections that set the Text property `prop` to an expression: (field, synthetic routine name,
/// expression).
fn field_expression_routines(obj: &ast::Object, prop: &str) -> Vec<(String, String, String)> {
    let base = usize::from(obj.syntax().text_range().start());
    let mut out = Vec::new();
    for sec in ast::sections_in(obj.syntax()) {
        let at = if sec.name().eq_ignore_ascii_case("field") {
            1
        } else if sec.name().eq_ignore_ascii_case("modify") {
            0
        } else {
            continue;
        };
        let Some(field) = sec.args().get(at).cloned() else {
            continue;
        };
        for p in sec.properties() {
            if !p.name().eq_ignore_ascii_case(prop) {
                continue;
            }
            let value = p.value_text();
            if is_literal(PropType::Text, &value) {
                continue;
            }
            let off = usize::from(p.syntax().text_range().start()) - base;
            out.push((field.clone(), synthetic_name(prop, off), value));
        }
    }
    out
}

/// The names of the fields whose `CaptionClass` a `field` section or `modify` block of a table or table extension sets,
/// literal or expression, once per section that sets it.
pub fn field_caption_class_settings(obj: &ast::Object) -> Vec<String> {
    let mut out = Vec::new();
    for sec in ast::sections_in(obj.syntax()) {
        let at = if sec.name().eq_ignore_ascii_case("field") {
            1
        } else if sec.name().eq_ignore_ascii_case("modify") {
            0
        } else {
            continue;
        };
        let Some(field) = sec.args().get(at).cloned() else {
            continue;
        };
        if sec.properties().any(|p| p.name().eq_ignore_ascii_case("CaptionClass")) {
            out.push(field);
        }
    }
    out
}

/// Every control section of a page object, in source order.
pub fn control_sections(obj: &ast::Object) -> Vec<ast::Section> {
    // Sections are object structure: a routine body never holds one, so the walk skips the routines.
    ast::sections_in(obj.syntax())
        .into_iter()
        .filter(|s| CONTROL_SECTIONS.iter().any(|c| s.name().eq_ignore_ascii_case(c)))
        .collect()
}

/// `src` with the synthetic routines of every page object appended, or `None`
/// when no page has an expression-valued property.
pub fn expand(src: &str, symbols: &BTreeSet<String>) -> Option<String> {
    expand_ranges(src, symbols).map(|(text, _)| text)
}

/// `expand`, with the byte ranges of the added text in the returned text (source positions
/// subtract them to give the author's lines).
pub fn expand_ranges(src: &str, symbols: &BTreeSet<String>) -> Option<(String, Vec<(usize, usize)>)> {
    match expand_or_parse(src, symbols) {
        Expansion::Expanded(text, ranges) => Some((text, ranges)),
        Expansion::Unchanged(_) => None,
    }
}

/// What [`expand_or_parse`] found.
pub enum Expansion {
    /// The text with the synthetic routines added, and the byte ranges of the added text (as [`expand_ranges`]).
    Expanded(String, Vec<(usize, usize)>),
    /// Nothing to add: the parse of the text as it is. A parse is a function of the text and the symbols, so this is
    /// the tree a parse of the file gives, and the caller need not parse the file again.
    Unchanged(al_syntax::Parse),
}

/// [`expand_ranges`], keeping the parse when nothing is added: every file the workspace parses at load goes through
/// here first, and most add nothing.
pub fn expand_or_parse(src: &str, symbols: &BTreeSet<String>) -> Expansion {
    let parse = crate::workspace::with_tree_cache(|cache| al_syntax::parse_with_cache(src, symbols, cache));
    match expansion(src, &parse) {
        Some((text, ranges)) => Expansion::Expanded(text, ranges),
        None => Expansion::Unchanged(parse),
    }
}

fn expansion(src: &str, parse: &al_syntax::Parse) -> Option<(String, Vec<(usize, usize)>)> {
    let root = ast::SourceFile::cast(parse.syntax())?;
    let mut inserts: Vec<(usize, String)> = Vec::new();
    for obj in root.objects() {
        // A report's request page has the same controls (corpus handlers/TestReportRequestPageControl.al).
        let kind = obj.kind().to_ascii_lowercase();
        let Some(close) = obj.close_brace_offset() else {
            continue;
        };
        // A table field's CaptionClass expression becomes a routine of the table or table extension that writes it.
        if kind == "table" || kind == "tableextension" {
            let text: String = field_caption_class_routines(&obj)
                .into_iter()
                .chain(field_auto_format_routines(&obj))
                .map(|(_, name, expr)| format!("\nprocedure {name}(): Text begin exit({expr}); end;"))
                .collect();
            if !text.is_empty() {
                inserts.push((close, text));
            }
            continue;
        }
        if !["page", "pageextension", "report", "reportextension"].contains(&kind.as_str()) {
            continue;
        }
        let globals = global_names(&obj);
        let extension_routines: Option<BTreeSet<String>> = kind
            .ends_with("extension")
            .then(|| obj.routines().map(|r| r.name().to_lowercase()).collect());
        let base = usize::from(obj.syntax().text_range().start());
        let mut text = String::new();
        // The column routines below go in before the request page, so the lowering, which reads the expanded text, finds a control
        // section that much further from the start of the object. The names of its routines carry that offset (Contact - Cover
        // Sheet: 13 controls bound to array elements, Tests-Marketing).
        let column_inserts: Vec<(usize, usize)> = if kind.starts_with("report") {
            column_sections(&obj)
                .iter()
                .enumerate()
                .filter_map(|(ordinal, sec)| {
                    let expr = sec.args_raw().get(1).cloned()?;
                    let len = format!(
                        "\nprocedure {}(): Variant begin exit({expr}); end;",
                        column_name(ordinal)
                    )
                    .len();
                    Some((usize::from(sec.syntax().text_range().end()) - base, len))
                })
                .collect()
        } else {
            Vec::new()
        };
        let shifted = |offset: usize| {
            offset
                + column_inserts
                    .iter()
                    .filter(|(at, _)| *at <= offset)
                    .map(|(_, l)| l)
                    .sum::<usize>()
        };
        for sec in control_sections(&obj) {
            for (prop, ty, offset, expr) in needed(&sec, base, &globals, extension_routines.as_ref()) {
                let offset = shifted(offset);
                text.push_str(&format!(
                    "\nprocedure {}(): {} begin exit({}); end;",
                    synthetic_name(&prop, offset),
                    ty.al(),
                    expr
                ));
                // A source that is a path to a place (`TotalPurchLine[1]."Inv. Discount Amount"`) can be written:
                // TestPage.SetValue on the control assigns it through this routine.
                if prop == "source" && assignable_path(&expr) {
                    text.push_str(&format!(
                        "\nprocedure {}(NewValue: Variant) begin {} := NewValue; end;",
                        synthetic_name("sourceset", offset),
                        expr
                    ));
                }
            }
        }
        if !text.is_empty() {
            inserts.push((close, text));
        }
        // The source of a dataset column is an expression of the data item's record: its routine
        // goes right after the column, inside the data item, so that the record is in scope.
        if kind.starts_with("report") {
            for (ordinal, sec) in column_sections(&obj).iter().enumerate() {
                let Some(expr) = sec.args_raw().get(1).cloned() else {
                    continue;
                };
                let at = usize::from(sec.syntax().text_range().end());
                inserts.push((
                    at,
                    format!(
                        "\nprocedure {}(): Variant begin exit({expr}); end;",
                        column_name(ordinal)
                    ),
                ));
            }
        }
    }
    if inserts.is_empty() {
        return None;
    }
    inserts.sort_by_key(|(at, _)| *at);
    let mut out = src.to_string();
    for (at, text) in inserts.iter().rev() {
        out.insert_str(*at, text);
    }
    // Each insert moves by the length of the ones before it.
    let mut added = 0;
    let ranges = inserts
        .iter()
        .map(|(at, text)| {
            let r = (at + added, at + added + text.len());
            added += text.len();
            r
        })
        .collect();
    Some((out, ranges))
}

/// The `column` sections of a report or reportextension dataset, in source order.
pub fn column_sections(obj: &ast::Object) -> Vec<ast::Section> {
    ast::sections_in(obj.syntax())
        .into_iter()
        .filter(|s| s.name().eq_ignore_ascii_case("column"))
        .collect()
}

/// Name of the synthetic routine that evaluates the `ordinal`-th column of a report (counted in
/// source order over the whole object, so inserting routines does not rename it).
pub fn column_name(ordinal: usize) -> String {
    format!("__column_{ordinal}")
}

/// A control found in the page's source, before bindings and routines are resolved.
pub struct RawControl {
    pub name: String,
    pub kind: al_ir::ControlKind,
    pub parent: Option<usize>,
    pub section: ast::Section,
    /// The page extension that declares the control (`None`: the page itself).
    pub owner: Option<usize>,
    /// `modify` blocks of page extensions that apply to this control, in the order the extensions
    /// apply: (extension object, the `modify` section).
    pub modifies: Vec<(usize, ast::Section)>,
}

/// The control tree of a page, from syntax alone, and why the client cannot
/// drive it yet.
pub struct Layout {
    pub controls: Vec<RawControl>,
    pub reasons: Vec<String>,
    /// The `area(...)` sections of `actions`, in source order, with the top-level controls each holds.
    pub action_areas: Vec<al_ir::ActionArea>,
    /// The unprobed assumptions this layout rests on (an extension adding into an implicit category group).
    pub assumes: Vec<al_builtins::assumptions::Assumption>,
}

/// A control in the tree the extensions are merged into.
struct Node {
    name: String,
    kind: al_ir::ControlKind,
    section: ast::Section,
    owner: Option<usize>,
    modifies: Vec<(usize, ast::Section)>,
    children: Vec<usize>,
    /// The control is only an anchor for extensions: it is not part of the page the test client sees
    /// (a `systempart`).
    anchor_only: bool,
}

/// An `area(...)` of the `layout` or `actions` section: where extensions attach controls by name.
struct Area {
    name: String,
    in_actions: bool,
    children: Vec<usize>,
}

/// Areas of the `actions` section a page extension may add to although the page declares none. Navigation, Creation and
/// Reporting: pages/W1ActionAreas.al:44-115 and :137-157 (ExtensionActions_InImplicitAreas_Run adds to all five on a page
/// with no actions, and each added action runs). The order of the areas among themselves is not pinned.
const IMPLICIT_ACTION_AREAS: &[&str] = &["Processing", "Prompting", "Navigation", "Creation", "Reporting"];

/// Promoted category groups of `actions` that a page extension may place actionrefs after although the page declares none.
const IMPLICIT_PROMOTED_CATEGORIES: &[&str] = &["Category_Process"];

/// A list of sibling controls: the children of an area or of a group-like control.
#[derive(Clone, Copy, PartialEq)]
enum ListId {
    Area(usize),
    Node(usize),
}

struct Tree {
    nodes: Vec<Node>,
    areas: Vec<Area>,
    reasons: Vec<String>,
    /// The areas of `layout` an extension may add to although the page declares none (`implicit_layout_areas`).
    implicit_areas: &'static [&'static str],
}

/// Areas of `layout` a page extension may add to on a Card page that declares none: pageext/lt-bc-implicit-areas/BcImplicitAreas.al
/// (FactBoxes on a card with only Content, :31-67; Content and FactBoxes on a card with no layout, :69-88).
const IMPLICIT_CARD_AREAS: &[&str] = &["Content", "FactBoxes"];

/// Areas of `layout` a page extension may add to on a List page that declares none: FactBoxes on a list with only Content
/// (pageext/lt-bc-implicit-areas/BcImplicitAreas.al:93-136, AL Runner issue 5674). Content on a List without a layout is not pinned.
const IMPLICIT_LIST_AREAS: &[&str] = &["FactBoxes"];

/// The implicit `layout` areas of a page of type `page_type`; other page types are not pinned.
fn implicit_layout_areas(page_type: &str) -> &'static [&'static str] {
    if page_type.eq_ignore_ascii_case("Card") {
        IMPLICIT_CARD_AREAS
    } else if page_type.eq_ignore_ascii_case("List") {
        IMPLICIT_LIST_AREAS
    } else {
        &[]
    }
}

/// Walks the `layout` and `actions` sections of page object `obj`.
pub fn layout(obj: &ast::Object) -> Layout {
    layout_with(obj, &[])
}

/// The control tree of page `obj` with the page extensions `exts` merged in, in the order given
/// (the order they apply: a later `modify` of a property wins).
pub fn layout_with(obj: &ast::Object, exts: &[(usize, ast::Object)]) -> Layout {
    let exts: Vec<(usize, Vec<ast::Section>)> = exts.iter().map(|(e, o)| (*e, o.sections().collect())).collect();
    let page_type = obj
        .properties()
        .find(|p| p.name().eq_ignore_ascii_case("PageType"))
        .map(|p| p.value_text().trim().to_string())
        .unwrap_or_default();
    layout_of(obj.sections(), &exts, implicit_layout_areas(&page_type))
}

/// The control tree among `sections` (the top-level sections of a page, or the sections of a
/// report's `requestpage`) with the sections of the extensions `exts` merged in, in the order given.
pub fn layout_from(sections: impl Iterator<Item = ast::Section>, exts: &[(usize, Vec<ast::Section>)]) -> Layout {
    layout_of(sections, exts, &[])
}

fn layout_of(
    sections: impl Iterator<Item = ast::Section>,
    exts: &[(usize, Vec<ast::Section>)],
    implicit_areas: &'static [&'static str],
) -> Layout {
    let mut tree = Tree {
        nodes: Vec::new(),
        areas: Vec::new(),
        reasons: Vec::new(),
        implicit_areas,
    };
    for top in sections {
        let n = top.name().to_ascii_lowercase();
        if n == "layout" || n == "actions" {
            let in_actions = n == "actions";
            // Controls written straight into `layout`/`actions` (no `area`) hang off an unnamed area.
            let mut loose = Vec::new();
            for s in top.sections() {
                if s.name().eq_ignore_ascii_case("area") {
                    let mut children = Vec::new();
                    tree.add_children(&mut children, &s, in_actions, None);
                    tree.areas.push(Area {
                        name: s.args().first().cloned().unwrap_or_default(),
                        in_actions,
                        children,
                    });
                } else {
                    tree.add_one(&mut loose, &s, in_actions, None);
                }
            }
            if !loose.is_empty() {
                tree.areas.push(Area {
                    name: String::new(),
                    in_actions,
                    children: loose,
                });
            }
        }
    }
    tree.merge(exts);
    tree.flatten()
}

/// One section of an extension that changes the page: `addlast(Target) { ... }` and the like.
struct Op {
    ext: usize,
    in_actions: bool,
    section: ast::Section,
}

impl Tree {
    fn add_children(&mut self, into: &mut Vec<usize>, section: &ast::Section, in_actions: bool, owner: Option<usize>) {
        for s in section.sections() {
            self.add_one(into, &s, in_actions, owner);
        }
    }

    /// The control (or, for `area`, the controls) that section `s` declares, appended to `into`.
    fn add_one(&mut self, into: &mut Vec<usize>, s: &ast::Section, in_actions: bool, owner: Option<usize>) {
        use al_ir::ControlKind as K;
        let name = s.name().to_ascii_lowercase();
        let arg0 = s.args().first().cloned().unwrap_or_default();
        let kind = match name.as_str() {
            "area" => {
                self.add_children(into, s, in_actions, owner);
                return;
            }
            "field" => K::Field,
            // A label IS a member of the TestPage: Visible, Enabled, Caption and Value (its caption), and not
            // Editable (handlers/TestPageLabelControlContracts.al:Label_IsATestPageMember_WithItsCaption).
            "label" => K::Field,
            "group" if in_actions => K::ActionGroup,
            "group" => K::Group,
            // The TestPage has no member for a cuegroup, fixed or grid control itself, but its fields and a
            // cuegroup's actions are members of the page (handlers/TestPageCueGroupFixedGridContracts.al:
            // CueGroup_Fields_ReadAndTheActionRuns, FixedAndGrid_Fields_ReadAndTakeSetValueLikeListFields,
            // FixedWithRepeater_MovesBetweenRowsLikeAList). They are groups that hold the controls.
            "cuegroup" => K::CueGroup,
            "fixed" | "grid" => K::Group,
            // The `actions` of a cuegroup hold its actions (PartPage_WithACueGroup_ReadsAndRunsItsAction).
            "actions" if !in_actions => {
                self.add_children(into, s, true, owner);
                return;
            }
            "repeater" => K::Repeater,
            // An actionref is a control that delegates to its action (corpus
            // handlers/TestPageActionRunObject_Tests.al:ActionrefToARunObjectActionOpensTheSameTarget,
            // handlers/TestPagePromotedActionref_Tests.al).
            "action" | "actionref" => K::Action,
            "systemaction" => K::SystemAction,
            "part" => K::Part,
            // The TestPage has no member for a system part (Notes, Links, MyNotes, ...), and the corpus
            // tests that open Base App pages declaring one (for example
            // handlers/TestPagePlatformPageSourceTable_Tests.al:77, handlers/TestReportRunFromPrecompiledCaller.al:31)
            // never reach it. A customaction (Power Automate;
            // pageextensioncontrol/TestPageExtensionModifyPrecompiled_Tests.al:69) and a usercontrol (a control add-in; likewise a fileuploadaction:
            // pageapplicationarea/TestPageApplicationAreaPrecompiledActionRemoval.al:39 opens Item List, which hosts one in a part) are treated the same way:
            // docs/probes-needed.md says what is not pinned. They stay in the tree so that `addafter`/
            // `modify` can name them, and are dropped from the controls the page lowers, so a TestPage
            // reference to one is refused ("control ... is not in the page layout"), never answered.
            "systempart" | "customaction" | "usercontrol" | "fileuploadaction" => {
                let idx = self.nodes.len();
                self.nodes.push(Node {
                    name: arg0,
                    kind: K::Part,
                    section: s.clone(),
                    owner,
                    modifies: Vec::new(),
                    children: Vec::new(),
                    anchor_only: true,
                });
                into.push(idx);
                return;
            }
            // A separator carries no behaviour of its own.
            "separator" => return,
            other => {
                self.reasons
                    .push(format!("page control kind `{other}` is not supported yet"));
                return;
            }
        };
        let idx = self.nodes.len();
        self.nodes.push(Node {
            name: arg0,
            kind: kind.clone(),
            section: s.clone(),
            owner,
            modifies: Vec::new(),
            children: Vec::new(),
            anchor_only: false,
        });
        into.push(idx);
        if matches!(kind, K::Group | K::Repeater | K::ActionGroup | K::CueGroup) {
            let mut kids = Vec::new();
            self.add_children(&mut kids, s, in_actions, owner);
            self.nodes[idx].children = kids;
        }
    }

    /// Whether an action of the page is promoted to category `cat` the legacy way (`PromotedCategory = cat`).
    fn promotes_to(&self, cat: &str) -> bool {
        self.nodes.iter().any(|n| {
            n.section.properties().any(|p| {
                p.name().eq_ignore_ascii_case("PromotedCategory") && p.value_text().trim().eq_ignore_ascii_case(cat)
            })
        })
    }

    fn find_node(&self, name: &str) -> Option<usize> {
        self.nodes.iter().position(|n| n.name.eq_ignore_ascii_case(name))
    }

    fn list(&self, id: ListId) -> &Vec<usize> {
        match id {
            ListId::Area(a) => &self.areas[a].children,
            ListId::Node(n) => &self.nodes[n].children,
        }
    }

    fn list_mut(&mut self, id: ListId) -> &mut Vec<usize> {
        match id {
            ListId::Area(a) => &mut self.areas[a].children,
            ListId::Node(n) => &mut self.nodes[n].children,
        }
    }

    /// The list that holds node `idx`, and its position there. Only nodes that are still
    /// reachable from an area count (a node whose declaration was refused is in no list).
    fn parent_list(&self, idx: usize) -> Option<(ListId, usize)> {
        let lists = (0..self.areas.len())
            .map(ListId::Area)
            .chain((0..self.nodes.len()).map(ListId::Node));
        for id in lists {
            if let Some(at) = self.list(id).iter().position(|&c| c == idx) {
                return Some((id, at));
            }
        }
        None
    }

    /// Applies one extension section. `Ok(false)` when its target is not in the tree yet.
    fn apply(&mut self, op: &Op) -> Result<bool, String> {
        let kind = op.section.name().to_ascii_lowercase();
        let target = op.section.args().first().cloned().unwrap_or_default();
        match kind.as_str() {
            "modify" => {
                let Some(n) = self.find_node(&target) else {
                    return Ok(false);
                };
                self.nodes[n].modifies.push((op.ext, op.section.clone()));
                Ok(true)
            }
            "addfirst" | "addlast" => {
                // An area of the same section, or a control that holds others.
                let area = self
                    .areas
                    .iter()
                    .position(|a| a.in_actions == op.in_actions && a.name.eq_ignore_ascii_case(&target));
                let holder = match area {
                    Some(a) => ListId::Area(a),
                    None => match self.find_node(&target) {
                        Some(n)
                            if matches!(
                                self.nodes[n].kind,
                                al_ir::ControlKind::Group
                                    | al_ir::ControlKind::CueGroup
                                    | al_ir::ControlKind::Repeater
                                    | al_ir::ControlKind::ActionGroup
                            ) =>
                        {
                            ListId::Node(n)
                        }
                        Some(_) => return Err(format!("{kind}({target}): `{target}` cannot hold controls")),
                        None => return Ok(false),
                    },
                };
                let mut fresh = Vec::new();
                self.add_children(&mut fresh, &op.section, op.in_actions, Some(op.ext));
                let list = self.list_mut(holder);
                if kind == "addfirst" {
                    list.splice(0..0, fresh);
                } else {
                    list.extend(fresh);
                }
                Ok(true)
            }
            "addafter" | "addbefore" => {
                let Some(n) = self.find_node(&target) else {
                    return Ok(false);
                };
                let Some((id, at)) = self.parent_list(n) else {
                    return Ok(false);
                };
                let mut fresh = Vec::new();
                self.add_children(&mut fresh, &op.section, op.in_actions, Some(op.ext));
                let at = if kind == "addafter" { at + 1 } else { at };
                self.list_mut(id).splice(at..at, fresh);
                Ok(true)
            }
            other => Err(format!("`{other}` in a page extension is not supported yet")),
        }
    }

    /// Merges the extensions in order. A section whose target another extension adds is retried
    /// until nothing more can be placed, so the order of the extensions never decides whether a
    /// target is found.
    fn merge(&mut self, exts: &[(usize, Vec<ast::Section>)]) {
        let mut pending: Vec<Op> = Vec::new();
        for (e, sections) in exts {
            for top in sections {
                let n = top.name().to_ascii_lowercase();
                if n != "layout" && n != "actions" {
                    continue;
                }
                for s in top.sections() {
                    pending.push(Op {
                        ext: *e,
                        in_actions: n == "actions",
                        section: s,
                    });
                }
            }
        }
        // The areas of `actions` that every page has, declared or not. Only the names corpus tests
        // reach are listed (see `IMPLICIT_ACTION_AREAS` and the test `an_extension_adds_to_the_processing_area_of_a_page_that_declares_none`).
        for op in &pending {
            let kind = op.section.name().to_ascii_lowercase();
            let target = op.section.args().first().cloned().unwrap_or_default();
            if op.in_actions
                && (kind == "addfirst" || kind == "addlast")
                && IMPLICIT_ACTION_AREAS.iter().any(|a| a.eq_ignore_ascii_case(&target))
                && !self
                    .areas
                    .iter()
                    .any(|a| a.in_actions && a.name.eq_ignore_ascii_case(&target))
            {
                self.areas.push(Area {
                    name: target,
                    in_actions: true,
                    children: Vec::new(),
                });
            }
        }
        for op in &pending {
            let kind = op.section.name().to_ascii_lowercase();
            let target = op.section.args().first().cloned().unwrap_or_default();
            if !op.in_actions
                && (kind == "addfirst" || kind == "addlast")
                && self.implicit_areas.iter().any(|a| a.eq_ignore_ascii_case(&target))
                && !self
                    .areas
                    .iter()
                    .any(|a| !a.in_actions && a.name.eq_ignore_ascii_case(&target))
            {
                self.areas.push(Area {
                    name: target,
                    in_actions: false,
                    children: Vec::new(),
                });
            }
        }
        // The promoted category groups that every page has. Only `Category_Process` is listed: it is the one
        // a corpus-reachable extension names (the test `an_extension_adds_after_the_process_category_...`).
        // The group is only an anchor: it is no control of the page.
        for op in &pending {
            let kind = op.section.name().to_ascii_lowercase();
            let target = op.section.args().first().cloned().unwrap_or_default();
            // A legacy promoted action (`PromotedCategory = Category4`) is in the implicit group
            // `Category_Category4`; an extension may add to it (Base Application's "Email Address Lookup"
            // extension; real BC compiles it). Nothing pins the group beyond being an anchor.
            let legacy = target
                .strip_prefix("Category_")
                .or_else(|| target.strip_prefix("category_"))
                .is_some_and(|cat| self.promotes_to(cat));
            if op.in_actions
                && (["addafter", "addbefore"].contains(&kind.as_str())
                    && IMPLICIT_PROMOTED_CATEGORIES
                        .iter()
                        .any(|c| c.eq_ignore_ascii_case(&target))
                    || legacy)
                && self.find_node(&target).is_none()
            {
                let idx = self.nodes.len();
                self.nodes.push(Node {
                    name: target,
                    kind: al_ir::ControlKind::ActionGroup,
                    section: op.section.clone(),
                    owner: None,
                    modifies: Vec::new(),
                    children: Vec::new(),
                    anchor_only: true,
                });
                let promoted = self
                    .areas
                    .iter()
                    .position(|a| a.in_actions && a.name.eq_ignore_ascii_case("Promoted"));
                let promoted = promoted.unwrap_or_else(|| {
                    self.areas.push(Area {
                        name: "Promoted".into(),
                        in_actions: true,
                        children: Vec::new(),
                    });
                    self.areas.len() - 1
                });
                self.areas[promoted].children.push(idx);
            }
        }
        loop {
            let before = pending.len();
            let mut later = Vec::new();
            for op in pending {
                match self.apply(&op) {
                    Ok(true) => {}
                    Ok(false) => later.push(op),
                    Err(why) => self.reasons.push(why),
                }
            }
            pending = later;
            if pending.is_empty() || pending.len() == before {
                break;
            }
        }
        for op in pending {
            let kind = op.section.name().to_ascii_lowercase();
            let target = op.section.args().first().cloned().unwrap_or_default();
            self.reasons.push(format!(
                "page extension: {kind}({target}): `{target}` is not on the page"
            ));
        }
    }

    /// The tree in source order: areas in turn, each control before its children.
    fn flatten(self) -> Layout {
        let Tree {
            nodes, areas, reasons, ..
        } = self;
        let mut slots: Vec<Option<Node>> = nodes.into_iter().map(Some).collect();
        let mut controls = Vec::new();
        fn emit(slots: &mut [Option<Node>], idx: usize, parent: Option<usize>, out: &mut Vec<RawControl>) {
            let Some(n) = slots[idx].take() else { return };
            // An anchor is no control; what was added into it is (`addlast(Category_Category4)`).
            if n.anchor_only {
                for c in n.children {
                    emit(slots, c, parent, out);
                }
                return;
            }
            let here = out.len();
            out.push(RawControl {
                name: n.name,
                kind: n.kind,
                parent,
                section: n.section,
                owner: n.owner,
                modifies: n.modifies,
            });
            for c in n.children {
                emit(slots, c, Some(here), out);
            }
        }
        let mut action_areas = Vec::new();
        for a in areas {
            let mut roots = Vec::new();
            for c in a.children {
                let first = controls.len();
                emit(&mut slots, c, None, &mut controls);
                // An anchor emits nothing, so it is no root.
                if controls.len() > first {
                    roots.push(first);
                }
            }
            if a.in_actions {
                action_areas.push(al_ir::ActionArea { name: a.name, roots });
            }
        }
        Layout {
            controls,
            reasons,
            action_areas,
            // addlast/addfirst on the implicit group of a legacy promoted category: pages/lt-page-ext-promoted/PageExtPromotedCategoryRisky.al.
            assumes: Vec::new(),
        }
    }
}

/// The right-hand side of one `SubPageLink` entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkRhs {
    /// `field(Name)`
    Field(String),
    /// `const(x)` / `filter(x)`: the filter text; quotes of a single-quoted literal are removed.
    Filter(String),
    /// `const(Database::"Name")`: the id of the named table.
    TableId(String),
}

/// `'H1'|'H3'` → `H1|H3`: the AL string literals of a `filter(...)` or `const(...)` value
/// without their quotes (`''` is a quote inside a literal); text outside quotes is kept.
fn unquote_literals(inner: &str) -> String {
    let mut out = String::new();
    let mut chars = inner.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\'' {
            out.push(c);
            continue;
        }
        while let Some(d) = chars.next() {
            if d == '\'' {
                if chars.peek() == Some(&'\'') {
                    chars.next();
                    out.push('\'');
                } else {
                    break;
                }
            } else {
                out.push(d);
            }
        }
    }
    out
}

/// `RunPageLink` has the shape of `SubPageLink`; the errors say which one it was.
pub fn parse_run_page_link(text: &str) -> Result<Vec<(String, LinkRhs)>, String> {
    parse_sub_page_link(text).map_err(|e| e.replace("SubPageLink", "RunPageLink"))
}

/// The value of a `RunObject` property: `Page "My Page"`, `Codeunit 80`, `Report Microsoft.Sales."X"`.
/// The object kind and the (possibly qualified) name or id, as parts.
pub fn parse_run_object(text: &str) -> Result<(al_ir::RunKind, Vec<String>), String> {
    let text = text.trim();
    let (word, rest) = text
        .split_once(char::is_whitespace)
        .ok_or_else(|| format!("RunObject `{text}`"))?;
    let kind = match word.to_ascii_lowercase().as_str() {
        "page" => al_ir::RunKind::Page,
        "codeunit" => al_ir::RunKind::Codeunit,
        "report" => al_ir::RunKind::Report,
        "xmlport" => al_ir::RunKind::XmlPort,
        "query" => al_ir::RunKind::Query,
        _ => return Err(format!("RunObject `{text}`")),
    };
    let mut parts = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    for c in rest.trim().chars() {
        match (quoted, c) {
            (_, '"') => quoted = !quoted,
            (false, '.') => parts.push(std::mem::take(&mut cur)),
            (_, c) => cur.push(c),
        }
    }
    parts.push(cur);
    if parts.iter().any(|p| p.trim().is_empty()) {
        return Err(format!("RunObject `{text}`"));
    }
    Ok((kind, parts.into_iter().map(|p| p.trim().to_string()).collect()))
}

/// Parses the value of a `SubPageLink` property: `A = field(B), C = const(x), D = filter(y | z)`.
/// `Err` names the part it does not understand.
pub fn parse_sub_page_link(text: &str) -> Result<Vec<(String, LinkRhs)>, String> {
    let mut out = Vec::new();
    for item in crate::scan::split_trimmed(text, ',') {
        let eq = {
            // The first `=` outside quotes separates the part's field from the rhs.
            let mut quote: Option<char> = None;
            item.char_indices().find_map(|(i, c)| match (quote, c) {
                (Some(q), c) if c == q => {
                    quote = None;
                    None
                }
                (Some(_), _) => None,
                (None, '\'' | '"') => {
                    quote = Some(c);
                    None
                }
                (None, '=') => Some(i),
                _ => None,
            })
        };
        let Some(eq) = eq else {
            return Err(format!("SubPageLink entry `{item}`"));
        };
        let left = crate::scan::unquote_double(&item[..eq]);
        let rhs = item[eq + 1..].trim();
        let open = rhs.find('(').ok_or_else(|| format!("SubPageLink entry `{item}`"))?;
        if !rhs.ends_with(')') {
            return Err(format!("SubPageLink entry `{item}`"));
        }
        let kind = rhs[..open].trim().to_ascii_lowercase();
        let inner = rhs[open + 1..rhs.len() - 1].trim();
        let parsed = match kind.as_str() {
            "field" => LinkRhs::Field(crate::scan::unquote_double(inner)),
            "const" | "filter" => {
                if let Some(t) = inner
                    .strip_prefix("Database::")
                    .or_else(|| inner.strip_prefix("database::"))
                {
                    LinkRhs::TableId(crate::scan::unquote_double(t))
                } else if inner.len() > 1
                    && inner.starts_with('"')
                    && inner.ends_with('"')
                    && !inner[1..inner.len() - 1].contains('"')
                {
                    // A quoted member name: `const("Released Production Order")` is the member, not text with quotes.
                    LinkRhs::Filter(inner[1..inner.len() - 1].to_string())
                } else {
                    LinkRhs::Filter(unquote_literals(inner))
                }
            }
            other => return Err(format!("SubPageLink with {other}(...)")),
        };
        out.push((left, parsed));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `expand_or_parse` hands back the parse of a text it adds nothing to: the tree a parse of that text gives, so the
    /// workspace need not parse the file a second time.
    #[test]
    fn a_text_with_nothing_to_expand_comes_back_with_its_parse() {
        let symbols = BTreeSet::new();
        let plain = [
            "codeunit 50100 C { procedure P() begin end; }",
            "table 50101 T { fields { field(1; K; Code[10]) { } } }\npage 50102 P { SourceTable = T; layout { area(Content) { field(K; Rec.K) { } } } }",
            "codeunit 50103 Broken { procedure P( begin end; }",
        ];
        for src in plain {
            match expand_or_parse(src, &symbols) {
                Expansion::Unchanged(parse) => {
                    let again = al_syntax::parse(src, &symbols);
                    assert_eq!(parse.green, again.green, "{src}");
                    assert_eq!(parse.errors, again.errors, "{src}");
                }
                Expansion::Expanded(..) => panic!("nothing to add to {src}"),
            }
            assert_eq!(expand_ranges(src, &symbols), None);
        }
        let page = "table 50101 T { fields { field(1; K; Code[10]) { } } }\npage 50102 P { SourceTable = T; layout { area(Content) { field(K; Rec.K) { Visible = Flag; } } } var Flag: Boolean; }";
        match expand_or_parse(page, &symbols) {
            Expansion::Expanded(text, ranges) => assert_eq!(Some((text, ranges)), expand_ranges(page, &symbols)),
            Expansion::Unchanged(_) => panic!("Visible = Flag is expanded"),
        }
    }

    fn routines(src: &str) -> Vec<String> {
        let parse = al_syntax::parse(src, &Default::default());
        let root = ast::SourceFile::cast(parse.syntax()).unwrap();
        assert!(parse.errors.is_empty(), "{:?}", parse.errors);
        root.objects()
            .flat_map(|o| {
                o.syntax()
                    .descendants()
                    .filter_map(ast::Routine::cast)
                    .collect::<Vec<_>>()
            })
            .map(|r| r.name())
            .collect()
    }

    // Corpus handlers/TestPageControlPropertyLiveness_Tests.al: `Visible = not HideIt`
    // style properties are expressions over page state.
    #[test]
    fn expression_properties_become_synthetic_routines() {
        let src = "page 50100 P { layout { area(Content) { field(A; Rec.X) { Visible = not Hide; Editable = true; } field(B; Hide) { } field(C; Format(Rec.X)) { } } } var Hide: Boolean; }";
        let out = expand(src, &Default::default()).expect("a page with expressions");
        let names = routines(&out);
        let vis = src.find("Visible").unwrap();
        let c = src.find("field(C").unwrap();
        // Names count from the start of the object (here: 0).
        assert_eq!(names, vec![synthetic_name("visible", vis), synthetic_name("source", c)]);
        assert!(out.starts_with(&src[..src.len() - 1]), "original text is only added to");
    }

    // Microsoft's page "Check Credit Limit" has `label(Control2) { CaptionClass = Format(StrSubstNo(Text000, Heading)); }`,
    // reached by Cust-Check Cr. Limit in ~850 Microsoft tests: a label is a control with expression properties.
    #[test]
    fn a_label_with_an_expression_caption_class_gets_a_routine() {
        let src = "page 50100 P { layout { area(Content) { label(Control2) { CaptionClass = Format(StrSubstNo(Text000, Heading)); } } } var Heading: Text; Text000: Label 'x %1'; }";
        let out = expand(src, &Default::default()).expect("a label with an expression");
        let at = src.find("CaptionClass").unwrap();
        assert_eq!(routines(&out), vec![synthetic_name("captionclass", at)]);
    }

    #[test]
    fn literal_properties_and_other_objects_are_left_alone() {
        assert_eq!(
            expand(
                "page 50100 P { layout { area(Content) { field(A; Rec.X) { Visible = false; } } } }",
                &Default::default()
            ),
            None
        );
        assert_eq!(
            expand("codeunit 50100 C { procedure P() begin end; }", &Default::default()),
            None
        );
    }

    #[test]
    fn client_compiled_control_expressions_that_bc_fails_on_are_named() {
        use super::{ClientExpr, client_expression};
        let refused = |e: &str| matches!(client_expression(e), Some(ClientExpr::Refused(_)));
        // pages/lt-cc-page-leads/CcPageLeads.al:764, :846, :880.
        assert!(refused("GetRow().Flag"));
        assert!(refused("Rec.D = 0D"));
        assert!(refused("GlobalRow.Grp = 2"));
        assert!(refused("Rec.T = 0T"));
        assert!(refused("IsOk()"));
        assert!(matches!(
            client_expression("0L <> Rec.Big"),
            Some(ClientExpr::BigIntegerLiteral(_))
        ));
        // Shapes the corpus and Microsoft's apps run.
        for ok in [
            "not HideIt",
            "Rec.Flag",
            "Rec.Value <> ''",
            "Rec.Value <> 'a.b(c)'",
            "(Rec.LineType = Rec.LineType::One) or (Rec.LineType = Rec.LineType::Two)",
            "Rec.\"Report Output Type\" = Rec.\"Report Output Type\"::Print",
            "(SourceItemTrackingEntryType = \"Item Tracking Entry Type\"::\"Serial No.\")",
            "this.UserGreetingVisible",
            "Cnt > 3",
            "(2 = 3)",
            "not (HideIt or SecondFlag)",
            "Rec.Amount > 1.5",
        ] {
            assert_eq!(client_expression(ok), None, "{ok}");
        }
    }

    #[test]
    fn sources_classify_as_record_field_global_or_expression() {
        let globals: BTreeSet<String> = ["glob".to_string()].into();
        assert_eq!(classify_source("Rec.Num", &globals), Source::RecField("Num".into()));
        assert_eq!(
            classify_source("Rec.\"Entry No.\"", &globals),
            Source::RecField("Entry No.".into())
        );
        assert_eq!(classify_source("Glob", &globals), Source::Var("Glob".into()));
        assert_eq!(classify_source("Format(Rec.Num)", &globals), Source::Expr);
    }

    fn merged(base: &str, exts: &[&str]) -> Layout {
        let objects = |src: &str| {
            let parse = al_syntax::parse(src, &Default::default());
            assert!(parse.errors.is_empty(), "{:?}", parse.errors);
            ast::SourceFile::cast(parse.syntax())
                .unwrap()
                .objects()
                .collect::<Vec<_>>()
        };
        let base = objects(base).remove(0);
        let exts: Vec<(usize, ast::Object)> = exts
            .iter()
            .enumerate()
            .map(|(i, e)| (i + 1, objects(e).remove(0)))
            .collect();
        layout_with(&base, &exts)
    }

    fn names(l: &Layout) -> Vec<String> {
        l.controls
            .iter()
            .map(|c| match c.parent {
                Some(p) => format!("{}>{}", l.controls[p].name, c.name),
                None => c.name.clone(),
            })
            .collect()
    }

    // Corpus pageextensioncontrol/TestPageExtensionControlTrapped_Page.al (`addafter("Rec Text")`),
    // handlers/TestPageExtensionActionWithPart_Page.al (`addfirst(FactBoxes)`), handlers/TestPageExtensionActionInvoke_Page.al
    // (`addlast(Processing)`): extension controls land where the section names them.
    #[test]
    fn extension_sections_place_controls_first_last_after_and_before() {
        let base = "page 50100 P { layout { area(Content) { group(G) { field(A; Rec.A) { } field(B; Rec.B) { } } field(Z; Rec.Z) { } } area(FactBoxes) { } } actions { area(Processing) { action(Act) { } } } }";
        let ext = "pageextension 50101 E extends P {
            layout {
                addfirst(G) { field(First; Rec.F) { } }
                addafter(A) { field(AfterA; Rec.F) { } }
                addbefore(B) { field(BeforeB; Rec.F) { } }
                addlast(Content) { field(Last; Rec.F) { } }
                addfirst(FactBoxes) { part(Box; Other) { } }
            }
            actions { addlast(Processing) { action(ExtAct) { } } addafter(Act) { action(AfterAct) { } } }
        }";
        let l = merged(base, &[ext]);
        assert!(l.reasons.is_empty(), "{:?}", l.reasons);
        assert_eq!(
            names(&l),
            [
                "G",
                "G>First",
                "G>A",
                "G>AfterA",
                "G>BeforeB",
                "G>B",
                "Z",
                "Last",
                "Box",
                "Act",
                "AfterAct",
                "ExtAct"
            ]
        );
        assert_eq!(l.controls.iter().filter(|c| c.owner == Some(1)).count(), 7);
    }

    // The order the extensions are given never decides whether a target is found.
    #[test]
    fn a_target_another_extension_adds_is_found_whatever_the_order() {
        let base = "page 50100 P { layout { area(Content) { field(A; Rec.A) { } } } }";
        let adds = "pageextension 50101 E1 extends P { layout { addlast(Content) { group(Extra) { } } } }";
        let into = "pageextension 50102 E2 extends P { layout { addlast(Extra) { field(Inner; Rec.F) { } } } }";
        for order in [[adds, into], [into, adds]] {
            let l = merged(base, &order);
            assert!(l.reasons.is_empty(), "{:?}", l.reasons);
            assert_eq!(names(&l), ["A", "Extra", "Extra>Inner"]);
        }
    }

    #[test]
    fn modify_blocks_attach_to_their_control_in_order_and_unknown_targets_are_reasons() {
        let base = "page 50100 P { layout { area(Content) { field(A; Rec.A) { } } } }";
        let e1 = "pageextension 50101 E1 extends P { layout { modify(A) { Caption = 'one'; } } }";
        let e2 = "pageextension 50102 E2 extends P { layout { modify(A) { Caption = 'two'; } addlast(Nowhere) { } movefirst(A; A) { } } }";
        let l = merged(base, &[e1, e2]);
        let a = &l.controls[0];
        assert_eq!(a.modifies.iter().map(|(e, _)| *e).collect::<Vec<_>>(), [1, 2]);
        assert_eq!(l.reasons.len(), 2, "{:?}", l.reasons);
        assert!(l.reasons.iter().any(|r| r.contains("Nowhere")));
        assert!(l.reasons.iter().any(|r| r.contains("movefirst")));
    }

    // Corpus handlers/TestPagePlatformPageSourceTable_Tests.al:77 (ReadsTheWholePositionedRow) and the other tests that open a
    // Base App page declaring `systempart(Control; Notes)`: the TestPage has no member for a system part, so
    // it is not a control of the page. An extension can still anchor on it.
    #[test]
    fn a_systempart_is_not_a_control_but_still_an_anchor() {
        let base = "page 50100 P { layout { area(Content) { field(A; Rec.A) { } label(Hint) { } } area(FactBoxes) { systempart(Control1; Notes) { } systempart(Control2; Links) { } } } actions { area(Processing) { group(Flow) { customaction(MakeFlow) { CustomActionType = FlowTemplateGallery; } } } } }";
        let ext = "pageextension 50101 E extends P { layout { addafter(Control1) { field(AfterNotes; Rec.F) { } } } }";
        let l = merged(base, &[ext]);
        assert!(l.reasons.is_empty(), "{:?}", l.reasons);
        // A label is a control (handlers/TestPageLabelControlContracts.al); the others are not.
        assert_eq!(names(&l), ["A", "Hint", "AfterNotes", "Flow"]);
    }

    // A customaction (or a label, a systempart, a usercontrol) stays in the tree as an anchor and is no control
    // of the page, so it is no root of an action area either. A root past the controls panicked the Page Action
    // table over handlers/TestPageLabelControlContracts.al: page "Probe Label Kinds" (`action(Plain)` then
    // `customaction(Cust)` in area(Processing)).
    #[test]
    fn an_action_area_has_no_root_for_a_customaction() {
        let base = "page 50100 P { layout { area(Content) { field(A; Rec.A) { } } } actions { area(Processing) { action(Plain) { } customaction(Cust) { } } } }";
        let l = merged(base, &[]);
        assert_eq!(names(&l), ["A", "Plain"]);
        let roots: Vec<usize> = l.action_areas.iter().flat_map(|a| a.roots.clone()).collect();
        assert_eq!(roots, [1]);
    }

    // A page that declares no `actions` still has the area an extension adds to. Microsoft's own extensions of
    // Base App pages do it and compile in real BC; the corpus tests that open them pass there
    // (handlers/TestPageModalHandler_PrecompiledPage.al:31 RunModalOnErrorMessages_HandlerInvokesOk_ReturnsActionOk
    // over "Error Messages" + `addfirst(Processing)`, RunModalOnNoSeries_HandlerInvokesOk_ReturnsActionOk over
    // "No. Series" + `addfirst(Prompting)`).
    #[test]
    fn an_extension_adds_to_the_factboxes_and_content_of_a_card_that_declares_neither() {
        // pageext/lt-bc-implicit-areas/BcImplicitAreaObjects.al: pages 80965 (Content only) and 80969 (no layout).
        let card = "page 50100 P { PageType = Card; layout { area(Content) { group(General) { field(Number; Rec.Number) { } } } } }";
        let links =
            "pageextension 50101 E extends P { layout { addfirst(FactBoxes) { systempart(BCLinks; Links) { } } } }";
        let part = "pageextension 50102 F extends P { layout { addlast(FactBoxes) { part(BCPart; Q) { } } } }";
        let after = "pageextension 50103 G extends P { layout { addafter(BCLinks) { part(BCPartAfter; Q) { } } } }";
        let l = merged(card, &[links, part, after]);
        assert!(l.reasons.is_empty(), "{:?}", l.reasons);
        // A systempart is no control of the layout; it is still an anchor for `addafter`.
        assert_eq!(names(&l), ["General", "General>Number", "BCPartAfter", "BCPart"]);
        let bare = "page 50100 P { PageType = Card; }";
        let ext = "pageextension 50101 E extends P { layout { addfirst(Content) { field(BCNumber; Rec.Number) { } } addlast(FactBoxes) { part(BCNoLayoutPart; Q) { } } } }";
        let l = merged(bare, &[ext]);
        assert!(l.reasons.is_empty(), "{:?}", l.reasons);
        assert_eq!(names(&l), ["BCNumber", "BCNoLayoutPart"]);
        // A List takes FactBoxes too (BcImplicitAreas.al:93-136); another page type is not pinned.
        let list = "page 50100 P { PageType = List; layout { area(Content) { repeater(R) { field(Number; Rec.Number) { } } } } }";
        let l = merged(list, &[part]);
        assert!(l.reasons.is_empty(), "{:?}", l.reasons);
        let document =
            "page 50100 P { PageType = Document; layout { area(Content) { field(Number; Rec.Number) { } } } }";
        let l = merged(document, &[part]);
        assert_eq!(l.reasons.len(), 1, "{:?}", l.reasons);
    }

    #[test]
    fn an_extension_adds_to_the_processing_area_of_a_page_that_declares_none() {
        let base = "page 50100 P { layout { area(Content) { field(A; Rec.A) { } } } }";
        let ext = "pageextension 50101 E extends P { actions { addfirst(Processing) { action(Act) { } } addlast(Prompting) { action(Ask) { } } } }";
        let l = merged(base, &[ext]);
        assert!(l.reasons.is_empty(), "{:?}", l.reasons);
        assert_eq!(names(&l), ["A", "Act", "Ask"]);
        assert_eq!(
            l.action_areas.iter().map(|a| a.name.as_str()).collect::<Vec<_>>(),
            ["Processing", "Prompting"]
        );
    }

    // Every page has the promoted category groups, declared or not: the same Microsoft extension of "Error Messages"
    // does `addafter(Category_Process) { actionref(...) }` on a page with no actions
    // (handlers/TestPageModalHandler_PrecompiledPage.al:31, pageextensioncontrol/TestPageExtensionModifyPrecompiled_Tests.al:69).
    #[test]
    fn an_extension_adds_after_the_process_category_of_a_page_that_declares_none() {
        let base = "page 50100 P { layout { area(Content) { field(A; Rec.A) { } } } }";
        let ext = "pageextension 50101 E extends P { actions { addfirst(Processing) { action(Act) { } } addafter(Category_Process) { actionref(Act_Promoted; Act) { } } } }";
        let l = merged(base, &[ext]);
        assert!(l.reasons.is_empty(), "{:?}", l.reasons);
        assert_eq!(names(&l), ["A", "Act", "Act_Promoted"]);
    }

    // A legacy promoted action (`Promoted = true; PromotedCategory = Category4`) puts the action in the implicit
    // group `Category_Category4`; Base Application's "Email Address Lookup" extension adds actionrefs to it with
    // `addlast(Category_Category4)`. Real BC compiles it (issue 40). The group is an anchor; the actionrefs are controls.
    #[test]
    fn an_extension_adds_to_the_implicit_group_of_a_legacy_promoted_category() {
        let base = "page 50100 P { layout { area(Content) { field(A; Rec.A) { } } } actions { area(Processing) { action(Act) { Promoted = true; PromotedCategory = Category4; } } } }";
        let ext = "pageextension 50101 E extends P { actions { addlast(Category_Category4) { actionref(Act_Promoted; Act) { } } } }";
        let l = merged(base, &[ext]);
        assert!(l.reasons.is_empty(), "{:?}", l.reasons);
        assert_eq!(names(&l), ["A", "Act", "Act_Promoted"]);
        // A category no action of the page is promoted to stays refused.
        let ext = "pageextension 50101 E extends P { actions { addlast(Category_Category7) { actionref(Act_Promoted; Act) { } } } }";
        let l = merged(base, &[ext]);
        assert!(l.reasons.join(";").contains("Category_Category7"), "{:?}", l.reasons);
    }

    // An area name nothing pins is still refused.
    #[test]
    fn an_extension_adding_to_an_unknown_area_is_still_refused() {
        let base = "page 50100 P { layout { area(Content) { field(A; Rec.A) { } } } }";
        let ext = "pageextension 50101 E extends P { actions { addfirst(Nowhere) { action(Act) { } } } }";
        let l = merged(base, &[ext]);
        assert_eq!(l.reasons.len(), 1, "{:?}", l.reasons);
    }

    // handlers/TestPageCueGroupFixedGridContracts.al: cuegroup, fixed and grid hold controls; a cuegroup's
    // `actions` hold its actions; a repeater may sit inside a fixed control.
    #[test]
    fn cuegroup_fixed_and_grid_hold_their_controls() {
        let base = "page 50100 P { layout { area(Content) { cuegroup(CG) { field(Cnt; Rec.Cnt) { } actions { action(CueAct) { } } }
            fixed(FX) { field(Txt; Rec.Txt) { } repeater(RR) { field(Amt; Rec.Amt) { } } }
            grid(GR) { group(GRA) { field(Flag; Rec.Flag) { } } } } } }";
        let l = merged(base, &[]);
        assert!(l.reasons.is_empty(), "{:?}", l.reasons);
        assert_eq!(
            names(&l),
            [
                "CG",
                "CG>Cnt",
                "CG>CueAct",
                "FX",
                "FX>Txt",
                "FX>RR",
                "RR>Amt",
                "GR",
                "GR>GRA",
                "GRA>Flag"
            ]
        );
        assert_eq!(l.controls[0].kind, al_ir::ControlKind::CueGroup);
    }

    // Corpus handlers/TestPagePromotedActionref_Page.al: an actionref is a control that names its target.
    #[test]
    fn an_actionref_is_a_control() {
        let base = "page 50100 P { actions { area(Processing) { action(Act) { } } area(Promoted) { actionref(Act_Promoted; Act) { } } } }";
        let l = merged(base, &[]);
        assert_eq!(names(&l), ["Act", "Act_Promoted"]);
        assert_eq!(l.controls[1].kind, al_ir::ControlKind::Action);
        assert!(l.controls[1].section.name().eq_ignore_ascii_case("actionref"));
        assert_eq!(l.controls[1].section.args(), ["Act_Promoted", "Act"]);
    }

    // A page field whose source is a lone quoted name with spaces or punctuation
    // (`field("Assign-to No."; "Assign-to No.")` in Base Application) keeps its quotes in the routine
    // that reads it; unquoted, `exit(Assign-to No.)` parses as a subtraction. The quotes of the
    // control's name are not part of the name (handlers/TestPageActionRunPageView.al names controls
    // the same way).
    #[test]
    fn a_lone_quoted_field_source_keeps_its_quotes_in_the_routine() {
        let src = "page 50100 P { SourceTable = T; layout { area(Content) { field(\"Assign-to No.\"; \"Assign-to No.\") { Visible = Show; } } } var Show: Boolean; }";
        let out = expand(src, &Default::default()).expect("a field with an expression source");
        assert!(out.contains("exit(\"Assign-to No.\")"), "{out}");
        assert!(!out.contains("exit(Assign-to"), "{out}");
    }

    // Corpus pageextensioncontrol/TestPageExtensionModifyEditable_Tests.al: `Editable = not PXMSLocked;` in a
    // `modify` block of a page extension is an expression over the extension's variables.
    #[test]
    fn expression_properties_of_page_extensions_and_modify_blocks_become_routines_of_the_extension() {
        let src = "pageextension 50101 E extends P { layout { modify(A) { Editable = not Locked; } addlast(Content) { field(B; Own) { Visible = Show; } } } var Locked: Boolean; Own: Integer; Show: Boolean; }";
        let out = expand(src, &Default::default()).expect("an extension with expressions");
        let names = routines(&out);
        assert_eq!(names.len(), 2, "{names:?}");
        assert!(names.iter().all(|n| is_synthetic(n)), "{names:?}");
    }

    // A quoted name in const(..) is the name (Base Application: `\"Document Type\" = const(\"Released Production Order\")`).
    #[test]
    fn a_quoted_member_name_in_a_link_constant_loses_its_quotes() {
        let got = parse_sub_page_link("\"Document Type\" = const(\"Released Production Order\")").unwrap();
        assert_eq!(
            got,
            vec![(
                "Document Type".to_string(),
                LinkRhs::Filter("Released Production Order".into())
            )]
        );
    }

    // Corpus handlers/TestPageSubpagePartConstFilter.al, TestPagePartLinkFilterGroup.al:
    // SubPageLink forms field(...), const(...), filter(...).
    #[test]
    fn sub_page_link_entries_parse() {
        let got = parse_sub_page_link(
            "\"Header No.\" = field(\"No.\"), Kind = const(Attachment), Status = filter(Open | Released), Cat = const('SPECIAL'), \"Table ID\" = const(Database::\"TSPL Header\")",
        )
        .unwrap();
        assert_eq!(
            got,
            vec![
                ("Header No.".to_string(), LinkRhs::Field("No.".into())),
                ("Kind".to_string(), LinkRhs::Filter("Attachment".into())),
                ("Status".to_string(), LinkRhs::Filter("Open | Released".into())),
                ("Cat".to_string(), LinkRhs::Filter("SPECIAL".into())),
                ("Table ID".to_string(), LinkRhs::TableId("TSPL Header".into())),
            ]
        );
        assert!(parse_sub_page_link("A = bogus(B)").is_err());
    }

    // Corpus handlers/TestPageActionRunPageLink.al:ActionRunPageLink_Filter_AppliesTheFilterExpression:
    // `filter('H1' | 'H3')` selects both values.
    #[test]
    fn quoted_literals_in_a_filter_lose_their_quotes_one_by_one() {
        let got = parse_run_page_link("\"Head No.\" = filter('H1' | 'H3'), B = const('it''s')").unwrap();
        assert_eq!(
            got,
            vec![
                ("Head No.".to_string(), LinkRhs::Filter("H1 | H3".into())),
                ("B".to_string(), LinkRhs::Filter("it's".into())),
            ]
        );
        assert!(
            parse_run_page_link("A = bogus(B)")
                .unwrap_err()
                .starts_with("RunPageLink")
        );
    }

    // Corpus handlers/TestPageActionRunObject_Objects.al: `RunObject = Page "TPARO Card Target"`.
    #[test]
    fn run_object_names_a_kind_and_an_object() {
        use al_ir::RunKind;
        assert_eq!(
            parse_run_object("Page \"TPARO Card Target\"").unwrap(),
            (RunKind::Page, vec!["TPARO Card Target".to_string()])
        );
        assert_eq!(
            parse_run_object("codeunit 80").unwrap(),
            (RunKind::Codeunit, vec!["80".to_string()])
        );
        assert_eq!(
            parse_run_object("Report Microsoft.Sales.\"Sales. Report\"").unwrap(),
            (
                RunKind::Report,
                vec!["Microsoft".into(), "Sales".into(), "Sales. Report".into()]
            )
        );
        assert!(parse_run_object("Table 18").is_err());
        assert!(parse_run_object("Page").is_err());
    }
}
