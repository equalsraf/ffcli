use std::str::FromStr;
use std::io::Result;
use std::env;

extern crate ff;
extern crate marionette;
use marionette::{MarionetteConnection, Element, JsonValue, WindowHandle};
use marionette::QueryMethod::CssSelector;
extern crate clap;
use clap::{App, Arg, ArgMatches, SubCommand, AppSettings};
extern crate stderrlog;
extern crate url;

fn cmd_start(port: Option<u16>, _: &ArgMatches) -> std::result::Result<ff::Browser, ff::RunnerError> {
    let mut browser = ff::Browser::start(port)?;
    browser.kill_on_drop(false);
    println!("FF_PORT={}", browser.port());
    Ok(browser)
}

fn cmd_go(conn: &mut MarionetteConnection, args: &ArgMatches) -> Result<()> {
    let url_arg = args.value_of("URL").unwrap();

    if let Err(url::ParseError::RelativeUrlWithoutBase) = url::Url::parse(url_arg) {
        conn.get(&("https://".to_owned() + url_arg))
    } else {
        conn.get(url_arg)
    }
}

fn cmd_back(conn: &mut MarionetteConnection, _: &ArgMatches) -> Result<()> {
    conn.go_back()
}

fn cmd_forward(conn: &mut MarionetteConnection, _: &ArgMatches) -> Result<()> {
    conn.go_forward()
}

fn cmd_windows(conn: &mut MarionetteConnection, _: &ArgMatches) -> Result<()> {
    let prev = conn.get_window_handle()?;
    for win in conn.get_window_handles()? {
        conn.switch_to_window(&win)?;
        let title = conn.get_title()?;
        println!("{} \"{}\"", win, title);
    }

    conn.switch_to_window(&prev)?;
    Ok(())
}

/// Filter elements based on a selector, then map them to a string
fn cmd_get_element_str_data<F>(conn: &mut MarionetteConnection, args: &ArgMatches, f: F) -> Result<()> 
        where F: Fn(&mut Element) -> Result<String> {

    for elemref in conn.find_elements(CssSelector, args.value_of("SELECTOR").unwrap(), None)? {
        let mut elem = Element::new(conn, &elemref);
        let text = f(&mut elem)?;
        if !text.is_empty() {
            println!("{}", text);
        }
    }
    Ok(())
}

/// Filter elements based on a selector, then map them to a json value
fn cmd_get_element_json_data<F>(conn: &mut MarionetteConnection, args: &ArgMatches, f: F) -> Result<()> 
        where F: Fn(&mut Element) -> Result<JsonValue> {

    for elemref in conn.find_elements(CssSelector, args.value_of("SELECTOR").unwrap(), None)? {
        let mut elem = Element::new(conn, &elemref);
        let val = f(&mut elem)?;
        if val != JsonValue::Null {
            println!("{}", val);
        }
    }
    Ok(())
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
                    .about("Start a new browser instance"))
        .subcommand(SubCommand::with_name("go")
                    .about("Navigate to URL")
                    .arg(Arg::with_name("URL")
                         .required(true)
                        ))
        .subcommand(SubCommand::with_name("back")
                    .about("Go back to the previous page in history"))
        .subcommand(SubCommand::with_name("forward")
                    .about("Go forward to the next page in history"))
        .subcommand(SubCommand::with_name("source")
                    .about("Print page source"))
        .subcommand(SubCommand::with_name("attr")
                    .arg(Arg::with_name("SELECTOR")
                         .required(true))
                    .arg(Arg::with_name("ATTRNAME")
                         .required(true))
                    .about("Print element attribute"))
        .subcommand(SubCommand::with_name("property")
                    .arg(Arg::with_name("SELECTOR")
                         .required(true))
                    .arg(Arg::with_name("NAME")
                         .required(true))
                    .about("Print element property"))
        .subcommand(SubCommand::with_name("text")
                    .arg(Arg::with_name("SELECTOR")
                         .required(true))
                    .about("Print element text"))
        .subcommand(SubCommand::with_name("title")
                    .about("Print page title"))
        .subcommand(SubCommand::with_name("url")
                    .about("Print page url"))
        .subcommand(SubCommand::with_name("quit")
                    .about("Close the browser"))
        .subcommand(SubCommand::with_name("windows")
                    .about("List browser windows"))
        .subcommand(SubCommand::with_name("switch")
                    .arg(Arg::with_name("WINDOW")
                         .required(true))
                    .about("Switch browser window"))
        .get_matches();

    stderrlog::new()
            .module(module_path!())
            .verbosity(matches.occurrences_of("verbose") as usize)
            .init()
            .expect("Unable to initialize stderr output");

    let port_arg = matches.value_of("PORT")
        .map(|val| val.to_owned())
        .or_else(|| env::var("FF_PORT").ok())
        .map(|ref s| u16::from_str(s).expect("Invalid port argument"));

    // start browser and exit
    if let Some(ref args) = matches.subcommand_matches("start") {
        cmd_start(port_arg, args).expect("Unable to start browser");
        return;
    }

    let port = port_arg.expect("No port given, use --port or $FF_PORT");

    let mut conn = MarionetteConnection::connect(port)
        .expect("Unable to connect to firefox");

    match matches.subcommand() {
        ("go", Some(ref args)) => cmd_go(&mut conn, args).unwrap(),
        ("back", Some(ref args)) => cmd_back(&mut conn, args).unwrap(),
        ("forward", Some(ref args)) => cmd_forward(&mut conn, args).unwrap(),
        ("source", _) => println!("{}", conn.get_page_source().unwrap()),
        ("text", Some(ref args)) =>
            cmd_get_element_str_data(&mut conn, args, |e| e.text()).unwrap(),
        ("attr", Some(ref args)) => {
            let attrname = args.value_of("ATTRNAME").unwrap();
            cmd_get_element_str_data(&mut conn, args, |e| e.attr(attrname)).unwrap();
        }
        ("property", Some(ref args)) => {
            let propname = args.value_of("NAME").unwrap();
            cmd_get_element_json_data(&mut conn, args, |e| e.property(propname)).unwrap();
        }
        ("title", _) => println!("{}", conn.get_title().unwrap()),
        ("url", _) => println!("{}", conn.get_url().unwrap()),
        ("quit", _) => conn.quit().unwrap(),
        ("switch", Some(ref args)) => {
            let handle = WindowHandle::from_str(args.value_of("WINDOW").unwrap());
            conn.switch_to_window(&handle).unwrap();
        }
        ("windows", Some(ref args)) => cmd_windows(&mut conn, args).unwrap(),
        _ => panic!("Unsupported command"),
    }
}

