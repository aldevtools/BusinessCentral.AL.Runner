//! Characterization tests, written against the code before ADR-072 step A: what a `Value::TestPage` and a
//! `Value::TestControl` hold, print and count. They must pass unchanged in their assertions after the state moves.

#[cfg(test)]
mod tests {
    use al_builtins::value::{ControlRef, ControlTarget, PageState, RtType, Value};
    use std::cell::RefCell;
    use std::rc::Rc;

    fn st_of(v: &Value) -> &Rc<RefCell<PageState>> {
        match v {
            Value::TestPage(st) => st,
            other => panic!("not a TestPage: {other:?}"),
        }
    }

    fn page_value(st: &Rc<RefCell<PageState>>) -> Value {
        Value::TestPage(st.clone())
    }

    fn test_control(page: Rc<RefCell<PageState>>, target: ControlTarget) -> Value {
        Value::TestControl(Rc::new(ControlRef { page, target }))
    }

    #[test]
    fn a_declared_page_variable_holds_a_closed_page_of_its_object() {
        for ty in [RtType::TestPage(5), RtType::Page(5)] {
            let v = Value::default_for(&ty);
            let s = st_of(&v).borrow();
            assert_eq!((s.object, s.open, s.ran, s.trapping), (5, false, false, false));
            assert!(s.host.is_none() && s.instance.is_none());
        }
        // Each variable has a page of its own; an array element too.
        let (a, b) = (Value::default_for(&RtType::TestPage(5)), Value::default_for(&RtType::TestPage(5)));
        assert!(!Rc::ptr_eq(st_of(&a), st_of(&b)));
        let Value::Array(cells) = Value::default_for(&RtType::Array(vec![2], Box::new(RtType::TestPage(7)))) else {
            panic!("not an array");
        };
        let cells = cells.borrow();
        assert_eq!(st_of(&cells[0]).borrow().object, 7);
        assert!(!Rc::ptr_eq(st_of(&cells[0]), st_of(&cells[1])));
    }

    #[test]
    fn a_page_value_prints_the_facts_of_its_page_and_nothing_else() {
        // `Value::identical` compares two handles by this text.
        let v = Value::default_for(&RtType::TestPage(5));
        assert_eq!(
            format!("{v:?}"),
            "TestPage(RefCell { value: PageState(page 5, open false, mode View) })"
        );
        let st = st_of(&v).clone();
        st.borrow_mut().open = true;
        let c = test_control(st.clone(), ControlTarget::Control(3));
        assert_eq!(
            format!("{c:?}"),
            "TestControl(ControlRef { page: RefCell { value: PageState(page 5, open true, mode View) }, target: Control(3) })"
        );
        assert!(v.identical(&page_value(&st)));
        assert!(!v.identical(&Value::default_for(&RtType::TestPage(5))));
        // A page that is borrowed for writing while it is printed.
        let held = st.borrow_mut();
        assert_eq!(format!("{v:?}"), "TestPage(RefCell { value: <borrowed> })");
        drop(held);
    }

    #[test]
    fn a_page_value_holds_one_reference_to_its_page() {
        // The page client counts the references to a page (`page_held_bindings.rs`, `pages/rows.rs`).
        let v = Value::default_for(&RtType::TestPage(5));
        assert_eq!(Rc::strong_count(st_of(&v)), 1);
        let st = st_of(&v).clone();
        assert_eq!(Rc::strong_count(&st), 2);
        let copy = v.clone();
        assert_eq!(Rc::strong_count(&st), 3);
        assert!(Rc::ptr_eq(st_of(&copy), &st));
        let again = page_value(&st);
        assert_eq!(Rc::strong_count(&st), 4);
        let c = test_control(st.clone(), ControlTarget::Filter);
        assert_eq!(Rc::strong_count(&st), 5);
        let c2 = c.clone();
        // A copy of a control handle shares the handle: no new reference to the page.
        assert_eq!(Rc::strong_count(&st), 5);
        drop((v, copy, again, c));
        assert_eq!(Rc::strong_count(&st), 2);
        drop(c2);
        assert_eq!(Rc::strong_count(&st), 1);
    }
}
