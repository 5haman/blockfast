use base58::ToBase58;
use std::collections::VecDeque;
use std::{fmt, hash};

use parser::timestamp_to_date;
use blockchain::hash::Hash;
use blockchain::hash160::Hash160;

#[derive(Clone, Debug)]
pub struct Taint {
    pub label: u8,
    pub amount: u64,
}

#[derive(Clone)]
pub struct Address {
    pub addr: Vec<u8>,
    pub balance: u64,
    pub taints: Option<VecDeque<Taint>>,
    pub firstseen: u32
}

impl PartialEq for Address {
    fn eq(&self, other: &Address) -> bool {
        self.addr == other.addr
    }
}

impl Eq for Address {}

impl hash::Hash for Address {
    fn hash<H>(&self, hasher: &mut H)
    where
        H: hash::Hasher,
    {
        hasher.write(&self.addr[..]);
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{},{},{}", self.firstseen, self.addr.to_base58(), self.balance)
    }
}

impl fmt::Debug for Address {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut d = f.debug_struct("Address");
        d.field("base58", &self.addr.to_base58());
        d.field("balance", &self.balance);
        d.field("taints", &self.taints);
        d.field("firstseen", &timestamp_to_date(self.firstseen));
        d.finish()
    }
}

impl Address {
    pub fn new(
        pubkey: &[u8],
        taints: Option<VecDeque<Taint>>,
        balance: u64,
        firstseen: u32
    ) -> Address {
        return Address {
            addr: pubkey.to_vec(),
            balance: balance,
            taints: taints,
            firstseen: firstseen
        }
    }

    pub fn from_pubkey(
        pubkey: &[u8],
        version: u8,
        taints: Option<VecDeque<Taint>>,
        balance: u64,
        firstseen: u32
    ) -> Address {
        let hash160 = Hash160::from_data(pubkey);
        return Address::from_hash160(
            &hash160,
            version,
            taints,
            balance,
            firstseen
        );
    }

    pub fn from_hash160(
        hash160: &Hash160,
        version: u8,
        taints: Option<VecDeque<Taint>>,
        balance: u64,
        firstseen: u32
    ) -> Address {
        let v: Vec<u8> = [&[version], hash160.as_slice()].concat();
        let h = Hash::from_data(&v);
        Address {
            addr: [&v, &h[0..4]].concat().to_vec(),
            taints: taints,
            balance: balance,
            firstseen: firstseen
        }
    }
}
