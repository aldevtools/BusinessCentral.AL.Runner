//! Permission cells that tests/al-language/permissions/testpermissions/Y19PermExtAndCompose.al and Y19InherentAttrIndirect.al
//! pin, each run in the measured shapes only:
//! lower-case `[InherentPermissions]` letters and the `IndirectRead` text (`inherent_letter`), denied reads (`reads`), and the
//! object grant of extension code (`extension_code`). Al-language-testlibraries/test-framework/testpermissions/BiTp*.al pin the
//! Insert the test runner's permission sets deny (`runner_sets_insert`).
mod extension_code;
mod inherent_letter;
pub(crate) mod reads;
mod runner_sets_insert;
