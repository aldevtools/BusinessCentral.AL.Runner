//! The XPath 1.0 subset the corpus pins.

use super::*;

// ------------------------------------------------------------- XPath 1.0 (the part the corpus pins)

/// Why an XPath did not produce a node set.
pub(super) enum XFail {
    /// `Expression must evaluate to a node-set.` (xml/TestXmlRuntimeContracts.al:XmlDocument_SelectNodes_NotANodeSet_Throws).
    NotNodeSet,
    /// Not valid XPath, or a prefix nobody declared: a call whose result is used answers false
    /// (xml/TestProbeXmlContracts.al:XPath_MalformedStatement, XPath_NamespaceManager); the statement form is refused.
    Malformed(String),
    Signal(Signal),
}

impl From<Signal> for XFail {
    fn from(s: Signal) -> Self {
        XFail::Signal(s)
    }
}

pub(super) type XRes<T> = Result<T, XFail>;

pub(super) fn malformed<T>(why: impl Into<String>) -> XRes<T> {
    Err(XFail::Malformed(why.into()))
}

/// Prefix to namespace declarations of an `XmlNamespaceManager`.
pub(super) type Prefixes<'a> = Option<&'a [(String, String)]>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Axis {
    Child,
    Descendant,
    DescendantOrSelf,
    Current,
    Parent,
    Ancestor,
    AncestorOrSelf,
    FollowingSibling,
    PrecedingSibling,
    /// Every node after the context node in document order, without its descendants (xml/lt-ay-xml/
    /// AyXPathAxesVariables.al:101 XPath_Axes_3).
    Following,
    /// Every node before the context node in document order, without its ancestors; positions count from the
    /// nearest node (AyXPathAxesVariables.al:116 XPath_Axes_4, :176 XPath_Axes_8).
    Preceding,
    Attribute,
}

#[derive(Debug, Clone)]
pub(super) enum Test {
    /// `prefix:local` or `local`. An unprefixed name matches only nodes in no namespace
    /// (XPath 1.0; xml/TestXmlRuntimeContracts.al:XmlDocument_SelectNodes_DefaultNamespace_NeedsAPrefixedName).
    Name(Option<String>, String),
    Any,
    Text,
    Node,
    Comment,
    Pi,
}

#[derive(Debug, Clone)]
pub(super) struct Step {
    axis: Axis,
    test: Test,
    preds: Vec<Expr>,
}

#[derive(Debug, Clone)]
pub(super) enum Expr {
    Num(f64),
    Str(String),
    /// An absolute (`true`) or relative location path.
    Path(bool, Vec<Step>),
    /// `a | b`
    Union(Vec<Expr>),
    Call(String, Vec<Expr>),
    Cmp(Box<Expr>, String, Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Neg(Box<Expr>),
    Arith(Box<Expr>, char, Box<Expr>),
    /// A filter on a function result or a parenthesised expression followed by steps.
    Filter(Box<Expr>, Vec<Step>),
    /// `(path)[n]`, `f()[p]`: predicates applied to a node set in document order.
    Predicated(Box<Expr>, Vec<Expr>),
}

pub(super) struct XParser<'a> {
    s: Vec<char>,
    i: usize,
    prefixes: Prefixes<'a>,
}

