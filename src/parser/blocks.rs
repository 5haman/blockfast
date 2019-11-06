use crossbeam_channel::Sender;
use std::collections::HashMap;

use blockchain::block::Block;
use blockchain::hash::{Hash, ZERO_HASH};
use parser::blockchain::Blockchain;
use parser::ParseError;

pub enum BlockMessage<'a> {
    OnBlock(Block<'a>),
    OnComplete(bool),
    OnError(ParseError),
}

pub struct Blocks<'a> {
    tx: Sender<BlockMessage<'a>>,
}

impl<'a> Blocks<'a> {
    pub fn new(tx: Sender<BlockMessage<'a>>) -> Self {
        Self { tx: tx.clone() }
    }

    pub fn run(&mut self, blockchain: &'a Blockchain) {
        let mut skipped: HashMap<Hash, Block> = HashMap::default();
        let mut goal_prev_hash: Hash = ZERO_HASH;
        let mut last_block: Option<Block> = None;
        let mut height = 0;

        for (n, mmap) in blockchain.maps.iter().enumerate() {
            let mmap_slice = &mut &mmap[..];

            info!(
                "Processing block file {}/{}, height {}",
                n,
                blockchain.maps.len() - 1,
                height
            );
            while mmap_slice.len() > 0 {
                if skipped.contains_key(&goal_prev_hash) {
                    self.tx
                        .send(BlockMessage::OnBlock(last_block.unwrap()))
                        .unwrap();
                    height += 1;
                    while let Some(block) = skipped.remove(&goal_prev_hash) {
                        self.tx.send(BlockMessage::OnBlock(block)).unwrap();
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
                    self.tx.send(BlockMessage::OnBlock(last_block)).unwrap();
                    height += 1;
                }

                goal_prev_hash = block.header().cur_hash();
                last_block = Some(block);
            }
        }

        self.tx.send(BlockMessage::OnComplete(true)).unwrap();
        return;
    }
}
