//! In this module we design another multi-user currency system. This one is not based on
//! accounts, but rather, is modelled after a paper cash system. The system tracks individual
//! cash bills. Each bill has an amount and an owner, and can be spent in its entirety.
//! When a state transition spends bills, new bills are created in lesser or equal amount.

use super::{StateMachine, User};
use std::collections::HashSet;

/// This state machine models a multi-user currency system. It tracks a set of bills in
/// circulation, and updates that set when money is transferred.
pub struct DigitalCashSystem;

/// A single bill in the digital cash system. Each bill has an owner who is allowed to spent
/// it and an amount that it is worth. It also has serial number to ensure that each bill
/// is unique.
#[derive(Eq, PartialEq, Clone, Debug, Hash)]
pub struct Bill {
    owner: User,
    amount: u64,
    serial: u64,
}

/// The State of a digital cash system. Primarily just the set of currently circulating bills.,
/// but also a counter for the next serial number.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct State {

    /// The set of currently circulating bills
    bills: HashSet<Bill>,
    /// The next serial number to use when a bill is created.
    next_serial: u64,
}

impl State {
    pub fn new() -> Self {
        State {
            bills: HashSet::<Bill>::new(),
            next_serial: 0,
        }
    }

    pub fn set_serial(&mut self, serial: u64) {
        self.next_serial = serial;
    }

    pub fn next_serial(&self) -> u64 {
        self.next_serial
    }

    fn increment_serial(&mut self) {
        self.next_serial += 1
    }

    fn add_bill(&mut self, elem: Bill) {
        self.bills.insert(elem);
        self.increment_serial()
    }
}

impl FromIterator<Bill> for State {
    fn from_iter<I: IntoIterator<Item = Bill>>(iter: I) -> Self {
        let mut state = State::new();

        for i in iter {
            state.add_bill(i)
        }
        state
    }
}

impl<const N: usize> From<[Bill; N]> for State {
    fn from(value: [Bill; N]) -> Self {
        State::from_iter(value)
    }
}

/// The state transitions that users can make in a digital cash system
pub enum CashTransaction {
    /// Mint a single new bill owned by the minter
    Mint { minter: User, amount: u64 },
    /// Send some money from some users to other users. The money does not all need
    /// to come from the same user, and it does not all need to go to the same user.
    /// The total amount received must be less than or equal to the amount spent.
    /// The discrepancy between the amount sent and received is destroyed. Therefore,
    /// no dedicated burn transaction is required.
    Transfer {
        spends: Vec<Bill>,
        receives: Vec<Bill>,
    },
}

/// We model this system as a state machine with two possible transitions
impl StateMachine for DigitalCashSystem {
    type State = State;
    type Transition = CashTransaction;

    fn next_state(starting_state: &Self::State, t: &Self::Transition) -> Self::State {
        let mut state = starting_state.clone();

        match t {
            CashTransaction::Mint { minter, amount } => {
                state.add_bill(Bill {
                    owner: *minter,
                    amount: *amount,
                    serial: state.next_serial(),
                });
            }
            CashTransaction::Transfer { spends, receives } => {
                let mut new_bills = HashSet::<Bill>::new();
                let mut total_spent = 0;
                let mut total_received = 0;

                let spends_serials: HashSet<_> = spends.iter().map(|bill| bill.serial).collect();
                if spends_serials.len() != spends.len() {
                    return state;
                }
                // check for receives
                let receives_serials: HashSet<_> = receives.iter().map(|bill| bill.serial).collect();
                if receives_serials.len() != receives.len() {
                    return state;
                }


                // check if spends are valid
                if spends.iter().any(|b| b.amount == 0) {
                    return state;
                }

                // check if bills are the same
                for bs in spends.iter() {
                    for br in receives.iter() {
                        if bs.serial == br.serial {
                            return state;
                        }
                    }
                }

                for bill in spends {
                    if !state.bills.contains(bill) {
                        return state;
                    }
                    total_spent += bill.amount;
                }

                for bill in receives {
                    if bill.amount == 0 {
                        return state;
                    }
                    if state.bills.contains(bill) {
                        return state;
                    }
                    if bill.serial == u64::MAX {
                        return state;
                    }
                    if (total_received + bill.amount) > total_spent {
                        return state;
                    }
                    total_received += bill.amount;
                }
                if total_spent < total_received {
                    return state;
                }
                for bill in spends {
                    state.bills.remove(bill);
                }

                for bill in receives {
                    new_bills.insert(bill.clone());
                }

                for bill in new_bills {
                    state.add_bill(bill);
                }
            }
        }

        state
    }
}

