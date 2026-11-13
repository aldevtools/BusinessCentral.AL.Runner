// The render mode of the helper (ADR-040 amendment 2026-11-13b): started with `--render`, one process per run. It calls
// BC's own Word merger and document converter from the assemblies of the user's BC artifact. Nothing here is patched or
// re-implemented; only unmodified assemblies run.
//
// Requests: `hello`, `render_probe` (configure the folders and pieces, answer per need), `word_merge`, `word_to_pdf`,
// `word_to_html`. Bytes travel as Base64.
using System.Reflection;
using System.Runtime.InteropServices;
using System.Runtime.Loader;
using System.Text.Json.Nodes;

namespace AlDotNetSidecar;

public sealed class RenderServer
{
    const string MergerAssembly = "Microsoft.Dynamics.Nav.OpenXml";
    const string MergerType = "Microsoft.Dynamics.Nav.OpenXml.Word.DocumentMerger.OfficeWordDocumentMerger";
    const string ConverterAssembly = "Microsoft.BusinessCentral.DocumentProcessor";
    const string ConverterType = "Microsoft.BusinessCentral.DocumentProcessor.WordTransformation";

    // The closed overload table: one row per entry point, the parameter types the round-0 spike read from the metadata of
    // builds 24.5, 27.5, 28.1, 28.5 and 29.1 (the same on all). An overload that is not here is refused by name.
    static readonly Type[] MergeRow =
    {
        typeof(Stream), typeof(Stream), typeof(Stream), typeof(string), typeof(string), typeof(string), typeof(string),
        typeof(CancellationToken),
    };
    static readonly Type[] ConvertRow = { typeof(Stream), typeof(Stream) };

    readonly List<Resolver.Dir> dirs = new();
    // Linux: System.Drawing.Common 6.0.0 and Microsoft.Win32.SystemEvents 6.0.0 by simple name, in place of the
    // artifact's Windows-only build.
    readonly Dictionary<string, string> drawing = new(StringComparer.OrdinalIgnoreCase);
    string? skiaNative;

    public RenderServer()
    {
        if (OperatingSystem.IsLinux()) AppContext.SetSwitch("System.Drawing.EnableUnixSupport", true);
        AssemblyLoadContext.Default.Resolving += OnResolving;
        AssemblyLoadContext.Default.ResolvingUnmanagedDll += OnResolvingNative;
    }

    public JsonObject Handle(JsonObject req)
    {
        switch (req["op"]!.GetValue<string>())
        {
            case "hello":
            {
                int theirs = req["protocol"]?.GetValue<int>() ?? 0;
                return Ok(new JsonObject
                {
                    ["protocol"] = Program.ProtocolVersion,
                    ["compatible"] = theirs == Program.ProtocolVersion,
                    ["runtime"] = RuntimeInformation.FrameworkDescription,
                    ["major"] = Environment.Version.Major,
                    ["mode"] = "render",
                });
            }
            case "render_probe":
                Configure(req);
                return Ok(Probe());
            case "word_merge":
            {
                var m = Method(MergerAssembly, MergerType, "MergeWordDocument", MergeRow);
                using var layout = new MemoryStream(Bytes(req, "layout"));
                using var data = new MemoryStream(Bytes(req, "dataset"));
                var output = new MemoryStream();
                // The two custom XML names of the recipe of al-runner issue 7977 (the merger's data part); the title and
                // the company are the caller's.
                var ret = Invoke(m, layout, data, output, "NavWordReportXmlPart", "urn:microsoft-dynamics-nav/reports/",
                    req["title"]?.GetValue<string>() ?? "", req["company"]?.GetValue<string>() ?? "",
                    CancellationToken.None);
                var stream = ret as Stream ?? output;
                return Ok(new JsonObject { ["bytes"] = Convert.ToBase64String(ReadAll(stream)) });
            }
            case "word_to_pdf":
                return Ok(new JsonObject { ["bytes"] = Convert.ToBase64String(ConvertDocx("ConvertToPdf", req)) });
            case "word_to_html":
                return Ok(new JsonObject { ["bytes"] = Convert.ToBase64String(ConvertDocx("ConvertToHtml", req)) });
            default:
                throw new Refusal($"unknown render request '{req["op"]}'");
        }
    }

    public JsonObject FailureOf(Exception ex)
    {
        var e = ex is TargetInvocationException { InnerException: { } inner } ? inner : ex;
        if (e is Refusal or MissingPiece) return new JsonObject { ["ok"] = false, ["kind"] = "refused", ["reason"] = e.Message };
        return new JsonObject
        {
            ["ok"] = false,
            ["kind"] = "exception",
            ["type"] = e.GetType().FullName,
            ["message"] = e.Message,
        };
    }

    static JsonObject Ok(JsonNode? value) => new() { ["ok"] = true, ["value"] = value };

    static byte[] Bytes(JsonObject req, string key) => Convert.FromBase64String(req[key]!.GetValue<string>());

