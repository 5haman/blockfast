use crossbeam_channel::Sender;
use std::collections::HashMap;
use std::collections::HashSet;
use vec_map::VecMap;

use blockchain::address::Address;
use blockchain::block::Block;
use blockchain::hash::{Hash, ZERO_HASH};
use blockchain::transaction::Transaction;
use reader::chain::Blockchain;
use types::ThreadResult;

pub struct Reader {
    tx: Sender<ThreadResult>,
}

impl Reader {
    pub fn new(tx: Sender<ThreadResult>) -> Self {
        Self { tx: tx.clone() }
    }

    pub fn run(&self, chain: &Blockchain) {
        let mut output_items: HashMap<Hash, VecMap<Vec<Address>>> = Default::default();
        let mut skipped: HashMap<Hash, Block> = Default::default();
        let mut goal_prev_hash: Hash = ZERO_HASH;
        let mut last_block: Option<Block> = None;
        let mut height = 0;

        for (n, mmap) in chain.maps.iter().enumerate() {
            let mmap_slice = &mut &mmap[..];

            info!(
                "Processing block file {}/{}, height {}",
                n,
                chain.maps.len() - 1,
                height
            );
            while mmap_slice.len() > 0 {
                if skipped.contains_key(&goal_prev_hash) {
                    self.on_block(&last_block.unwrap(), &mut output_items);
                    height += 1;
                    while let Some(block) = skipped.remove(&goal_prev_hash) {
                        self.on_block(&block, &mut output_items);
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
                    skipped.insert(*block.header().prev_hash(), block);

                    if last_block.is_some()
                        && block.header().prev_hash() == last_block.unwrap().header().prev_hash()
                    {
                        let first_orphan = last_block.unwrap();
                        let second_orphan = block;

                        loop {
                            let block = match Block::read(mmap_slice) {
                                Ok(block) => block,
                                Err(_) => {
                                    assert_eq!(mmap_slice.len(), 0);
                                    break;
                                }
                            };

                            skipped.insert(*block.header().prev_hash(), block);
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
                    self.on_block(&last_block, &mut output_items);
                    height += 1;
                }

                goal_prev_hash = block.header().cur_hash();
                last_block = Some(block);
            }
        }

        self.tx
            .send(ThreadResult::OnComplete("Finished".to_string()))
            .unwrap();
    }

    pub fn on_block(&self, block: &Block, output_items: &mut HashMap<Hash, VecMap<Vec<Address>>>) {
        let transactions = block.transactions();
        let mut slice = transactions.slice;
        for _ in 0..transactions.count {
            let mut transaction_item = HashSet::<Address>::new();

            match Transaction::read(
                &mut slice,
                block.header().timestamp(),
                output_items,
                &mut transaction_item,
            ) {
                Ok(ok) => {
                    if ok {
                        self.tx
                            .send(ThreadResult::OnTransaction(transaction_item))
                            .unwrap();
                    }
                }
                Err(_) => {}
            }
        }

        assert_eq!(slice.len(), 0);
    }
}
