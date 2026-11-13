// Requests and reflection dispatch (ADR-014 sections 4, 7 and 10).
using System.Globalization;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Text.Json.Nodes;

namespace AlDotNetSidecar;

public sealed class Server
{
    readonly Resolver resolver = new();
    readonly Marshaller marshal;
    SessionGuard? guard;

    public Server()
    {
        marshal = new Marshaller(resolver.ResolveLoaded);
    }

    public JsonObject Handle(JsonObject req)
    {
        marshal.BeginRequest();
        foldedCase = false;
        textAsGuid = false;
        bound = null;
        switch (req["op"]!.GetValue<string>())
        {
            case "hello":
            {
                guard ??= SessionGuard.Capture();
                int theirs = req["protocol"]?.GetValue<int>() ?? 0;
                return Ok(new JsonObject
                {
                    ["protocol"] = Program.ProtocolVersion,
                    ["compatible"] = theirs == Program.ProtocolVersion,
                    ["runtime"] = RuntimeInformation.FrameworkDescription,
                    ["major"] = Environment.Version.Major,
                    ["culture"] = CultureInfo.CurrentCulture.Name,
                    ["timezone"] = TimeZoneInfo.Local.Id,
                    ["cwd"] = Directory.GetCurrentDirectory(),
                });
            }
            case "probe":
                resolver.AddDirs(req["dirs"]!.AsArray().Select(d =>
                    new Resolver.Dir(d!["path"]!.GetValue<string>(), d["recursive"]?.GetValue<bool>() ?? false)));
                return Ok(null);
            case "reset":
            {
                // The end of a session (ADR-014 amendment 2026-11-02): drop what it made, report whether the process is
                // still equal to a fresh one.
                marshal.ReleaseAll();
                var used = resolver.TakeUsed();
                if (guard == null)
                    return Ok(new JsonObject { ["reusable"] = false, ["dirty"] = new JsonArray("no baseline") });
                return Ok(guard.Reset(used));
            }
            case "release":
                marshal.Release(req["handles"]!.AsArray());
                return Ok(null);
            case "count":
                return Ok(marshal.HandleCount);
            case "stream_write":
            {
                // CopyStream from an AL InStream into a .NET stream (ADR-024).
                var stream = StreamOf(req["handle"]!.GetValue<long>());
                if (!stream.CanWrite) throw new Refusal($"'{stream.GetType().FullName}' is a Stream that cannot be written");
                var bytes = Convert.FromBase64String(req["data"]!.GetValue<string>());
                stream.Write(bytes, 0, bytes.Length);
                return Ok(null);
            }
            case "stream_read":
            {
                // CopyStream from a .NET stream into an AL OutStream (ADR-024). Only a MemoryStream is
                // pinned (tests/al-language-onprem-dotnet/TestDotNetValueKindsContracts.al, codeunit
                // 61400, CopyStream_WithAMemoryStream): without a count the WHOLE stream whatever its
                // position, position 0 afterwards; with a count from the position, which follows.
                var stream = StreamOf(req["handle"]!.GetValue<long>());
                // The stream ZipArchiveEntry.Open() gives in Read mode, used once as the source of CopyStream without a count:
                // all its bytes (dotnet/zip-create/Y44ZipCreateMode.al ReadZip, Zip_TwoEntries_CopyStreamIntoOpen
                // `content_<i>`; a compressed entry gives a DeflateStream, the empty stored one `d.txt` a SubReadStream). The
                // runtime sends it only for a temporary Open() stream nothing read before.
                var entryRead = stream is System.IO.Compression.DeflateStream
                    || stream.GetType().FullName == "System.IO.Compression.SubReadStream";
                if (entryRead && req["count"] is null)
                {
                    var all = new MemoryStream();
                    stream.CopyTo(all);
                    return Ok(new JsonObject { ["t"] = "ins", ["v"] = Convert.ToBase64String(all.ToArray()) });
                }
                if (stream is not MemoryStream ms)
                    throw new Refusal($"reading '{stream.GetType().FullName}' for CopyStream is not pinned (only a MemoryStream is)");
                byte[] data;
                if (req["count"] is JsonNode c)
                {
                    long want = c.GetValue<long>();
                    long left = Math.Max(0, ms.Length - ms.Position);
                    data = new byte[Math.Min(want, left)];
                    ms.ReadExactly(data, 0, data.Length);
                }
                else
                {
                    data = ms.ToArray();
                    ms.Position = 0;
                }
                return Ok(new JsonObject { ["t"] = "ins", ["v"] = Convert.ToBase64String(data) });
            }
            case "bye":
                return Ok(null);
            case "construct":
            {
                var type = DeclaredType(req["type"]!);
                // A generic type AL declares without type arguments is made closed over System.Object, as BC does: the type
                // its messages name is `Dictionary`2[System.Object,System.Object]` (tests/al-language-onprem-dotnet/
                // dotnet/dotnet-dictionary/Y17DotNetDictionary.al:126, :147). System Application's ABS and AFS Operation Payload
                // make a SortedDictionary`2 so.
                if (type.IsGenericTypeDefinition)
                    type = type.MakeGenericType(type.GetGenericArguments().Select(_ => typeof(object)).ToArray());
                var args = DecodeArgs(req);
                var (best, coerced, _) = Pick(type.GetConstructors(BindingFlags.Public | BindingFlags.Instance), args, $"new {type.FullName}", textAsGuidFallback: true);
                bound = PlainParameters(best) ? "constructor" : null;
                object? obj = ((ConstructorInfo)best).Invoke(coerced);
                return Done(obj, best, coerced, args);
            }
            case "invoke":
            {
                var (target, type) = Target(req["target"]!);
                var args = DecodeArgs(req);
                var flags = BindingFlags.Public | BindingFlags.Static | BindingFlags.FlattenHierarchy | (target != null ? BindingFlags.Instance : 0);
                string name = BindName(type, flags, req["member"]!.GetValue<string>());
                var cands = type.GetMethods(flags).Where(m => m.Name == name && !m.IsGenericMethodDefinition).ToArray();
                if (cands.Length == 0)
                {
                    // A method that is generic only, whose one type parameter is its return type (OpenXml's
                    // `T WorkbookReader.GetWorksheetByName<T>(string)`): the runtime closes it over the declared type of the
                    // DotNet variable the result is stored into (`generic`); without one the type argument is not known.
                    // Only an overload that can take this many arguments counts: `JProperty.Value()` (Base Application,
                    // "JSON Management".GetPropertyValueFromJObjectByName) is the property `JToken Value`, read below as a
                    // call, not JToken's `T Value<T>(object key)`, which needs its key (al-runner issue 6927;
                    // tests/al-language-onprem-dotnet/dotnet/dotnet/R6927JPropertyValue.al:31,
                    // passed on BC 27.5, 28.5 and 29.0).
                    var generic = type.GetMethods(flags)
                        .Where(m => m.Name == name && m.IsGenericMethodDefinition && m.GetGenericArguments().Length == 1
                            && m.ReturnType == m.GetGenericArguments()[0]
                            && TakesArgCount(m, args.Length))
                        .ToArray();
                    if (generic.Length > 0)
                    {
                        if (req["generic"] is not JsonNode g)
                            throw new Refusal($"member '{name}' of '{type.FullName}' is generic only: its type argument is not known");
                        var arg = DeclaredType(g);
                        var closed = new List<MethodInfo>();
                        foreach (var m in generic)
                        {
                            try { closed.Add(m.MakeGenericMethod(arg)); }
                            catch (ArgumentException) { } // the type argument breaks a constraint of this overload
                        }
                        if (closed.Count == 0)
                            throw new Refusal($"member '{name}' of '{type.FullName}' cannot be closed over '{arg.FullName}'");
                        cands = closed.ToArray();
                    }
                }
                // A foreach loop calls MoveNext by name. An enumerator that implements IEnumerator only explicitly
                // (System.Xml.EmptyEnumerator, from an empty XmlAttributeCollection; al-runner issue 6316) has no public
                // member of that name: it answers through the interface.
                if (cands.Length == 0 && args.Length == 0 && name == "MoveNext" && target is System.Collections.IEnumerator ie)
                    return Done(ie.MoveNext(), null, null, null, typeof(bool));
                if (cands.Length == 0 && args.Length == 0)
                {
                    // AL lets a property be written as a call: `Encoding.UTF8()`.
                    var p = type.GetProperty(name, flags, null, null, Type.EmptyTypes, null);
                    if (p != null && p.GetMethod != null)
                        return Done(p.GetValue(p.GetMethod.IsStatic ? null : target), null, null, null, p.PropertyType);
                    var fld = type.GetField(name, flags);
                    if (fld != null)
                        return Done(fld.GetValue(fld.IsStatic ? null : target), null, null, null, fld.FieldType);
                }
                if (cands.Length == 0 && args.Length == 1)
                {
                    // AL lets a property be assigned as a call with the value as the one argument:
                    // `XmlNode.InnerText(Text)` (System Application, XML DOM Management.XMLEscape).
                    var sp = type.GetProperty(name, flags, null, null, Type.EmptyTypes, null);
                    if (sp != null && sp.SetMethod != null && sp.SetMethod.IsPublic)
                    {
                        var cv = Coerce(args[0], sp.PropertyType) ?? throw new Refusal($"cannot assign {Describe(args[0])} to {type.FullName}.{name} ({sp.PropertyType.FullName})");
                        sp.SetValue(sp.SetMethod.IsStatic ? null : target, cv.Value);
                        return Done(null, null, null, null);
                    }
                }
                if (cands.Length == 0 && args.Length > 0)
                {
                    // AL calls an indexer by the name the framework gives it (IndexerName):
                    // `XmlAttributeCollection.ItemOf('name')`. The getter takes the index arguments.
                    var indexers = type.GetProperties(flags).Where(p => p.Name == name).ToArray();
                    var getters = indexers
                        .Where(p => p.GetIndexParameters().Length == args.Length && p.GetMethod != null && p.GetMethod.IsPublic)
                        .Select(p => (MethodBase)p.GetMethod!).ToArray();
                    // With one argument more than the index, the last one is the value assigned through the indexer:
                    // `DataRow.Item(ColumnName, Value)` (System Application "Business Chart Impl."; corpus
                    // tests/al-language/streams/AuEdgeTests.al:608, real BC reads the value back).
                    // Both can fit two arguments (`DataRow.Item(String, DataRowVersion)` reads, `Item(String) = Object`
                    // writes): the closer fit wins and a tie reads. An Integer value is written, as on real BC (the
                    // corpus test above sets 7 with SetValueByIndex and reads it back).
                    var setters = indexers
                        .Where(p => p.GetIndexParameters().Length == args.Length - 1 && p.GetIndexParameters().Length > 0
                            && p.SetMethod != null && p.SetMethod.IsPublic)
                        .Select(p => (MethodBase)p.SetMethod!).ToArray();
                    (MethodBase Best, object?[] Coerced, int Score)? read = null, write = null;
                    Refusal? noFit = null;
                    if (getters.Length > 0)
                        try { read = Pick(getters, args, $"{type.FullName}.{name}"); } catch (Refusal r) { noFit = r; }
                    if (setters.Length > 0)
                        try { write = Pick(setters, args, $"{type.FullName}.{name}"); } catch (Refusal r) { noFit ??= r; }
                    if (read is { } g && (write is not { } w0 || g.Score <= w0.Score))
                    {
                        var gm = (MethodInfo)g.Best;
                        return Done(gm.Invoke(gm.IsStatic ? null : target, g.Coerced), g.Best, g.Coerced, args, gm.ReturnType);
                    }
                    if (write is { } w)
                    {
                        var sm = (MethodInfo)w.Best;
                        sm.Invoke(sm.IsStatic ? null : target, w.Coerced);
                        return Done(null, null, null, null);
                    }
                    if (noFit != null) throw noFit;
                }
                if (cands.Length == 0)
                    throw new Refusal(target == null && type.GetMethods(BindingFlags.Public | BindingFlags.Instance).Any(m => m.Name == name)
                        ? $"'{type.FullName}.{name}' is an instance member and the variable holds no object"
                        : $"member '{name}' not found on '{type.FullName}'");
                var (best, coerced, _) = Pick(cands, args, $"{type.FullName}.{name}", textAsGuidFallback: true);
                bound = PlainParameters(best) ? "method" : null;
                object? result = ((MethodInfo)best).Invoke(((MethodInfo)best).IsStatic ? null : target, coerced);
                return Done(result, best, coerced, args, ((MethodInfo)best).ReturnType);
            }
            case "get":
            {
                var (target, type) = Target(req["target"]!);
                var flags = BindingFlags.Public | BindingFlags.Static | BindingFlags.FlattenHierarchy | (target != null ? BindingFlags.Instance : 0);
                string name = BindName(type, flags, req["member"]!.GetValue<string>());
                var prop = type.GetProperty(name, flags, null, null, Type.EmptyTypes, null);
                if (prop != null && prop.GetMethod != null)
                {
                    bound = "property";
                    return Done(prop.GetValue(prop.GetMethod.IsStatic ? null : target), null, null, null, prop.PropertyType);
                }
                var field = type.GetField(name, flags);
                if (field != null)
                    return Done(field.GetValue(field.IsStatic ? null : target), null, null, null, field.FieldType);
                // `Current` of an enumerator that implements IEnumerator only explicitly (see MoveNext in "invoke").
                if (name == "Current" && target is System.Collections.IEnumerator cur)
                    return Done(cur.Current, null, null, null, typeof(object));
                // AL lets a method be called without parentheses.
                var m = type.GetMethods(flags).Where(x => x.Name == name && x.GetParameters().Length == 0 && !x.IsGenericMethodDefinition).FirstOrDefault();
                if (m != null)
                    return Done(m.Invoke(m.IsStatic ? null : target, Array.Empty<object>()), null, null, null, m.ReturnType);
                throw new Refusal($"property, field or method '{name}' not found on '{type.FullName}'");
            }
            case "set":
            {
                var (target, type) = Target(req["target"]!);
                var value = marshal.Decode(req["value"], 0);
                var flags = BindingFlags.Public | BindingFlags.Static | BindingFlags.FlattenHierarchy | (target != null ? BindingFlags.Instance : 0);
                string name = BindName(type, flags, req["member"]!.GetValue<string>());
                var prop = type.GetProperty(name, flags, null, null, Type.EmptyTypes, null);
                if (prop != null && prop.SetMethod != null)
                {
                    var v = Coerce(value, prop.PropertyType) ?? throw new Refusal($"cannot assign {Describe(value)} to {type.FullName}.{name} ({prop.PropertyType.FullName})");
                    prop.SetValue(prop.SetMethod.IsStatic ? null : target, v.Value);
                    return Done(null, null, null, null);
                }
                var field = type.GetField(name, flags);
                if (field != null && !field.IsInitOnly)
                {
                    var v = Coerce(value, field.FieldType) ?? throw new Refusal($"cannot assign {Describe(value)} to {type.FullName}.{name} ({field.FieldType.FullName})");
                    field.SetValue(field.IsStatic ? null : target, v.Value);
                    return Done(null, null, null, null);
                }
                throw new Refusal($"settable property or field '{name}' not found on '{type.FullName}'");
            }
            default:
                throw new Refusal("unknown op " + req["op"]);
        }
    }

