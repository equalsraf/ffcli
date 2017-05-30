use std::str::FromStr;

extern crate marionette;
use marionette::*;
extern crate env_logger;

#[test]
fn windows() {   
    let _ = env_logger::init();
    let mut conn = MarionetteConnection::connect(2828).unwrap();
    let windows = conn.get_window_handles().unwrap();

    for window in windows {
        conn.switch_to_window(&window).unwrap();
        assert_eq!(window, conn.get_window_handle().unwrap());
    }
}

#[test]
fn context() {
    let _ = env_logger::init();
    let mut conn = MarionetteConnection::connect(2828).unwrap();

    let ctx = conn.get_context().unwrap();

    conn.set_context(Context::Chrome).unwrap();
    assert_eq!(Context::Chrome, conn.get_context().unwrap());
    conn.set_context(Context::Content).unwrap();
    assert_eq!(Context::Content, conn.get_context().unwrap());

    conn.set_context(ctx).unwrap();
}

#[test]
fn set_pref() {
    let _ = env_logger::init();
    let mut conn = MarionetteConnection::connect(2828).unwrap();

    conn.set_pref("browser.uitour.enabled", JsonValue::Bool(false)).unwrap();
    let res = conn.get_pref("browser.uitour.enabled").unwrap();
    assert_eq!(res, JsonValue::Bool(false));

    conn.set_pref("browser.uitour.enabled", JsonValue::Bool(true)).unwrap();
    let res = conn.get_pref("browser.uitour.enabled").unwrap();
    assert_eq!(res, JsonValue::Bool(true));

    assert_eq!(conn.get_context().unwrap(), Context::Content);
}

#[test]
fn set_pref_string() {
    let _ = env_logger::init();
    let mut conn = MarionetteConnection::connect(2828).unwrap();

    conn.set_pref("startup.homepage_welcome_url", JsonValue::String("http://localhost".to_owned())).unwrap();
    let homepage = conn.get_pref("startup.homepage_welcome_url").unwrap();
    assert_eq!(homepage, JsonValue::String("http://localhost".to_owned()));
}

#[test]
fn set_pref_fails_but_restores_ctx() {
    let _ = env_logger::init();
    let mut conn = MarionetteConnection::connect(2828).unwrap();

    assert!(conn.set_pref("startup.homepage_welcome_url", JsonValue::from_str("42").unwrap()).is_err());
    assert_eq!(conn.get_context().unwrap(), Context::Content);
}
