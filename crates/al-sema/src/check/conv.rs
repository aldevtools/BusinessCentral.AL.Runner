//! Implicit conversions at compile time: which value a context takes where a type is expected, and the AL compiler's
//! code when it does not. The rules are the compile matrix of corpus writer Y31 (corpus repo
//! `scripts/conversion-matrix/compile-matrix.csv`, branch `al-runtime-rs-staging-y31`, measured with the AL compiler
//! for runtime 16.1, 17.1 and 18.0); `tests/conversion_matrix.rs` compares every cell of it with this checker.
//!
//! Only what the matrix measured is judged, so nothing outside it is rejected on a guess:
//! - operands of its shapes: a variable, parameter or field, a literal, an enum value, an option member, and the
//!   `Value` of a test page control. A call, an index, a parenthesized or any other expression is not judged.
//! - its types: the scalars, Text and Code (with and without a length), Label and TextConst, Date, Time, DateTime,
//!   Duration, DateFormula, Guid, RecordId, Variant, TextBuilder, Media, MediaSet, enums and interfaces. A record,
//!   a list, an object variable other than an interface, a DotNet value is not judged.
//! - its contexts: `:=`, `+=`, a by-value and a `var` argument of a procedure without overloads, `exit`, the operators
//!   `=`, `<>`, `<`, `+` and `-`, `x in [y]`, a case label and the false branch of `c ? a : b`.
//! - in the positions the matrix fills with any source (the right of `:=` and `+=`, an argument, `exit`, the left
//!   operand, the `x` of `x in [...]`, the selector of a case, the true branch), every shape above; in the positions it
//!   fills with a variable or field only (the right operand, a set element, a case label, the false branch), a
//!   variable, parameter, field or control Value: a literal there is not judged.
//!
//! Each judged site reports at most the codes the matrix holds for it; the older pinned checks (`infer.rs`) still judge
//! the shapes this module does not.
use super::*;

/// A type as the conversion rules see it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum K {
    Int,
    BigInt,
    Dec,
    Bool,
    Char,
    Byte,
    Opt,
    /// An enum, by object index.
    Enum(usize),
    Text(Option<u32>),
    Code(Option<u32>),
    /// A Label or TextConst variable.
    Label,
    Date,
    Time,
    DateTime,
    Dur,
    DateFormula,
    Guid,
    RecordId,
    Variant,
    TextBuilder,
    /// An interface, by object index.
    Interface(usize),
    Media,
    MediaSet,
    /// The `Value` of a test page control: Text where it is set (`TP.F.Value := x` is a call taking a Text: AL0133),
    /// a Text shorter than a Guid's where it gives the type of a set or conditional expression (AL0777).
    TestValue,
}

/// How an operand is written.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Shape {
    /// A variable, parameter or field: assignable, so a `var` argument.
    Place,
    /// A text literal of this many characters. One character is a Char where a value converts.
    Str(usize),
    Int,
    /// A BigInteger literal; whether its value fits an Integer.
    BigInt {
        fits: bool,
    },
    Dec,
    /// A Boolean, Date, Time or DateTime literal.
    Other,
    EnumValue,
    OptionMember,
    TestValue,
}

#[derive(Clone, Debug)]
pub(crate) struct Operand {
    pub(crate) k: K,
    pub(crate) shape: Shape,
    /// The operand's node, for the literal as written in the constant diagnostics (none for a computed operand).
    node: Option<SyntaxNode>,
    /// The type as alc names it in a message, worked out only for a message.
    name: Name,
}

/// How a message names an operand's type: by its type, or by a fixed name.
#[derive(Clone, Debug)]
enum Name {
    Of(Ty),
    Fixed(&'static str),
}

/// Why a conversion is rejected.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Reject {
    /// No implicit conversion: AL0122, AL0133 for an argument.
    No,
    /// A Guid into a Text or Code shorter than its text: AL0777.
    Overflow,
    /// A BigInteger constant that does not fit an Integer-ranged target: AL0369.
    Range,
    /// A Decimal constant as an Option: AL0398.
    Ordinal,
}

