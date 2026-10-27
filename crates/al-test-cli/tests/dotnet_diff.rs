//! ADR-038: the in-process stand-ins against the sidecar. Every member of `inproc.txt` runs through the stand-in and
//! through the bare sidecar on generated inputs, and what comes back (value, handle type, or the .NET exception type and
//! message) must be equal. Needs a .NET 8 runtime and SDK; without them the test skips with a message, and with
//! `ALRT_REQUIRE_DOTNET=1` (CI) a missing .NET fails.

use al_host::dotnet::*;
use al_host::inproc::{InprocHost, MEMBERS};
use al_test_cli::dotnet::{SidecarConfig, SidecarHost};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn sidecar() -> Option<Arc<SidecarHost>> {
    let h = Arc::new(SidecarHost::new(SidecarConfig {
        source_root: Some(root()),
        cache_dir: Some(Path::new(env!("CARGO_TARGET_TMPDIR")).join("dotnet-sidecar-cache")),
        ..Default::default()
    }));
    match h.availability() {
        Ok(()) => Some(h),
        Err(why) => {
            assert!(
                std::env::var("ALRT_REQUIRE_DOTNET").is_err(),
                "ALRT_REQUIRE_DOTNET is set but .NET is unavailable: {why}"
            );
            eprintln!("SKIPPED (no usable .NET): DotNet {why}");
            None
        }
    }
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n.max(1)
    }
    fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.next() as u8).collect()
    }
    /// Text with ASCII, accents, non-BMP characters, NUL and a few awkward characters.
    fn text(&mut self, n: usize) -> String {
        const POOL: &[char] = &[
            'a',
            'Z',
            '0',
            ' ',
            '\0',
            '\n',
            'é',
            'ß',
            '€',
            '漢',
            '😀',
            '\u{10FFFF}',
            '\u{FFFD}',
            '~',
        ];
        (0..n).map(|_| POOL[self.below(POOL.len() as u64) as usize]).collect()
    }
}

fn ty(name: &str) -> DotNetType {
    DotNetType {
        assembly: "netstandard".into(),
        culture: Some("neutral".into()),
        public_key_token: Some("cc7b13ffcd2ddd51".into()),
        name: name.into(),
        ..Default::default()
    }
}

fn st(name: &str) -> DotNetTarget {
    DotNetTarget::Static(ty(name))
}

fn a(v: DotNetValue) -> DotNetArg {
    DotNetArg {
        value: v,
        by_ref: false,
    }
}

fn s(v: &str) -> DotNetArg {
    a(DotNetValue::Str(v.into()))
}

fn i(v: i32) -> DotNetArg {
    a(DotNetValue::Int32(v))
}

fn h(handle: u64, ty: &str) -> DotNetArg {
    a(DotNetValue::Handle { handle, ty: ty.into() })
}

/// One session with the observations its calls made.
struct Run {
    session: Box<dyn DotNetSession>,
    seen: Vec<String>,
    covered: BTreeSet<String>,
}

impl Run {
    fn show(r: &Result<DotNetReply, DotNetFailure>) -> String {
        match r {
            Ok(r) => match &r.value {
                DotNetValue::Handle { ty, .. } => format!("handle<{ty}>"),
                other => format!("{other:?}"),
            },
            Err(DotNetFailure::Exception(e)) => format!("EXC {}: {} {:?}", e.clr_type, e.message, e.inner),
            Err(DotNetFailure::Refused(why)) => format!("REFUSED {why}"),
        }
    }

    fn note(&mut self, what: &str, r: &Result<DotNetReply, DotNetFailure>) -> Option<u64> {
        self.seen.push(format!("{what} => {}", Self::show(r)));
        match r {
            Ok(DotNetReply {
                value: DotNetValue::Handle { handle, .. },
                ..
            }) => Some(*handle),
            _ => None,
        }
    }

    fn cover(&mut self, member: &str) {
        self.covered.insert(member.to_string());
    }

    fn invoke(
        &mut self,
        cover: &str,
        target: DotNetTarget,
        member: &str,
        args: Vec<DotNetArg>,
    ) -> Option<(u64, String)> {
        self.cover(cover);
        let r = self.session.invoke(target, member, args);
        let ty = match &r {
            Ok(DotNetReply {
                value: DotNetValue::Handle { ty, .. },
                ..
            }) => ty.clone(),
            _ => String::new(),
        };
        self.note(&format!("{cover}.{member}"), &r).map(|h| (h, ty))
    }

