use crossbeam_channel::{Sender};
use std::collections::HashMap;

use blockchain::block::{Block};
use reader::chain::Blockchain;
use types::{ThreadResult};
use blockchain::hash::{Hash, ZERO_HASH};
use blockchain::transaction::Transaction;

//use reader::Visitor;

pub struct Reader<'a> {
    tx: Sender<ThreadResult<'a>>,
}

impl<'a> Reader<'a> {
    pub fn new(tx: Sender<ThreadResult<'a>>) -> Self {
        Self { tx: tx.clone() }
    }

    pub fn run(&self, chain: &'a Blockchain) {
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
                    self.on_block(&last_block.unwrap(), height);
                    height += 1;
                    while let Some(block) = skipped.remove(&goal_prev_hash) {
                        self.on_block(&block, height);
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
                    self.on_block(&last_block, height);
                    height += 1;
                }

                goal_prev_hash = block.header().cur_hash();
                last_block = Some(block);
            }
        }

        //visitor.done();

        self.tx
            .send(ThreadResult::OnComplete("Finished".to_string()))
            .unwrap();
    }

    pub fn on_block(&self, block: &Block<'a>, height: u64) {
        let mut transactions = block.transactions();
        let mut slice = transactions.slice;
        for _ in 0..transactions.count {
            match Transaction::read(&mut slice, block.header().timestamp(), height) {
                Ok(transaction) => {
                    self.tx
                        .send(ThreadResult::OnTransaction(transaction))
                        .unwrap();
                }
                Err(err) => {
                    self.tx.send(ThreadResult::OnError(err)).unwrap();
                }
            }
        }

        assert_eq!(slice.len(), 0);
    }
}
