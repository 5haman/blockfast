#[macro_use]
extern crate arrayref;

#[macro_use]
extern crate log;
extern crate colog;

extern crate base58;
extern crate bitcoin_bech32;
extern crate byteorder;
extern crate clap;
extern crate crossbeam_channel;
extern crate crossbeam_utils;
extern crate crypto;
extern crate dirs;
extern crate memmap;
extern crate rayon;
extern crate rustc_serialize;
extern crate time;
extern crate vec_map;

pub mod blockchain;
pub mod reader;
pub mod types;

use clap::{App, Arg};
use crossbeam_channel::bounded;
use crossbeam_utils::thread;
use std::env;
use std::io::Write;

use reader::chain::Blockchain;
use reader::reader::Reader;
use reader::consumer:: Consumer;

#[cfg(all(target_env = "musl", target_pointer_width = "64"))]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");
const DEF_BLOCKS_DIR: &'static str = "~/.bitcoin/blocks";
const DEF_MAX_BLOCK: &'static str = "10";
const DEF_QUEUE_SIZE: &'static str = "1024";

fn main() {
    let matches = App::new("Fast Blockchain Parser")
        .version(VERSION)
        .about("A Bitcoin blockchain parser with clustering capabilities")
        .arg(
            Arg::with_name("blocks_dir")
                .help("Sets the path to the bitcoind blocks directory")
                .long("blocks-dir")
                .short("b")
                .takes_value(true)
                .default_value(DEF_BLOCKS_DIR),
        )
        .arg(
            Arg::with_name("max_block")
                .help("Process up to blk0xxxx.dat file")
                .long("max-block")
                .short("m")
                .takes_value(true)
                .default_value(DEF_MAX_BLOCK),
        )
        .arg(
            Arg::with_name("queue_size")
                .help("Size of wworkers queue")
                .long("queue-size")
                .short("q")
                .takes_value(true)
                .default_value(DEF_QUEUE_SIZE),
        )
        .get_matches();

    let mut clog = colog::builder();
    clog.filter(None, log::LevelFilter::Debug);
    clog.format(|buf, record| {
        let t = time::now();
        writeln!(
            buf,
            "{}.{:04} {}: {}",
            time::strftime("%Y-%m-%d %H:%M:%S", &t).unwrap(),
            t.tm_nsec / 100_000,
            record.level(),
            record.args()
        )
    });
    clog.init();

    info!("Starting blockchain parser...");

    let blocks_dir = matches.value_of("blocks_dir").unwrap();
    let max_block = matches.value_of("max_block").unwrap().parse().unwrap();
    let queue_size = matches.value_of("queue_size").unwrap().parse().unwrap();

    let chain = unsafe { Blockchain::read(blocks_dir, max_block) };
    let (tx, rx) = bounded(queue_size);

    thread::scope(|scope| {
        let _ = scope.spawn(|_| {
            let reader = Reader::new(tx);
            reader.run(&chain);
        });

        let _ = scope.spawn(|_| {
            let mut consumer = Consumer::new(rx);
            consumer.run();
        });
    })
    .unwrap();

    info!("Finished succesfully");
}
