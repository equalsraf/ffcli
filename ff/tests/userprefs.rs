extern crate ff;

extern crate marionette;
use marionette::{MarionetteConnection, JsonValue};


#[test]
fn user_js_file() {
    let browser = ff::Browser::start(7933,
                                         None,
                                         None,
                                         Some("tests/data/test-user.js"),
                                         None).unwrap();

    ff::check_connection(browser.runner.port()).unwrap();

    let mut conn = MarionetteConnection::connect(browser.runner.port()).unwrap();
    let res = conn.get_pref("ff.testpref.canary").unwrap();
    assert_eq!(res, JsonValue::String("the canary is dead".to_string()));
}
