use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use crate::DataError;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Market {
    USStock,
    AShare,
    HKStock,
    Crypto,
    Forex,
}

impl fmt::Display for Market {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Market::USStock => write!(f, "US"),
            Market::AShare  => write!(f, "A股"),
            Market::HKStock => write!(f, "HK"),
            Market::Crypto  => write!(f, "Crypto"),
            Market::Forex   => write!(f, "Forex"),
        }
    }
}

impl FromStr for Market {
    type Err = DataError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "us" | "us_stock" | "nasdaq" | "nyse" => Ok(Market::USStock),
            "a_share" | "ashare" | "cn" | "china" => Ok(Market::AShare),
            "hk" | "hk_stock" | "hkex"            => Ok(Market::HKStock),
            "crypto" | "btc" | "eth"               => Ok(Market::Crypto),
            "forex" | "fx"                         => Ok(Market::Forex),
            _ => Err(DataError::Parse(format!("Unknown market: {s}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_market_display() {
        assert_eq!(Market::USStock.to_string(), "US");
        assert_eq!(Market::AShare.to_string(), "A股");
        assert_eq!(Market::HKStock.to_string(), "HK");
        assert_eq!(Market::Crypto.to_string(), "Crypto");
    }

    #[test]
    fn test_market_from_str() {
        assert_eq!("us".parse::<Market>().unwrap(), Market::USStock);
        assert_eq!("a_share".parse::<Market>().unwrap(), Market::AShare);
        assert_eq!("hk".parse::<Market>().unwrap(), Market::HKStock);
        assert!("invalid".parse::<Market>().is_err());
    }
}
