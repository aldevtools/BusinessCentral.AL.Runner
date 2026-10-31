MediaExportStreamJfif300DpiKept => {
    id: "media-exportstream-jfif-dpi-jpeg-kept",
    assumed: "From the newest major on (the same major where MediaSet keeps a JPEG), Media.ExportStream of a JPEG that Media.ImportFile read, whose JFIF header has units 1 (dots per inch) and density 300 x 300, writes the file's bytes unchanged. Measured on Windows BC of the newest major (probe cu 61288 SC_DemoTool_Media_ExportStream, case JD: export = file, Tenant Media Content = file). Other JPEG headers are rewritten there (units 0: three header bytes; no JFIF: a JFIF segment inserted) and stay refused.",
    tests: "Every test of Microsoft's Legacy test lane on a release whose artifact has no RapidStart package: the DemoTool's Export RapidStart Packages step exports the Person, Contact, Customer, Vendor, Employee and Salesperson pictures (Config. XML Exchange.ExportMedia, ConfigXMLExchange.Codeunit.al:1486), all 300 dpi JFIF JPEGs, and they pass in Microsoft's CI.",
    probe: "A corpus test of Media.ImportFile then Media.ExportStream of a 300 dpi JFIF JPEG on the newest major (and the two older majors), asserting the exported bytes equal the file.",
}
