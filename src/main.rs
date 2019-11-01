#[macro_use]
extern crate arrayref;

#[macro_use]
extern crate crossbeam_channel;

#[macro_use]
extern crate log;

extern crate base58;
extern crate bitcoin_bech32;
extern crate byteorder;
extern crate clap;
extern crate crossbeam_utils;
extern crate crypto;
extern crate dirs;
extern crate env_logger;
extern crate memmap;
extern crate rustc_serialize;
extern crate time;
extern crate vec_map;

pub mod blockchain;
pub mod parser;
pub mod visitors;

use clap::{App, Arg};
use crossbeam_channel::bounded;
use crossbeam_utils::thread;
use env_logger::Builder;
use log::LevelFilter;
use std::env;
use std::io::Write;

use blockchain::chain::Blockchain;
use parser::{Parser, ThreadReceiver};

#[cfg(all(target_env = "musl", target_pointer_width = "64"))]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

fn main() {
    let default_blocks_dir = dirs::home_dir()
        .expect("Unable to get the home directory!")
        .join(".bitcoin")
        .join("blocks")
        .into_os_string()
        .into_string()
        .expect("Unable to build a default bitcoind blocks directory");

    let default_max_block = "10";
    let default_queue_size = "1024";

    let matches = App::new("Fast Blockchain Parser")
        .version(VERSION)
        .about("A Bitcoin blockchain parser with clustering capabilities")
        .arg(
            Arg::with_name("blocks_dir")
                .help("Sets the path to the bitcoind blocks directory")
                .long("blocks-dir")
                .short("b")
                .takes_value(true)
                .default_value(&default_blocks_dir),
        )
        .arg(
            Arg::with_name("max_block")
                .help("Process up to blk0xxxx.dat file")
                .long("max-block")
                .short("m")
                .takes_value(true)
                .default_value(&default_max_block),
        )
        .arg(
            Arg::with_name("queue_size")
                .help("Size of wworkers queue")
                .long("queue-size")
                .short("q")
                .takes_value(true)
                .default_value(&default_queue_size),
        )
        .get_matches();

    initialize_logger();

    info!("Starting blockchain parser...");

    let blocks_dir = matches.value_of("blocks_dir").unwrap();
    let max_block = matches.value_of("max_block").unwrap().parse().unwrap();
    let queue_size = matches.value_of("queue_size").unwrap().parse().unwrap();
    let chain = unsafe { Blockchain::read(blocks_dir, max_block) };
    let (tx, rx) = bounded(queue_size);

    thread::scope(|scope| {
        let handle1 = scope.spawn(|_| {
            let parser = Parser::new(tx);
            parser.handle(&chain);
        });

        let handle2 = scope.spawn(|_| {
            let receiver = ThreadReceiver::new(rx);
            receiver.handle();
        });
    })
    .unwrap();

    info!("Finished succesfully");
}

fn initialize_logger() {
    Builder::new()
        .filter(None, LevelFilter::Info)
        .format(|buf, record| {
            let t = time::now();
            writeln!(
                buf,
                "{}.{:02} [{}] {}",
                time::strftime("%Y-%m-%d %H:%M:%S", &t).unwrap(),
                t.tm_nsec / 100_000,
                record.level(),
                record.args()
            )
        })
        .init();
}
