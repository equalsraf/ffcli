extern crate ff;

extern crate marionette;
use marionette::{MarionetteConnection, JsonValue};
use std::thread;
use std::time::Duration;

#[test]
fn user_js_file() {
    let browser = ff::Browser::start(65333,
                                         None,
                                         None,
                                         Some("tests/data/test-user.js"),
                                         None,
                                         None).unwrap();

    ff::check_connection(browser.runner.port()).unwrap();

    thread::sleep(Duration::new(5, 0));
    let mut conn = MarionetteConnection::connect(browser.runner.port()).unwrap();
    let res = conn.get_pref("ff.testpref.canary").unwrap();
    assert_eq!(res, JsonValue::String("the canary is dead".to_string()));
}
