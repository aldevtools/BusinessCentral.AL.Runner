//! The assemblies a .NET assembly references: the names in the `AssemblyRef` table of its metadata (ECMA-335
//! partition II, sections 24 and 22). [`crate::service_tier`] follows them to fetch only the service-tier
//! assemblies a declared one needs; [`literal_i32`] reads a `const int` of a type. Pure: it reads a byte slice and never loads
//! the assembly.

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
    let t = Tables::parse(bytes)?;
    // Assembly: u32 hash algorithm, then four u16 version parts.
    let version = match t.rows[0x20] {
        0 => None,
        _ => {
            let at = t.start[0x20];
            Some([t.r.u16(at + 4)?, t.r.u16(at + 6)?, t.r.u16(at + 8)?, t.r.u16(at + 10)?])
        }
    };
    // AssemblyRef: four u16 version parts, u32 flags, blob public key or token, string name, string culture, blob hash.
    let assembly_ref = 12 + 2 * t.b + 2 * t.s;
    let mut out = Vec::with_capacity(t.rows[0x23] as usize);
    for i in 0..t.rows[0x23] as usize {
        out.push(t.string(t.start[0x23] + i * assembly_ref + 12 + t.b)?.to_string());
    }
    Some((out, version))
}

/// The value of the `int32` literal field `field` of the type `namespace`.`type_name` defined in `bytes` (a `const int`
/// in C#): the field's row in the `Constant` table (ECMA-335 II.22.9). `None` when `bytes` is not an assembly this reader
/// understands, defines no such type or field, or the field is not an `int32` literal.
pub fn literal_i32(bytes: &[u8], namespace: &str, type_name: &str, field: &str) -> Option<i32> {
    let t = Tables::parse(bytes)?;
    // A FieldPtr table (an unoptimized `#-` stream) puts an indirection between a type and its fields: not read.
    if t.rows[0x03] != 0 {
        return None;
    }
    let fields = t.rows[0x04] as usize;
    let type_def = (0..t.rows[0x02] as usize).find(|&i| {
        let row = t.start[0x02] + i * t.size[0x02];
        t.string(row + 4) == Some(type_name) && t.string(row + 4 + t.s) == Some(namespace)
    })?;
    // TypeDef: u32 flags, string name, string namespace, TypeDefOrRef extends, Field index (the first of its run).
    let field_list = |i: usize| -> Option<usize> {
        if i >= t.rows[0x02] as usize {
            return Some(fields + 1);
        }
        t.index(
            t.start[0x02] + i * t.size[0x02] + 4 + 2 * t.s + t.type_def_or_ref,
            t.wide_index(0x04),
        )
    };
    let (first, end) = (field_list(type_def)?, field_list(type_def + 1)?);
    // Field: u16 flags, string name, blob signature. 0x0050 = Static | Literal.
    let row = (first..end.min(fields + 1)).find(|&f| {
        let at = t.start[0x04] + (f - 1) * t.size[0x04];
        t.r.u16(at).is_some_and(|flags| flags & 0x0050 == 0x0050) && t.string(at + 2) == Some(field)
    })?;
    // Constant: u8 type, u8 padding, HasConstant parent (tag 0 = Field), blob value.
    let parent = (row << 2) as u32;
    let constant = (0..t.rows[0x0b] as usize).find_map(|i| {
        let at = t.start[0x0b] + i * t.size[0x0b];
        (t.index(at + 2, t.has_constant)? as u32 == parent).then_some(at)
    })?;
    // ELEMENT_TYPE_I4.
    if *t.r.0.get(constant)? != 0x08 {
        return None;
    }
    let value = t.blob(t.index(constant + 2 + t.has_constant, t.b)?)?;
    Some(i32::from_le_bytes(value.try_into().ok()?))
}

