use std::error::Error;
use std::fmt;

use crate::actions::*;
use crate::amount::*;
use crate::ledger::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransactionError {
    AccountLocked,          //try to access locked account
    InvalidAmount,          //zero or negative transaction amount
    WouldOverFlow,          //can not book that much amount
    DisputeNotOpenedYet,    //resolve/charge back needs open dispute first
    AlreadyInDispute,       //a dispute already opened with the given transaction id
    AlreadyChargedBack,     //already charged back
    InvalidTransactionId,   //there is no such transaction in the ledger
    InvalidTransactionType, //based on assumption that withdrawals can not be disputed
    RepeatedTransactionId, //this check is theoretically not needed (unique TransactionIds guaranteed in specification)
    DbError,               //a ledger real DB would have possible access errors
    Unexpected,            //this should have never happened
}

impl fmt::Display for TransactionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let description = match self {
            TransactionError::AccountLocked => "try to access locked account",
            TransactionError::InvalidAmount => "zero or negative transaction amount",
            TransactionError::WouldOverFlow => "can not book that much amount",
            TransactionError::DisputeNotOpenedYet => "resolve/charge back needs open dispute first",
            TransactionError::AlreadyInDispute => "a dispute already opened with the given transaction id",
            TransactionError::AlreadyChargedBack => "already charged back",
            TransactionError::InvalidTransactionId => "there is no such transaction in the ledger",
            TransactionError::InvalidTransactionType => "based on assumption that withdrawals can not be disputed",
            TransactionError::RepeatedTransactionId => "this check is theoretically not needed (unique TransactionIds guaranteed in specification)",
            TransactionError::DbError => "a ledger real DB would have possible access errors",
            TransactionError::Unexpected => "this should have never happened",
        };
        write!(f, "{:?} ({description})", self)
    }
}

impl Error for TransactionError {}

pub struct Account {
    total: Amount,
    held: Amount,
    locked: bool,
    ledger: Box<dyn Ledger<Error = (), Key = TransactionId, Value = TransactionState>>,
}

impl Account {
    pub fn new(
        ledger: Box<dyn Ledger<Error = (), Key = TransactionId, Value = TransactionState>>,
    ) -> Self {
        Account {
            total: Amount::ZERO,
            held: Amount::ZERO,
            locked: false,
            ledger: ledger,
        }
    }

    /// The total funds that are available for trading (can be negative due to charge backs!)
    pub fn available(&self) -> Amount {
        Amount::checked_sub(self.total, self.held).unwrap_or(Amount::ZERO)
    }

    /// The total funds that are held for dispute (can not be negative, if everything works fine!)
    pub fn held(&self) -> Amount {
        self.held
    }

    /// The total funds that are available or held (can be negative due to charge backs!)
    pub fn total(&self) -> Amount {
        self.total
    }

    /// Whether the account is locked (due to a charge back)
    pub fn is_locked(&self) -> bool {
        self.locked
    }

    /// Deposit/Withdraw funds to/from the account
    /// REQUIRES: unique TransactionIds (guaranteed in specification)
    async fn transact(&mut self, data: TransactionData) -> Result<(), TransactionError> {
        if self.is_locked() {
            return Err(TransactionError::AccountLocked); //TODO ASK! should we allow deposit in this case?
        }
        match self.ledger.contains(data.id).await //this check is theoretically not needed (unique TransactionIds guaranteed in specification)
        {
            Ok(true) => { return Err(TransactionError::RepeatedTransactionId); }
            Err(_) => { return Err(TransactionError::DbError) }
            _ => {}
        }

        match data.transaction {
            Transaction::Deposit(amount) => {
                if amount <= Amount::ZERO {
                    return Err(TransactionError::InvalidAmount);
                }
                if let Some(new_total) = Amount::checked_add(self.total, amount) {
                    self.ledger
                        .insert(data.id, TransactionState::Deposit(amount))
                        .await
                        .and_then(|_| {
                            self.total = new_total;
                            Ok(()) //return success only if the ledger logged the transaction and everything was perfect!
                        })
                        .map_err(|_| TransactionError::DbError)
                } else {
                    Err(TransactionError::WouldOverFlow)
                }
            }
            Transaction::Withdrawal(amount) => {
                if amount <= Amount::ZERO || self.available() < amount {
                    return Err(TransactionError::InvalidAmount); //* this case triggers the need for the ordered processing of transactions!
                }
                if let Some(new_total) = Amount::checked_sub(self.total, amount) {
                    self.ledger
                        .insert(data.id, TransactionState::Withdrawal(amount))
                        .await
                        .and_then(|_| {
                            self.total = new_total;
                            Ok(()) //return success only if the ledger logged the transaction and everything was perfect!
                        })
                        .map_err(|_| TransactionError::DbError)
                } else {
                    //we should never get here
                    Err(TransactionError::Unexpected)
                }
            }
        }
    }

