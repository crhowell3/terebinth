use std::fmt;
use std::str::FromStr;

use terebinth_macros::{Decodable, Encodable, HashStable_Generic};

#[derive(
    Clone, Copy, Hash, PartialEq, PartialOrd, Debug, Encodable, Decodable, Eq, HashStable_Generic,
)]
pub enum Edition {
    Edition2025,
    EditionFuture,
}

pub const ALL_EDITIONS: &[Edition] = &[Edition::Edition2025, Edition::EditionFuture];

pub const EDITION_NAME_LIST: &str = "2025";
pub const DEFAULT_EDITION: Edition = Edition::Edition2025;
pub const LATEST_STABLE_EDITION: Edition = Edition::Edition2025;

impl fmt::Display for Edition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match *self {
            Edition::Edition2025 => "2025",
            Edition::EditionFuture => "Future",
        };
        write!(f, "{s}")
    }
}

impl Edition {
    pub fn lint_name(self) -> &'static str {
        match self {
            Edition::Edition2025 => "terebinth_2025_compatibility",
            Edition::EditionFuture => "edition_future_compatibility",
        }
    }

    pub fn is_stable(self) -> bool {
        match self {
            Edition::Edition2025 => true,
            Edition::EditionFuture => false,
        }
    }

    pub fn is_terebinth_2025(self) -> bool {
        self == Edition::Edition2025
    }

    pub fn at_least_edition_future(self) -> bool {
        self >= Edition::EditionFuture
    }
}

impl FromStr for Edition {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, ()> {
        match s {
            "2025" => Ok(Edition::Edition2025),
            "Future" => Ok(Edition::EditionFuture),
            _ => Err(()),
        }
    }
}