impl XParser<'_> {
    fn ws(&mut self) {
        while self.s.get(self.i).is_some_and(|c| c.is_whitespace()) {
            self.i += 1;
        }
    }
    fn peek(&self) -> Option<char> {
        self.s.get(self.i).copied()
    }
    fn starts(&self, t: &str) -> bool {
        t.chars().enumerate().all(|(k, c)| self.s.get(self.i + k) == Some(&c))
    }
    fn eat(&mut self, t: &str) -> bool {
        self.ws();
        if self.starts(t) {
            self.i += t.chars().count();
            true
        } else {
            false
        }
    }
    fn name(&mut self) -> Option<String> {
        let start = self.i;
        if !self.peek().is_some_and(is_name_start) {
            return None;
        }
        while self.peek().is_some_and(is_name_char) {
            self.i += 1;
        }
        Some(self.s[start..self.i].iter().collect())
    }

    fn expr(&mut self) -> XRes<Expr> {
        self.or()
    }
    fn or(&mut self) -> XRes<Expr> {
        let mut l = self.and()?;
        while self.word("or") {
            let r = self.and()?;
            l = Expr::Or(Box::new(l), Box::new(r));
        }
        Ok(l)
    }
    fn and(&mut self) -> XRes<Expr> {
        let mut l = self.cmp()?;
        while self.word("and") {
            let r = self.cmp()?;
            l = Expr::And(Box::new(l), Box::new(r));
        }
        Ok(l)
    }
    fn word(&mut self, w: &str) -> bool {
        self.ws();
        let save = self.i;
        if self.starts(w) && !self.s.get(self.i + w.len()).copied().is_some_and(is_name_char) {
            self.i += w.len();
            true
        } else {
            self.i = save;
            false
        }
    }
    fn cmp(&mut self) -> XRes<Expr> {
        let mut l = self.additive()?;
        loop {
            self.ws();
            let op = ["!=", "<=", ">=", "=", "<", ">"].into_iter().find(|o| self.starts(o));
            let Some(op) = op else { return Ok(l) };
            self.i += op.len();
            let r = self.additive()?;
            l = Expr::Cmp(Box::new(l), op.to_string(), Box::new(r));
        }
    }
    fn additive(&mut self) -> XRes<Expr> {
        let mut l = self.multiplicative()?;
        loop {
            self.ws();
            let op = match self.peek() {
                Some('+') => '+',
                // `-` is an operator only when a name char does not follow directly (names may hold `-`).
                Some('-') => '-',
                _ => return Ok(l),
            };
            self.i += 1;
            let r = self.multiplicative()?;
            l = Expr::Arith(Box::new(l), op, Box::new(r));
        }
    }
    fn multiplicative(&mut self) -> XRes<Expr> {
        let mut l = self.unary()?;
        loop {
            self.ws();
            if self.peek() == Some('*') {
                self.i += 1;
                let r = self.unary()?;
                l = Expr::Arith(Box::new(l), '*', Box::new(r));
            } else if self.word("div") {
                let r = self.unary()?;
                l = Expr::Arith(Box::new(l), '/', Box::new(r));
            } else if self.word("mod") {
                let r = self.unary()?;
                l = Expr::Arith(Box::new(l), '%', Box::new(r));
            } else {
                return Ok(l);
            }
        }
    }
    fn unary(&mut self) -> XRes<Expr> {
        self.ws();
        if self.peek() == Some('-') {
            self.i += 1;
            return Ok(Expr::Neg(Box::new(self.unary()?)));
        }
        self.union()
    }
    fn union(&mut self) -> XRes<Expr> {
        let mut parts = vec![self.path_expr()?];
        while self.eat("|") {
            parts.push(self.path_expr()?);
        }
        Ok(if parts.len() == 1 {
            parts.remove(0)
        } else {
            Expr::Union(parts)
        })
    }

    fn path_expr(&mut self) -> XRes<Expr> {
        self.ws();
        match self.peek() {
            Some('/') => self.location(true),
            Some('\'') | Some('"') => {
                let q = self.peek().unwrap();
                self.i += 1;
                let start = self.i;
                while self.peek().is_some_and(|c| c != q) {
                    self.i += 1;
                }
                if self.peek().is_none() {
                    return malformed("unterminated literal");
                }
                let t: String = self.s[start..self.i].iter().collect();
                self.i += 1;
                Ok(Expr::Str(t))
            }
            Some(c) if c.is_ascii_digit() || c == '.' && self.s.get(self.i + 1).is_some_and(|d| d.is_ascii_digit()) => {
                let start = self.i;
                while self.peek().is_some_and(|c| c.is_ascii_digit() || c == '.') {
                    self.i += 1;
                }
                let t: String = self.s[start..self.i].iter().collect();
                t.parse().map(Expr::Num).or_else(|_| malformed("number"))
            }
            Some('(') => {
                self.i += 1;
                let e = self.expr()?;
                if !self.eat(")") {
                    return malformed("unclosed (");
                }
                self.filter_tail(e)
            }
            Some('$') => malformed("variables are not supported"),
            Some(c) if is_name_start(c) => {
                // A function call is a name followed by `(`, unless it is a node test or an axis.
                let save = self.i;
                let name = self.name().unwrap();
                self.ws();
                let is_node_test = matches!(name.as_str(), "text" | "node" | "comment" | "processing-instruction");
                if self.peek() == Some('(') && !is_node_test {
                    self.i += 1;
                    let mut args = Vec::new();
                    if !self.eat(")") {
                        loop {
                            args.push(self.expr()?);
                            if self.eat(",") {
                                continue;
                            }
                            if self.eat(")") {
                                break;
                            }
                            return malformed("unclosed function call");
                        }
                    }
                    return self.filter_tail(Expr::Call(name, args));
                }
                self.i = save;
                self.location(false)
            }
            Some(_) => self.location(false),
            None => malformed("empty expression"),
        }
    }

    /// Why a predicate has no `]`: the path ended (the pinned "not a node set", P90CDataRead.al `/r/a[`), or another token
    /// stands there (not pinned).
    fn unclosed_bracket(&mut self) -> &'static str {
        self.ws();
        if self.peek().is_none() {
            "unclosed ["
        } else {
            "a token where ] belongs"
        }
    }

    /// Steps after a function call or a parenthesised expression: `(a)/b`, `f()[1]`.
    fn filter_tail(&mut self, e: Expr) -> XRes<Expr> {
        self.ws();
        let mut e = e;
        let mut preds = Vec::new();
        while self.peek() == Some('[') {
            self.i += 1;
            preds.push(self.expr()?);
            if !self.eat("]") {
                return malformed(self.unclosed_bracket());
            }
            self.ws();
        }
        if !preds.is_empty() {
            e = Expr::Predicated(Box::new(e), preds);
        }
        if self.peek() == Some('/') {
            let mut steps = Vec::new();
            while self.peek() == Some('/') {
                self.i += 1;
                if self.peek() == Some('/') {
                    self.i += 1;
                    steps.push(Step {
                        axis: Axis::DescendantOrSelf,
                        test: Test::Node,
                        preds: vec![],
                    });
                }
                steps.push(self.step()?);
                self.ws();
            }
            return Ok(Expr::Filter(Box::new(e), steps));
        }
        Ok(e)
    }

    fn location(&mut self, absolute_allowed: bool) -> XRes<Expr> {
        let mut steps = Vec::new();
        let mut absolute = false;
        self.ws();
        if absolute_allowed && self.peek() == Some('/') {
            absolute = true;
            self.i += 1;
            if self.peek() == Some('/') {
                self.i += 1;
                steps.push(Step {
                    axis: Axis::DescendantOrSelf,
                    test: Test::Node,
                    preds: vec![],
                });
                steps.push(self.step()?);
            } else {
                self.ws();
                // `/` alone is the root.
                if self.peek().is_none() || matches!(self.peek(), Some(')' | ']' | '|' | ',' | '=' | '!' | '<' | '>')) {
                    return Ok(Expr::Path(true, steps));
                }
                steps.push(self.step()?);
            }
        } else {
            steps.push(self.step()?);
        }
        loop {
            self.ws();
            if self.peek() == Some('/') {
                self.i += 1;
                if self.peek() == Some('/') {
                    self.i += 1;
                    steps.push(Step {
                        axis: Axis::DescendantOrSelf,
                        test: Test::Node,
                        preds: vec![],
                    });
                }
                steps.push(self.step()?);
            } else {
                break;
            }
        }
        Ok(Expr::Path(absolute, steps))
    }

    fn step(&mut self) -> XRes<Step> {
        self.ws();
        let (axis, test);
        if self.starts("..") {
            self.i += 2;
            axis = Axis::Parent;
            test = Test::Node;
        } else if self.peek() == Some('.') {
            self.i += 1;
            axis = Axis::Current;
            test = Test::Node;
        } else {
            let mut ax = Axis::Child;
            if self.peek() == Some('@') {
                self.i += 1;
                ax = Axis::Attribute;
            } else {
                // `axis::`
                let save = self.i;
                if let Some(n) = self.name() {
                    if self.starts("::") {
                        self.i += 2;
                        ax = match n.as_str() {
                            "child" => Axis::Child,
                            "descendant" => Axis::Descendant,
                            "descendant-or-self" => Axis::DescendantOrSelf,
                            "self" => Axis::Current,
                            "parent" => Axis::Parent,
                            "ancestor" => Axis::Ancestor,
                            "ancestor-or-self" => Axis::AncestorOrSelf,
                            "following-sibling" => Axis::FollowingSibling,
                            "preceding-sibling" => Axis::PrecedingSibling,
                            "attribute" => Axis::Attribute,
                            "following" => Axis::Following,
                            "preceding" => Axis::Preceding,
                            // AyXPathAxesVariables.al:161 XPath_Axes_7 pins only the names of the namespace nodes
                            // (`xmlns:p`, `xmlns:xml`), not what they are (value, owner element, identity with the
                            // declaring attribute, order of several declarations).
                            "namespace" => {
                                return Err(XFail::Signal(Signal::Unsupported(
                                    "the XPath namespace:: axis (only the node names are pinned)".into(),
                                )));
                            }
                            other => return malformed(format!("axis {other}")),
                        };
                    } else {
                        self.i = save;
                    }
                }
            }
            axis = ax;
            self.ws();
            if self.peek() == Some('*') {
                self.i += 1;
                test = Test::Any;
            } else {
                // A name may hold one `:`.
                let Some(first) = self.name() else {
                    // A character no XPath token starts with is an invalid token (xml/XmlXPathNamespaces.al:640
                    // SelectNodes_Backslash_Path `\\contract`); anything else is a missing step.
                    if self.peek().is_some_and(|c| !is_xpath_token_char(c)) {
                        return malformed("an invalid token");
                    }
                    return malformed("a step without a name");
                };
                let mut qn = first;
                if self.peek() == Some(':') && self.s.get(self.i + 1) != Some(&':') {
                    self.i += 1;
                    if self.peek() == Some('*') {
                        self.i += 1;
                        qn.push_str(":*");
                    } else {
                        let Some(l) = self.name() else {
                            return malformed("a prefix without a name");
                        };
                        qn = format!("{qn}:{l}");
                    }
                }
                self.ws();
                if self.peek() == Some('(') {
                    self.i += 1;
                    if !self.eat(")") {
                        return malformed("node test with arguments");
                    }
                    test = match qn.as_str() {
                        "text" => Test::Text,
                        "node" => Test::Node,
                        "comment" => Test::Comment,
                        "processing-instruction" => Test::Pi,
                        other => return malformed(format!("function {other}() as a step")),
                    };
                } else {
                    let (pre, local) = match qn.split_once(':') {
                        Some((p, l)) => (Some(p.to_string()), l.to_string()),
                        None => (None, qn),
                    };
                    if pre.is_some() && self.prefixes.is_none() {
                        return malformed("a prefixed name needs a namespace manager");
                    }
                    // A prefix the manager does not know is an error whether or not a node is tested against it
                    // (xml/lt-xml-remaining-node-methods/P104SelCData.al:P104_Nodes_WithManager `zz:r` answers No).
                    if let (Some(p), Some(m)) = (&pre, self.prefixes) {
                        if !m.iter().any(|(q, _)| q == p) {
                            return malformed(format!("prefix {p} is not declared"));
                        }
                    }
                    test = Test::Name(pre, local);
                }
            }
        }
        let mut preds = Vec::new();
        loop {
            self.ws();
            if self.peek() != Some('[') {
                break;
            }
            self.i += 1;
            preds.push(self.expr()?);
            if !self.eat("]") {
                return malformed(self.unclosed_bracket());
            }
        }
        Ok(Step { axis, test, preds })
    }
}

