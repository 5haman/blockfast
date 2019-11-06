use bitcoin_bech32::constants::Network;
use bitcoin_bech32::WitnessProgram;

use blockchain::bytecode::Bytecode;
use blockchain::bytecode::Bytecode::*;
use parser::{ParseError, ParseResult};

#[derive(PartialEq, Clone)]
pub enum ScriptType<'a> {
    Pubkey(&'a [u8]),
    PubkeyHash(&'a [u8; 20]),
    WitnessPubkeyHash(WitnessProgram),
    Multisig(u32, Vec<&'a [u8]>),
    ScriptHash(&'a [u8; 20]),
    WitnessScriptHash(WitnessProgram),
    Unknown(Script<'a>),
    Invalid,
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub struct Script<'a> {
    slice: &'a [u8],
    timestamp: u32,
}

#[derive(PartialEq, Eq, Clone, Copy)]
struct ScriptIter<'a> {
    slice: &'a [u8],
    timestamp: u32,
}

impl<'a> Script<'a> {
    pub fn new(slice: &'a [u8], timestamp: u32) -> Script<'a> {
        Script { slice, timestamp }
    }

    fn iter(&self) -> ScriptIter<'a> {
        ScriptIter {
            slice: self.slice,
            timestamp: self.timestamp,
        }
    }

    pub fn as_slice(&self) -> &'a [u8] {
        self.slice
    }

    pub fn to_scripttype(&self) -> ScriptType<'a> {
        let mut skipped_iter = self.iter();
        skipped_iter.skip_nops();
        let skipped_slice = skipped_iter.slice;

        match skipped_slice.len() {
            22 => {
                if self.timestamp >= 1503539857 {
                    if &self.slice[..2] == &[0x00, 0x14] {
                        return match WitnessProgram::from_scriptpubkey(
                            &self.slice[..22],
                            Network::Bitcoin,
                        ) {
                            Ok(w) => ScriptType::WitnessPubkeyHash(w),
                            Err(_) => ScriptType::Invalid,
                        };
                    }
                }
            }
            25 => {
                if &skipped_slice[..3] == &[0x76, 0xa9, 0x14]
                    && (&skipped_slice[23..] == &[0x88, 0xac]
                        || &skipped_slice[23..] == &[0x88, 0xac, 0x61])
                {
                    return ScriptType::PubkeyHash(array_ref!(skipped_slice, 3, 20));
                }
                if self.timestamp >= 1333238400 {
                    if &self.slice[..2] == &[0xa9, 0x14] && self.slice[22] == 0x87 {
                        return ScriptType::ScriptHash(array_ref!(self.slice, 2, 20));
                    }
                }
            }
            26 => {
                if &skipped_slice[..3] == &[0x76, 0xa9, 0x14]
                    && &skipped_slice[23..] == &[0x88, 0xac, 0x61]
                {
                    return ScriptType::PubkeyHash(array_ref!(skipped_slice, 3, 20));
                }
            }
            34 => {
                if self.timestamp >= 1503539857 {
                    if &self.slice[..2] == &[0x00, 0x20] {
                        return match WitnessProgram::from_scriptpubkey(
                            &self.slice[..34],
                            Network::Bitcoin,
                        ) {
                            Ok(w) => ScriptType::WitnessScriptHash(w),
                            Err(_) => ScriptType::Invalid,
                        };
                    }
                }
            }
            35 => {
                if skipped_slice[0] == 33 && skipped_slice[34] == 0xac {
                    let pubkey = &skipped_slice[1..1 + 33];
                    if is_valid_pubkey(pubkey) {
                        return ScriptType::Pubkey(pubkey);
                    } else {
                        return ScriptType::Invalid;
                    }
                }
            }
            67 => {
                if skipped_slice[0] == 65 && skipped_slice[66] == 0xac {
                    let pubkey = &skipped_slice[1..1 + 65];
                    if is_valid_pubkey(pubkey) {
                        return ScriptType::Pubkey(pubkey);
                    } else {
                        return ScriptType::Invalid;
                    }
                }
            }
            _ => {}
        }

        if let Ok(res) = skipped_iter.clone().read_pay_to_multisig() {
            return res;
        }

        {
            let mut skipped_iter = skipped_iter.clone();
            let mut nest_level = 0;
            loop {
                match skipped_iter.read() {
                    Err(ParseError::Eof) => {
                        if nest_level == 0 {
                            break;
                        } else {
                            return ScriptType::Invalid;
                        }
                    }
                    Err(ParseError::Invalid) => return ScriptType::Invalid,
                    Ok(OP_ELSE) | Ok(OP_ENDIF) | Ok(OP_RETURN) | Ok(OP_INVALID) | Ok(OP_VER)
                        if nest_level == 0 =>
                    {
                        return ScriptType::Invalid
                    }
                    Ok(OP_IF) | Ok(OP_NOTIF) => {
                        nest_level += 1;
                    }
                    Ok(OP_ENDIF) => {
                        nest_level -= 1;
                    }
                    Ok(_) => {}
                }
            }
        }

        ScriptType::Unknown(*self)
    }
}

impl<'a> ScriptIter<'a> {
    pub fn read(&mut self) -> ParseResult<Bytecode<'a>> {
        Bytecode::read(&mut self.slice)
    }

    pub fn skip_nops(&mut self) {
        loop {
            let saved = self.slice;
            match self.read() {
                Ok(OP_PUSH(_)) | Ok(OP_DUP) => match self.read() {
                    Ok(OP_DROP) => continue,
                    _ => {}
                },
                Ok(OP_DROP) | Ok(OP_MIN) | Ok(OP_CHECKSIG) | Ok(OP_CHECKMULTISIG) => continue,
                _ => {}
            }
            self.slice = saved;
            return;
        }
    }

    pub fn read_pay_to_multisig(&mut self) -> ParseResult<ScriptType<'a>> {
        let signeed = match self.read() {
            Ok(OP_PUSH(data)) => bytes_to_u32(data)?,
            _ => return Err(ParseError::Invalid),
        };

        let mut out: Vec<&[u8]> = Vec::new();

        loop {
            match self.read() {
                Ok(OP_PUSH(bytes)) => out.push(bytes),
                Ok(OP_CHECKMULTISIG) => break,
                _ => return Err(ParseError::Invalid),
            }
        }

        if !self.slice.is_empty() {
            return Err(ParseError::Invalid);
        }

        let sigtotal = match out.pop() {
            Some(slice) => bytes_to_u32(slice)?,
            None => return Err(ParseError::Invalid),
        };

        if sigtotal as usize == out.len() {
            if signeed as usize <= out.iter().filter(|pubkey| is_valid_pubkey(pubkey)).count() {
                out.shrink_to_fit();
                Ok(ScriptType::Multisig(signeed, out))
            } else {
                Ok(ScriptType::Invalid)
            }
        } else {
            Err(ParseError::Invalid)
        }
    }
}

pub fn bytes_to_i32(slice: &[u8]) -> ParseResult<i32> {
    if slice.is_empty() {
        return Ok(0);
    }

    let neg = slice[0] & 0x80 != 0;

    let mut res: u32 = (slice[0] & 0x7f) as u32;

    for b in slice[1..].iter() {
        if res & 0xff000000 != 0 {
            return Err(ParseError::Invalid);
        }
        res = (res << 8) | (*b as u32);
    }

    if neg {
        Ok(-(res as i32))
    } else {
        Ok(res as i32)
    }
}

pub fn bytes_to_u32(slice: &[u8]) -> ParseResult<u32> {
    let res = bytes_to_i32(slice)?;
    if res >= 0 {
        Ok(res as u32)
    } else {
        Err(ParseError::Invalid)
    }
}

pub fn is_valid_pubkey(pubkey: &[u8]) -> bool {
    if pubkey.is_empty() {
        return false;
    }

    match (pubkey[0], pubkey.len()) {
        (0x02, 33) => true,
        (0x03, 33) => true,
        (0x04, 65) => true,
        _ => false,
    }
}
