//! The assemblies a .NET assembly references: the names in the `AssemblyRef` table of its metadata (ECMA-335
//! partition II, sections 24 and 22). [`crate::service_tier`] follows them to fetch only the service-tier
//! assemblies a declared one needs. Pure: it reads a byte slice and never loads the assembly.

/// The simple names in `bytes`' `AssemblyRef` table, in table order. `None` when `bytes` is not a .NET assembly
/// this reader understands (no CLI header, a damaged table stream); the caller then follows no references.
pub fn assembly_refs(bytes: &[u8]) -> Option<Vec<String>> {
    metadata(bytes).map(|m| m.0)
}

/// The version in `bytes`' `Assembly` table (major, minor, build, revision). `None` when `bytes` is not a .NET assembly
/// this reader understands, or has no `Assembly` row (a module).
pub fn assembly_version(bytes: &[u8]) -> Option<[u16; 4]> {
    metadata(bytes)?.1
}

/// The `AssemblyRef` names and the `Assembly` version of `bytes`.
fn metadata(bytes: &[u8]) -> Option<(Vec<String>, Option<[u16; 4]>)> {
    let r = Reader(bytes);
    let pe = r.u32(0x3c)? as usize;
    if r.slice(pe, 4)? != b"PE\0\0" {
        return None;
    }
    let sections = r.u16(pe + 6)? as usize;
    let optional = pe + 24;
    let optional_size = r.u16(pe + 20)? as usize;
    let directories = match r.u16(optional)? {
        0x10b => optional + 96,
        0x20b => optional + 112,
        _ => return None,
    };
    // Data directory 14 is the CLI header.
    let cli_rva = r.u32(directories + 14 * 8)?;
    if cli_rva == 0 {
        return None;
    }
    let table = optional + optional_size;
    let offset_of = |rva: u32| -> Option<usize> {
        (0..sections).find_map(|i| {
            let s = table + 40 * i;
            let va = r.u32(s + 12)?;
            let size = r.u32(s + 8)?.max(r.u32(s + 16)?);
            (va <= rva && rva < va.checked_add(size)?)
                .then(|| (rva - va) as usize + r.u32(s + 20).unwrap_or(0) as usize)
        })
    };
    let cli = offset_of(cli_rva)?;
    let root = offset_of(r.u32(cli + 8)?)?;
    if r.u32(root)? != 0x424A_5342 {
        return None;
    }
    let version_len = r.u32(root + 12)? as usize;
    let mut at = root + 16 + version_len;
    let streams = r.u16(at + 2)?;
    at += 4;
    let (mut tables, mut strings) = (None, None);
    for _ in 0..streams {
        let (offset, size) = (r.u32(at)? as usize, r.u32(at + 4)? as usize);
        let name_at = at + 8;
        let len = bytes.get(name_at..)?.iter().position(|&b| b == 0)?;
        let name = &bytes[name_at..name_at + len];
        at = (name_at + len + 4) & !3;
        let start = root.checked_add(offset)?;
        match name {
            b"#~" | b"#-" => tables = Some(start),
            b"#Strings" => strings = Some(r.slice(start, size)?),
            _ => {}
        }
    }
    read_assembly_refs(&r, tables?, strings?)
}