    /// dispute represents a client's claim that a transaction was erroneous and
    /// should be reversed. The funds associated with this transaction should be
    /// held back from usage until the dispute resolution/charge back
    async fn start_dispute(&mut self, id: TransactionId) -> Result<(), TransactionError> {
        match self.ledger.get(id).await {
            Err(_) => Err(TransactionError::DbError),
            Ok(None) => Err(TransactionError::InvalidTransactionId),
            Ok(Some(state)) => match state {
                TransactionState::ChargedBack(_) => Err(TransactionError::AlreadyChargedBack),
                TransactionState::DepositInDispute(_) => Err(TransactionError::AlreadyInDispute),
                TransactionState::Withdrawal(_) => Err(TransactionError::InvalidTransactionType),
                TransactionState::Deposit(amount) => {
                    if let Some(new_held) = Amount::checked_add(self.held, amount) {
                        self.ledger
                            .insert(id, TransactionState::DepositInDispute(amount))
                            .await
                            .and_then(|_| {
                                self.held = new_held;
                                Ok(())
                            })
                            .map_err(|_| TransactionError::DbError)
                    } else {
                        Err(TransactionError::WouldOverFlow)
                    }
                }
            },
        }
    }

    /// A resolve represents a resolution to a dispute, releasing the associated held funds
    async fn resolve_dispute(&mut self, id: TransactionId) -> Result<(), TransactionError> {
        //only open disputes can be resolved!
        match self.ledger.get(id).await {
            Err(_) => Err(TransactionError::DbError),
            Ok(None) => Err(TransactionError::InvalidTransactionId),
            Ok(Some(state)) => match state {
                TransactionState::ChargedBack(_) => Err(TransactionError::AlreadyChargedBack),
                TransactionState::Withdrawal(_) => Err(TransactionError::DisputeNotOpenedYet),
                TransactionState::Deposit(_) => Err(TransactionError::DisputeNotOpenedYet),
                TransactionState::DepositInDispute(amount) => {
                    if let Some(new_held) = Amount::checked_sub(self.held, amount) {
                        self.ledger
                            .insert(id, TransactionState::Deposit(amount))
                            .await
                            .and_then(|_| {
                                self.held = new_held;
                                Ok(())
                            })
                            .map_err(|_| TransactionError::DbError)
                    } else {
                        Err(TransactionError::Unexpected)
                    }
                }
            },
        }
    }

    /// A charge back means: the client reversing a transaction
    /// If a charge back occurs, the account is immediately frozen
    /// NOTE: if the amount of transaction is greater than the total,
    /// total will be zeroed, and the missing amount will stay in held
    /// (based on these negative available amount will be returned in Err)
    async fn resolve_dispute_with_charge_back(
        &mut self,
        id: TransactionId,
    ) -> Result<(), TransactionError> {
        //protect against repeated charge backs:
        match self.ledger.get(id).await {
            Err(_) => Err(TransactionError::DbError),
            Ok(None) => Err(TransactionError::InvalidTransactionId),
            Ok(Some(state)) => match state {
                TransactionState::ChargedBack(_) => Err(TransactionError::AlreadyChargedBack),
                TransactionState::Withdrawal(_) => Err(TransactionError::DisputeNotOpenedYet),
                TransactionState::Deposit(_) => Err(TransactionError::DisputeNotOpenedYet),
                TransactionState::DepositInDispute(amount) => {
                    if let (Some(new_held), Some(new_total)) = (
                        Amount::checked_sub(self.held, amount),
                        Amount::checked_sub(self.total, amount),
                    ) {
                        self.ledger
                            .insert(id, TransactionState::ChargedBack(amount))
                            .await
                            .and_then(|_| {
                                self.locked = true;
                                self.total = new_total;
                                self.held = new_held;
                                Ok(())
                            })
                            .map_err(|_| TransactionError::DbError)
                    } else {
                        Err(TransactionError::Unexpected)
                    }
                }
            },
        }
    }

