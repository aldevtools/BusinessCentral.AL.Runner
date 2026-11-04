GetUrlClientHostPort => {
    id: "geturl-client-host-port",
    assumed: "The host name of GetUrl(ClientType::Default), Windows, Phone, Tablet and Desktop is the host name of the server base address (--server-url, ADR-042). Default and Windows use port 7046 (BC's default client services port); Phone, Tablet and Desktop keep the port of the base address. Pinned by the corpus and not part of the assumption: the schemes dynamicsnav and ms-dynamicsnav, the instance path, the company and object parts (metadata/session-system/TGetUrl.al:203-285, session-system/session/GetUrlShape.al:270), and, on corpus branch al-runtime-rs-staging-p6924c, that Default has the Web host name and a port other than the Web port, 7047 and 7048 (GetUrlShape.al GU6924_ServiceForms_ShareWebHost `default`).",
    tests: "No Microsoft test needs the host part; it is the address a test sees when it prints or compares the full text.",
    probe: "Show the port of GetUrl(ClientType::Default) itself (expected 7046) and the host and port of GetUrl(ClientType::Phone, Tablet, Desktop) relative to GetUrl(ClientType::Web), as GetUrlShape.al GuRelative does.",
}
