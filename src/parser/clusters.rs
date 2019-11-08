use crossbeam_channel::Receiver;
use crypto::digest::Digest;
use crypto::md5::Md5;
use hash_hasher::HashBuildHasher;
use rustc_serialize::hex::ToHex;
use std::collections::HashSet;
use std::fs::File;
use std::io::{LineWriter, Write};

use blockchain::address::Address;
use parser::transactions::TransactionMessage;
use parser::union::UnionFind;
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

    pub fn run(&mut self, clusters: &mut UnionFind<Address, HashBuildHasher>) {
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
        clusters: &mut UnionFind<Address, HashBuildHasher>,
    ) {
        let inputs = tx_item.first().unwrap();

        if inputs.len() > 1 {
            let mut tx_inputs_iter = inputs.iter();
            let mut last_address = tx_inputs_iter.next().unwrap();

            if !clusters.contains(&last_address.to_owned()) {
                clusters.make_set(last_address.to_owned());
            }

            for address in tx_inputs_iter {
                if !clusters.contains(&address.to_owned()) {
                    clusters.make_set(address.to_owned());
                }
                clusters.union(last_address.to_owned(), address.to_owned());
                last_address = address;
            }
        }
    }

    fn done(&mut self, clusters: &mut UnionFind<Address, HashBuildHasher>) {
        info!("Done");
        info!("Found {} addresses", clusters.len());

        let prefix = "kyblsoft.cz".to_string();
        let mut cache: Vec<(Address, [u8; 32])> = Default::default();
        let mut pos = 0;
        let mut count = 0;

        for set in clusters.into_iter() {
            if count % 1000000 == 0 && count != 0 {
                info!("Processed {} addresses, {} clusters", count, pos);
            }

            for address in set.into_iter() {
                let mut hasher = Md5::new();
                let mut hash = [0u8; 32];
                hasher.input(format!("{}:{}", prefix, address).as_bytes());
                hasher.result(&mut hash);
                cache.push((address.clone(), hash));
                count = count + 1;
            }
            cache.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

            let (address, hash) = cache.pop().unwrap();
            let digest = array_ref!(hash, 0, 8);
            self.writer
                .write(&format!("{},{},{}\n", pos, address, digest.to_hex()).as_bytes())
                .expect("Unable to write to output file!");

            for (address, _) in &cache {
                self.writer
                    .write(&format!("{},{},{}\n", pos, address, digest.to_hex()).as_bytes())
                    .expect("Unable to write to output file!");
            }
            pos = pos + 1;
            cache.clear();
        }
        info!("Done");
        info!("Found {} clusters", pos);
    }
}
