GetUrlCurrentIsWeb => {
    id: "geturl-current-is-web",
    assumed: "GetUrl(ClientType::Current) in a test session with a company or a record answers what GetUrl(ClientType::Web) answers with the same arguments. Pinned and not part of the assumption: Current alone and with a page equals Web (session-system/session/GetUrlShape.al:424, 429, GU6924_Current_EqualsWhichForm).",
    tests: "Microsoft and partner code that asks GetUrl(ClientType::Current) for a link.",
    probe: "Extend GetUrlShape.al GU6924_Current_EqualsWhichForm with Current against Web for a company and for a page with a record (UseFilters false and true).",
}