/// The table stream of an assembly's metadata, with where each table up to `AssemblyRef` (0x23) starts and how wide its
/// rows are (ECMA-335 partition II, sections 24 and 22).
struct Tables<'a> {
    r: Reader<'a>,
    strings: &'a [u8],
    blobs: &'a [u8],
    rows: [u32; 64],
    start: [usize; 0x24],
    size: [usize; 0x23],
    /// The widths of a string and a blob heap index, and of the two coded indexes the readers above need.
    s: usize,
    b: usize,
    type_def_or_ref: usize,
    has_constant: usize,
}

impl<'a> Tables<'a> {
    fn parse(bytes: &'a [u8]) -> Option<Tables<'a>> {
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
        let (mut tables, mut strings, mut blobs) = (None, None, &[][..]);
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
                b"#Blob" => blobs = r.slice(start, size)?,
                _ => {}
            }
        }
        Self::layout(r, tables?, strings?, blobs)
    }

    /// Rows of each table are skipped by their sizes; the coded index sizes follow II.24.2.6.
    fn layout(r: Reader<'a>, tables: usize, strings: &'a [u8], blobs: &'a [u8]) -> Option<Tables<'a>> {
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
                0x06, 0x04, 0x01, 0x02, 0x08, 0x09, 0x0a, 0x00, 0x0e, 0x17, 0x14, 0x11, 0x1a, 0x1b, 0x20, 0x23, 0x26,
                0x27, 0x28, 0x2a, 0x2c, 0x2b,
            ],
        );
        let custom_attribute_type = coded(3, &[0x06, 0x0a]);
        let has_field_marshal = coded(1, &[0x04, 0x08]);
        let has_decl_security = coded(2, &[0x02, 0x06, 0x20]);
        let has_semantics = coded(1, &[0x14, 0x17]);
        let method_def_or_ref = coded(1, &[0x06, 0x0a]);
        let member_forwarded = coded(1, &[0x04, 0x06]);
        let size: [usize; 0x23] = [
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
        let mut start = [0usize; 0x24];
        for (t, size) in size.iter().enumerate() {
            start[t] = at;
            at = at.checked_add((rows[t] as usize).checked_mul(*size)?)?;
        }
        start[0x23] = at;
        Some(Tables {
            r,
            strings,
            blobs,
            rows,
            start,
            size,
            s,
            b,
            type_def_or_ref,
            has_constant,
        })
    }

    /// The width of a simple index into table `t`.
    fn wide_index(&self, t: usize) -> usize {
        if self.rows[t] >= 1 << 16 { 4 } else { 2 }
    }

    /// An index of `width` bytes at `at`.
    fn index(&self, at: usize, width: usize) -> Option<usize> {
        match width {
            4 => self.r.u32(at).map(|v| v as usize),
            _ => self.r.u16(at).map(usize::from),
        }
    }

    /// The string the string heap index at `at` names.
    fn string(&self, at: usize) -> Option<&'a str> {
        let rest = self.strings.get(self.index(at, self.s)?..)?;
        let end = rest.iter().position(|&c| c == 0)?;
        std::str::from_utf8(&rest[..end]).ok()
    }

    /// The blob at `offset` of the blob heap, after its compressed length (II.24.2.4).
    fn blob(&self, offset: usize) -> Option<&'a [u8]> {
        let h = self.blobs;
        let first = *h.get(offset)?;
        let (len, skip) = match first {
            f if f & 0x80 == 0 => (usize::from(f), 1),
            f if f & 0xc0 == 0x80 => ((usize::from(f & 0x3f) << 8) | usize::from(*h.get(offset + 1)?), 2),
            _ => return None,
        };
        h.get(offset + skip..offset + skip + len)
    }
}

fn wide(flag: u8) -> usize {
    if flag != 0 { 4 } else { 2 }
}

#[derive(Clone, Copy)]
struct Reader<'a>(&'a [u8]);

impl<'a> Reader<'a> {
    fn slice(&self, at: usize, len: usize) -> Option<&'a [u8]> {
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
    //! Minimal PE32 files with CLI metadata: a Module row and the given `AssemblyRef` rows ([`assembly`]), or a type with
    //! one literal field ([`literal`]). Nothing here is Microsoft's.

