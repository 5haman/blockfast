use std::collections::HashSet;
use std::result;

use blockchain::address::Address;

#[derive(Debug)]
pub struct EofError;

pub type Result<T> = result::Result<T, EofError>;

#[derive(Debug)]
pub enum ParseError {
    Eof,
    Invalid,
}

pub enum ThreadResult {
    OnTransaction(HashSet<Address>),
    OnComplete(String),
    OnError(ParseError),
}

pub type ParseResult<T> = result::Result<T, ParseError>;

impl From<EofError> for ParseError {
    fn from(_: EofError) -> ParseError {
        ParseError::Eof
    }
}
