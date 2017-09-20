use std::str::FromStr;
use std::env;
use std::process::{Command, Stdio, exit};
use std::io::{self, Read, Write};
use std::path::Path;
use std::panic;

extern crate ff;
extern crate marionette;
use marionette::{MarionetteConnection, Element, JsonValue, WindowHandle, Result, Script};
use marionette::QueryMethod::CssSelector;
#[macro_use]
extern crate clap;
use clap::{App, Arg, ArgMatches, SubCommand, AppSettings};
#[macro_use]
extern crate log;
extern crate stderrlog;
extern crate url;
#[cfg(unix)]
extern crate chan_signal;

const ISSUES_URL: &'static str = "https://github.com/equalsraf/ffcli/issues";

trait ExitOnError<T>: Sized {
    fn exit(code: i32, msg: Option<&str>) -> ! {
        if let Some(msg) = msg {
            let _ = writeln!(&mut std::io::stderr(), "{}", msg);
        }
        exit(code);
    }

    fn unwrap_or_exit(self, code: i32) -> T;
    fn unwrap_or_exitmsg(self, code: i32, msg: &str) -> T;
}

impl<T, E: std::error::Error> ExitOnError<T> for std::result::Result<T, E> {
    fn unwrap_or_exit(self, code: i32) -> T {
        match self {
            Ok(res) => res,
            Err(err) => {
                Self::exit(code, Some(err.description()));
            }
        }
    }

    fn unwrap_or_exitmsg(self, code: i32, msg: &str) -> T {
        match self {
            Ok(res) => res,
            Err(err) => {
                Self::exit(code, Some(&format!("{}: {}", msg, err.description())));
            }
        }
    }
}
impl<T> ExitOnError<T> for Option<T> {
    fn unwrap_or_exit(self, code: i32) -> T {
        match self {
            Some(res) => res,
            None => Self::exit(code, None),
        }
    }
    fn unwrap_or_exitmsg(self, code: i32, msg: &str) -> T {
        match self {
            Some(res) => res,
            None => Self::exit(code, Some(msg)),
        }
    }
}


#[cfg(unix)]
fn setup_signals() {
    // For now Ignore SIGINT
    chan_signal::notify(&[chan_signal::Signal::INT]);
}
#[cfg(not(unix))]
fn setup_signals() {}

fn cmd_start(args: &ArgMatches) -> Result<()> {

    let port_arg = args.value_of("PORT")
        .map(|val| val.to_owned())
        .map(|ref s| u16::from_str(s).unwrap_or_exitmsg(-1, "Invalid port argument"));

    // Check TCP port availability
    let portnum = ff::check_tcp_port(port_arg)?;

    if args.is_present("no-fork") {
        setup_signals();

        let mut browser = ff::Browser::start(portnum, args.value_of("PROFILE"), args.value_of("FIREFOX-BIN"), args.value_of("SESSION"))?;
        debug!("New ff session {}", browser.session_file().to_string_lossy());

        let status = browser.runner.process.wait()?;
        info!("Firefox exited with status {}", status);
    } else {
        let mut child_args: Vec<_> = env::args().collect();
        child_args.push("--no-fork".to_owned());
        if port_arg.is_none() {
            child_args.push("--port".to_owned());
            child_args.push(format!("{}", portnum));
        }

        debug!("Spawning ff process {:?}", child_args);
        Command::new(&child_args[0])
            .args(&child_args[1..])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .spawn()?;

        let mut conn = ff::check_connection(portnum)?;
        if let Some(url) = args.value_of("URL") {
            conn.get(&convert_url(url))?;
        }
        println!("{}", portnum);
    }

    Ok(())
}

fn cmd_install(args: &ArgMatches) -> Result<()> {
    connect_to_port(args)
        .addon_install(Path::new(args.value_of("PATH").unwrap()))
}

fn cmd_instances() -> Result<()> {
    for instance in ff::instances()? {
        println!("{}", instance);
    }
    Ok(())
}

fn convert_url(url_in: &str) -> String {
    if let Err(url::ParseError::RelativeUrlWithoutBase) = url::Url::parse(url_in) {
        "https://".to_owned() + url_in
    } else {
        url_in.to_owned()
    }
}

fn cmd_go(args: &ArgMatches) -> Result<()> {
    let url_arg = args.value_of("URL").unwrap();

    let mut conn = connect_to_port(args);
    conn.get(&convert_url(url_arg))
}

fn cmd_download(args: &ArgMatches) -> Result<()> {
    let url_arg = args.value_of("URL").unwrap();
    let path = args.value_of("FILE").unwrap();

    let mut conn = connect_to_port(args);
    ff::downloads::start(&mut conn, url_arg, Path::new(path))
}