/// Whether an XPath token can start with `c` (what .NET's XPath scanner reads): white space, punctuation of the grammar,
/// quotes, digits and name characters.
pub(super) fn is_xpath_token_char(c: char) -> bool {
    c.is_whitespace() || "()[]@,/|.*+-=!<>:$\"'".contains(c) || c.is_ascii_digit() || is_name_start(c)
}

pub(super) fn parse_xpath(p: &str, prefixes: Prefixes) -> XRes<Expr> {
    // `/r/@@@`: more than one `@` in a row is the pinned "not a node set" (a path that cannot be evaluated).
    if p.contains("@@") {
        return Err(XFail::NotNodeSet);
    }
    let mut parser = XParser {
        s: p.chars().collect(),
        i: 0,
        prefixes,
    };
    let e = parser.expr()?;
    parser.ws();
    if parser.i != parser.s.len() {
        return malformed("trailing characters");
    }
    Ok(e)
}

#[derive(Clone)]
pub(super) enum XVal {
    Nodes(Vec<Handle>),
    Num(f64),
    Str(String),
    Bool(bool),
}

pub(super) struct XCtx<'a> {
    prefixes: Prefixes<'a>,
    /// Document order of every node of the tree the query started in.
    order: std::collections::HashMap<*const RefCell<LibraryValue>, usize>,
}

