//! A Business Central license file (`.bclicense`), parsed (ADR-060 decision 3).
//!
//! The host reads the file (the license of the BC artifact the run uses, or `--license FILE`) and passes the bytes to
//! [`parse`]; this module does no OS access (ADR-006). The parsed [`License`] is a host input of the run, held by the
//! engine and shared read-only by every session.
//!
//! The format (ADR-060, "What the license file is"): UTF-16 LE text with a byte order mark; a plain-text header
//! (licensee, product line, expiry, granted modules), then one line of XML with a `License` root. Its
//! `PermissionCollections` hold one list (`pl`, attribute `t` the object type) per object type, each with ranges (`p`:
//! first number `f`, last number `t`, permission bit mask `pbm`). The XML signature is not verified (decision 7).
//!
//! The permission mask (license facts measured on BC 28.5 with a customer license, 2026-11-06, owner decision; ADR-060
//! open question 1): the bits 1, 2, 4, 8 and 16 are Read, Insert, Modify, Delete and Execute at Yes; the bits 32, 64,
//! 128 and 256 are Read, Insert, Modify and Delete at Indirect. So letter k is Yes if bit k is set and Indirect if bit
//! k << 5 is set. Not pinned, so a level of such a range is refused where it is asked for ([`License::levels`]): the bit
//! 512 (by the pattern Execute Indirect) and every higher bit, and a letter with both its bits set. The limited-usage
//! list keeps its own meaning (32 Excluded, 64 Included) and is never read as levels. MenuSuite rows do not follow the
//! mask (the same facts), so only a MenuSuite mask of all five or none is answered.
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// A permission level of the license for one operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Blank,
    Yes,
    /// Allowed only indirectly: the code's object must grant it in its `Permissions` property (ADR-062 amendment 1).
    Indirect,
}

/// The five operations of a License Permission row, in its column order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Read,
    Insert,
    Modify,
    Delete,
    Execute,
}

/// One range of a permission list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    pub first: i64,
    pub last: i64,
    pub mask: u32,
}

/// The values of the plain-text header the run report names.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Header {
    /// `Licensed to` (its first line).
    pub licensee: Option<String>,
    /// `Expires`, as the file writes it.
    pub expires: Option<String>,
    /// `Product Line`.
    pub product_line: Option<String>,
    /// No expiry label the reader knows was found, but a line under a label it does not know holds a date: the expiry
    /// is probably there in a header language the reader does not read (al-runner #6408).
    pub expiry_label_unknown: bool,
}

/// A parsed license.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct License {
    /// Per list name (`Codeunit`, `TableData`, `TableDescription`, ...), the ranges sorted by first number.
    lists: BTreeMap<String, Vec<Range>>,
    /// The `LimitedUsageTable` list as the file holds it (the meaning of its masks is not known, decision 2).
    limited_usage: Vec<Range>,
    pub header: Header,
    /// SHA-256 of the file's bytes, lower-case hex.
    pub sha256: String,
}

/// The bits of a mask whose meaning is known (license facts, see the module text): Yes for Read, Insert, Modify,
/// Delete and Execute, and Indirect for Read, Insert, Modify and Delete, five places higher.
const DIRECT_BITS: u32 = 0b1_1111;
const INDIRECT_BITS: u32 = 0b1111 << 5;
const KNOWN_BITS: u32 = DIRECT_BITS | INDIRECT_BITS;
/// The list whose rows in License Permission do not follow the mask but for all or nothing.
const MENUSUITE_LIST: &str = "MenuSuite";

/// The list that holds the limited-usage tables.
const LIMITED_USAGE_LIST: &str = "LimitedUsageTable";

/// SHA-256 of the limited-usage list of the license that ships with the BC artifacts (the Cronus demo license; the same
/// file in the artifacts of the three supported majors), written one range per line as `first-last:mask\n`. With that
/// list, Limited Usage Permission is blank on every pinned row (metadata/platform-tables/AgLicensePermission.al,
/// permissions/permission-sets/SsLicensePermissionObjects.al:31-34, metadata/platform-tables/PlatformRowsLicensePermission.al:66-73),
/// also for table 18, which the list names (ADR-060 decision 2). Only the hash is kept here, never the list.
const ARTIFACT_LIMITED_USAGE_SHA256: &str = "76825f1c208349aa31bafb5e55fbdcb439ce0932f53cf0b8a14a3d77a11ddb14";