fn cmd_windows(args: &ArgMatches) -> Result<()> {
    let mut conn = connect_to_port(args);
    let prev = conn.get_window_handle()?;
    for win in conn.get_window_handles()? {
        conn.switch_to_window(&win)?;
        let title = conn.get_title()?;
        println!("{} \"{}\"", win, title);
    }

    conn.switch_to_window(&prev)?;
    Ok(())
}

const FRAME_SELECTOR: &'static str = "iframe, frame";

/// Iterate over all frames under the current frame
fn foreach_frame<F>(conn: &mut MarionetteConnection, args: &ArgMatches, f: &F) -> Result<()>
        where F: Fn(&mut MarionetteConnection, &ArgMatches) -> Result<()> {
    f(conn, args)?;
    for frameref in conn.find_elements(CssSelector, FRAME_SELECTOR, None)? {
        conn.switch_to_frame(Some(frameref))?;
        foreach_frame(conn, args, f)?;
        conn.switch_to_parent_frame()?;
    }
    Ok(())
}

/// Iterate over elements based on argument "SELECTOR"
fn foreach_element<F, T>(conn: &mut MarionetteConnection, args: &ArgMatches, f: &F) -> Result<()>
        where F: Fn(&mut Element) -> Result<T> {
    let selector =  args.value_of("SELECTOR").unwrap();
    for elemref in conn.find_elements(CssSelector, selector, None)? {
        f(&mut Element::new(conn, &elemref))?;
    }
    Ok(())
}

fn print_json_value(val: &JsonValue, args: &ArgMatches) {
    if args.is_present("FILTER-STR") {
        if let JsonValue::String(ref val) = *val {
            println!("{}", val);
        }
    } else if JsonValue::Null != *val {
        println!("{}", val);
    }
}

/// Panics unless --port of $FF_PORT is a valid port number
fn connect_to_port(args: &ArgMatches) -> MarionetteConnection {
    let port_arg = args.value_of("PORT")
        .map(|val| val.to_owned())
        .or_else(|| env::var("FF_PORT").ok())
        .map(|ref s| u16::from_str(s).unwrap_or_exitmsg(-1, "Invalid port argument"));

    let port = port_arg.unwrap_or_exitmsg(-1, "No port given, use --port or $FF_PORT");
    MarionetteConnection::connect(port)
        .unwrap_or_exitmsg(-1, "Unable to connect to firefox")
}

fn option_port<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name("PORT")
        .takes_value(true)
        .long("port")
}

/// Common options for handling JSON data, see `print_json_value()`
fn option_json_filters<'a, 'b>() -> [Arg<'a, 'b>; 1] {
    [
        Arg::with_name("FILTER-STR")
            .long("filter-str")
            .short("S")
            .help("Print string values, ignore other types"),
    ]
}

