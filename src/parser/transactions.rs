use crossbeam_channel::{Receiver, Sender};
use fasthash::{xx, RandomState};
use rustc_serialize::hex::FromHex;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use vec_map::VecMap;

use blockchain::address::{Address, Taint};
use blockchain::block::Transactions as TransactionList;
use blockchain::hash::Hash;
use blockchain::transaction::Transaction;
use parser::blocks::BlockMessage;
use parser::Config;
use parser::ParseError;

pub enum TransactionMessage {
    OnTransaction(Transaction),
    OnComplete(bool),
    OnError(ParseError),
}

pub struct Transactions<'a> {
    tx: Sender<TransactionMessage>,
    rx: Receiver<BlockMessage<'a>>,
    input: String,
    start_txs: HashMap<Hash, VecDeque<Taint>>,
    labels: HashMap<String, u8>,
}

impl<'a> Transactions<'a> {
    pub fn new(
        rx: Receiver<BlockMessage<'a>>,
        tx: Sender<TransactionMessage>,
        config: &Config,
    ) -> Self {
        let input = &config.input;

        Self {
            rx: rx,
            tx: tx,
            input: input.to_string(),
            start_txs: Default::default(),
            labels: Default::default(),
        }
    }

    pub fn run(&mut self) {
        let mut done = false;
        let mut output_items: HashMap<Hash, VecMap<Vec<(Address, u64)>>, RandomState<xx::Hash64>> =
            HashMap::with_capacity_and_hasher(1_000_000, RandomState::<xx::Hash64>::new());

        self.read_input();

        loop {
            if !self.rx.is_empty() {
                match self.rx.recv() {
                    Ok(msg) => match msg {
                        BlockMessage::OnBlock(block) => {
                            self.on_block(
                                &mut block.transactions(),
                                block.header().timestamp(),
                                &mut output_items,
                            );
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

    fn on_block(
        &mut self,
        transactions: &mut TransactionList,
        timestamp: u32,
        output_items: &mut HashMap<Hash, VecMap<Vec<(Address, u64)>>, RandomState<xx::Hash64>>,
    ) {
        let mut slice = transactions.slice;

        for _ in 0..transactions.count {
            if slice.len() > 0 {
                let transaction: Transaction = match Transaction::read(
                    &mut slice,
                    timestamp,
                    output_items,
                    &mut self.start_txs,
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

    fn read_input(&mut self) {
        if self.input.len() == 0 {
            return;
        }

        let mut label: u8 = 1;
        let path = Path::new(&self.input);
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

            if !self.start_txs.contains_key(&hash) {
                self.labels.insert(tag, label);

                let mut taints: VecDeque<Taint> = VecDeque::new();
                let amount = parts.next().unwrap().parse::<u64>().unwrap();
                taints.push_back(Taint {
                    label: label,
                    amount: amount,
                });
                label += 1;

                self.start_txs.insert(*hash, taints);
            }
        }
    }
}
