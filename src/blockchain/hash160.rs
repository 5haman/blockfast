use crypto::digest::Digest;
use crypto::ripemd160::Ripemd160;
use crypto::sha2::Sha256;
use std::ops::{Deref, DerefMut};
use std::{hash, mem};

#[derive(PartialEq, Eq, Copy, Clone, Debug, Default, Ord, PartialOrd)]
pub struct Hash160([u8; 20]);

impl hash::Hash for Hash160 {
    fn hash<H>(&self, hasher: &mut H)
    where
        H: hash::Hasher,
    {
        hasher.write(&self.0);
    }
}

impl Hash160 {
    pub fn from_data(data: &[u8]) -> Hash160 {
        let mut intermediate = [0u8; 32];
        let mut out = [0u8; 20];
        let mut sha256_hasher = Sha256::new();
        let mut ripemd_hasher = Ripemd160::new();

        sha256_hasher.input(data);
        sha256_hasher.result(&mut intermediate);

        ripemd_hasher.input(&intermediate);
        ripemd_hasher.result(&mut out);

        Hash160(out)
    }

    pub fn from_slice(slice: &[u8; 20]) -> &Hash160 {
        unsafe { mem::transmute(slice) }
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.0
    }
}

impl Deref for Hash160 {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        self.as_slice()
    }
}

impl DerefMut for Hash160 {
    fn deref_mut(&mut self) -> &mut [u8] {
        self.as_mut_slice()
    }
}
