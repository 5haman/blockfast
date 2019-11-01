use crossbeam_channel::{Receiver};

use types::{ThreadResult};

//use reader::clusterizer::Clusterizer;
use reader::Visitor;

pub struct Consumer<'a> {
    rx: Receiver<ThreadResult<'a>>,
}

impl<'a> Consumer<'a> {
    pub fn new(rx: Receiver<ThreadResult<'a>>) -> Self {
        Self { rx: rx.clone() }
    }

    pub fn run(&self) {
        loop {
            match self.rx.recv() {
                Ok(msg) => match msg {
                    ThreadResult::OnTransaction(transaction) => {
                        debug!("Received transaction: {}", transaction.txid);
                    }
                    ThreadResult::OnComplete(msg) => {
                        return;
                    }
                    ThreadResult::OnError(err) => {
                        warn!("Error processing transaction");
                    }
                },
                Err(_) => {
                    warn!("Error processing transaction");
                }
            }
        }
    }
}
