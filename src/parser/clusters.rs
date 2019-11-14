use crypto::digest::Digest;
use crypto::md5::Md5;
use fasthash::{xx, RandomState};
use rustc_serialize::hex::ToHex;
use std::fs::File;
use std::io::{LineWriter, Write};

use blockchain::address::Address;
use blockchain::transaction::Transaction;
use parser::union::UnionFind;
use parser::Config;

pub struct Clusters {
    writer: LineWriter<File>,
    clusters: UnionFind<Address, RandomState<xx::Hash64>>,
}

impl Clusters {
    pub fn new(config: &Config) -> Self {
        let output = &config.output;
        let writer = LineWriter::new(File::create(output).expect("Unable to create output file!"));
        let clusters: UnionFind<Address, RandomState<xx::Hash64>> =
            UnionFind::with_hasher(RandomState::<xx::Hash64>::new());

        Self {
            writer: writer,
            clusters: clusters,
        }
    }

    pub fn on_transaction(&mut self, transaction: &mut Transaction) {
        // Inputs
        if transaction.inputs.len() > 0 {
            let mut tx_inputs = transaction.inputs.iter();
            let (mut last_address, _) = tx_inputs.next().unwrap();
            let mut is_cluster = false;

            if transaction.outputs.len() > 0 {
                let (output1, amount) = transaction.outputs.iter().next().unwrap();
                if transaction.outputs.len() == 2 && transaction.inputs.len() != 2 {
                    let (output2, _) = &transaction.outputs.iter().next().unwrap();

                    if !self.clusters.contains(&output1) && self.clusters.contains(&output2) {
                        let amount = *amount / 100_000_000;
                        let vals = format!("{}", amount);
                        let vec: Vec<&str> = vals.split(".").collect();
                        let len = vec.last().unwrap().len();

                        if len > 4 {
                            is_cluster = true;
                            self.clusters.make_set(output1.to_owned());
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
                    if !self.clusters.contains(&last_address) {
                        self.clusters.make_set(last_address.to_owned());
                    }
                    if is_cluster {
                        self.clusters
                            .union(last_address.to_owned(), output1.clone());
                    }
                }
            }

            for (address, _) in tx_inputs {
                if transaction.outputs.len() == 1 {
                    if !self.clusters.contains(&address) {
                        self.clusters.make_set(address.to_owned());
                    }

                    self.clusters
                        .union(last_address.to_owned(), address.to_owned());
                    last_address = address;
                }
            }
        }

        // Outputs
        if transaction.outputs.len() == 1 {
            for (address, _) in transaction.outputs.iter() {
                if !self.clusters.contains(&address) {
                    self.clusters.make_set(address.to_owned());
                }
            }
        }
    }

    pub fn done(&mut self) {
        info!("Done");
        info!("Found {} addresses", self.clusters.len());

        let prefix = "kyblsoft.cz".to_string();
        let mut cache: Vec<(Address, [u8; 32])> = Default::default();
        let mut pos = 0;
        let mut count = 0;

        for set in self.clusters.into_iter() {
            for address in set.into_iter() {
                match &address.taints {
                    Some(_) => {
                        let mut hasher = Md5::new();
                        let mut hash = [0u8; 32];
                        hasher.input(format!("{}:{}", prefix, address).as_bytes());
                        hasher.result(&mut hash);
                        cache.push((address.clone(), hash));
                        count = count + 1;
                    }
                    None => {}
                }
                if count % 1000000 == 0 && count != 0 {
                    info!("Processed {} addresses, {} self.clusters", count, pos);
                }
            }
            cache.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

            if cache.len() > 0 {
                let (address, hash) = cache.pop().unwrap();
                let digest = array_ref!(hash, 0, 8);
                self.writer
                    .write(&format!("{},{},{}\n", pos, digest.to_hex(), address.clone()).as_bytes())
                    .expect("Unable to write to output file!");

                for (address, _) in &cache {
                    self.writer
                        .write(
                            &format!("{},{},{}\n", pos, digest.to_hex(), address.clone())
                                .as_bytes(),
                        )
                        .expect("Unable to write to output file!");
                }
                pos = pos + 1;
                cache.clear();
            }
        }
        info!("Done");
        info!("Found {} clusters", pos);
    }
}