#[test]
fn sm_5_mint_new_cash() {
    let start = State::new();
    let end = DigitalCashSystem::next_state(
        &start,
        &CashTransaction::Mint {
            minter: User::Alice,
            amount: 20,
        },
    );

    let expected = State::from([Bill {
        owner: User::Alice,
        amount: 20,
        serial: 0,
    }]);
    assert_eq!(end, expected);
}

#[test]
fn sm_5_overflow_receives_fails() {
    let start = State::from([Bill {
        owner: User::Alice,
        amount: 42,
        serial: 0,
    }]);
    let end = DigitalCashSystem::next_state(
        &start,
        &CashTransaction::Transfer {
            spends: vec![Bill {
                owner: User::Alice,
                amount: 42,
                serial: 0,
            }],
            receives: vec![
                Bill {
                    owner: User::Alice,
                    amount: u64::MAX,
                    serial: 1,
                },
                Bill {
                    owner: User::Alice,
                    amount: 42,
                    serial: 2,
                },
            ],
        },
    );
    let expected = State::from([Bill {
        owner: User::Alice,
        amount: 42,
        serial: 0,
    }]);
    assert_eq!(end, expected);
}

#[test]
fn sm_5_empty_spend_fails() {
    let start = State::from([Bill {
        owner: User::Alice,
        amount: 20,
        serial: 0,
    }]);
    let end = DigitalCashSystem::next_state(
        &start,
        &CashTransaction::Transfer {
            spends: vec![],
            receives: vec![Bill {
                owner: User::Alice,
                amount: 15,
                serial: 1,
            }],
        },
    );
    let expected = State::from([Bill {
        owner: User::Alice,
        amount: 20,
        serial: 0,
    }]);
    assert_eq!(end, expected);
}

#[test]
fn sm_5_empty_receive_fails() {
    let start = State::from([Bill {
        owner: User::Alice,
        amount: 20,
        serial: 0,
    }]);
    let end = DigitalCashSystem::next_state(
        &start,
        &CashTransaction::Transfer {
            spends: vec![Bill {
                owner: User::Alice,
                amount: 20,
                serial: 0,
            }],
            receives: vec![],
        },
    );
    let mut expected = State::from([]);
    expected.set_serial(1);
    assert_eq!(end, expected);
}

#[test]
fn sm_5_output_value_0_fails() {
    let start = State::from([Bill {
        owner: User::Alice,
        amount: 20,
        serial: 0,
    }]);
    let end = DigitalCashSystem::next_state(
        &start,
        &CashTransaction::Transfer {
            spends: vec![Bill {
                owner: User::Alice,
                amount: 20,
                serial: 0,
            }],
            receives: vec![Bill {
                owner: User::Bob,
                amount: 0,
                serial: 1,
            }],
        },
    );
    let expected = State::from([Bill {
        owner: User::Alice,
        amount: 20,
        serial: 0,
    }]);
    assert_eq!(end, expected);
}

#[test]
fn sm_5_serial_number_already_seen_fails() {
    let start = State::from([Bill {
        owner: User::Alice,
        amount: 20,
        serial: 0,
    }]);
    let end = DigitalCashSystem::next_state(
        &start,
        &CashTransaction::Transfer {
            spends: vec![Bill {
                owner: User::Alice,
                amount: 20,
                serial: 0,
            }],
            receives: vec![Bill {
                owner: User::Alice,
                amount: 18,
                serial: 0,
            }],
        },
    );
    let expected = State::from([Bill {
        owner: User::Alice,
        amount: 20,
        serial: 0,
    }]);
    assert_eq!(end, expected);
}

