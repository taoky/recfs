use std::fmt::{Display, Formatter};
use std::str::FromStr;
use uuid::Uuid;

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Fid {
    id: Option<Uuid>,
}

impl Fid {
    pub fn root() -> Self {
        Self { id: None }
    }
}

impl Display for Fid {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.to_string().as_str())
    }
}

impl FromStr for Fid {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match Uuid::parse_str(s) {
            Ok(id) => Ok(Self { id: Some(id) }),
            Err(e) => {
                if s == "0" {
                    Ok(Self { id: None })
                } else {
                    Err(e)
                }
            }
        }
    }
}
