use std::collections::HashSet;
use std::fs::File;
use std::io::{LineWriter, Write};

use blockchain::address::Address;
use blockchain::hash160::Hash160;
use blockchain::script::HighLevel;
use blockchain::transaction::{Transaction, TransactionInput, TransactionOutput};
use visitors::disjoint::DisjointSet;
use visitors::Visitor;

pub struct Clusterizer {
    clusters: DisjointSet<Address>,
    writer: LineWriter<File>,
}

impl<'a> Visitor<'a> for Clusterizer {
    type BlockItem = ();
    type TransactionItem = HashSet<Address>;
    type OutputItem = Vec<Address>;
    type DoneItem = ();

    fn new() -> Self {
        Self {
            clusters: DisjointSet::<Address>::new(),
            writer: LineWriter::new(
                File::create("nodes.csv").expect("Unable to create nodes file!"),
            ),
        }
    }

    //fn visit_block_begin(&mut self, _block: Block<'a>, _height: u64) {}

    fn visit_transaction_begin(
        &mut self,
        //_block_item: &mut Self::BlockItem,
    ) -> Self::TransactionItem {
        HashSet::new()
    }

    fn visit_transaction_input(
        &mut self,
        _txin: TransactionInput<'a>,
        //_block_item: &mut Self::BlockItem,
        tx_item: &mut Self::TransactionItem,
        output_item: Option<Self::OutputItem>,
    ) {
        let mut tx_output_iter = output_item.into_iter();

        if tx_output_iter.len() > 0 {
            for a in tx_output_iter.next().unwrap() {
                match Some(a) {
                    Some(address) => {
                        tx_item.insert(address);
                    }
                    None => {}
                }
            }
        }
    }

    fn visit_transaction_output(
        &mut self,
        txout: TransactionOutput<'a>,
        //_block_item: &mut (),
        _transaction_item: &mut (Self::TransactionItem),
    ) -> Option<Self::OutputItem> {
        match txout.script.to_highlevel() {
            HighLevel::PayToPubkeyHash(pkh) => {
                Some(vec![Address::from_hash160(Hash160::from_slice(pkh), 0x00)])
            }
            HighLevel::PayToScriptHash(pkh) => {
                Some(vec![Address::from_hash160(Hash160::from_slice(pkh), 0x05)])
            }
            HighLevel::PayToPubkey(pk) => {
                Some(vec![Address::from_hash160(&Hash160::from_data(pk), 0x00)])
            }
            HighLevel::PayToMultisig(_, pks) => Some(
                pks.iter()
                    .map(|pk| Address::from_pubkey(pk, 0x05))
                    .collect(),
            ),
            HighLevel::PayToWitnessPubkeyHash(w) | HighLevel::PayToWitnessScriptHash(w) => {
                Some(vec![Address(w.to_address())])
            }
            _ => None,
        }
    }

    fn visit_transaction_end(
        &mut self,
        _tx: Transaction<'a>,
        //_block_item: &mut Self::BlockItem,
        tx_item: Self::TransactionItem,
    ) {
        if tx_item.len() > 0 {
            let mut tx_inputs_iter = tx_item.iter();
            let mut last_address = tx_inputs_iter.next().unwrap();
            self.clusters.make_set(last_address.to_owned());
            for address in tx_inputs_iter {
                self.clusters.make_set(address.to_owned());
                let _ = self.clusters.union(last_address, address);
                last_address = &address;
            }
        }
    }

    fn done(&mut self) {
        //self.clusters.finalize();

        for (address, tag) in &self.clusters.map {
            self.writer
                .write(&format!("{} {}\n", self.clusters.parent[*tag], address).as_bytes())
                .expect("Unable to write nodes file!");
            //println!("{} {}", self.clusters.parent[*tag], address);
        }
    }
}
