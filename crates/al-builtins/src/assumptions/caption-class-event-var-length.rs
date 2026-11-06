CaptionClassEventVarLength => {
    id: "caption-class-event-var-length",
    assumed: "A subscriber of Caption Class.OnAfterCaptionClassResolve that reads MaxStrLen of its var Caption gets 1024, the length the publisher declares (var Caption: Text[1024]); one of OnResolveCaptionClass gets the unlimited length of its var Caption: Text. The platform raises both events, so what its own argument holds is not pinned.",
    tests: "No Microsoft test. An outside report (al-runner issue 6924): ISV event subscribers that cut their text with CopyStr(.., MaxStrLen(Caption)).",
    probe: "A corpus test with a subscriber of OnAfterCaptionClassResolve that returns MaxStrLen(Caption) into the translation, and a second one of OnResolveCaptionClass (types/caption-class-events/Y71CaptionClassEvents.al has the subscribers but never reads the length).",
}
