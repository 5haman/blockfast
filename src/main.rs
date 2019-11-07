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
pub mod parser;

use std::collections::HashMap;
use std::io::Write;

use blockchain::address::Address;
use parser::disjoint::UnionFind;
use parser::Config;

const MAX_CLUSTERS: usize = 10_000_000;

fn main() {
    let config = init();

    info!("Starting blockchain parser...");

    let mut addresses: HashMap<Address, u32> = HashMap::with_capacity(MAX_CLUSTERS);
    let mut clusters = UnionFind::new(MAX_CLUSTERS);
    parser::run(&config, &mut clusters, &mut addresses);
    parser::run(&config, &mut clusters, &mut addresses);

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