    // AL does not tell identifiers apart by letter case, so a DotNet member written in another case than its .NET name
    // binds to it: Base Application codeunit 1203 "Import XML File to Data Exch.", ParseParentChildLine
    // (ImportXMLFiletoDataExch.Codeunit.al:106) writes `XmlElement.OuterXML` for the .NET `OuterXml`; it compiles and the
    // line runs on BC (al-runner issue 6897, compile acceptance as 6488). A Microsoft test that passes through it:
    // see assumption `dotnet-member-name-case`. A name some public method, property or field carries exactly is kept;
    // otherwise the one public name equal to it ignoring case is taken (and the reply says so, so the host marks the
    // assumption); two or more such names are refused, because which one BC binds is not pinned.
    bool foldedCase;
    // The call bound an AL Text argument to a System.Guid parameter (see PickTextAsGuid); the host marks its assumption.
    bool textAsGuid;
    // What the request called when the called code threw (the "bound" of an exception reply): "constructor", "method" (a
    // method called as a method), "property" (a property get of the get op), each only with [PlainParameters]. Null for every
    // other binding (an indexer, a setter, a property or field reached through another op): the host keeps those refused.
    string? bound;

    // Every parameter of the bound member is passed by value. A member with a ref or out parameter gets no "bound": what it
    // wrote before it threw does not reach the host, so the host keeps it refused.
    static bool PlainParameters(MethodBase m) => m.GetParameters().All(p => p.ParameterType.IsByRef == false);

