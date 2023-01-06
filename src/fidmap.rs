use crate::fid::Fid;
use bimap::BiBTreeMap;
use std::collections::HashMap;

pub struct FidMap {
    map: BiBTreeMap<u64, Fid>,
    parent_map: HashMap<Fid, Option<Fid>>,
}

impl FidMap {
    pub fn new() -> Self {
        Self {
            map: BiBTreeMap::new(),
            parent_map: HashMap::new(),
        }
    }

    pub fn get(&self, fh: u64) -> Option<Fid> {
        self.map.get_by_left(&fh).cloned()
    }

    pub fn get_parent(&self, fid: Fid) -> Option<Option<Fid>> {
        self.parent_map.get(&fid).cloned()
    }

    pub fn set(&mut self, fid: Fid, parent_fid: Option<Fid>) -> u64 {
        match self.map.get_by_right(&fid) {
            Some(&fh) => fh,
            None => {
                let fh = self.next_fh();
                self.map.insert(fh, fid.clone());
                self.parent_map.insert(fid, parent_fid);
                fh
            }
        }
    }

    fn next_fh(&self) -> u64 {
        self.map
            .iter()
            .enumerate()
            .find(|(i, (&a, _))| (*i as u64) < a)
            .map(|(i, _)| (i as u64) + 4)
            .unwrap_or((self.map.len() as u64) + 3)
    }
}
