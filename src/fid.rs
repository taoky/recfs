use std::fmt::{Display, Formatter};
use std::str::FromStr;

#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Fid {
    // Possible of a Fid could be: 0, UUID, B_0, R_0
    id: String,
}

impl Fid {
    pub fn root() -> Self {
        Self {
            id: "0".to_string(),
        }
    }
}

impl Display for Fid {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.id.as_str())
    }
}

impl FromStr for Fid {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self { id: s.to_string() })
    }
}
