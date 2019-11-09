use crossbeam_channel::Receiver;
use fasthash::{xx, RandomState};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{LineWriter, Write};

use blockchain::address::Address;
use parser::transactions::TransactionMessage;
use parser::union::UnionFind;
use parser::Config;

pub struct Graph {
    rx: Receiver<TransactionMessage>,
    writer: LineWriter<File>,
}

impl Graph {
    pub fn new(rx: Receiver<TransactionMessage>, config: &Config) -> Self {
        let output = &config.graph;
        let writer = LineWriter::new(File::create(output).expect("Unable to create output file!"));

        Self {
            writer: writer,
            rx: rx.clone(),
        }
    }

    pub fn run(&mut self, clusters: &mut UnionFind<Address, RandomState<xx::Hash64>>) {
        let mut done = false;
        //let mut uniq: HashMap<String, bool, RandomState<xx::Hash64>> =
        //    HashMap::with_capacity_and_hasher(1_000_000, RandomState::<xx::Hash64>::new());

        loop {
            if !self.rx.is_empty() {
                match self.rx.recv() {
                    Ok(msg) => match msg {
                        TransactionMessage::OnTransaction((mut tx_item, _)) => {
                            self.on_transaction(&mut tx_item, clusters);
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
                //self.done(&uniq);
                return;
            }
        }
    }

    fn on_transaction(
        &mut self,
        tx_item: &mut Vec<HashSet<Address>>,
        clusters: &mut UnionFind<Address, RandomState<xx::Hash64>>,
        //uniq: &mut HashMap<String, bool, RandomState<xx::Hash64>>,
    ) {
        let outputs = tx_item.pop().unwrap();
        let inputs = tx_item.pop().unwrap();

        if inputs.len() == 0 || outputs.len() == 0 {
            return;
        }

        let mut inputs_map: HashMap<Address, usize, RandomState<xx::Hash64>> =
            HashMap::with_capacity_and_hasher(100, RandomState::<xx::Hash64>::new());

        let mut outputs_map: HashMap<Address, usize, RandomState<xx::Hash64>> =
            HashMap::with_capacity_and_hasher(100, RandomState::<xx::Hash64>::new());

        for src in inputs.iter() {
            match clusters.ids.get(&src) {
                Some(src_id) => {
                    inputs_map.insert(src.clone(), *src_id as usize);
                }
                None => {
                    continue;
                }
            }
        }

        for dst in outputs.iter() {
            match clusters.ids.get(&dst) {
                Some(dst_id) => {
                    outputs_map.insert(dst.clone(), *dst_id as usize);
                }
                None => {
                    continue;
                }
            }
        }

        for (_, src_id) in outputs_map.iter() {
            for (_, dst_id) in outputs_map.iter() {
                if src_id != dst_id {
                    self.writer
                        .write(&format!("{} {}\n", src_id, dst_id).as_bytes())
                        .expect("Unable to write to output file!");
                }
            }
        }
    }

    /*
    fn done(&mut self, uniq: &HashMap<String, bool, RandomState<xx::Hash64>>) {
        self.writer
            .write(&format!("{} {} {}\n", self.max_src, self.max_dst, self.edges).as_bytes())
            .expect("Unable to write to output file!");

        for (edge, _) in uniq {
            self.writer
                .write(&format!("{}\n", edge).as_bytes())
                .expect("Unable to write to output file!");
        }
    }
    */
}
