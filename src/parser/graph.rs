use crossbeam_channel::Receiver;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{LineWriter, Write};

use blockchain::address::Address;
use parser::disjoint::UnionFind;
use parser::transactions::TransactionMessage;
use parser::Config;

pub struct Graph {
    rx: Receiver<TransactionMessage>,
    writer: LineWriter<File>,
    max_src: usize,
    max_dst: usize,
    edges: usize,
}

impl Graph {
    pub fn new(rx: Receiver<TransactionMessage>, config: &Config) -> Self {
        let output = &config.graph;
        let writer = LineWriter::new(File::create(output).expect("Unable to create output file!"));

        Self {
            writer: writer,
            rx: rx.clone(),
            max_src: 0,
            max_dst: 0,
            edges: 0,
        }
    }

    pub fn run(&mut self, clusters: &mut UnionFind, addresses: &mut HashMap<Address, u32>) {
        let mut done = false;
        let mut uniq: HashMap<String, bool> = HashMap::new();

        loop {
            if !self.rx.is_empty() {
                match self.rx.recv() {
                    Ok(msg) => match msg {
                        TransactionMessage::OnTransaction(mut tx_item) => {
                            self.on_transaction(&mut tx_item, clusters, &mut uniq, addresses);
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
                self.done(&uniq);
                return;
            }
        }
    }

    fn on_transaction(
        &mut self,
        tx_item: &mut Vec<HashSet<Address>>,
        clusters: &mut UnionFind,
        uniq: &mut HashMap<String, bool>,
        addresses: &mut HashMap<Address, u32>,
    ) {
        let outputs = tx_item.pop().unwrap();
        let inputs = tx_item.pop().unwrap();
        if inputs.len() > 0 && outputs.len() > 0 {
            for src in inputs.iter() {
                for dst in outputs.iter() {
                    if src != dst {
                        let src_id = match addresses.get(src) {
                            Some(src_id) => {
                                let id = *src_id;
                                clusters.find(id as usize)
                            }
                            None => {
                                continue;
                            }
                        };
                        let dst_id = match addresses.get(dst) {
                            Some(dst_id) => {
                                let id = *dst_id;
                                clusters.find(id as usize)
                            }
                            None => {
                                continue;
                            }
                        };
                        if src_id != dst_id {
                            match uniq.insert(format!("{} {}", src_id, dst_id), true) {
                                Some(_) => {}
                                None => {
                                    self.edges = self.edges + 1;
                                }
                            }
                            if self.max_src < src_id {
                                self.max_src = src_id;
                            }
                            if self.max_dst < dst_id {
                                self.max_dst = dst_id;
                            }
                        }
                    }
                }
            }
        }
    }

    fn done(&mut self, uniq: &HashMap<String, bool>) {
        self.writer
            .write(&format!("{} {} {}\n", self.max_src, self.max_dst, self.edges).as_bytes())
            .expect("Unable to write to output file!");

        for (edge, _) in uniq {
            self.writer
                .write(&format!("{}\n", edge).as_bytes())
                .expect("Unable to write to output file!");
        }
    }
}
