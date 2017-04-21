use std::str::FromStr;
use std::io::Result;
use std::env;

extern crate ff;
extern crate marionette;
use marionette::MarionetteConnection;
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
        .subcommand(SubCommand::with_name("quit")
                    .about("Close the browser"))
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
        ("quit", _) => conn.quit().unwrap(),
        _ => panic!("Unsupported command"),
    }
}

