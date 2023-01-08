use std::fmt::{Display, Formatter};
use std::str::FromStr;

use uuid::Uuid;

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd, Debug)]
enum FidValue {
    Root,
    UUID(Uuid),
    BackupRoot,
    RecycleRoot,
    Write(usize),
}

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd, Debug)]
pub struct Fid {
    // Possible of a Fid could be: 0, UUID, B_0, R_0
    id: FidValue,
}

impl Fid {
    pub fn root() -> Self {
        Self { id: FidValue::Root }
    }

    pub fn is_created(&self) -> bool {
        match self.id {
            FidValue::Write(_) => true,
            _ => false,
        }
    }
}

impl Display for Fid {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.id {
            FidValue::Root => return f.write_str("0"),
            FidValue::BackupRoot => return f.write_str("B_0"),
            FidValue::RecycleRoot => return f.write_str("R_0"),
            FidValue::Write(id) => return f.write_fmt(format_args!("write-{}", id)),
            FidValue::UUID(uid) => return f.write_str(&uid.to_string()),
        }
    }
}

impl FromStr for Fid {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match Uuid::parse_str(s) {
            Ok(id) => Ok(Self {
                id: FidValue::UUID(id),
            }),
            Err(e) => match s {
                "0" => Ok(Self { id: FidValue::Root }),
                "B_0" => Ok(Self {
                    id: FidValue::BackupRoot,
                }),
                "R_0" => Ok(Self {
                    id: FidValue::RecycleRoot,
                }),
                _ => {
                    let s = s
                        .strip_prefix("write-")
                        .ok_or_else(|| anyhow::anyhow!("Invalid Fid: {}", s))?;
                    let write_id = s.parse::<usize>()?;
                    Ok(Self {
                        id: FidValue::Write(write_id),
                    })
                }
            },
        }
    }
}

// impl From<String> for Fid {
//     fn from(s: String) -> Self {
//         Self { id: s }
//     }
// }
