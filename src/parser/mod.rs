use clap::{App, Arg};
use std::result;

pub mod blockchain;
pub mod clusters;
pub mod parser;
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
                    .takes_value(true),
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