#[test]
fn sm_5_spending_and_receiving_same_bill_fails() {
    let start = State::from([Bill {
        owner: User::Alice,
        amount: 20,
        serial: 0,
    }]);
    let end = DigitalCashSystem::next_state(
        &start,
        &CashTransaction::Transfer {
            spends: vec![Bill {
                owner: User::Alice,
                amount: 20,
                serial: 0,
            }],
            receives: vec![Bill {
                owner: User::Alice,
                amount: 20,
                serial: 0,
            }],
        },
    );
    let expected = State::from([Bill {
        owner: User::Alice,
        amount: 20,
        serial: 0,
    }]);
    assert_eq!(end, expected);
}

#[test]
fn sm_5_receiving_bill_with_incorrect_serial_fails() {
    let start = State::from([Bill {
        owner: User::Alice,
        amount: 20,
        serial: 0,
    }]);
    let end = DigitalCashSystem::next_state(
        &start,
        &CashTransaction::Transfer {
            spends: vec![Bill {
                owner: User::Alice,
                amount: 20,
                serial: 0,
            }],
            receives: vec![
                Bill {
                    owner: User::Alice,
                    amount: 10,
                    serial: u64::MAX,
                },
                Bill {
                    owner: User::Bob,
                    amount: 10,
                    serial: 4000,
                },
            ],
        },
    );
    let expected = State::from([Bill {
        owner: User::Alice,
        amount: 20,
        serial: 0,
    }]);
    assert_eq!(end, expected);
}

#[test]
fn sm_5_spending_bill_with_incorrect_amount_fails() {
    let start = State::from([Bill {
        owner: User::Alice,
        amount: 20,
        serial: 0,
    }]);
    let end = DigitalCashSystem::next_state(
        &start,
        &CashTransaction::Transfer {
            spends: vec![Bill {
                owner: User::Alice,
                amount: 40,
                serial: 0,
            }],
            receives: vec![Bill {
                owner: User::Bob,
                amount: 40,
                serial: 1,
            }],
        },
    );
    let expected = State::from([Bill {
        owner: User::Alice,
        amount: 20,
        serial: 0,
    }]);
    assert_eq!(end, expected);
}

#[test]
fn sm_5_spending_same_bill_fails() {
    let start = State::from([Bill {
        owner: User::Alice,
        amount: 40,
        serial: 0,
    }]);
    let end = DigitalCashSystem::next_state(
        &start,
        &CashTransaction::Transfer {
            spends: vec![
                Bill {
                    owner: User::Alice,
                    amount: 40,
                    serial: 0,
                },
                Bill {
                    owner: User::Alice,
                    amount: 40,
                    serial: 0,
                },
            ],
            receives: vec![
                Bill {
                    owner: User::Bob,
                    amount: 20,
                    serial: 1,
                },
                Bill {
                    owner: User::Bob,
                    amount: 20,
                    serial: 2,
                },
                Bill {
                    owner: User::Alice,
                    amount: 40,
                    serial: 3,
                },
            ],
        },
    );
    let expected = State::from([Bill {
        owner: User::Alice,
        amount: 40,
        serial: 0,
    }]);
    assert_eq!(end, expected);
}

#[test]
fn sm_5_spending_more_than_bill_fails() {
    let start = State::from([
        Bill {
            owner: User::Alice,
            amount: 40,
            serial: 0,
        },
        Bill {
            owner: User::Charlie,
            amount: 42,
            serial: 1,
        },
    ]);
    let end = DigitalCashSystem::next_state(
        &start,
        &CashTransaction::Transfer {
            spends: vec![
                Bill {
                    owner: User::Alice,
                    amount: 40,
                    serial: 0,
                },
                Bill {
                    owner: User::Charlie,
                    amount: 42,
                    serial: 1,
                },
            ],
            receives: vec![
                Bill {
                    owner: User::Bob,
                    amount: 20,
                    serial: 2,
                },
                Bill {
                    owner: User::Bob,
                    amount: 20,
                    serial: 3,
                },
                Bill {
                    owner: User::Alice,
                    amount: 52,
                    serial: 4,
                },
            ],
        },
    );
    let expected = State::from([
        Bill {
            owner: User::Alice,
            amount: 40,
            serial: 0,
        },
        Bill {
            owner: User::Charlie,
            amount: 42,
            serial: 1,
        },
    ]);
    assert_eq!(end, expected);
}

