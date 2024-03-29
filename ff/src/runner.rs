use std::convert::From;
use std::io::ErrorKind;
use std::io::Error as IoError;
use std::io::Result as IoResult;
use std::path::{PathBuf, Path};
use std::process;
use std::process::{Command, Stdio, Child};
use std::fs;

extern crate marionette;
use mktemp::Temp;

use mozrunner::firefox_default_path;
use mozprofile::profile::Profile;
use mozprofile::preferences::Pref;

#[cfg(unix)]
fn spawn_firefox(firefox_bin: &Path, profile: &Path, marionette: bool) -> IoResult<Child> {
    let mut cmd = Command::new(firefox_bin);
    if marionette {
        cmd.arg("--marionette");
    }

    cmd.arg("--profile")
        .arg(profile)
        .stdout(Stdio::null()).stderr(Stdio::null())
        .env("MOZ_NO_REMOTE", "1").env("NO_EM_RESTART", "1")
        .spawn()
}

#[cfg(windows)]
fn spawn_firefox(firefox_bin: &Path, profile: &Path, marionette: bool) -> IoResult<Child> {
    let mut cmd = Command::new(firefox_bin);
    if marionette {
        cmd.arg("-marionette");
    }

    cmd.arg("-profile")
        .arg(profile)
        .stdout(Stdio::null()).stderr(Stdio::null())
        .env("MOZ_NO_REMOTE", "1").env("NO_EM_RESTART", "1")
        .spawn()
}

pub struct FirefoxRunner {
    pub process: process::Child,
    pub profile: Profile,
    port: Option<u16>,
    profile_tmpdir: Option<Temp>,
    drop_browser: bool,
}

impl FirefoxRunner {
    /// Run a new browser instance, listening on the given port.
    /// Creates a temporary profile for this instance.
    ///
    /// firefox_path: is an optional path to the firefox executable
    /// user_prefs: is an optional path to a user.js file to be copied into
    ///             the new profile
    pub fn tmp<P: AsRef<Path>>(port: Option<u16>,
                               firefox_path: Option<P>,
                               user_prefs: Option<P>,
                               extraprefs: Option<Vec<P>>) -> IoResult<FirefoxRunner> {

        let profile_tmpdir = Temp::new_dir()?;

        if let Some(src) = user_prefs {
            fs::copy(src, profile_tmpdir.as_ref().join("user.js"))?;
        }

        if let Some(files) = extraprefs {
            for file in &files {
                if let Some(filename) = file.as_ref().file_name() {
                    fs::copy(file, profile_tmpdir.as_ref().join(filename))?;
                }
            }
        }


        let mut profile = Profile::new_from_path(profile_tmpdir.as_ref())?;

        {
            let prefs = profile.user_prefs()
                .map_err(|err| IoError::new(ErrorKind::Other, format!("{}", err)))?;
            if let Some(port) = port {
                prefs.insert("marionette.port", Pref::new(port as i64));
                prefs.insert("marionette.defaultPrefs.port", Pref::new(port as i64));
            }

            prefs.write()?;
        }

        let bin = firefox_path
            .map(|p| p.as_ref().to_owned())
            .or(firefox_default_path())
            .unwrap_or(PathBuf::from("firefox"));
        let child = spawn_firefox(&bin, &profile.path, port.is_some())?;

        info!("Started firefox: {:?}", port);

        Ok(FirefoxRunner {
            process: child,
            profile: profile,
            port: port,
            profile_tmpdir: Some(profile_tmpdir),
            drop_browser: true,
        })
    }

    /// Starts a new firefox instance using the profile at the given path, if the
    /// path does not exist it is created.
    pub fn from_path<P: AsRef<Path>>(profile_path: P,
                                     port: Option<u16>,
                                     firefox_path: Option<P>,
                                     user_prefs: Option<P>,
                                     extraprefs: Option<Vec<P>>) -> IoResult<FirefoxRunner> {
        fs::create_dir_all(&profile_path)?;

        if let Some(src) = user_prefs {
            fs::copy(src, profile_path.as_ref().join("user.js"))?;
        }

        if let Some(files) = extraprefs {
            for file in &files {
                if let Some(filename) = file.as_ref().file_name() {
                    fs::copy(file, profile_path.as_ref().join(filename))?;
                }
            }
        }

        let mut profile = Profile::new_from_path(profile_path.as_ref())?;

        if let Some(port) = port {
            let prefs = profile.user_prefs()
                .map_err(|err| IoError::new(ErrorKind::Other, format!("{}", err)))?;
            prefs.insert("marionette.port", Pref::new(port as i64));
            prefs.insert("marionette.defaultPrefs.port", Pref::new(port as i64));
            prefs.write()?;
        }

        let bin = firefox_path
            .map(|p| p.as_ref().to_owned())
            .or(firefox_default_path())
            .unwrap_or(PathBuf::from("firefox"));
        let child = spawn_firefox(&bin, &profile.path, port.is_some())?;

        info!("Started firefox: {:?}", port);

        Ok(FirefoxRunner {
            process: child,
            profile: profile,
            port: port,
            profile_tmpdir: None,
            drop_browser: true,
        })
    }

    /// The marionette port the browser is listening on
    pub fn port(&self) -> Option<u16> { self.port }

    /// If true (the default) the browser process will be killed
    /// on Drop.
    pub fn kill_on_drop(&mut self, drop: bool) {
        self.drop_browser = drop;
    }
}

impl Drop for FirefoxRunner {
    fn drop(&mut self) {
        if !self.drop_browser {
            return;
        }

        let _ = self.process.kill();
    }
}
