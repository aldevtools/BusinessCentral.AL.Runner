// The .NET helper process of ADR-014 (DotNet provider seam).
//
// Protocol: strictly one request, one reply, in order, over stdin and stdout. Each message is a
// frame: a 4-byte little-endian length, then that many bytes of UTF-8 JSON. Stdout carries
// protocol bytes only; anything the .NET code prints goes to stderr. The format is internal to
// this process and the Rust `SidecarHost`; the first message is `hello`, which checks the version.
//
// Values are tagged objects, see Marshal.cs. Dispatch is reflection by name, with an overload
// pick on the runtime types of the arguments, as BC's NavAutomationHelper.GetMethodInfo does it
// (see Dispatch.cs).
using System.Globalization;
using System.Text;
using System.Text.Json.Nodes;

namespace AlDotNetSidecar;

public static class Program
{
    public const int ProtocolVersion = 3;

    public static int Main(string[] args)
    {
        // The oracle's invariant (ADR-014 section 9): culture en-US. The time zone (UTC) and the
        // cleared environment come from the parent when it starts this process.
        var culture = new CultureInfo("en-US");
        CultureInfo.DefaultThreadCurrentCulture = culture;
        CultureInfo.DefaultThreadCurrentUICulture = culture;
        CultureInfo.CurrentCulture = culture;
        CultureInfo.CurrentUICulture = culture;
        // BC's server answers the Windows and DOS code pages (1252, 850, 437) by number and by name; .NET 8 has them only
        // through this provider (part of the shared framework, no NuGet). Pinned by corpus
        // tests/al-language-onprem-dotnet/lt-ra-dotnet/RaEncodingCodePages.al:115, :131, :244.
        Encoding.RegisterProvider(CodePagesEncodingProvider.Instance);

        var protocolIn = Console.OpenStandardInput();
        var protocolOut = Console.OpenStandardOutput();
        // Stray Console.Write calls from .NET code under test must not corrupt the protocol.
        Console.SetOut(Console.Error);

        var server = new Server();
        while (true)
        {
            byte[]? frame = ReadFrame(protocolIn);
            if (frame == null) return 0;
            JsonObject resp;
            long id = 0;
            bool bye = false;
            try
            {
                var req = JsonNode.Parse(Encoding.UTF8.GetString(frame))!.AsObject();
                id = req["id"]?.GetValue<long>() ?? 0;
                bye = req["op"]?.GetValue<string>() == "bye";
                resp = server.Handle(req);
            }
            catch (Exception ex)
            {
                resp = Server.Failure(ex);
            }
            resp["id"] = id;
            WriteFrame(protocolOut, Encoding.UTF8.GetBytes(resp.ToJsonString()));
            if (bye) return 0;
        }
    }

    static byte[]? ReadFrame(Stream s)
    {
        var head = new byte[4];
        if (!ReadExactly(s, head)) return null;
        int len = BitConverter.ToInt32(head, 0);
        var body = new byte[len];
        return ReadExactly(s, body) ? body : null;
    }

    static bool ReadExactly(Stream s, byte[] buf)
    {
        int off = 0;
        while (off < buf.Length)
        {
            int n = s.Read(buf, off, buf.Length - off);
            if (n <= 0) return false;
            off += n;
        }
        return true;
    }

    static void WriteFrame(Stream s, byte[] body)
    {
        s.Write(BitConverter.GetBytes(body.Length));
        s.Write(body);
        s.Flush();
    }
}
