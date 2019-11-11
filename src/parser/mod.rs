use clap::{App, Arg};
use crossbeam_channel::bounded;
use crossbeam_utils::thread;
use fasthash::{xx, RandomState};
use std::result;
use chrono::{NaiveDateTime, DateTime};
use chrono::prelude::Utc;

use blockchain::address::Address;
use parser::blockchain::Blockchain;
use parser::blocks::Blocks;
use parser::clusters::Clusters;
//use parser::graph::Graph;
use parser::transactions::Transactions;
use parser::union::UnionFind;

pub mod blockchain;
pub mod blocks;
pub mod clusters;
//pub mod graph;
pub mod transactions;
pub mod union;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");
const BLOCKS_DIR: &'static str = "~/.bitcoin/blocks";
const OUTPUT: &'static str = "clusters.csv";
const QUEUE_SIZE: usize = 1000;

pub type Result<T> = result::Result<T, EofError>;

pub type ParseResult<T> = result::Result<T, ParseError>;

#[derive(Debug)]
pub struct EofError;

#[derive(Debug)]
pub enum ParseError {
    Eof,
    Invalid,
}

impl From<EofError> for ParseError {
    fn from(_: EofError) -> ParseError {
        ParseError::Eof
    }
}

pub struct Config {
    pub blocks_dir: String,
    pub input: String,
    pub output: String,
    pub max_block: usize,
    pub queue_size: usize,
}

impl Config {
    pub fn new() -> Self {
        let matches = App::new("Fast Blockchain Parser")
            .version(VERSION)
            .about("A Bitcoin blockchain parser with clustering capabilities")
            .arg(
                Arg::with_name("blocks_dir")
                    .help("Sets the path to the bitcoind blocks directory")
                    .long("blocks-dir")
                    .short("b")
                    .takes_value(true)
                    .default_value(BLOCKS_DIR),
            )
            .arg(
                Arg::with_name("input")
                    .help("Input file with started transactions")
                    .long("input")
                    .short("i")
                    .takes_value(true)
            )
            .arg(
                Arg::with_name("output")
                    .help("Output file")
                    .long("output")
                    .short("o")
                    .takes_value(true)
                    .default_value(OUTPUT),
            )
            .arg(
                Arg::with_name("max_block")
                    .help("Process up to blk0xxxx.dat file")
                    .long("max-block")
                    .short("m")
                    .takes_value(true),
            )
            .get_matches();

        let blocks_dir = matches.value_of("blocks_dir").unwrap().as_bytes().to_vec();
        let blocks_dir = String::from_utf8(blocks_dir).expect("Found invalid UTF-8");

        let output = matches.value_of("output").unwrap().as_bytes().to_vec();
        let output = String::from_utf8(output).expect("Found invalid UTF-8");

        let input = matches.value_of("input").unwrap().as_bytes().to_vec();
        let input = String::from_utf8(input).expect("Found invalid UTF-8");

        let max_block = match matches.value_of("max_block") {
            Some(max_block) => (max_block.parse().unwrap()),
            None => (0),
        };

        Config {
            blocks_dir: blocks_dir,
            input: input,
            output: output,
            max_block: max_block,
            queue_size: QUEUE_SIZE,
        }
    }
}

pub fn run(config: &Config, clusters: &mut UnionFind<Address, RandomState<xx::Hash64>>) {
    let blockchain: Blockchain = Blockchain::new(&config.blocks_dir, config.max_block);
    let (block_out, block_in) = bounded(2);
    let (tx_out, tx_in) = bounded(config.queue_size);

    thread::scope(|scope| {
        let _ = scope.spawn(|_| {
            let mut b = Blocks::new(block_out);
            b.run(&blockchain);
        });

        let _ = scope.spawn(|_| {
            let t = Transactions::new(block_in, tx_out);
            t.run();
        });

        //if clusters.is_empty() {
        info!("Processing clusters...");
        let _ = scope.spawn(|_| {
            let mut c = Clusters::new(tx_in, config);
            c.run(clusters);
        });
        /*
        } else {
            info!("Processing graph...");
            let _ = scope.spawn(|_| {
                let mut g = Graph::new(tx_in, config);
                g.run(clusters);
            });
            return;
        }
        */
    })
    .unwrap();
}

pub fn timestamp_to_date(timestamp: u32) -> String {
    let naive_datetime = NaiveDateTime::from_timestamp(timestamp as i64, 0);
    let datetime_again: DateTime<Utc> = DateTime::from_utc(naive_datetime, Utc);
    return datetime_again.format("%Y-%m-%d %H:%M:%S").to_string();
}