    pub fn assembly(refs: &[&str]) -> Vec<u8> {
        let mut strings = vec![0u8];
        let mut name_offsets = Vec::new();
        for r in refs {
            name_offsets.push(strings.len() as u16);
            strings.extend_from_slice(r.as_bytes());
            strings.push(0);
        }
        let mut tables = header(&[(0x00, 1), (0x23, refs.len() as u32)]);
        tables.extend_from_slice(&[0u8; 10]); // the Module row
        for off in &name_offsets {
            tables.extend_from_slice(&[0u8; 12]); // version and flags
            tables.extend_from_slice(&0u16.to_le_bytes()); // public key or token
            tables.extend_from_slice(&off.to_le_bytes());
            tables.extend_from_slice(&0u16.to_le_bytes()); // culture
            tables.extend_from_slice(&0u16.to_le_bytes()); // hash
        }
        pe(tables, strings, None)
    }

    /// A module that defines the type `namespace`.`type_name` with one literal field `field` of the element type
    /// `element` (0x08 is `int32`) whose constant blob is `value`.
    pub fn literal(namespace: &str, type_name: &str, field: &str, element: u8, value: &[u8]) -> Vec<u8> {
        let mut strings = vec![0u8];
        let mut add = |s: &str| {
            let at = strings.len() as u16;
            strings.extend_from_slice(s.as_bytes());
            strings.push(0);
            at
        };
        let (module, ns, ty, f) = (add("<Module>"), add(namespace), add(type_name), add(field));
        // The blob heap: the empty blob, the field signature (FIELD, the element type), the value.
        let mut blobs = vec![0u8, 2, 0x06, element];
        let value_at = blobs.len() as u16;
        blobs.push(value.len() as u8);
        blobs.extend_from_slice(value);
        let mut tables = header(&[(0x00, 1), (0x02, 2), (0x04, 1), (0x0b, 1)]);
        tables.extend_from_slice(&[0u8; 10]); // the Module row
        let u16s = |t: &mut Vec<u8>, vs: &[u16]| vs.iter().for_each(|v| t.extend_from_slice(&v.to_le_bytes()));
        // TypeDef: flags, name, namespace, extends, the first field, the first method.
        tables.extend_from_slice(&0u32.to_le_bytes());
        u16s(&mut tables, &[module, 0, 0, 1, 1]);
        tables.extend_from_slice(&0u32.to_le_bytes());
        u16s(&mut tables, &[ty, ns, 0, 1, 1]);
        // Field: Private | Static | Literal | HasDefault, name, signature.
        u16s(&mut tables, &[0x8051, f, 1]);
        // Constant: element type and padding, parent (field 1, tag 0), value.
        tables.extend_from_slice(&[element, 0]);
        u16s(&mut tables, &[1 << 2, value_at]);
        pe(tables, strings, Some(blobs))
    }

    /// The start of a table stream with 2-byte heap indexes: the rows of each present table.
    fn header(rows: &[(u32, u32)]) -> Vec<u8> {
        let mut t = Vec::new();
        t.extend_from_slice(&0u32.to_le_bytes());
        t.extend_from_slice(&[2, 0, 0, 1]);
        let valid = rows.iter().fold(0u64, |m, (table, _)| m | (1u64 << table));
        t.extend_from_slice(&valid.to_le_bytes());
        t.extend_from_slice(&0u64.to_le_bytes());
        for (_, n) in rows {
            t.extend_from_slice(&n.to_le_bytes());
        }
        t
    }

