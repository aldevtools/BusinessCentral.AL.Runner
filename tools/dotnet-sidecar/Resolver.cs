// Finding assemblies and types (ADR-014 section 3).
//
// The default load context serves the .NET standard library from the installed runtime. For
// everything else, the host sends probe directories (the Add-ins folders of the platform package,
// its service folder, then user-supplied folders) and this resolver serves an assembly by simple
// name from them, in that order. The version rule follows AL Runner's guard: serve a candidate
// whose assembly version is at least the requested one; refuse an older one, except for
// Microsoft.Dynamics.*, which is stamped per BC build.
using System.Reflection;
using System.Runtime.Loader;

namespace AlDotNetSidecar;

public sealed class Resolver
{
    public sealed record Dir(string Path, bool Recursive);

    readonly List<Dir> dirs = new();
    readonly Dictionary<string, Type> types = new();
    // The types the session named, for the check at its end (`SessionGuard`).
    readonly HashSet<Type> used = new();

    /// The types named since the last call.
    public List<Type> TakeUsed()
    {
        var l = used.ToList();
        used.Clear();
        return l;
    }
    [ThreadStatic] static bool readingVersion;

    public Resolver()
    {
        AssemblyLoadContext.Default.Resolving += OnResolving;
    }

    public void AddDirs(IEnumerable<Dir> more) => dirs.AddRange(more);

    Assembly? OnResolving(AssemblyLoadContext ctx, AssemblyName name)
    {
        if (name.Name == null) return null;
        foreach (var dir in dirs)
        {
            if (!Directory.Exists(dir.Path)) continue;
            foreach (var ext in new[] { ".dll", ".exe" })
            {
                var opt = dir.Recursive ? SearchOption.AllDirectories : SearchOption.TopDirectoryOnly;
                string? hit = Directory.EnumerateFiles(dir.Path, name.Name + ext, opt).FirstOrDefault();
                if (hit == null) continue;
                Guard(name, hit);
                return ctx.LoadFromAssemblyPath(hit);
            }
        }
        return null;
    }

    static void Guard(AssemblyName requested, string path)
    {
        if (requested.Version == null) return;
        if (requested.Name!.StartsWith("Microsoft.Dynamics.", StringComparison.OrdinalIgnoreCase)) return;
        // Reading a version loads System.Reflection.Metadata, which can re-enter this handler:
        // refuse instead of recursing.
        if (readingVersion)
            throw new FileLoadException($"assembly '{requested}' requested while a version was being read; refusing to recurse");
        readingVersion = true;
        try
        {
            var found = AssemblyName.GetAssemblyName(path).Version;
            if (found != null && found < requested.Version)
                throw new FileLoadException(
                    $"assembly '{requested.Name}' {requested.Version} requested; {path} is {found}, which is older");
        }
        finally { readingVersion = false; }
    }

    /// Resolves a declared type: `{"assembly":..,"version":..,"culture":..,"token":..,"name":..}`.
    public Type ResolveType(string assembly, string? version, string? culture, string? token, string name)
    {
        string key = assembly + "|" + name;
        if (types.TryGetValue(key, out var cached)) { used.Add(cached); return cached; }
        Type? t = null;
        Exception? loadError = null;
        Exception? typeError = null;
        try
        {
            if (name.Contains('['))
            {
                // A closed generic keeps the CLR spelling: List`1[[System.String, mscorlib]].
                t = Type.GetType(name + ", " + assembly, a => LoadAssembly(a), null, false);
            }
            else
            {
                var asm = LoadAssembly(new AssemblyName(assembly));
                t = asm.GetType(name, false);
                // GetType(name, false) also answers null for a type the assembly defines but the runtime cannot load (a
                // base type or a referenced assembly that is missing or too new: BC 29's Microsoft.Dynamics.Nav.OpenXml
                // references System.Runtime 10.0, al-runner issue 6924). Ask again with the exception, for the message.
                if (t == null) typeError = TypeLoadError(asm, name);
            }
        }
        catch (Exception ex) when (ex is FileNotFoundException or FileLoadException or BadImageFormatException)
        {
            loadError = ex;
        }
        if (t == null && loadError == null)
        {
            // The declaring assembly may be a facade or may not be the one that holds it.
            foreach (var a in AppDomain.CurrentDomain.GetAssemblies())
            {
                t = a.GetType(name, false);
                if (t != null) break;
            }
        }
        if (t == null)
            throw new Refusal(loadError != null
                ? $"assembly '{assembly}' could not be loaded: {Chain(loadError)} {Searched(assembly)}"
                : $"type '{name}' not found in assembly '{assembly}'{Because(typeError)}");
        types[key] = t;
        used.Add(t);
        return t;
    }

    /// Why `asm` does not give the type `name`: the exception GetType raises when asked to throw, `null` when it raises none.
    static Exception? TypeLoadError(Assembly asm, string name)
    {
        try
        {
            asm.GetType(name, true);
            return null;
        }
        catch (Exception ex) when (ex is TypeLoadException or FileNotFoundException or FileLoadException
            or BadImageFormatException or ReflectionTypeLoadException)
        {
            return ex;
        }
    }

    /// The text after "type ... not found": the type-load exception and its inner exceptions, when there is one.
    static string Because(Exception? ex) => ex == null ? "" : ": " + Chain(ex);

    /// An exception's type and message, then each inner exception's, so a missing dependency names itself.
    public static string Chain(Exception ex)
    {
        var parts = new List<string>();
        for (var e = ex; e != null; e = e.InnerException)
            parts.Add($"{e.GetType().Name}: {e.Message.Trim()}");
        return string.Join(" <- ", parts);
    }

    /// Where an assembly that did not load was looked for, and how to add a folder (al-runner issues 6924, 6927).
    string Searched(string assembly)
    {
        string simple = assembly;
        try { simple = new AssemblyName(assembly).Name ?? assembly; }
        catch (Exception ex) when (ex is ArgumentException or FileLoadException) { }
        string where = dirs.Count == 0
            ? "Besides the .NET runtime's own assemblies, no folder was searched."
            : "Searched the .NET runtime's own assemblies, then: "
              + string.Join(", ", dirs.Select(d => d.Recursive ? d.Path + " (with subfolders)" : d.Path)) + ".";
        return $"{where} To use it, pass a folder that holds {simple}.dll with --dotnet-assembly-dir.";
    }

    /// A type named only by its full name, as it arrives inside an enum value.
    public Type ResolveLoaded(string name)
    {
        var t = Type.GetType(name, false);
        if (t != null) { used.Add(t); return t; }
        foreach (var a in AppDomain.CurrentDomain.GetAssemblies())
        {
            t = a.GetType(name, false);
            if (t != null) { used.Add(t); return t; }
        }
        throw new Refusal($"type '{name}' is not loaded");
    }

    static Assembly LoadAssembly(AssemblyName name) => Assembly.Load(name);
}
