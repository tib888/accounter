/// Account related actions
use std::fmt::Display;
use std::str::FromStr;

/// Transaction ids wrapped in new type to avoid mixing them with other ids
use crate::amount::Amount;
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
    /// Means: increase the balance of an account by the given amount
    Deposit(Amount),
    /// Means: decrease the balance of an account by the given amount
    Withdrawal(Amount),
}

/// List of account manipulation actions
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Action {
    Transact((TransactionId, Transaction)),
    Dispute(TransactionId),
    Resolve(TransactionId),
    ChargeBack(TransactionId),
}
