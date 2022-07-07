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
	#[allow(unused_variables)]
    let config = match parse_args() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error: {}.", e);
            std::process::exit(1);
        }
    };

    println!("debug args: {:#?}", config);
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


#[allow(dead_code)]
const HELP: &str = "\
Xorzee MVR
USAGE:
  mvr [OPTIONS]
FLAGS:
  --help                This help information
OPTIONS:
  --version             Outputs version of Xorzee MVR.
  --width NUMBER        Sets screen width in motion vectors.
                        (deafult: 121 for 1920)
  --height NUMBER       Sets screen height in motion vectors.
                        (default: 68 for 1080)
  --minmagnitude NUMBER Sets minimum magnitude for a vector
                        to count as active.
                        (default: 2)
  --epsilon NUMBER      Sets maximum distance for points to 
                        belong to a cluster.
                        (default: 2)
  --minpoints NUMBER    Sets minimum number of points to classify
                        something as a cluster.
                        (default: 4)
  --listen ADDRESS      Sets IP address to listen to.
                        (default: 127.0.0.1)
  --port PORT           Sets port to listen to.
                        (default: 8001)
  --output [JSON|DEBUG] Set output to JSON or DEBUG.
                        (default: JSON)
  --ignore POLYGONS     Set polygons to specify areas that should
                        be ignored.
                        (default: none)
  --discardafter NUMBER Set the time for which clusters should be 
                        discarded if they are inactive.
                        (default: 2000)
  --sadthreshold NUMBER Set the minimum SAD that needs to be met to
                        classify a block as active.
                        (default: 250)
";

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct AppArgs {
    width: usize,
    height: usize,
    minmagnitude: f32,
    epsilon: f32,
    minpoints: usize,
    listen: String,
    port: String,
    output: String,
    ignore: String,
    discardafter: u32,
    sadthreshold: u32

    // number: u32,
    // opt_number: u32,
    // input: Option<std::path::PathBuf>,
    // output: td::path::PathBuf,
}

/*
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
        // number: pargs.value_from_str("--number")?,

        // Parses an optional value that implements `FromStr`.
        // opt_number: pargs.opt_value_from_str("--opt-number")?,

        // Parses an optional value from `&str` using a specified function.
        width: pargs.opt_value_from_fn("--width", parse_number)?.unwrap_or(121),
        height: pargs.opt_value_from_fn("--height", parse_number)?.unwrap_or(68),
        minmagnitude: pargs.opt_value_from_fn("--minmagnitude", parse_number)?.unwrap_or(2),
        epsilon: pargs.opt_value_from_fn("--epsilon", parse_number)?.unwrap_or(2),
        minpoints: pargs.opt_value_from_fn("--minpoints", parse_number)?.unwrap_or(4),

        listen: pargs.opt_value_from_fn("--listen", parse_ip)?.unwrap_or("127.0.0.1"),
        port: pargs.opt_value_from_fn("--port", parse_port)?.unwrap_or("8001"),
        output: pargs.opt_value_from_fn("--output", parse_output)?.unwrap_or("JSON"),
        ignore: pargs.opt_value_from_fn("--ignore", parse_polygons)?.unwrap_or(""),
        discardafter: pargs.opt_value_from_fn("--discardafter", parse_number)?.unwrap_or(2000),
        sadthreshold: pargs.opt_value_from_fn("--sadthreshold", parse_number)?.unwrap_or(250),
    
        // Parses an optional value from `&OsStr` using a specified function.
        // output: pargs.opt_value_from_os_str("--input", parse_path)?,

        // Parses a required free-standing/positional argument.
        // input: pargs.free_from_str()?,
    };

    // It's up to the caller what to do with the remaining arguments.
    let remaining = pargs.finish();
    if !remaining.is_empty() {
        eprintln!("Warning: unused arguments left: {:?}.", remaining);
    }

    Ok(args)
}
*/
