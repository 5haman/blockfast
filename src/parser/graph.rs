use crossbeam_channel::Receiver;
use fast_paths::InputGraph;
use fasthash::{xx, RandomState};
use std::collections::hash_map::Entry as HashEntry;
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
    graph: HashMap<usize, HashMap<usize, usize, RandomState<xx::Hash64>>, RandomState<xx::Hash64>>,
}

impl Graph {
    pub fn new(rx: Receiver<TransactionMessage>, config: &Config) -> Self {
        let output = &config.graph;
        let writer = LineWriter::new(File::create(output).expect("Unable to create output file!"));

        Self {
            writer: writer,
            rx: rx.clone(),
            graph: HashMap::with_capacity_and_hasher(1_000_000, RandomState::<xx::Hash64>::new()),
        }
    }

    pub fn run(&mut self, clusters: &mut UnionFind<Address, RandomState<xx::Hash64>>) {
        let mut done = false;

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
                self.done();
                return;
            }
        }
    }

    fn on_transaction(
        &mut self,
        tx_item: &mut Vec<HashSet<Address>>,
        clusters: &mut UnionFind<Address, RandomState<xx::Hash64>>,
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
                    if let HashEntry::Occupied(mut set) = self.graph.entry(*src_id) {
                        let set = set.get_mut();
                        if let HashEntry::Occupied(mut entry) = set.entry(*dst_id) {
                            let weight = entry.get_mut();
                            *weight = *weight + 1;
                        } else {
                            set.insert(*dst_id, 1);
                        }
                    } else {
                        let mut set: HashMap<usize, usize, RandomState<xx::Hash64>> =
                            HashMap::with_hasher(RandomState::<xx::Hash64>::new());
                        set.insert(*dst_id, 1);
                        self.graph.insert(*src_id, set);
                    }
                }
            }
        }
    }

    fn done(&mut self) {
        let mut graph = InputGraph::new();

        let mut count = 0;
        for (src_id, set) in &self.graph {
            for (dst_id, weight) in set {
                graph.add_edge(*src_id, *dst_id, *weight);
                count = count + 1;
                self.writer
                    .write(&format!("{} {}\n", src_id, dst_id).as_bytes())
                    .expect("Unable to write to output file!");
            }
        }

        graph.freeze();
        let fast_graph = fast_paths::prepare(&graph);
        let _ = fast_paths::save_to_disk(&fast_graph, "graph.dat");
    }
}