/// Parses the bytes of a `.bclicense` file. The error says why the file is not a license this reader understands; a run
/// stops on it before any test starts (ADR-060 decision 2).
pub fn parse(bytes: &[u8]) -> Result<License, String> {
    let text = utf16_text(bytes)?;
    let start = text
        .find("<License")
        .ok_or("no `<License>` XML element after the header")?;
    let header = parse_header(&text[..start]);
    let (lists, limited_usage) = parse_xml(&text[start..])?;
    Ok(License {
        lists,
        limited_usage,
        header,
        sha256: hex(&Sha256::digest(bytes)),
    })
}

fn utf16_text(bytes: &[u8]) -> Result<String, String> {
    let Some(body) = bytes.strip_prefix(&[0xFF, 0xFE]) else {
        return Err("not a license file: it does not start with the byte order mark of UTF-16 LE text".into());
    };
    if body.len() % 2 != 0 {
        return Err("not a license file: an odd number of bytes after the UTF-16 byte order mark".into());
    }
    let units = body.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]]));
    let text: String = char::decode_utf16(units)
        .collect::<Result<String, _>>()
        .map_err(|_| "not a license file: the text is not valid UTF-16")?;
    // The file has a second byte order mark between the header and the XML.
    Ok(text.replace('\u{FEFF}', ""))
}

/// The header labels of the expiry date the reader knows: English (the Cronus license of the BC artifacts) and Danish
/// (al-runner #6408, a partner license issued in Danish). The header is only reported, never used for a decision.
const EXPIRY_LABELS: &[&str] = &["Expires", "Udløber"];

fn parse_header(text: &str) -> Header {
    let mut h = Header::default();
    let mut unknown_date = false;
    for line in text.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim();
        let key = key.trim();
        let slot = match key {
            "Licensed to" => &mut h.licensee,
            "Product Line" => &mut h.product_line,
            k if EXPIRY_LABELS.contains(&k) => &mut h.expires,
            _ => {
                unknown_date |= !key.is_empty() && looks_like_date(value);
                continue;
            }
        };
        if slot.is_none() && !value.is_empty() {
            *slot = Some(value.to_string());
        }
    }
    h.expiry_label_unknown = h.expires.is_none() && unknown_date;
    h
}

/// Whether a header value starts with a date: three groups of digits split by `/`, `-` or `.` (`3/13/2033`,
/// `13-03-2033`, `13.03.2033`).
fn looks_like_date(value: &str) -> bool {
    let first = value.split_whitespace().next().unwrap_or("");
    let parts: Vec<&str> = first.split(['/', '-', '.']).collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
}

type Lists = (BTreeMap<String, Vec<Range>>, Vec<Range>);