pub(super) fn doc_order(root: &Handle, out: &mut std::collections::HashMap<*const RefCell<LibraryValue>, usize>) {
    let n = out.len();
    out.insert(Rc::as_ptr(root), n);
    for a in attrs_of(root) {
        let n = out.len();
        out.insert(Rc::as_ptr(&a), n);
    }
    for c in children_of(root) {
        doc_order(&c, out);
    }
}

pub(super) fn string_value(h: &Handle) -> String {
    with(h, |d| match d {
        XData::Attribute { value, .. } => Some(value.clone()),
        XData::Text(t) | XData::CData(t) | XData::Comment(t) => Some(t.clone()),
        XData::Pi { data, .. } => Some(data.clone()),
        _ => None,
    })
    .unwrap_or_else(|| {
        let mut s = String::new();
        inner_text(h, &mut s);
        s
    })
}

pub(super) fn to_str(v: &XVal) -> String {
    match v {
        XVal::Str(s) => s.clone(),
        XVal::Num(n) => {
            if n.fract() == 0.0 && n.is_finite() {
                format!("{}", *n as i64)
            } else {
                n.to_string()
            }
        }
        XVal::Bool(b) => b.to_string(),
        XVal::Nodes(n) => n.first().map(string_value).unwrap_or_default(),
    }
}

