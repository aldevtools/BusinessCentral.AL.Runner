//! Typed views over the untyped tree. Each wrapper is a cheap handle around a
//! `SyntaxNode` of one kind; accessors read children on demand.

use crate::{SyntaxKind, SyntaxNode, SyntaxToken};

macro_rules! ast_node {
    ($name:ident, $kind:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub struct $name(pub SyntaxNode);
        impl $name {
            pub fn cast(node: SyntaxNode) -> Option<Self> {
                (node.kind() == SyntaxKind::$kind).then(|| Self(node))
            }
            pub fn syntax(&self) -> &SyntaxNode {
                &self.0
            }
        }
    };
}

ast_node!(SourceFile, SourceFile);
ast_node!(Object, Object);
ast_node!(Property, Property);
ast_node!(Section, Section);
ast_node!(VarSection, VarSection);
ast_node!(VarDecl, VarDecl);
ast_node!(TypeRef, TypeRef);
ast_node!(Param, Param);
ast_node!(Attribute, Attribute);

/// A procedure or a trigger.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Routine(pub SyntaxNode);
impl Routine {
    pub fn cast(node: SyntaxNode) -> Option<Self> {
        matches!(node.kind(), SyntaxKind::Procedure | SyntaxKind::Trigger).then(|| Self(node))
    }
    pub fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

/// Every routine under `node`, in source order. A routine's body holds only
/// statements, so unlike `descendants()` this does not walk into routines.
pub fn routines_in(node: &SyntaxNode) -> Vec<Routine> {
    let mut out = Vec::new();
    walk_outside_routines(node, |n| {
        if let Some(r) = Routine::cast(n.clone()) {
            out.push(r);
        }
    });
    out
}

/// Every `Section` under `node`, in source order, not looking inside routines
/// (sections are object structure; a routine body never contains one).
pub fn sections_in(node: &SyntaxNode) -> Vec<Section> {
    let mut out = Vec::new();
    walk_outside_routines(node, |n| {
        if n.kind() == SyntaxKind::Section {
            out.push(Section(n.clone()));
        }
    });
    out
}

/// Preorder over `node`'s descendants, calling `f` on each node and skipping the
/// children of procedures and triggers (the routine node itself is visited).
fn walk_outside_routines(node: &SyntaxNode, mut f: impl FnMut(&SyntaxNode)) {
    let mut it = node.preorder();
    while let Some(ev) = it.next() {
        if let rowan::WalkEvent::Enter(n) = ev {
            f(&n);
            if matches!(n.kind(), SyntaxKind::Procedure | SyntaxKind::Trigger) {
                it.skip_subtree();
            }
        }
    }
}

/// AL identifiers compare case-insensitively; quoted ones lose their quotes.
pub fn ident_text(tok: &SyntaxToken) -> String {
    unquote_ident(tok.text())
}

/// A quoted name loses its quotes and a doubled quote inside it becomes one quote. Text that is not quoted is
/// returned as it is.
pub fn unquote_ident(t: &str) -> String {
    match t.strip_prefix('"').and_then(|t| t.strip_suffix('"')) {
        Some(inner) => inner.replace("\"\"", "\""),
        None => t.to_string(),
    }
}

fn significant_tokens(node: &SyntaxNode) -> impl Iterator<Item = SyntaxToken> {
    node.children_with_tokens()
        .filter_map(|e| e.into_token())
        .filter(|t| !t.kind().is_trivia())
}

fn children<T>(node: &SyntaxNode, cast: fn(SyntaxNode) -> Option<T>) -> impl Iterator<Item = T> {
    node.children().filter_map(cast)
}

fn trimmed(node: &SyntaxNode) -> String {
    node.text().to_string().trim().to_string()
}

/// The text of `node` without the preprocessor directives (`#pragma`, ...), comments and disabled
/// text inside it, trimmed. A property value such as a `CalcFormula` can span lines that carry them.
fn trimmed_code(node: &SyntaxNode) -> String {
    let text = node.text().to_string();
    if !text.contains(['#', '/']) {
        return text.trim().to_string();
    }
    let mut out = String::with_capacity(text.len());
    for t in node.descendants_with_tokens().filter_map(|e| e.into_token()) {
        if !matches!(
            t.kind(),
            SyntaxKind::Directive | SyntaxKind::LineComment | SyntaxKind::BlockComment | SyntaxKind::DisabledText
        ) {
            out.push_str(t.text());
        }
    }
    out.trim().to_string()
}

/// Joins `A . B . "C"` style token runs into `A.B.C`.
fn qualified(tokens: &[SyntaxToken]) -> String {
    tokens
        .iter()
        .map(|t| {
            if t.kind() == SyntaxKind::Dot {
                ".".to_string()
            } else {
                ident_text(t)
            }
        })
        .collect()
}

fn is_name(t: &SyntaxToken) -> bool {
    matches!(t.kind(), SyntaxKind::Ident | SyntaxKind::QuotedIdent | SyntaxKind::Dot)
}

/// One `type("Full.Name"; "Alias")` of a `dotnet { assembly(..) { } }` block.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DotNetTypeDecl {
    pub assembly: String,
    pub version: Option<String>,
    pub culture: Option<String>,
    pub public_key_token: Option<String>,
    /// The CLR type name as written (generic names keep the CLR spelling).
    pub type_name: String,
    pub alias: String,
    /// Offset of the `type` section in the file.
    pub offset: usize,
}

