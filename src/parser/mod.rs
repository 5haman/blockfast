use clap::{App, Arg};
use crossbeam_channel::bounded;
use crossbeam_utils::thread;
use std::collections::HashMap;
use std::result;

use blockchain::address::Address;
use parser::blockchain::Blockchain;
use parser::blocks::Blocks;
use parser::clusters::Clusters;
//use disjoint_sets::UnionFind;
use disjoint_sets::UnionFind;
use parser::graph::Graph;
use parser::transactions::Transactions;

pub mod blockchain;
pub mod blocks;
pub mod clusters;
//pub mod disjoint;
pub mod graph;
pub mod transactions;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");
const BLOCKS_DIR: &'static str = "~/.bitcoin/blocks";
const OUTPUT: &'static str = "clusters.csv";
const GRAPH: &'static str = "blockchain.mtx";
const MAX_BLOCK: &'static str = "10";
const QUEUE_SIZE: usize = 10_000_000;

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
    pub output: String,
    pub graph: String,
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
                Arg::with_name("output")
                    .help("Output file")
                    .long("output")
                    .short("o")
                    .takes_value(true)
                    .default_value(OUTPUT),
            )
            .arg(
                Arg::with_name("graph")
                    .help("Graph output file")
                    .long("graph")
                    .short("g")
                    .takes_value(true)
                    .default_value(GRAPH),
            )
            .arg(
                Arg::with_name("max_block")
                    .help("Process up to blk0xxxx.dat file")
                    .long("max-block")
                    .short("m")
                    .takes_value(true)
                    .default_value(MAX_BLOCK),
            )
            .get_matches();

        let blocks_dir = matches.value_of("blocks_dir").unwrap().as_bytes().to_vec();
        let blocks_dir = String::from_utf8(blocks_dir).expect("Found invalid UTF-8");

        let output = matches.value_of("output").unwrap().as_bytes().to_vec();
        let output = String::from_utf8(output).expect("Found invalid UTF-8");

        let graph = matches.value_of("graph").unwrap().as_bytes().to_vec();
        let graph = String::from_utf8(graph).expect("Found invalid UTF-8");

        Config {
            blocks_dir: blocks_dir,
            output: output,
            graph: graph,
            max_block: matches.value_of("max_block").unwrap().parse().unwrap(),
            queue_size: QUEUE_SIZE,
        }
    }
}

pub fn run(
    config: &Config,
    clusters: &mut UnionFind,
    addresses: &mut HashMap<Address, u32>,
    iter: u8,
) {
    let blockchain: Blockchain = Blockchain::new(&config.blocks_dir, config.max_block);
    let (block_out, block_in) = bounded(config.queue_size);
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

        if iter == 0 {
            info!("Processing clusters...");
            let _ = scope.spawn(|_| {
                let mut c = Clusters::new(tx_in, config);
                c.run(clusters, addresses);
            });
        } else {
            info!("Processing graph...");
            let _ = scope.spawn(|_| {
                let mut g = Graph::new(tx_in, config);
                g.run(clusters, addresses);
            });
            return;
        }
    })
    .unwrap();
}
