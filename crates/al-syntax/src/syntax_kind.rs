//! Every token and node kind. Keywords are not separate kinds: AL keywords are
//! case-insensitive and mostly contextual, so the parser recognises them from
//! `Ident` text.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u16)]
pub enum SyntaxKind {
    // Trivia
    Whitespace,
    Newline,
    LineComment,
    BlockComment,
    /// A whole `#if`/`#else`/`#endif`/`#pragma`/... line, without its newline.
    Directive,
    /// UTF-8 byte order mark at the start of a file.
    Bom,
    /// Source inside an inactive `#if` branch, one token per contiguous run.
    DisabledText,

    // Atoms
    Ident,
    QuotedIdent,
    StringLit,
    Integer,
    Decimal,
    /// `20240101D`, `0D`
    DateLit,
    /// `120000T`
    TimeLit,
    /// `0DT`
    DateTimeLit,
    /// `1L`
    BigIntLit,

    // Punctuation
    LParen,
    RParen,
    LBrack,
    RBrack,
    LBrace,
    RBrace,
    Comma,
    Semicolon,
    Colon,
    ColonColon,
    Dot,
    DotDot,
    Assign,
    PlusAssign,
    MinusAssign,
    StarAssign,
    SlashAssign,
    Plus,
    Minus,
    Star,
    Slash,
    Eq,
    Neq,
    Lt,
    Le,
    Gt,
    Ge,
    /// Ternary `cond ? a : b`.
    Question,
    /// Alternative separator in `filter(A | B)` expressions.
    Pipe,
    /// Term joiner in `filter(<> A & <> B)` expressions (record/lt-table-relation-quotes/TableRelationQuotes.al:65).
    Amp,

    /// Anything the lexer cannot classify, including unterminated strings and
    /// comments. The lexer never fails; errors are tokens.
    Error,

    // ---- Nodes ----
    SourceFile,
    /// Tokens the parser could not fit into the grammar.
    ErrorNode,

    // Expressions
    Literal,
    NameRef,
    BinExpr,
    PrefixExpr,
    MemberExpr,
    ScopeExpr,
    CallExpr,
    ArgList,
    IndexExpr,
    ParenExpr,
    SetExpr,
    RangeExpr,
    TernaryExpr,

    // Statements
    Block,
    EmptyStmt,
    AssignStmt,
    ExprStmt,
    IfStmt,
    CaseStmt,
    CaseBranch,
    CaseElse,
    WhileStmt,
    RepeatStmt,
    ForStmt,
    ForeachStmt,
    WithStmt,
    ExitStmt,
    BreakStmt,
    ContinueStmt,
    AssertErrorStmt,

    // Declarations
    VarSection,
    VarDecl,
    TypeRef,
    TypeArgs,
    Procedure,
    Trigger,
    ParamList,
    Param,
    ReturnSpec,
    Attribute,

    // Objects
    NamespaceDecl,
    UsingDecl,
    Object,
    /// A top-level `dotnet { assembly(...) { type(...) } }` declaration block (no id, no name).
    DotNetBlock,
    ObjectBody,
    Property,
    PropertyValue,
    Section,
    SectionArgs,

    #[doc(hidden)]
    __Last,
}

impl SyntaxKind {
    pub fn is_trivia(self) -> bool {
        matches!(
            self,
            SyntaxKind::Whitespace
                | SyntaxKind::Newline
                | SyntaxKind::LineComment
                | SyntaxKind::BlockComment
                | SyntaxKind::Directive
                | SyntaxKind::Bom
                | SyntaxKind::DisabledText
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AlLanguage {}

impl rowan::Language for AlLanguage {
    type Kind = SyntaxKind;
    fn kind_from_raw(raw: rowan::SyntaxKind) -> SyntaxKind {
        assert!(raw.0 < SyntaxKind::__Last as u16);
        // SAFETY: SyntaxKind is repr(u16) with contiguous discriminants below __Last.
        unsafe { std::mem::transmute::<u16, SyntaxKind>(raw.0) }
    }
    fn kind_to_raw(kind: SyntaxKind) -> rowan::SyntaxKind {
        rowan::SyntaxKind(kind as u16)
    }
}

pub type SyntaxNode = rowan::SyntaxNode<AlLanguage>;
pub type SyntaxToken = rowan::SyntaxToken<AlLanguage>;

impl From<SyntaxKind> for rowan::SyntaxKind {
    fn from(k: SyntaxKind) -> Self {
        rowan::SyntaxKind(k as u16)
    }
}
