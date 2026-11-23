//! Stage numbers and three-state options of a page as typed values: `ModalRun` (how the modal run
//! started), `ChainRefill`, `ActionSaveCaptions`, `ReadUpFromLast`, `UnreadEdges` (al-runner issue 7685).
#![deny(clippy::wildcard_enum_match_arm)]
//! Stage numbers and three-state options of a page, each one typed value. The rules that read them stay at their callers,
//! with their corpus citations; each type here names its states.

/// How the current modal run started.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ModalRun {
    /// Any other run (an action, a lookup), or no modal run.
    #[default]
    Other,
    /// The static `Page.RunModal(Id)`.
    Static,
    /// A Page variable's `RunModal()`.
    Variable,
}

/// A FactBox part of a Provider chain at the refill after its host was inserted by the first call on a part
/// (`provider_hidden.rs`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ChainRefill {
    /// Not in such a refill.
    #[default]
    Off,
    /// The refill is running and skips the part.
    Skipped,
    /// The refill is done; the key typed into the host reads the part.
    AwaitingKey,
}

/// Where an action of a Document page stands that saved a SetValue of the host before its trigger
/// (record/captions/QdCaptionClassAreas.al:1015-1049 QO_Doc_OpenEdit_HostSteps `update`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ActionSaveCaptions {
    /// No such action.
    #[default]
    Off,
    /// The refresh of that save is running.
    Refreshing,
    /// The save is done and a CurrPage.Update in the trigger has not refreshed yet.
    Saved,
    /// A CurrPage.Update in the trigger refreshed.
    Updated,
}

/// A modal list in a key other than the primary key that opened on the last row of its descending view and read the rows
/// above it (see `read_rows_up_from_the_landed_last_row`). Other moves from there are not pinned.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ReadUpFromLast {
    /// Not such a list, or a move ended it.
    #[default]
    Off,
    /// The open read the rows above the last row.
    Opened,
    /// A GoToKey after that open walked the rows from the first one.
    WalkedFromFirst,
}

/// The open read of a plain list on the row its OnOpenPage left Rec on held more rows than a window: which edges of the rows
/// it holds have rows behind them left unread. `First` and `Last` read on from that edge (al-vm
/// `read_rows_above_the_moved_rec`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnreadEdges {
    /// Rows above the top row it holds were left unread.
    pub above: bool,
    /// Rows below the bottom row it holds were left unread.
    pub below: bool,
}