fn main() {
    let matches = App::new("ff")
        .about("Firefox from your shell")
        .version(crate_version!())
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .setting(AppSettings::VersionlessSubcommands)
        .arg(Arg::with_name("verbose")
             .help("Increases verbosity")
             .short("v")
             .multiple(true)
             .long("verbose"))
        .subcommand(SubCommand::with_name("start")
                    .arg(option_port())
                    .arg(Arg::with_name("no-fork")
                         .help("Run ff in the foreground")
                         .long("no-fork"))
                    .arg(Arg::with_name("PROFILE")
                         .takes_value(true)
                         .help("Profile path")
                         .long("profile")
                         .short("P"))
                    .arg(Arg::with_name("SESSION")
                         .takes_value(true)
                         .help("Give a name to this session")
                         .long("session")
                         .short("S"))
                    .arg(Arg::with_name("FIREFOX-BIN")
                         .takes_value(true)
                         .help("Firefox binary path")
                         .long("firefox-bin"))
                    .arg(Arg::with_name("URL")
                         .help("Open the given URL after starting"))
                    .about("Start a new browser instance"))
        .subcommand(SubCommand::with_name("go")
                    .arg(option_port())
                    .about("Navigate to URL")
                    .arg(Arg::with_name("URL")
                         .required(true)
                        ))
        .subcommand(SubCommand::with_name("download")
                    .arg(option_port())
                    .about("Download URL")
                    .arg(Arg::with_name("URL")
                         .required(true))
                    .arg(Arg::with_name("FILE")
                         .required(true)))
        .subcommand(SubCommand::with_name("back")
                    .arg(option_port())
                    .about("Go back to the previous page in history"))
        .subcommand(SubCommand::with_name("forward")
                    .arg(option_port())
                    .about("Go forward to the next page in history"))
        .subcommand(SubCommand::with_name("source")
                    .arg(option_port())
                    .about("Print page source"))
        .subcommand(SubCommand::with_name("attr")
                    .arg(option_port())
                    .arg(Arg::with_name("SELECTOR")
                         .required(true))
                    .arg(Arg::with_name("ATTRNAME")
                         .required(true))
                    .about("Print element attribute"))
        .subcommand(SubCommand::with_name("exec")
                    .arg(option_port())
                    .arg(Arg::with_name("SCRIPT")
                         .required(true)
                         .help("Javascript code"))
                    .arg(Arg::with_name("ASYNC")
                         .takes_value(false)
                         .long("async")
                         .help("Run asynchronous script"))
                    .arg(Arg::with_name("SANDBOX")
                         .takes_value(true)
                         .long("sandbox")
                         .required(false)
                         .help("Sandbox name"))
                    .arg(Arg::with_name("TIMEOUT")
                         .takes_value(true)
                         .long("timeout")
                         .required(false)
                         .help("Timeout script execution after TIMEOUT milliseconds"))
                    .arg(Arg::with_name("ARG")
                         .multiple(true)
                         .required(false)
                         .help("Script arguments[]"))
                    .args(&option_json_filters())
                    .about("Executes script in all frames, print its return value"))
        .subcommand(SubCommand::with_name("property")
                    .arg(option_port())
                    .arg(Arg::with_name("SELECTOR")
                         .required(true))
                    .arg(Arg::with_name("NAME")
                         .required(true))
                    .args(&option_json_filters())
                    .about("Print element property")
                    .alias("prop"))
        .subcommand(SubCommand::with_name("text")
                    .arg(option_port())
                    .arg(Arg::with_name("SELECTOR")
                         .required(true))
                    .about("Print element text"))
        .subcommand(SubCommand::with_name("install")
                    .arg(option_port())
                    .arg(Arg::with_name("PATH")
                         .required(true))
                    .about("Install XPI addon"))
        .subcommand(SubCommand::with_name("prefget")
                    .arg(option_port())
                    .arg(Arg::with_name("NAME")
                         .required(true))
                    .about("Get firefox preference"))
        .subcommand(SubCommand::with_name("prefset")
                    .arg(option_port())
                    .arg(Arg::with_name("NAME")
                         .required(true))
                    .arg(Arg::with_name("VALUE")
                         .required(true))
                    .about("Set firefox preference"))
        .subcommand(SubCommand::with_name("instances")
                    .about("List running ff instances"))
        .subcommand(SubCommand::with_name("title")
                    .arg(option_port())
                    .about("Print page title"))
        .subcommand(SubCommand::with_name("url")
                    .arg(option_port())
                    .about("Print page url"))
        .subcommand(SubCommand::with_name("quit")
                    .arg(option_port())
                    .about("Close the browser"))
        .subcommand(SubCommand::with_name("windows")
                    .arg(option_port())
                    .about("List browser windows"))
        .subcommand(SubCommand::with_name("switch")
                    .arg(option_port())
                    .arg(Arg::with_name("index")
                         .help("Treat WINDOW as an index instead of a window id")
                         .long("idx"))
                    .arg(Arg::with_name("WINDOW")
                         .required(true))
                    .about("Switch browser window"))
        .get_matches();

    stderrlog::new()
            .module(module_path!())
            .verbosity(matches.occurrences_of("verbose") as usize)
            .init()
            .expect("Unable to initialize stderr output");

    let def_panic_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let _ = writeln!(&mut std::io::stderr(), "ff has just crashed, this is likely a bug on our side. Please take the time to report this issue so 
it can be fixed - {}\n", ISSUES_URL);
        def_panic_hook(info);
    }));

    match matches.subcommand() {
        ("go", Some(ref args)) => cmd_go(args).unwrap_or_exit(-1),
        ("back", Some(ref args)) => connect_to_port(args).go_back().unwrap_or_exit(-1),
        ("download", Some(ref args)) => cmd_download(args).unwrap_or_exit(-1),
        ("forward", Some(ref args)) => connect_to_port(args).go_forward().unwrap_or_exit(-1),
        ("source", Some(ref args)) => println!("{}", connect_to_port(args).get_page_source().unwrap_or_exit(-1)),
        ("text", Some(ref args)) => {
            let mut conn = connect_to_port(args);
            conn.switch_to_frame(None).unwrap_or_exit(-1);
            foreach_frame(&mut conn, args, &|conn, args| {
                foreach_element(conn, args, &|elem| {
                    let text = elem.text()?;
                    if !text.is_empty() {
                        println!("{}", text);
                    }
                    Ok(())
                })
            }).unwrap_or_exit(-1);
            conn.switch_to_frame(None).unwrap_or_exit(-1);
        }
        ("attr", Some(ref args)) => {
            let mut conn = connect_to_port(args);
            let attrname = args.value_of("ATTRNAME").unwrap();
            conn.switch_to_frame(None).unwrap_or_exit(-1);
            foreach_frame(&mut conn, args, &|conn, args| {
                foreach_element(conn, args, &|elem| {
                    let text = elem.attr(attrname)?;
                    if !text.is_empty() {
                        println!("{}", text);
                    }
                    Ok(())
                })
            }).unwrap_or_exit(-1);
            conn.switch_to_frame(None).unwrap_or_exit(-1);
        }
        ("exec", Some(ref args)) => {
            let mut conn = connect_to_port(args);
            let mut js = args.value_of("SCRIPT").unwrap().to_owned();
            if js == "-" {
                js.clear();
                io::stdin().read_to_string(&mut js).unwrap_or_exitmsg(-1, "Error reading stdin");
            }

            let mut script = Script::new(&js);

            if args.is_present("SANDBOX") {
                script.sandbox(args.value_of("SANDBOX").unwrap());
            }

            if let Some(s_timeout) = args.value_of("TIMEOUT") {
                let ms = u64::from_str(s_timeout)
                    .expect("Invalid TIMEOUT value");
                script.timeout(ms);
            }

            if let Some(iter) = args.values_of("ARG") {
                let mut script_args: Vec<JsonValue> = Vec::new();
                for arg in iter {
                    let val = JsonValue::from_str(arg).unwrap_or_exitmsg(-1, "Script argument is invalid JSON");
                    script_args.push(val);
                }
                script.arguments(script_args).unwrap_or_exit(-1);
            }

            conn.switch_to_frame(None).unwrap_or_exit(-1);
            foreach_frame(&mut conn, args, &|conn, args| {
                let res = if args.is_present("ASYNC") {
                    conn.execute_async_script(&script)
                } else {
                    conn.execute_script(&script)
                };

                match res {
                    Ok(val) => print_json_value(&val, args),
                    Err(ref err) if !err.is_fatal() => error!("Error executing script: {}", err),
                    Err(err) => return Err(err),
                }
                Ok(())
            }).unwrap_or_exit(-1);
            conn.switch_to_frame(None).unwrap_or_exit(-1);
        }
        ("prefget", Some(ref args)) => println!("{}", connect_to_port(args).get_pref(args.value_of("NAME").unwrap()).unwrap()),
        ("property", Some(ref args)) => {
            let mut conn = connect_to_port(args);
            let propname = args.value_of("NAME").unwrap();
            conn.switch_to_frame(None).unwrap_or_exit(-1);
            foreach_frame(&mut conn, args, &|conn, args| {
                foreach_element(conn, args, &|elem| {
                    let val = elem.property(propname)?;
                    print_json_value(&val, args);
                    Ok(())
                })
            }).unwrap_or_exit(-1);
            conn.switch_to_frame(None).unwrap_or_exit(-1);
        }
        ("prefset", Some(ref args)) => {
            let name = args.value_of("NAME").unwrap();
            let value = JsonValue::from_str(args.value_of("VALUE").unwrap()).unwrap_or_exitmsg(-1, "Invalid JSON argument");
            connect_to_port(args).set_pref(name, value).unwrap_or_exitmsg(-1, "Unable to set preference");
        }
        ("title", Some(ref args)) => println!("{}", connect_to_port(args).get_title().unwrap_or_exit(-1)),
        ("url", Some(ref args)) => println!("{}", connect_to_port(args).get_url().unwrap_or_exit(-1)),
        ("quit", Some(ref args)) => connect_to_port(args).quit().unwrap_or_exit(-1),
        ("start", Some(ref args)) => cmd_start(args).unwrap_or_exitmsg(-1, "Unable to start browser"),
        ("install", Some(ref args)) => cmd_install(args).unwrap_or_exitmsg(-1, "Unable to install addon"),
        ("instances", _) => cmd_instances().unwrap_or_exitmsg(-1, "Unable to list ff instances"),
        ("switch", Some(ref args)) => {
            let mut conn = connect_to_port(args);
            let handle = if args.is_present("index") {
                let idx = usize::from_str(args.value_of("WINDOW").unwrap())
                    .expect("Invalid WINDOW index");
                let mut handles = conn.get_window_handles()
                    .unwrap_or_exitmsg(-1, "Unable to get window list");
                let handle = handles.drain((..))
                    .nth(idx)
                    .unwrap_or_exitmsg(-1, "Index is invalid");
                handle
            } else {
                WindowHandle::from_str(args.value_of("WINDOW").unwrap())
            };
            conn.switch_to_window(&handle)
                .unwrap_or_exitmsg(-1, "Unable to switch window");
        }
        ("windows", Some(ref args)) => cmd_windows(args).unwrap_or_exit(-1),
        _ => panic!("Unsupported command"),
    }
}

