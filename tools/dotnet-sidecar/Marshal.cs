// Tagged values on the wire (ADR-014 section 4) and their conversion to and from .NET objects.
//
//   {"t":"n"}                              null
//   {"t":"ns"}                             null whose declared type is string (ADR-024)
//   {"t":"b","v":true}                     bool
//   {"t":"u8","v":200}                     byte
//   {"t":"c","v":65}                       char (UTF-16 code unit)
//   {"t":"i4","v":1}  {"t":"i8","v":1}     int, long
//   {"t":"d","v":"1.50"}                   decimal, as text
//   {"t":"s","v":"text"}                   string
//   {"t":"dt","v":"638396640000000000"}    DateTime ticks, kind unspecified
//   {"t":"dtu","v":"638396640000000000"}   DateTime ticks, kind Utc (only sent, never returned)
//   {"t":"ts","v":"15000000"}              TimeSpan ticks
//   {"t":"g","v":"01234567-89ab-..."}      Guid, in textual order
//   {"t":"e","type":"System.X","v":"Name","o":1}   enum value by name and ordinal
//   {"t":"h","v":12,"type":"System.IO.MemoryStream"}   object held here
//   {"t":"ins","v":"<base64>"}             AL InStream: the unread bytes
//   {"t":"ins","v":"<base64>","p":n}       AL InStream past its start: all its bytes, the stream at position n
//   {"t":"outs"}                           AL OutStream: becomes a bridge stream
//   {"t":"outs","v":"<base64>"}            AL OutStream over a Blob that holds these bytes: the bridge starts with them
using System.Globalization;
using System.Text.Json.Nodes;

namespace AlDotNetSidecar;

public sealed class Marshaller
{
    readonly Dictionary<long, object> handles = new();
    long nextHandle = 1;
    readonly Func<string, Type> resolveType;

    // Streams passed in the current request, and bridges that live until the process ends.
    public sealed class InArg { public int Index; public MemoryStream Stream = null!; }
    public readonly List<InArg> Ins = new();
    public readonly List<(int Index, long Bridge)> NewBridges = new();
    readonly Dictionary<long, Bridge> bridges = new();
    long nextBridge = 1;

    public Marshaller(Func<string, Type> resolveType) { this.resolveType = resolveType; }

    public int HandleCount => handles.Count;

    public object Handle(long h) =>
        handles.TryGetValue(h, out var o) ? o : throw new Refusal($"unknown object handle {h}");

    public void Release(JsonArray hs)
    {
        foreach (var h in hs) handles.Remove(h!.GetValue<long>());
    }

    /// The end of a session: every handle and stream bridge goes. The counters are not restarted, so a stale handle is
    /// an error, never a hit on another session's object.
    public void ReleaseAll()
    {
        handles.Clear();
        bridges.Clear();
        updateArchives.Clear();
    }

    // The one .NET writer allowed to rewrite the bytes an AL OutStream held before the call. The pin: System Application's Data
    // Compression makes `ZipArchive(OutStream, ZipArchiveMode.Update, false, Encoding)` over a Blob it copied a zip into, and the
    // archive's Dispose rewrites that zip: the Blob then holds the archive's whole final content (corpus
    // tests/al-language/streams/blob-tempblob/ZipBlobFieldUpdate.al:5-8, lengths 119 and 216 at :7; al-runner issue 8360).
    // Facts read, each by equality: a ZipArchive made by a constructor of two, three or four arguments, its Mode Update, its
    // stream a bridge that started with the Blob's bytes; the call is that archive's Dispose with no argument, written
    // `Dispose()` or `Dispose;` (one AL call). The host decides which of these rewrites it serves ([Bridge.ReplacedKind]: the
    // pin above is the four-argument one; the two- and three-argument ones only over a fresh CreateOutStream, assumption
    // zip-blob-writes). Every other writer keeps the refusal of [Bridge].
    sealed record UpdateOf(Bridge Bridge, int Arity);
    readonly System.Runtime.CompilerServices.ConditionalWeakTable<System.IO.Compression.ZipArchive, UpdateOf> updateArchives = new();

