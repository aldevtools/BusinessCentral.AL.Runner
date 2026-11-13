//! The shape of one row of the save table (`cells`): the key it matches, the answer, the events, the rgba column and the
//! evidence.

use super::Format;
use super::facts::*;
use al_builtins::assumptions::Assumption;

/// One fact of a row's key: the value the row was measured with, or `Unread` when the row's answer does not read the fact
/// (step 1 round 1 keeps the facts main reads; round 3 replaces `Unread` with measured values and drop records). Rows that
/// overlap must nest, and the most specific one answers (`lookup`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum K<T> {
    Is(T),
    Unread,
}

use K::Unread;

/// A row's key: one [`K`] per fact of [`Facts`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Key {
    pub format: K<Format>,
    pub target: K<Target>,
    pub answer: K<AnswerForm>,
    pub kind: K<Kind>,
    pub layout: K<Layout>,
    pub decl: K<Decl>,
    pub declares: K<Declares>,
    pub file: K<LayoutFile>,
    pub processing_only: K<bool>,
    pub data_items: K<DataItems>,
    pub extensions: K<Extensions>,
    pub triggers: K<Triggers>,
    pub custom: K<bool>,
    pub company_build: K<bool>,
    pub listed: K<bool>,
    pub selection: K<Selection>,
    pub stored: K<Stored>,
    pub named: K<bool>,
    pub rows: K<Rows>,
    pub dataset_rgba: K<bool>,
}

impl Key {
    /// The key that reads no fact.
    pub(crate) const ANY: Key = Key {
        format: Unread,
        target: Unread,
        answer: Unread,
        kind: Unread,
        layout: Unread,
        decl: Unread,
        declares: Unread,
        file: Unread,
        processing_only: Unread,
        data_items: Unread,
        extensions: Unread,
        triggers: Unread,
        custom: Unread,
        company_build: Unread,
        listed: Unread,
        selection: Unread,
        stored: Unread,
        named: Unread,
        rows: Unread,
        dataset_rgba: Unread,
    };
}

/// Which layout renders the placeholder a row writes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Renders {
    /// The report's default layout.
    Default,
    /// The report's RDLC layout, which the stored selection row names.
    StoredRdlc,
    /// The user-defined Word layout a temporary selection named.
    Selected,
}

/// The errors a row may answer with (`errors`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ErrorId {
    /// "Report N does not have a valid layout. ..." with the code `Dialog`.
    NoValidLayout,
    /// The same text with the code `ReportNoLayout`.
    NoLayout,
    /// `ReportUnsupportedFormat`.
    UnsupportedFormat,
    /// `ReportResultEmpty`.
    ResultEmpty,
}

/// What a row answers.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Answer {
    /// Before the run: false with this error as the last error; nothing runs and nothing is written.
    False(ErrorId),
    /// Before or after the run: refused, with the reason (BC's answer is measured and not reproduced here, or the old
    /// explicit refusal of a shape the cells do not cover).
    Refuse(&'static str),
    /// Before the run (policy): run and write the dataset.
    Dataset,
    /// Before the run (policy): run and write nothing.
    Discard,
    /// After the run: write the placeholder of the save's format.
    Placeholder(Renders),
    /// After the run with no dataset row: false with `ReportResultEmpty` (a statement raises it).
    Empty,
    /// After the run with no dataset row: false, the last error left as it was.
    FalseKeepError,
    /// After the run with no dataset row: true, nothing written.
    TrueNothing,
}

impl Answer {
    /// Whether a row with this answer that reads no run fact answers before the report runs.
    pub(crate) fn before_run(self) -> bool {
        matches!(self, Answer::False(_) | Answer::Dataset | Answer::Discard) || matches!(self, Answer::Refuse(_))
    }
}

/// The layout events BC raises around the render that this runtime does not raise.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Events {
    /// A subscriber the test can see to a layout event refuses, except these events (BC does not raise them on this path,
    /// or the caller raised them already).
    NotRaised(&'static [&'static str]),
    /// The row's path checks no subscriber (main's answer).
    NotChecked,
}

/// A corpus line.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Pin {
    pub file: &'static str,
    pub line: u32,
}

/// Why a row may answer.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Evidence {
    /// Pinned by the corpus.
    Pinned(&'static [Pin]),
    /// Measured by an open probe; the row marks the assumption when it answers.
    Awaiting(Assumption, &'static [Pin]),
    /// Main's answer before the table, not measured for this shape; the cells nearest to it. A row that writes a
    /// placeholder is flagged by the placeholder's own assumption; the other kept rows run unflagged, as on main, until the
    /// step-1 probe measures them.
    Kept(&'static [Pin]),
    /// An owner decision, not a measurement.
    Policy(&'static str),
}

impl Evidence {
    /// The assumption the row marks when it answers.
    pub(crate) fn marks(self) -> Option<Assumption> {
        match self {
            Evidence::Awaiting(a, _) => Some(a),
            _ => None,
        }
    }

    /// The first corpus line, as `file:line`, for a refusal.
    pub(crate) fn cite(self) -> String {
        match self {
            Evidence::Pinned(p) | Evidence::Awaiting(_, p) | Evidence::Kept(p) => {
                p.first().map_or_else(String::new, |p| format!("{}:{}", p.file, p.line))
            }
            Evidence::Policy(why) => why.to_string(),
        }
    }
}

/// One row of the save table.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Cell {
    pub key: Key,
    pub answer: Answer,
    pub events: Events,
    /// The rgba replace of `Email Message.Create` is measured on the Html this row writes.
    pub rgba: bool,
    pub evidence: Evidence,
}
