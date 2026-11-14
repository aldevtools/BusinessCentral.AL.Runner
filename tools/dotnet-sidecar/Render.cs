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

    // The closed overload table: one row per entry point, the return type and the parameter types the round-0 spike read
    // from the metadata of builds 24.5, 27.5, 28.1, 28.5 and 29.1 (the same on all). An overload that is not here is
    // refused by name.
    sealed record Row(Type Returns, Type[] Parameters);
    static readonly Row MergeRow = new(typeof(Stream), new[]
    {
        typeof(Stream), typeof(Stream), typeof(Stream), typeof(string), typeof(string), typeof(string), typeof(string),
        typeof(CancellationToken),
    });
    static readonly Row ConvertRow = new(typeof(void), new[] { typeof(Stream), typeof(Stream) });

    readonly List<Resolver.Dir> dirs = new();
    // Linux: System.Drawing.Common 6.0.0 and Microsoft.Win32.SystemEvents 6.0.0 by simple name, in place of the
    // artifact's Windows-only build.
    readonly Dictionary<string, string> drawing = new(StringComparer.OrdinalIgnoreCase);
    string? skiaNative;
    // The SkiaSharp assembly whose native imports are bound to the probed file (set once per process).
    Assembly? skiaBound;
    // Set by `render_probe`: the simple names of each set's files (`merge`, `conversion`).
    readonly Dictionary<string, List<string>> sets = new();

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
        sets.Clear();
        if (req["sets"] is JsonObject given)
            foreach (var (name, files) in given)
                sets[name] = files!.AsArray().Select(f => f!.GetValue<string>()).ToList();
    }

    // ---- availability -------------------------------------------------------------------------

    JsonObject Probe()
    {
        var merge = Set("merge") ?? Check(() => Method(MergerAssembly, MergerType, "MergeWordDocument", MergeRow));
        var pictures = merge["ok"]!.GetValue<bool>() ? Pictures() : merge.DeepClone().AsObject();
        var conversion = Set("conversion");
        var pdf = conversion ?? Converter("ConvertToPdf");
        var html = conversion?.DeepClone().AsObject() ?? Converter("ConvertToHtml");
        return new JsonObject { ["merge"] = merge, ["merge_pictures"] = pictures, ["pdf"] = pdf, ["html"] = html };
    }

    // Every file of the set, by name, in the probe folders; the first one missing is the answer. No list: the host did
    // not send the set, so nothing of it is available.
    JsonObject? Set(string name)
    {
        if (!sets.TryGetValue(name, out var files)) return Missing($"the {name} set", "not given");
        foreach (var f in files)
            if (FindInDirs(f, ".dll") == null) return Missing(f + ".dll", "searched " + Searched());
        return null;
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
        // Windows: the artifact's own System.Drawing (not measured here). Linux: the 6.0.0 pair and libgdiplus.
        if (OperatingSystem.IsWindows()) return Available();
        if (!OperatingSystem.IsLinux()) return Missing("System.Drawing", "not supported on this system");
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
    // So SkiaSharp's native imports are bound to the probed file (a resolver runs before .NET's own search, which would
    // take a native beside SkiaSharp.dll or in a system folder first), and SkiaSharp's own compatibility check is asked
    // on the bound library. Either failing makes PDF and Html unavailable, by name.
    JsonObject Skia()
    {
        Assembly skia;
        try { skia = AssemblyLoadContext.Default.LoadFromAssemblyName(new AssemblyName("SkiaSharp")); }
        catch (Exception ex) when (ex is FileNotFoundException or FileLoadException or BadImageFormatException)
        {
            return Missing("SkiaSharp.dll", Resolver.Chain(ex) + "; searched " + Searched());
        }
        var managed = skia.GetName().Version;
        string piece = $"libSkiaSharp for SkiaSharp {managed}";
        string? native = SkiaNativePath();
        if (native == null || !File.Exists(native)) return Missing(piece, "not fetched");
        if (skiaBound == null)
        {
            try { NativeLibrary.SetDllImportResolver(skia, ResolveSkia); }
            catch (InvalidOperationException ex) { return Missing(piece, "its native imports cannot be bound: " + ex.Message); }
            skiaBound = skia;
        }
        else if (skiaBound != skia) return Missing(piece, "a second SkiaSharp assembly was loaded");
        var version = skia.GetType("SkiaSharp.SkiaSharpVersion", false);
        var check = version?.GetMethod("CheckNativeLibraryCompatible", BindingFlags.Public | BindingFlags.Static,
            new[] { typeof(bool) });
        if (check == null || check.ReturnType != typeof(bool))
            return Missing("SkiaSharpVersion.CheckNativeLibraryCompatible(bool)", $"not in {skia.Location}");
        bool compatible;
        try { compatible = (bool)Invoke(check, false)!; }
        catch (Exception ex) when (ex is DllNotFoundException or EntryPointNotFoundException or BadImageFormatException
                                       or TypeInitializationException)
        {
            return Missing(piece, $"{native}: {ex.GetType().Name}: {ex.Message}");
        }
        if (!compatible) return Missing(piece, $"{native} is not compatible (SkiaSharp's own check)");
        return new JsonObject { ["ok"] = true, ["skia"] = version!.GetProperty("Native")?.GetValue(null)?.ToString() };
    }

    // Linux: the file the host fetched and checked by its SHA-256. Elsewhere: the artifact's own.
    string? SkiaNativePath() => OperatingSystem.IsLinux() ? skiaNative : FindInDirs("libSkiaSharp", ".dll");

    // Binds every native import of SkiaSharp to the probed file. A file that does not load is not replaced by another.
    IntPtr ResolveSkia(string lib, Assembly asm, DllImportSearchPath? path)
    {
        if (!lib.Contains("SkiaSharp", StringComparison.OrdinalIgnoreCase)) return IntPtr.Zero;
        string? native = SkiaNativePath();
        if (native != null && NativeLibrary.TryLoad(native, out var h)) return h;
        throw new DllNotFoundException($"{native ?? "no Skia native"} does not load");
    }

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

    MethodInfo Method(string assembly, string type, string name, Row row)
    {
        Assembly asm;
        try { asm = AssemblyLoadContext.Default.LoadFromAssemblyName(new AssemblyName(assembly)); }
        catch (Exception ex) when (ex is FileNotFoundException or FileLoadException or BadImageFormatException)
        {
            throw new MissingPiece(assembly + ".dll", Resolver.Chain(ex) + "; searched " + Searched());
        }
        var t = asm.GetType(type, false) ?? throw new MissingPiece(type, $"not in {asm.Location}");
        var all = t.GetMethods(BindingFlags.Public | BindingFlags.Static).Where(m => m.Name == name).ToList();
        var hit = all.FirstOrDefault(m =>
            m.ReturnType == row.Returns && m.GetParameters().Select(p => p.ParameterType).SequenceEqual(row.Parameters));
        if (hit != null) return hit;
        var found = string.Join("; ", all.Select(m =>
            m.ReturnType.Name + " (" + string.Join(", ", m.GetParameters().Select(p => p.ParameterType.Name)) + ")"));
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
            if (lib == "harfbuzz") return Load(new[] { "libharfbuzz.so.0", "libharfbuzz.so" });
            else if (lib is "libgdiplus" or "gdiplus") return Load(new[] { "libgdiplus.so.0", "libgdiplus.so" });
        }
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