pub(super) fn to_num(v: &XVal) -> f64 {
    match v {
        XVal::Num(n) => *n,
        XVal::Bool(b) => f64::from(u8::from(*b)),
        other => to_str(other).trim().parse().unwrap_or(f64::NAN),
    }
}

pub(super) fn to_bool(v: &XVal) -> bool {
    match v {
        XVal::Bool(b) => *b,
        XVal::Num(n) => *n != 0.0 && !n.is_nan(),
        XVal::Str(s) => !s.is_empty(),
        XVal::Nodes(n) => !n.is_empty(),
    }
}

pub(super) fn node_matches(h: &Handle, test: &Test, principal_attr: bool, ctx: &XCtx) -> XRes<bool> {
    let wanted = |pre: &Option<String>| -> XRes<String> {
        match pre {
            None => Ok(String::new()),
            Some(pre) => ctx
                .prefixes
                .and_then(|m| m.iter().rev().find(|(p, _)| p == pre))
                .map(|(_, u)| u.clone())
                .map_or_else(|| malformed(format!("prefix {pre} is not declared")), Ok),
        }
    };
    Ok(match test {
        Test::Name(pre, n) => {
            let want = wanted(pre)?;
            // `prefix:*` is every element of that namespace (xml/XmlXPathNamespaces.al:344 SelectNodes_WithNamespaceManager
            // `x_star`); on the attribute axis it is not pinned.
            let any_local = pre.is_some() && n == "*";
            if any_local && principal_attr {
                return Err(XFail::Signal(Signal::Unsupported(
                    "the XPath test prefix:* on attributes (not pinned)".into(),
                )));
            }
            with(h, |d| match d {
                XData::Element { local, ns, .. } if !principal_attr => *ns == want && (any_local || local == n),
                XData::Attribute { local, ns, .. } if principal_attr => *ns == want && local == n,
                _ => false,
            })
        }
        Test::Any => with(h, |d| match d {
            XData::Element { .. } => !principal_attr,
            XData::Attribute { ns, .. } => principal_attr && ns != XMLNS_URI,
            _ => false,
        }),
        Test::Node => true,
        Test::Text => with(h, |d| matches!(d, XData::Text(_) | XData::CData(_))),
        Test::Comment => with(h, |d| matches!(d, XData::Comment(_))),
        Test::Pi => with(h, |d| matches!(d, XData::Pi { .. })),
    })
}