    fn get(&mut self, cover: &str, target: DotNetTarget, name: &str) -> Option<(u64, String)> {
        self.cover(cover);
        let r = self.session.get_property(target, name);
        let ty = match &r {
            Ok(DotNetReply {
                value: DotNetValue::Handle { ty, .. },
                ..
            }) => ty.clone(),
            _ => String::new(),
        };
        self.note(&format!("get {cover}.{name}"), &r).map(|h| (h, ty))
    }

    fn construct(&mut self, cover: &str, ty_name: &str, args: Vec<DotNetArg>) -> Option<(u64, String)> {
        self.cover(cover);
        let r = self.session.construct(&ty(ty_name), args);
        let t = match &r {
            Ok(DotNetReply {
                value: DotNetValue::Handle { ty, .. },
                ..
            }) => ty.clone(),
            _ => String::new(),
        };
        self.note(&format!("new {ty_name}"), &r).map(|h| (h, t))
    }

    fn inst(&mut self, cover: &str, o: &(u64, String), member: &str, args: Vec<DotNetArg>) -> Option<(u64, String)> {
        self.invoke(cover, DotNetTarget::Instance(o.0), member, args)
    }

    /// The bytes of a byte[] handle, read through the members of the set (so a wrong stand-in shows).
    fn read_bytes(&mut self, o: &(u64, String)) {
        let r = self.session.invoke(DotNetTarget::Instance(o.0), "Length", vec![]);
        self.cover("System.Array.Length");
        self.seen.push(format!("Length => {}", Self::show(&r)));
        let n = match r {
            Ok(DotNetReply {
                value: DotNetValue::Int32(n),
                ..
            }) => n,
            _ => return,
        };
        let mut hex = String::new();
        for k in 0..n.min(16) {
            let r = self.session.invoke(DotNetTarget::Instance(o.0), "GetValue", vec![i(k)]);
            self.cover("System.Array.GetValue");
            if let Ok(DotNetReply {
                value: DotNetValue::Byte(b),
                ..
            }) = &r
            {
                let t = self
                    .session
                    .invoke(DotNetTarget::Value(DotNetValue::Byte(*b)), "ToString", vec![s("X2")]);
                self.cover("System.Byte.ToString");
                if let Ok(DotNetReply {
                    value: DotNetValue::Str(x),
                    ..
                }) = t
                {
                    hex.push_str(&x);
                }
            } else {
                hex.push_str(&Self::show(&r));
            }
        }
        // The whole array, however long, through Base64 (a static member of the set).
        let b64 = self
            .session
            .invoke(st("System.Convert"), "ToBase64String", vec![h(o.0, &o.1)]);
        self.cover("System.Convert.ToBase64String");
        self.seen.push(format!("hex[..16]={hex} b64={}", Self::show(&b64)));
        if let Ok(DotNetReply {
            value: DotNetValue::Str(t),
            ..
        }) = b64
        {
            self.seen.push(format!("b64 text {t}"));
        }
    }
}

/// One session on the stand-ins and one on the bare sidecar, kept for the whole test (a session costs a process start).
struct Pair {
    bare: Option<Box<dyn DotNetSession>>,
    mine: Option<Box<dyn DotNetSession>>,
}

/// Runs `script` on both sessions and compares what each saw.
fn both(pair: &mut Pair, script: &dyn Fn(&mut Run)) -> BTreeSet<String> {
    let mut bare = Run {
        session: pair.bare.take().unwrap(),
        seen: vec![],
        covered: BTreeSet::new(),
    };
    script(&mut bare);
    let mut mine = Run {
        session: pair.mine.take().unwrap(),
        seen: vec![],
        covered: BTreeSet::new(),
    };
    script(&mut mine);
    let covered = mine.covered.clone();
    let (x, y) = (bare.seen.clone(), mine.seen.clone());
    pair.bare = Some(bare.session);
    pair.mine = Some(mine.session);
    assert_eq!(
        x.len(),
        y.len(),
        "number of observations\nsidecar: {x:?}\nin proc: {y:?}"
    );
    for (k, (x, y)) in x.iter().zip(&y).enumerate() {
        assert_eq!(x, y, "observation {k}\nsidecar : {x}\nin proc : {y}");
    }
    covered
}

