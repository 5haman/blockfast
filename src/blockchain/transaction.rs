use bitcoin_bech32::constants::Network;
use bitcoin_bech32::WitnessProgram;
use crypto::digest::Digest;
use crypto::sha2::Sha256;
use fasthash::{xx, RandomState};
use std::collections::hash_map::Entry as HashEntry;
use std::collections::{HashMap, VecDeque};
use std::fmt;
use vec_map::VecMap;

use blockchain::address::{Address, Taint};
use blockchain::buffer::*;
use blockchain::hash::*;
use blockchain::hash160::Hash160;
use blockchain::script::*;
use parser::{ParseError, ParseResult};

#[derive(PartialEq, Eq, Clone)]
pub struct Transaction {
    pub version: u32,
    pub txid: Hash,
    pub inputs_count: u64,
    pub outputs_count: u64,
    pub lock_time: u32,
    pub inputs: HashMap<Address, u64>,
    pub outputs: HashMap<Address, u64>,
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub struct TransactionInput<'a> {
    pub prev_hash: &'a Hash,
    pub prev_index: u32,
    pub script: Script<'a>,
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub struct TransactionOutput<'a> {
    pub amount: u64,
    pub script: Script<'a>,
}

impl fmt::Debug for Transaction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut d = f.debug_struct("Transaction");
        d.field("txid", &self.txid);
        d.field("version", &self.version);
        d.field("lock_time", &self.lock_time);
        d.field("inputs_count", &self.inputs_count);
        d.field("outputs_count", &self.outputs_count);
        if self.inputs_count == 1 && self.inputs.len() == 0 {
            d.field("inputs", &"coinbase".to_string());
        } else {
            d.field("inputs", &self.inputs);
        }
        d.field("outputs", &self.outputs);
        d.finish()
    }
}

impl Transaction {
    pub fn read(
        slice: &mut &[u8],
        timestamp: u32,
        output_items: &mut HashMap<Hash, VecMap<Vec<(Address, u64)>>, RandomState<xx::Hash64>>,
        start_txs: &mut HashMap<Hash, VecDeque<Taint>>,
    ) -> ParseResult<Transaction> {
        let mut tx_hash = [0u8; 32];
        let mut sha256_hasher1 = Sha256::new();
        let mut sha256_hasher2 = sha256_hasher1;
        let mut inputs = HashMap::<Address, u64>::new();
        let mut outputs = HashMap::<Address, u64>::new();

        sha256_hasher1.input(&slice[..4]);
        let version = read_u32(slice)?;

        let marker = slice[0];
        let inputs_count: u64;
        let mut slice_inputs_and_outputs = *slice;
        if marker == 0x00 {
            // Consume marker
            *slice = &slice[1..];
            let flag = read_u8(slice)?;
            slice_inputs_and_outputs = *slice;
            if flag == 0x01 {
                inputs_count = read_var_int(slice)?;
            } else {
                return Err(ParseError::Invalid);
            }
        } else {
            inputs_count = read_var_int(slice)?;
        }

        // Read the inputs
        let mut cur_taints: VecDeque<Taint> = Default::default();
        for _ in 0..inputs_count {
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
                Some(output) => {
                    for (mut address, amount) in output {
                        let a = address.clone();
                        match address.taints {
                            Some(mut taint) => {
                                address.taints = is_taint(&mut taint, amount);
                                for t in address.taints.unwrap() {
                                    cur_taints.push_back(t);
                                }
                            }
                            None => {}
                        }
                        inputs.insert(a.to_owned(), amount);
                    }
                }
                None => {}
            }
        }

        // Read the outputs
        let outputs_count = read_var_int(slice)?;
        let mut raw_outputs: Vec<TransactionOutput> = Vec::new();
        for _ in 0..outputs_count {
            let txout = TransactionOutput::read(slice, timestamp)?;
            raw_outputs.push(txout);
        }

        // Hash the transaction data before the witnesses
        let slice_len = slice_inputs_and_outputs.len() - slice.len();
        sha256_hasher1.input(read_slice(&mut slice_inputs_and_outputs, slice_len)?);

        // Read the witnesses
        if marker == 0x00 {
            for _ in 0..inputs_count {
                let item_count = read_var_int(slice)?;
                for _ in 0..item_count {
                    let witness_len = read_var_int(slice)? as usize;
                    let _witness = read_slice(slice, witness_len)?;
                }
            }
        }

        sha256_hasher1.input(&slice[..4]);
        let lock_time = read_u32(slice)?;
        sha256_hasher1.result(&mut tx_hash);
        sha256_hasher2.input(&tx_hash);
        sha256_hasher2.result(&mut tx_hash);
        let txid = *Hash::from_slice(&tx_hash);

