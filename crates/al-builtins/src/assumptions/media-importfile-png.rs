MediaImportFilePng => {
    id: "media-importfile-png",
    assumed: "Media.ImportFile of a `.png` file that holds a PNG imports it as Media.ImportStream does a PNG (`image/png`, the picture's size) and its Tenant Media File Name is the path as given, as the corpus pins for a `.jpg` file holding a JPEG (onprem media/MediaFiles.al:293-340). For an upper-case `.PNG` file the corpus pins the Mime Type, the size and that the File Name contains the file's name (MediaFiles.al:394-447), not the File Name itself; a lower-case `.png` into a MediaSet is assumed to import as a `.PNG` does.",
    tests: "Every test of Microsoft's Legacy test lane (for example Tests-SCM-Warehouse codeunit 137015): their company is built by the DemoTool, whose Create Getting Started Data imports the WalkMeTour `.png` files through Media Resources Mgt.InsertMediaFromFile (Base App MediaResourcesMgt.Codeunit.al:34), and they pass in Microsoft's CI.",
    probe: "Media.ImportFile and MediaSet.ImportFile of a lower-case `.png` file holding a PNG (and a `.jpeg` file): Tenant Media Mime Type, Width, Height; the exact File Name of a `.PNG` Media import on Windows BC.",
}