    /// The reply for an exception the request raised: [Failure], with the binding that threw and whether the member name
    /// was folded to its .NET letter case, which the host needs for the error text AL sees. No binding when the request
    /// wrote into an OutStream bridge ([Marshaller.BridgesUnwritten]): the host keeps that call refused.
    public JsonObject FailureOf(Exception ex)
    {
        var reply = Failure(ex);
        if ((string?)reply["kind"] == "exception")
        {
            if (bound != null && marshal.BridgesUnwritten()) reply["bound"] = bound;
            if (foldedCase) reply["folded"] = true;
        }
        return reply;
    }

    string BindName(Type type, BindingFlags flags, string name)
    {
        foldedCase = false;
        var names = new HashSet<string>(StringComparer.Ordinal);
        foreach (var m in type.GetMethods(flags)) names.Add(m.Name);
        foreach (var p in type.GetProperties(flags)) names.Add(p.Name);
        foreach (var f in type.GetFields(flags)) names.Add(f.Name);
        if (names.Contains(name)) return name;
        var like = names.Where(n => string.Equals(n, name, StringComparison.OrdinalIgnoreCase)).OrderBy(n => n, StringComparer.Ordinal).ToArray();
        if (like.Length == 0) return name;
        if (like.Length > 1)
            throw new Refusal($"member '{name}' of '{type.FullName}' matches {string.Join(", ", like.Select(n => $"'{n}'"))} ignoring letter case: which one BC binds is not pinned");
        foldedCase = true;
        return like[0];
    }

