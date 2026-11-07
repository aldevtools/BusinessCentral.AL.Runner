GetUrlClientHostPort => {
    id: "geturl-client-host-port",
    assumed: "The host name and port of GetUrl(ClientType::Phone, Tablet, Desktop) are those of the server base address (--server-url, ADR-042). Pinned by the corpus and not part of the assumption: the scheme ms-dynamicsnav, the instance path, the company and object parts (metadata/session-system/TGetUrl.al:248-285), and the Web host name with port 7085 for Default and Windows (session-system/session/GetUrlShape.al:465-468).",
    tests: "No Microsoft test needs the host part; it is the address a test sees when it prints or compares the full text.",
    probe: "Show the host and port of GetUrl(ClientType::Phone, Tablet, Desktop) relative to GetUrl(ClientType::Web), as GetUrlShape.al GuPort does.",
}