    /// The execution order of the transactions must be kept
    /// (Out of order transaction processing must NOT be used!)
    /// Concurrent transaction processing is also forbidden!
    pub async fn execute(&mut self, action: Action) -> Result<(), TransactionError> {
        match action {
            Action::Transact(data) => self.transact(data).await,
            Action::Dispute(id) => self.start_dispute(id).await,
            Action::Resolve(id) => self.resolve_dispute(id).await,
            Action::ChargeBack(id) => self.resolve_dispute_with_charge_back(id).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    async fn deposit(
        account: &mut Account,
        id: u32,
        amount: &str,
        expected: Result<(), TransactionError>,
    ) {
        assert_eq!(
            account
                .execute(Action::Transact(TransactionData {
                    id: TransactionId::from(id),
                    transaction: Transaction::Deposit(Amount::from_str(amount).unwrap())
                }))
                .await,
            expected
        );
    }

    async fn withdraw(
        account: &mut Account,
        id: u32,
        amount: &str,
        expected: Result<(), TransactionError>,
    ) {
        assert_eq!(
            account
                .execute(Action::Transact(TransactionData {
                    id: TransactionId::from(id),
                    transaction: Transaction::Withdrawal(Amount::from_str(amount).unwrap())
                }))
                .await,
            expected
        );
    }

    async fn dispute(account: &mut Account, id: u32, expected: Result<(), TransactionError>) {
        assert_eq!(
            account
                .execute(Action::Dispute(TransactionId::from(id)))
                .await,
            expected
        );
    }
    async fn resolve(account: &mut Account, id: u32, expected: Result<(), TransactionError>) {
        assert_eq!(
            account
                .execute(Action::Resolve(TransactionId::from(id)))
                .await,
            expected
        );
    }
    async fn charge_back(account: &mut Account, id: u32, expected: Result<(), TransactionError>) {
        assert_eq!(
            account
                .execute(Action::ChargeBack(TransactionId::from(id)))
                .await,
            expected
        );
    }

    fn expect_balance(
        account: &mut Account,
        available: &str,
        total: &str,
        held: &str,
        locked: bool,
    ) {
        assert_eq!(account.available(), Amount::from_str(available).unwrap());
        assert_eq!(account.total(), Amount::from_str(total).unwrap());
        assert_eq!(account.held(), Amount::from_str(held).unwrap());
        assert_eq!(account.is_locked(), locked);
    }

    #[tokio::test]
    async fn starting_from_zero() {
        let account = Account::new(Box::new(InMemoryLedger::new()));
        assert_eq!(account.available(), Amount::ZERO);
        assert_eq!(account.total(), Amount::ZERO);
        assert_eq!(account.held(), Amount::ZERO);
    }

    #[tokio::test]
    async fn deposit_sum_up() {
        let mut account = Account::new(Box::new(InMemoryLedger::new()));
        let amount1 = "1234567890.1234";
        let amount2 = "1.2";
        let amount3 = "1234567891.3234";
        deposit(&mut account, 0, amount1, Ok(())).await;
        deposit(&mut account, 1, "0", Err(TransactionError::InvalidAmount)).await;
        deposit(&mut account, 2, "-1", Err(TransactionError::InvalidAmount)).await;
        expect_balance(&mut account, amount1, amount1, "0", false);
        deposit(&mut account, 3, amount2, Ok(())).await;
        expect_balance(&mut account, amount3, amount3, "0", false);
        deposit(
            &mut account,
            4,
            "0.00001",
            Err(TransactionError::InvalidAmount),
        )
        .await;
        expect_balance(&mut account, amount3, amount3, "0", false);
        deposit(
            &mut account,
            5,
            "922337203685477.5807",
            Err(TransactionError::WouldOverFlow),
        )
        .await;
        expect_balance(&mut account, amount3, amount3, "0", false);
        dispute(&mut account, 6, Err(TransactionError::InvalidTransactionId)).await;
    }

    #[tokio::test]
    async fn withdrawals() {
        let mut account = Account::new(Box::new(InMemoryLedger::new()));
        deposit(&mut account, 1, "0.1", Ok(())).await;
        withdraw(
            &mut account,
            2,
            "-0.0001",
            Err(TransactionError::InvalidAmount),
        )
        .await;
        withdraw(&mut account, 3, "0", Err(TransactionError::InvalidAmount)).await;
        withdraw(&mut account, 4, "1", Err(TransactionError::InvalidAmount)).await;
        expect_balance(&mut account, "0.1", "0.1", "0", false);
        withdraw(&mut account, 5, "0.1", Ok(())).await;
        expect_balance(&mut account, "0", "0", "0", false);

        withdraw(&mut account, 6, "1", Err(TransactionError::InvalidAmount)).await;
        expect_balance(&mut account, "0", "0", "0", false);

        deposit(&mut account, 7, "100", Ok(())).await;
        expect_balance(&mut account, "100", "100", "0", false);

        withdraw(&mut account, 9, "5", Ok(())).await;
        expect_balance(&mut account, "95", "95", "0", false);
        withdraw(&mut account, 10, "99", Err(TransactionError::InvalidAmount)).await;
        expect_balance(&mut account, "95", "95", "0", false);

        deposit(&mut account, 11, "200.124", Ok(())).await;
        expect_balance(&mut account, "295.124", "295.124", "0", false);
    }

    #[tokio::test]
    async fn disputes() {
        let mut account = Account::new(Box::new(InMemoryLedger::new()));
        withdraw(&mut account, 1, "0", Err(TransactionError::InvalidAmount)).await;
        withdraw(&mut account, 2, "1", Err(TransactionError::InvalidAmount)).await;

        deposit(&mut account, 3, "100", Ok(())).await;
        withdraw(&mut account, 4, "0", Err(TransactionError::InvalidAmount)).await;
        withdraw(&mut account, 5, "5", Ok(())).await;
        withdraw(&mut account, 6, "99", Err(TransactionError::InvalidAmount)).await;

        deposit(&mut account, 7, "200", Ok(())).await;
        withdraw(&mut account, 8, "290", Ok(())).await;

        deposit(&mut account, 9, "1", Ok(())).await;

        expect_balance(&mut account, "6", "6", "0", false);
        resolve(&mut account, 3, Err(TransactionError::DisputeNotOpenedYet)).await;
        expect_balance(&mut account, "6", "6", "0", false);
        charge_back(&mut account, 3, Err(TransactionError::DisputeNotOpenedYet)).await;
        expect_balance(&mut account, "6", "6", "0", false);
        dispute(&mut account, 9, Ok(())).await; //-1
        expect_balance(&mut account, "5", "6", "1", false);
        dispute(&mut account, 7, Ok(())).await; //-200
        expect_balance(&mut account, "-195", "6", "201", false);
        dispute(&mut account, 9, Err(TransactionError::AlreadyInDispute)).await; //1
        expect_balance(&mut account, "-195", "6", "201", false);
        resolve(&mut account, 7, Ok(())).await; //+200
        expect_balance(&mut account, "5", "6", "1", false);

        charge_back(&mut account, 7, Err(TransactionError::DisputeNotOpenedYet)).await;
        expect_balance(&mut account, "5", "6", "1", false);
        resolve(&mut account, 7, Err(TransactionError::DisputeNotOpenedYet)).await;
        expect_balance(&mut account, "5", "6", "1", false);
        dispute(&mut account, 7, Ok(())).await; //-200
        expect_balance(&mut account, "-195", "6", "201", false);
        charge_back(&mut account, 7, Ok(())).await;
        expect_balance(&mut account, "-195", "-194", "1", true);
        charge_back(&mut account, 7, Err(TransactionError::AlreadyChargedBack)).await;
        expect_balance(&mut account, "-195", "-194", "1", true);
        deposit(
            &mut account,
            11,
            "200",
            Err(TransactionError::AccountLocked),
        )
        .await; //TODO ASK! - I think we should allow this
        expect_balance(&mut account, "-195", "-194", "1", true);
        withdraw(&mut account, 12, "1", Err(TransactionError::AccountLocked)).await;
        expect_balance(&mut account, "-195", "-194", "1", true);
        dispute(&mut account, 7, Err(TransactionError::AlreadyChargedBack)).await; //-200
        expect_balance(&mut account, "-195", "-194", "1", true);
    }

    #[tokio::test]
    async fn disputes2() {
        let mut account = Account::new(Box::new(InMemoryLedger::new()));
        deposit(&mut account, 3, "100", Ok(())).await;
        withdraw(&mut account, 4, "0", Err(TransactionError::InvalidAmount)).await;
        withdraw(&mut account, 5, "5", Ok(())).await;
        withdraw(&mut account, 6, "99", Err(TransactionError::InvalidAmount)).await;

        deposit(&mut account, 7, "200", Ok(())).await;
        withdraw(&mut account, 8, "290", Ok(())).await;

        deposit(
            &mut account,
            8,
            "1",
            Err(TransactionError::RepeatedTransactionId),
        )
        .await;
        deposit(&mut account, 9, "1", Ok(())).await;

        expect_balance(&mut account, "6", "6", "0", false);
        dispute(&mut account, 2, Err(TransactionError::InvalidTransactionId)).await;
        expect_balance(&mut account, "6", "6", "0", false);

        dispute(
            &mut account,
            5,
            Err(TransactionError::InvalidTransactionType),
        )
        .await; //TODO ASK! - Is it possible to dispute a withdrawal?
        expect_balance(&mut account, "6", "6", "0", false);
    }
}