pub(super) fn descendants(h: &Handle, out: &mut Vec<Handle>) {
    for c in children_of(h) {
        out.push(c.clone());
        descendants(&c, out);
    }
}

pub(super) fn sort_dedup(mut v: Vec<Handle>, ctx: &XCtx) -> Vec<Handle> {
    v.sort_by_key(|h| ctx.order.get(&Rc::as_ptr(h)).copied().unwrap_or(usize::MAX));
    v.dedup_by(|a, b| Rc::ptr_eq(a, b));
    v
}

/// The nodes of `step` from `node`, in axis order (reverse axes nearest first).
pub(super) fn axis_nodes(node: &Handle, step: &Step, ctx: &XCtx) -> XRes<Vec<Handle>> {
    let mut cand: Vec<Handle> = Vec::new();
    match step.axis {
        Axis::Child => cand = children_of(node),
        Axis::Descendant => descendants(node, &mut cand),
        Axis::DescendantOrSelf => {
            cand.push(node.clone());
            descendants(node, &mut cand);
        }
        Axis::Current => cand.push(node.clone()),
        Axis::Parent => cand.extend(parent_of(node)),
        Axis::Ancestor | Axis::AncestorOrSelf => {
            if step.axis == Axis::AncestorOrSelf {
                cand.push(node.clone());
            }
            let mut cur = parent_of(node);
            while let Some(p) = cur {
                cand.push(p.clone());
                cur = parent_of(&p);
            }
        }
        Axis::FollowingSibling | Axis::PrecedingSibling => {
            if let Some(p) = parent_of(node) {
                let sibs = children_of(&p);
                if let Some(at) = sibs.iter().position(|c| Rc::ptr_eq(c, node)) {
                    if step.axis == Axis::FollowingSibling {
                        cand = sibs[at + 1..].to_vec();
                    } else {
                        cand = sibs[..at].iter().rev().cloned().collect();
                    }
                }
            }
        }
        Axis::Following | Axis::Preceding => {
            // Pinned only from an element (AyXPathAxesVariables.al:93-180); from an attribute the axes would also
            // reach the owner's children, which no corpus test shows.
            if with(node, |d| matches!(d, XData::Attribute { .. })) {
                return Err(XFail::Signal(Signal::Unsupported(
                    "the XPath following:: or preceding:: axis from an attribute".into(),
                )));
            }
            let mut cur = node.clone();
            while let Some(p) = parent_of(&cur) {
                let sibs = children_of(&p);
                if let Some(at) = sibs.iter().position(|c| Rc::ptr_eq(c, &cur)) {
                    if step.axis == Axis::Following {
                        for s in &sibs[at + 1..] {
                            cand.push(s.clone());
                            descendants(s, &mut cand);
                        }
                    } else {
                        for s in sibs[..at].iter().rev() {
                            let mut sub = Vec::new();
                            descendants(s, &mut sub);
                            cand.extend(sub.into_iter().rev());
                            cand.push(s.clone());
                        }
                    }
                }
                cur = p;
            }
        }
        Axis::Attribute => cand = attrs_of(node),
    }
    let principal_attr = step.axis == Axis::Attribute;
    let mut found = Vec::new();
    for c in cand {
        if node_matches(&c, &step.test, principal_attr, ctx)? {
            found.push(c);
        }
    }
    for p in &step.preds {
        let last = found.len();
        let mut keep = Vec::new();
        for (i, n) in found.iter().enumerate() {
            let v = eval(p, n, i + 1, last, ctx)?;
            let pass = match v {
                XVal::Num(x) => x == (i + 1) as f64,
                other => to_bool(&other),
            };
            if pass {
                keep.push(n.clone());
            }
        }
        found = keep;
    }
    Ok(found)
}

pub(super) fn run_steps(start: Vec<Handle>, steps: &[Step], ctx: &XCtx) -> XRes<Vec<Handle>> {
    let mut current = start;
    for step in steps {
        let mut next: Vec<Handle> = Vec::new();
        for c in &current {
            next.extend(axis_nodes(c, step, ctx)?);
        }
        current = sort_dedup(next, ctx);
    }
    Ok(current)
}