/// Rows of the tables before `AssemblyRef` (0x23) are skipped by their sizes; the coded index sizes follow II.24.2.6.
fn read_assembly_refs(r: &Reader, tables: usize, strings: &[u8]) -> Option<(Vec<String>, Option<[u16; 4]>)> {
    let heaps = *r.0.get(tables + 6)?;
    let valid = r.u64(tables + 8)?;
    let mut rows = [0u32; 64];
    let mut at = tables + 24;
    for (i, n) in rows.iter_mut().enumerate() {
        if valid >> i & 1 == 1 {
            *n = r.u32(at)?;
            at += 4;
        }
    }
    if heaps & 0x40 != 0 {
        at += 4;
    }
    let (s, g, b) = (wide(heaps & 1), wide(heaps & 2), wide(heaps & 4));
    let idx = |t: usize| if rows[t] >= 1 << 16 { 4 } else { 2 };
    let coded = |bits: u32, ts: &[usize]| {
        let max = ts.iter().map(|&t| rows[t]).max().unwrap_or(0);
        if max >= 1 << (16 - bits) { 4 } else { 2 }
    };
    let type_def_or_ref = coded(2, &[0x02, 0x01, 0x1b]);
    let resolution_scope = coded(2, &[0x00, 0x1a, 0x23, 0x01]);
    let member_ref_parent = coded(3, &[0x02, 0x01, 0x1a, 0x06, 0x1b]);
    let has_constant = coded(2, &[0x04, 0x08, 0x17]);
    let has_custom_attribute = coded(
        5,
        &[
            0x06, 0x04, 0x01, 0x02, 0x08, 0x09, 0x0a, 0x00, 0x0e, 0x17, 0x14, 0x11, 0x1a, 0x1b, 0x20, 0x23, 0x26, 0x27,
            0x28, 0x2a, 0x2c, 0x2b,
        ],
    );
    let custom_attribute_type = coded(3, &[0x06, 0x0a]);
    let has_field_marshal = coded(1, &[0x04, 0x08]);
    let has_decl_security = coded(2, &[0x02, 0x06, 0x20]);
    let has_semantics = coded(1, &[0x14, 0x17]);
    let method_def_or_ref = coded(1, &[0x06, 0x0a]);
    let member_forwarded = coded(1, &[0x04, 0x06]);
    let row_size: [usize; 0x23] = [
        2 + s + 3 * g,                                       // 0x00 Module
        resolution_scope + 2 * s,                            // 0x01 TypeRef
        4 + 2 * s + type_def_or_ref + idx(0x04) + idx(0x06), // 0x02 TypeDef
        idx(0x04),                                           // 0x03 FieldPtr
        2 + s + b,                                           // 0x04 Field
        idx(0x06),                                           // 0x05 MethodPtr
        8 + s + b + idx(0x08),                               // 0x06 MethodDef
        idx(0x08),                                           // 0x07 ParamPtr
        4 + s,                                               // 0x08 Param
        idx(0x02) + type_def_or_ref,                         // 0x09 InterfaceImpl
        member_ref_parent + s + b,                           // 0x0a MemberRef
        2 + has_constant + b,                                // 0x0b Constant
        has_custom_attribute + custom_attribute_type + b,    // 0x0c CustomAttribute
        has_field_marshal + b,                               // 0x0d FieldMarshal
        2 + has_decl_security + b,                           // 0x0e DeclSecurity
        6 + idx(0x02),                                       // 0x0f ClassLayout
        4 + idx(0x04),                                       // 0x10 FieldLayout
        b,                                                   // 0x11 StandAloneSig
        idx(0x02) + idx(0x14),                               // 0x12 EventMap
        idx(0x14),                                           // 0x13 EventPtr
        2 + s + type_def_or_ref,                             // 0x14 Event
        idx(0x02) + idx(0x17),                               // 0x15 PropertyMap
        idx(0x17),                                           // 0x16 PropertyPtr
        2 + s + b,                                           // 0x17 Property
        2 + idx(0x06) + has_semantics,                       // 0x18 MethodSemantics
        idx(0x02) + 2 * method_def_or_ref,                   // 0x19 MethodImpl
        s,                                                   // 0x1a ModuleRef
        b,                                                   // 0x1b TypeSpec
        2 + member_forwarded + s + idx(0x1a),                // 0x1c ImplMap
        4 + idx(0x04),                                       // 0x1d FieldRVA
        8,                                                   // 0x1e EncLog
        4,                                                   // 0x1f EncMap
        16 + b + 2 * s,                                      // 0x20 Assembly
        4,                                                   // 0x21 AssemblyProcessor
        12,                                                  // 0x22 AssemblyOS
    ];
    let mut version = None;
    for (t, size) in row_size.iter().enumerate() {
        // Assembly: u32 hash algorithm, then four u16 version parts.
        if t == 0x20 && rows[t] > 0 {
            version = Some([r.u16(at + 4)?, r.u16(at + 6)?, r.u16(at + 8)?, r.u16(at + 10)?]);
        }
        at = at.checked_add((rows[t] as usize).checked_mul(*size)?)?;
    }
    // AssemblyRef: four u16 version parts, u32 flags, blob public key or token, string name, string culture, blob hash.
    let assembly_ref = 12 + 2 * b + 2 * s;
    let mut out = Vec::with_capacity(rows[0x23] as usize);
    for i in 0..rows[0x23] as usize {
        let name_at = at + i * assembly_ref + 12 + b;
        let offset = if s == 4 {
            r.u32(name_at)? as usize
        } else {
            r.u16(name_at)? as usize
        };
        let rest = strings.get(offset..)?;
        let end = rest.iter().position(|&c| c == 0)?;
        out.push(String::from_utf8_lossy(&rest[..end]).into_owned());
    }
    Some((out, version))
}

