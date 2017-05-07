use std::io;
use std::io::{Write, Read};
use std::net::TcpListener;
use std::env;
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
use marionette::Result;

mod runner;
use runner::FirefoxRunner;

pub struct Browser {
    pub runner: FirefoxRunner,
    session_file: PathBuf,
}

impl Browser {
    pub fn start<P: AsRef<Path>>(port: u16, profile_path: Option<P>) -> io::Result<Self> {
        let runner = match profile_path {
            None => FirefoxRunner::tmp(port)?,
            Some(path) => FirefoxRunner::from_path(path, port)?,
        };
        let session_file = create_instance_file(None, port)?;
        Ok(Browser {
            runner: runner,
            session_file: session_file,
        })
    }

    pub fn session_file(&self) -> &Path {
        self.session_file.as_path()
    }
}

impl Drop for Browser {
    fn drop(&mut self) {
        debug!("Removing session file");
        let _ = fs::remove_file(&self.session_file);
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
    let mut path = env::home_dir()
        .ok_or(io::Error::new(io::ErrorKind::Other, "Could not determine your HOME folder"))?;
    path.push(".ff");
    path.push("instances");
    Ok(path)
}

fn create_instance_file(name: Option<&str>, port: u16) -> io::Result<PathBuf> {
    let mut path = instance_root_path()?;
    fs::create_dir_all(&path)?;

    path.push(format!("{}", port));
    let mut f = fs::File::create(&path)?;
    f.write_all(format!("{}/{}", port, name.unwrap_or("")).as_bytes())?;
    Ok(path)
}

/// List available instances
pub fn instances() -> io::Result<Vec<String>> {
    let mut res = Vec::new();
    for entry in fs::read_dir(&instance_root_path()?)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            let mut data = String::new();
            let mut f = fs::File::open(&path)?;
            f.read_to_string(&mut data)?;
            res.push(data);
        }
    }
    Ok(res)
}

/// Test the marionette connection by attempting to connect multiple times
pub fn check_connection(port: u16) -> Result<()> {
    let mut retry = 1;
    loop {
        thread::sleep(Duration::new(retry*2, 0));
        match marionette::MarionetteConnection::connect(port) {
            Ok(_) => break,
            Err(err) => {
                debug!("#{} Failed to connect to firefox({}): {}", retry, port, err);
                if 4 <= retry {
                    return Err(err)?;
                }
            }
        }
        retry += 1;
    }
    Ok(())
}