pub(super) fn eval(e: &Expr, node: &Handle, pos: usize, last: usize, ctx: &XCtx) -> XRes<XVal> {
    Ok(match e {
        Expr::Num(n) => XVal::Num(*n),
        Expr::Str(s) => XVal::Str(s.clone()),
        Expr::Path(absolute, steps) => {
            let start = if *absolute { root_of(node) } else { node.clone() };
            XVal::Nodes(run_steps(vec![start], steps, ctx)?)
        }
        Expr::Union(parts) => {
            let mut all = Vec::new();
            for p in parts {
                match eval(p, node, pos, last, ctx)? {
                    XVal::Nodes(n) => all.extend(n),
                    _ => return Err(XFail::NotNodeSet),
                }
            }
            XVal::Nodes(sort_dedup(all, ctx))
        }
        Expr::Predicated(inner, preds) => match eval(inner, node, pos, last, ctx)? {
            XVal::Nodes(mut found) => {
                for p in preds {
                    let total = found.len();
                    let mut keep = Vec::new();
                    for (i, n) in found.iter().enumerate() {
                        let pass = match eval(p, n, i + 1, total, ctx)? {
                            XVal::Num(x) => x == (i + 1) as f64,
                            other => to_bool(&other),
                        };
                        if pass {
                            keep.push(n.clone());
                        }
                    }
                    found = keep;
                }
                XVal::Nodes(found)
            }
            _ => return Err(XFail::NotNodeSet),
        },
        Expr::Filter(inner, steps) => match eval(inner, node, pos, last, ctx)? {
            XVal::Nodes(n) => XVal::Nodes(run_steps(n, steps, ctx)?),
            _ => return Err(XFail::NotNodeSet),
        },
        Expr::Neg(x) => XVal::Num(-to_num(&eval(x, node, pos, last, ctx)?)),
        Expr::Arith(a, op, b) => {
            let (x, y) = (
                to_num(&eval(a, node, pos, last, ctx)?),
                to_num(&eval(b, node, pos, last, ctx)?),
            );
            XVal::Num(match op {
                '+' => x + y,
                '-' => x - y,
                '*' => x * y,
                '/' => x / y,
                _ => x % y,
            })
        }
        Expr::And(a, b) => {
            XVal::Bool(to_bool(&eval(a, node, pos, last, ctx)?) && to_bool(&eval(b, node, pos, last, ctx)?))
        }
        Expr::Or(a, b) => {
            XVal::Bool(to_bool(&eval(a, node, pos, last, ctx)?) || to_bool(&eval(b, node, pos, last, ctx)?))
        }
        Expr::Cmp(a, op, b) => {
            let (x, y) = (eval(a, node, pos, last, ctx)?, eval(b, node, pos, last, ctx)?);
            XVal::Bool(compare(&x, op, &y))
        }
        Expr::Call(name, args) => {
            let mut vals = Vec::new();
            for a in args {
                vals.push(eval(a, node, pos, last, ctx)?);
            }
            let first_node = |vals: &[XVal]| match vals.first() {
                Some(XVal::Nodes(n)) => n.first().cloned(),
                None => Some(node.clone()),
                _ => None,
            };
            match (name.as_str(), vals.len()) {
                ("last", 0) => XVal::Num(last as f64),
                ("position", 0) => XVal::Num(pos as f64),
                ("count", 1) => match &vals[0] {
                    XVal::Nodes(n) => XVal::Num(n.len() as f64),
                    _ => return Err(XFail::NotNodeSet),
                },
                ("local-name", _) => XVal::Str(
                    first_node(&vals)
                        .map(|n| {
                            with(&n, |d| match d {
                                XData::Element { local, .. } | XData::Attribute { local, .. } => local.clone(),
                                XData::Pi { target, .. } => target.clone(),
                                _ => String::new(),
                            })
                        })
                        .unwrap_or_default(),
                ),
                ("namespace-uri", _) => XVal::Str(
                    first_node(&vals)
                        .map(|n| {
                            with(&n, |d| match d {
                                XData::Element { ns, .. } | XData::Attribute { ns, .. } => ns.clone(),
                                _ => String::new(),
                            })
                        })
                        .unwrap_or_default(),
                ),
                ("name", _) => XVal::Str(
                    first_node(&vals)
                        .map(|n| {
                            with(&n, |d| match d {
                                XData::Element { prefix, local, .. } | XData::Attribute { prefix, local, .. } => {
                                    qname(prefix, local)
                                }
                                _ => String::new(),
                            })
                        })
                        .unwrap_or_default(),
                ),
                ("concat", n) if n >= 2 => XVal::Str(vals.iter().map(to_str).collect::<String>()),
                ("string", 0) => XVal::Str(string_value(node)),
                ("string", 1) => XVal::Str(to_str(&vals[0])),
                ("number", 1) => XVal::Num(to_num(&vals[0])),
                ("not", 1) => XVal::Bool(!to_bool(&vals[0])),
                ("true", 0) => XVal::Bool(true),
                ("false", 0) => XVal::Bool(false),
                ("boolean", 1) => XVal::Bool(to_bool(&vals[0])),
                ("contains", 2) => XVal::Bool(to_str(&vals[0]).contains(&to_str(&vals[1]))),
                ("starts-with", 2) => XVal::Bool(to_str(&vals[0]).starts_with(&to_str(&vals[1]))),
                ("string-length", 1) => XVal::Num(to_str(&vals[0]).chars().count() as f64),
                ("normalize-space", 1) => XVal::Str(to_str(&vals[0]).split_whitespace().collect::<Vec<_>>().join(" ")),
                (other, n) => return malformed(format!("function {other} with {n} arguments")),
            }
        }
    })
}

