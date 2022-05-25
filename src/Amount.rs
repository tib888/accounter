/// Amount is a new type which represent funds.
/// Any arithmetics with it must be carefully thought so the usual operators are not implemented, just checked ones
/// Amount chosen not to be 'Decimal' based on the assumption that no more than 2^63/10000-1 units expected per transaction (or even in one account balance)
/// + it is faster, more efficient to use fixed point calculations, than to work on decimals
use rust_decimal::{prelude::ToPrimitive, Decimal};
use std::fmt::Display;
pub use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]

pub struct Amount(i64);

impl Amount {
    const FRACT: i64 = 10_000;
    const FRACT_DEC: Decimal = Decimal::from_parts(10_000, 0, 0, false, 0);

    pub const MAX: Amount = Amount(i64::MAX);
    pub const MIN: Amount = Amount(i64::MIN);
    pub const ZERO: Amount = Amount(0);
    pub const ONE: Amount = Amount(Amount::FRACT);
    pub const MINUS_ONE: Amount = Amount(-Amount::FRACT);

    /// returns None in cases when of overflow would happen!
    pub fn checked_add(lhs: Amount, rhs: Amount) -> Option<Amount> {
        lhs.0.checked_add(rhs.0).map(|val| Amount(val))
    }

    /// returns None in cases when of overflow would happen!
    pub fn checked_sub(lhs: Amount, rhs: Amount) -> Option<Amount> {
        lhs.0.checked_sub(rhs.0).map(|val| Amount(val))
    }
}

impl Display for Amount {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let wholes = self.0 / Amount::FRACT;
        write!(f, "{wholes}").and_then(|()| {
            let fraction = (self.0 % Amount::FRACT).abs() + Amount::FRACT;
            if fraction != Amount::FRACT {
                let s = format!("{fraction}");
                write!(f, ".{}", &s.trim_end_matches('0')[1..])
            } else {
                Ok(())
            }
        })
    }
}

#[derive(Debug, PartialEq)]
pub struct ParseError;