fn parse_xml(xml: &str) -> Result<Lists, String> {
    use quick_xml::events::Event;
    let mut reader = quick_xml::Reader::from_str(xml);
    let mut lists: BTreeMap<String, Vec<Range>> = BTreeMap::new();
    let mut in_collections = false;
    let mut list: Option<String> = None;
    let mut seen_collections = false;
    loop {
        let event = reader
            .read_event()
            .map_err(|e| format!("the license XML cannot be read: {e}"))?;
        match event {
            Event::Start(e) | Event::Empty(e) if in_collections => match e.name().as_ref() {
                b"pl" => {
                    let name = attr(&e, b"t")?.ok_or("a permission list without an object type (`t`)")?;
                    if lists.contains_key(&name) {
                        return Err(format!("the permission list `{name}` appears twice"));
                    }
                    lists.insert(name.clone(), Vec::new());
                    list = Some(name);
                }
                b"p" => {
                    let Some(name) = &list else {
                        return Err("a permission range outside a permission list".into());
                    };
                    let num = |key: &[u8], what: &str| -> Result<i64, String> {
                        let v = attr(&e, key)?.ok_or_else(|| format!("a range of `{name}` without its {what}"))?;
                        v.trim()
                            .parse::<i64>()
                            .map_err(|_| format!("a range of `{name}` has the {what} `{v}`, which is no number"))
                    };
                    let range = Range {
                        first: num(b"f", "first number")?,
                        last: num(b"t", "last number")?,
                        mask: u32::try_from(num(b"pbm", "permission mask")?)
                            .map_err(|_| format!("a range of `{name}` has a negative permission mask"))?,
                    };
                    if range.first > range.last {
                        return Err(format!(
                            "a range of `{name}` ends ({}) before it starts ({})",
                            range.last, range.first
                        ));
                    }
                    lists.get_mut(name).expect("the list was inserted").push(range);
                }
                _ => {}
            },
            Event::Start(e) if e.name().as_ref() == b"PermissionCollections" => {
                in_collections = true;
                seen_collections = true;
            }
            Event::End(e) if e.name().as_ref() == b"PermissionCollections" => in_collections = false,
            Event::End(e) if e.name().as_ref() == b"pl" => list = None,
            Event::Eof => break,
            _ => {}
        }
    }
    if !seen_collections {
        return Err("the license XML has no `PermissionCollections`".into());
    }
    let limited_usage = lists.remove(LIMITED_USAGE_LIST).unwrap_or_default();
    for (name, ranges) in &mut lists {
        ranges.sort_by_key(|r| r.first);
        if let Some(w) = ranges.windows(2).find(|w| w[1].first <= w[0].last) {
            return Err(format!(
                "the ranges {}..{} and {}..{} of `{name}` overlap",
                w[0].first, w[0].last, w[1].first, w[1].last
            ));
        }
    }
    Ok((lists, limited_usage))
}

/// The five levels of `mask` (of `number` in `list`, for the message).
fn levels_of_mask(list: &str, number: i64, mask: u32) -> Result<[Level; 5], String> {
    let unknown = mask & !KNOWN_BITS;
    if unknown != 0 {
        return Err(format!(
            "the license gives `{list}` {number} the permission mask {mask}, with the bits {unknown} whose meaning is not pinned (512 is probably Execute Indirect; the license facts of ADR-060 pin 1 to 256 only)"
        ));
    }
    if list == MENUSUITE_LIST && mask != 0 && mask != DIRECT_BITS {
        return Err(format!(
            "the license gives `{list}` {number} the permission mask {mask}: MenuSuite rows do not follow the mask but for all or nothing"
        ));
    }
    const NAMES: [&str; 5] = ["Read", "Insert", "Modify", "Delete", "Execute"];
    let mut levels = [Level::Blank; 5];
    for (k, level) in levels.iter_mut().enumerate() {
        let yes = mask & (1 << k) != 0;
        let indirect = k < 4 && mask & (1 << (k + 5)) != 0;
        *level = match (yes, indirect) {
            (true, true) => {
                return Err(format!(
                    "the license gives `{list}` {number} the permission mask {mask}, which sets both the direct and the indirect bit of {}: which one wins is not pinned",
                    NAMES[k]
                ));
            }
            (true, false) => Level::Yes,
            (false, true) => Level::Indirect,
            (false, false) => Level::Blank,
        };
    }
    Ok(levels)
}