/// `'text', Locked = true` / `, MaxLength = 40` / `, Comment = '..'`: a text property keeps its literal and drops the attributes that follow
/// it (corpus pageext/lt-page-ext-properties/TableKeyProps.al:P123_TableCaptionsLocked, P123_MaxLengthCaptions). On a `Caption`,
/// `MaxLength = n` cuts the literal to n characters, as it cuts a Label (record/lt-u-table-field-props/UTableTextSearch.al:41-44: a
/// table caption and a field caption; text/lt-label-textbuilder/TxCLabel.al:282). On any other property a text longer than its
/// MaxLength is not pinned: the value keeps its attributes, [`text_over_max_length`] finds it and the readers refuse it.
pub(crate) fn strip_text_attributes(property: &str, text: String) -> String {
    if !text.starts_with('\'') {
        return text;
    }
    let bytes = text.as_bytes();
    let mut i = 1;
    while i < bytes.len() {
        if bytes[i] == b'\'' {
            if bytes.get(i + 1) == Some(&b'\'') {
                i += 2;
                continue;
            }
            let rest = text[i + 1..].trim_start();
            if let Some(after) = rest.strip_prefix(',') {
                let name = after
                    .trim_start()
                    .split(|c: char| !c.is_ascii_alphabetic())
                    .next()
                    .unwrap_or("");
                if ["Locked", "MaxLength", "Comment"]
                    .iter()
                    .any(|a| a.eq_ignore_ascii_case(name))
                {
                    let literal = &text[..=i];
                    return match max_length_attribute(&text[i + 1..]) {
                        Some(n) if !property.eq_ignore_ascii_case("Caption") => {
                            let len = literal[1..literal.len() - 1].replace("''", "'").chars().count();
                            if len > n { text } else { literal.to_string() }
                        }
                        Some(n) => {
                            let inner = literal[1..literal.len() - 1].replace("''", "'");
                            let cut: String = inner.chars().take(n).collect();
                            format!("'{}'", cut.replace('\'', "''"))
                        }
                        None => literal.to_string(),
                    };
                }
            }
            return text;
        }
        i += 1;
    }
    text
}

/// A property value that kept its attributes because its text is longer than its `MaxLength` (see [`strip_text_attributes`]).
pub fn text_over_max_length(value: &str) -> bool {
    value.starts_with('\'')
        && strip_text_attributes("Caption", value.to_string()) != strip_text_attributes("x", value.to_string())
}

