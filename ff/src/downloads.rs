//! Wrapper around Downloads.jsm
//!
//! https://developer.mozilla.org/en-US/docs/Mozilla/JavaScript_code_modules/Downloads.jsm

use marionette::{MarionetteConnection, Result, Script};
use std::path::Path;
use std::env;

/// Start a new download.
pub fn start(conn: &mut MarionetteConnection, url: &str, path: &Path) -> Result<()> {
    let mut s = Script::new(include_str!("js/create_download.js"));
    s.system_sandbox();

    if path.is_relative() {
        let mut absolute_path = try!(env::current_dir());
        absolute_path.push(path);
        s.arguments(&(url, absolute_path))?;
    } else {
        s.arguments(&(url, path))?;
    }

    conn.execute_script(&s)?;
    Ok(())
}
