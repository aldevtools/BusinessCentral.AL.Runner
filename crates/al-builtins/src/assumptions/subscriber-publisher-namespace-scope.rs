SubscriberPublisherNamespaceScope => {
    id: "subscriber-publisher-namespace-scope",
    assumed: "An `[EventSubscriber]` whose publisher name (kind and name, or id) more than one object visible from the subscriber's app has subscribes to the one object the subscriber's file names (al-sema `Workspace::subscriber_publisher`), and the event of every other of those objects does not reach it. Only three shapes pick an object: a qualified name (`Codeunit::Ns.\"Name\"`) that one of them has in that namespace; a plain name in a file with a namespace that one of them has as its namespace (whatever the `using`s say); a plain name in a file with a namespace that none of them has, whose `using`s name exactly one of their namespaces. Every other shape binds the subscriber to none of them and refuses the raise of each one's event: a file without a namespace (alc ignores its `using`s, AL0789), a `using` of two of their namespaces, no `using` of theirs, two of them in the picked namespace, an id that two objects have. A name one visible object has keeps the binding of that object, unmarked. The mark is raised wherever the program asks for the subscribers of an event of one of those objects.",
    tests: "No Microsoft test. An outside report: the reporter's alc 18.1 compiles `Codeunit::\"Shared Name\"` through `using Repro.Copy;` to codeunit 51655 and compiles the qualified `Codeunit::Repro.Copy.\"Shared Name\"` (al-runner issue 8171). The AL compilers of BC 28 and 29 locally compile the qualified, own-namespace and `using` files of corpus PR 1423, and reject a file without a namespace and a file with a `using` of both namespaces (AL0275) and a file with a namespace and no `using` (AL0118).",
    probe: "Corpus PR 1423: events/subscriber-namespace/SnUsingTests.al (plain name through a `using`, each qualified name), SnOwnNamespaceTests.al (the file's own namespace), SnOwnOverUsingTests.al (own namespace and a `using` of the other twin's namespace); each test raises each twin's event and records which subscribers ran. Its observe-form CI run 43805337922 (BC 27.5, 28.5, 29.0) measured what this entry assumes: through the `using` the plain subscriber runs on the Beta twin only, each qualified one on its twin only; through the own namespace, with or without a `using` of the other namespace, on the Alpha twin only. Must-not-compile 3171 to 3173 (AL0275, AL0275, AL0118). Wanted next: a twin without a namespace against a namespaced one (the no-namespace tier), a twin a page or table event names, a subscriber in a dependency app that is not compiled.",
    awaiting: Awaiting {
        issues: &[8171],
        files: &[
            "tests/al-language/events/subscriber-namespace/SnUsingTests.al",
            "tests/al-language/events/subscriber-namespace/SnOwnNamespaceTests.al",
            "tests/al-language/events/subscriber-namespace/SnOwnOverUsingTests.al",
        ],
        since: "2026-11-16",
    },
}
