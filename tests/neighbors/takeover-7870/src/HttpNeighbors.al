// The shapes next to the two HTTP cells of corpus PR 1424 (runner PR 7870): HttpHeaders.Contains('') and
// HttpContent.WriteFrom(InStream) after the Content-Type changed. The Measured_ tests repeat a probe cell; every other test
// flips one fact of it. Each test asserts what the probe's cell suggests; a test the runner refuses is not run.
table 62700 "HN Blob"
{
    TableType = Temporary;
    fields
    {
        field(1; Id; Integer) { }
        field(2; Data; Blob) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

codeunit 62701 "HN Contains Tests"
{
    Subtype = Test;
    TestHttpRequestPolicy = AllowOutboundFromHandler;

    local procedure ExpectEmptyNameError(var H: HttpHeaders)
    var
        Found: Boolean;
    begin
        ClearLastError();
        asserterror Found := H.Contains('');
        if GetLastErrorCode() <> 'InvalidHeader:Argument' then
            Error('code %1', GetLastErrorCode());
    end;

    [Test]
    procedure Measured_FreshRequest()
    var
        R: HttpRequestMessage;
        H: HttpHeaders;
    begin
        R.GetHeaders(H);
        ExpectEmptyNameError(H);
    end;

    [Test]
    procedure Measured_FreshContent()
    var
        C: HttpContent;
        H: HttpHeaders;
    begin
        C.GetHeaders(H);
        ExpectEmptyNameError(H);
    end;

    [Test]
    procedure Measured_RequestContentAfterTextWrite()
    var
        R: HttpRequestMessage;
        H: HttpHeaders;
    begin
        R.Content.WriteFrom('body');
        R.Content.GetHeaders(H);
        ExpectEmptyNameError(H);
    end;

    [Test]
    procedure StandaloneContentAfterTextWrite()
    var
        C: HttpContent;
        H: HttpHeaders;
    begin
        C.WriteFrom('body');
        C.GetHeaders(H);
        ExpectEmptyNameError(H);
    end;

    [Test]
    procedure ResponseContentAfterTextWrite()
    var
        Resp: HttpResponseMessage;
        H: HttpHeaders;
    begin
        Resp.Content.WriteFrom('x');
        Resp.Content.GetHeaders(H);
        ExpectEmptyNameError(H);
    end;

    [Test]
    [HandlerFunctions('WritingHandler')]
    procedure HandlerWrittenResponseContent()
    var
        Client: HttpClient;
        Resp: HttpResponseMessage;
        H: HttpHeaders;
    begin
        Client.Get('http://localhost/a', Resp);
        Resp.Content.GetHeaders(H);
        ExpectEmptyNameError(H);
    end;

    [Test]
    [HandlerFunctions('PlainHandler')]
    procedure HandlerResponseHeadersUntouched()
    var
        Client: HttpClient;
        Resp: HttpResponseMessage;
        H: HttpHeaders;
    begin
        Client.Get('http://localhost/a', Resp);
        H := Resp.Headers();
        ExpectEmptyNameError(H);
    end;

    [Test]
    [HandlerFunctions('PlainHandler')]
    procedure HandlerResponseContentStreamWrite()
    var
        Client: HttpClient;
        Resp: HttpResponseMessage;
        B: Record "HN Blob";
        HNStreams: Codeunit "HN Streams";
        InS: InStream;
    begin
        Client.Get('http://localhost/a', Resp);
        HNStreams.Fill(B, InS);
        Resp.Content.WriteFrom(InS);
    end;

    [HttpClientHandler]
    procedure WritingHandler(Request: TestHttpRequestMessage; var Response: TestHttpResponseMessage): Boolean
    begin
        Response.HttpStatusCode := 200;
        Response.Content.WriteFrom('x');
        exit(false);
    end;

    [HttpClientHandler]
    procedure PlainHandler(Request: TestHttpRequestMessage; var Response: TestHttpResponseMessage): Boolean
    begin
        Response.HttpStatusCode := 200;
        exit(false);
    end;

    [Test]
    procedure RequestAfterAddRemove()
    var
        R: HttpRequestMessage;
        H: HttpHeaders;
    begin
        R.GetHeaders(H);
        H.Add('X-A', 'v');
        H.Remove('X-A');
        ExpectEmptyNameError(H);
    end;

    [Test]
    procedure ClientAfterAddClear()
    var
        Client: HttpClient;
        H: HttpHeaders;
    begin
        H := Client.DefaultRequestHeaders();
        H.Add('X-A', 'v');
        H.Clear();
        ExpectEmptyNameError(H);
    end;

    [Test]
    procedure ContentTypeRemovedAndReadded()
    var
        C: HttpContent;
        H: HttpHeaders;
    begin
        C.GetHeaders(H);
        H.Remove('Content-Type');
        H.Add('Content-Type', 'text/plain; charset=utf-8');
        ExpectEmptyNameError(H);
    end;

    [Test]
    procedure ContentAfterSecondTextWrite()
    var
        C: HttpContent;
        H: HttpHeaders;
    begin
        C.WriteFrom('one');
        C.WriteFrom('two');
        C.GetHeaders(H);
        ExpectEmptyNameError(H);
    end;

    [Test]
    procedure ContentAfterStreamWrite()
    var
        C: HttpContent;
        H: HttpHeaders;
        B: Record "HN Blob";
        InS: InStream;
    begin
        HNStreams.Fill(B, InS);
        C.WriteFrom(InS);
        C.GetHeaders(H);
        ExpectEmptyNameError(H);
    end;

    [Test]
    procedure ContentAfterClear()
    var
        C: HttpContent;
        H: HttpHeaders;
    begin
        C.WriteFrom('one');
        C.Clear();
        C.GetHeaders(H);
        ExpectEmptyNameError(H);
    end;

    [Test]
    procedure CopyOfWrittenContent()
    var
        C: HttpContent;
        Copy: HttpContent;
        H: HttpHeaders;
    begin
        C.WriteFrom('one');
        Copy := C;
        Copy.GetHeaders(H);
        ExpectEmptyNameError(H);
    end;

    [Test]
    procedure Measured_FreshResponseContent()
    var
        R: HttpResponseMessage;
        H: HttpHeaders;
    begin
        R.Content.GetHeaders(H);
        ExpectEmptyNameError(H);
    end;

    [Test]
    procedure RequestContentAfterReadAsStream()
    var
        R: HttpRequestMessage;
        H: HttpHeaders;
        InS: InStream;
    begin
        R.Content.WriteFrom('body');
        R.Content.ReadAs(InS);
        R.Content.GetHeaders(H);
        ExpectEmptyNameError(H);
    end;

    [Test]
    procedure FreshContentAfterReadAsStream()
    var
        C: HttpContent;
        H: HttpHeaders;
        InS: InStream;
    begin
        C.ReadAs(InS);
        C.GetHeaders(H);
        ExpectEmptyNameError(H);
    end;

    [Test]
    procedure ResponseContentAfterReadAsStream()
    var
        R: HttpResponseMessage;
        H: HttpHeaders;
        InS: InStream;
    begin
        R.Content.ReadAs(InS);
        R.Content.GetHeaders(H);
        ExpectEmptyNameError(H);
    end;

    [Test]
    procedure RequestContentAfterReadAsSecret()
    var
        R: HttpRequestMessage;
        H: HttpHeaders;
        S: SecretText;
    begin
        R.Content.WriteFrom('body');
        if R.Content.ReadAs(S) then;
        R.Content.GetHeaders(H);
        ExpectEmptyNameError(H);
    end;

    [Test]
    procedure NonEmptyNameOnFreshRequest()
    var
        R: HttpRequestMessage;
        H: HttpHeaders;
    begin
        R.GetHeaders(H);
        if H.Contains('X-A') then
            Error('X-A found');
    end;

    var
        HNStreams: Codeunit "HN Streams";
}

codeunit 62702 "HN Streams"
{
    procedure Fill(var B: Record "HN Blob"; var InS: InStream)
    var
        OutS: OutStream;
    begin
        B.Init();
        B.Id := 1;
        B.Data.CreateOutStream(OutS);
        OutS.WriteText('stream body');
        B.Insert();
        B.Data.CreateInStream(InS);
    end;

    procedure TypeOf(var C: HttpContent): Text
    var
        H: HttpHeaders;
        Values: array[10] of Text;
    begin
        C.GetHeaders(H);
        if not H.Contains('Content-Type') then
            exit('<absent>');
        H.GetValues('Content-Type', Values);
        exit(Values[1]);
    end;
}

codeunit 62703 "HN Stream Tests"
{
    Subtype = Test;

    var
        HNStreams: Codeunit "HN Streams";

    local procedure Expect(var C: HttpContent; Want: Text)
    begin
        if HNStreams.TypeOf(C) <> Want then
            Error('type %1, want %2', HNStreams.TypeOf(C), Want);
    end;

    [Test]
    procedure Measured_FreshRemoved()
    var
        C: HttpContent;
        H: HttpHeaders;
        B: Record "HN Blob";
        InS: InStream;
    begin
        C.GetHeaders(H);
        H.Remove('Content-Type');
        HNStreams.Fill(B, InS);
        C.WriteFrom(InS);
        Expect(C, '<absent>');
    end;

    [Test]
    procedure Measured_CopyRemoved()
    var
        C: HttpContent;
        Copy: HttpContent;
        H: HttpHeaders;
        B: Record "HN Blob";
        InS: InStream;
    begin
        C.GetHeaders(H);
        H.Remove('Content-Type');
        Copy := C;
        HNStreams.Fill(B, InS);
        Copy.WriteFrom(InS);
        Expect(Copy, '<absent>');
    end;

    [Test]
    procedure Measured_FreshDefault()
    var
        C: HttpContent;
        B: Record "HN Blob";
        InS: InStream;
    begin
        HNStreams.Fill(B, InS);
        C.WriteFrom(InS);
        Expect(C, 'text/plain; charset=utf-8');
    end;

    [Test]
    procedure ReadAsBeforeStreamWrite()
    var
        C: HttpContent;
        H: HttpHeaders;
        B: Record "HN Blob";
        InS: InStream;
        T: Text;
    begin
        C.WriteFrom('x');
        C.ReadAs(T);
        C.GetHeaders(H);
        H.Clear();
        HNStreams.Fill(B, InS);
        C.WriteFrom(InS);
        Expect(C, '<absent>');
    end;

    [Test]
    procedure PartlyReadStream()
    var
        C: HttpContent;
        H: HttpHeaders;
        B: Record "HN Blob";
        InS: InStream;
        T: Text;
    begin
        C.GetHeaders(H);
        H.Remove('Content-Type');
        HNStreams.Fill(B, InS);
        InS.ReadText(T, 3);
        C.WriteFrom(InS);
        Expect(C, '<absent>');
    end;

    [Test]
    procedure StreamNotOnBlobField()
    var
        C: HttpContent;
        Source: HttpContent;
        H: HttpHeaders;
        InS: InStream;
    begin
        Source.WriteFrom('stream body');
        Source.ReadAs(InS);
        C.GetHeaders(H);
        H.Remove('Content-Type');
        C.WriteFrom(InS);
        Expect(C, '<absent>');
    end;

    [Test]
    procedure SecondStreamWrite()
    var
        C: HttpContent;
        H: HttpHeaders;
        B: Record "HN Blob";
        B2: Record "HN Blob";
        InS: InStream;
        InS2: InStream;
    begin
        C.GetHeaders(H);
        H.Remove('Content-Type');
        HNStreams.Fill(B, InS);
        C.WriteFrom(InS);
        HNStreams.Fill(B2, InS2);
        C.WriteFrom(InS2);
        Expect(C, '<absent>');
    end;

    [Test]
    procedure JsonWithCharset()
    var
        C: HttpContent;
        H: HttpHeaders;
        B: Record "HN Blob";
        InS: InStream;
    begin
        C.WriteFrom('x');
        C.GetHeaders(H);
        H.Remove('Content-Type');
        H.Add('Content-Type', 'application/json; charset=utf-8');
        HNStreams.Fill(B, InS);
        C.WriteFrom(InS);
        Expect(C, 'application/json; charset=utf-8');
    end;

    [Test]
    procedure Measured_JsonReadded()
    var
        C: HttpContent;
        H: HttpHeaders;
        B: Record "HN Blob";
        InS: InStream;
    begin
        C.WriteFrom('x');
        C.GetHeaders(H);
        H.Remove('Content-Type');
        H.Add('Content-Type', 'application/json');
        HNStreams.Fill(B, InS);
        C.WriteFrom(InS);
        Expect(C, 'application/json');
    end;

    [Test]
    procedure RemovedThenContentClear()
    var
        C: HttpContent;
        H: HttpHeaders;
        B: Record "HN Blob";
        InS: InStream;
    begin
        C.GetHeaders(H);
        H.Remove('Content-Type');
        C.Clear();
        HNStreams.Fill(B, InS);
        C.WriteFrom(InS);
        Expect(C, 'text/plain; charset=utf-8');
    end;
}