    public void NoteConstructed(object? obj, object?[] coerced)
    {
        if (obj is System.IO.Compression.ZipArchive { Mode: System.IO.Compression.ZipArchiveMode.Update } za
            && coerced.Length is 2 or 3 or 4 && coerced[0] is Bridge { Seeded: true } b)
            updateArchives.AddOrUpdate(za, new UpdateOf(b, coerced.Length));
        // The writers of a bridge, for [Bridge.OverwriteMeasured]: StreamWriter(OutStream) is the measured one.
        if (obj is StreamWriter && coerced.Length > 0 && coerced[0] is Bridge w)
        {
            if (coerced.Length == 1) w.PlainWriters++;
            else w.OtherWriters++;
        }
    }

    /// The bridge whose seed this call may rewrite ([Bridge.RewriteAllowed]), or null.
    public Bridge? RewriteBridge(object? target, string member, int argc)
    {
        if (target is System.IO.Compression.ZipArchive za && member == "Dispose" && argc == 0
            && updateArchives.TryGetValue(za, out var u))
        {
            u.Bridge.RewriteKind = u.Arity == 4 ? "update4" : "update";
            return u.Bridge;
        }
        return null;
    }

    /// The bridges a call rewrote ([Bridge.Replaced]), as [[id, bytes the Blob held before, base64 of the whole content, kind], ...].
    /// Filled by [DrainBridges].
    public JsonArray Replaced { get; } = new();

    public void BeginRequest()
    {
        Ins.Clear();
        NewBridges.Clear();
    }

    /// Bytes written into any bridge since the last call, as [[id, base64], ...]. This runs after every call and a session
    /// can hold thousands of bridges (one per OutStream argument), so a bridge nothing wrote into must cost no copy: its
    /// buffer is read in place (al-runner issue 7016; copying every bridge made one codeunit take 17 minutes). Both bridge
    /// kinds are expandable MemoryStreams, whose buffer stays readable after Dispose. A bridge whose stream .NET closed hands
    /// back its last bytes now and is then dropped: nothing can write into a closed MemoryStream any more.
    public JsonArray DrainBridges()
    {
        TakeOverwritten();
        Replaced.Clear();
        var arr = new JsonArray();
        foreach (var id in bridges.Keys.ToList())
        {
            var b = bridges[id];
            if (!b.TryGetBuffer(out var buf)) buf = new ArraySegment<byte>(b.ToArray());
            if (b.Replaced)
            {
                Replaced.Add(new JsonArray(id, b.Drained, Convert.ToBase64String(buf.Array!, buf.Offset, buf.Count), b.ReplacedKind));
                b.Drained = buf.Count;
                b.Replaced = false;
            }
            else if (buf.Count > b.Drained)
            {
                arr.Add(new JsonArray(id, Convert.ToBase64String(buf.Array!, buf.Offset + b.Drained, buf.Count - b.Drained)));
                b.Drained = buf.Count;
            }
            if (!b.CanRead) bridges.Remove(id);
        }
        return arr;
    }

    /// The ids of the bridges a write went over bytes they had handed back ([Bridge.Overwritten]) since the last call. It runs
    /// first in [DrainBridges], before a closed bridge is dropped.
    public JsonArray Overwritten { get; } = new();

    void TakeOverwritten()
    {
        Overwritten.Clear();
        foreach (var (id, b) in bridges)
        {
            if (!b.Overwritten) continue;
            Overwritten.Add(id);
            b.Overwritten = false;
        }
    }

    /// No bridge holds bytes [DrainBridges] has not handed back yet. A failed request sends no bytes (only [Done] drains), so
    /// a failure the host may show as an AL error must have written into no bridge: else the bytes would reach the Blob only
    /// with a later reply, after AL went on.
    public bool BridgesUnwritten()
    {
        foreach (var b in bridges.Values)
        {
            if (!b.TryGetBuffer(out var buf)) buf = new ArraySegment<byte>(b.ToArray());
            if (buf.Count != b.Drained || b.Overwritten || b.Replaced) return false;
        }
        return true;
    }

    // ---- wire to .NET ----

