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
use mozprofile::prefdata::FIREFOX_PREFERENCES;

#[cfg(unix)]
fn spawn_firefox(firefox_bin: &Path, profile: &Path) -> IoResult<Child> {
    Command::new(firefox_bin)
        .arg("--marionette")
        .arg("--profile")
        .arg(profile)
        .stdout(Stdio::null()).stderr(Stdio::null())
        .env("MOZ_NO_REMOTE", "1").env("NO_EM_RESTART", "1")
        .spawn()
}

#[cfg(windows)]
fn spawn_firefox(firefox_bin: &Path, profile: &Path) -> IoResult<Child> {
    Command::new(firefox_bin)
        .arg("-marionette")
        .arg("-profile")
        .arg(profile)
        .stdout(Stdio::null()).stderr(Stdio::null())
        .env("MOZ_NO_REMOTE", "1").env("NO_EM_RESTART", "1")
        .spawn()
}

pub struct FirefoxRunner {
    pub process: process::Child,
    pub profile: Profile,
    port: u16,
    profile_tmpdir: Option<Temp>,
    drop_browser: bool,
}

impl FirefoxRunner {
    /// Run a new browser instance, listening on the given port.
    /// Creates a temporary profile for this instance.
    pub fn tmp(port: u16) -> IoResult<FirefoxRunner> {
        let profile_tmpdir = Temp::new_dir()?;
        let mut profile = Profile::new(Some(profile_tmpdir.as_ref()))?;

        {
            let mut prefs = profile.user_prefs()
                .map_err(|err| IoError::new(ErrorKind::Other, format!("{}", err)))?;
            prefs.insert("marionette.port", Pref::new(port as i64));
            prefs.insert("marionette.defaultPrefs.port", Pref::new(port as i64));
            // Startup with a blank page
            prefs.insert("browser.startup.page", Pref::new(0 as i64));
            prefs.insert("browser.startup.homepage_override.mstone", Pref::new("ignore"));

            // Disable the UI tour
            prefs.insert("browser.uitour.enabled", Pref::new(false));
            // Disable first-run welcome page
            prefs.insert("startup.homepage_welcome_url", Pref::new("about:blank"));
            prefs.insert("startup.homepage_welcome_url.additional", Pref::new(""));

            // Disable autoplay
            prefs.insert("media.autoplay.enabled", Pref::new(false));
            // Enable private browsing
            prefs.insert("browser.privatebrowsing.autostart", Pref::new(false));

            prefs.insert_slice(&FIREFOX_PREFERENCES[..]);
            prefs.write()?;
        }

        let bin = firefox_default_path().unwrap_or(PathBuf::from("firefox"));
        let child = spawn_firefox(&bin, &profile.path)?;

        info!("Started firefox on port {}", port);

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
    pub fn from_path<P: AsRef<Path>>(profile_path: P, port: u16) -> IoResult<FirefoxRunner> {
        fs::create_dir_all(&profile_path)?;

        let mut profile = Profile::new(Some(profile_path.as_ref()))?;

        {
            let mut prefs = profile.user_prefs()
                .map_err(|err| IoError::new(ErrorKind::Other, format!("{}", err)))?;
            prefs.insert("marionette.port", Pref::new(port as i64));
            prefs.insert("marionette.defaultPrefs.port", Pref::new(port as i64));
            prefs.write()?;
        }

        let bin = firefox_default_path().unwrap_or(PathBuf::from("firefox"));
        let child = spawn_firefox(&bin, &profile.path)?;

        info!("Started firefox on port {}", port);

        Ok(FirefoxRunner {
            process: child,
            profile: profile,
            port: port,
            profile_tmpdir: None,
            drop_browser: true,
        })
    }

    /// The marionette port the browser is listening on
    pub fn port(&self) -> u16 { self.port }

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
