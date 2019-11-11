use crossbeam_channel::Receiver;
use crypto::digest::Digest;
use crypto::md5::Md5;
use fasthash::{xx, RandomState};
use rustc_serialize::hex::{FromHex, ToHex};
use std::collections::hash_map::Entry as HashEntry;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs::File;
use std::io::{BufRead, BufReader, LineWriter, Write};
use std::path::Path;

use blockchain::address::{Address, Taint};
use blockchain::hash::Hash;
use blockchain::transaction::Transaction;
use parser::transactions::TransactionMessage;
use parser::union::UnionFind;
use parser::Config;

pub struct Clusters {
    rx: Receiver<TransactionMessage>,
    writer: LineWriter<File>,
    input: String,
    start_txs: HashMap<Hash, VecDeque<Taint>>,
    taint_outputs: HashSet<Address>,
    labels: HashMap<String, u8>,
}

impl Clusters {
    pub fn new(rx: Receiver<TransactionMessage>, config: &Config) -> Self {
        let output = &config.output;
        let writer = LineWriter::new(File::create(output).expect("Unable to create output file!"));

        let input = &config.input;

        Self {
            writer: writer,
            input: input.to_string(),
            rx: rx.clone(),
            start_txs: Default::default(),
            labels: HashMap::new(),
            taint_outputs: HashSet::new(),
        }
    }

    fn read_initial_list(&mut self) {
        let mut label: u8 = 1;
        let path = Path::new(&self.input);
        let file = File::open(path).unwrap();
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let buf = line.unwrap();
            let mut parts = buf.split(",");
            let tx = parts.next().unwrap();
            let tag = String::from(parts.next().unwrap());

            let txid: &mut [u8] = &mut tx.to_string().from_hex().unwrap();
            txid.reverse();
            let hash = Hash::from_slice(array_ref!(txid, 0, 32));

            if !self.start_txs.contains_key(&hash) {
                self.labels.insert(tag, label);

                let mut taints: VecDeque<Taint> = VecDeque::new();
                let amount = parts.next().unwrap().parse::<u64>().unwrap();
                taints.push_back(Taint {
                    label: label,
                    amount: amount,
                });
                label += 1;

                self.start_txs.insert(*hash, taints);
            }
        }
    }

    pub fn run(&mut self, clusters: &mut UnionFind<Address, RandomState<xx::Hash64>>) {
        let mut done = false;

        self.read_initial_list();

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
            
            let (output1, amount) = transaction.outputs.iter().next().unwrap();
            if transaction.outputs.len() == 2 && transaction.inputs.len() != 2 {
                let (output2, balance) = &transaction.outputs.iter().next().unwrap();

                if !clusters.contains(&output1) && clusters.contains(&output2) {
                    let amount = *amount / 100_000_000;
                    let vals = format!("{}", amount);
                    let vec: Vec<&str> = vals.split(".").collect();
                    let len = vec.last().unwrap().len();

                    if len > 4 {
                        is_cluster = true;
                        let mut addr = output1.clone();
                        addr.balance = **balance;
                        clusters.make_set(addr);
                    }
                }

                for (address, _) in transaction.inputs.iter() {
                    if *address == *output1 || address == *output2 {
                        is_cluster = false;
                        break;
                    }
                }
            }


            if is_cluster {
                clusters.union(last_address.to_owned(), output1.clone());
            }

            self.check_taint(&mut last_address.to_owned(), transaction, clusters);
            if !clusters.contains(&last_address) {
                clusters.make_set(last_address.to_owned());
            } else {
                clusters.insert(last_address.to_owned());
            }

            for (address, amount) in tx_inputs {
                let remove = self.check_taint(&mut address.to_owned(), transaction, clusters);

                if !clusters.contains(&address) {
                    clusters.make_set(address.to_owned());
                } else {
                    clusters.insert(address.to_owned());
                }

                if transaction.outputs.len() == 1 {
                    clusters.union(last_address.to_owned(), address.to_owned());
                }
                last_address = address;

                if let HashEntry::Occupied(mut old_amount) = clusters.ids.entry(address.to_owned())
                {
                    let mut _entry = *old_amount.get_mut();
                    if _entry >= *amount as usize {
                        _entry -= *amount as usize;
                    }
                }
                if remove {
                    self.taint_outputs.remove(&address);
                }
            }
        }

        // Outputs
        if transaction.outputs.len() > 0 {
            let mut remove = false;
            for (address, amount) in transaction.outputs.iter() {
                if clusters.contains(&address) {
                    let (old_address, _) = clusters.ids.get_key_value(&address).unwrap();
                    let mut old_address = old_address.clone();
                    old_address.balance += *amount;
                    remove = self.check_start_taint(&mut old_address, &transaction, *amount);
                    clusters.insert(old_address.to_owned());
                } else {
                    let mut address = address.clone();
                    address.balance = *amount;
                    remove = self.check_start_taint(&mut address, &transaction, *amount);
                    clusters.make_set(address.to_owned());
                }
            }

            if remove {
                self.start_txs.remove(&transaction.txid);
            }
        }
    }

    fn check_taint(
        &mut self,
        address: &Address,
        transaction: &Transaction,
        clusters: &mut UnionFind<Address, RandomState<xx::Hash64>>,
    ) -> bool {
        let mut remove = true;
        if self.taint_outputs.contains(&address) {
            let address_taint = self.taint_outputs.get(&address).unwrap();
            let mut taints = address_taint.to_owned().taints.unwrap();
            for (address_out, amount) in transaction.outputs.iter() {
                let mut address_out = address_out.to_owned();
                address_out.taints = check_taint(&mut taints, *amount);
                if !clusters.contains(&address_out) {
                    clusters.make_set(address_out.to_owned());
                } else {
                    clusters.insert(address_out.to_owned());
                }

                for t in address_out.to_owned().taints.unwrap() {
                    if t.label != 0 {
                        remove = false;
                        self.taint_outputs.insert(address_out.clone());
                        break;
                    }
                }
            }
        }

        return remove;
    }

    fn check_start_taint(
        &mut self,
        address: &mut Address,
        transaction: &Transaction,
        amount: u64,
    ) -> bool {
        let mut remove = false;
        if self.start_txs.contains_key(&transaction.txid) {
            remove = true;

            let mut taint = self.start_txs.get_mut(&transaction.txid).unwrap();
            address.taints = check_taint(&mut taint, amount);
            self.taint_outputs.insert(address.clone());
        }

        return remove;
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
            //let mut addr = address.clone();
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

fn check_taint(taints: &mut VecDeque<Taint>, amount: u64) -> Option<VecDeque<Taint>> {
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
