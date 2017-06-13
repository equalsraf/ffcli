//! This behaves a bit like doctests in rustdoc. It looks for code blocks in
//! the markdown docs and executes them.
//!
//! Only lines starting with a dollar are executed e.g.
//!
//!     $ this line will be executed
//!     but this line will not
//! ```

use std::fs::File;
use std::io::Read;
use std::process::{Command, Stdio};
use std::env;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

extern crate pulldown_cmark;
use pulldown_cmark::{Parser, Event, Tag};
extern crate env_logger;
#[macro_use]
extern crate log;

#[cfg(unix)]
fn run_shell_command(cmd: &str) {
    let _ = env_logger::init();
    debug!("executing doctest command: {}", cmd);
    let mut child = Command::new("/bin/sh")
        .arg("-c")
        .arg(cmd)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .expect(&format!("Failed to execute"));

    let ecode = child.wait()
        .expect("failed to wait on child command");

    if !ecode.success() {
        panic!("Command failed");
    }
}

#[cfg(windows)]
fn run_shell_command(cmd: &str) {
    let _ = env_logger::init();
    debug!("executing doctest command: {}", cmd);
    let mut child = Command::new("powershell")
        .arg("-Command")
        .arg(cmd)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .expect(&format!("Failed to execute"));

    let ecode = child.wait()
        .expect("failed to wait on child command");

    if !ecode.success() {
        panic!("Command failed");
    }
}

#[test]
fn manual() {
    if let Some(path) = env::var_os("PATH") {
        let mut paths = env::split_paths(&path).collect::<Vec<_>>();
        paths.push(PathBuf::from("../target/debug/"));
        let new_path = env::join_paths(paths).unwrap();
        env::set_var("PATH", &new_path);
    }

    let mut data = String::new();
    let mut f = File::open("MANUAL.md").unwrap();
    f.read_to_string(&mut data).unwrap();

    let parser = Parser::new(&data);

    let mut inside_shellblock = false;
    for item in parser {
        match item {
            Event::Start(Tag::CodeBlock(_)) => inside_shellblock = true,
            Event::End(Tag::CodeBlock(_)) => inside_shellblock = false,
            Event::Text(ref s) if inside_shellblock => {

                for line in s.lines().filter(|line| line.starts_with('$')) {
                    run_shell_command(&line[1..]);
                    thread::sleep(Duration::new(3, 1));
                }
            }
            _ => (),
        }
    }
}