fn attr(e: &quick_xml::events::BytesStart, key: &[u8]) -> Result<Option<String>, String> {
    for a in e.attributes() {
        let a = a.map_err(|err| format!("the license XML cannot be read: {err}"))?;
        if a.key.as_ref() == key {
            let v = a
                .unescape_value()
                .map_err(|err| format!("the license XML cannot be read: {err}"))?;
            return Ok(Some(v.into_owned()));
        }
    }
    Ok(None)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

impl License {
    /// The names of the permission lists the file holds (without the limited-usage list).
    pub fn list_names(&self) -> impl Iterator<Item = &str> {
        self.lists.keys().map(String::as_str)
    }

    /// The ranges of list `list`, sorted; `None` when the file has no such list.
    pub fn ranges(&self, list: &str) -> Option<&[Range]> {
        self.lists.get(list).map(Vec::as_slice)
    }

    /// The mask of `number` in list `list`: `None` when the file has no such list, `Some(None)` for a number outside
    /// every range of it.
    pub fn mask(&self, list: &str, number: i64) -> Option<Option<u32>> {
        let ranges = self.lists.get(list)?;
        let i = ranges.partition_point(|r| r.first <= number);
        Some(
            i.checked_sub(1)
                .map(|i| ranges[i])
                .filter(|r| number <= r.last)
                .map(|r| r.mask),
        )
    }

    /// The level of Read, Insert, Modify, Delete and Execute for `number` in list `list`: every level blank for a
    /// number outside every range. `Ok(None)` when the file has no such list. An error for a mask whose meaning is not
    /// pinned (see the module text): the bit 512 or a higher one, a letter with both its direct and its indirect bit, a
    /// MenuSuite mask other than all or nothing.
    pub fn levels(&self, list: &str, number: i64) -> Result<Option<[Level; 5]>, String> {
        let Some(mask) = self.mask(list, number) else {
            return Ok(None);
        };
        levels_of_mask(list, number, mask.unwrap_or(0)).map(Some)
    }

    /// The level of one operation (for the permission check of the enforcement phase, ADR-060 decision 6). Same
    /// answers and errors as [`License::levels`].
    pub fn level(&self, list: &str, number: i64, op: Op) -> Result<Option<Level>, String> {
        Ok(self.levels(list, number)?.map(|l| l[op as usize]))
    }

    /// Whether the limited-usage list is the one of the license that ships with the BC artifacts, for which
    /// Limited Usage Permission is pinned blank (ADR-060 decision 2).
    pub fn limited_usage_is_artifact_default(&self) -> bool {
        let text: String = self
            .limited_usage
            .iter()
            .map(|r| format!("{}-{}:{}\n", r.first, r.last, r.mask))
            .collect();
        hex(&Sha256::digest(text.as_bytes())) == ARTIFACT_LIMITED_USAGE_SHA256
    }

    /// The ranges of the limited-usage list, in file order.
    pub fn limited_usage(&self) -> &[Range] {
        &self.limited_usage
    }
}

/// Test helpers: a synthetic license file (made-up licensee and ranges, no signature), as UTF-16 LE bytes. No real
/// license file is ever used as a fixture (ADR-060, Conformance).
pub mod synthetic {
    /// A permission list: its name and its ranges `(first, last, mask)`.
    pub type List<'a> = (&'a str, &'a [(i64, i64, u32)]);

    /// A license file with the header values given and one permission list per `(list, ranges)` entry, each range
    /// `(first, last, mask)`.
    pub fn file(licensee: &str, expires: &str, lists: &[List]) -> Vec<u8> {
        file_with_header(
            &format!(
                "Licensed to             : {licensee}\r\n                          Somewhere\r\n\
                 Product Line            : Synthetic test product\r\n\
                 Expires                 : {expires}\r\n"
            ),
            lists,
        )
    }

    /// A license file whose header lines (between the two star lines) are `header`, as written.
    pub fn file_with_header(header: &str, lists: &[List]) -> Vec<u8> {
        let mut text = String::from("Synthetic License Information\r\n****\r\n");
        text.push_str(header);
        text.push_str("****\r\n\r\n\u{FEFF}");
        text.push_str("<License><Properties><Property name=\"CC\" type=\"String\" id=\"1\"><![CDATA[XX]]></Property></Properties><PermissionCollections>");
        for (name, ranges) in lists {
            text.push_str(&format!("<pl t=\"{name}\" c=\"{}\"><ps>", ranges.len()));
            for (f, t, m) in *ranges {
                text.push_str(&format!("<p f=\"{f}\" t=\"{t}\" pbm=\"{m}\" />"));
            }
            text.push_str("</ps></pl>");
        }
        text.push_str("</PermissionCollections></License>");
        let mut bytes = vec![0xFF, 0xFE];
        for u in text.encode_utf16() {
            bytes.extend_from_slice(&u.to_le_bytes());
        }
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<u8> {
        synthetic::file(
            "Test Licensee",
            "1/2/2040",
            &[
                (
                    "Codeunit",
                    &[(1, 999, 31), (5000, 5999, 31), (2000000000, 2000000100, 31)],
                ),
                ("TableData", &[(1, 999, 31)]),
                ("TableDescription", &[(1, 999, 31)]),
                ("Page", &[(1, 999, 31)]),
                ("Report", &[(1, 999, 31)]),
                ("XMLPort", &[(1, 999, 31)]),
                ("Query", &[(1, 999, 31)]),
                ("MenuSuite", &[(1, 999, 31)]),
                ("FieldNo", &[(1, 999, 31)]),
                ("System", &[(1, 99, 31), (500, 599, 16)]),
                ("LimitedUsageTable", &[(17, 17, 32), (18, 19, 64)]),
            ],
        )
    }

    #[test]
    fn the_header_values_and_the_hash_are_read() {
        let bytes = sample();
        let l = parse(&bytes).unwrap();
        assert_eq!(l.header.licensee.as_deref(), Some("Test Licensee"));
        assert_eq!(l.header.expires.as_deref(), Some("1/2/2040"));
        assert_eq!(l.header.product_line.as_deref(), Some("Synthetic test product"));
        assert_eq!(l.sha256, hex(&Sha256::digest(&bytes)));
        assert_eq!(l.sha256.len(), 64);
    }

    /// A header in another language (al-runner #6408: a Danish license writes `Udløber` for `Expires` and the date as
    /// `dd-MM-yyyy`). Synthetic values only.
    #[test]
    fn a_danish_expiry_label_is_read() {
        let header = "Licensed to              : Test Licensee\r\n\
                      Sprog                    : Danish\r\n\
                      Oprettet Dato            : 01-02-2030 10:11:12 (-07:00)\r\n\
                      Udløber                  : 03-04-2040\r\n\
                      Konfiguration            : XX\r\n";
        let l = parse(&synthetic::file_with_header(header, &[("Codeunit", &[(1, 9, 31)])])).unwrap();
        assert_eq!(l.header.licensee.as_deref(), Some("Test Licensee"));
        assert_eq!(l.header.expires.as_deref(), Some("03-04-2040"));
        assert!(!l.header.expiry_label_unknown);
    }

    /// A header whose expiry label the reader does not know: no value is guessed, the header says the expiry was not
    /// read, as long as some line under an unknown label holds a date.
    #[test]
    fn an_unknown_expiry_label_is_reported_as_not_read() {
        let header = "Licensed to              : Test Licensee\r\n\
                      Synthlabel A             : 01.02.2030 10:11:12\r\n\
                      Synthlabel B             : 03.04.2040\r\n";
        let l = parse(&synthetic::file_with_header(header, &[("Codeunit", &[(1, 9, 31)])])).unwrap();
        assert_eq!(l.header.expires, None);
        assert!(l.header.expiry_label_unknown);

        let header = "Licensed to              : Test Licensee\r\nProduct Line            : X\r\n";
        let l = parse(&synthetic::file_with_header(header, &[("Codeunit", &[(1, 9, 31)])])).unwrap();
        assert_eq!(l.header.expires, None);
        assert!(!l.header.expiry_label_unknown);
    }

    #[test]
    fn every_object_type_list_is_read_and_ranges_and_gaps_answer() {
        let l = parse(&sample()).unwrap();
        let names: Vec<&str> = l.list_names().collect();
        assert_eq!(
            names,
            [
                "Codeunit",
                "FieldNo",
                "MenuSuite",
                "Page",
                "Query",
                "Report",
                "System",
                "TableData",
                "TableDescription",
                "XMLPort"
            ]
        );
        const YES: [Level; 5] = [Level::Yes; 5];
        const BLANK: [Level; 5] = [Level::Blank; 5];
        for name in names {
            assert_eq!(l.levels(name, 1).unwrap(), Some(YES), "{name} 1");
            assert_eq!(l.levels(name, 0).unwrap(), Some(BLANK), "{name} 0 is in a gap");
        }
        assert_eq!(l.levels("Codeunit", 999).unwrap(), Some(YES));
        assert_eq!(l.levels("Codeunit", 1000).unwrap(), Some(BLANK));
        assert_eq!(l.levels("Codeunit", 4999).unwrap(), Some(BLANK));
        assert_eq!(l.levels("Codeunit", 5000).unwrap(), Some(YES));
        assert_eq!(l.levels("Codeunit", 2000000100).unwrap(), Some(YES));
        assert_eq!(l.levels("Codeunit", 2000000101).unwrap(), Some(BLANK));
        assert_eq!(l.levels("Codeunit", -1).unwrap(), Some(BLANK));
        assert_eq!(l.levels("Codeunit", i64::from(i32::MAX)).unwrap(), Some(BLANK));
        // The Execute bit alone.
        assert_eq!(
            l.levels("System", 550).unwrap(),
            Some([Level::Blank, Level::Blank, Level::Blank, Level::Blank, Level::Yes])
        );
        assert_eq!(l.level("System", 550, Op::Execute).unwrap(), Some(Level::Yes));
        assert_eq!(l.level("System", 550, Op::Read).unwrap(), Some(Level::Blank));
        assert_eq!(l.levels("Table", 1).unwrap(), None, "no list named Table");
        assert_eq!(l.mask("Codeunit", 1000), Some(None));
        assert_eq!(l.limited_usage().len(), 2);
        assert!(!l.limited_usage_is_artifact_default());
    }

    /// The masks the license facts give (measured on BC 28.5 with a customer license, 2026-11-06, owner decision): the
    /// levels BC shows in License Permission for each mask. Synthetic ranges only.
    #[test]
    fn direct_and_indirect_bits_are_read_per_letter() {
        use Level::{Blank as B, Indirect as N, Yes as Y};
        let cases: &[(u32, [Level; 5])] = &[
            (7, [Y, Y, Y, B, B]),
            (15, [Y, Y, Y, Y, B]),
            (31, [Y, Y, Y, Y, Y]),
            (193, [Y, N, N, B, B]),
            (201, [Y, N, N, Y, B]),
            (325, [Y, N, Y, N, B]),
            (387, [Y, Y, N, N, B]),
            (449, [Y, N, N, N, B]),
            (480, [N, N, N, N, B]),
            (8, [B, B, B, Y, B]),
            (12, [B, B, Y, Y, B]),
            (26, [B, Y, B, Y, Y]),
            (29, [Y, B, Y, Y, Y]),
            (4, [B, B, Y, B, B]),
            (14, [B, Y, Y, Y, B]),
            (16, [B, B, B, B, Y]),
            (93, [Y, N, Y, Y, Y]),
        ];
        let ranges: Vec<(i64, i64, u32)> = cases
            .iter()
            .enumerate()
            .map(|(i, (m, _))| (i as i64 * 10 + 1, i as i64 * 10 + 9, *m))
            .collect();
        let l = parse(&synthetic::file(
            "X",
            "1/1/2040",
            &[("TableData", &ranges), ("Codeunit", &ranges)],
        ))
        .unwrap();
        for (i, (mask, want)) in cases.iter().enumerate() {
            let n = i as i64 * 10 + 5;
            assert_eq!(l.levels("TableData", n).unwrap(), Some(*want), "TableData mask {mask}");
            assert_eq!(l.levels("Codeunit", n).unwrap(), Some(*want), "Codeunit mask {mask}");
        }
        assert_eq!(
            l.level("TableData", 5 + 10 * 8, Op::Read).unwrap(),
            Some(Level::Indirect)
        );
    }

    /// Bit 512 (by the pattern Execute Indirect) and every higher bit are not pinned, and neither is a letter with both
    /// its direct and its indirect bit: the reader reads the file, and a level of such a range is refused with the
    /// reason, not guessed.
    #[test]
    fn unpinned_bits_are_refused_where_they_are_used() {
        let l = parse(&synthetic::file(
            "X",
            "1/1/2040",
            &[(
                "TableData",
                &[(1, 10, 512 | 31), (11, 20, 1 | 32), (21, 30, 1024 | 1), (31, 40, 31)],
            )],
        ))
        .unwrap();
        let e = l.levels("TableData", 5).unwrap_err();
        assert!(
            e.contains("mask 543") && e.contains("512") && e.contains("not pinned"),
            "{e}"
        );
        let e = l.levels("TableData", 15).unwrap_err();
        assert!(e.contains("both") && e.contains("Read"), "{e}");
        assert!(l.levels("TableData", 25).unwrap_err().contains("1024"));
        assert_eq!(l.levels("TableData", 35).unwrap(), Some([Level::Yes; 5]));
        // The limited-usage list keeps its own meaning (32 Excluded, 64 Included): its bits are not Indirect.
        let l = parse(&synthetic::file(
            "X",
            "1/1/2040",
            &[("LimitedUsageTable", &[(1, 1, 64), (2, 2, 32)])],
        ))
        .unwrap();
        assert_eq!(l.limited_usage().len(), 2);
        assert_eq!(l.levels("LimitedUsageTable", 1).unwrap(), None);
    }

    /// MenuSuite rows in License Permission do not follow the mask (the license facts: Read shown for mask 24, Modify
    /// dropped for mask 29), and MenuSuite has no AL object type: only all or nothing is answered.
    #[test]
    fn a_partial_menusuite_mask_is_refused() {
        let l = parse(&synthetic::file(
            "X",
            "1/1/2040",
            &[("MenuSuite", &[(1, 10, 31), (11, 20, 24), (21, 30, 0)])],
        ))
        .unwrap();
        assert_eq!(l.levels("MenuSuite", 5).unwrap(), Some([Level::Yes; 5]));
        assert_eq!(l.levels("MenuSuite", 25).unwrap(), Some([Level::Blank; 5]));
        assert!(l.levels("MenuSuite", 15).unwrap_err().contains("MenuSuite"));
    }

    #[test]
    fn a_file_that_is_no_license_is_refused_with_the_reason() {
        assert!(parse(b"<License/>").unwrap_err().contains("byte order mark"));
        let mut odd = vec![0xFF, 0xFE];
        odd.extend_from_slice(&[b'a', 0, b'b']);
        assert!(parse(&odd).unwrap_err().contains("odd number"));
        let no_xml = synthetic::file("X", "1/1/2040", &[])[..60].to_vec();
        assert!(parse(&no_xml).unwrap_err().contains("<License>"));
        let mut bad = vec![0xFF, 0xFE];
        for u in "header\r\n<License><Properties/></License>".encode_utf16() {
            bad.extend_from_slice(&u.to_le_bytes());
        }
        assert!(parse(&bad).unwrap_err().contains("PermissionCollections"));
        let reversed = synthetic::file("X", "1/1/2040", &[("Page", &[(10, 1, 31)])]);
        assert!(parse(&reversed).unwrap_err().contains("ends"));
        let overlap = synthetic::file("X", "1/1/2040", &[("Page", &[(1, 10, 31), (5, 20, 31)])]);
        assert!(parse(&overlap).unwrap_err().contains("overlap"));
        let text = synthetic::file("X", "1/1/2040", &[("Page", &[(1, 10, 31)])]);
        let s: String = char::decode_utf16(text[2..].chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])))
            .map(|c| c.unwrap())
            .collect::<String>()
            .replace("pbm=\"31\"", "pbm=\"x\"");
        let mut b = vec![0xFF, 0xFE];
        for u in s.encode_utf16() {
            b.extend_from_slice(&u.to_le_bytes());
        }
        assert!(parse(&b).unwrap_err().contains("no number"));
    }

    #[test]
    fn ranges_are_sorted_whatever_the_file_order() {
        let l = parse(&synthetic::file(
            "X",
            "1/1/2040",
            &[("Page", &[(100, 200, 31), (1, 10, 16)])],
        ))
        .unwrap();
        assert_eq!(l.ranges("Page").unwrap()[0].first, 1);
        assert_eq!(l.mask("Page", 150), Some(Some(31)));
        assert_eq!(l.mask("Page", 5), Some(Some(16)));
    }
}
