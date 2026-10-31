GZipStreamCloseIsDispose => {
    id: "gzipstream-close-is-dispose",
    assumed: "DotNet System.IO.Compression.GZipStream.Close of a compressing stream does what Dispose does: it writes the compressed bytes to the wrapped stream. Measured on Windows BC of the newest major (probe cu 61289 SC_GZipStream_Close_ConfigPackageShape: no error, the file starts 1F 8B 08 00, Data Compression.GZipDecompress gives the source back).",
    tests: "Every test of Microsoft's Legacy test lane on a release whose artifact has no RapidStart package: the DemoTool's Export RapidStart Packages step compresses each package (Config. Pckg. Compression Mgt.ProcessGZip, ConfigPckgCompressionMgt.Codeunit.al:80), and they pass in Microsoft's CI.",
    probe: "A corpus test of GZipStream(OutStream, Compress), CopyStream, Close on the newest major, then GZipDecompress of the result.",
}
