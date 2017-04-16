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
