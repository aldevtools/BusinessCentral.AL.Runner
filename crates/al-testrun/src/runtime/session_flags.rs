//! The run options that each test session takes as they are (`RunOptions::ignore_permission_mock`,
//! `RunOptions::permissions_off`, `RunOptions::dev_endpoint_ui`), carried as one value from the options to the session.
use super::RunOptions;
use al_vm::Session;

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct SessionFlags {
    ignore_permission_mock: bool,
    permissions_off: bool,
    dev_endpoint_ui: bool,
}

impl SessionFlags {
    pub(super) fn of(o: &RunOptions) -> Self {
        SessionFlags {
            ignore_permission_mock: o.ignore_permission_mock,
            permissions_off: o.permissions_off,
            dev_endpoint_ui: o.dev_endpoint_ui,
        }
    }

    pub(super) fn apply(self, s: &mut Session) {
        s.ignore_permission_mock = self.ignore_permission_mock;
        s.permissions_off = self.permissions_off;
        s.set_dev_endpoint_ui(self.dev_endpoint_ui);
    }
}