    static byte[] ReadAll(Stream s)
    {
        if (s is MemoryStream ms) return ms.ToArray();
        s.Position = 0;
        using var copy = new MemoryStream();
        s.CopyTo(copy);
        return copy.ToArray();
    }

    byte[] ConvertDocx(string name, JsonObject req)
    {
        var m = Method(ConverterAssembly, ConverterType, name, ConvertRow);
        using var input = new MemoryStream(Bytes(req, "docx"));
        using var output = new MemoryStream();
        Invoke(m, input, output);
        return output.ToArray();
    }

    static object? Invoke(MethodInfo m, params object?[] args)
    {
        try { return m.Invoke(null, args); }
        catch (TargetInvocationException t) when (t.InnerException != null) { throw t.InnerException; }
    }

    void Configure(JsonObject req)
    {
        dirs.Clear();
        foreach (var d in req["dirs"]?.AsArray() ?? new JsonArray())
            dirs.Add(new Resolver.Dir(d!["path"]!.GetValue<string>(), d["recursive"]?.GetValue<bool>() ?? false));
        drawing.Clear();
        foreach (var p in req["drawing"]?.AsArray() ?? new JsonArray())
        {
            var path = p!.GetValue<string>();
            drawing[Path.GetFileNameWithoutExtension(path)] = path;
        }
        skiaNative = req["skia"]?.GetValue<string>();
    }

    // ---- availability -------------------------------------------------------------------------

    JsonObject Probe()
    {
        var merge = Check(() => Method(MergerAssembly, MergerType, "MergeWordDocument", MergeRow));
        var pictures = merge["ok"]!.GetValue<bool>() ? Pictures() : merge.DeepClone().AsObject();
        var pdf = Converter("ConvertToPdf");
        var html = Converter("ConvertToHtml");
        return new JsonObject { ["merge"] = merge, ["merge_pictures"] = pictures, ["pdf"] = pdf, ["html"] = html };
    }

    static JsonObject Available() => new() { ["ok"] = true };

    static JsonObject Missing(string piece, string detail) =>
        new() { ["ok"] = false, ["piece"] = piece, ["detail"] = detail };

    static JsonObject Check(Func<MethodInfo> find)
    {
        try
        {
            find();
            return Available();
        }
        catch (MissingPiece m) { return Missing(m.Piece, m.Detail); }
    }

    JsonObject Pictures()
    {
        if (!OperatingSystem.IsLinux()) return Available();
        foreach (var name in new[] { "System.Drawing.Common", "Microsoft.Win32.SystemEvents" })
            if (!drawing.TryGetValue(name, out var p) || !File.Exists(p))
                return Missing(name + ".dll 6.0.0", "not fetched");
        if (!TryLoadSystem(new[] { "libgdiplus.so.0", "libgdiplus.so", "libgdiplus" }))
            return Missing("libgdiplus", "install the distribution's libgdiplus package");
        return Available();
    }

    JsonObject Converter(string entry)
    {
        var found = Check(() => Method(ConverterAssembly, ConverterType, entry, ConvertRow));
        if (!found["ok"]!.GetValue<bool>()) return found;
        if (OperatingSystem.IsLinux())
        {
            if (!TryLoadSystem(new[] { "libharfbuzz.so.0", "libharfbuzz.so" }))
                return Missing("libharfbuzz.so.0", "install the distribution's HarfBuzz package");
            if (!HasFont()) return Missing("a font", "no font file under /usr/share/fonts or /usr/local/share/fonts");
        }
        return Skia();
    }

    // A missing or mismatched Skia native raises nothing on a conversion: pictures are dropped silently (round-0 spike).
    // So the native is loaded here and its version compared with the managed SkiaSharp the build ships.
    JsonObject Skia()
    {
        string? managedPath = FindInDirs("SkiaSharp", ".dll");
        if (managedPath == null) return Missing("SkiaSharp.dll", "not in the probe folders");
        var managed = System.Diagnostics.FileVersionInfo.GetVersionInfo(managedPath);
        string want = $"{managed.FileMajorPart}.{managed.FileMinorPart}.{managed.FileBuildPart}";
        string? native = OperatingSystem.IsLinux() ? skiaNative : FindInDirs("libSkiaSharp", ".dll");
        if (native == null || !File.Exists(native)) return Missing($"libSkiaSharp {want}", "not fetched");
        if (!NativeLibrary.TryLoad(native, out var handle)) return Missing($"libSkiaSharp {want}", $"{native} does not load");
        try
        {
            var milestone = Export(handle, "sk_version_get_milestone");
            var increment = Export(handle, "sk_version_get_increment");
            if (milestone == null || increment == null)
                return Missing($"libSkiaSharp {want}", $"{native} exports no version");
            string got = $"{milestone()}.{increment()}";
            // SkiaSharp's own rule: the native milestone is the managed minor version.
            if (milestone() != managed.FileMinorPart)
                return Missing($"libSkiaSharp {want}", $"{native} is Skia milestone {got}");
            return new JsonObject { ["ok"] = true, ["skia"] = got };
        }
        finally { NativeLibrary.Free(handle); }
    }

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    delegate int IntGetter();