        let mut cur_outputs = VecMap::with_capacity(outputs_count as usize);
        let mut remove = false;
        for n in 0..raw_outputs.len() {
            let txout = raw_outputs[n];
            let output_item = match txout.script.to_scripttype() {
                ScriptType::PubkeyHash(pkh) => Some(vec![Address::from_hash160(
                    Hash160::from_slice(pkh),
                    0x00,
                    None,
                )]),
                ScriptType::ScriptHash(pkh) => Some(vec![Address::from_hash160(
                    Hash160::from_slice(pkh),
                    0x05,
                    None,
                )]),
                ScriptType::Pubkey(pk) => Some(vec![Address::from_hash160(
                    &Hash160::from_data(pk),
                    0x00,
                    None,
                )]),
                ScriptType::Multisig(_, pks) => Some(
                    pks.iter()
                        .map(|pk| Address::from_pubkey(pk, 0x05, None))
                        .collect(),
                ),
                ScriptType::WitnessScriptHash(w) => Some(vec![Address {
                    addr: WitnessProgram::from_scriptpubkey(w, Network::Bitcoin)
                        .unwrap()
                        .to_address()
                        .as_bytes()
                        .to_vec(),
                    taints: None,
                }]),
                ScriptType::WitnessPubkeyHash(w) => Some(vec![Address {
                    addr: WitnessProgram::from_scriptpubkey(w, Network::Bitcoin)
                        .unwrap()
                        .to_address()
                        .as_bytes()
                        .to_vec(),
                    taints: None,
                }]),
                _ => None,
            };

            if let Some(output_item) = output_item {
                let mut cur_output = Vec::new();

                for m in 0..output_item.len() {
                    let mut address = output_item[m].to_owned();
                    if start_txs.len() > 0 && start_txs.contains_key(&txid) {
                        remove = true;
                        address.taints = is_taint(start_txs.get_mut(&txid).unwrap(), txout.amount);
                    }
                    if cur_taints.len() > 0 {
                        let taints = is_taint(&mut cur_taints, txout.amount);
                        for t in taints.clone().unwrap() {
                            if t.label != 0 {
                                address.taints = taints;
                                break;
                            }
                        }
                    }
                    cur_output.insert(m as usize, (address.to_owned(), txout.amount));
                    outputs.insert(address.to_owned(), txout.amount);
                }
                cur_outputs.insert(n as usize, cur_output);
            };
        }
        if remove {
            start_txs.remove(&txid);
        }

        if cur_outputs.len() > 0 {
            let len = cur_outputs.len();
            cur_outputs.reserve_len_exact(len);
            output_items.insert(txid, cur_outputs);
        }

        let tx = Transaction {
            version,
            txid: txid,
            inputs_count,
            outputs_count,
            lock_time,
            inputs,
            outputs,
        };

        Ok(tx)
    }
}

fn is_taint(taints: &mut VecDeque<Taint>, amount: u64) -> Option<VecDeque<Taint>> {
    let mut remaining = amount;
    let mut new_taints = VecDeque::new();

    while remaining > 0 {
        if taints.is_empty() {
            new_taints.push_back(Taint {
                label: 0,
                amount: remaining,
            });
            remaining = 0;
        } else {
            let mut taint = taints.pop_front().unwrap();
            if remaining >= taint.amount {
                remaining -= taint.amount;
                new_taints.push_back(taint);
            } else {
                taint.amount -= remaining;
                new_taints.push_back(Taint {
                    label: taint.label,
                    amount: remaining,
                });
                taints.push_front(taint);
                remaining = 0;
            }
        }
    }

    if remaining > 0 {
        new_taints.push_back(Taint {
            label: 0,
            amount: remaining,
        });
    }

    if new_taints.len() == 0 {
        return None;
    } else {
        return Some(new_taints);
    }
}

impl<'a> TransactionInput<'a> {
    pub fn read(slice: &mut &'a [u8], timestamp: u32) -> ParseResult<TransactionInput<'a>> {
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
    pub fn read(slice: &mut &'a [u8], timestamp: u32) -> ParseResult<TransactionOutput<'a>> {
        // Save the initial position
        let mut init_slice = *slice;

        // Read the amount
        let amount = read_u64(slice)?;

        // Read the script
        let nbytes = read_var_int(slice)? as usize;
        let script = read_slice(slice, nbytes)?;

        // Return the transaction output
        let len = init_slice.len() - slice.len();
        let _ = read_slice(&mut init_slice, len)?;
        Ok(TransactionOutput {
            amount,
            script: Script::new(script, timestamp),
        })
    }
}