    public object? Decode(JsonNode? n, int index)
    {
        if (n == null) return null;
        var o = n.AsObject();
        switch (o["t"]!.GetValue<string>())
        {
            case "n": return null;
            case "b": return o["v"]!.GetValue<bool>();
            case "u8": return (byte)o["v"]!.GetValue<int>();
            case "c": return (char)o["v"]!.GetValue<int>();
            case "i4": return o["v"]!.GetValue<int>();
            case "i8": return o["v"]!.GetValue<long>();
            case "d": return decimal.Parse(o["v"]!.GetValue<string>(), NumberStyles.Float, CultureInfo.InvariantCulture);
            case "s": return o["v"]!.GetValue<string>();
            case "dt": return new DateTime(long.Parse(o["v"]!.GetValue<string>(), CultureInfo.InvariantCulture), DateTimeKind.Unspecified);
            case "dtu": return new DateTime(long.Parse(o["v"]!.GetValue<string>(), CultureInfo.InvariantCulture), DateTimeKind.Utc);
            case "ts": return new TimeSpan(long.Parse(o["v"]!.GetValue<string>(), CultureInfo.InvariantCulture));
            case "g": return Guid.Parse(o["v"]!.GetValue<string>());
            case "e": return Enum.Parse(resolveType(o["type"]!.GetValue<string>()), o["v"]!.GetValue<string>());
            case "h": return Handle(o["v"]!.GetValue<long>());
            case "ins":
            {
                var ms = new MemoryStream(Convert.FromBase64String(o["v"]!.GetValue<string>()), writable: false);
                // An InStream that stands further on: all its bytes, at its position ("p"), so .NET can seek back
                // (al-runner issue 7134). The consumed count is the position in these bytes.
                if (o["p"] is JsonNode at) ms.Position = at.GetValue<long>();
                Ins.Add(new InArg { Index = index, Stream = ms });
                return ms;
            }
            case "outs":
            {
                // An OutStream over bytes AL already wrote: BC hands .NET the Blob's own stream, which reads, writes and
                // seeks, standing at the end of the bytes (al-runner issue 6317, corpus dotnet/dotnet/WdZipArchiveOverOutStream.al).
                var seed = o["v"] is JsonNode sv ? Convert.FromBase64String(sv.GetValue<string>()) : Array.Empty<byte>();
                var ms = new Bridge(seed);
                long id = nextBridge++;
                bridges[id] = ms;
                NewBridges.Add((index, id));
                return ms;
            }
            default: throw new Refusal("unknown value tag " + o["t"]);
        }
    }

    // ---- .NET to wire ----

    public JsonNode Encode(object? v)
    {
        switch (v)
        {
            case null: return Tag("n");
            case string s: return Tag("s", s);
            case bool b: return Tag("b", b);
            case int i: return Tag("i4", i);
            case long l: return Tag("i8", l);
            case byte u: return Tag("u8", (int)u);
            case char c: return Tag("c", (int)c);
            case short sh: return Tag("i4", (int)sh);
            case ushort us: return Tag("i4", (int)us);
            case sbyte sb: return Tag("i4", (int)sb);
            case uint ui: return Tag("i8", (long)ui);
            case decimal m: return Tag("d", m.ToString(CultureInfo.InvariantCulture));
            case double d:
                return Tag("d", ToDecimal(d).ToString(CultureInfo.InvariantCulture));
            case float f:
                return Tag("d", ToDecimal(f).ToString(CultureInfo.InvariantCulture));
            case DateTime dt: return Tag("dt", dt.Ticks.ToString(CultureInfo.InvariantCulture));
            case TimeSpan ts: return Tag("ts", ts.Ticks.ToString(CultureInfo.InvariantCulture));
            case Guid g: return Tag("g", g.ToString("D"));
            case Enum e:
            {
                var o = Tag("e", e.ToString());
                o["type"] = e.GetType().FullName;
                o["o"] = Convert.ToInt64(e, CultureInfo.InvariantCulture);
                return o;
            }
            default:
            {
                long h = nextHandle++;
                handles[h] = v;
                var o = Tag("h", h);
                o["type"] = v.GetType().FullName;
                return o;
            }
        }
    }

    static decimal ToDecimal(double d)
    {
        try { return (decimal)d; }
        catch (OverflowException) { throw new Refusal($"the .NET number {d.ToString(CultureInfo.InvariantCulture)} does not fit an AL Decimal"); }
    }

