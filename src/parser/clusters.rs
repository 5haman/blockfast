use crossbeam_channel::Receiver;
use rustc_serialize::hex::ToHex;
use std::collections::HashSet;
use std::fs::File;
use std::io::{LineWriter, Write};

use blockchain::address::Address;
use parser::disjoint::DisjointSet;
use parser::transactions::TransactionMessage;
use parser::Config;

pub struct Clusters {
    rx: Receiver<TransactionMessage>,
    writer: LineWriter<File>,
}

impl Clusters {
    pub fn new(rx: Receiver<TransactionMessage>, config: &Config) -> Self {
        let output = &config.output;
        let writer = LineWriter::new(File::create(output).expect("Unable to create output file!"));
        Self {
            writer: writer,
            rx: rx.clone(),
        }
    }

    pub fn run(&mut self, clusters: &mut DisjointSet<Address>) {
        let mut done = false;
        loop {
            if !self.rx.is_empty() {
                match self.rx.recv() {
                    Ok(msg) => match msg {
                        TransactionMessage::OnTransaction(tx_item) => {
                            self.on_transaction(&tx_item, clusters);
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
                self.done(clusters);
                return;
            }
        }
    }

    fn on_transaction(
        &mut self,
        tx_item: &Vec<HashSet<Address>>,
        clusters: &mut DisjointSet<Address>,
    ) {
        let inputs = tx_item.first().unwrap();
        if inputs.len() > 0 {
            let mut tx_inputs_iter = inputs.iter();
            let mut last_address = tx_inputs_iter.next().unwrap();
            clusters.make_set(last_address.to_owned());
            for address in tx_inputs_iter {
                clusters.make_set(address.to_owned());
                let _ = clusters.union(last_address, address);
                last_address = &address;
            }
        }
    }

    fn done(&mut self, clusters: &mut DisjointSet<Address>) {
        clusters.finalize();

        let prefix = "kyblsoft.cz".to_string();
        let mut id_vec: Vec<(u32, u32, &Address)> = Default::default();

        for (address, tag) in &clusters.map {
            id_vec.push((*tag as u32, clusters.parent[*tag] as u32, &address));
        }
        id_vec.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        let mut pos = 0;
        let mut prev_tag = 0;
        let mut cache: Vec<(u32, &Address, [u8; 16])> = Default::default();
        let mut newparent: Vec<u32> = Vec::new();
        for (_, tag, address) in id_vec {
            if tag != prev_tag {
                cache.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap());

                let (_, raddress, rdigest) = cache.pop().unwrap();
                self.writer
                    .write(&format!("{},{},{}\n", pos, raddress, rdigest.to_hex().to_string()).as_bytes())
                    .expect("Unable to write to output file!");
                newparent.push(pos as u32);

                for (_, raddress, _) in &cache {
                    self.writer
                        .write(&format!("{},{},{}\n", pos, raddress, rdigest.to_hex().to_string()).as_bytes())
                        .expect("Unable to write to output file!");
                    newparent.push(pos as u32);
                }
                pos = pos + 1;
                cache.clear();
            }
            let hash = md5::compute(format!("{}:{}", prefix, address));
            cache.push((tag, &address, *hash));
            prev_tag = tag;
        }
    }
}
