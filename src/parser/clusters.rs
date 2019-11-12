use crossbeam_channel::Receiver;
use crypto::digest::Digest;
use crypto::md5::Md5;
use fasthash::{xx, RandomState};
use rustc_serialize::hex::ToHex;
use std::fs::File;
use std::io::{LineWriter, Write};

use blockchain::address::Address;
use blockchain::transaction::Transaction;
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

    pub fn run(&mut self, clusters: &mut UnionFind<Address, RandomState<xx::Hash64>>) {
        let mut done = false;
        loop {
            if !self.rx.is_empty() {
                match self.rx.recv() {
                    Ok(msg) => match msg {
                        TransactionMessage::OnTransaction(mut transaction) => {
                            self.on_transaction(&mut transaction, clusters);
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
        transaction: &mut Transaction,
        clusters: &mut UnionFind<Address, RandomState<xx::Hash64>>,
    ) {
        // Inputs
        if transaction.inputs.len() > 0 {
            let mut tx_inputs = transaction.inputs.iter();
            let (mut last_address, _) = tx_inputs.next().unwrap();
            let mut is_cluster = false;

            if transaction.outputs.len() > 0 {
                let (output1, amount) = transaction.outputs.iter().next().unwrap();
                if transaction.outputs.len() == 2 && transaction.inputs.len() != 2 {
                    let (output2, _) = &transaction.outputs.iter().next().unwrap();

                    if !clusters.contains(&output1) && clusters.contains(&output2) {
                        let amount = *amount / 100_000_000;
                        let vals = format!("{}", amount);
                        let vec: Vec<&str> = vals.split(".").collect();
                        let len = vec.last().unwrap().len();

                        if len > 4 {
                            is_cluster = true;
                            clusters.make_set(output1.to_owned());
                        }
                    }

                    for (address, _) in transaction.inputs.iter() {
                        if *address == *output1 || address == *output2 {
                            is_cluster = false;
                            break;
                        }
                    }
                }

                if is_cluster || transaction.outputs.len() == 1 {
                    if !clusters.contains(&last_address) {
                        clusters.make_set(last_address.to_owned());
                    } else {
                        clusters.insert(last_address.to_owned());
                    }
                    if is_cluster {
                        clusters.union(last_address.to_owned(), output1.clone());
                    }
                }
            }

            for (address, _) in tx_inputs {
                if transaction.outputs.len() == 1 {
                    if !clusters.contains(&address) {
                        clusters.make_set(address.to_owned());
                    } else {
                        clusters.insert(address.to_owned());
                    }

                    clusters.union(last_address.to_owned(), address.to_owned());
                    last_address = address;
                }
            }
        }

        // Outputs
        if transaction.outputs.len() > 0 {
            for (address, _) in transaction.outputs.iter() {
                if !clusters.contains(&address) {
                    clusters.make_set(address.to_owned());
                }
            }
        }
    }

    fn done(&mut self, clusters: &mut UnionFind<Address, RandomState<xx::Hash64>>) {
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
            match &address.taints {
                Some(_) => {
                    self.writer
                        .write(
                            &format!("{},{},{}\n", pos, digest.to_hex(), address.clone())
                                .as_bytes(),
                        )
                        .expect("Unable to write to output file!");
                }
                None => {}
            }

            for (address, _) in &cache {
                match &address.taints {
                    Some(_) => {
                        self.writer
                            .write(
                                &format!("{},{},{}\n", pos, digest.to_hex(), address.clone())
                                    .as_bytes(),
                            )
                            .expect("Unable to write to output file!");
                    }
                    None => {}
                }
            }
            pos = pos + 1;
            cache.clear();
        }
        info!("Done");
        info!("Found {} clusters", pos);
    }
}