fn wide(flag: u8) -> usize {
    if flag != 0 { 4 } else { 2 }
}

struct Reader<'a>(&'a [u8]);

impl Reader<'_> {
    fn slice(&self, at: usize, len: usize) -> Option<&[u8]> {
        self.0.get(at..at.checked_add(len)?)
    }
    fn u16(&self, at: usize) -> Option<u16> {
        Some(u16::from_le_bytes(self.slice(at, 2)?.try_into().ok()?))
    }
    fn u32(&self, at: usize) -> Option<u32> {
        Some(u32::from_le_bytes(self.slice(at, 4)?.try_into().ok()?))
    }
    fn u64(&self, at: usize) -> Option<u64> {
        Some(u64::from_le_bytes(self.slice(at, 8)?.try_into().ok()?))
    }
}

#[cfg(test)]
pub(crate) mod testutil {
    //! A minimal PE32 file with CLI metadata that holds a Module row and the given `AssemblyRef` rows. Nothing here is
    //! Microsoft's.

    pub fn assembly(refs: &[&str]) -> Vec<u8> {
        let mut strings = vec![0u8];
        let mut name_offsets = Vec::new();
        for r in refs {
            name_offsets.push(strings.len() as u16);
            strings.extend_from_slice(r.as_bytes());
            strings.push(0);
        }
        while strings.len() % 4 != 0 {
            strings.push(0);
        }
        let mut tables = Vec::new();
        tables.extend_from_slice(&0u32.to_le_bytes());
        tables.extend_from_slice(&[2, 0, 0, 1]);
        tables.extend_from_slice(&((1u64 << 0x00) | (1u64 << 0x23)).to_le_bytes());
        tables.extend_from_slice(&0u64.to_le_bytes());
        tables.extend_from_slice(&1u32.to_le_bytes());
        tables.extend_from_slice(&(refs.len() as u32).to_le_bytes());
        tables.extend_from_slice(&[0u8; 10]); // the Module row
        for off in &name_offsets {
            tables.extend_from_slice(&[0u8; 12]); // version and flags
            tables.extend_from_slice(&0u16.to_le_bytes()); // public key or token
            tables.extend_from_slice(&off.to_le_bytes());
            tables.extend_from_slice(&0u16.to_le_bytes()); // culture
            tables.extend_from_slice(&0u16.to_le_bytes()); // hash
        }
        while tables.len() % 4 != 0 {
            tables.push(0);
        }
        // Metadata root: signature, versions, reserved, version string, flags, two stream headers.
        let mut root = Vec::new();
        root.extend_from_slice(&0x424A_5342u32.to_le_bytes());
        root.extend_from_slice(&[1, 0, 1, 0, 0, 0, 0, 0]);
        root.extend_from_slice(&12u32.to_le_bytes());
        root.extend_from_slice(b"v4.0.30319\0\0");
        root.extend_from_slice(&[0, 0, 2, 0]);
        let headers = 8 + 4 + 8 + 12; // "#~\0\0" and "#Strings\0\0\0\0"
        let tables_at = (root.len() + headers) as u32;
        root.extend_from_slice(&tables_at.to_le_bytes());
        root.extend_from_slice(&(tables.len() as u32).to_le_bytes());
        root.extend_from_slice(b"#~\0\0");
        root.extend_from_slice(&(tables_at + tables.len() as u32).to_le_bytes());
        root.extend_from_slice(&(strings.len() as u32).to_le_bytes());
        root.extend_from_slice(b"#Strings\0\0\0\0");
        root.extend_from_slice(&tables);
        root.extend_from_slice(&strings);
        const VA: u32 = 0x2000;
        const RAW: usize = 0x200;
        let mut section = vec![0u8; 72]; // CLI header
        section[0..4].copy_from_slice(&72u32.to_le_bytes());
        section[8..12].copy_from_slice(&(VA + 72).to_le_bytes());
        section[12..16].copy_from_slice(&(root.len() as u32).to_le_bytes());
        section.extend_from_slice(&root);
        let mut out = vec![0u8; RAW];
        out[0..2].copy_from_slice(b"MZ");
        out[0x3c..0x40].copy_from_slice(&0x40u32.to_le_bytes());
        out[0x40..0x44].copy_from_slice(b"PE\0\0");
        let coff = 0x44;
        out[coff..coff + 2].copy_from_slice(&0x14cu16.to_le_bytes());
        out[coff + 2..coff + 4].copy_from_slice(&1u16.to_le_bytes());
        out[coff + 16..coff + 18].copy_from_slice(&224u16.to_le_bytes());
        let opt = coff + 20;
        out[opt..opt + 2].copy_from_slice(&0x10bu16.to_le_bytes());
        let cli_dir = opt + 96 + 14 * 8;
        out[cli_dir..cli_dir + 4].copy_from_slice(&VA.to_le_bytes());
        out[cli_dir + 4..cli_dir + 8].copy_from_slice(&72u32.to_le_bytes());
        let sec = opt + 224;
        let len = section.len() as u32;
        out[sec + 8..sec + 12].copy_from_slice(&len.to_le_bytes());
        out[sec + 12..sec + 16].copy_from_slice(&VA.to_le_bytes());
        out[sec + 16..sec + 20].copy_from_slice(&len.to_le_bytes());
        out[sec + 20..sec + 24].copy_from_slice(&(RAW as u32).to_le_bytes());
        out.extend_from_slice(&section);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_assembly_ref_names_are_read_in_table_order() {
        let bytes = testutil::assembly(&[
            "System.Runtime",
            "DocumentFormat.OpenXml",
            "Microsoft.Dynamics.Nav.Types",
        ]);
        assert_eq!(
            assembly_refs(&bytes),
            Some(vec![
                "System.Runtime".to_string(),
                "DocumentFormat.OpenXml".to_string(),
                "Microsoft.Dynamics.Nav.Types".to_string()
            ])
        );
        assert_eq!(assembly_refs(&testutil::assembly(&[])), Some(Vec::new()));
    }

    #[test]
    fn a_file_that_is_not_an_assembly_has_no_references() {
        assert_eq!(assembly_refs(b"not a PE file"), None);
        assert_eq!(assembly_refs(&[]), None);
        let mut cut = testutil::assembly(&["A"]);
        cut.truncate(0x260);
        assert_eq!(assembly_refs(&cut), None);
    }

    /// The real reader against a real assembly of the installed .NET runtime, when there is one.
    #[test]
    fn a_framework_assembly_of_the_installed_runtime_references_system_runtime() {
        let Some(dll) = ["/usr/share/dotnet", "/usr/lib/dotnet", "/usr/local/share/dotnet"]
            .iter()
            .filter_map(|d| std::fs::read_dir(format!("{d}/shared/Microsoft.NETCore.App")).ok())
            .flatten()
            .flatten()
            .map(|e| e.path().join("System.Text.Json.dll"))
            .find(|p| p.is_file())
        else {
            eprintln!("SKIPPED: no .NET runtime with System.Text.Json.dll in the usual folders");
            return;
        };
        let refs = assembly_refs(&std::fs::read(&dll).unwrap()).expect("a .NET assembly");
        assert!(
            refs.iter().any(|r| r == "System.Runtime"),
            "{}: {refs:?}",
            dll.display()
        );
        // The runtime folder is named by the runtime version; its assemblies carry the major as their version.
        let folder = dll.parent().unwrap().file_name().unwrap().to_string_lossy().into_owned();
        let major: u16 = folder.split('.').next().unwrap().parse().unwrap();
        let v = assembly_version(&std::fs::read(&dll).unwrap()).expect("an Assembly row");
        assert_eq!((v[0], v[2], v[3]), (major, 0, 0), "{}: {v:?}", dll.display());
    }

    #[test]
    fn a_module_without_an_assembly_row_has_no_version() {
        let bytes = testutil::assembly(&["System.Runtime"]);
        assert_eq!(assembly_version(&bytes), None);
        assert_eq!(assembly_refs(&bytes).unwrap(), vec!["System.Runtime".to_string()]);
    }
}
