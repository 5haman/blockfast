use crossbeam_channel::Receiver;
use rustc_serialize::hex::ToHex;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{LineWriter, Write};

use blockchain::address::Address;
//use disjoint_sets::UnionFind;
use disjoint_sets::UnionFind;
use parser::transactions::TransactionMessage;
use parser::Config;

pub struct Clusters {
    rx: Receiver<TransactionMessage>,
    writer: LineWriter<File>,
    id: u32,
}

impl Clusters {
    pub fn new(rx: Receiver<TransactionMessage>, config: &Config) -> Self {
        let output = &config.output;
        let writer = LineWriter::new(File::create(output).expect("Unable to create output file!"));
        Self {
            writer: writer,
            rx: rx.clone(),
            id: 0,
        }
    }

    pub fn run(&mut self, clusters: &mut UnionFind, addresses: &mut HashMap<Address, u32>) {
        let mut done = false;
        loop {
            if !self.rx.is_empty() {
                match self.rx.recv() {
                    Ok(msg) => match msg {
                        TransactionMessage::OnTransaction(tx_item) => {
                            self.on_transaction(&tx_item, clusters, addresses);
                        }
                        TransactionMessage::OnComplete(_) => {
                            done = true;
                        }
                        TransactionMessage::OnError(_) => {
                            warn!("Error processing transaction");
                        }
                    },
                    Err(_) => {
                        warn!("Error processing transaction");
                    }
                }
            } else if done {
                self.done(clusters, addresses);
                return;
            }
        }
    }

    fn on_transaction(
        &mut self,
        tx_item: &Vec<HashSet<Address>>,
        clusters: &mut UnionFind,
        addresses: &mut HashMap<Address, u32>, //clusters: &mut UnionFind,
    ) {
        let inputs = tx_item.first().unwrap();

        if inputs.len() > 0 {
            let mut tx_inputs_iter = inputs.iter();
            let last_address = tx_inputs_iter.next().unwrap();

            let mut last_id = match addresses.insert(last_address.to_owned(), self.id) {
                Some(id) => (id),
                None => {
                    let id = self.id;
                    self.id = self.id + 1;
                    id
                }
            };

            for address in tx_inputs_iter {
                let id = match addresses.insert(address.to_owned(), self.id) {
                    Some(id) => (id),
                    None => {
                        let id = self.id;
                        self.id = self.id + 1;
                        id
                    }
                };

                if id != last_id {
                    clusters.union(last_id as usize, id as usize);
                }
                last_id = id;
            }
        }
    }

    fn done(&mut self, clusters: &mut UnionFind, addresses: &mut HashMap<Address, u32>) {
        clusters.force();

        let prefix = "kyblsoft.cz".to_string();
        let mut id_vec: Vec<(u32, u32, &Address)> = Default::default();

        for (address, tag) in addresses {
            id_vec.push((*tag as u32, clusters.find(*tag as usize) as u32, &address));
        }
        id_vec.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        let mut pos = 0;
        let mut prev_tag = 1;
        let mut cache: Vec<(u32, &Address, String)> = Default::default();
        let mut newparent: Vec<u32> = Vec::new();
        for (_, tag, address) in id_vec {
            if tag != prev_tag {
                cache.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap());

                let (_, raddress, rdigest) = cache.pop().unwrap();

                self.writer
                    .write(&format!("{},{},{}\n", pos, raddress, rdigest).as_bytes())
                    .expect("Unable to write to output file!");
                newparent.push(pos as u32);

                for (_, raddress, _) in &cache {
                    self.writer
                        .write(&format!("{},{},{}\n", pos, raddress, rdigest).as_bytes())
                        .expect("Unable to write to output file!");
                    newparent.push(pos as u32);
                }
                pos = pos + 1;
                cache.clear();
            }
            let hash = md5::compute(format!("{}:{}", prefix, address)).to_hex();
            let digest = &hash[0..16];
            cache.push((tag, &address, digest.to_string()));
            prev_tag = tag;
        }
    }
}
