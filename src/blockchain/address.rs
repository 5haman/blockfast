use base58::ToBase58;
use std::fmt;

use blockchain::hash::Hash;
use blockchain::hash160::Hash160;

#[derive(PartialEq, Eq, Copy, Clone, Hash, Default, Ord, PartialOrd)]
pub struct Address {
    pub hash: Hash160,
    pub version: u8,
}

impl fmt::Display for Address {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let v: Vec<u8> = [&[self.version], &self.hash[..]].concat();
        let h = Hash::from_data(&v);
        let address = [&v, &h[0..4]].concat().to_base58();
        address.fmt(formatter)
    }
}

impl Address {
    pub fn from_pubkey(pubkey: &[u8], version: u8) -> Address {
        let hash = Hash160::from_data(pubkey);
        return Address { hash, version };
    }

    pub fn from_hash160(hash160: &Hash160, version: u8) -> Address {
        let hash = *hash160;
        return Address { hash, version };
    }
}
