use base58::ToBase58;
//use bitcoin_bech32::constants::Network;
//use bitcoin_bech32::WitnessProgram;
use std::{fmt, hash};

use blockchain::hash::Hash;
use blockchain::hash160::Hash160;

#[derive(Copy, Clone, Default, Ord, PartialOrd)]
pub struct Address {
    pub hash: Hash160,
    pub ver: u8,
}

/*
#[derive(Copy, Clone)]
pub enum Address {
    Base58(AddressBase58),
    //WitnessPubkey([u8; 22]),
    //WitnessScript([u8; 34]),
}
*/
impl PartialEq for Address {
    fn eq(&self, other: &Address) -> bool {
        self == other
    }
}

impl Eq for Address {}

impl hash::Hash for Address {
    fn hash<H>(&self, hasher: &mut H)
    where
        H: hash::Hasher,
    {
        //match self {
        //Address::Base58(address) => {
        hasher.write(&self.hash[..]);
        //}
        /*
        Address::WitnessScript(s) => {
            hasher.write(s);
        }
        Address::WitnessPubkey(pk) => {
            hasher.write(pk);
        }
        */
        //}
    }
}

impl fmt::Display for Address {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        //match self {
        //Address::Base58(address) => {
        let v: Vec<u8> = [&[self.ver], &self.hash[..]].concat();
        let h = Hash::from_data(&v);
        let address = [&v, &h[0..4]].concat().to_base58();
        address.fmt(formatter)
        //}
        /*
        Address::WitnessScript(s) => {
            let w = WitnessProgram::from_scriptpubkey(s, Network::Bitcoin)
                .unwrap()
                .to_address();
            w.fmt(formatter)
        }
        Address::WitnessPubkey(pk) => {
            let w = WitnessProgram::from_scriptpubkey(pk, Network::Bitcoin)
                .unwrap()
                .to_address();
            w.fmt(formatter)
        }
        */
        //}
    }
}

impl Address {
    pub fn from_pubkey(pubkey: &[u8], ver: u8) -> Address {
        let hash = Hash160::from_data(pubkey);
        Address { hash, ver }
        //return Address::Base58(AddressBase58 { hash, ver });
    }

    pub fn from_hash160(hash160: &Hash160, ver: u8) -> Address {
        let hash = *hash160;
        Address { hash, ver }
        //return Address::Base58(AddressBase58 { hash, ver });
    }

    /*
    pub fn from_witness_script(script: &[u8; 34]) -> Address {

        unsafe {
            let out: &[u8; 34] = mem::transmute(script);
            return Address::WitnessScript(*out);
        }

    }

    pub fn from_witness_pubkey(pubkey: &[u8; 22]) -> Address {

        unsafe {
            let out: &[u8; 22] = mem::transmute(pubkey);
            return Address::WitnessPubkey(*out);
        }

    }
    */
}
