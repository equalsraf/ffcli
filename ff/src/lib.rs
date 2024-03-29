use std::io;
use std::net::TcpListener;
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

extern crate mozrunner;
extern crate mozprofile;
#[macro_use]
extern crate log;
extern crate mktemp;
extern crate marionette;
use marionette::{Result, MarionetteConnection};
extern crate dirs;
extern crate serde;
#[macro_use]
extern crate serde_derive;

mod runner;
use runner::FirefoxRunner;

pub struct Browser {
    pub runner: FirefoxRunner,
    session_file: Option<PathBuf>,
}

impl Browser {
    pub fn start<P: AsRef<Path>>(port: Option<u16>,
                                 profile_path: Option<P>,
                                 firefox_path: Option<P>,
                                 userjs_path: Option<P>,
                                 extraprefs_paths: Option<Vec<P>>,
                                 session_name: Option<&str>) -> io::Result<Self> {
        let runner = match profile_path {
            None => FirefoxRunner::tmp(port, firefox_path, userjs_path, extraprefs_paths)?,
            Some(path) => FirefoxRunner::from_path(path, port, firefox_path, userjs_path, extraprefs_paths)?,
        };

        let session_file = match port {
            Some(port) => Some(create_instance_file(session_name, port)?),
            None => None,
        };

        Ok(Browser {
            runner: runner,
            session_file: session_file,
        })
    }

    pub fn session_file(&self) -> Option<&Path> {
        self.session_file.as_ref().map(|p| p.as_path())
    }
}

impl Drop for Browser {
    fn drop(&mut self) {
        if let Some(file) = &self.session_file {
            debug!("Removing session file");
            let _ = fs::remove_file(&file);
        }
    }
}

/// Find a TCP port number to use. This is racy but see
/// https://bugzilla.mozilla.org/show_bug.cgi?id=1240830
///
/// If port is Some, check if we can bind to the given port. Otherwise
/// pick a random port.
pub fn check_tcp_port(port: Option<u16>) -> io::Result<u16> {
    TcpListener::bind(&("localhost", port.unwrap_or(0)))
        .and_then(|stream| stream.local_addr())
        .map(|x| x.port())
}

fn instance_root_path() -> io::Result<PathBuf> {
    let mut path = dirs::home_dir()
        .ok_or(io::Error::new(io::ErrorKind::Other, "Could not determine your HOME folder"))?;
    path.push(".ff");
    path.push("instances");
    Ok(path)
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Instance {
    pub port: u16,
    pub name: String,
}

fn create_instance_file(name: Option<&str>, port: u16) -> io::Result<PathBuf> {
    let mut path = instance_root_path()?;
    fs::create_dir_all(&path)?;

    path.push(format!("{}", port));
    let f = fs::File::create(&path)?;
    serde_json::to_writer(f, &Instance { port: port, name: name.unwrap_or("").to_string()})?;
    Ok(path)
}

/// List available instances
pub fn instances() -> io::Result<Vec<Instance>> {
    let mut res = Vec::new();
    for entry in fs::read_dir(&instance_root_path()?)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            let f = fs::File::open(&path)?;
            if let Ok(instance) = serde_json::from_reader(f) {
                res.push(instance);
            }
        }
    }
    Ok(res)
}

/// Test the marionette connection by attempting to connect multiple times
pub fn check_connection(port: u16) -> Result<MarionetteConnection> {
    let mut retry = 1;
    loop {
        thread::sleep(Duration::new(retry*2, 0));
        match marionette::MarionetteConnection::connect(port) {
            Ok(conn) => return Ok(conn),
            Err(err) => {
                debug!("#{} Failed to connect to firefox({}): {}", retry, port, err);
                if 4 <= retry {
                    return Err(err)?;
                }
            }
        }
        retry += 1;
    }
}

pub mod downloads;
