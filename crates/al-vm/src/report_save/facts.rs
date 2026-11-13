//! The fact vector of a rendering save (ADR-040 amendment 2026-11-13, part 1, point 1). Each fact is a closed enum; one owner
//! file computes each group: the call (`fact_call`), the report (`fact_report`), the layout selection (`fact_selection`) and
//! the run (`fact_run`). No other code computes a fact. `describe` gives the words a refusal uses.

use super::Format;

/// Where the save writes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Target {
    Stream,
    File,
}

/// How the caller takes the answer.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum AnswerForm {
    /// `if Report.SaveAs(..)`, `Ok := ..`: false comes back with the last error.
    Used,
    /// A statement: an error is raised.
    Statement,
    /// `TestRequestPage.SaveAsPdf` and `SaveAsExcel`, which have no answer.
    None,
}

/// `ir::Report::default_layout_kind`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Kind {
    Rdlc,
    Word,
    Excel,
    /// `custom`, or a kind that cannot be told.
    Other,
}

/// `ir::Report::layout` is `ReportLayout::Rdlc`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Layout {
    Rdlc,
    NotRdlc,
}

/// How the default layout is named.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Decl {
    /// `DefaultRenderingLayout` names it.
    Rendering,
    /// `DefaultLayout` with `WordLayout`/`RDLCLayout`, or only one layout.
    Legacy,
}

/// `ir::Report::layout_file_missing`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum LayoutFile {
    Present,
    Missing,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum DataItems {
    One,
    /// A top-level item and one item inside it.
    ParentAndChild,
    Other,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Extensions {
    None,
    /// Exactly one, which only adds columns (no trigger, data item, label or `modify`).
    OneAddingColumns,
    Other,
}

/// The report triggers OnInitReport, OnPreReport, OnPostReport and OnPreRendering the report declares.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Triggers {
    None,
    /// At least one, not OnPostReport.
    WithoutOnPost,
    OnPost,
}

/// Which layout selection BC follows, as Base Application's ReportManagement answers it today (round 2 takes it from the
/// events; `fact_selection`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Selection {
    /// No Report Layout Selection row of the company, no Tenant Report Layout Selection row, no design-time selection.
    None,
    /// Only rows that select a built-in layout of the default layout's kind.
    SameKind,
    /// A row selects the built-in RDLC or Word layout of the other kind, which the report declares.
    OtherKind,
    /// A row selects anything else.
    Unknown,
    /// "Design-time Report Selection" holds a selection outside the shape `TempWord` resolves.
    DesignTimeHeld,
    /// A temporary selection named one user-defined Word layout of the report (ADR-040 amendment 2026-11-12, A7).
    TempWord,
}

/// The stored Report Layout Selection row the probe measured (`fact_selection::stored`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Stored {
    None,
    /// The one row of the report is the company's, Type "RDLC (built-in)", an empty code; the report declares an RDLC layout.
    RdlcBuiltin,
    /// The one row of the report is the company's, Type "Custom Layout", a code that names no layout.
    CustomUnknownCode,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Rows {
    Zero,
    Some,
}

/// The facts of one save. The run facts (`rows`, `dataset_rgba`, `named`) are `None` before the run.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Facts {
    pub format: Format,
    pub target: Target,
    pub answer: AnswerForm,
    pub kind: Kind,
    pub layout: Layout,
    pub decl: Decl,
    pub file: LayoutFile,
    pub processing_only: bool,
    pub data_items: DataItems,
    pub extensions: Extensions,
    pub triggers: Triggers,
    /// A layout of Type Custom renders (the dataset goes to the merger; `policy`).
    pub custom: bool,
    /// Microsoft's DemoTool builds the company (`policy`).
    pub company_build: bool,
    /// The report is on the ADR-040 allow-list (`Session::output_is_ignored`).
    pub listed: bool,
    pub selection: Selection,
    pub stored: Stored,
    /// A row of Report Layout Selection (any company) or Tenant Report Layout Selection names the report; read again after
    /// the run.
    pub named: Option<bool>,
    pub rows: Option<Rows>,
    /// The dataset Xml holds the text `rgba(`.
    pub dataset_rgba: Option<bool>,
}

/// The name of each fact, in the order a refusal lists them.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Fact {
    Format,
    Target,
    Answer,
    Kind,
    Layout,
    Decl,
    File,
    ProcessingOnly,
    DataItems,
    Extensions,
    Triggers,
    Custom,
    CompanyBuild,
    Listed,
    Selection,
    Stored,
    Named,
    Rows,
    DatasetRgba,
}

impl Fact {
    pub(crate) const ALL: [Fact; 19] = [
        Fact::Format,
        Fact::Target,
        Fact::Answer,
        Fact::Kind,
        Fact::Layout,
        Fact::Decl,
        Fact::File,
        Fact::ProcessingOnly,
        Fact::DataItems,
        Fact::Extensions,
        Fact::Triggers,
        Fact::Custom,
        Fact::CompanyBuild,
        Fact::Listed,
        Fact::Selection,
        Fact::Stored,
        Fact::Named,
        Fact::Rows,
        Fact::DatasetRgba,
    ];

