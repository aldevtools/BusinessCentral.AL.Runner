//! Permission cells corpus PR 1403 (open) measured, each run through its assumption entry in the measured shapes only:
//! lower-case `[InherentPermissions]` letters and the `IndirectRead` text (`inherent_letter`), denied reads (`reads`), and the
//! object grant of extension code (`extension_code`).
mod extension_code;
mod inherent_letter;
pub(crate) mod reads;