#[test]
fn sm_5_spending_non_existent_bill_fails() {
    let start = State::from([Bill {
        owner: User::Alice,
        amount: 32,
        serial: 0,
    }]);
    let end = DigitalCashSystem::next_state(
        &start,
        &CashTransaction::Transfer {
            spends: vec![Bill {
                owner: User::Bob,
                amount: 1000,
                serial: 32,
            }],
            receives: vec![Bill {
                owner: User::Bob,
                amount: 1000,
                serial: 33,
            }],
        },
    );
    let expected = State::from([Bill {
        owner: User::Alice,
        amount: 32,
        serial: 0,
    }]);
    assert_eq!(end, expected);
}

#[test]
fn sm_5_spending_from_alice_to_all() {
    let start = State::from([Bill {
        owner: User::Alice,
        amount: 42,
        serial: 0,
    }]);
    let end = DigitalCashSystem::next_state(
        &start,
        &CashTransaction::Transfer {
            spends: vec![Bill {
                owner: User::Alice,
                amount: 42,
                serial: 0,
            }],
            receives: vec![
                Bill {
                    owner: User::Alice,
                    amount: 10,
                    serial: 1,
                },
                Bill {
                    owner: User::Bob,
                    amount: 10,
                    serial: 2,
                },
                Bill {
                    owner: User::Charlie,
                    amount: 10,
                    serial: 3,
                },
            ],
        },
    );
    let mut expected = State::from([
        Bill {
            owner: User::Alice,
            amount: 10,
            serial: 1,
        },
        Bill {
            owner: User::Bob,
            amount: 10,
            serial: 2,
        },
        Bill {
            owner: User::Charlie,
            amount: 10,
            serial: 3,
        },
    ]);
    expected.set_serial(4);
    assert_eq!(end, expected);
}

#[test]
fn sm_5_spending_from_bob_to_all() {
    let start = State::from([Bill {
        owner: User::Bob,
        amount: 42,
        serial: 0,
    }]);
    let end = DigitalCashSystem::next_state(
        &start,
        &CashTransaction::Transfer {
            spends: vec![Bill {
                owner: User::Bob,
                amount: 42,
                serial: 0,
            }],
            receives: vec![
                Bill {
                    owner: User::Alice,
                    amount: 10,
                    serial: 1,
                },
                Bill {
                    owner: User::Bob,
                    amount: 10,
                    serial: 2,
                },
                Bill {
                    owner: User::Charlie,
                    amount: 22,
                    serial: 3,
                },
            ],
        },
    );
    let mut expected = State::from([
        Bill {
            owner: User::Alice,
            amount: 10,
            serial: 1,
        },
        Bill {
            owner: User::Bob,
            amount: 10,
            serial: 2,
        },
        Bill {
            owner: User::Charlie,
            amount: 22,
            serial: 3,
        },
    ]);
    expected.set_serial(4);
    assert_eq!(end, expected);
}

#[test]
fn sm_5_spending_from_charlie_to_all() {
    let mut start = State::from([
        Bill {
            owner: User::Charlie,
            amount: 68,
            serial: 54,
        },
        Bill {
            owner: User::Alice,
            amount: 4000,
            serial: 58,
        },
    ]);
    start.set_serial(59);
    let end = DigitalCashSystem::next_state(
        &start,
        &CashTransaction::Transfer {
            spends: vec![Bill {
                owner: User::Charlie,
                amount: 68,
                serial: 54,
            }],
            receives: vec![
                Bill {
                    owner: User::Alice,
                    amount: 42,
                    serial: 59,
                },
                Bill {
                    owner: User::Bob,
                    amount: 5,
                    serial: 60,
                },
                Bill {
                    owner: User::Charlie,
                    amount: 5,
                    serial: 61,
                },
            ],
        },
    );
    let mut expected = State::from([
        Bill {
            owner: User::Alice,
            amount: 4000,
            serial: 58,
        },
        Bill {
            owner: User::Alice,
            amount: 42,
            serial: 59,
        },
        Bill {
            owner: User::Bob,
            amount: 5,
            serial: 60,
        },
        Bill {
            owner: User::Charlie,
            amount: 5,
            serial: 61,
        },
    ]);
    expected.set_serial(62);
    assert_eq!(end, expected);
}
