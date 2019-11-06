pub mod blockchain;
pub mod blocks;
pub mod clusters;
pub mod disjoint;
pub mod transactions;

use std::collections::HashSet;
use std::result;

use blockchain::address::Address;
use blockchain::block::Block;

#[derive(Debug)]
pub struct EofError;

pub type Result<T> = result::Result<T, EofError>;

#[derive(Debug)]
pub enum ParseError {
    Eof,
    Invalid,
}

pub enum BlockMessage<'a> {
    OnBlock(Block<'a>),
    OnComplete(bool),
    OnError(ParseError),
}

pub enum TransactionMessage {
    OnTransaction(HashSet<Address>),
    OnComplete(bool),
    OnError(ParseError),
}

pub type ParseResult<T> = result::Result<T, ParseError>;

impl From<EofError> for ParseError {
    fn from(_: EofError) -> ParseError {
        ParseError::Eof
    }
}
