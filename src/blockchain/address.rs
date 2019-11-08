use base58::ToBase58;
use blockchain::hash::Hash;
use blockchain::hash160::Hash160;
use std::fmt;

#[derive(PartialEq, Eq, Debug, Clone, Default, Hash, Ord, PartialOrd)]
pub struct Address {
    pub addr: Vec<u8>,
}

impl fmt::Display for Address {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        self.addr.to_base58().fmt(formatter)
    }
}

impl Address {
    pub fn from_pubkey(pubkey: &[u8], version: u8) -> Address {
        let hash160 = Hash160::from_data(pubkey);
        return Address::from_hash160(&hash160, version);
    }

    pub fn from_hash160(hash160: &Hash160, version: u8) -> Address {
        let v: Vec<u8> = [&[version], hash160.as_slice()].concat();
        let h = Hash::from_data(&v);
        Address {
            addr: [&v, &h[0..4]].concat().to_vec(),
        }
    }
}
