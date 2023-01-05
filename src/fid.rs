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
        f.write_str(
            self.id
                .map(|i| i.to_string())
                .unwrap_or_else(|| "0".to_owned())
                .as_str(),
        )
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
