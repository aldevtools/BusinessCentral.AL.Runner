//! The cookies of an `HttpRequestMessage` (xml/lt-xml-attr-doc-http-members/P97ReqCookie.al, P97ReqCookieObj.al).

use al_testkit::{run, text};

const SRC: &str = r#"
codeunit 50160 C
{
    local procedure Names(var Req: HttpRequestMessage): Text
    var
        N: Text;
        R: Text;
    begin
        foreach N in Req.GetCookieNames() do
            R += N + ';';
        exit(R);
    end;

    local procedure Get(var Req: HttpRequestMessage; Name: Text): Text
    var
        C: Cookie;
    begin
        if not Req.GetCookie(Name, C) then
            exit('No');
        exit(C.Name() + '=' + C.Value() + '|' + C.Domain() + '|' + C.Path() + '|' + Format(C.Secure()) + Format(C.HttpOnly()));
    end;

    procedure NameValue(): Text
    var
        Req: HttpRequestMessage;
        R: Text;
    begin
        R := Names(Req) + '#';
        R += Format(Req.SetCookie('a', '1')) + Format(Req.SetCookie('b', '2')) + Format(Req.SetCookie('a', '9'));
        R += '#' + Names(Req) + '#' + Get(Req, 'a');
        R += '#' + Format(Req.SetCookie('A', 'upper')) + Names(Req) + Get(Req, 'a');
        R += '#' + Format(Req.SetCookie('', 'v')) + Format(Req.SetCookie('a;b', 'v')) + Format(Req.SetCookie('$a', 'v')) + Format(Req.SetCookie('sv', 'a;b'));
        R += Format(Req.SetCookie('a b', 'v')) + Format(Req.SetCookie('ev', 'a=b')) + Get(Req, 'ev');
        exit(R);
    end;

    procedure Remove(): Text
    var
        Req: HttpRequestMessage;
    begin
        Req.SetRequestUri('https://example.invalid/');
        Req.SetCookie('a', '1');
        Req.SetCookie('b', '2');
        exit(Format(Req.RemoveCookie('a')) + Format(Req.RemoveCookie('a')) + Format(Req.RemoveCookie('B')) + Format(Req.RemoveCookie('')) + Names(Req) + Get(Req, 'a'));
    end;

    procedure Objects(): Text
    var
        Req: HttpRequestMessage;
        C: Cookie;
        Empty: Cookie;
        Back: Cookie;
        R: Text;
    begin
        C.Name('obj');
        C.Value('v1');
        R := Format(Req.SetCookie(C)) + Get(Req, 'obj');
        C.Value('v2');
        R += '#' + Get(Req, 'obj');
        Req.GetCookie('obj', Back);
        Back.Value('changed');
        R += '#' + Get(Req, 'obj') + '#' + Format(Req.SetCookie(Empty));
        exit(R);
    end;

    procedure Headers(): Text
    var
        Req: HttpRequestMessage;
        H: HttpHeaders;
    begin
        Req.SetRequestUri('https://example.invalid/');
        Req.GetHeaders(H);
        Req.SetCookie('a', '1');
        exit(Format(H.Contains('Cookie')) + Format(H.Keys().Count()) + Format(H.Add('Cookie', 'manual=1')) + Names(Req));
    end;
}
"#;

#[test]
fn set_get_and_names_follow_the_pins() {
    assert_eq!(
        text(run(SRC, "NameValue")),
        "#YesYesYes#a;b;#a=9||/|NoNo#YesA;b;No#NoNoNoNoYesYesev=a=b||/|NoNo"
    );
}

#[test]
fn remove_takes_the_exact_name() {
    assert_eq!(text(run(SRC, "Remove")), "YesNoNoNob;No");
}

#[test]
fn a_cookie_object_is_copied_in_and_out() {
    assert_eq!(
        text(run(SRC, "Objects")),
        "Yesobj=v1||/|NoNo#obj=v1||/|NoNo#obj=v1||/|NoNo#No"
    );
}

#[test]
fn the_cookies_are_the_cookie_header() {
    assert_eq!(text(run(SRC, "Headers")), "Yes1Yesa;manual;");
}
