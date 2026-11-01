//! `Xmlport.Run` (issue 28): where a run without a stream writes (a file on the service tier a test cannot read) is not
//! pinned, so a run of a port without a request page is refused with a reason.

#[test]
fn xmlport_run_is_refused_with_its_reason() {
    let src = "xmlport 63100 X { schema { textelement(Root) { } } }
         codeunit 63101 T { procedure Static() begin Xmlport.Run(63100, false, false); end;
                            procedure Inst() var P: XmlPort X; begin P.Run(); end; }";
    let l = al_testkit::compile(src);
    let oi = l.program.objects.iter().position(|o| o.name == "T").unwrap();
    // The static form raises its errors when it runs (branch ms-top-refusals-1). The instance form lowers since corpus master
    // ac3499d6 pins it for an export port with a request page (xmlport/lt-y40-xmlport-request-page/Y40XmlPortRequestPage.al:398);
    // on a port without one it is refused when it runs (al-vm xmlports/run_request.rs).
    let ri = l.program.objects[oi]
        .routines
        .iter()
        .position(|r| r.name == "Inst")
        .unwrap();
    assert!(l.unsupported[oi][ri].is_empty(), "Inst: {:?}", l.unsupported[oi][ri]);
    match al_testkit::run_in(src, "T", "Inst") {
        Err(al_builtins::native::Signal::Unsupported(why)) => {
            assert!(why.contains("Xmlport.Run") && why.contains("not pinned"), "Inst: {why}")
        }
        other => panic!("Inst: {other:?}"),
    }
}