impl FromStr for Amount {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(decimal) = Decimal::from_str(s) {
            (decimal * Amount::FRACT_DEC)
                .to_i64()
                .map(|int| Amount(int))
                .ok_or(ParseError)
        } else {
            Err(ParseError)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants() {
        assert_eq!(Amount::ZERO.0, 0);
        assert_eq!(Amount::MAX.0, 9223372036854775807);
        assert_eq!(Amount::MIN.0, -9223372036854775808);
    }
    #[test]
    fn from_string() {
        assert!(Amount::from_str("").is_err());
        assert!(Amount::from_str(" ").is_err());
        assert!(Amount::from_str(".").is_err());
        assert!(Amount::from_str(" .").is_err());
        assert!(Amount::from_str(". ").is_err());
        assert!(Amount::from_str(" . ").is_err());
        assert!(Amount::from_str("a").is_err());
        assert!(Amount::from_str(".a").is_err());
        assert!(Amount::from_str("a.a").is_err());
        assert!(Amount::from_str("0. 0").is_err());
        assert!(Amount::from_str("0 .0").is_err());
        assert!(Amount::from_str(" 0.0").is_err());
        assert!(Amount::from_str("0.0 ").is_err());
        assert!(Amount::from_str(" 0.0 ").is_err());
        assert!(Amount::from_str("+ 1.0").is_err());
        assert!(Amount::from_str("- 1.0").is_err());
        assert_eq!(Amount::from_str("0"), Ok(Amount::ZERO));
        assert_eq!(Amount::from_str(".0"), Ok(Amount::ZERO));
        assert_eq!(Amount::from_str("0."), Ok(Amount::ZERO));
        assert_eq!(Amount::from_str("0.0"), Ok(Amount::ZERO));
        assert_eq!(Amount::from_str("1.0"), Ok(Amount(Amount::FRACT)));
        assert_eq!(Amount::from_str("+1.0"), Ok(Amount(Amount::FRACT)));
        assert_eq!(Amount::from_str("-1.0"), Ok(Amount(-Amount::FRACT)));
        assert_eq!(Amount::from_str("922337203685477.5807"), Ok(Amount::MAX));
        assert_eq!(Amount::from_str("+922337203685477.5807"), Ok(Amount::MAX));
        assert_eq!(Amount::from_str("-922337203685477.5808"), Ok(Amount::MIN));
    }

    #[test]
    fn display() {
        assert_eq!(
            format!("{}", Amount::from_str("-922337203685477.5808").unwrap()),
            "-922337203685477.5808"
        );
        assert_eq!(
            format!("{}", Amount::from_str("922337203685477.5807").unwrap()),
            "922337203685477.5807"
        );
        assert_eq!(format!("{}", Amount::from_str("0.0").unwrap()), "0");
        assert_eq!(format!("{}", Amount::from_str("1.1").unwrap()), "1.1");
        assert_eq!(format!("{}", Amount::from_str("1.01").unwrap()), "1.01");
        assert_eq!(format!("{}", Amount::from_str("1.001").unwrap()), "1.001");
        assert_eq!(format!("{}", Amount::from_str("1.0001").unwrap()), "1.0001");
        assert_eq!(format!("{}", Amount::from_str("1.00001").unwrap()), "1");
        assert_eq!(
            format!("{}", Amount::from_str("1.00011").unwrap()),
            "1.0001"
        );
        assert_eq!(
            format!("{}", Amount::from_str("-1.0001").unwrap()),
            "-1.0001"
        );
        assert_eq!(
            format!("{}", Amount::from_str("-1.00011").unwrap()),
            "-1.0001"
        );
    }

    #[test]
    fn adding() {
        assert_eq!(
            Amount::checked_add(
                Amount::from_str("0").unwrap(),
                Amount::from_str("0").unwrap()
            )
            .unwrap(),
            Amount::from_str("0").unwrap()
        );
        assert_eq!(
            Amount::checked_add(
                Amount::from_str("1").unwrap(),
                Amount::from_str("0").unwrap()
            )
            .unwrap(),
            Amount::from_str("1").unwrap()
        );
        assert_eq!(
            Amount::checked_add(
                Amount::from_str("0").unwrap(),
                Amount::from_str("1").unwrap()
            )
            .unwrap(),
            Amount::from_str("1").unwrap()
        );
        assert_eq!(
            Amount::checked_add(
                Amount::from_str("1").unwrap(),
                Amount::from_str("1").unwrap()
            )
            .unwrap(),
            Amount::from_str("2").unwrap()
        );
        assert_eq!(
            Amount::checked_add(
                Amount::from_str("56.1234").unwrap(),
                Amount::from_str("78.1234").unwrap()
            )
            .unwrap(),
            Amount::from_str("134.2468").unwrap()
        );
        assert_eq!(
            Amount::checked_add(
                Amount::from_str("56.12349").unwrap(),
                Amount::from_str("78.12349").unwrap()
            )
            .unwrap(),
            Amount::from_str("134.2468").unwrap()
        );

        assert_eq!(
            Amount::checked_add(Amount::MAX, Amount::from_str("0.0001").unwrap()),
            None
        ); //overflow
    }

    #[test]
    fn subtracting() {
        assert_eq!(
            Amount::checked_sub(
                Amount::from_str("0").unwrap(),
                Amount::from_str("0").unwrap()
            )
            .unwrap(),
            Amount::from_str("0").unwrap()
        );
        assert_eq!(
            Amount::checked_sub(
                Amount::from_str("1").unwrap(),
                Amount::from_str("0").unwrap()
            )
            .unwrap(),
            Amount::from_str("1").unwrap()
        );
        assert_eq!(
            Amount::checked_sub(
                Amount::from_str("0").unwrap(),
                Amount::from_str("1").unwrap()
            ),
            Some(Amount::from_str("-1").unwrap())
        );
        assert_eq!(
            Amount::checked_sub(
                Amount::from_str("1").unwrap(),
                Amount::from_str("1").unwrap()
            ),
            Some(Amount::from_str("0").unwrap())
        );
        assert_eq!(
            Amount::checked_sub(
                Amount::from_str("78.1234").unwrap(),
                Amount::from_str("56.1234").unwrap()
            )
            .unwrap(),
            Amount::from_str("22").unwrap()
        );
        assert_eq!(
            Amount::checked_sub(
                Amount::from_str("78.12345").unwrap(),
                Amount::from_str("56.12343").unwrap()
            )
            .unwrap(),
            Amount::from_str("22").unwrap()
        );

        assert_eq!(
            Amount::checked_sub(Amount::MIN, Amount::from_str("0.0001").unwrap()),
            None
        ); //overflow

        assert_eq!(Amount::checked_sub(Amount::MAX, Amount::MIN,), None); //overflow
    }

    #[test]
    fn compare() {
        assert_eq!(
            Amount::from_str("0").unwrap() > Amount::from_str("0").unwrap(),
            false
        );
        assert_eq!(
            Amount::from_str("-1").unwrap() > Amount::from_str("1").unwrap(),
            false
        );
        assert_eq!(
            Amount::from_str("1").unwrap() < Amount::from_str("1").unwrap(),
            false
        );
        assert_eq!(
            Amount::from_str("-1").unwrap() < Amount::from_str("1").unwrap(),
            true
        );

        assert_eq!(
            Amount::from_str("2.5").unwrap() == Amount::from_str("2.5").unwrap(),
            true
        );
        assert_eq!(
            Amount::from_str("1").unwrap() == Amount::from_str("-1").unwrap(),
            false
        );

        assert_eq!(
            Amount::from_str("2.50001").unwrap() == Amount::from_str("2.50003").unwrap(),
            true
        );
        assert_eq!(
            Amount::from_str("2.5001").unwrap() == Amount::from_str("2.5003").unwrap(),
            false
        );

        assert_eq!(
            Amount::from_str("0.1").unwrap() > Amount::from_str("1.1").unwrap(),
            false
        );
        assert_eq!(
            Amount::from_str("1.5565").unwrap() < Amount::from_str("1.5566").unwrap(),
            true
        );
        assert_eq!(
            Amount::from_str("0.1").unwrap() < Amount::from_str("1.1").unwrap(),
            true
        );
        assert_eq!(
            Amount::from_str("1.5565").unwrap() > Amount::from_str("1.5566").unwrap(),
            false
        );
    }
}
