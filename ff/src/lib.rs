use std::path::PathBuf;
use std::net::TcpListener;
use std::thread;
use std::time::Duration;

extern crate mozrunner;
use mozrunner::firefox_default_path;
use mozrunner::runner::{FirefoxRunner, Runner};
pub use mozrunner::runner::RunnerError;

extern crate mozprofile;
use mozprofile::profile::Profile;
use mozprofile::preferences::Pref;

#[macro_use]
extern crate log;
extern crate mktemp;

extern crate marionette;

pub struct Browser {
    runner: FirefoxRunner,
    pub connection: marionette::MarionetteConnection,
    pub tmpdir: PathBuf,
    port: u16,
    drop_browser: bool,
}
impl Browser {
    pub fn start(port: Option<u16>) -> Result<Self, RunnerError> {

        let mut tmpdir = mktemp::Temp::new_dir()?;
        let mut profile = Profile::new(Some(tmpdir.as_ref()))?;
        
        // racy but see https://bugzilla.mozilla.org/show_bug.cgi?id=1240830
        // also used in geckodriver
        let portnum = TcpListener::bind(&("localhost", port.unwrap_or(0)))
            .and_then(|stream| stream.local_addr())
            .map(|x| x.port())?;

        {
            let mut prefs = profile.user_prefs()?;
            prefs.insert("marionette.port", Pref::new(portnum as i64));
            prefs.insert("marionette.defaultPrefs.port", Pref::new(portnum as i64));
            // Startup with a blank page
            prefs.insert("browser.startup.page", Pref::new(0 as i64));
            prefs.insert("browser.startup.homepage_override.mstone", Pref::new("ignore"));

            // Disable the UI tour
            prefs.insert("browser.uitour.enabled", Pref::new(false));
            // Disable first-run welcome page
            prefs.insert("startup.homepage_welcome_url", Pref::new("about:blank"));
            prefs.insert("startup.homepage_welcome_url.additional", Pref::new(""));
        }

        let bin = firefox_default_path().unwrap_or(PathBuf::from("firefox"));
        let mut runner = FirefoxRunner::new(&bin, Some(profile))?;
        runner.start()?;

        info!("Started firefox on port {}: {}", portnum,
               runner.args().iter()
                   .fold(String::new(), |acc, ref x| acc + &x)
               );

        let connection;
        let mut retry = 1;
        loop {
            thread::sleep(Duration::new(retry, 0));
            if !runner.is_running() {
                debug!("Firefox is not running!");
            }
            match marionette::MarionetteConnection::connect(portnum) {
                Ok(conn) => {
                    connection = conn;
                    break;
                }
                Err(err) => {
                    debug!("Failed to connect to firefox({}): {}", portnum, err);
                    if 4 <= retry {
                        return Err(err)?;
                    }
                }
            }
            retry += 1;
        }

        // release tempdir here, because ultimately we may want to exit after
        // launching firefox, in which case we cannot remove that tmp dir
        tmpdir.release();
        Ok(Browser {
            runner: runner,
            connection: connection,
            tmpdir: tmpdir.to_path_buf(),
            port: portnum,
            drop_browser: true,
        })
    }

    pub fn profile(&self) -> &Profile { &self.runner.profile }

    pub fn port(&self) -> u16 { self.port }

    /// If true (the default) the browser process will be killed
    /// on Drop.
    pub fn kill_on_drop(&mut self, drop: bool) {
        self.drop_browser = drop;
    }
}

impl Drop for Browser {
    fn drop(&mut self) {
        if self.drop_browser {
            let _ = self.runner.stop();
        }
    }
}