    Stream StreamOf(long handle) =>
        marshal.Handle(handle) as Stream
        ?? throw new Refusal($"'{marshal.Handle(handle).GetType().FullName}' is not a System.IO.Stream");

    // ---- replies ----

    static JsonObject Ok(JsonNode? value) => new() { ["ok"] = true, ["value"] = value };

    JsonObject Done(object? result, MethodBase? method, object?[]? coerced, object?[]? args, Type? declared = null)
    {
        // A null whose declared type is string is tagged "ns" (ADR-024): AL reads it as an empty text.
        var reply = Ok(result == null && declared == typeof(string) ? new JsonObject { ["t"] = "ns" } : marshal.Encode(result));
        if (foldedCase) reply["folded"] = true;
        if (textAsGuid) reply["textAsGuid"] = true;
        // New values of by-reference parameters, by argument position.
        if (method != null && coerced != null)
        {
            var byref = new JsonArray();
            var ps = method.GetParameters();
            for (int i = 0; i < ps.Length; i++)
                if (ps[i].ParameterType.IsByRef && byRefFlags[i])
                    byref.Add(new JsonArray(i, marshal.Encode(coerced[i])));
            if (byref.Count > 0) reply["byref"] = byref;
        }
        var streams = new JsonObject();
        var consumed = new JsonArray();
        foreach (var s in marshal.Ins) consumed.Add(new JsonArray(s.Index, (int)s.Stream.Position));
        if (consumed.Count > 0) streams["consumed"] = consumed;
        var bridges = new JsonArray();
        foreach (var (idx, id) in marshal.NewBridges) bridges.Add(new JsonArray(idx, id));
        if (bridges.Count > 0) streams["bridges"] = bridges;
        var written = marshal.DrainBridges();
        if (written.Count > 0) streams["written"] = written;
        if (streams.Count > 0) reply["streams"] = streams;
        return reply;
    }

