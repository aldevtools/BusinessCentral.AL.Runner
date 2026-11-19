CurrpageCaptionOwnProcedure => {
    id: "currpage-caption-own-procedure",
    assumed: "Beside one page procedure named Caption, BC names the platform method Page.Caption in these forms only: on a Card page with the procedure and the code in the page, CurrPage.Caption and CurrPage.Caption() read in OnOpenPage, CurrPage.Caption read in OnAction, CurrPage.Caption := in OnOpenPage, CurrPage.Caption('X') and the bare Caption('X') as statements in OnAction, for a procedure Caption(): Text (public, local, protected, internal), Caption(): Integer, Caption() (public, local) and Caption(NewCaption: Text); on a Card page with a public Caption(): Text split between the page and a page extension (the procedure in the page with the code in an extension, the procedure in an extension with the code in the page, or both in one extension), the reads and := in OnOpenPage, and the base page's bare Caption() in OnOpenPage beside a procedure only the extension declares; on a List page with a public Caption(): Text, the reads in OnOpenPage and := in OnAfterGetCurrRecord; in a [Test] procedure of a test codeunit, P.Caption('X') on a Page variable of a Card page that declares a public Caption(): Text or Caption(NewCaption: Text), and P.Caption() beside Caption(NewCaption: Text). Spelled CurrPage and Caption, unquoted. The bare Caption and Caption() and P.Caption and P.Caption() beside an own parameterless procedure keep calling it, as before.",
    tests: "No Microsoft test. Evidence: corpus PR 1456 (merged), measured by its CI on every pinned major (pages/lifecycle/CurrPageCaptionOwn*.al, must-not-compile 3210-3215). The page shape of al-runner issues 8193 and 8444 is Base Application's page 5911.",
    probe: "Corpus PR 1456: pages/lifecycle/CurrPageCaptionOwnPublic.al, CurrPageCaptionOwnAccess.al, CurrPageCaptionOwnSignature.al, CurrPageCaptionOwnList.al and CurrPageCaptionOwnExtension.al. Not measured and refused: other page types, report request pages, two or more procedures named Caption, other signatures, other spelling or a quoted name, a compound assignment, the setter and := in other triggers (OnInit, a field's OnValidate) or in procedures, the bare Caption := and P.Caption := beside an own procedure, and calls with two or more arguments.",
    awaiting: Awaiting {
        issues: &[8193, 8444],
        files: &[
            "tests/al-language/pages/lifecycle/CurrPageCaptionOwnPublic.al",
            "tests/al-language/pages/lifecycle/CurrPageCaptionOwnAccess.al",
            "tests/al-language/pages/lifecycle/CurrPageCaptionOwnSignature.al",
            "tests/al-language/pages/lifecycle/CurrPageCaptionOwnList.al",
            "tests/al-language/pages/lifecycle/CurrPageCaptionOwnExtension.al",
        ],
        since: "2026-11-19",
    },
}
