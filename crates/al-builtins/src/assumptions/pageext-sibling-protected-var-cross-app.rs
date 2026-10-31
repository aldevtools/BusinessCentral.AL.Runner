PageExtSiblingProtectedVarCrossApp => {
    id: "pageext-sibling-protected-var-cross-app",
    assumed: "The same holds when the declaring page extension is in another app that the using extension's app depends on (directly or through other apps), and that extension's OnOpenPage runs first.",
    tests: "No Microsoft test: a partner suite of three apps on real BC (base page, an extension that declares the variable, an extension that uses it). Corpus writer UE observed it on real BC (the cross-app form compiled and shared the variable) but left it out of the corpus: a dependency app change forces a full CI run.",
    probe: "UeProtectedVarTests.al with the declaring extension in a dependency app of the corpus (an xapp fixture app) and the using one in the main app: the trace and both Visible values.",
}