    /// Reflection wraps an exception raised by the called code in TargetInvocationException;
    /// anything else (type or member not found, no overload, a value that does not decode) is
    /// this process refusing the request, not an exception of the code under test.
    public static JsonObject Failure(Exception ex)
    {
        if (ex is not TargetInvocationException { InnerException: not null } tie)
            return new JsonObject { ["ok"] = false, ["kind"] = "refused", ["reason"] = ex.Message };
        ex = tie.InnerException!;
        // A refusal raised inside the called code (a write over a seeded bridge) stays a refusal.
        if (ex is Refusal r)
            return new JsonObject { ["ok"] = false, ["kind"] = "refused", ["reason"] = r.Message };
        var inner = new JsonArray();
        for (var e = ex.InnerException; e != null; e = e.InnerException)
            inner.Add(new JsonArray(e.GetType().FullName, e.Message));
        return new JsonObject
        {
            ["ok"] = false,
            ["kind"] = "exception",
            ["type"] = ex.GetType().FullName,
            ["message"] = ex.Message,
            ["inner"] = inner,
        };
    }

    // ---- targets, arguments, overloads ----

    Type DeclaredType(JsonNode t)
    {
        var o = t.AsObject();
        return resolver.ResolveType(
            o["assembly"]!.GetValue<string>(),
            o["version"]?.GetValue<string>(),
            o["culture"]?.GetValue<string>(),
            o["token"]?.GetValue<string>(),
            o["name"]!.GetValue<string>());
    }

