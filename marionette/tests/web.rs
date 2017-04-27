extern crate marionette;
use marionette::*;
use marionette::messages::Script;
extern crate env_logger;

#[test]
fn script() {   
    let _ = env_logger::init();
    let mut conn = MarionetteConnection::connect(2828).unwrap();

    let script = Script::new("return 42;");
    let res = conn.execute_script(&script).unwrap();
    assert_eq!(res, JsonValue::from(42));
}

#[test]
fn script_arguments() {   
    let _ = env_logger::init();
    let mut conn = MarionetteConnection::connect(2828).unwrap();

    let mut script = Script::new("return arguments[0];");
    script.arguments(vec![84]);
    let res = conn.execute_script(&script).unwrap();
    assert_eq!(res, JsonValue::from(84));
}

#[test]
fn page_source() {
    let _ = env_logger::init();
    let mut conn = MarionetteConnection::connect(2828).unwrap();

    conn.get("https://www.youtube.com/watch?v=dQw4w9WgXcQ").unwrap();
    let source = conn.get_page_source().unwrap();
    println!("{}", source);
}

#[test]
fn elements() {
    let _ = env_logger::init();
    let mut conn = MarionetteConnection::connect(2828).unwrap();

    conn.get("https://www.youtube.com/watch?v=dQw4w9WgXcQ").unwrap();
    let elements = conn.find_elements(QueryMethod::CssSelector, "body", None).unwrap();
    assert!(!elements.is_empty());
    let elements = conn.find_elements(QueryMethod::CssSelector, "video", Some(&elements[0])).unwrap();
    assert!(!elements.is_empty());

    let src = conn.get_element_attribute(&elements[0], "src").unwrap();
    println!("{}", src);

    let outer = conn.get_element_property(&elements[0], "outerHTML").unwrap();
    println!("{}", outer);

    let text = conn.find_elements(QueryMethod::CssSelector, "a", None).unwrap()
        .iter()
        .map(|elemref| Element::new(&mut conn, elemref).text().unwrap())
        .next().unwrap();
    println!("{}", text);

    for element_ref in &conn.find_elements(QueryMethod::CssSelector, "a", None).unwrap()[..10] {
        let mut a = Element::new(&mut conn, &element_ref);
        println!("{}", a.text().unwrap());
    }
}

#[test]
fn element_property_is_json() {
    let _ = env_logger::init();
    let mut conn = MarionetteConnection::connect(2828).unwrap();

    let elements = conn.find_elements(QueryMethod::CssSelector, "html", None).unwrap();
    assert!(!elements.is_empty());
    let hidden = conn.get_element_property(&elements[0], "hidden").unwrap();
    assert_eq!(hidden, JsonValue::Bool(false));
}
