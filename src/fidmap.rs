use crate::{client::list::RecListItem, fid::Fid};
use bimap::BiBTreeMap;
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct FidCachedList {
    pub children: Option<Vec<RecListItem>>, // None => type is not dir
}

pub struct FidMap {
    fhmap: BiBTreeMap<u64, Fid>, // a bidirectional map of "file handle" and Fid
    listing_map: HashMap<Fid, FidCachedList>, // a map from Fid to the HTTP cache of listing
    parent_map: HashMap<Fid, Option<Fid>>, // a map from Fid to its parent
}

impl FidMap {
    pub fn new() -> Self {
        let mut fm = Self {
            fhmap: BiBTreeMap::new(),
            listing_map: HashMap::new(),
            parent_map: HashMap::new(),
        };
        fm.parent_map.insert(Fid::root(), None);
        fm
    }

    pub fn get_fid_by_fh(&self, fh: u64) -> Option<Fid> {
        self.fhmap.get_by_left(&fh).cloned()
    }

    pub fn get_parent_fid(&self, fid: &Fid) -> Option<Option<Fid>> {
        self.parent_map.get(fid).cloned()
    }

    pub fn get_listing(&self, fid: &Fid) -> Option<&FidCachedList> {
        self.listing_map.get(fid)
    }

    pub fn get_listing_mut(&mut self, fid: Fid) -> &mut FidCachedList {
        self.listing_map
            .entry(fid)
            .or_default()
    }

    pub fn get_parentmap_mut(&mut self) -> &mut HashMap<Fid, Option<Fid>> {
        &mut self.parent_map
    }

    // set the file handle for a Fid, and return the file handle
    // it will not update maps if file handle exists
    pub fn set_fh(&mut self, fid: &Fid, parent: Option<&Fid>, list: Option<&FidCachedList>) -> u64 {
        match self.fhmap.get_by_right(fid) {
            Some(&fh) => fh,
            None => {
                let fh = self.next_fh();
                self.fhmap.insert(fh, fid.clone());
                match list {
                    Some(list) => {
                        self.update_fid(fid, parent, list);
                    }
                    None => {
                        assert!(
                            (self.listing_map.contains_key(fid)
                                && self.parent_map.contains_key(fid))
                                || (fid.is_created())
                        );
                    }
                };

                fh
            }
        }
    }

    pub fn update_fid(&mut self, fid: &Fid, parent: Option<&Fid>, list: &FidCachedList) {
        self.listing_map.insert(fid.clone(), list.clone());
        self.parent_map.insert(fid.clone(), parent.cloned());
    }

    fn next_fh(&self) -> u64 {
        self.fhmap
            .iter()
            .enumerate()
            .find(|(i, (&a, _))| (*i as u64) < a)
            .map(|(i, _)| (i as u64) + 4)
            .unwrap_or((self.fhmap.len() as u64) + 3)
    }
}
