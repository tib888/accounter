/// Account related actions (IDs wrapped in new type to avoid mixing them)
use crate::amount::Amount;
use std::fmt::Display;
use std::str::FromStr;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, PartialOrd)]
pub struct TransactionId(u32);

impl From<u32> for TransactionId {
    fn from(v: u32) -> Self {
        TransactionId(v)
    }
}

impl Display for TransactionId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for TransactionId {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        u32::from_str(s).map(|id| TransactionId(id))
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Transaction {
    Deposit(Amount),
    Withdrawal(Amount),
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct TransactionData {
    pub id: TransactionId,
    pub transaction: Transaction,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Action {
    Transact(TransactionData),
    Dispute(TransactionId),
    Resolve(TransactionId),
    ChargeBack(TransactionId),
}
