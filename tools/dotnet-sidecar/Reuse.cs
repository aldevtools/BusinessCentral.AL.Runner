// Reusing this process for the next session (ADR-014, amendment 2026-11-02). At the end of a session the host
// sends `reset`. The process answers whether it is still equal to a fresh one on everything it can measure; a
// process that is not is recycled (killed) by the host. Nothing here claims a complete reset: statics of loaded
// types cannot be put back, so a session that touched anything outside the allow-list below makes the process dirty.
using System.Collections;
using System.Globalization;
using System.Reflection;
using System.Text.Json.Nodes;

namespace AlDotNetSidecar;

public sealed class SessionGuard
{
    // A type in one of these namespaces can reach process-wide state no check below sees (threads, timers, the console,
    // AppContext data, reflection, the network): a session that used one is dirty.
    static readonly string[] DirtyNamespaces =
    {
        "System.Threading", "System.Timers", "System.Diagnostics", "System.Net", "System.Console", "System.AppContext",
        "System.AppDomain", "System.Runtime", "System.Reflection", "System.IO.Pipes", "System.IO.MemoryMappedFiles",
        "System.Security", "System.GC",
    };

    static readonly string FrameworkDir = Path.GetDirectoryName(typeof(object).Assembly.Location) ?? "";

    readonly HashSet<string> assemblies;
    readonly CultureInfo culture, uiCulture;
    readonly CultureInfo? defaultCulture, defaultUiCulture;
    readonly string[] formats;
    readonly string cwd;
    readonly Dictionary<string, string> env;
    readonly long timers;

    SessionGuard()
    {
        assemblies = Loaded();
        culture = CultureInfo.CurrentCulture;
        uiCulture = CultureInfo.CurrentUICulture;
        defaultCulture = CultureInfo.DefaultThreadCurrentCulture;
        defaultUiCulture = CultureInfo.DefaultThreadCurrentUICulture;
        formats = Formats(culture);
        cwd = Directory.GetCurrentDirectory();
        env = Env();
        timers = Timer.ActiveCount;
    }

    /// The state of a process that just answered `hello`.
    public static SessionGuard Capture() => new();

    static HashSet<string> Loaded() =>
        AppDomain.CurrentDomain.GetAssemblies().Select(a => a.GetName().Name ?? "").ToHashSet();

    static Dictionary<string, string> Env() =>
        Environment.GetEnvironmentVariables().Cast<DictionaryEntry>()
            .ToDictionary(e => (string)e.Key, e => (string?)e.Value ?? "");

    /// Every public property of the date and time formats of `c`, as text (name=value): the patterns, names, designators
    /// and separators .NET formats a DateTime with, and the calendar.
    static string[] Formats(CultureInfo c)
    {
        var f = c.DateTimeFormat;
        return typeof(DateTimeFormatInfo).GetProperties(BindingFlags.Public | BindingFlags.Instance)
            .Where(p => p.GetIndexParameters().Length == 0)
            .OrderBy(p => p.Name, StringComparer.Ordinal)
            .Select(p => p.Name + "=" + p.GetValue(f) switch
            {
                string[] a => string.Join("\u0001", a),
                Calendar cal => cal.GetType().FullName + "/" + cal.TwoDigitYearMax.ToString(CultureInfo.InvariantCulture),
                var v => Convert.ToString(v, CultureInfo.InvariantCulture),
            })
            .ToArray();
    }

    /// The current culture is the object of a process that just answered `hello` and its date and time formats have their
    /// values of then (the `formatting` request; al-vm `dotnet/datetime_text.rs` reads it).
    public bool FormattingAtStart() => ReferenceEquals(CultureInfo.CurrentCulture, culture) && FormatsAtStart();

    bool FormatsAtStart() => Formats(culture).SequenceEqual(formats);

    static bool Framework(Assembly a) =>
        !a.IsDynamic && FrameworkDir.Length > 0 && a.Location.StartsWith(FrameworkDir, StringComparison.Ordinal);

    /// Puts back what .NET allows to put back and reports what it cannot. `used`: the types the session named.
    public JsonObject Reset(IEnumerable<Type> used)
    {
        GC.Collect();
        GC.WaitForPendingFinalizers();
        GC.Collect();
        var repaired = new JsonArray();
        var dirty = new JsonArray();
        if (!ReferenceEquals(CultureInfo.CurrentCulture, culture) || !ReferenceEquals(CultureInfo.CurrentUICulture, uiCulture)
            || !ReferenceEquals(CultureInfo.DefaultThreadCurrentCulture, defaultCulture)
            || !ReferenceEquals(CultureInfo.DefaultThreadCurrentUICulture, defaultUiCulture))
        {
            CultureInfo.DefaultThreadCurrentCulture = defaultCulture;
            CultureInfo.DefaultThreadCurrentUICulture = defaultUiCulture;
            CultureInfo.CurrentCulture = culture;
            CultureInfo.CurrentUICulture = uiCulture;
            repaired.Add("culture");
        }
        // The start culture object itself, changed in place (`DateTimeFormat.LongTimePattern := ..`), cannot be put back.
        if (!FormatsAtStart()) dirty.Add("culture-formats");
        if (Directory.GetCurrentDirectory() != cwd)
        {
            Directory.SetCurrentDirectory(cwd);
            repaired.Add("cwd");
        }
        var now = Env();
        if (now.Count != env.Count || now.Any(kv => !env.TryGetValue(kv.Key, out var v) || v != kv.Value))
        {
            foreach (var k in now.Keys.Where(k => !env.ContainsKey(k))) Environment.SetEnvironmentVariable(k, null);
            foreach (var kv in env) Environment.SetEnvironmentVariable(kv.Key, kv.Value);
            TimeZoneInfo.ClearCachedData();
            repaired.Add("environment");
        }
        var extra = new JsonArray();
        foreach (var a in AppDomain.CurrentDomain.GetAssemblies())
        {
            var name = a.GetName().Name ?? "";
            if (assemblies.Contains(name)) continue;
            if (Framework(a)) { extra.Add(name); continue; }
            dirty.Add("assembly:" + name);
        }
        foreach (var t in used)
        {
            var ns = t.Namespace ?? "";
            var full = t.FullName ?? t.Name;
            if (!Framework(t.Assembly) && !assemblies.Contains(t.Assembly.GetName().Name ?? "")) continue; // counted above
            if (DirtyNamespaces.Any(p => ns == p || ns.StartsWith(p + ".", StringComparison.Ordinal) || full == p))
                dirty.Add("type:" + full);
        }
        if (Timer.ActiveCount > timers) dirty.Add("timers");
        return new JsonObject { ["reusable"] = dirty.Count == 0, ["dirty"] = dirty, ["repaired"] = repaired, ["framework_assemblies"] = extra };
    }
}
