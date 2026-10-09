#[qi_macros::object]
trait MutSelf {
    #[qi::property]
    fn position(&mut self) -> i32;
}

#[qi_macros::object]
trait NoReceiver {
    #[qi::method]
    fn bar(position: i32);
}

#[qi_macros::object]
trait ByValue {
    #[qi::signal]
    fn moving(self) -> bool;
}

#[qi_macros::object]
trait Generic<T> {}

#[qi_macros::object]
trait UnknownAttr {
    #[qi::bogus]
    fn bar(&self);
}

#[qi_macros::object]
trait UnknownArg {
    #[qi::method(bogus = "x")]
    fn bar(&self);
}

#[qi_macros::object]
trait NameNotString {
    #[qi::method(name = 1)]
    fn bar(&self);
}

#[qi_macros::object]
trait TaggedType {
    #[qi::method]
    type Foo;
}

#[qi_macros::object]
trait NameEquals {
    #[qi::method = "goTo"]
    fn bar(&self);
}

fn main() {}