/// Where a value is converted. `Value`: assignment and `exit`; `Arg`: a by-value argument; `Set`: an element of
/// `x in [...]` or the false branch of a conditional expression; `Case`: a case label.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Ctx {
    Value,
    Arg,
    Set,
    Case,
}

fn numeric(k: K) -> bool {
    matches!(k, K::Int | K::BigInt | K::Dec | K::Char | K::Byte | K::Opt | K::Dur)
}

/// Text-like targets and sources: Text, Code, Label, TextConst and a test page control's Value.
fn texty(k: K) -> bool {
    matches!(k, K::Text(_) | K::Code(_) | K::Label | K::TestValue)
}

/// The numbers an Enum mixes with in an operator (alc warns AL0603), and that convert to an Enum.
fn int_like(k: K) -> bool {
    matches!(k, K::Int | K::Char | K::Byte | K::Opt)
}

fn date_like(k: K) -> bool {
    matches!(k, K::Date | K::Time | K::DateTime)
}

/// A Text or Code of this length is shorter than a Guid's text. Only the lengths the matrix measured are judged
/// (Text[3], Text[10], Code[3], Code[10], an empty and a three-character literal reject; Code[250] and Text
/// without a length take a Guid); the lengths between are not judged.
fn shorter_than_guid(k: K) -> bool {
    matches!(k, K::Text(Some(n)) | K::Code(Some(n)) if n <= 10) || k == K::TestValue
}

impl Operand {
    /// The kind this operand converts as: a one-character literal is a Char.
    fn value_kind(&self) -> K {
        match self.shape {
            Shape::Str(1) => K::Char,
            _ => self.k,
        }
    }

    /// The type this operand gives a set or conditional expression: a text literal is a Text of its length.
    fn type_kind(&self) -> K {
        match self.shape {
            Shape::Str(1) => K::Char,
            Shape::Str(n) => K::Text(Some(u32::try_from(n).unwrap_or(u32::MAX))),
            _ => self.k,
        }
    }

    /// The type as alc names it in a message.
    fn name(&self) -> String {
        match &self.name {
            Name::Fixed(n) => (*n).to_string(),
            Name::Of(Ty::Enum(n)) => format!("Enum \"{n}\""),
            Name::Of(Ty::Object(ObjKind::Interface, n)) => format!("Interface \"{n}\""),
            Name::Of(Ty::Label) => "Label".into(),
            Name::Of(_) if matches!(self.shape, Shape::Str(1)) => "Char".into(),
            Name::Of(t) => t.to_string(),
        }
    }

    /// The literal as written.
    fn text(&self) -> String {
        self.node.as_ref().map(|n| n.text().to_string()).unwrap_or_default()
    }

    fn is_place(&self) -> bool {
        self.shape == Shape::Place
    }

    /// A variable, parameter, field or control Value: the operands the matrix measured in every position.
    fn is_variable(&self) -> bool {
        matches!(self.shape, Shape::Place | Shape::TestValue)
    }
}

