use std::collections::HashSet;
use crossbeam_channel::Receiver;
use std::fs::File;
use std::io::{LineWriter, Write};

use types::ThreadResult;
use blockchain::address::Address;
use reader::disjoint::DisjointSet;

pub struct Consumer {
    rx: Receiver<ThreadResult>,
    clusters: DisjointSet<Address>,
    writer: LineWriter<File>,
}

impl Consumer {
    pub fn new(rx: Receiver<ThreadResult>) -> Self {
        Self {
            rx: rx.clone(),
            clusters: DisjointSet::<Address>::new(),
            writer: LineWriter::new(
                File::create("nodes.csv").expect("Unable to create nodes file!"),
            ),
        }
    }

    pub fn run(&mut self) {
        let mut done = false;
        loop {
            if !self.rx.is_empty() {
                match self.rx.recv() {
                    Ok(msg) => match msg {
                        ThreadResult::OnTransaction(tx_item) => {
                            self.on_transaction(&tx_item);
                        }
                        ThreadResult::OnComplete(_) => {
                            done = true;
                        }
                        ThreadResult::OnError(_) => {
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

    fn on_transaction(
        &mut self,
        tx_item: &HashSet<Address>,
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
        for (address, tag) in &self.clusters.map {
            self.writer
                .write(&format!("{} {}\n", self.clusters.parent[*tag], address).as_bytes())
                .expect("Unable to write nodes file!");
        }
    }
}
