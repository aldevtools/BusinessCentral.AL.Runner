//! Permission cells that tests/al-language/permissions/testpermissions/Y19PermExtAndCompose.al and Y19InherentAttrIndirect.al
//! pin, each run in the measured shapes only:
//! lower-case `[InherentPermissions]` letters and the `IndirectRead` text (`inherent_letter`), denied reads (`reads`), and the
//! object grant of extension code (`extension_code`).
mod extension_code;
mod inherent_letter;
pub(crate) mod reads;
