DotNetEnumMemberThroughProperty => {
    id: "dotnet-enum-member-through-property",
    assumed: "`X.P::M` on a DotNet variable X names the member M of the .NET enum that the property P holds. BC's compiler takes P's type from the assembly and reads the member statically. Here P is read first, for the type of its value, then M is read on that type. This differs only when reading P fails: an unassigned X is refused here (BC still answers the member), and an exception of P's getter would be raised.",
    tests: "No corpus test. System Application, Image Impl..Resize (ImageImpl.Codeunit.al:97 and :129, `Graphics.SmoothingMode::AntiAlias`), which passes on real BC (the reporter's run of Image.Resize).",
    probe: "An OnPrem test with X: DotNet XmlDocument: `X.NodeType::Element` formatted with X assigned and with X unassigned; and an enum member through a property whose getter throws.",
}