#[test]
fn the_stand_ins_answer_what_the_sidecar_answers() {
    let Some(host) = sidecar() else { return };
    let bare_host: Arc<dyn DotNetHost> = host.clone();
    let mut pair = Pair {
        bare: Some(bare_host.open_session(&DotNetSessionConfig::default())),
        mine: Some(InprocHost::new(host.clone()).open_session(&DotNetSessionConfig::default())),
    };
    let mut covered: BTreeSet<String> = BTreeSet::new();
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);

    // ---- hashes: sizes around every block boundary, then random ones ----
    let mut sizes: Vec<usize> = vec![
        0, 1, 2, 3, 54, 55, 56, 57, 63, 64, 65, 111, 112, 119, 120, 127, 128, 129, 255, 1000,
    ];
    sizes.extend((0..12).map(|_| rng.below(70_000) as usize));
    for algo in ["MD5", "SHA1", "SHA256", "SHA384", "SHA512"] {
        for &n in &sizes {
            let data = rng.bytes(n);
            let b64 = base64(&data);
            covered.extend(both(&mut pair, &|r| {
                let b = r.invoke(
                    "System.Convert.FromBase64String",
                    st("System.Convert"),
                    "FromBase64String",
                    vec![s(&b64)],
                );
                let Some(b) = b else { return };
                let ha = r.invoke(
                    "System.Security.Cryptography.HashAlgorithm.Create",
                    st("System.Security.Cryptography.HashAlgorithm"),
                    "Create",
                    vec![s(algo)],
                );
                let Some(ha) = ha else { return };
                let d = r.inst(
                    "System.Security.Cryptography.HashAlgorithm.ComputeHash",
                    &ha,
                    "ComputeHash",
                    vec![h(b.0, &b.1)],
                );
                if let Some(d) = d {
                    r.read_bytes(&d);
                }
                let _ = r.inst(
                    "System.Security.Cryptography.HashAlgorithm.Dispose",
                    &ha,
                    "Dispose",
                    vec![],
                );
                // After Dispose: .NET's own error.
                let _ = r.inst(
                    "System.Security.Cryptography.HashAlgorithm.ComputeHash",
                    &ha,
                    "ComputeHash",
                    vec![h(b.0, &b.1)],
                );
            }));
        }
    }
    // Names the stand-in does not take, and no argument.
    for name in ["sha256", "SHA-256", "System.Security.Cryptography.SHA256", "", "NoSuch"] {
        covered.extend(both(&mut pair, &|r| {
            let x = r.invoke(
                "System.Security.Cryptography.HashAlgorithm.Create",
                st("System.Security.Cryptography.HashAlgorithm"),
                "Create",
                vec![s(name)],
            );
            if let Some(x) = x {
                let _ = r.inst(
                    "System.Security.Cryptography.HashAlgorithm.Dispose",
                    &x,
                    "Dispose",
                    vec![],
                );
            }
        }));
    }

    // ---- encodings ----
    for enc in ["UTF8", "ASCII", "Unicode"] {
        for round in 0..200 {
            let n = rng.below(60) as usize;
            let text = rng.text(n);
            let n = rng.below(40) as usize;
            let raw = rng.bytes(n);
            let raw64 = base64(&raw);
            covered.extend(both(&mut pair, &|r| {
                let label = format!("System.Text.Encoding.{enc}");
                let e = r.invoke(&label, st("System.Text.Encoding"), enc, vec![]);
                let Some(e) = e else { return };
                if let Some(b) = r.inst("System.Text.Encoding.GetBytes", &e, "GetBytes", vec![s(&text)]) {
                    r.read_bytes(&b);
                    let _ = r.inst("System.Text.Encoding.GetString", &e, "GetString", vec![h(b.0, &b.1)]);
                }
                let _ = r.inst("System.Text.Encoding.GetByteCount", &e, "GetByteCount", vec![s(&text)]);
                // Bytes nobody encoded: invalid sequences are the codec's to repair.
                if let Some(b) = r.invoke(
                    "System.Convert.FromBase64String",
                    st("System.Convert"),
                    "FromBase64String",
                    vec![s(&raw64)],
                ) {
                    let _ = r.inst("System.Text.Encoding.GetString", &e, "GetString", vec![h(b.0, &b.1)]);
                }
                let _ = round;
            }));
        }
        // The static property written as a get.
        covered.extend(both(&mut pair, &|r| {
            let label = format!("System.Text.Encoding.{enc}");
            let _ = r.get(&label, st("System.Text.Encoding"), enc);
        }));
    }

    // ---- Base64 ----
    let texts: Vec<String> = vec![
        "".into(),
        "AA==".into(),
        "AAA=".into(),
        "AAAA".into(),
        "AA".into(),
        "A".into(),
        "AAA".into(),
        "AA=A".into(),
        "====".into(),
        "AA==AA==".into(),
        " AAAA".into(),
        "AA AA".into(),
        "AAAA\n".into(),
        "AAAA\r\n".into(),
        "AAA*".into(),
        "-_-_".into(),
        "QUJD".into(),
        "QUJDRA==".into(),
        "QUJDRA=".into(),
        "ÅAAA".into(),
        "AAAA====".into(),
        "A===".into(),
        "/+/+".into(),
        "AB==".into(),
        "ABC=".into(),
    ];
    for t in &texts {
        covered.extend(both(&mut pair, &|r| {
            if let Some(b) = r.invoke(
                "System.Convert.FromBase64String",
                st("System.Convert"),
                "FromBase64String",
                vec![s(t)],
            ) {
                r.read_bytes(&b);
            }
        }));
    }
    for _ in 0..100 {
        let n = rng.below(200) as usize;
        let data = rng.bytes(n);
        let b64 = base64(&data);
        covered.extend(both(&mut pair, &|r| {
            if let Some(b) = r.invoke(
                "System.Convert.FromBase64String",
                st("System.Convert"),
                "FromBase64String",
                vec![s(&b64)],
            ) {
                r.read_bytes(&b);
            }
        }));
    }

    // ---- StringBuilder ----
    for round in 0..500u64 {
        let mut local = Rng(0x1234_5678 + round * 7919);
        let ops: Vec<(u64, String, i32, i32)> = (0..12)
            .map(|_| {
                let op = local.below(9);
                let n = local.below(6) as usize;
                let text = local.text(n);
                (op, text, local.below(12) as i32 - 3, local.below(12) as i32 - 3)
            })
            .collect();
        let start = local.below(3);
        let n = local.below(5) as usize;
        let initial = local.text(n);
        covered.extend(both(&mut pair, &|r| {
            let sb = match start {
                0 => r.construct("System.Text.StringBuilder..ctor", "System.Text.StringBuilder", vec![]),
                1 => r.construct(
                    "System.Text.StringBuilder..ctor",
                    "System.Text.StringBuilder",
                    vec![s(&initial)],
                ),
                _ => r.construct(
                    "System.Text.StringBuilder..ctor",
                    "System.Text.StringBuilder",
                    vec![i(16)],
                ),
            };
            let Some(sb) = sb else { return };
            for (op, text, x, y) in &ops {
                match op {
                    0 => {
                        let _ = r.inst("System.Text.StringBuilder.Append", &sb, "Append", vec![s(text)]);
                    }
                    1 => {
                        // A char: a unit of text, including half a surrogate pair.
                        let unit = text.encode_utf16().next().unwrap_or(0x41);
                        let _ = r.inst(
                            "System.Text.StringBuilder.Append",
                            &sb,
                            "Append",
                            vec![a(DotNetValue::Char(unit))],
                        );
                    }
                    2 => {
                        let _ = r.inst("System.Text.StringBuilder.AppendLine", &sb, "AppendLine", vec![]);
                    }
                    3 => {
                        let _ = r.inst("System.Text.StringBuilder.AppendLine", &sb, "AppendLine", vec![s(text)]);
                    }
                    4 => {
                        let _ = r.inst("System.Text.StringBuilder.Clear", &sb, "Clear", vec![]);
                    }
                    5 => {
                        let _ = r.inst("System.Text.StringBuilder.Length", &sb, "Length", vec![]);
                        let _ = r.get(
                            "System.Text.StringBuilder.Length",
                            DotNetTarget::Instance(sb.0),
                            "Length",
                        );
                    }
                    6 => {
                        let _ = r.inst("System.Text.StringBuilder.ToString", &sb, "ToString", vec![]);
                    }
                    _ => {
                        let _ = r.inst(
                            "System.Text.StringBuilder.ToString",
                            &sb,
                            "ToString",
                            vec![i(*x), i(*y)],
                        );
                    }
                }
            }
            let _ = r.inst("System.Text.StringBuilder.ToString", &sb, "ToString", vec![]);
        }));
    }

    // ---- byte[]: GetValue, SetValue, Length, indices in and out of range ----
    let data = rng.bytes(10);
    let b64 = base64(&data);
    covered.extend(both(&mut pair, &|r| {
        let Some(b) = r.invoke(
            "System.Convert.FromBase64String",
            st("System.Convert"),
            "FromBase64String",
            vec![s(&b64)],
        ) else {
            return;
        };
        for k in [-2, -1, 0, 1, 9, 10, 11, 1000] {
            let _ = r.inst("System.Array.GetValue", &b, "GetValue", vec![i(k)]);
            let _ = r.inst(
                "System.Array.SetValue",
                &b,
                "SetValue",
                vec![a(DotNetValue::Byte((k + 7) as u8)), i(k)],
            );
        }
        let _ = r.inst("System.Array.Length", &b, "Length", vec![]);
        let _ = r.get("System.Array.Length", DotNetTarget::Instance(b.0), "Length");
        r.read_bytes(&b);
    }));

    // ---- boxed Byte and Int32 ToString ----
    let formats: Vec<Option<&str>> = vec![
        None,
        Some("X2"),
        Some("x2"),
        Some("X"),
        Some("x"),
        Some("D"),
        Some("d"),
        Some("D5"),
        Some("d3"),
        Some("X8"),
        Some("x1"),
        Some("N"),
        Some("G"),
        Some("F2"),
        Some("E"),
        Some("X100"),
        Some("D0"),
        Some("Q"),
        Some(""),
        Some("00"),
    ];
    for f in &formats {
        for &v in &[
            0i32,
            1,
            9,
            10,
            15,
            16,
            255,
            256,
            65535,
            1_000_000,
            i32::MAX,
            -1,
            -255,
            i32::MIN,
        ] {
            covered.extend(both(&mut pair, &|r| {
                let args = || f.map_or(vec![], |f| vec![s(f)]);
                if (0..=255).contains(&v) {
                    r.cover("System.Byte.ToString");
                    let x = r
                        .session
                        .invoke(DotNetTarget::Value(DotNetValue::Byte(v as u8)), "ToString", args());
                    r.note(&format!("Byte {v} ToString {f:?}"), &x);
                }
                r.cover("System.Int32.ToString");
                let x = r
                    .session
                    .invoke(DotNetTarget::Value(DotNetValue::Int32(v)), "ToString", args());
                r.note(&format!("Int32 {v} ToString {f:?}"), &x);
            }));
        }
    }

    // ---- migration: an in-process object handed to a member of the sidecar ----
    covered.extend(both(&mut pair, &|r| {
        let Some(sb) = r.construct(
            "System.Text.StringBuilder..ctor",
            "System.Text.StringBuilder",
            vec![s("abc")],
        ) else {
            return;
        };
        let _ = r.inst("System.Text.StringBuilder.Append", &sb, "Append", vec![s("def")]);
        // String.Concat(object, object) is no member of the set: the builder moves to the sidecar.
        let _ = r.invoke(
            "(sidecar) System.String.Concat",
            st("System.String"),
            "Concat",
            vec![h(sb.0, &sb.1), s("!")],
        );
        // After the move, the same object answers through the sidecar and sees later appends.
        let _ = r.inst("System.Text.StringBuilder.Append", &sb, "Append", vec![s("ghi")]);
        let _ = r.inst("System.Text.StringBuilder.ToString", &sb, "ToString", vec![]);
        let _ = r.inst(
            "(sidecar) System.Text.StringBuilder.Insert",
            &sb,
            "Insert",
            vec![i(0), s(">")],
        );
        let _ = r.inst("System.Text.StringBuilder.ToString", &sb, "ToString", vec![]);

        let Some(b) = r.invoke(
            "System.Convert.FromBase64String",
            st("System.Convert"),
            "FromBase64String",
            vec![s("AQID")],
        ) else {
            return;
        };
        // MemoryStream(byte[]) shares the array: a write through the stream shows in the array.
        let ms = r.construct(
            "(sidecar) System.IO.MemoryStream",
            "System.IO.MemoryStream",
            vec![h(b.0, &b.1)],
        );
        if let Some(ms) = ms {
            let _ = r.inst(
                "(sidecar) System.IO.Stream.WriteByte",
                &ms,
                "WriteByte",
                vec![a(DotNetValue::Byte(9))],
            );
        }
        r.read_bytes(&b);
        let _ = r.inst(
            "System.Array.SetValue",
            &b,
            "SetValue",
            vec![a(DotNetValue::Byte(7)), i(2)],
        );
        r.read_bytes(&b);
        // A byte[] the sidecar made (MemoryStream.ToArray, the path of GenerateHash(InStream)) is read by value for the
        // stand-in's hash and codec.
        let ms2 = r.construct(
            "(sidecar) System.IO.MemoryStream",
            "System.IO.MemoryStream",
            vec![h(b.0, &b.1)],
        );
        if let Some(ms2) = ms2 {
            if let Some(arr) = r.inst("(sidecar) System.IO.MemoryStream.ToArray", &ms2, "ToArray", vec![]) {
                for algo in ["SHA256", "MD5"] {
                    if let Some(ha) = r.invoke(
                        "System.Security.Cryptography.HashAlgorithm.Create",
                        st("System.Security.Cryptography.HashAlgorithm"),
                        "Create",
                        vec![s(algo)],
                    ) {
                        if let Some(d) = r.inst(
                            "System.Security.Cryptography.HashAlgorithm.ComputeHash",
                            &ha,
                            "ComputeHash",
                            vec![h(arr.0, &arr.1)],
                        ) {
                            r.read_bytes(&d);
                        }
                    }
                }
                let _ = r.invoke(
                    "System.Convert.ToBase64String",
                    st("System.Convert"),
                    "ToBase64String",
                    vec![h(arr.0, &arr.1)],
                );
                let _ = r.inst("System.Array.Length", &arr, "Length", vec![]);
            }
        }
        let Some(e) = r.invoke("System.Text.Encoding.UTF8", st("System.Text.Encoding"), "UTF8", vec![]) else {
            return;
        };
        // An encoding handed to a sidecar member: its type name.
        let _ = r.invoke(
            "(sidecar) System.Object.ReferenceEquals",
            st("System.Object"),
            "ReferenceEquals",
            vec![h(e.0, &e.1), h(e.0, &e.1)],
        );
        let _ = r.inst("System.Text.Encoding.GetBytes", &e, "GetBytes", vec![s("é")]);
    }));

    // ---- TimeZoneInfo and DateTimeOffset (ADR-038 amendment 2026-10-17) ----
    covered.extend(time_zone_checks(&mut pair));

    // ---- the list of members equals what was covered ----
    let listed: BTreeSet<String> = MEMBERS
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(String::from)
        .collect();
    let ours: BTreeSet<String> = covered
        .iter()
        .filter(|c| !c.starts_with("(sidecar)"))
        .cloned()
        .collect();
    assert_eq!(listed, ours, "inproc.txt and the members the differential test covers");
}