impl<'a> Checker<'a> {
    /// The operand of `node` (of type `ty`), if it has a shape and a type the matrix measured.
    pub(crate) fn conv_operand(&self, sc: &Scope, node: &SyntaxNode, ty: &Ty) -> Option<Operand> {
        if self.ws.is_dependency(sc.oi) {
            return None;
        }
        let key = |n: &SyntaxNode| {
            let r = n.text_range();
            (sc.file, usize::from(r.start()), usize::from(r.end()))
        };
        let shape = match node.kind() {
            SyntaxKind::Literal => {
                let tok = al_syntax::ast::significant_green_tokens(node).find(|t| !t.kind().is_trivia())?;
                match tok.kind() {
                    SyntaxKind::StringLit => {
                        let text = tok.text();
                        let inner = text.get(1..text.len().saturating_sub(1))?;
                        // Each doubled quote is one character of the value (`replace` and `matches` both take the
                        // pairs from the left).
                        Shape::Str(inner.chars().count() - inner.matches("''").count())
                    }
                    SyntaxKind::Integer => Shape::Int,
                    SyntaxKind::Decimal => Shape::Dec,
                    SyntaxKind::BigIntLit => {
                        let digits: String = tok.text().chars().filter(char::is_ascii_digit).collect();
                        Shape::BigInt {
                            fits: digits.parse::<i64>().is_ok_and(|v| i32::try_from(v).is_ok()),
                        }
                    }
                    _ if matches!(ty, Ty::Boolean | Ty::Date | Ty::Time | Ty::DateTime) => Shape::Other,
                    _ => return None,
                }
            }
            SyntaxKind::ScopeExpr => match self.resolutions.get(&key(node)).map(Res::inner) {
                Some(Res::EnumValue { .. }) => Shape::EnumValue,
                Some(Res::OptionValue(_)) => Shape::OptionMember,
                _ => return None,
            },
            SyntaxKind::NameRef => match self.resolutions.get(&key(node)).map(Res::inner) {
                Some(Res::Local(_) | Res::Global(_) | Res::Field { .. }) => Shape::Place,
                _ => return None,
            },
            SyntaxKind::MemberExpr => {
                let kids: Vec<SyntaxNode> = node.children().collect();
                let [base, member] = kids.as_slice() else { return None };
                let base_ty = self.types.get(&key(base));
                if matches!(base_ty, Some(Ty::Builtin(b)) if b.eq_ignore_ascii_case("TestField"))
                    && name_of(member).eq_ignore_ascii_case("value")
                {
                    Shape::TestValue
                } else if matches!(
                    self.resolutions
                        .get(&key(node))
                        .or_else(|| self.resolutions.get(&key(member)))
                        .map(Res::inner),
                    Some(Res::Field { .. })
                ) && matches!(base_ty, Some(Ty::Record { .. }))
                {
                    Shape::Place
                } else {
                    return None;
                }
            }
            _ => return None,
        };
        if shape == Shape::TestValue {
            return Some(Operand {
                k: K::TestValue,
                shape,
                node: None,
                name: Name::Fixed("Text"),
            });
        }
        let k = match ty {
            Ty::Integer => K::Int,
            Ty::BigInteger => K::BigInt,
            Ty::Decimal => K::Dec,
            Ty::Boolean => K::Bool,
            Ty::Char => K::Char,
            Ty::Byte => K::Byte,
            Ty::Option(_) => K::Opt,
            Ty::Enum(n) => K::Enum(self.resolve(sc, ObjKind::Enum, n)?),
            Ty::Text(n) => K::Text(*n),
            Ty::Code(n) => K::Code(*n),
            Ty::Label => K::Label,
            Ty::Date => K::Date,
            Ty::Time => K::Time,
            Ty::DateTime => K::DateTime,
            Ty::Duration => K::Dur,
            Ty::DateFormula => K::DateFormula,
            Ty::Guid => K::Guid,
            Ty::Variant => K::Variant,
            Ty::Object(ObjKind::Interface, n) => K::Interface(self.resolve(sc, ObjKind::Interface, n)?),
            Ty::Builtin(b) if b.eq_ignore_ascii_case("RecordId") => K::RecordId,
            Ty::Builtin(b) if b.eq_ignore_ascii_case("TextBuilder") => K::TextBuilder,
            Ty::Builtin(b) if b.eq_ignore_ascii_case("Media") => K::Media,
            Ty::Builtin(b) if b.eq_ignore_ascii_case("MediaSet") => K::MediaSet,
            _ => return None,
        };
        // A literal's type is its shape's: the checker types every text literal as a Text.
        let k = match (shape, k) {
            (Shape::Str(_), K::Text(_)) => K::Text(None),
            (Shape::Str(_), _) => return None,
            (_, k) => k,
        };
        Some(Operand {
            k,
            shape,
            node: Some(node.clone()),
            name: Name::Of(ty.clone()),
        })
    }

    /// Whether enum `e` implements interface `i` (or one that extends it). An implemented interface that does not
    /// resolve counts as `i`: an unknown is never rejected.
    fn conv_enum_implements(&self, e: usize, i: usize) -> bool {
        let family = self.ws.interface_family(i);
        let file = self.ws.objects[e].file;
        self.ws.objects[e].implements.iter().any(|p| {
            self.resolve_parts(file, ObjKind::Interface, p)
                .is_none_or(|j| family.contains(&j))
        })
    }

