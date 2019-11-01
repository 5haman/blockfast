use blockchain::chain::Blockchain;
use crossbeam_channel::{Receiver, Sender, TryRecvError};
use crossbeam_utils::thread;

use blockchain::address::Address;
use blockchain::block::Block;
use blockchain::error::ParseError;
use blockchain::error::ParseResult;
use blockchain::hash::Hash;
use blockchain::hash::ZERO_HASH;
use blockchain::transaction::Transaction;
use std::collections::HashMap;
use vec_map::VecMap;

use visitors::clusterizer::Clusterizer;
use visitors::Visitor;

pub enum ThreadResult<'a> {
    OnTransaction(Transaction<'a>),
    OnComplete(String),
    OnError(ParseError),
}

pub struct Parser<'a> {
    tx: Sender<ThreadResult<'a>>,
}

pub struct ThreadReceiver<'a> {
    rx: Receiver<ThreadResult<'a>>,
}

impl<'a> Parser<'a> {
    pub fn new(tx: Sender<ThreadResult<'a>>) -> Self {
        Self { tx: tx.clone() }
    }

    pub fn handle(&self, chain: &'a Blockchain) {
        //let mut visitor = Clusterizer::new();
        let mut skipped: HashMap<Hash, Block> = Default::default();
        //let mut output_items: HashMap<Hash, VecMap<Vec<Address>>> = Default::default();
        let mut goal_prev_hash: Hash = ZERO_HASH;
        let mut last_block: Option<Block> = None;
        let mut height = 0;

        for (n, mmap) in chain.maps.iter().enumerate() {
            let mmap_slice = &mut &mmap[..];

            info!(
                "Processing block file {}/{}, height {}",
                n,
                chain.maps.len() - 1,
                height,
            );

            while mmap_slice.len() > 0 {
                if skipped.contains_key(&goal_prev_hash) {
                    self.read_block(&last_block.unwrap(), height);
                    height += 1;
                    while let Some(block) = skipped.remove(&goal_prev_hash) {
                        self.read_block(&block, height);
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
                    self.read_block(&last_block, height);
                    height += 1;
                }

                goal_prev_hash = block.header().cur_hash();
                last_block = Some(block);
            }
        }

        //visitor.done();

        self.tx.send(ThreadResult::OnComplete("Finished".to_string()))
            .unwrap();
    }

    pub fn read_block(
        &self,
        block: &Block<'a>,
        //visitor: &mut Clusterizer,
        height: u64,
        //output_items: &mut HashMap<Hash, VecMap<Vec<Address>>>,
    ) {
        let header = block.header();
        //let mut block_item = visitor.visit_block_begin(*self, height);
        match block.transactions() {
            Ok(transactions) => {
                let mut slice = transactions.slice;
                for _ in 0..transactions.count {
                    match Transaction::read_and_walk(
                        &mut slice,
                        //visitor,
                        header.timestamp(),
                        height,
                        //block_item,
                        //output_items,
                    ) {
                        Ok(transaction) => {
                            self.tx.send(ThreadResult::OnTransaction(transaction)).unwrap();
                        }
                        Err(err) => {
                            self.tx.send(ThreadResult::OnError(err)).unwrap();
                        }
                    }
                }
                assert_eq!(slice.len(), 0);
            }
            Err(_) => {}
        }

        //visitor.visit_block_end(*self, height, block_item);
    }
}

impl<'a> ThreadReceiver<'a> {
    pub fn new(rx: Receiver<ThreadResult<'a>>) -> Self {
        Self { rx: rx.clone() }
    }

    pub fn handle(&self) {
        loop {
            match self.rx.try_recv() {
                Ok(msg) => {
                    match msg {
                        ThreadResult::OnTransaction(transaction) => {
                            //info!("on transaction: {}", transaction.txins_count);
                        }
                        ThreadResult::OnComplete(msg) => {
                            return;
                        }
                        ThreadResult::OnError(err) => {
                            info!("Error processing transaction");
                            //return;
                        }
                    }
                }
                Err(TryRecvError::Disconnected) => {
                    return;
                }
                Err(TryRecvError::Empty) => {}
            }
        }
    }
}