    /// Whether the fact is known only after the run.
    pub(crate) fn after_run(self) -> bool {
        matches!(self, Fact::Named | Fact::Rows | Fact::DatasetRgba)
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Fact::Format => "format",
            Fact::Target => "target",
            Fact::Answer => "answer",
            Fact::Kind => "default layout",
            Fact::Layout => "layout kind",
            Fact::Decl => "declared by",
            Fact::File => "layout file",
            Fact::ProcessingOnly => "processing only",
            Fact::DataItems => "data items",
            Fact::Extensions => "report extensions",
            Fact::Triggers => "report triggers",
            Fact::Custom => "custom layout",
            Fact::CompanyBuild => "company build",
            Fact::Listed => "allow-listed",
            Fact::Selection => "layout selection",
            Fact::Stored => "stored selection row",
            Fact::Named => "a selection row names the report",
            Fact::Rows => "dataset rows",
            Fact::DatasetRgba => "dataset holds rgba(",
        }
    }
}

fn yes_no(b: bool) -> &'static str {
    if b { "yes" } else { "no" }
}

/// The value of a fact as words; `None` for a run fact not known yet.
pub(crate) trait Describe {
    fn describe(&self) -> &'static str;
}

macro_rules! describe {
    ($t:ty { $($v:path => $s:literal),* $(,)? }) => {
        impl Describe for $t {
            fn describe(&self) -> &'static str {
                match self { $($v => $s),* }
            }
        }
    };
}

describe!(Format { Format::Pdf => "Pdf", Format::Word => "Word", Format::Excel => "Excel", Format::Html => "Html" });
describe!(Target { Target::Stream => "a stream", Target::File => "a file" });
describe!(AnswerForm { AnswerForm::Used => "used", AnswerForm::Statement => "a statement", AnswerForm::None => "none (request page)" });
describe!(Kind { Kind::Rdlc => "RDLC", Kind::Word => "Word", Kind::Excel => "Excel", Kind::Other => "another kind" });
describe!(Layout { Layout::Rdlc => "RDLC", Layout::NotRdlc => "not RDLC" });
describe!(Decl { Decl::Rendering => "DefaultRenderingLayout", Decl::Legacy => "DefaultLayout or a single layout" });
describe!(LayoutFile { LayoutFile::Present => "present", LayoutFile::Missing => "missing" });
describe!(DataItems { DataItems::One => "one", DataItems::ParentAndChild => "one inside one", DataItems::Other => "another tree" });
describe!(Extensions {
    Extensions::None => "none",
    Extensions::OneAddingColumns => "one adding columns",
    Extensions::Other => "another set",
});
describe!(Triggers { Triggers::None => "none", Triggers::WithoutOnPost => "without OnPostReport", Triggers::OnPost => "with OnPostReport" });
describe!(Selection {
    Selection::None => "none",
    Selection::SameKind => "a built-in layout of the default kind",
    Selection::OtherKind => "the built-in layout of the other kind",
    Selection::Unknown => "another selection",
    Selection::DesignTimeHeld => "a design-time selection",
    Selection::TempWord => "a temporary selection of a user-defined Word layout",
});
describe!(Stored {
    Stored::None => "none",
    Stored::RdlcBuiltin => "the company's RDLC (built-in) row",
    Stored::CustomUnknownCode => "the company's Custom Layout row with an unknown code",
});
describe!(Rows { Rows::Zero => "none", Rows::Some => "some" });
impl Describe for bool {
    fn describe(&self) -> &'static str {
        yes_no(*self)
    }
}
impl<T: Describe> Describe for Option<T> {
    fn describe(&self) -> &'static str {
        self.as_ref().map_or("not known before the run", Describe::describe)
    }
}

impl Facts {
    /// The value of `fact` as words.
    pub(crate) fn describe(&self, fact: Fact) -> &'static str {
        match fact {
            Fact::Format => self.format.describe(),
            Fact::Target => self.target.describe(),
            Fact::Answer => self.answer.describe(),
            Fact::Kind => self.kind.describe(),
            Fact::Layout => self.layout.describe(),
            Fact::Decl => self.decl.describe(),
            Fact::File => self.file.describe(),
            Fact::ProcessingOnly => self.processing_only.describe(),
            Fact::DataItems => self.data_items.describe(),
            Fact::Extensions => self.extensions.describe(),
            Fact::Triggers => self.triggers.describe(),
            Fact::Custom => self.custom.describe(),
            Fact::CompanyBuild => self.company_build.describe(),
            Fact::Listed => self.listed.describe(),
            Fact::Selection => self.selection.describe(),
            Fact::Stored => self.stored.describe(),
            Fact::Named => self.named.describe(),
            Fact::Rows => self.rows.describe(),
            Fact::DatasetRgba => self.dataset_rgba.describe(),
        }
    }
}
