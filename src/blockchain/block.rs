use byteorder::{LittleEndian, ReadBytesExt};

use blockchain::buffer::*;
use blockchain::hash::Hash;
use types::{ParseError, ParseResult};

pub struct BlockHeader<'a>(&'a [u8; 80]);

#[derive(PartialEq, Eq, Clone, Copy)]
pub struct Block<'a>(pub &'a [u8]);

pub struct Transactions<'a> {
    pub count: u64,
    pub slice: &'a [u8],
}

impl<'a> Block<'a> {
    pub fn read(slice: &mut &'a [u8]) -> ParseResult<Block<'a>> {
        while slice.len() > 0 && slice[0] == 0 {
            *slice = &slice[1..];
        }
        if slice.len() == 0 {
            Err(ParseError::Eof)
        } else {
            let block_magic = read_u32(slice)?;
            match block_magic {
                // Incomplete blk file
                0x00 => Err(ParseError::Eof),
                // Bitcoin magic value
                0xd9b4bef9 => {
                    let block_len = read_u32(slice)? as usize;
                    if block_len < 80 {
                        Err(ParseError::Eof)
                    } else {
                        Ok(Block(read_slice(slice, block_len)?))
                    }
                }
                _ => Err(ParseError::Invalid),
            }
        }
    }

    pub fn header(&self) -> BlockHeader {
        let mut slice = self.0;
        BlockHeader::new(read_array!(&mut slice, 80).unwrap())
    }

    pub fn transactions(&self) -> Transactions {
        Transactions::new(&self.0[80..])
    }
}

impl<'a> Transactions<'a> {
    pub fn new(mut slice: &[u8]) -> Transactions {
        let count = read_var_int(&mut slice);
        match count {
            Ok(count) => Transactions { count, slice },
            Err(_) => Transactions { count: 0, slice },
        }
    }
}

impl<'a> BlockHeader<'a> {
    pub fn new(slice: &[u8; 80]) -> BlockHeader {
        BlockHeader(slice)
    }

    pub fn cur_hash(&self) -> Hash {
        Hash::from_data(self.0)
    }

    pub fn prev_hash(&self) -> &'a Hash {
        Hash::from_slice(array_ref!(self.0, 4, 32))
    }

    pub fn timestamp(&self) -> u32 {
        let mut slice = &self.0[68..];
        slice.read_u32::<LittleEndian>().unwrap()
    }
}
