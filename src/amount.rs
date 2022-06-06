use rust_decimal::{prelude::ToPrimitive, Decimal};
use std::error::Error;
use std::fmt;
use std::fmt::Display;
use std::str::FromStr;

/// Amount is a new type which represent funds.
/// Any arithmetics with it must be carefully thought so the usual operators are not implemented, just checked ones.
///
/// Amount chosen not to be 'Decimal' based on the assumption that no more than 2^63/10000-1 units expected
/// per transaction (or even in one account balance).
/// It is using fixed point arithmetics with 4 digits precision, on a 64bit signed integer
/// this way faster, more memory efficient, than to work on decimals
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Amount(i64);

impl Amount {
    const FRACTION_DIGITS: usize = 4; //number of fractional digits to use
    const FRACTION: i64 = i64::pow(10, Amount::FRACTION_DIGITS as u32); //10^4 = 10_000
    const FRACTION_DEC: Decimal = Decimal::from_parts(Amount::FRACTION as u32, 0, 0, false, 0); //10^4 = 10_000

    pub const MAX: Amount = Amount(i64::MAX);
    pub const MIN: Amount = Amount(i64::MIN);
    pub const ZERO: Amount = Amount(0);
    pub const ONE: Amount = Amount(Amount::FRACTION);
    pub const MINUS_ONE: Amount = Amount(-Amount::FRACTION);

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
        if self.0 == 0 {
            write!(f, "0")
        } else if self.0 >= Amount::FRACTION || self.0 <= -Amount::FRACTION {
            let s = format!("{}", self.0);
            let l = s.len();
            write!(f, "{}", &s[0..l - Amount::FRACTION_DIGITS])?;
            let fraction = &s[l - Amount::FRACTION_DIGITS..l].trim_end_matches('0');
            if fraction.len() > 0 {
                write!(f, ".{}", fraction)
            } else {
                Ok(())
            }
        } else {
            let s = format!("{}", self.0.abs() + Amount::FRACTION);
            let l = s.len();
            if self.0 > 0 {
                write!(f, "0.")?;
            } else {
                write!(f, "-0.")?;
            };
            write!(
                f,
                "{}",
                s[l - Amount::FRACTION_DIGITS..l].trim_end_matches('0')
            )
        }
    }
}

/// Signals that amount parsing from string was not successful
#[derive(Debug, PartialEq, Eq)]
pub struct ParseError;

impl Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "parse error")
    }
}

impl Error for ParseError {}

impl FromStr for Amount {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(decimal) = Decimal::from_str(s) {
            let n = decimal * Amount::FRACTION_DEC;
            if !n.fract().is_zero() {
                return Err(ParseError);
            };
            n.to_i64().map(|int| Amount(int)).ok_or(ParseError)
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
        assert!(Amount::from_str("1.00001").is_err());
        assert!(Amount::from_str("-1.00001").is_err());
        assert_eq!(Amount::from_str("0"), Ok(Amount::ZERO));
        assert_eq!(Amount::from_str(".0"), Ok(Amount::ZERO));
        assert_eq!(Amount::from_str("0."), Ok(Amount::ZERO));
        assert_eq!(Amount::from_str("0.0"), Ok(Amount::ZERO));
        assert_eq!(Amount::from_str("1.0"), Ok(Amount(Amount::FRACTION)));
        assert_eq!(Amount::from_str("+1.0"), Ok(Amount(Amount::FRACTION)));
        assert_eq!(Amount::from_str("-1.0"), Ok(Amount(-Amount::FRACTION)));
        assert_eq!(Amount::from_str("1.00000"), Ok(Amount(Amount::FRACTION)));
        assert_eq!(Amount::from_str("+1.00000"), Ok(Amount(Amount::FRACTION)));
        assert_eq!(Amount::from_str("-1.00000"), Ok(Amount(-Amount::FRACTION)));
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
        assert_eq!(format!("{}", Amount::from_str("1.00000").unwrap()), "1");

        assert_eq!(format!("{}", Amount::from_str("+1.1").unwrap()), "1.1");
        assert_eq!(format!("{}", Amount::from_str("+1.01").unwrap()), "1.01");
        assert_eq!(format!("{}", Amount::from_str("+1.001").unwrap()), "1.001");
        assert_eq!(
            format!("{}", Amount::from_str("+1.0001").unwrap()),
            "1.0001"
        );
        assert_eq!(format!("{}", Amount::from_str("+1.00000").unwrap()), "1");

        assert_eq!(format!("{}", Amount::from_str("+0.1").unwrap()), "0.1");
        assert_eq!(format!("{}", Amount::from_str("+0.01").unwrap()), "0.01");
        assert_eq!(format!("{}", Amount::from_str("+0.001").unwrap()), "0.001");
        assert_eq!(
            format!("{}", Amount::from_str("+0.0001").unwrap()),
            "0.0001"
        );
        assert_eq!(format!("{}", Amount::from_str("+0.00000").unwrap()), "0");

        assert_eq!(format!("{}", Amount::from_str("-1.1").unwrap()), "-1.1");
        assert_eq!(format!("{}", Amount::from_str("-1.01").unwrap()), "-1.01");
        assert_eq!(format!("{}", Amount::from_str("-1.001").unwrap()), "-1.001");
        assert_eq!(
            format!("{}", Amount::from_str("-1.0001").unwrap()),
            "-1.0001"
        );
        assert_eq!(format!("{}", Amount::from_str("-1.00000").unwrap()), "-1");

        assert_eq!(format!("{}", Amount::from_str("-0.1").unwrap()), "-0.1");
        assert_eq!(format!("{}", Amount::from_str("-0.01").unwrap()), "-0.01");
        assert_eq!(format!("{}", Amount::from_str("-0.001").unwrap()), "-0.001");
        assert_eq!(
            format!("{}", Amount::from_str("-0.0001").unwrap()),
            "-0.0001"
        );
        assert_eq!(format!("{}", Amount::from_str("-0.00000").unwrap()), "0");

        assert_eq!(
            format!("{}", Amount::from_str("1.00010").unwrap()),
            "1.0001"
        );
        assert_eq!(
            format!("{}", Amount::from_str("-1.0001").unwrap()),
            "-1.0001"
        );
        assert!(Amount::from_str("-1.00011").is_err());
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
                Amount::from_str("56.12340").unwrap(),
                Amount::from_str("78.12340").unwrap()
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
                Amount::from_str("78.12340").unwrap(),
                Amount::from_str("56.12340").unwrap()
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
            Amount::from_str("2.50000").unwrap() == Amount::from_str("2.50000").unwrap(),
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
