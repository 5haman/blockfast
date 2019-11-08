#[macro_use]
extern crate arrayref;

#[macro_use]
extern crate log;
extern crate colog;

extern crate advanced_collections;
extern crate base58;
extern crate bitcoin_bech32;
extern crate byteorder;
extern crate clap;
extern crate crossbeam_channel;
extern crate crossbeam_utils;
extern crate crypto;
extern crate dirs;
extern crate hash_hasher;
extern crate memmap;
extern crate rustc_serialize;
extern crate time;
extern crate vec_map;

use hash_hasher::HashBuildHasher;
use std::io::Write;

pub mod blockchain;
pub mod parser;

use blockchain::address::Address;
use parser::union::UnionFind;
use parser::Config;

fn main() {
    let config = init();

    info!("Starting blockchain parser...");

    let mut clusters: UnionFind<Address, HashBuildHasher> =
        UnionFind::with_hasher(HashBuildHasher::default());
    parser::run(&config, &mut clusters);
    parser::run(&config, &mut clusters);

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
