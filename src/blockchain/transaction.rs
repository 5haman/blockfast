use bitcoin_bech32::constants::Network;
use bitcoin_bech32::WitnessProgram;
use crypto::digest::Digest;
use crypto::sha2::Sha256;
use fasthash::{xx, RandomState};
use std::collections::hash_map::Entry as HashEntry;
use std::collections::{HashMap, HashSet};
use vec_map::VecMap;

use blockchain::address::Address;
use blockchain::buffer::*;
use blockchain::hash::*;
use blockchain::hash160::Hash160;
use blockchain::script::*;
use parser::Result;

#[derive(PartialEq, Eq, Clone, Copy)]
pub struct Transaction {}

#[derive(PartialEq, Eq, Clone, Copy)]
pub struct TransactionInput<'a> {
    pub prev_hash: &'a Hash,
    pub prev_index: u32,
    pub script: Script<'a>,
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub struct TransactionOutput<'a> {
    pub value: u64,
    pub script: Script<'a>,
}

impl Transaction {
    pub fn read(
        slice: &mut &[u8],
        timestamp: u32,
        output_items: &mut HashMap<Hash, VecMap<Vec<Address>>, RandomState<xx::Hash64>>,
        inputs: &mut HashSet<Address>,
        outputs: &mut HashSet<Address>,
    ) -> Result<bool> {
        let mut tx_hash = [0u8; 32];
        let mut sha256_hasher1 = Sha256::new();
        let mut sha256_hasher2 = sha256_hasher1;

        sha256_hasher1.input(&slice[..4]);
        let _ = read_u32(slice)?;

        let marker = slice[0];
        let txins_count: u64;
        let mut slice_inputs_and_outputs = *slice;
        if marker == 0x00 {
            // Consume marker
            *slice = &slice[1..];
            let flag = read_u8(slice)?;
            slice_inputs_and_outputs = *slice;
            if flag == 0x01 {
                txins_count = read_var_int(slice)?;
            } else {
                return Ok(false);
            }
        } else {
            txins_count = read_var_int(slice)?;
        }

        // Read the inputs
        for _ in 0..txins_count {
            let txin = TransactionInput::read(slice, timestamp)?;
            let mut output_item = None;
            if let HashEntry::Occupied(mut occupied) = output_items.entry(*txin.prev_hash) {
                output_item = occupied.get_mut().remove(txin.prev_index as usize);
                if occupied.get().len() == 0 {
                    occupied.remove();
                }
            }

            if txin.prev_hash == &ZERO_HASH {
                continue;
            }
            match output_item {
                Some(address) => {
                    for n in 0..address.len() {
                        inputs.insert(address[n].clone());
                    }
                }
                None => {}
            }
        }

        // Read the outputs
        let txouts_count = read_var_int(slice)?;

        let mut cur_output_items = VecMap::with_capacity(txouts_count as usize);
        for n in 0..txouts_count {
            let txout = TransactionOutput::read(slice, timestamp)?;
            let output_item = match txout.script.to_scripttype() {
                ScriptType::PubkeyHash(pkh) => {
                    Some(vec![Address::from_hash160(Hash160::from_slice(pkh), 0x00)])
                }
                ScriptType::ScriptHash(pkh) => {
                    Some(vec![Address::from_hash160(Hash160::from_slice(pkh), 0x05)])
                }
                ScriptType::Pubkey(pk) => {
                    Some(vec![Address::from_hash160(&Hash160::from_data(pk), 0x00)])
                }
                ScriptType::Multisig(_, pks) => Some(
                    pks.iter()
                        .map(|pk| Address::from_pubkey(pk, 0x05))
                        .collect(),
                ),
                ScriptType::WitnessScriptHash(w) => Some(vec![Address {
                    addr: WitnessProgram::from_scriptpubkey(w, Network::Bitcoin)
                        .unwrap()
                        .to_address()
                        .as_bytes()
                        .to_vec(),
                }]),
                ScriptType::WitnessPubkeyHash(w) => Some(vec![Address {
                    addr: WitnessProgram::from_scriptpubkey(w, Network::Bitcoin)
                        .unwrap()
                        .to_address()
                        .as_bytes()
                        .to_vec(),
                }]),
                _ => None,
            };

            if let Some(output_item) = output_item {
                let ins = output_item.clone();
                cur_output_items.insert(n as usize, output_item);
                for addr in ins {
                    outputs.insert(addr);
                }
            };
        }

        // Hash the transaction data before the witnesses
        let slice_len = slice_inputs_and_outputs.len() - slice.len();
        sha256_hasher1.input(read_slice(&mut slice_inputs_and_outputs, slice_len)?);

        // Read the witnesses
        if marker == 0x00 {
            for _ in 0..txins_count {
                let item_count = read_var_int(slice)?;
                for _ in 0..item_count {
                    let witness_len = read_var_int(slice)? as usize;
                    let _witness = read_slice(slice, witness_len)?;
                }
            }
        }

        sha256_hasher1.input(&slice[..4]);
        let _ = read_u32(slice)?;
        sha256_hasher1.result(&mut tx_hash);
        sha256_hasher2.input(&tx_hash);
        sha256_hasher2.result(&mut tx_hash);

        if cur_output_items.len() > 0 {
            let len = cur_output_items.len();
            cur_output_items.reserve_len_exact(len);
            output_items.insert(*Hash::from_slice(&tx_hash), cur_output_items);
        }

        Ok(true)
    }
}

impl<'a> TransactionInput<'a> {
    pub fn read(slice: &mut &'a [u8], timestamp: u32) -> Result<TransactionInput<'a>> {
        // Save the initial position
        let mut init_slice = *slice;

        // Read the prev_hash
        let prev_hash = Hash::from_slice(read_array!(slice, 32)?);

        // Read the prev_index
        let prev_index = read_u32(slice)?;

        // Read the script
        let nbytes = read_var_int(slice)? as usize;
        let script = read_slice(slice, nbytes)?;

        // Read the sequence_no
        let _ = read_u32(slice)?;
        let len = init_slice.len() - slice.len();
        let _ = read_slice(&mut init_slice, len)?;

        Ok(TransactionInput {
            prev_hash,
            prev_index,
            script: Script::new(script, timestamp),
        })
    }
}

impl<'a> TransactionOutput<'a> {
    pub fn read(slice: &mut &'a [u8], timestamp: u32) -> Result<TransactionOutput<'a>> {
        // Save the initial position
        let mut init_slice = *slice;

        // Read the value
        let value = read_u64(slice)?;

        // Read the script
        let nbytes = read_var_int(slice)? as usize;
        let script = read_slice(slice, nbytes)?;

        // Return the transaction output
        let len = init_slice.len() - slice.len();
        let _ = read_slice(&mut init_slice, len)?;
        Ok(TransactionOutput {
            value,
            script: Script::new(script, timestamp),
        })
    }
}
