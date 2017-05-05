use std::str::FromStr;
use std::env;
use std::process::{Command, Stdio};

extern crate ff;
extern crate marionette;
use marionette::{MarionetteConnection, Element, JsonValue, WindowHandle, Result};
use marionette::QueryMethod::CssSelector;
extern crate clap;
use clap::{App, Arg, ArgMatches, SubCommand, AppSettings};
#[macro_use]
extern crate log;
extern crate stderrlog;
extern crate url;
#[cfg(unix)]
extern crate chan_signal;

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
        .map(|ref s| u16::from_str(s).expect("Invalid port argument"));

    // Check TCP port availability
    let portnum = ff::check_tcp_port(port_arg)?;

    if args.is_present("no-fork") {
        setup_signals();

        let mut browser = ff::Browser::start(portnum)?;
        debug!("New ff session {}", browser.session_file().to_string_lossy());

        let status = browser.runner.process.wait()?;
        info!("Firefox exited with status {}", status);
    } else {
        let mut args: Vec<_> = env::args().collect();
        args.push("--no-fork".to_owned());
        if port_arg.is_none() {
            args.push("--port".to_owned());
            args.push(format!("{}", portnum));
        }

        debug!("Spawning ff process {:?}", args);
        Command::new(&args[0])
            .args(&args[1..])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .spawn()?;

        ff::check_connection(portnum)?;
        println!("FF_PORT={}", portnum);
    }

    Ok(())
}

fn cmd_instances() -> Result<()> {
    for instance in ff::instances()? {
        println!("{}", instance);
    }
    Ok(())
}

fn cmd_go(args: &ArgMatches) -> Result<()> {
    let url_arg = args.value_of("URL").unwrap();

    let mut conn = connect_to_port(args);
    if let Err(url::ParseError::RelativeUrlWithoutBase) = url::Url::parse(url_arg) {
        conn.get(&("https://".to_owned() + url_arg))
    } else {
        conn.get(url_arg)
    }
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

/// Filter elements based on a selector, then map them to a string
fn cmd_get_element_str_data<F>(conn: &mut MarionetteConnection, args: &ArgMatches, f: &F) -> Result<()>
        where F: Fn(&mut Element) -> Result<String> {

    let selector =  args.value_of("SELECTOR").unwrap();
    for elemref in conn.find_elements(CssSelector, selector, None)? {
        let mut elem = Element::new(conn, &elemref);
        let text = f(&mut elem)?;
        if !text.is_empty() {
            println!("{}", text);
        }
    }

    for frameref in conn.find_elements(CssSelector, FRAME_SELECTOR, None)? {
        conn.switch_to_frame(&frameref)?;
        cmd_get_element_str_data(conn, args, f)?;
        conn.switch_to_top_frame()?;
    }
    Ok(())
}

/// Filter elements based on a selector, then map them to a json value
fn cmd_get_element_json_data<F>(conn: &mut MarionetteConnection, args: &ArgMatches, f: &F) -> Result<()> 
        where F: Fn(&mut Element) -> Result<JsonValue> {

    for elemref in conn.find_elements(CssSelector, args.value_of("SELECTOR").unwrap(), None)? {
        let mut elem = Element::new(conn, &elemref);
        let val = f(&mut elem)?;
        if val != JsonValue::Null {
            println!("{}", val);
        }
    }

    for frameref in conn.find_elements(CssSelector, FRAME_SELECTOR, None)? {
        conn.switch_to_frame(&frameref)?;
        cmd_get_element_json_data(conn, args, f)?;
        conn.switch_to_top_frame()?;
    }
    Ok(())
}

/// Panics unless --port of $FF_PORT is a valid port number
fn connect_to_port(args: &ArgMatches) -> MarionetteConnection {
    let port_arg = args.value_of("PORT")
        .map(|val| val.to_owned())
        .or_else(|| env::var("FF_PORT").ok())
        .map(|ref s| u16::from_str(s).expect("Invalid port argument"));

    let port = port_arg.expect("No port given, use --port or $FF_PORT");
    MarionetteConnection::connect(port)
        .expect("Unable to connect to firefox")
}

fn option_port<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name("PORT")
        .takes_value(true)
        .long("port")
}

fn main() {
    let matches = App::new("ff")
        .about("Firefox from your shell")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .setting(AppSettings::VersionlessSubcommands)
        .arg(Arg::with_name("PORT")
             .takes_value(true)
             .long("port"))
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
                    .about("Start a new browser instance"))
        .subcommand(SubCommand::with_name("go")
                    .arg(option_port())
                    .about("Navigate to URL")
                    .arg(Arg::with_name("URL")
                         .required(true)
                        ))
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
        .subcommand(SubCommand::with_name("property")
                    .arg(option_port())
                    .arg(Arg::with_name("SELECTOR")
                         .required(true))
                    .arg(Arg::with_name("NAME")
                         .required(true))
                    .about("Print element property"))
        .subcommand(SubCommand::with_name("text")
                    .arg(option_port())
                    .arg(Arg::with_name("SELECTOR")
                         .required(true))
                    .about("Print element text"))
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
                    .arg(Arg::with_name("WINDOW")
                         .required(true))
                    .about("Switch browser window"))
        .get_matches();

    stderrlog::new()
            .module(module_path!())
            .verbosity(matches.occurrences_of("verbose") as usize)
            .init()
            .expect("Unable to initialize stderr output");


    match matches.subcommand() {
        ("go", Some(ref args)) => cmd_go(args).unwrap(),
        ("back", Some(ref args)) => connect_to_port(args).go_back().unwrap(),
        ("forward", Some(ref args)) => connect_to_port(args).go_forward().unwrap(),
        ("source", Some(ref args)) => println!("{}", connect_to_port(args).get_page_source().unwrap()),
        ("text", Some(ref args)) => {
            let mut conn = connect_to_port(args);
            cmd_get_element_str_data(&mut conn, args, &|e| e.text()).unwrap();
            conn.switch_to_top_frame().unwrap();
        }
        ("attr", Some(ref args)) => {
            let mut conn = connect_to_port(args);
            let attrname = args.value_of("ATTRNAME").unwrap();
            cmd_get_element_str_data(&mut conn, args, &|e| e.attr(attrname)).unwrap();
            conn.switch_to_top_frame().unwrap();
        }
        ("property", Some(ref args)) => {
            let mut conn = connect_to_port(args);
            let propname = args.value_of("NAME").unwrap();
            cmd_get_element_json_data(&mut conn, args, &|e| e.property(propname)).unwrap();
            conn.switch_to_top_frame().unwrap();
        }
        ("title", Some(ref args)) => println!("{}", connect_to_port(args).get_title().unwrap()),
        ("url", Some(ref args)) => println!("{}", connect_to_port(args).get_url().unwrap()),
        ("quit", Some(ref args)) => connect_to_port(args).quit().unwrap(),
        ("start", Some(ref args)) => cmd_start(args).expect("Unable to start browser"),
        ("instances", _) => cmd_instances().expect("Unable to list ff instances"),
        ("switch", Some(ref args)) => {
            let mut conn = connect_to_port(args);
            let handle = WindowHandle::from_str(args.value_of("WINDOW").unwrap());
            conn.switch_to_window(&handle).unwrap();
        }
        ("windows", Some(ref args)) => cmd_windows(args).unwrap(),
        _ => panic!("Unsupported command"),
    }
}

