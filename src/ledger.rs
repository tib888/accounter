use crate::actions::TransactionId;
use crate::amount::Amount;
use std::collections::HashMap;

pub trait Ledger {
    type Error;
    type Key;
    type Value;

    fn contains(&self, key: Self::Key) -> Result<bool, Self::Error> {
        self.get(key).map(|_| true)
    }

    fn get(&self, key: Self::Key) -> Result<Option<Self::Value>, Self::Error>;

    /// must always check if returned with success! (a real db could return Err<DbError>)
    /// NOTE: if the network would lose the response of the server that is a big problem!!!
    #[must_use]
    fn insert(&mut self, key: Self::Key, state: Self::Value) -> Result<(), Self::Error>;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransactionState {
    Deposit(Amount),
    DepositInDispute(Amount),
    ChargedBack(Amount),
    //InDisputeWithdrawal(Amount),  //TODO ASK!
    Withdrawal(Amount), //TODO ASK! if Withdrawal dispute is not possible, this could be omitted, but in that case restore from db by transaction replay would not be possible, so I leave it here...
}

//Hopefully this fits in memory, but persistent storage would be better (Vec would use less memory, but slow, allocated in one large block)
pub struct InMemoryLedger {
    db: HashMap<TransactionId, TransactionState>,
}

impl InMemoryLedger {
    pub fn new() -> Self {
        Self {
            db: HashMap::<TransactionId, TransactionState>::new(),
        }
    }
}

impl Ledger for InMemoryLedger {
    type Error = ();
    type Key = TransactionId;
    type Value = TransactionState;

    fn contains(&self, key: Self::Key) -> Result<bool, Self::Error> {
        //real db could return Err<DbError>
        Ok(self.db.contains_key(&key))
    }

    fn get(&self, key: Self::Key) -> Result<Option<TransactionState>, Self::Error> {
        //real db could return Err<DbError>
        Ok(self.db.get(&key).map(|v| *v))
    }

    /// must always check if returned with success!
    /// (a real db could return Err<DbError>)
    #[must_use]
    fn insert(&mut self, key: Self::Key, state: TransactionState) -> Result<(), Self::Error> {
        self.db.insert(key, state);
        Ok(())
    }
}
