use crossbeam_channel::{Receiver, Sender};
use fasthash::{xx, RandomState};
use std::collections::{HashMap, HashSet};
use vec_map::VecMap;

use blockchain::address::Address;
use blockchain::hash::Hash;
use blockchain::transaction::Transaction;
use parser::blocks::BlockMessage;
use parser::ParseError;

pub enum TransactionMessage {
    OnTransaction((Vec<HashSet<Address>>, Vec<u64>)),
    OnComplete(bool),
    OnError(ParseError),
}

pub struct Transactions<'a> {
    tx: Sender<TransactionMessage>,
    rx: Receiver<BlockMessage<'a>>,
}

impl<'a> Transactions<'a> {
    pub fn new(rx: Receiver<BlockMessage<'a>>, tx: Sender<TransactionMessage>) -> Self {
        Self { rx: rx, tx: tx }
    }

    pub fn run(&self) {
        let mut done = false;
        let mut output_items: HashMap<Hash, VecMap<Vec<Address>>, RandomState<xx::Hash64>> =
            HashMap::with_capacity_and_hasher(1_000_000, RandomState::<xx::Hash64>::new());

        loop {
            if !self.rx.is_empty() {
                match self.rx.recv() {
                    Ok(msg) => match msg {
                        BlockMessage::OnBlock(block) => {
                            let transactions = block.transactions();
                            let mut slice = transactions.slice;

                            for _ in 0..transactions.count {
                                if slice.len() > 0 {
                                    let mut inputs = HashSet::<Address>::with_capacity(100);
                                    let mut outputs = HashSet::<Address>::with_capacity(100);
                                    let mut values = Vec::<u64>::with_capacity(100);

                                    match Transaction::read(
                                        &mut slice,
                                        block.header().timestamp(),
                                        &mut output_items,
                                        &mut inputs,
                                        &mut outputs,
                                        &mut values,
                                    ) {
                                        Ok(ok) => {
                                            if ok {
                                                let mut tx_msg = Vec::new();

                                                tx_msg.push(inputs);
                                                tx_msg.push(outputs);
                                                self.tx
                                                    .send(TransactionMessage::OnTransaction((
                                                        tx_msg, values,
                                                    )))
                                                    .unwrap();
                                            }
                                        }
                                        Err(_) => {
                                            warn!("Error processing transaction");
                                        }
                                    }
                                }
                            }
                            assert_eq!(slice.len(), 0);
                        }
                        BlockMessage::OnComplete(_) => {
                            done = true;
                        }
                        BlockMessage::OnError(_) => {
                            warn!("Error processing block");
                        }
                    },
                    Err(_) => {
                        warn!("Error processing block");
                    }
                }
            } else if done {
                self.tx.send(TransactionMessage::OnComplete(true)).unwrap();
                return;
            }
        }
    }
}
