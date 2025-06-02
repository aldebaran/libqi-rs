#[qi::object]
trait MyObject {
    fn my_method(&self, a: i32, b: Vec<f64>) -> String;
}

#[test]
fn test_object() {
    let meta = MyObject::meta_object();
}
