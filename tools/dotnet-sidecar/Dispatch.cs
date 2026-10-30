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

    public Server()
    {
        marshal = new Marshaller(resolver.ResolveLoaded);
    }

    public JsonObject Handle(JsonObject req)
    {
        marshal.BeginRequest();
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
                    ["culture"] = CultureInfo.CurrentCulture.Name,
                    ["timezone"] = TimeZoneInfo.Local.Id,
                    ["cwd"] = Directory.GetCurrentDirectory(),
                });
            }
            case "probe":
                resolver.AddDirs(req["dirs"]!.AsArray().Select(d =>
                    new Resolver.Dir(d!["path"]!.GetValue<string>(), d["recursive"]?.GetValue<bool>() ?? false)));
                return Ok(null);
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
                var args = DecodeArgs(req);
                var (best, coerced, _) = Pick(type.GetConstructors(BindingFlags.Public | BindingFlags.Instance), args, $"new {type.FullName}");
                object? obj = ((ConstructorInfo)best).Invoke(coerced);
                return Done(obj, best, coerced, args);
            }
            case "invoke":
            {
                var (target, type) = Target(req["target"]!);
                string name = req["member"]!.GetValue<string>();
                var args = DecodeArgs(req);
                var flags = BindingFlags.Public | BindingFlags.Static | BindingFlags.FlattenHierarchy | (target != null ? BindingFlags.Instance : 0);
                var cands = type.GetMethods(flags).Where(m => m.Name == name && !m.IsGenericMethodDefinition).ToArray();
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
                    var getters = type.GetProperties(flags)
                        .Where(p => p.Name == name && p.GetIndexParameters().Length > 0 && p.GetMethod != null && p.GetMethod.IsPublic)
                        .Select(p => (MethodBase)p.GetMethod!).ToArray();
                    if (getters.Length > 0)
                    {
                        var (ib, ic, _) = Pick(getters, args, $"{type.FullName}.{name}");
                        var gm = (MethodInfo)ib;
                        return Done(gm.Invoke(gm.IsStatic ? null : target, ic), ib, ic, args, gm.ReturnType);
                    }
                }
                if (cands.Length == 0)
                    throw new Refusal(target == null && type.GetMethods(BindingFlags.Public | BindingFlags.Instance).Any(m => m.Name == name)
                        ? $"'{type.FullName}.{name}' is an instance member and the variable holds no object"
                        : $"member '{name}' not found on '{type.FullName}'");
                var (best, coerced, _) = Pick(cands, args, $"{type.FullName}.{name}");
                object? result = ((MethodInfo)best).Invoke(((MethodInfo)best).IsStatic ? null : target, coerced);
                return Done(result, best, coerced, args, ((MethodInfo)best).ReturnType);
            }
            case "get":
            {
                var (target, type) = Target(req["target"]!);
                string name = req["member"]!.GetValue<string>();
                var flags = BindingFlags.Public | BindingFlags.Static | BindingFlags.FlattenHierarchy | (target != null ? BindingFlags.Instance : 0);
                var prop = type.GetProperty(name, flags, null, null, Type.EmptyTypes, null);
                if (prop != null && prop.GetMethod != null)
                    return Done(prop.GetValue(prop.GetMethod.IsStatic ? null : target), null, null, null, prop.PropertyType);
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
                string name = req["member"]!.GetValue<string>();
                var value = marshal.Decode(req["value"], 0);
                var flags = BindingFlags.Public | BindingFlags.Static | BindingFlags.FlattenHierarchy | (target != null ? BindingFlags.Instance : 0);
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

    Stream StreamOf(long handle) =>
        marshal.Handle(handle) as Stream
        ?? throw new Refusal($"'{marshal.Handle(handle).GetType().FullName}' is not a System.IO.Stream");

    // ---- replies ----

    static JsonObject Ok(JsonNode? value) => new() { ["ok"] = true, ["value"] = value };

    JsonObject Done(object? result, MethodBase? method, object?[]? coerced, object?[]? args, Type? declared = null)
    {
        // A null whose declared type is string is tagged "ns" (ADR-024): AL reads it as an empty text.
        var reply = Ok(result == null && declared == typeof(string) ? new JsonObject { ["t"] = "ns" } : marshal.Encode(result));
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
    (MethodBase Best, object?[] Coerced, int Score) Pick(IEnumerable<MethodBase> cands, object?[] args, string what)
    {
        MethodBase? best = null;
        var bestScore = int.MaxValue;
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
                var s = Cost(args[i], pt);
                if (s < 0) ok = false; else score += s;
            }
            if (ok && score < bestScore) { best = c; bestScore = score; }
        }
        if (best == null)
            throw new Refusal($"{what} has no overload taking {args.Length} argument(s) of the given types ({string.Join(", ", args.Select(Describe))})");
        var ps2 = best.GetParameters();
        var coerced = new object?[args.Length];
        for (int i = 0; i < args.Length; i++)
        {
            var pt = ps2[i].ParameterType;
            if (pt.IsByRef) pt = pt.GetElementType()!;
            coerced[i] = Coerce(args[i], pt)?.Value;
        }
        return (best, coerced, bestScore);
    }

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
        try { return new Boxed(Convert.ChangeType(a, pt, CultureInfo.InvariantCulture)); }
        catch (Exception ex) when (ex is InvalidCastException or OverflowException or FormatException) { return null; }
    }
}