    static IntGetter? Export(IntPtr lib, string name) =>
        NativeLibrary.TryGetExport(lib, name, out var p) ? Marshal.GetDelegateForFunctionPointer<IntGetter>(p) : null;

    static bool TryLoadSystem(string[] names)
    {
        foreach (var n in names)
            if (NativeLibrary.TryLoad(n, out _)) return true;
        return false;
    }

    static bool HasFont()
    {
        foreach (var root in new[] { "/usr/share/fonts", "/usr/local/share/fonts" })
        {
            if (!Directory.Exists(root)) continue;
            try
            {
                if (Directory.EnumerateFiles(root, "*", SearchOption.AllDirectories).Any(f =>
                        f.EndsWith(".ttf", StringComparison.OrdinalIgnoreCase)
                        || f.EndsWith(".otf", StringComparison.OrdinalIgnoreCase)
                        || f.EndsWith(".ttc", StringComparison.OrdinalIgnoreCase)))
                    return true;
            }
            catch (UnauthorizedAccessException) { }
        }
        return false;
    }

    // ---- entry points -------------------------------------------------------------------------

    sealed class MissingPiece : Exception
    {
        public string Piece { get; }
        public string Detail { get; }
        public MissingPiece(string piece, string detail) : base($"{piece} not found ({detail})") { Piece = piece; Detail = detail; }
    }

    MethodInfo Method(string assembly, string type, string name, Type[] row)
    {
        Assembly asm;
        try { asm = AssemblyLoadContext.Default.LoadFromAssemblyName(new AssemblyName(assembly)); }
        catch (Exception ex) when (ex is FileNotFoundException or FileLoadException or BadImageFormatException)
        {
            throw new MissingPiece(assembly + ".dll", Resolver.Chain(ex) + "; searched " + Searched());
        }
        var t = asm.GetType(type, false) ?? throw new MissingPiece(type, $"not in {asm.Location}");
        var all = t.GetMethods(BindingFlags.Public | BindingFlags.Static).Where(m => m.Name == name).ToList();
        var hit = all.FirstOrDefault(m => m.GetParameters().Select(p => p.ParameterType).SequenceEqual(row));
        if (hit != null) return hit;
        var found = string.Join("; ", all.Select(m => "(" + string.Join(", ", m.GetParameters().Select(p => p.ParameterType.Name)) + ")"));
        throw new MissingPiece($"{name} overload", $"no overload of the table; found {(found.Length == 0 ? "none" : found)}");
    }

    string Searched() => dirs.Count == 0 ? "no folder" : string.Join(", ", dirs.Select(d => d.Path));

    string? FindInDirs(string simple, string ext)
    {
        foreach (var dir in dirs)
        {
            if (!Directory.Exists(dir.Path)) continue;
            var opt = dir.Recursive ? SearchOption.AllDirectories : SearchOption.TopDirectoryOnly;
            var hit = Directory.EnumerateFiles(dir.Path, simple + ext, opt).FirstOrDefault();
            if (hit != null) return hit;
        }
        return null;
    }

    Assembly? OnResolving(AssemblyLoadContext ctx, AssemblyName name)
    {
        if (name.Name == null) return null;
        if (OperatingSystem.IsLinux() && (name.Name == "System.Drawing.Common" || name.Name == "Microsoft.Win32.SystemEvents"))
            return drawing.TryGetValue(name.Name, out var p) && File.Exists(p) ? ctx.LoadFromAssemblyPath(p) : null;
        var hit = FindInDirs(name.Name, ".dll");
        return hit == null ? null : ctx.LoadFromAssemblyPath(hit);
    }

    IntPtr OnResolvingNative(Assembly asm, string lib)
    {
        string? path = null;
        if (OperatingSystem.IsLinux())
        {
            if (lib.Contains("SkiaSharp", StringComparison.OrdinalIgnoreCase)) path = skiaNative;
            else if (lib == "harfbuzz") return Load(new[] { "libharfbuzz.so.0", "libharfbuzz.so" });
            else if (lib is "libgdiplus" or "gdiplus") return Load(new[] { "libgdiplus.so.0", "libgdiplus.so" });
        }
        else if (lib.Contains("SkiaSharp", StringComparison.OrdinalIgnoreCase)) path = FindInDirs("libSkiaSharp", ".dll");
        else if (lib == "harfbuzz") path = FindInDirs("harfbuzz", ".dll");
        return path != null && NativeLibrary.TryLoad(path, out var h) ? h : IntPtr.Zero;
    }

    static IntPtr Load(string[] names)
    {
        foreach (var n in names)
            if (NativeLibrary.TryLoad(n, out var h)) return h;
        return IntPtr.Zero;
    }
}