    /// A PE32 file whose CLI metadata holds `tables` as its `#~` stream and the given heaps.
    fn pe(mut tables: Vec<u8>, mut strings: Vec<u8>, blobs: Option<Vec<u8>>) -> Vec<u8> {
        for s in [&mut tables, &mut strings] {
            while s.len() % 4 != 0 {
                s.push(0);
            }
        }
        let mut streams: Vec<(&[u8], Vec<u8>)> = vec![(b"#~\0\0", tables), (b"#Strings\0\0\0\0", strings)];
        if let Some(mut b) = blobs {
            while b.len() % 4 != 0 {
                b.push(0);
            }
            streams.push((b"#Blob\0\0\0", b));
        }
        // Metadata root: signature, versions, reserved, version string, flags, the stream headers.
        let mut root = Vec::new();
        root.extend_from_slice(&0x424A_5342u32.to_le_bytes());
        root.extend_from_slice(&[1, 0, 1, 0, 0, 0, 0, 0]);
        root.extend_from_slice(&12u32.to_le_bytes());
        root.extend_from_slice(b"v4.0.30319\0\0");
        root.extend_from_slice(&[0, 0, streams.len() as u8, 0]);
        let headers: usize = streams.iter().map(|(name, _)| 8 + name.len()).sum();
        let mut at = (root.len() + headers) as u32;
        for (name, data) in &streams {
            root.extend_from_slice(&at.to_le_bytes());
            root.extend_from_slice(&(data.len() as u32).to_le_bytes());
            root.extend_from_slice(name);
            at += data.len() as u32;
        }
        for (_, data) in &streams {
            root.extend_from_slice(data);
        }
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
        let folder = dll
            .parent()
            .unwrap()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let major: u16 = folder.split('.').next().unwrap().parse().unwrap();
        let v = assembly_version(&std::fs::read(&dll).unwrap()).expect("an Assembly row");
        assert_eq!((v[0], v[2], v[3]), (major, 0, 0), "{}: {v:?}", dll.display());
    }

    #[test]
    fn the_literal_of_a_type_is_read_from_its_constant_row() {
        let bytes = testutil::literal("N.S", "T", "F", 0x08, &29022i32.to_le_bytes());
        assert_eq!(literal_i32(&bytes, "N.S", "T", "F"), Some(29022));
        assert_eq!(literal_i32(&bytes, "N.S", "T", "G"), None);
        assert_eq!(literal_i32(&bytes, "N", "T", "F"), None);
        // A literal of another element type (int64) is not read as an int32.
        let wide = testutil::literal("N.S", "T", "F", 0x0a, &29022i64.to_le_bytes());
        assert_eq!(literal_i32(&wide, "N.S", "T", "F"), None);
    }

    #[test]
    fn an_assembly_without_the_type_has_no_literal() {
        let bytes = testutil::assembly(&["System.Runtime"]);
        assert_eq!(literal_i32(&bytes, "System", "Int32", "MaxValue"), None);
        assert_eq!(literal_i32(b"not a PE file", "System", "Int32", "MaxValue"), None);
    }

    /// The literal reader against the core library of the installed .NET runtime, when there is one.
    #[test]
    fn the_int32_literals_of_the_core_library_are_read() {
        let Some(dll) = ["/usr/share/dotnet", "/usr/lib/dotnet", "/usr/local/share/dotnet"]
            .iter()
            .filter_map(|d| std::fs::read_dir(format!("{d}/shared/Microsoft.NETCore.App")).ok())
            .flatten()
            .flatten()
            .map(|e| e.path().join("System.Private.CoreLib.dll"))
            .find(|p| p.is_file())
        else {
            eprintln!("SKIPPED: no .NET runtime with System.Private.CoreLib.dll in the usual folders");
            return;
        };
        let bytes = std::fs::read(&dll).unwrap();
        assert_eq!(literal_i32(&bytes, "System", "Int32", "MaxValue"), Some(i32::MAX));
        assert_eq!(literal_i32(&bytes, "System", "Int32", "MinValue"), Some(i32::MIN));
        // A literal of another element type, a field that is no literal, and a type that does not exist.
        assert_eq!(literal_i32(&bytes, "System", "Int64", "MaxValue"), None);
        assert_eq!(literal_i32(&bytes, "System", "String", "Empty"), None);
        assert_eq!(literal_i32(&bytes, "System", "NoSuchType", "MaxValue"), None);
    }

    #[test]
    fn a_module_without_an_assembly_row_has_no_version() {
        let bytes = testutil::assembly(&["System.Runtime"]);
        assert_eq!(assembly_version(&bytes), None);
        assert_eq!(assembly_refs(&bytes).unwrap(), vec!["System.Runtime".to_string()]);
    }
}