    static JsonObject Tag(string t, JsonNode? v = null)
    {
        var o = new JsonObject { ["t"] = t };
        if (v != null) o["v"] = v;
        return o;
    }
}

/// A request this process cannot serve: type or member not found, no matching overload. It is
/// reported as a refusal, not as an exception of the code under test (ADR-014 section 7).
/// The stream .NET gets for an AL OutStream. It may start with the bytes the AL Blob holds, positioned at their end. [Drained]
/// counts the bytes the Blob already has: the seed, then what [Marshaller.DrainBridges] handed back after each call. The host
/// appends only what is written after them. A write over the seed (or a length below it) is refused (al-runner issue 6317). A
/// write over bytes handed back later, or a length below them, sets [Overwritten]: the call goes on, as in BC
/// (tests/al-language-onprem-dotnet/dotnet/zip-create/Y44ZipCreateMode.al, Zip_OpenTemporaryNoCopy_SecondCreate), and the host
/// marks the Blob, as what it then holds is not pinned (al-runner issue 7947), unless [OverwriteMeasured]. While [RewriteAllowed]
/// (the one pinned writer, [Marshaller.RewriteBridge]) a write over those bytes sets [Replaced] instead: the host replaces the
/// Blob's bytes with the whole content. [ReplacedKind] says which writer did: "update4" or "update" (a ZipArchive in Update
/// mode, made with four or with two or three arguments) or "overwrite" ([OverwriteMeasured]); the host gates each.
public sealed class Bridge : MemoryStream
{
    readonly int seeded;
    public int Drained;
    public bool Overwritten;
    public bool RewriteAllowed;
    public string RewriteKind = "update4";
    public bool Replaced;
    public string ReplacedKind = "update4";
    public int PlainWriters;
    public int OtherWriters;
    /// AL set `Position` of this stream to 0 ([Server] "set"), or AL's `SetLength` call on it runs.
    public bool AlPositionZero;
    public bool AlSetLength;
    /// AL's call of a method of this stream other than SetLength runs ([Server] "InvokeMethod").
    public bool AlDirectCall;
    public bool Seeded => seeded > 0;

    public Bridge(byte[] seed)
    {
        if (seed.Length > 0) base.Write(seed, 0, seed.Length);
        seeded = seed.Length;
        Drained = seed.Length;
    }

    // The measured .NET overwrite of handed-back bytes (corpus PR 1633, tests/al-language-onprem-dotnet/dotnet/zip-blob-writes/
    // ZipBlobDotNetOverwrite.al, al-runner issue 7947; assumption zip-blob-writes): a bridge that started empty, whose one writer
    // is a StreamWriter made with the stream alone, after AL set its Position to 0 or while AL's SetLength runs; the write over
    // them is that StreamWriter's (AL's call of a method of the stream itself, such as WriteByte, keeps the mark). The host
    // hands the whole content to the Blob only when the Blob held just the bridge's bytes.
    bool OverwriteMeasured => seeded == 0 && PlainWriters == 1 && OtherWriters == 0 && AlDirectCall == false
        && (AlPositionZero || AlSetLength);

    void Check(long at)
    {
        if (RewriteAllowed && at < Drained)
        {
            Replaced = true;
            ReplacedKind = RewriteKind;
            return;
        }
        if (at < seeded)
            throw new Refusal("a .NET write over the bytes an AL OutStream held before the call (what the Blob then holds is not pinned)");
        if (at < Drained && OverwriteMeasured)
        {
            Replaced = true;
            ReplacedKind = "overwrite";
        }
        else if (at < Drained) Overwritten = true;
    }

    public override void Write(byte[] buffer, int offset, int count) { if (count > 0) Check(Position); base.Write(buffer, offset, count); }
    public override void Write(ReadOnlySpan<byte> buffer) { if (buffer.Length > 0) Check(Position); base.Write(buffer); }
    public override void WriteByte(byte value) { Check(Position); base.WriteByte(value); }
    public override void SetLength(long value) { Check(value); base.SetLength(value); }
}

public sealed class Refusal : Exception
{
    public Refusal(string message) : base(message) { }
}
