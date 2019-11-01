pub mod clusterizer;
pub mod disjoint;
pub mod reader;
pub mod consumer;
pub mod chain;

use blockchain::transaction::{Transaction, TransactionInput, TransactionOutput};

pub trait Visitor<'a> {
    type BlockItem;
    type TransactionItem;
    type OutputItem;
    type DoneItem;

    fn new() -> Self;

    fn visit_transaction_begin(&mut self) -> Self::TransactionItem;

    fn visit_transaction_input(
        &mut self,
        _txin: TransactionInput<'a>,
        _tx_item: &mut Self::TransactionItem,
        _output_item: Option<Self::OutputItem>,
    ) {
    }

    fn visit_transaction_output(
        &mut self,
        _txout: TransactionOutput<'a>,
        _tx_item: &mut Self::TransactionItem,
    ) -> Option<Self::OutputItem> {
        None
    }

    fn visit_transaction_end(&mut self, _tx: Transaction<'a>, _tx_item: Self::TransactionItem) {}

    fn done(&mut self) {}
}
