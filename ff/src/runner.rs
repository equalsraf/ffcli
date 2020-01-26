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
use mozprofile::profile::{Profile, PrefFile};
use mozprofile::preferences::Pref;

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

/// Create a new profile with some default user settings
///
/// importpath: is a path to an existing user.js file with settings to be imported
pub fn create_profile<P1: AsRef<Path>, P2: AsRef<Path>>(profilepath: P1, userjs: Vec<P2>) -> IoResult<Profile> {
    let mut profile = Profile::new_from_path(profilepath.as_ref())?;
    let prefs = profile.user_prefs()
        .map_err(|err| IoError::new(ErrorKind::Other, format!("{}", err)))?;
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

    // Disable privacy notice
    prefs.insert("datareporting.policy.firstRunURL", Pref::new(""));

    // Import preferences from an existing user.js
    for p in userjs {
        let src = PrefFile::new(PathBuf::from(p.as_ref()))
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "Invalid user.js"))?;
        for (name, value) in src.iter() {
            prefs.insert(name, value.clone());
        }
    }

    prefs.write()?;
    Ok(profile)
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
    pub fn tmp<P1: AsRef<Path>, P2: AsRef<Path>>(port: u16, firefox_path: Option<P1>, user_js: Vec<P2>) -> IoResult<FirefoxRunner> {
        let profile_tmpdir = Temp::new_dir()?;
        let mut profile = create_profile(&profile_tmpdir, user_js)?;

        {
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
    pub fn from_path<P: AsRef<Path>>(profile_path: P, port: u16, firefox_path: Option<P>) -> IoResult<FirefoxRunner> {
        fs::create_dir_all(&profile_path)?;

        let mut profile = Profile::new_from_path(profile_path.as_ref())?;

        {
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
