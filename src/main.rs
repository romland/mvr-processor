/*
for testing, on raspi:
/usr/bin/raspivid --verbose --inline --spstimings --hflip --vflip --annotate 1036 --annotate " My Awesome Sensor \n %Y-%m-%d %X " --annotateex 16,0x00,0x4C96B0,2,0,990 --width 1920 --height 1080 --timeout 0 --framerate 2 --bitrate 1700000 --profile baseline --vectors tcp://192.168.178.100:8001 --output - > /dev/null

to test command line args:
cargo run -- --version

*/
mod dbscan;
mod mvrprocessor;

use std::str::FromStr;
use std::net::{TcpListener, SocketAddr};
use std::thread::spawn;
use bufstream::BufStream;
use std::sync::{Arc,RwLock,mpsc};
use std::sync::mpsc::{Sender, Receiver};


#[allow(unused_mut)]
fn main()
{
	/*
    let args = match parse_args() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error: {}.", e);
            std::process::exit(1);
        }
    };

    println!("debug args: {:#?}", args);
	*/

	let addr: SocketAddr = SocketAddr::from_str("0.0.0.0:8001").unwrap();
    let listener = TcpListener::bind(addr).unwrap();

    let (send, recv): (Sender<String>, Receiver<String>) = mpsc::channel();
    let arc: Arc<RwLock<Vec<String>>> = Arc::new(RwLock::new(Vec::new()));

    let arc_w = arc.clone();

    spawn(move|| {
        loop {
            let msg = recv.recv().unwrap();
            print!("DEBUG: msg {}", msg);
            {
                let mut arc_w = arc_w.write().unwrap();
                arc_w.push(msg);
            } // write lock is released at the end of this scope
        }
    });

    for stream in listener.incoming() {
        match stream {
            Err(_) => println!("listen error"),
            Ok(mut stream) => {
                // println!("connection from {} to {}",
                //          stream.peer_addr().unwrap(),
                //          stream.local_addr().unwrap());
                let send = send.clone();
                let arc = arc.clone();
                spawn(move|| {
                    let mut stream = BufStream::new(stream);
                    mvrprocessor::handle_raw_mvr_connection(&mut stream, send, arc);
                });
            }
        }
    }
}



/*
todo cmd line args:
	- version
	- vector min magnitude
		default 2
	- cluster epsilon
		default 2
	- cluster min points
		defaul 4
	- listen address
		default 127.0.0.1
	- listen port
		default 8001
	- width in vectors
		default 121
	- height in vectors
		default 68
	- output-type
		- json (default)
		- debug (will not output json...)
		- full-screen render of ascii
	- ignore area
	- discardInactiveAfter
	- sadThreshold (a TODO in node too!) but wow, this worked really well
*/
#[allow(dead_code)]
const HELP: &str = "\
Xorzee MVR
USAGE:
  mvr [OPTIONS]
FLAGS:
  -h, --help            This help information
OPTIONS:
  --number NUMBER       Sets a number
  --opt-number NUMBER   Sets an optional number
  --width WIDTH         Sets width [default: 10]
  --output PATH         Sets an output path
";

#[allow(dead_code)]
#[derive(Debug)]
struct AppArgs {
    number: u32,
    opt_number: Option<u32>,
    width: u32,
    input: std::path::PathBuf,
    output: Option<std::path::PathBuf>,
}


#[allow(dead_code)]
fn parse_args() -> Result<AppArgs, pico_args::Error> {
    let mut pargs = pico_args::Arguments::from_env();

    // Help has a higher priority and should be handled separately.
    if pargs.contains(["-h", "--help"]) {
        print!("{}", HELP);
        std::process::exit(0);
    }

    let args = AppArgs {
        // Parses a required value that implements `FromStr`.
        // Returns an error if not present.
        number: pargs.value_from_str("--number")?,
        // Parses an optional value that implements `FromStr`.
        opt_number: pargs.opt_value_from_str("--opt-number")?,
        // Parses an optional value from `&str` using a specified function.
        width: pargs.opt_value_from_fn("--width", parse_width)?.unwrap_or(10),
        // Parses an optional value from `&OsStr` using a specified function.
        output: pargs.opt_value_from_os_str("--input", parse_path)?,
        // Parses a required free-standing/positional argument.
        input: pargs.free_from_str()?,
    };

    // It's up to the caller what to do with the remaining arguments.
    let remaining = pargs.finish();
    if !remaining.is_empty() {
        eprintln!("Warning: unused arguments left: {:?}.", remaining);
    }

    Ok(args)
}

#[allow(dead_code)]
fn parse_width(s: &str) -> Result<u32, &'static str> {
    s.parse().map_err(|_| "not a number")
}

#[allow(dead_code)]
fn parse_path(s: &std::ffi::OsStr) -> Result<std::path::PathBuf, &'static str> {
    Ok(s.into())
}
