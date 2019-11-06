use crossbeam_channel::Receiver;
use rustc_serialize::hex::ToHex;
use std::collections::HashSet;
use std::fs::File;
use std::io::{LineWriter, Write};

use blockchain::address::Address;
use parser::disjoint::DisjointSet;
use parser::TransactionMessage;

pub struct Clusters {
    rx: Receiver<TransactionMessage>,
    clusters: DisjointSet<Address>,
    writer: LineWriter<File>,
}

impl Clusters {
    pub fn new(rx: Receiver<TransactionMessage>) -> Self {
        Self {
            rx: rx.clone(),
            clusters: DisjointSet::<Address>::new(),
            writer: LineWriter::new(
                File::create("nodes2.csv").expect("Unable to create nodes file!"),
            ),
        }
    }

    pub fn run(&mut self) {
        let mut done = false;
        loop {
            if !self.rx.is_empty() {
                match self.rx.recv() {
                    Ok(msg) => match msg {
                        TransactionMessage::OnTransaction(tx_item) => {
                            self.on_transaction(&tx_item);
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
                self.done();
                return;
            }
        }
    }

    fn on_transaction(&mut self, tx_item: &HashSet<Address>) {
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
        self.clusters.finalize();

        let prefix = "kyblsoft.cz".to_string();
        let mut id_vec: Vec<(&usize, &usize, &Address, String)> = Default::default();

        for (address, tag) in &self.clusters.map {
            let hash = md5::compute(format!("{}:{}", prefix, address)).to_hex();
            let digest = &hash[0..16];
            id_vec.push((tag, &self.clusters.parent[*tag], &address, digest.to_string()));
        }
        id_vec.sort_by(|a, b| a.1.partial_cmp(b.1).unwrap());

        let mut pos = 0;
        let mut prev_tag = 0;
        let mut cache: Vec<(usize, Address, String)> = Default::default();
        for (_, tag, address, dig) in id_vec {
            if *tag != prev_tag {
                cache.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap());

                let (_, raddress, rdigest) = cache.pop().unwrap();
                self.writer
                    .write(&format!("{},{},{}\n", pos, raddress, rdigest).as_bytes())
                    .expect("Unable to write nodes file!");

                for (_, raddress, _) in &cache {
                    self.writer
                        .write(&format!("{},{},{}\n", pos, raddress, rdigest).as_bytes())
                        .expect("Unable to write nodes file!");
                }
                pos = pos + 1;
                cache.clear();
            }
            cache.push((tag.clone(), address.clone(), dig.clone()));
            prev_tag = *tag;
        }
    }
}
