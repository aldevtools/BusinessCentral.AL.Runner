//! An `HttpContent` no method was called on stays shared when it is assigned, passed by value, set on a request or handed to
//! `HttpClient.Post`: the next variable assigned from it shares it too, and a method on one of them shows on all. The mark that
//! `HttpContent.ReadAs(SecretText)` keeps on such a content (`http::content_handed_on`) changes none of this. Expected values
//! are main's results before the mark (reviewer probe app http3 of PR 7926, cells M01-M23).

const SRC: &str = r#"
codeunit 50193 C
{
    local procedure Rd(var Ct: HttpContent): Text
    var
        T: Text;
    begin
        if not Ct.ReadAs(T) then
            exit('<false>');
        exit('[' + T + ']');
    end;

    local procedure Nop(Ct: HttpContent)
    begin
    end;

    local procedure Wr(Ct: HttpContent; V: Text)
    begin
        Ct.WriteFrom(V);
    end;

    [TryFunction]
    local procedure TryPost(var Ct: HttpContent)
    var
        Cl: HttpClient;
        Resp: HttpResponseMessage;
    begin
        Cl.Post('http://example.invalid/a', Ct, Resp);
    end;

    procedure AssignedTwice(): Text
    var
        Ct: HttpContent;
        C2: HttpContent;
        C3: HttpContent;
    begin
        C2 := Ct;
        C3 := Ct;
        Ct.WriteFrom('x');
        exit(Rd(Ct) + Rd(C2) + Rd(C3));
    end;

    procedure NopThenWriteByValue(): Text
    var
        Ct: HttpContent;
    begin
        Nop(Ct);
        Wr(Ct, 'x');
        exit(Rd(Ct));
    end;

    procedure WriteByValue(): Text
    var
        Ct: HttpContent;
    begin
        Wr(Ct, 'x');
        exit(Rd(Ct));
    end;

    procedure SetOnRequestThenAssign(): Text
    var
        Req: HttpRequestMessage;
        Ct: HttpContent;
        C2: HttpContent;
    begin
        Req.Content(Ct);
        C2 := Ct;
        Ct.WriteFrom('x');
        exit(Rd(Ct) + Rd(C2));
    end;

    procedure PostThenAssign(): Text
    var
        Ct: HttpContent;
        C2: HttpContent;
    begin
        if TryPost(Ct) then;
        C2 := Ct;
        Ct.WriteFrom('x');
        exit(Rd(Ct) + Rd(C2));
    end;

    procedure AssignSameTwiceWriteCopy(): Text
    var
        Ct: HttpContent;
        C2: HttpContent;
    begin
        C2 := Ct;
        C2 := Ct;
        C2.WriteFrom('q');
        exit(Rd(Ct) + Rd(C2));
    end;

    procedure AssignedTwiceStream(): Text
    var
        Ct: HttpContent;
        C2: HttpContent;
        I: InStream;
        T: Text;
    begin
        C2 := Ct;
        C2 := Ct;
        Ct.WriteFrom('x');
        C2.ReadAs(I);
        I.ReadText(T);
        exit('[' + T + ']');
    end;

    procedure ThroughAVariant(): Text
    var
        Ct: HttpContent;
        C2: HttpContent;
        V: Variant;
    begin
        V := Ct;
        C2 := V;
        Ct.WriteFrom('x');
        exit(Rd(Ct) + Rd(C2));
    end;

    procedure AssignWriteAssign(): Text
    var
        Ct: HttpContent;
        C2: HttpContent;
        C3: HttpContent;
    begin
        C2 := Ct;
        C2.WriteFrom('y');
        C3 := Ct;
        Ct.WriteFrom('z');
        exit(Rd(Ct) + Rd(C2) + Rd(C3));
    end;
}
"#;

fn run(routine: &str) -> String {
    al_testkit::text(al_testkit::run_in(SRC, "C", routine))
}

#[test]
fn an_untouched_content_stays_shared_when_assigned_more_than_once() {
    // http3 M01, M16, M23.
    assert_eq!(run("AssignedTwice"), "[x][x][x]");
    assert_eq!(run("AssignSameTwiceWriteCopy"), "[q][q]");
    assert_eq!(run("AssignedTwiceStream"), "[x]");
}

#[test]
fn an_untouched_content_passed_by_value_stays_shared() {
    // http3 M02, M03.
    assert_eq!(run("NopThenWriteByValue"), "[x]");
    assert_eq!(run("WriteByValue"), "[x]");
}

#[test]
fn an_untouched_content_set_on_a_request_or_posted_stays_shared() {
    // http3 M11, M12.
    assert_eq!(run("SetOnRequestThenAssign"), "[x][x]");
    assert_eq!(run("PostThenAssign"), "[x][x]");
}

#[test]
fn the_neighbors_main_already_answered_keep_their_results() {
    // http3 M22 (through a Variant) and M04 (a method on the shared content, then a copy).
    assert_eq!(run("ThroughAVariant"), "[x][x]");
    assert_eq!(run("AssignWriteAssign"), "[z][z][y]");
}
