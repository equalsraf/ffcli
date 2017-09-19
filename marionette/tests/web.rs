extern crate marionette;
use marionette::*;
use marionette::messages::Script;
use marionette::messages::Timeouts;
extern crate env_logger;

#[test]
fn script_system() {
    let _ = env_logger::init();
    let mut conn = MarionetteConnection::connect(2828).unwrap();

    let mut script = Script::new("return 42;");
    script.sandbox("system");
    let res = conn.execute_script(&script).unwrap();
    assert_eq!(res, JsonValue::from(42));
}

#[test]
fn script() {   
    let _ = env_logger::init();
    let mut conn = MarionetteConnection::connect(2828).unwrap();

    let script = Script::new("return 42;");
    let res = conn.execute_script(&script).unwrap();
    assert_eq!(res, JsonValue::from(42));
}

#[test]
fn async_script() {
    let _ = env_logger::init();
    let mut conn = MarionetteConnection::connect(2828).unwrap();

    // async scripts terminate using marionetteScriptFinished()
    let script = Script::new("marionetteScriptFinished(42); return 1;");
    let res = conn.execute_async_script(&script).unwrap();
    assert_eq!(res, JsonValue::from(42));
}

#[test]
fn script_arguments() {   
    let _ = env_logger::init();
    let mut conn = MarionetteConnection::connect(2828).unwrap();

    let mut script = Script::new("return arguments[0];");
    script.arguments(vec![84]).unwrap();
    let res = conn.execute_script(&script).unwrap();
    assert_eq!(res, JsonValue::from(84));
}

#[test]
fn script_timeout() {
    let _ = env_logger::init();
    let mut conn = MarionetteConnection::connect(2828).unwrap();

    let t = Timeouts {
        script: 1000,
        pageLoad: 10000,
        implicit: 10000,
    };
    conn.set_timeouts(t).unwrap();
    let script = Script::new(r#"
            setTimeout(function() {
                marionetteScriptFinished(42);
            }, 3000);
            "#);
    assert!(conn.execute_async_script(&script).is_err());

    let t = Timeouts {
        script: 11000,
        pageLoad: 10000,
        implicit: 10000,
    };
    conn.set_timeouts(t).unwrap();
    let res = conn.execute_async_script(&script).unwrap();
    assert_eq!(res, JsonValue::from(42));
}

#[test]
fn script_arguments_element() {
    let _ = env_logger::init();
    let mut conn = MarionetteConnection::connect(2828).unwrap();
    conn.get("https://www.youtube.com/watch?v=dQw4w9WgXcQ").unwrap();

    let elements = conn.find_elements(QueryMethod::CssSelector, "video", None).unwrap();
    let mut script = Script::new("arguments[0].pause(); arguments[0].play(); return arguments[0].src");
    script.arguments(&[&elements[0]]).unwrap();
    let res = conn.execute_script(&script).unwrap();

    println!("{:?}", res);
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

    for element_ref in &conn.find_elements(QueryMethod::CssSelector, "a", None).unwrap() {
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

#[test]
fn frames() {
    let _ = env_logger::init();
    let mut conn = MarionetteConnection::connect(2828).unwrap();

    conn.switch_to_frame(None).unwrap();
    let _ = conn.get_active_frame().unwrap();
    conn.switch_to_parent_frame().unwrap();
}
