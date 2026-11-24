//! The run options that each test session takes as they are (`RunOptions::ignore_permission_mock`,
//! `RunOptions::permissions_off`, `RunOptions::dev_endpoint_ui`, `RunOptions::no_client`, `RunOptions::login_replay_check`,
//! `RunOptions::login_replay`),
//! carried as one value from the options to the session. The test session's login code runs under them, so the whole value
//! is a field of the login replay check's key (`login_replay.rs` `Key::flags`): a flag added here is in the key too.
use super::RunOptions;
use al_vm::Session;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct SessionFlags {
    ignore_permission_mock: bool,
    permissions_off: bool,
    dev_endpoint_ui: bool,
    no_client: bool,
    cell_tap: bool,
    login_replay_check: bool,
    login_replay: bool,
}

impl SessionFlags {
    pub(super) fn of(o: &RunOptions) -> Self {
        SessionFlags {
            ignore_permission_mock: o.ignore_permission_mock,
            permissions_off: o.permissions_off,
            dev_endpoint_ui: o.dev_endpoint_ui,
            no_client: o.no_client,
            cell_tap: o.cell_tap,
            login_replay_check: o.login_replay_check,
            login_replay: o.login_replay,
        }
    }

    /// `RunOptions::login_replay_check`: `open_company` checks the raised login (`login_replay.rs`).
    pub(super) fn login_replay_check(self) -> bool {
        self.login_replay_check
    }

    /// Whether a test session may take a login start in place of its login (`login_start.rs`): under
    /// `RunOptions::login_replay`, and not under the check, which runs every login and compares.
    pub(super) fn login_replay(self) -> bool {
        self.login_replay && !self.login_replay_check
    }

    pub(super) fn apply(self, s: &mut Session) {
        s.ignore_permission_mock = self.ignore_permission_mock;
        s.permissions_off = self.permissions_off;
        // The dev endpoint's answers are measured in a session with a client: under `--no-client` none applies.
        s.set_dev_endpoint_ui(self.dev_endpoint_ui && !self.no_client);
        s.set_clientless_session(self.no_client);
        if self.cell_tap {
            s.enable_cell_tap();
        }
    }
}
