/// Account related actions (IDs wrapped in new type to avoid mixing them)
use crate::amount::Amount;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, PartialOrd)]
pub struct TransactionId(u32);

impl From<u32> for TransactionId {
    fn from(v: u32) -> Self {
        TransactionId(v)
    }
}

#[derive(Debug, PartialEq, Clone, Copy, PartialOrd)]
pub enum Transaction {
    Deposit(Amount),
    Withdrawal(Amount),
}

#[derive(Debug, PartialEq, Clone, Copy, PartialOrd)]
pub struct TransactionData {
    pub id: TransactionId,
    pub transaction: Transaction,
}

#[derive(Debug, PartialEq, Clone, Copy, PartialOrd)]
pub enum Action {
    Transact(TransactionData),
    Dispute(TransactionId),
    Resolve(TransactionId),
    ChargeBack(TransactionId),
}