    (object? Target, Type Type) Target(JsonNode t)
    {
        var o = t.AsObject();
        if (o["handle"] is JsonNode h)
        {
            var obj = marshal.Handle(h.GetValue<long>());
            return (obj, obj.GetType());
        }
        if (o["value"] is JsonNode v)
        {
            // A primitive an AL DotNet variable holds (ADR-024): box it for the call.
            var boxed = marshal.Decode(v, 0) ?? throw new Refusal("a member of a null value");
            return (boxed, boxed.GetType());
        }
        return (null, DeclaredType(o["static"]!));
    }

    bool[] byRefFlags = Array.Empty<bool>();

    object?[] DecodeArgs(JsonObject req)
    {
        var arr = req["args"]?.AsArray() ?? new JsonArray();
        byRefFlags = new bool[arr.Count];
        var res = new object?[arr.Count];
        for (int i = 0; i < arr.Count; i++)
        {
            byRefFlags[i] = arr[i]!["ref"]?.GetValue<bool>() ?? false;
            res[i] = marshal.Decode(arr[i]!["v"], i);
        }
        return res;
    }

    /// The candidate whose parameters accept the runtime types of the arguments with the fewest
    /// conversions: exact type, assignable, enum from integer, numeric conversion. By-reference
    /// parameters need an argument the AL caller passed as a variable.
    (MethodBase Best, object?[] Coerced, int Score) Pick(IEnumerable<MethodBase> cands, object?[] args, string what, bool textAsGuidFallback = false)
    {
        var all = cands as IReadOnlyCollection<MethodBase> ?? cands.ToList();
        var r = PickBy(all, args, what, false);
        if (r != null) return r.Value;
        // BC binds an AL Text to a System.Guid parameter (System Application Image Impl. EncodeToImage,
        // ImageImpl.Codeunit.al:347 `Encoder.Encoder(QualityEncodeGuidTxt)`; assumption dotnet-text-argument-as-guid). Only when no
        // overload takes the arguments as they are, the text is a Guid in form D or B, and exactly one overload fits that way.
        if (textAsGuidFallback && args.Any(a => a is string))
        {
            r = PickBy(all, args, what, true);
            if (r != null) { textAsGuid = true; return r.Value; }
        }
        throw new Refusal($"{what} has no overload taking {args.Length} argument(s) of the given types ({string.Join(", ", args.Select(Describe))})");
    }

    (MethodBase Best, object?[] Coerced, int Score)? PickBy(IReadOnlyCollection<MethodBase> cands, object?[] args, string what, bool guidText)
    {
        MethodBase? best = null;
        var bestScore = int.MaxValue;
        int fits = 0;
        foreach (var c in cands)
        {
            var ps = c.GetParameters();
            if (ps.Length != args.Length) continue;
            int score = 0;
            bool ok = true;
            for (int i = 0; i < ps.Length && ok; i++)
            {
                var pt = ps[i].ParameterType;
                if (pt.IsByRef)
                {
                    if (!byRefFlags[i]) { ok = false; break; }
                    pt = pt.GetElementType()!;
                    score += 1;
                }
                var s = guidText && args[i] is string t && pt == typeof(Guid) ? (IsGuidText(t) ? 6 : -1) : Cost(args[i], pt);
                if (s < 0) ok = false; else score += s;
            }
            if (ok) fits++;
            if (ok && score < bestScore) { best = c; bestScore = score; }
        }
        if (best == null || (guidText && fits != 1)) return null;
        var ps2 = best.GetParameters();
        var coerced = new object?[args.Length];
        for (int i = 0; i < args.Length; i++)
        {
            var pt = ps2[i].ParameterType;
            if (pt.IsByRef) pt = pt.GetElementType()!;
            coerced[i] = guidText && args[i] is string g && pt == typeof(Guid) ? Guid.Parse(g) : Coerce(args[i], pt)?.Value;
        }
        return (best, coerced, bestScore);
    }

