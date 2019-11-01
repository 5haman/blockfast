use blockchain::buffer::{read_slice, read_u32, read_var_int};
use blockchain::error::{ParseError, ParseResult, Result};
use blockchain::header::BlockHeader;

#[derive(PartialEq, Eq, Clone, Copy)]
pub struct Block<'a>(&'a [u8]);

#[derive(PartialEq, Eq, Clone, Copy)]
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

    pub fn header(&self) -> BlockHeader<'a> {
        let mut slice = self.0;
        BlockHeader::new(read_array!(&mut slice, 80).unwrap())
    }

    pub fn transactions(&self) -> Result<Transactions<'a>> {
        Transactions::new(&self.0[80..])
    }
}

impl<'a> Transactions<'a> {
    pub fn new(mut slice: &[u8]) -> Result<Transactions> {
        let count = read_var_int(&mut slice)?;
        Ok(Transactions { count, slice })
    }
}