    /// Whether `src` converts to a `t` in `ctx` (compile matrix, contexts assign, exit, arg_byvalue, in_set,
    /// cond_branches and case_label).
    fn conv_accepts(&self, src: &Operand, t: K, ctx: Ctx) -> Result<(), Reject> {
        use K::*;
        match src.shape {
            Shape::BigInt { fits: false } if matches!(t, Int | Char | Byte | Opt | Enum(_)) => {
                return Err(Reject::Range);
            }
            Shape::Dec if t == Opt => return Err(Reject::Ordinal),
            // A one-character literal argument is a text that also converts to a Char and a Byte (arg_byvalue).
            Shape::Str(1) if ctx == Ctx::Arg => {
                return if texty(t) || matches!(t, Guid | Char | Byte | Variant) {
                    Ok(())
                } else {
                    Err(Reject::No)
                };
            }
            _ => {}
        }
        let s = src.value_kind();
        if t == Variant {
            return Ok(());
        }
        let ok = match s {
            Variant => !matches!(t, Enum(_) | Interface(_) | Media | MediaSet | Label),
            Char => numeric(t) || texty(t),
            Int | BigInt | Opt => numeric(t) || matches!(t, Enum(_)),
            Dec | Byte | Dur => numeric(t),
            Enum(e) => match t {
                Int | BigInt | Opt => true,
                Enum(f) => e == f || self.members[f].assignment_compatible,
                Interface(i) => self.conv_enum_implements(e, i),
                _ => false,
            },
            Guid | Text(_) | Code(_) | Label | TestValue => {
                if texty(t) {
                    if s == Guid && shorter_than_guid(t) && !matches!(ctx, Ctx::Arg | Ctx::Case) {
                        return Err(Reject::Overflow);
                    }
                    true
                } else {
                    t == Guid
                }
            }
            // Two different interfaces are not in the matrix: not judged.
            Interface(_) => matches!(t, Interface(_)),
            Bool | Date | Time | DateTime | DateFormula | RecordId | TextBuilder | Media | MediaSet => s == t,
        };
        if ok { Ok(()) } else { Err(Reject::No) }
    }

    /// The type name of `t` as alc writes it in a message.
    fn conv_name(&self, t: K, fallback: &Ty) -> String {
        match t {
            K::Enum(i) => format!("Enum \"{}\"", self.ws.objects[i].name),
            K::Interface(i) => format!("Interface \"{}\"", self.ws.objects[i].name),
            K::TestValue => "Text".into(),
            K::Label => "Label".into(),
            _ => fallback.to_string(),
        }
    }

    /// The diagnostic of a rejected conversion of `src` to `t` at `node`; `no` is the code of "no conversion".
    #[allow(clippy::too_many_arguments)]
    fn conv_report(
        &mut self,
        sc: &Scope,
        node: &SyntaxNode,
        src: &Operand,
        t_name: &str,
        why: Reject,
        no: &'static str,
        arg: usize,
    ) {
        let s = &src.name();
        let (code, message) = match why {
            Reject::No if no == "AL0133" => (
                "AL0133",
                format!("Argument {arg}: cannot convert from '{s}' to '{t_name}'"),
            ),
            Reject::No => (no, format!("Cannot implicitly convert type '{s}' to '{t_name}'")),
            Reject::Overflow => (
                "AL0777",
                format!(
                    "The implicit conversion will overflow when converting from a '{s}' to a '{t_name}' which is shorter than the converted textual representation of a '{s}'."
                ),
            ),
            Reject::Range => (
                "AL0369",
                format!(
                    "Constant value '{}' cannot be converted to a '{t_name}'.",
                    src.text().trim_end_matches(['L', 'l'])
                ),
            ),
            Reject::Ordinal => (
                "AL0398",
                format!(
                    "Constant value '{}' is outside of the valid ordinal range for this Option type",
                    src.text()
                ),
            ),
        };
        self.diag(sc, node, code, message);
    }