    // Whether a method can be called with this many arguments: every required parameter given, and no more arguments
    // than parameters unless the last one is a params array.
    static bool TakesArgCount(MethodBase m, int count)
    {
        var ps = m.GetParameters();
        bool rest = ps.Length > 0 && ps[^1].IsDefined(typeof(ParamArrayAttribute), false);
        int required = ps.Count(p => !p.IsOptional) - (rest ? 1 : 0);
        return count >= required && (rest || count <= ps.Length);
    }

    static bool IsGuidText(string t) => Guid.TryParseExact(t, "D", out _) || Guid.TryParseExact(t, "B", out _);

    static string Describe(object? a) => a == null ? "null" : a.GetType().Name;

    /// -1: does not fit.
    static int Cost(object? a, Type pt)
    {
        if (a == null) return pt.IsValueType && Nullable.GetUnderlyingType(pt) == null ? -1 : 0;
        var at = a.GetType();
        if (pt == at) return 0;
        if (pt.IsAssignableFrom(at)) return 1;
        if (pt.IsEnum && (a is int or long)) return 2;
        if (pt.IsPrimitive && at.IsPrimitive && a is IConvertible && Widens(at, pt)) return 3;
        if (pt == typeof(decimal) && (a is int or long or byte or short)) return 3;
        // An AL Decimal is a .NET decimal; a double parameter takes it by numeric conversion.
        if (a is decimal && pt.IsPrimitive && pt != typeof(bool) && pt != typeof(char)) return 4;
        // A one-character AL text into a char[] (String.Split(','), whose one-argument form is `params char[]`): the
        // character (tests/al-language-onprem-dotnet/TestDotNetValueKindsContracts.al:250 Foreach_OverClrCollections,
        // dotnet/dotnet-builtin-args/Y51DotNetBuiltinArgs.al DotNetString_ErrorInfoCreate `Source.Split(',')`).
        if (pt == typeof(char[]) && a is string { Length: 1 }) return 5;
        return -1;
    }

    static bool Widens(Type from, Type to)
    {
        if (to == typeof(bool) || from == typeof(bool)) return false;
        if (to == typeof(char) || from == typeof(char)) return false;
        return true;
    }

    sealed record Boxed(object? Value);

    static Boxed? Coerce(object? a, Type pt)
    {
        if (a == null) return pt.IsValueType && Nullable.GetUnderlyingType(pt) == null ? null : new Boxed(null);
        if (pt.IsAssignableFrom(a.GetType())) return new Boxed(a);
        if (pt.IsEnum && a is int i) return new Boxed(Enum.ToObject(pt, i));
        if (pt.IsEnum && a is long l) return new Boxed(Enum.ToObject(pt, l));
        if (pt == typeof(char[]) && a is string { Length: 1 } one) return new Boxed(one.ToCharArray());
        try { return new Boxed(Convert.ChangeType(a, pt, CultureInfo.InvariantCulture)); }
        catch (Exception ex) when (ex is InvalidCastException or OverflowException or FormatException) { return null; }
    }
}

/// Test fixture for the case-binding rule (`BindName`): public members that differ only in letter case, which no
/// .NET framework type has. Reached by tests as assembly `AlDotNetSidecar`, type `AlDotNetSidecar.CaseFixture`.
public class CaseFixture
{
    public string value = "lower";
    public string Value = "upper";
    public string Single = "single";
    public static string StaticMember = "static";
    public string Echo(string s) => s;
}

/// Test fixture for the AL Text to System.Guid argument conversion (assumption `dotnet-text-argument-as-guid`): the shapes
/// of System.Drawing.Imaging.Encoder (one constructor taking a Guid) and of overload sets that make the conversion
/// unnecessary or ambiguous. Reached by tests as assembly `AlDotNetSidecar`, type `AlDotNetSidecar.GuidFixture`.
public class GuidFixture
{
    public string Got = "";
    public GuidFixture(Guid g) { Got = "guid " + g.ToString("D"); }
    public static string OnlyGuid(Guid g) => "guid " + g.ToString("D");
    public static string Both(string s) => "string " + s;
    public static string Both(Guid g) => "guid " + g.ToString("D");
    public static string Amb(Guid g, int i) => "int";
    public static string Amb(Guid g, long l) => "long";
    public static string Nullable(Guid? g) => "nullable";
}