/// The `n` of a `MaxLength = n` among the attributes after a text literal (`, Comment = '..', MaxLength = 5`).
fn max_length_attribute(attrs: &str) -> Option<usize> {
    let mut quoted = false;
    let mut start = 0;
    let mut parts = Vec::new();
    for (i, c) in attrs.char_indices() {
        match c {
            '\'' => quoted = !quoted,
            ',' if !quoted => {
                parts.push(&attrs[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(&attrs[start..]);
    parts.into_iter().find_map(|p| {
        let (k, v) = p.split_once('=')?;
        k.trim()
            .eq_ignore_ascii_case("MaxLength")
            .then(|| v.trim().parse().ok())
            .flatten()
    })
}

fn unquote_literal(v: &str) -> String {
    let v = v.trim();
    match v.strip_prefix('\'').and_then(|r| r.strip_suffix('\'')) {
        Some(inner) => inner.replace("''", "'"),
        None => v.to_string(),
    }
}

impl SourceFile {
    /// Every `type(..)` declared by the file's `dotnet { }` blocks, in source order.
    pub fn dotnet_types(&self) -> Vec<DotNetTypeDecl> {
        let mut out = Vec::new();
        for block in self.0.children().filter(|n| n.kind() == SyntaxKind::DotNetBlock) {
            let members = block
                .children()
                .filter(|n| n.kind() == SyntaxKind::ObjectBody)
                .flat_map(|b| b.children());
            for asm in members.filter_map(Section::cast) {
                if !asm.name().eq_ignore_ascii_case("assembly") {
                    continue;
                }
                let assembly = asm.args().into_iter().next().unwrap_or_default();
                let prop = |n: &str| {
                    asm.properties()
                        .find(|p| p.name().eq_ignore_ascii_case(n))
                        .map(|p| unquote_literal(&p.value_text()))
                };
                for ty in asm.sections().filter(|t| t.name().eq_ignore_ascii_case("type")) {
                    let args = ty.args();
                    let Some(type_name) = args.first().cloned() else {
                        continue;
                    };
                    // `type("System.Convert")` with no alias is the type's own simple name.
                    let alias = args
                        .get(1)
                        .cloned()
                        .unwrap_or_else(|| type_name.rsplit('.').next().unwrap_or(&type_name).to_string());
                    out.push(DotNetTypeDecl {
                        assembly: assembly.clone(),
                        version: prop("Version"),
                        culture: prop("Culture"),
                        public_key_token: prop("PublicKeyToken"),
                        type_name,
                        alias,
                        offset: ty.syntax().text_range().start().into(),
                    });
                }
            }
        }
        out
    }
    pub fn objects(&self) -> impl Iterator<Item = Object> {
        children(&self.0, Object::cast)
    }
    pub fn namespace(&self) -> Option<String> {
        let ns = self.0.children().find(|n| n.kind() == SyntaxKind::NamespaceDecl)?;
        let toks: Vec<_> = significant_tokens(&ns).skip(1).filter(is_name).collect();
        Some(qualified(&toks))
    }
    pub fn usings(&self) -> Vec<String> {
        self.0
            .children()
            .filter(|n| n.kind() == SyntaxKind::UsingDecl)
            .map(|u| qualified(&significant_tokens(&u).skip(1).filter(is_name).collect::<Vec<_>>()))
            .collect()
    }
}

impl Object {
    fn header(&self) -> Vec<SyntaxToken> {
        significant_tokens(&self.0).collect()
    }
    fn body(&self) -> Option<SyntaxNode> {
        self.0.children().find(|n| n.kind() == SyntaxKind::ObjectBody)
    }
    fn members(&self) -> impl Iterator<Item = SyntaxNode> {
        self.body().into_iter().flat_map(|b| b.children())
    }
    /// Lower-case object kind keyword: `codeunit`, `tableextension`, ...
    pub fn kind(&self) -> String {
        self.header()
            .first()
            .map(|t| t.text().to_ascii_lowercase())
            .unwrap_or_default()
    }
    pub fn id(&self) -> Option<u32> {
        self.header()
            .iter()
            .find(|t| t.kind() == SyntaxKind::Integer)
            .and_then(|t| t.text().parse().ok())
    }
    /// Header words after the name: (`extends`|`implements`|`customizes`, names as parts).
    fn clause_parts(&self) -> Vec<(String, Vec<Vec<String>>)> {
        let toks = self.header();
        let mut out = Vec::new();
        let is_clause = |t: &SyntaxToken| {
            ["extends", "implements", "customizes"]
                .iter()
                .any(|k| t.text().eq_ignore_ascii_case(k))
        };
        let mut i = toks.iter().position(is_clause).unwrap_or(toks.len());
        while i < toks.len() && is_clause(&toks[i]) {
            let kw = toks[i].text().to_ascii_lowercase();
            i += 1;
            let mut names = Vec::new();
            loop {
                let mut parts = Vec::new();
                // A target by its id (`extends 50100`) is one part.
                while i < toks.len()
                    && (is_name(&toks[i]) || toks[i].kind() == SyntaxKind::Integer)
                    && !is_clause(&toks[i])
                {
                    if toks[i].kind() != SyntaxKind::Dot {
                        parts.push(ident_text(&toks[i]));
                    }
                    i += 1;
                }
                names.push(parts);
                if toks.get(i).is_some_and(|t| t.kind() == SyntaxKind::Comma) {
                    i += 1;
                } else {
                    break;
                }
            }
            out.push((kw, names));
        }
        out
    }
    pub fn extends_parts(&self) -> Option<Vec<String>> {
        self.clause_parts()
            .into_iter()
            .find(|(k, _)| k == "extends")
            .and_then(|(_, n)| n.into_iter().next())
    }
    /// The object a `pagecustomization` customizes, as name parts.
    pub fn customizes_parts(&self) -> Option<Vec<String>> {
        self.clause_parts()
            .into_iter()
            .find(|(k, _)| k == "customizes")
            .and_then(|(_, n)| n.into_iter().next())
    }
    pub fn implements_parts(&self) -> Vec<Vec<String>> {
        self.clause_parts()
            .into_iter()
            .find(|(k, _)| k == "implements")
            .map(|(_, n)| n)
            .unwrap_or_default()
    }
    fn clauses(&self) -> (String, Vec<(String, Vec<String>)>) {
        let toks = self.header();
        let mut i = 1;
        if toks.get(i).is_some_and(|t| t.kind() == SyntaxKind::Integer) {
            i += 1;
        }
        let is_clause = |t: &SyntaxToken| {
            ["extends", "implements", "customizes"]
                .iter()
                .any(|k| t.text().eq_ignore_ascii_case(k))
        };
        let start = i;
        while i < toks.len() && is_name(&toks[i]) && !is_clause(&toks[i]) {
            i += 1;
        }
        let name = qualified(&toks[start..i]);
        let mut clauses = Vec::new();
        while i < toks.len() && is_clause(&toks[i]) {
            let kw = toks[i].text().to_ascii_lowercase();
            i += 1;
            let mut names = Vec::new();
            loop {
                let s = i;
                while i < toks.len()
                    && (is_name(&toks[i]) || toks[i].kind() == SyntaxKind::Integer)
                    && !is_clause(&toks[i])
                {
                    i += 1;
                }
                names.push(qualified(&toks[s..i]));
                if toks.get(i).is_some_and(|t| t.kind() == SyntaxKind::Comma) {
                    i += 1;
                } else {
                    break;
                }
            }
            clauses.push((kw, names));
        }
        (name, clauses)
    }
    pub fn name(&self) -> String {
        self.clauses().0
    }
    pub fn extends(&self) -> Option<String> {
        self.clauses()
            .1
            .into_iter()
            .find(|(k, _)| k == "extends")
            .and_then(|(_, n)| n.into_iter().next())
    }
    pub fn implements(&self) -> Vec<String> {
        self.clauses()
            .1
            .into_iter()
            .find(|(k, _)| k == "implements")
            .map(|(_, n)| n)
            .unwrap_or_default()
    }
    pub fn properties(&self) -> impl Iterator<Item = Property> {
        self.members().filter_map(Property::cast)
    }
    /// Offset of the `}` that closes the object body.
    pub fn close_brace_offset(&self) -> Option<usize> {
        let b = self.body()?;
        let t = b.last_token()?;
        (t.kind() == SyntaxKind::RBrace).then(|| usize::from(t.text_range().start()))
    }
    pub fn var_decls(&self) -> impl Iterator<Item = VarDecl> {
        self.members()
            .filter_map(VarSection::cast)
            .flat_map(|s| s.decls().collect::<Vec<_>>())
    }
    /// The variables a report or report extension declares in a `var` section of its `requestpage`: the globals of the
    /// request page, which only the triggers of that request page see.
    pub fn request_page_var_decls(&self) -> impl Iterator<Item = VarDecl> {
        self.sections()
            .filter(|s| s.name().eq_ignore_ascii_case("requestpage"))
            .flat_map(|s| s.var_decls().collect::<Vec<_>>())
    }
    /// The variables of the `protected var` sections: the ones a page or report extension may name.
    pub fn protected_var_decls(&self) -> impl Iterator<Item = VarDecl> {
        self.members()
            .filter_map(VarSection::cast)
            .filter(|s| s.is_protected())
            .flat_map(|s| s.decls().collect::<Vec<_>>())
    }
    pub fn routines(&self) -> impl Iterator<Item = Routine> {
        self.members()
            .filter(|n| n.kind() == SyntaxKind::Procedure)
            .filter_map(Routine::cast)
    }
    pub fn triggers(&self) -> impl Iterator<Item = Routine> {
        self.members()
            .filter(|n| n.kind() == SyntaxKind::Trigger)
            .filter_map(Routine::cast)
    }
    pub fn sections(&self) -> impl Iterator<Item = Section> {
        self.members().filter_map(Section::cast)
    }
}

impl Property {
    pub fn name(&self) -> String {
        significant_tokens(&self.0)
            .next()
            .map(|t| ident_text(&t))
            .unwrap_or_default()
    }
    pub fn value_text(&self) -> String {
        let text = self
            .0
            .children()
            .find(|n| n.kind() == SyntaxKind::PropertyValue)
            .map(|v| trimmed_code(&v))
            .unwrap_or_default();
        strip_text_attributes(&self.name(), text)
    }
}

impl Section {
    pub fn name(&self) -> String {
        significant_tokens(&self.0)
            .next()
            .map(|t| t.text().to_string())
            .unwrap_or_default()
    }
    /// The `;`-separated arguments in parentheses, each as trimmed source text
    /// (a lone quoted identifier loses its quotes).
    pub fn args(&self) -> Vec<String> {
        self.args_as(true)
    }
    /// `args`, with a lone quoted identifier keeping its quotes: the text can be put back into code.
    pub fn args_raw(&self) -> Vec<String> {
        self.args_as(false)
    }
    fn args_as(&self, unquote: bool) -> Vec<String> {
        self.split_args()
            .into_iter()
            .map(|sig| match sig.as_slice() {
                [] => String::new(),
                [one] if unquote && one.kind() == SyntaxKind::QuotedIdent => ident_text(one),
                [first, .., last] => self.text_between(first, last),
                [one] => one.text().to_string(),
            })
            .collect()
    }

    /// Argument `i` read as an object name: the parts of `Ns.Sub."Name"`, quotes removed. A lone
    /// quoted name is one part even when it holds dots (`"Pstd. Phys. Invt. Order Hdr"`). `None`
    /// when the argument is not a name (an expression, a type with a size).
    pub fn arg_name_parts(&self, i: usize) -> Option<Vec<String>> {
        let sig = self.split_args().into_iter().nth(i)?;
        let mut parts = Vec::new();
        let mut expect_name = true;
        for t in &sig {
            if expect_name {
                if !matches!(t.kind(), SyntaxKind::Ident | SyntaxKind::QuotedIdent) {
                    return None;
                }
                parts.push(ident_text(t));
            } else if t.kind() != SyntaxKind::Dot {
                return None;
            }
            expect_name = !expect_name;
        }
        (!parts.is_empty() && !expect_name).then_some(parts)
    }

    /// The significant tokens of each `;`-separated argument.
    fn split_args(&self) -> Vec<Vec<SyntaxToken>> {
        let Some(args) = self.0.children().find(|n| n.kind() == SyntaxKind::SectionArgs) else {
            return Vec::new();
        };
        let toks: Vec<_> = args.descendants_with_tokens().filter_map(|e| e.into_token()).collect();
        let inner = &toks[1..toks.len().saturating_sub(1)];
        let mut out = Vec::new();
        let mut cur: Vec<SyntaxToken> = Vec::new();
        let mut depth = 0;
        for t in inner {
            match t.kind() {
                SyntaxKind::LParen | SyntaxKind::LBrack => depth += 1,
                SyntaxKind::RParen | SyntaxKind::RBrack => depth -= 1,
                SyntaxKind::Semicolon if depth == 0 => {
                    out.push(std::mem::take(&mut cur));
                    continue;
                }
                _ => {}
            }
            if !t.kind().is_trivia() {
                cur.push(t.clone());
            }
        }
        if !inner.is_empty() {
            out.push(cur);
        }
        out
    }

    /// The source text from token `first` to token `last`, trimmed.
    fn text_between(&self, first: &SyntaxToken, last: &SyntaxToken) -> String {
        // Only the tokens from `first` to `last` are read: the section's own text can be a whole page layout, and a
        // slice of that text walks all of it.
        let mut out = String::new();
        let mut t = first.clone();
        loop {
            out.push_str(t.text());
            if t == *last {
                break;
            }
            match t.next_token() {
                Some(n) => t = n,
                None => break,
            }
        }
        out.trim().to_string()
    }
    fn members(&self) -> impl Iterator<Item = SyntaxNode> {
        self.0
            .children()
            .filter(|n| n.kind() == SyntaxKind::ObjectBody)
            .flat_map(|b| b.children())
    }
    pub fn properties(&self) -> impl Iterator<Item = Property> {
        self.members().filter_map(Property::cast)
    }
    pub fn sections(&self) -> impl Iterator<Item = Section> {
        self.members().filter_map(Section::cast)
    }
    pub fn triggers(&self) -> impl Iterator<Item = Routine> {
        self.members().filter_map(Routine::cast)
    }
    /// The variables of the `var` sections inside this section (the `var` of a `requestpage`).
    pub fn var_decls(&self) -> impl Iterator<Item = VarDecl> {
        self.members()
            .filter_map(VarSection::cast)
            .flat_map(|s| s.decls().collect::<Vec<_>>())
    }
}

/// Whether the source text of `node` holds `needle` (lower case ASCII), whatever the case, without building the text as one
/// string. Every chunk is one token, and `needle` is a name, so it cannot span two of them.
pub fn text_contains_ignore_case(node: &SyntaxNode, needle: &str) -> bool {
    let n = needle.as_bytes();
    let Some(&first) = n.first() else { return true };
    node.text()
        .try_for_each_chunk(|chunk| {
            let b = chunk.as_bytes();
            let found = b.len() >= n.len()
                && (0..=b.len() - n.len())
                    .any(|i| b[i].to_ascii_lowercase() == first && b[i..i + n.len()].eq_ignore_ascii_case(n));
            if found { Err(()) } else { Ok(()) }
        })
        .is_err()
}

impl VarSection {
    pub fn decls(&self) -> impl Iterator<Item = VarDecl> {
        children(&self.0, VarDecl::cast)
    }
    pub fn is_protected(&self) -> bool {
        significant_tokens(&self.0)
            .next()
            .is_some_and(|t| t.text().eq_ignore_ascii_case("protected"))
    }
}

impl VarDecl {
    pub fn names(&self) -> Vec<String> {
        significant_tokens(&self.0)
            .take_while(|t| t.kind() != SyntaxKind::Colon)
            .filter(|t| matches!(t.kind(), SyntaxKind::Ident | SyntaxKind::QuotedIdent))
            .map(|t| ident_text(&t))
            .collect()
    }
    pub fn type_ref(&self) -> Option<TypeRef> {
        children(&self.0, TypeRef::cast).next()
    }
    /// The attributes written before the declaration (`[RunOnClient] B: DotNet X;`): siblings of it in the
    /// `var` section, nearest last.
    pub fn attributes(&self) -> Vec<Attribute> {
        let mut out = Vec::new();
        let mut at = self.0.prev_sibling();
        while let Some(n) = at {
            match Attribute::cast(n.clone()) {
                Some(a) => out.push(a),
                None => break,
            }
            at = n.prev_sibling();
        }
        out.reverse();
        out
    }
}

impl TypeRef {
    pub fn text(&self) -> String {
        trimmed(&self.0)
    }
    /// `Record`, `Text`, `List`, `array`, ...
    pub fn type_name(&self) -> String {
        significant_tokens(&self.0)
            .next()
            .map(|t| t.text().to_string())
            .unwrap_or_default()
    }
    /// For object types, the referenced object (`"ALT Universal"` → `ALT Universal`,
    /// `18` → `18`, `System.Utilities."Temp Blob"` → `System.Utilities.Temp Blob`).
    pub fn object_name(&self) -> Option<String> {
        let toks: Vec<_> = significant_tokens(&self.0).skip(1).collect();
        let first = toks.first()?;
        if first.kind() == SyntaxKind::Integer {
            return Some(first.text().to_string());
        }
        let run: Vec<_> = toks
            .iter()
            .take_while(|t| {
                is_name(t) && !t.text().eq_ignore_ascii_case("temporary") && !t.text().eq_ignore_ascii_case("of")
            })
            .cloned()
            .collect();
        (!run.is_empty() && first.kind() != SyntaxKind::StringLit).then(|| qualified(&run))
    }
    /// The object reference split into namespace segments and the name
    /// (`System.Utilities."Temp Blob"` → `["System", "Utilities", "Temp Blob"]`).
    pub fn object_name_parts(&self) -> Vec<String> {
        let toks: Vec<_> = significant_tokens(&self.0).skip(1).collect();
        match toks.first() {
            Some(t) if t.kind() == SyntaxKind::Integer => vec![t.text().to_string()],
            Some(t) if t.kind() == SyntaxKind::StringLit => Vec::new(),
            _ => toks
                .iter()
                .take_while(|t| {
                    is_name(t) && !t.text().eq_ignore_ascii_case("temporary") && !t.text().eq_ignore_ascii_case("of")
                })
                .filter(|t| t.kind() != SyntaxKind::Dot)
                .map(ident_text)
                .collect(),
        }
    }
    pub fn is_temporary(&self) -> bool {
        significant_tokens(&self.0).any(|t| t.text().eq_ignore_ascii_case("temporary"))
    }
    /// Integer arguments in brackets: `Text[30]` → `[30]`, `array[2, 3] of T` → `[2, 3]`.
    pub fn size_args(&self) -> Vec<u32> {
        self.0
            .children()
            .filter(|n| n.kind() == SyntaxKind::TypeArgs)
            .flat_map(|a| {
                a.children()
                    .filter(|n| n.kind() == SyntaxKind::Literal)
                    .collect::<Vec<_>>()
            })
            .filter_map(|l| l.text().to_string().trim().parse().ok())
            .collect()
    }
    /// `Option ,A,"B C"` → `["", "A", "B C"]`.
    pub fn option_members(&self) -> Vec<String> {
        let toks: Vec<_> = significant_tokens(&self.0).skip(1).collect();
        let mut out = vec![String::new()];
        for t in toks {
            match t.kind() {
                SyntaxKind::Comma => out.push(String::new()),
                SyntaxKind::Ident | SyntaxKind::QuotedIdent | SyntaxKind::Integer => {
                    // A quoted empty name (`""`) shows as one space; an unquoted empty member shows its number
                    // (types/lt-scalars/ScT70053.al:Option_Format `blank_len` against misc/lt-inferred-behaviors/P69OptionInText.al:
                    // Option_StrSubstNo `subst_2`). The two are told apart by keeping the quoted one as a space.
                    let name = ident_text(&t);
                    *out.last_mut().expect("non-empty") = if name.is_empty() && t.kind() == SyntaxKind::QuotedIdent {
                        " ".into()
                    } else {
                        name
                    }
                }
                _ => {}
            }
        }
        out
    }
    /// Nested type arguments (`List of [T]`, `array[..] of T`).
    pub fn type_args(&self) -> Vec<TypeRef> {
        let mut out: Vec<TypeRef> = children(&self.0, TypeRef::cast).collect();
        for args in self.0.children().filter(|n| n.kind() == SyntaxKind::TypeArgs) {
            out.extend(children(&args, TypeRef::cast));
        }
        out
    }
}

impl Routine {
    pub fn is_trigger(&self) -> bool {
        self.0.kind() == SyntaxKind::Trigger
    }
    fn is_routine_keyword(t: &SyntaxToken) -> bool {
        t.text().eq_ignore_ascii_case("procedure") || t.text().eq_ignore_ascii_case("trigger")
    }
    pub fn name(&self) -> String {
        let mut toks = significant_tokens(&self.0).skip_while(|t| !Self::is_routine_keyword(t));
        let Some(first) = toks.nth(1) else {
            return String::new();
        };
        let mut name = ident_text(&first);
        // `trigger Variable::Event(...)`: the handler's name has both parts.
        if let (Some(sep), Some(event)) = (toks.next(), toks.next()) {
            if sep.kind() == SyntaxKind::ColonColon {
                name = format!("{name}::{}", ident_text(&event));
            }
        }
        name
    }
    /// Offset of the `procedure` or `trigger` keyword (after any attributes and modifiers).
    pub fn keyword_offset(&self) -> Option<usize> {
        significant_tokens(&self.0)
            .find(Self::is_routine_keyword)
            .map(|t| usize::from(t.text_range().start()))
    }
    /// Offset of the `begin` that opens the body (a routine without a body has none).
    pub fn begin_offset(&self) -> Option<usize> {
        let tok = self.body()?.first_token()?;
        tok.text()
            .eq_ignore_ascii_case("begin")
            .then(|| usize::from(tok.text_range().start()))
    }
    /// Offset of the `end` that closes the body (a routine without a body has none).
    pub fn end_offset(&self) -> Option<usize> {
        let mut t = self.0.last_token();
        while let Some(tok) = t {
            if tok.kind().is_trivia() || tok.text() == ";" {
                t = tok.prev_token();
            } else {
                return tok
                    .text()
                    .eq_ignore_ascii_case("end")
                    .then(|| usize::from(tok.text_range().start()));
            }
        }
        None
    }
    fn has_modifier(&self, m: &str) -> bool {
        significant_tokens(&self.0)
            .take_while(|t| !Self::is_routine_keyword(t))
            .any(|t| t.text().eq_ignore_ascii_case(m))
    }
    pub fn is_local(&self) -> bool {
        self.has_modifier("local")
    }
    pub fn is_internal(&self) -> bool {
        self.has_modifier("internal")
    }
    pub fn is_protected(&self) -> bool {
        self.has_modifier("protected")
    }
    pub fn attributes(&self) -> impl Iterator<Item = Attribute> {
        children(&self.0, Attribute::cast)
    }
    pub fn params(&self) -> impl Iterator<Item = Param> {
        self.0
            .children()
            .filter(|n| n.kind() == SyntaxKind::ParamList)
            .flat_map(|l| children(&l, Param::cast).collect::<Vec<_>>())
    }
    fn return_spec(&self) -> Option<SyntaxNode> {
        self.0.children().find(|n| n.kind() == SyntaxKind::ReturnSpec)
    }
    pub fn return_name(&self) -> Option<String> {
        let first = significant_tokens(&self.return_spec()?).next()?;
        (first.kind() != SyntaxKind::Colon).then(|| ident_text(&first))
    }
    pub fn return_type(&self) -> Option<TypeRef> {
        children(&self.return_spec()?, TypeRef::cast).next()
    }
    pub fn local_var_decls(&self) -> impl Iterator<Item = VarDecl> {
        children(&self.0, VarSection::cast).flat_map(|s| s.decls().collect::<Vec<_>>())
    }
    pub fn body(&self) -> Option<SyntaxNode> {
        self.0.children().find(|n| n.kind() == SyntaxKind::Block)
    }
}

impl Param {
    pub fn is_var(&self) -> bool {
        significant_tokens(&self.0)
            .next()
            .is_some_and(|t| t.text().eq_ignore_ascii_case("var"))
    }
    pub fn name(&self) -> String {
        significant_tokens(&self.0)
            .find(|t| {
                matches!(t.kind(), SyntaxKind::Ident | SyntaxKind::QuotedIdent) && !t.text().eq_ignore_ascii_case("var")
            })
            .map(|t| ident_text(&t))
            .unwrap_or_default()
    }
    pub fn type_ref(&self) -> Option<TypeRef> {
        children(&self.0, TypeRef::cast).next()
    }
}

impl Attribute {
    pub fn name(&self) -> String {
        significant_tokens(&self.0)
            .nth(1)
            .map(|t| ident_text(&t))
            .unwrap_or_default()
    }
    pub fn args(&self) -> Option<SyntaxNode> {
        self.0.children().find(|n| n.kind() == SyntaxKind::ArgList)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unquoting_an_identifier_unescapes_a_doubled_quote() {
        assert_eq!(unquote_ident("\"A.\"\"B (C)\"\"\""), "A.\"B (C)\"");
        assert_eq!(unquote_ident("\"Plain name\""), "Plain name");
        assert_eq!(unquote_ident("Bare"), "Bare");
        assert_eq!(unquote_ident("\"\""), "");
    }

    #[test]
    fn a_section_argument_read_as_an_object_name() {
        let f = file(
            "report 1 R { dataset { dataitem(A; System.Utilities.Integer) { } dataitem(B; Ns.\"Q. Name\") { } dataitem(C; \"Pstd. Hdr. Q\") { } dataitem(D; Cust) { } field(E; Rec.F) { } } }",
        );
        let obj = f.objects().next().unwrap();
        let secs = sections_in(obj.syntax());
        let names = |n: usize| secs[n].arg_name_parts(1);
        let s = |v: &[&str]| Some(v.iter().map(|x| x.to_string()).collect::<Vec<_>>());
        assert_eq!(names(1), s(&["System", "Utilities", "Integer"]));
        assert_eq!(names(2), s(&["Ns", "Q. Name"]));
        assert_eq!(names(3), s(&["Pstd. Hdr. Q"]));
        assert_eq!(names(4), s(&["Cust"]));
        assert_eq!(secs[1].args()[1], "System.Utilities.Integer");
        assert_eq!(secs[3].args()[1], "Pstd. Hdr. Q");
        // `Rec.F` also reads as two parts: the caller decides whether the section takes a name.
        assert_eq!(names(5), s(&["Rec", "F"]));
        assert_eq!(secs[2].arg_name_parts(5), None);
    }

    #[test]
    fn a_section_argument_keeps_its_inner_spacing_and_drops_the_outer() {
        let f = file("page 1 P { layout { area(Content) { field(  X ;   Rec.A  +  1 ; ) { } } } }");
        let secs = sections_in(f.objects().next().unwrap().syntax());
        let field = secs.iter().find(|s| s.name() == "field").unwrap();
        assert_eq!(
            field.args(),
            vec!["X".to_string(), "Rec.A  +  1".to_string(), String::new()]
        );
    }

    #[test]
    fn text_is_searched_without_regard_to_case_across_tokens() {
        let f = file("table 1 T { fields { field(1; A; Integer) { CaptionClass = 'x'; } } }");
        let o = f.objects().next().unwrap();
        assert!(text_contains_ignore_case(o.syntax(), "captionclass"));
        assert!(text_contains_ignore_case(o.syntax(), "integer"));
        assert!(!text_contains_ignore_case(o.syntax(), "rendering"));
    }

    fn file(src: &str) -> SourceFile {
        let p = crate::parse(src, &Default::default());
        assert!(p.errors.is_empty(), "{:?}", p.errors);
        SourceFile::cast(p.syntax()).unwrap()
    }

    const SRC: &str = r#"namespace A.B;
codeunit 60100 "Test X" implements IOne, "I Two"
{
    Subtype = Test;
    Permissions = tabledata "ALT Universal" = RIMD;

    var
        Assert: Codeunit Assert;
        A, B: Record "ALT Universal" temporary;

    trigger OnRun()
    begin
    end;

    [Test]
    [HandlerFunctions('H')]
    local procedure T(var R: Record Customer; N: Integer) Result: Text[30]
    var
        L: List of [Integer];
    begin
    end;
}
tableextension 50100 Ext extends "Sales Header" { fields { field(50100; F; Integer) { } } }
"#;

    #[test]
    fn dotnet_blocks_list_their_types() {
        let f = file(
            "dotnet\n{\n assembly(\"mscorlib\")\n {\n  Version = '4.0.0.0';\n  Culture = 'neutral';\n  PublicKeyToken = 'b77a5c561934e089';\n  type(\"System.Convert\"; \"Convert\") { }\n  type(\"System.Text.Encoding\"; Encoding) { }\n }\n assembly(Other) { type(\"A.B`1[[System.String, mscorlib]]\"; \"Gen\") { } }\n}\ncodeunit 1 C { }",
        );
        let d = f.dotnet_types();
        assert_eq!(d.len(), 3);
        assert_eq!(
            (d[0].assembly.as_str(), d[0].type_name.as_str(), d[0].alias.as_str()),
            ("mscorlib", "System.Convert", "Convert")
        );
        assert_eq!(d[0].version.as_deref(), Some("4.0.0.0"));
        assert_eq!(d[0].culture.as_deref(), Some("neutral"));
        assert_eq!(d[0].public_key_token.as_deref(), Some("b77a5c561934e089"));
        assert_eq!(d[1].alias, "Encoding");
        assert_eq!(
            (d[2].assembly.as_str(), d[2].type_name.as_str(), d[2].alias.as_str()),
            ("Other", "A.B`1[[System.String, mscorlib]]", "Gen")
        );
        assert_eq!(d[2].version, None);
        assert_eq!(f.objects().count(), 1);
    }

    #[test]
    fn source_file_lists_objects_and_namespace() {
        let f = file(SRC);
        assert_eq!(f.namespace().as_deref(), Some("A.B"));
        let objs: Vec<_> = f.objects().map(|o| (o.kind(), o.id(), o.name())).collect();
        assert_eq!(
            objs,
            vec![
                ("codeunit".to_string(), Some(60100), "Test X".to_string()),
                ("tableextension".to_string(), Some(50100), "Ext".to_string())
            ]
        );
    }

    #[test]
    fn object_header_extends_and_implements() {
        let f = file(SRC);
        let objs: Vec<_> = f.objects().collect();
        assert_eq!(objs[0].implements(), vec!["IOne", "I Two"]);
        assert_eq!(objs[0].extends(), None);
        assert_eq!(objs[1].extends().as_deref(), Some("Sales Header"));
        assert_eq!(objs[1].extends_parts(), Some(vec!["Sales Header".to_string()]));
        assert_eq!(
            objs[0].implements_parts(),
            vec![vec!["IOne".to_string()], vec!["I Two".to_string()]]
        );
    }

    #[test]
    fn a_property_value_drops_directives_and_comments_inside_it() {
        // Base Application's Item Journal Line "Reserved Qty. (Base)" has `#pragma warning disable`
        // lines inside its CalcFormula; a directive or comment is not part of the value.
        let o = file(
            "table 1 T { fields { field(1; F; Decimal) { CalcFormula = sum(L.Q where(A = field(B),\n#pragma warning disable AL0603\n C = field(D), // note\n#pragma warning restore\n E = const(0))); } } }",
        )
        .objects()
        .next()
        .unwrap();
        let f = o
            .sections()
            .flat_map(|s| s.sections().collect::<Vec<_>>())
            .next()
            .expect("field");
        let v = f.properties().next().expect("CalcFormula").value_text();
        assert!(!v.contains('#') && !v.contains("note"), "{v:?}");
        assert!(v.contains("C = field(D)") && v.contains("E = const(0)"), "{v:?}");
    }

    #[test]
    fn properties_by_name_and_raw_value() {
        let o = file(SRC).objects().next().unwrap();
        let props: Vec<_> = o.properties().map(|p| (p.name(), p.value_text())).collect();
        assert_eq!(
            props,
            vec![
                ("Subtype".to_string(), "Test".to_string()),
                (
                    "Permissions".to_string(),
                    "tabledata \"ALT Universal\" = RIMD".to_string()
                )
            ]
        );
    }

    #[test]
    fn global_variables_with_types() {
        let o = file(SRC).objects().next().unwrap();
        let vars: Vec<_> = o
            .var_decls()
            .map(|v| (v.names(), v.type_ref().unwrap().text()))
            .collect();
        assert_eq!(
            vars,
            vec![
                (vec!["Assert".to_string()], "Codeunit Assert".to_string()),
                (
                    vec!["A".to_string(), "B".to_string()],
                    "Record \"ALT Universal\" temporary".to_string()
                )
            ]
        );
    }

    #[test]
    fn type_ref_parts() {
        let o = file(SRC).objects().next().unwrap();
        let t = o.var_decls().nth(1).unwrap().type_ref().unwrap();
        assert_eq!(t.type_name(), "Record");
        assert_eq!(t.object_name().as_deref(), Some("ALT Universal"));
        assert!(t.is_temporary());
        assert_eq!(t.object_name_parts(), vec!["ALT Universal"]);
    }

    #[test]
    fn variable_declarations_may_carry_attributes() {
        // Microsoft's source: ItemVendor.Table.al, `[SecurityFiltering(SecurityFilter::Ignored)]`.
        let f = file(
            "codeunit 1 C { var [NonDebuggable] G: Text; procedure P() var [SecurityFiltering(SecurityFilter::Ignored)] R: Record Item; L: Integer; begin end; }",
        );
        let o = f.objects().next().unwrap();
        assert_eq!(
            o.var_decls().map(|v| v.names()).collect::<Vec<_>>(),
            vec![vec!["G".to_string()]]
        );
        let r = o.routines().next().unwrap();
        assert_eq!(
            r.local_var_decls().map(|v| v.names()).collect::<Vec<_>>(),
            vec![vec!["R".to_string()], vec!["L".to_string()]]
        );
        let attrs = |v: VarDecl| v.attributes().iter().map(|a| a.name()).collect::<Vec<_>>();
        assert_eq!(attrs(o.var_decls().next().unwrap()), vec!["NonDebuggable".to_string()]);
        let mut locals = r.local_var_decls();
        assert_eq!(attrs(locals.next().unwrap()), vec!["SecurityFiltering".to_string()]);
        assert!(attrs(locals.next().unwrap()).is_empty());
    }

    #[test]
    fn dotnet_typed_variables_and_parameters_parse() {
        // Microsoft's source declares .NET types as `DotNet Name` (System Application, 134 files).
        let f = file(
            "codeunit 1 C { var Conv: DotNet Convert; Plain: DotNet \"Dotted.Name\"; procedure P(X: DotNet String): DotNet Array begin end; }",
        );
        let o = f.objects().next().unwrap();
        let types: Vec<_> = o.var_decls().map(|v| v.type_ref().unwrap().text()).collect();
        assert_eq!(types, ["DotNet Convert", "DotNet \"Dotted.Name\""]);
        assert_eq!(o.var_decls().next().unwrap().type_ref().unwrap().type_name(), "DotNet");
    }

    #[test]
    fn routine_signature_attributes_and_locals() {
        let o = file(SRC).objects().next().unwrap();
        let r = o.routines().next().unwrap();
        assert_eq!(r.name(), "T");
        assert!(r.is_local());
        assert_eq!(
            r.attributes().map(|a| a.name()).collect::<Vec<_>>(),
            vec!["Test", "HandlerFunctions"]
        );
        let params: Vec<_> = r
            .params()
            .map(|p| (p.is_var(), p.name(), p.type_ref().unwrap().text()))
            .collect();
        assert_eq!(
            params,
            vec![
                (true, "R".to_string(), "Record Customer".to_string()),
                (false, "N".to_string(), "Integer".to_string())
            ]
        );
        assert_eq!(r.return_name().as_deref(), Some("Result"));
        assert_eq!(r.return_type().unwrap().text(), "Text[30]");
        assert_eq!(
            r.local_var_decls().map(|v| v.names()).collect::<Vec<_>>(),
            vec![vec!["L".to_string()]]
        );
        assert!(r.body().is_some());
    }

    #[test]
    fn triggers_and_sections() {
        let f = file(SRC);
        let objs: Vec<_> = f.objects().collect();
        assert_eq!(objs[0].triggers().map(|t| t.name()).collect::<Vec<_>>(), vec!["OnRun"]);
        let fields = objs[1].sections().next().unwrap();
        assert_eq!(fields.name(), "fields");
        let field = fields.sections().next().unwrap();
        assert_eq!(field.name(), "field");
        assert_eq!(field.args(), vec!["50100", "F", "Integer"]);
    }

    const WITH_SECTIONS: &str = r#"page 50100 P
{
    layout
    {
        area(Content)
        {
            field(A; Rec.A)
            {
                trigger OnValidate()
                begin
                    Message('x');
                end;
            }
        }
    }
    actions
    {
        area(Processing)
        {
            action(Go)
            {
                trigger OnAction()
                begin
                end;
            }
        }
    }

    trigger OnOpenPage()
    begin
    end;

    procedure Q()
    begin
    end;
}
"#;

    #[test]
    fn routines_in_matches_a_full_descendant_walk() {
        let f = file(WITH_SECTIONS);
        let o = f.objects().next().unwrap();
        let walked: Vec<_> = o.syntax().descendants().filter_map(Routine::cast).collect();
        assert_eq!(routines_in(o.syntax()), walked);
        assert_eq!(
            routines_in(o.syntax()).iter().map(|r| r.name()).collect::<Vec<_>>(),
            vec!["OnValidate", "OnAction", "OnOpenPage", "Q"]
        );
    }

    #[test]
    fn sections_in_matches_a_full_descendant_walk() {
        let f = file(WITH_SECTIONS);
        let o = f.objects().next().unwrap();
        let walked: Vec<_> = o.syntax().descendants().filter_map(Section::cast).collect();
        assert_eq!(sections_in(o.syntax()), walked);
        assert_eq!(
            sections_in(o.syntax()).iter().map(|s| s.name()).collect::<Vec<_>>(),
            vec!["layout", "area", "field", "actions", "area", "action"]
        );
    }
}