    /// `target := source` (compile matrix context assign). `None` when the site is not judged here.
    pub(crate) fn conv_assign(
        &mut self,
        sc: &Scope,
        lhs: &SyntaxNode,
        target: &Ty,
        rhs: &SyntaxNode,
        source: &Ty,
    ) -> Option<()> {
        let t = self.conv_operand(sc, lhs, target)?;
        let s = self.conv_operand(sc, rhs, source)?;
        if t.k == K::Label {
            self.diag(
                sc,
                rhs,
                "AL0129",
                "The left-hand side of an assignment must be a variable or field".into(),
            );
            return Some(());
        }
        if t.k == K::TestValue {
            // Setting a control's Value is a call that takes a Text.
            if self.conv_accepts(&s, K::Text(None), Ctx::Value).is_err() {
                self.conv_report(sc, rhs, &s, "Text", Reject::No, "AL0133", 1);
            }
            return Some(());
        }
        if !t.is_place() {
            return None;
        }
        if let Err(why) = self.conv_accepts(&s, t.k, Ctx::Value) {
            let name = self.conv_name(t.k, target);
            self.conv_report(sc, rhs, &s, &name, why, "AL0122", 0);
        }
        Some(())
    }

    /// `exit(source)` in a routine returning `ret` (compile matrix context exit).
    pub(crate) fn conv_exit(&mut self, sc: &Scope, ret: &Ty, value: &SyntaxNode, source: &Ty) -> Option<()> {
        let t = self.conv_kind(sc, ret)?;
        let s = self.conv_operand(sc, value, source)?;
        if let Err(why) = self.conv_accepts(&s, t, Ctx::Value) {
            let name = self.conv_name(t, ret);
            self.conv_report(sc, value, &s, &name, why, "AL0122", 0);
        }
        Some(())
    }

    /// The kind of a declared type (a parameter or a result), if the matrix holds it. A Label is no declarable type.
    fn conv_kind(&self, sc: &Scope, ty: &Ty) -> Option<K> {
        Some(match ty {
            Ty::Integer => K::Int,
            Ty::BigInteger => K::BigInt,
            Ty::Decimal => K::Dec,
            Ty::Boolean => K::Bool,
            Ty::Char => K::Char,
            Ty::Byte => K::Byte,
            Ty::Option(_) => K::Opt,
            Ty::Enum(n) => K::Enum(self.resolve(sc, ObjKind::Enum, n)?),
            Ty::Text(n) => K::Text(*n),
            Ty::Code(n) => K::Code(*n),
            Ty::Date => K::Date,
            Ty::Time => K::Time,
            Ty::DateTime => K::DateTime,
            Ty::Duration => K::Dur,
            Ty::DateFormula => K::DateFormula,
            Ty::Guid => K::Guid,
            Ty::Variant => K::Variant,
            Ty::Object(ObjKind::Interface, n) => K::Interface(self.resolve(sc, ObjKind::Interface, n)?),
            Ty::Builtin(b) if b.eq_ignore_ascii_case("RecordId") => K::RecordId,
            Ty::Builtin(b) if b.eq_ignore_ascii_case("TextBuilder") => K::TextBuilder,
            Ty::Builtin(b) if b.eq_ignore_ascii_case("Media") => K::Media,
            Ty::Builtin(b) if b.eq_ignore_ascii_case("MediaSet") => K::MediaSet,
            _ => return None,
        })
    }

