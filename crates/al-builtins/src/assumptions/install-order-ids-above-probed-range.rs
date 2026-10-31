InstallOrderIdsAboveProbedRange => {
    id: "install-order-ids-above-probed-range",
    assumed: "An install routine of an app whose codeunit id is above 99999 runs in the same ascending integer order of codeunit ids as the pinned ids up to 99999 (ADR-046 amendment 3). The probe could not publish ids above 99999 on the sandbox, so nothing pins that range.",
    tests: "No Microsoft test: a partner app whose install codeunits have ids in the millions (the owner's partner measurement).",
    probe: "Install codeunits of one app with ids such as 60100, 70000 and 130000 (and one in the millions) sharing a log table and a SingleInstance codeunit, as in UrBlockOrderTests.al; it needs a BC environment that accepts ids above 99999 (the sandbox refused to publish id 120000).",
}
