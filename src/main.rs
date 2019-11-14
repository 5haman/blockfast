#![feature(toowned_clone_into)]

#[macro_use]
extern crate arrayref;

#[macro_use]
extern crate log;
extern crate colog;

extern crate base58;
extern crate bitcoin_bech32;
extern crate byteorder;
extern crate clap;
extern crate crypto;
extern crate dirs;
extern crate fasthash;
extern crate memmap;
extern crate rustc_serialize;
extern crate time;
extern crate vec_map;

use std::io::Write;

pub mod blockchain;
pub mod parser;

use parser::parser::Parser;
use parser::Config;

fn main() {
    let config = init();

    info!("Starting blockchain parser...");

    let mut parser = Parser::new(&config);
    parser.run();

    info!("Finished succesfully");
}

fn init() -> Config {
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

    let config = Config::new();
    return config;
}