    /// Argument `i` (0-based) `arg` of type `ty` for a parameter of type `param` (compile matrix contexts arg_byvalue and
    /// arg_var).
    pub(crate) fn conv_argument(&mut self, sc: &Scope, i: usize, arg: &SyntaxNode, ty: &Ty, param: &Ty, by_var: bool) {
        let Some(p) = self.conv_kind(sc, param) else { return };
        let Some(s) = self.conv_operand(sc, arg, ty) else {
            return;
        };
        let p_name = self.conv_name(p, param);
        if !by_var {
            if let Err(why) = self.conv_accepts(&s, p, Ctx::Arg) {
                if why != Reject::Overflow {
                    self.conv_report(sc, arg, &s, &p_name, why, "AL0133", i + 1);
                }
            }
            return;
        }
        // The type a `var` parameter takes: the same, an Integer for a BigInteger and back, any Option for an Option, an
        // Option for an Enum and back, any Text for a Text, any Code for a Code; a Variant for all but an Enum or an
        // interface.
        let fits = |s: K| match (s, p) {
            (K::Variant, p) => !matches!(p, K::Enum(_) | K::Interface(_)),
            (K::Int | K::BigInt, K::Int | K::BigInt) | (K::Opt, K::Opt) => true,
            (K::Opt, K::Enum(_)) | (K::Enum(_), K::Opt) => true,
            (K::Text(_), K::Text(_)) | (K::Code(_), K::Code(_)) => true,
            (s, p) => s == p,
        };
        let var_name = format!("var {p_name}");
        let constant = "A 'var' argument must be an assignable variable (field, variable, or parameter), not a constant or expression.";
        let literal = match s.shape {
            Shape::Place => {
                if !fits(s.k) {
                    self.conv_report(sc, arg, &s, &var_name, Reject::No, "AL0133", i + 1);
                }
                return;
            }
            // A text literal and a control's Value never fit a `var` parameter.
            Shape::Str(_) | Shape::TestValue => None,
            // An option member is an Integer constant here.
            Shape::OptionMember => Some(K::Int),
            _ => Some(s.k),
        };
        match literal {
            Some(k) if fits(k) => {
                self.diag(sc, arg, "AL0130", constant.into());
                if matches!(s.shape, Shape::BigInt { fits: false }) && p == K::Int {
                    self.conv_report(sc, arg, &s, &p_name, Reject::Range, "AL0133", i + 1);
                }
            }
            _ => self.conv_report(sc, arg, &s, &var_name, Reject::No, "AL0133", i + 1),
        }
    }

    /// `target += source` (compile matrix context compound_add): `target + source`, assigned to the target.
    pub(crate) fn conv_compound_add(
        &mut self,
        sc: &Scope,
        lhs: &SyntaxNode,
        target: &Ty,
        rhs: &SyntaxNode,
        source: &Ty,
    ) -> Option<()> {
        let t = self.conv_operand(sc, lhs, target)?;
        let s = self.conv_operand(sc, rhs, source)?;
        if matches!(t.k, K::Label | K::TestValue) {
            self.diag(
                sc,
                rhs,
                "AL0129",
                "The left-hand side of an assignment must be a variable or field".into(),
            );
            return Some(());
        }
        if !t.is_place() {
            return None;
        }
        if let Err(code) = binary_verdict("+", &t, &s) {
            let verb = if code == "AL0174" {
                "is ambiguous on"
            } else {
                "cannot be applied to"
            };
            self.diag(
                sc,
                rhs,
                code,
                format!(
                    "Operator '+=' {verb} operands of type '{}' and '{}'",
                    t.name(),
                    s.name()
                ),
            );
            return Some(());
        }
        let (lk, rk) = (t.value_kind(), s.value_kind());
        let result = if date_like(lk) {
            lk
        } else if (texty(lk) || matches!(lk, K::Guid | K::Char))
            && (texty(rk) || matches!(rk, K::Guid | K::Char))
            && !(lk == K::Char && rk == K::Char)
        {
            K::Text(None)
        } else if (int_like(lk) || matches!(lk, K::Enum(_))) && (int_like(rk) || matches!(rk, K::Enum(_))) {
            K::Int
        } else {
            K::Dec
        };
        let r = Operand {
            k: result,
            shape: Shape::Place,
            node: None,
            name: match result {
                K::Text(_) => Name::Fixed("Text"),
                K::Int => Name::Fixed("Integer"),
                K::Dec => Name::Fixed("Decimal"),
                _ => t.name.clone(),
            },
        };
        if self.conv_accepts(&r, t.k, Ctx::Value).is_err() {
            let name = t.name();
            self.conv_report(sc, rhs, &r, &name, Reject::No, "AL0122", 0);
        }
        Some(())
    }