pub(super) fn compare(a: &XVal, op: &str, b: &XVal) -> bool {
    let cmp_num = |x: f64, y: f64| match op {
        "=" => x == y,
        "!=" => x != y,
        "<" => x < y,
        "<=" => x <= y,
        ">" => x > y,
        _ => x >= y,
    };
    let cmp_str = |x: &str, y: &str| match op {
        "=" => x == y,
        "!=" => x != y,
        _ => cmp_num(
            x.trim().parse().unwrap_or(f64::NAN),
            y.trim().parse().unwrap_or(f64::NAN),
        ),
    };
    match (a, b) {
        (XVal::Nodes(n), XVal::Nodes(m)) => n
            .iter()
            .any(|x| m.iter().any(|y| cmp_str(&string_value(x), &string_value(y)))),
        (XVal::Nodes(n), XVal::Num(y)) | (XVal::Num(y), XVal::Nodes(n)) => {
            let flip = matches!(a, XVal::Num(_));
            n.iter().any(|x| {
                let v: f64 = string_value(x).trim().parse().unwrap_or(f64::NAN);
                if flip { cmp_num(*y, v) } else { cmp_num(v, *y) }
            })
        }
        (XVal::Nodes(n), XVal::Str(s)) => n.iter().any(|x| cmp_str(&string_value(x), s)),
        (XVal::Str(s), XVal::Nodes(n)) => n.iter().any(|x| cmp_str(s, &string_value(x))),
        (XVal::Bool(_), _) | (_, XVal::Bool(_)) => {
            let (x, y) = (to_bool(a), to_bool(b));
            match op {
                "=" => x == y,
                "!=" => x != y,
                _ => cmp_num(f64::from(u8::from(x)), f64::from(u8::from(y))),
            }
        }
        (XVal::Num(_), _) | (_, XVal::Num(_)) => cmp_num(to_num(a), to_num(b)),
        _ => cmp_str(&to_str(a), &to_str(b)),
    }
}

/// The nodes `path` selects from `ctx`. A result that is not a node set (a number, a string) selects nothing.
pub(super) fn xpath(ctx: &Handle, path: &str, prefixes: Prefixes) -> XRes<Vec<Handle>> {
    let expr = parse_xpath(path, prefixes)?;
    let root = root_of(ctx);
    let mut order = std::collections::HashMap::new();
    doc_order(&root, &mut order);
    let xc = XCtx { prefixes, order };
    match eval(&expr, ctx, 1, 1, &xc)? {
        XVal::Nodes(n) => Ok(n),
        _ => Ok(Vec::new()),
    }
}
