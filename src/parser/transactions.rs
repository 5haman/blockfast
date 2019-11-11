use crossbeam_channel::{Receiver, Sender};
use fasthash::{xx, RandomState};
use std::collections::HashMap;
use vec_map::VecMap;

use blockchain::address::Address;
use blockchain::hash::Hash;
use blockchain::transaction::Transaction;
use parser::blocks::BlockMessage;
use parser::ParseError;

pub enum TransactionMessage {
    OnTransaction(Transaction),
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
        let mut output_items: HashMap<Hash, VecMap<Vec<(Address, u64)>>, RandomState<xx::Hash64>> =
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

                                    let transaction: Transaction = match Transaction::read(
                                        &mut slice,
                                        block.header().timestamp(),
                                        &mut output_items
                                    ) {
                                        Ok(transaction) => (transaction),
                                        Err(_) => {
                                            warn!("Error processing transaction");
                                            continue;
                                        }
                                    };

                                    self.tx
                                        .send(TransactionMessage::OnTransaction(transaction))
                                        .unwrap();
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