    /// `l op r` for `=`, `<>`, `<`, `+` and `-` (compile matrix contexts eq, ne, lt, add and sub). `None` when the
    /// operator or an operand is not judged here.
    pub(crate) fn conv_binary(
        &mut self,
        sc: &Scope,
        node: &SyntaxNode,
        kids: &[SyntaxNode],
        lt: &Ty,
        rt: &Ty,
    ) -> Option<()> {
        let op = op_token(node);
        if !matches!(op.as_str(), "=" | "<>" | "<" | "+" | "-") {
            return None;
        }
        let [a, b] = kids else { return None };
        let l = self.conv_operand(sc, a, lt)?;
        let r = self.conv_operand(sc, b, rt).filter(Operand::is_variable)?;
        if let Err(code) = binary_verdict(&op, &l, &r) {
            let verb = if code == "AL0174" {
                "is ambiguous on"
            } else {
                "cannot be applied to"
            };
            self.diag(
                sc,
                node,
                code,
                format!(
                    "Operator '{op}' {verb} operands of type '{}' and '{}'",
                    l.name(),
                    r.name()
                ),
            );
        }
        Some(())
    }

    /// `value` converted to the type `of` gives: an element of `x in [...]` (`case_label` false), the false branch of a
    /// conditional expression, or a case label (`case_label` true; compile matrix contexts in_set, cond_branches,
    /// case_label). `None` when it is not judged here.
    pub(crate) fn conv_to_type_of(
        &mut self,
        sc: &Scope,
        of: &SyntaxNode,
        of_ty: &Ty,
        value: &SyntaxNode,
        value_ty: &Ty,
        case_label: bool,
    ) -> Option<()> {
        let t = self.conv_operand(sc, of, of_ty)?;
        let s = self.conv_operand(sc, value, value_ty).filter(Operand::is_variable)?;
        let tk = t.type_kind();
        let ctx = if case_label { Ctx::Case } else { Ctx::Set };
        if case_label && matches!(tk, K::Interface(_)) {
            self.diag(
                sc,
                value,
                "AL0698",
                format!("Type {} cannot be used in a 'case' statement.", t.name()),
            );
        }
        if let Err(why) = self.conv_accepts(&s, tk, ctx) {
            let name = match tk {
                K::Text(Some(n)) if matches!(t.shape, Shape::Str(_)) => format!("Text[{n}]"),
                _ => t.name(),
            };
            self.conv_report(sc, value, &s, &name, why, "AL0122", 0);
        }
        Some(())
    }
}

/// The verdict of `l op r`: `Ok`, or AL0175 (no such operator for the types) or AL0174 (ambiguous).
fn binary_verdict(op: &str, l: &Operand, r: &Operand) -> Result<(), &'static str> {
    use K::*;
    let (a, b) = (l.value_kind(), r.value_kind());
    let stringish = |k: K| texty(k) || matches!(k, Guid | Char);
    let enum_mix = |a: K, b: K| {
        matches!((a, b), (Enum(_), Enum(_)))
            || (matches!(a, Enum(_)) && int_like(b))
            || (int_like(a) && matches!(b, Enum(_)))
    };
    let ok = match op {
        "=" | "<>" | "<" => {
            numeric(a) && numeric(b)
                || enum_mix(a, b)
                || stringish(a) && stringish(b)
                || a == b && matches!(a, Bool | Date | Time | DateTime)
                || op != "<" && a == b && matches!(a, DateFormula | RecordId)
        }
        "+" | "-" => {
            // A one-character literal and a Byte: alc cannot tell Char + Byte from Text + Byte.
            let lit1 = |o: &Operand| matches!(o.shape, Shape::Str(1));
            if (lit1(l) && b == Byte) || (lit1(r) && a == Byte) {
                return Err("AL0174");
            }
            numeric(a) && numeric(b)
                || enum_mix(a, b)
                || date_like(a) && (numeric(b) || matches!(b, Enum(_)))
                || op == "+" && stringish(a) && stringish(b)
                || op == "-" && date_like(a) && a == b
        }
        _ => return Ok(()),
    };
    if ok { Ok(()) } else { Err("AL0175") }
}
