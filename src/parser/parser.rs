use fasthash::{xx, RandomState};
use rustc_serialize::hex::FromHex;
use std::collections::{HashMap, VecDeque};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use vec_map::VecMap;

use blockchain::address::{Address, Taint};
use blockchain::block::{Block, Transactions};
use blockchain::hash::{Hash, ZERO_HASH};
use blockchain::transaction::Transaction;
use parser::blockchain::Blockchain;
use parser::clusters::Clusters;
use parser::Config;

pub struct Parser {
    input_path: String,
    blocks_dir: String,
    max_block: usize,
    labels: HashMap<String, u8>,
    clusters: Clusters,
}

impl Parser {
    pub fn new(config: &Config) -> Self {
        let input_path = &config.input;
        let blocks_dir = &config.blocks_dir;
        let max_block = config.max_block;

        Self {
            input_path: input_path.to_string(),
            blocks_dir: blocks_dir.to_string(),
            max_block: max_block,
            labels: Default::default(),
            clusters: Clusters::new(config),
        }
    }

    pub fn run(&mut self) {
        let mut goal_prev_hash: Hash = ZERO_HASH;
        let mut last_block: Option<Block> = None;
        let mut height = 0;

        let mut start_txs: HashMap<Hash, VecDeque<Taint>> = Default::default();

        let mut output_items: HashMap<Hash, VecMap<Vec<(Address, u64)>>, RandomState<xx::Hash64>> =
            HashMap::with_hasher(RandomState::<xx::Hash64>::new());

        let mut skipped_blocks: HashMap<Hash, Block, RandomState<xx::Hash64>> =
            HashMap::with_hasher(RandomState::<xx::Hash64>::new());

        let blockchain: Blockchain = Blockchain::new(&self.blocks_dir, self.max_block);

        self.read_input(&mut start_txs);

        for (n, mmap) in blockchain.maps.iter().enumerate() {
            let mmap_slice = &mut &mmap[..];

            info!(
                "Processing block file {}/{}, height {}",
                n,
                blockchain.maps.len() - 1,
                height
            );

            while mmap_slice.len() > 0 {
                if skipped_blocks.contains_key(&goal_prev_hash) {
                    self.on_block(
                        &mut last_block.unwrap().transactions(),
                        last_block.unwrap().header().timestamp(),
                        &mut output_items,
                        &mut start_txs,
                    );
                    height += 1;
                    while let Some(block) = skipped_blocks.remove(&goal_prev_hash) {
                        self.on_block(
                            &mut block.transactions(),
                            block.header().timestamp(),
                            &mut output_items,
                            &mut start_txs,
                        );
                        height += 1;
                        goal_prev_hash = block.header().cur_hash();
                        last_block = None;
                    }
                }

                let block = match Block::read(mmap_slice) {
                    Ok(block) => block,
                    Err(_) => {
                        assert_eq!(mmap_slice.len(), 0);
                        break;
                    }
                };

                if *block.header().prev_hash() != goal_prev_hash {
                    skipped_blocks.insert(*block.header().prev_hash(), block);

                    if last_block.is_some()
                        && block.header().prev_hash() == last_block.unwrap().header().prev_hash()
                    {
                        let first_orphan = last_block.unwrap().clone();
                        let second_orphan = block;

                        loop {
                            let block = match Block::read(mmap_slice) {
                                Ok(block) => block,
                                Err(_) => {
                                    assert_eq!(mmap_slice.len(), 0);
                                    break;
                                }
                            };

                            skipped_blocks.insert(*block.header().prev_hash(), block);
                            if block.header().prev_hash() == &first_orphan.header().cur_hash() {
                                break;
                            }
                            if block.header().prev_hash() == &second_orphan.header().cur_hash() {
                                goal_prev_hash = second_orphan.header().cur_hash();
                                last_block = Some(second_orphan);
                                break;
                            }
                        }
                    }
                    continue;
                }

                if let Some(last_block) = last_block {
                    self.on_block(
                        &mut last_block.transactions(),
                        last_block.header().timestamp(),
                        &mut output_items,
                        &mut start_txs,
                    );
                    height += 1;
                }

                goal_prev_hash = block.header().cur_hash();
                last_block = Some(block);
            }
        }

        self.clusters.done();
    }

    fn on_block(
        &mut self,
        transactions: &mut Transactions,
        timestamp: u32,
        output_items: &mut HashMap<Hash, VecMap<Vec<(Address, u64)>>, RandomState<xx::Hash64>>,
        start_txs: &mut HashMap<Hash, VecDeque<Taint>>,
    ) {
        let mut slice = transactions.slice;

        for _ in 0..transactions.count {
            if slice.len() > 0 {
                let mut transaction: Transaction =
                    match Transaction::read(&mut slice, timestamp, output_items, start_txs) {
                        Ok(transaction) => (transaction),
                        Err(_) => {
                            warn!("Error processing transaction");
                            continue;
                        }
                    };

                self.clusters.on_transaction(&mut transaction);
            }
        }
    }

    fn read_input(&mut self, start_txs: &mut HashMap<Hash, VecDeque<Taint>>) {
        if self.input_path.len() == 0 {
            return;
        }

        let mut label: u8 = 1;
        let path = Path::new(&self.input_path);
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

            if !start_txs.contains_key(&hash) {
                self.labels.insert(tag, label);

                let mut taints: VecDeque<Taint> = VecDeque::new();
                let amount = parts.next().unwrap().parse::<u64>().unwrap();
                taints.push_back(Taint {
                    label: label,
                    amount: amount,
                });
                label += 1;

                start_txs.insert(*hash, taints);
            }
        }
    }
}