fn base64(b: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for c in b.chunks(3) {
        let n =
            (u32::from(c[0]) << 16) | (u32::from(*c.get(1).unwrap_or(&0)) << 8) | u32::from(*c.get(2).unwrap_or(&0));
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if c.len() > 1 {
            T[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if c.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    out
}

const MAX_TICKS: i64 = 3_155_378_975_999_999_999;
const MINUTE: i64 = 600_000_000;
const HOUR: i64 = 36_000_000_000;
const DAY: i64 = 24 * HOUR;

fn dt(t: i64) -> DotNetArg {
    a(DotNetValue::DateTime(t))
}

fn ts(t: i64) -> DotNetArg {
    a(DotNetValue::TimeSpan(t))
}

/// Ticks of a calendar hour (proleptic Gregorian; 0001-01-01 is 0).
fn ticks_of(y: i64, m: i64, d: i64, h: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days_since_1970 = era * 146_097 + doe - 719_468;
    (days_since_1970 + 719_162) * DAY + h * HOUR
}

fn time_zone_checks(pair: &mut Pair) -> BTreeSet<String> {
    let mut covered = BTreeSet::new();
    let mut rng = Rng(0xD1B5_4A32_D192_ED03);

    // Ticks: the ends, leap days, DST switch days of several zones, random ones.
    let mut ticks: Vec<i64> = vec![
        0,
        1,
        MINUTE,
        DAY,
        MAX_TICKS,
        MAX_TICKS - 1,
        MAX_TICKS - DAY,
        MAX_TICKS - 14 * HOUR,
        14 * HOUR,
        14 * HOUR - 1,
        14 * HOUR + 1,
    ];
    for (y, m, d) in [
        (2000, 2, 29),
        (2024, 2, 29),
        (1900, 2, 28),
        (2100, 3, 1),
        (2024, 3, 10),
        (2024, 11, 3),
        (2024, 3, 31),
        (2024, 10, 27),
        (2024, 4, 7),
        (2024, 10, 6),
        (1970, 1, 1),
        (1582, 10, 15),
        (9999, 12, 31),
    ] {
        for h in [0, 1, 2, 3, 12, 23] {
            ticks.push(ticks_of(y, m, d, h));
        }
    }
    ticks.extend((0..30).map(|_| (rng.next() % (MAX_TICKS as u64)) as i64));

    // DateTimeOffset(DateTime): every member on each tick.
    for &t in &ticks {
        covered.extend(both(pair, &|r| {
            let Some(o) = r.construct("System.DateTimeOffset..ctor", "System.DateTimeOffset", vec![dt(t)]) else {
                return;
            };
            let me = DotNetTarget::Instance(o.0);
            r.get("System.DateTimeOffset.Offset", me.clone(), "Offset");
            r.get("System.DateTimeOffset.LocalDateTime", me.clone(), "LocalDateTime");
            r.get("System.DateTimeOffset.DateTime", me.clone(), "DateTime");
            if let Some(l) = r.inst("System.DateTimeOffset.ToLocalTime", &o, "ToLocalTime", vec![]) {
                r.get("System.DateTimeOffset.Offset", DotNetTarget::Instance(l.0), "Offset");
                r.get(
                    "System.DateTimeOffset.DateTime",
                    DotNetTarget::Instance(l.0),
                    "DateTime",
                );
            }
        }));
    }
    // DateTimeOffset(Int64, TimeSpan): valid and invalid offsets, ticks near both ends.
    let offsets = [
        0,
        HOUR,
        -HOUR,
        14 * HOUR,
        -14 * HOUR,
        14 * HOUR + MINUTE,
        -14 * HOUR - MINUTE,
        5 * HOUR + 30 * MINUTE,
        -(9 * HOUR + 30 * MINUTE),
        5 * HOUR + 45 * MINUTE,
        12 * HOUR + 45 * MINUTE,
        10_000_000,
        MINUTE + 1,
        MINUTE - 1,
        i64::MAX,
        i64::MIN,
    ];
    for &t in ticks.iter().take(40) {
        for &o in &offsets {
            covered.extend(both(pair, &|r| {
                let Some(d) = r.construct(
                    "System.DateTimeOffset..ctor",
                    "System.DateTimeOffset",
                    vec![a(DotNetValue::Int64(t)), ts(o)],
                ) else {
                    return;
                };
                let me = DotNetTarget::Instance(d.0);
                r.get("System.DateTimeOffset.Offset", me.clone(), "Offset");
                r.get("System.DateTimeOffset.LocalDateTime", me.clone(), "LocalDateTime");
                r.get("System.DateTimeOffset.DateTime", me.clone(), "DateTime");
                r.inst("System.DateTimeOffset.ToLocalTime", &d, "ToLocalTime", vec![]);
            }));
        }
    }
    // A value that is not a DateTime for the constructor goes to the sidecar and fails the same way.
    covered.extend(both(pair, &|r| {
        r.construct("System.DateTimeOffset..ctor", "System.DateTimeOffset", vec![i(5)]);
        r.construct("System.DateTimeOffset..ctor", "System.DateTimeOffset", vec![]);
    }));

    // TimeZoneInfo: every Windows id of the table, UTC, and ids the stand-in must not take.
    let mut ids: Vec<String> = al_host::windows_zones::WINDOWS_ZONES
        .iter()
        .map(|(w, _)| w.to_string())
        .collect();
    ids.extend(
        [
            "UTC",
            "Etc/UTC",
            "Europe/Berlin",
            "utc",
            "w. europe standard time",
            "W. Europe Standard Time ",
            "",
            "Nowhere Standard Time",
            "Greenwich Standard Time",
            "Mexico Standard Time",
            "Turkey Standard Time",
            "Kaliningrad Standard Time",
            "Russian Standard Time",
            "Iran Standard Time",
            "Nepal Standard Time",
            "Chatham Islands Standard Time",
            "Line Islands Standard Time",
            "Samoa Standard Time",
            "Morocco Standard Time",
            "Sudan Standard Time",
            "Eastern Standard Time",
            "China Standard Time",
        ]
        .map(String::from),
    );
    for id in &ids {
        covered.extend(both(pair, &|r| {
            r.seen.push(format!("zone id {id:?}"));
            let Some(z) = r.invoke(
                "System.TimeZoneInfo.FindSystemTimeZoneById",
                st("System.TimeZoneInfo"),
                "FindSystemTimeZoneById",
                vec![s(id)],
            ) else {
                return;
            };
            r.get(
                "System.TimeZoneInfo.BaseUtcOffset",
                DotNetTarget::Instance(z.0),
                "BaseUtcOffset",
            );
        }));
    }

    // GetUtcOffset(DateTime): served in process for the UTC zone only; every other zone moves to the sidecar on its first
    // call. Every id of the list on a spread of ticks, so a zone the stand-in took by mistake shows here.
    for id in &ids {
        covered.extend(both(pair, &|r| {
            r.seen.push(format!("zone id {id:?}"));
            let Some(z) = r.invoke(
                "System.TimeZoneInfo.FindSystemTimeZoneById",
                st("System.TimeZoneInfo"),
                "FindSystemTimeZoneById",
                vec![s(id)],
            ) else {
                return;
            };
            for &t in ticks.iter().step_by(7) {
                r.inst("System.TimeZoneInfo.GetUtcOffset", &z, "GetUtcOffset", vec![dt(t)]);
            }
        }));
    }
    for id in ["UTC", "utc"] {
        covered.extend(both(pair, &|r| {
            let Some(z) = r.invoke(
                "System.TimeZoneInfo.FindSystemTimeZoneById",
                st("System.TimeZoneInfo"),
                "FindSystemTimeZoneById",
                vec![s(id)],
            ) else {
                return;
            };
            for &t in &ticks {
                r.inst("System.TimeZoneInfo.GetUtcOffset", &z, "GetUtcOffset", vec![dt(t)]);
            }
            // An argument that is not a DateTime goes to the sidecar and answers or fails the same way.
            r.inst("System.TimeZoneInfo.GetUtcOffset", &z, "GetUtcOffset", vec![i(5)]);
            r.inst("System.TimeZoneInfo.GetUtcOffset", &z, "GetUtcOffset", vec![]);
        }));
    }

    // Mixing: objects made in process move to the sidecar when a member outside the set is called.
    for id in [
        "UTC",
        "W. Europe Standard Time",
        "India Standard Time",
        "Pacific Standard Time",
    ] {
        for t in [
            ticks_of(2024, 3, 31, 1),
            ticks_of(2024, 11, 3, 8),
            ticks_of(2024, 7, 1, 12),
            0,
            MAX_TICKS,
        ] {
            covered.extend(both(pair, &|r| {
                let z = r.invoke(
                    "System.TimeZoneInfo.FindSystemTimeZoneById",
                    st("System.TimeZoneInfo"),
                    "FindSystemTimeZoneById",
                    vec![s(id)],
                );
                let z2 = r.invoke(
                    "System.TimeZoneInfo.FindSystemTimeZoneById",
                    st("System.TimeZoneInfo"),
                    "FindSystemTimeZoneById",
                    vec![s(id)],
                );
                let d = r.construct("System.DateTimeOffset..ctor", "System.DateTimeOffset", vec![dt(t)]);
                let (Some(z), Some(z2), Some(d)) = (z, z2, d) else {
                    return;
                };
                let zt = DotNetTarget::Instance(z.0);
                // Members outside the set, on a moved object.
                r.invoke(
                    "System.TimeZoneInfo.GetUtcOffset",
                    zt.clone(),
                    "GetUtcOffset",
                    vec![dt(t)],
                );
                r.invoke(
                    "(sidecar) TimeZoneInfo.IsDaylightSavingTime",
                    zt.clone(),
                    "IsDaylightSavingTime",
                    vec![dt(t)],
                );
                r.get(
                    "(sidecar) TimeZoneInfo.SupportsDaylightSavingTime",
                    zt.clone(),
                    "SupportsDaylightSavingTime",
                );
                r.get("(sidecar) TimeZoneInfo.Id", zt.clone(), "Id");
                // The second handle on the same id is the same object.
                r.invoke(
                    "(sidecar) Object.ReferenceEquals",
                    st("System.Object"),
                    "ReferenceEquals",
                    vec![h(z.0, &z.1), h(z2.0, &z2.1)],
                );
                // Two moved objects in one sidecar call.
                r.invoke(
                    "(sidecar) TimeZoneInfo.ConvertTime",
                    st("System.TimeZoneInfo"),
                    "ConvertTime",
                    vec![h(d.0, &d.1), h(z.0, &z.1)],
                );
                let dto = DotNetTarget::Instance(d.0);
                r.invoke(
                    "(sidecar) DateTimeOffset.ToString",
                    dto.clone(),
                    "ToString",
                    vec![s("o")],
                );
                r.invoke(
                    "(sidecar) DateTimeOffset.ToUnixTimeSeconds",
                    dto.clone(),
                    "ToUnixTimeSeconds",
                    vec![],
                );
                r.get("(sidecar) DateTimeOffset.UtcDateTime", dto.clone(), "UtcDateTime");
                // Still the same object afterwards.
                r.get("System.DateTimeOffset.LocalDateTime", dto, "LocalDateTime");
                r.get("System.TimeZoneInfo.BaseUtcOffset", zt, "BaseUtcOffset");
            }));
        }
    }
    // A sidecar-made DateTimeOffset stays in the sidecar and still answers the members of the set.
    covered.extend(both(pair, &|r| {
        let Some(p) = r.invoke(
            "(sidecar) DateTimeOffset.Parse",
            st("System.DateTimeOffset"),
            "Parse",
            vec![s("2024-03-31T01:30:00+05:30")],
        ) else {
            return;
        };
        let me = DotNetTarget::Instance(p.0);
        r.get("System.DateTimeOffset.Offset", me.clone(), "Offset");
        r.get("System.DateTimeOffset.LocalDateTime", me.clone(), "LocalDateTime");
        r.get("System.DateTimeOffset.DateTime", me, "DateTime");
        r.inst("System.DateTimeOffset.ToLocalTime", &p, "ToLocalTime", vec![]);
    }));
    covered
}
