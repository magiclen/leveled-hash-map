use std::collections::{HashMap, HashSet, hash_map::Keys};
use std::hash::Hash;
use std::sync::Arc;
use std::fmt::{self, Debug, Formatter};

#[derive(Debug)]
pub struct LeveledHashMap<K: Eq + Hash, V> {
    pool: Vec<HashMap<Arc<K>, (Option<Arc<K>>, V)>>,
    sub: Vec<HashMap<Arc<K>, HashSet<Arc<K>>>>,
}

pub enum LeveledHashMapError<K> {
    KeyTooMany,
    KeyNotExist {
        level: usize,
        key: Arc<K>,
    },
    KeyChainEmpty,
    KeyChainIncorrect {
        level: usize,
        key: Arc<K>,
        last_key: Option<Arc<K>>,
    },
}

impl<K> Debug for LeveledHashMapError<K> {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        match self {
            LeveledHashMapError::KeyTooMany => {
                f.write_str("KeyTooMany")?;
            }
            LeveledHashMapError::KeyNotExist { level, .. } => {
                f.write_fmt(format_args!("KeyNotExist(Level={})", level))?;
            }
            LeveledHashMapError::KeyChainEmpty => {
                f.write_str("KeyChainEmpty")?;
            }
            LeveledHashMapError::KeyChainIncorrect { level, .. } => {
                f.write_fmt(format_args!("KeyChainIncorrect(Level={})", level))?;
            }
        }

        Ok(())
    }
}

impl<K: Clone + Eq + Hash, V> LeveledHashMap<K, V> {
    pub fn new() -> LeveledHashMap<K, V> {
        LeveledHashMap {
            pool: Vec::new(),
            sub: Vec::new(),
        }
    }

    pub fn get(&self, key_chain: &[Arc<K>]) -> Option<&V> {
        self.get_advanced(key_chain, 0)
    }

    pub fn get_advanced(&self, key_chain: &[Arc<K>], start_level: usize) -> Option<&V> {
        self.get_low_level(key_chain, start_level).ok().map(|v| v.1)
    }

    pub fn get_low_level(&self, key_chain: &[Arc<K>], start_level: usize) -> Result<(Option<Arc<K>>, &V), LeveledHashMapError<K>> {
        let key_chain_len = key_chain.len();

        if key_chain_len == 0 {
            return Err(LeveledHashMapError::KeyChainEmpty);
        } else if key_chain_len + start_level > self.pool.len() {
            return Err(LeveledHashMapError::KeyTooMany);
        }

        let key_chain_len_dec = key_chain_len - 1;

        let mut i = 0;

        let mut last_key = None;

        while i < key_chain_len_dec {
            let ii = i + start_level;
            let ck = &key_chain[i];
            match self.pool[ii].get(ck) {
                Some((pk, _)) => {
                    if ii > start_level && last_key.ne(&pk.as_ref()) {
                        return Err(LeveledHashMapError::KeyChainIncorrect {
                            level: ii,
                            key: Arc::clone(ck),
                            last_key: pk.as_ref().map(|v| Arc::clone(v)),
                        });
                    }
                    last_key = Some(&ck);
                }
                None => return Err(LeveledHashMapError::KeyNotExist {
                    level: ii,
                    key: Arc::clone(ck),
                })
            }

            i += 1;
        }

        let ck = &key_chain[key_chain_len_dec];

        let ii = key_chain_len_dec + start_level;

        match self.pool[ii].get(ck) {
            Some((pk, v)) => {
                if ii > start_level && last_key.ne(&pk.as_ref()) {
                    return Err(LeveledHashMapError::KeyChainIncorrect {
                        level: ii,
                        key: Arc::clone(ck),
                        last_key: pk.as_ref().map(|v| Arc::clone(v)),
                    });
                }
                let pk = pk.as_ref().map(|v| Arc::clone(v));
                Ok((pk, v))
            }
            None => Err(LeveledHashMapError::KeyNotExist {
                level: ii,
                key: Arc::clone(ck),
            })
        }
    }

    pub fn remove(&mut self, key_chain: &[Arc<K>]) -> Option<(V, Vec<HashMap<Arc<K>, (Option<Arc<K>>, V)>>)> {
        self.remove_advanced(key_chain, 0)
    }

    pub fn remove_advanced(&mut self, key_chain: &[Arc<K>], start_level: usize) -> Option<(V, Vec<HashMap<Arc<K>, (Option<Arc<K>>, V)>>)> {
        self.remove_low_level(key_chain, start_level).ok().map(|v| (v.1, v.2))
    }

    pub fn remove_low_level(&mut self, key_chain: &[Arc<K>], start_level: usize) -> Result<(Option<Arc<K>>, V, Vec<HashMap<Arc<K>, (Option<Arc<K>>, V)>>), LeveledHashMapError<K>> {
        let last_key = self.get_low_level(key_chain, start_level)?.0;

        let key_chain_len = key_chain.len();

        let key_chain_len_dec = key_chain_len - 1;

        let level = key_chain_len_dec + start_level;

        let (pk, v) = self.pool[level].remove(&key_chain[key_chain_len_dec]).unwrap();

        if level > 0 {
            if let Some(v) = self.sub[level - 1].get_mut(&last_key.unwrap()) {
                v.remove(&key_chain[key_chain_len_dec]);
            }
        }

        let sub = self.sub[level].remove(&key_chain[key_chain_len_dec]).unwrap();

        if sub.is_empty() {
            return Ok((pk, v, Vec::new()));
        }

        let mut sub_values: Vec<HashMap<Arc<K>, (Option<Arc<K>>, V)>> = Vec::new();
        let mut my_sub_values = HashMap::new();

        for s in sub {
            let (a, b, mut c) = self.remove_low_level(&[Arc::clone(&s)], level + 1).unwrap();

            let len = c.len();

            sub_values.reserve(len);

            c.reverse();

            for i in (0..len).rev() {
                match sub_values.get_mut(i) {
                    Some(h) => {
                        for (k, v) in c.remove(i) {
                            h.insert(k, v);
                        }
                    }
                    None => {
                        sub_values.push(c.remove(i));
                    }
                }
            }

            my_sub_values.insert(s, (a, b));
        }

        sub_values.insert(0, my_sub_values);

        Ok((pk, v, sub_values))
    }

    pub fn insert(&mut self, key_chain: &[Arc<K>], value: V) -> Result<Option<V>, LeveledHashMapError<K>> {
        let key_chain_len = key_chain.len();

        if key_chain_len == 0 {
            return Err(LeveledHashMapError::KeyChainEmpty);
        }

        let key_chain_len_dec = key_chain_len - 1;

        if key_chain_len_dec > self.pool.len() {
            return Err(LeveledHashMapError::KeyTooMany);
        }

        match self.get_low_level(key_chain, 0) {
            Ok(_) => {
                if key_chain_len_dec > 0 {
                    Ok(self.pool[key_chain_len_dec].insert(Arc::clone(&key_chain[key_chain_len_dec]), (Some(Arc::clone(&key_chain[key_chain_len_dec - 1])), value)).map(|v| v.1))
                } else {
                    Ok(self.pool[0].insert(Arc::clone(&key_chain[0]), (None, value)).map(|v| v.1))
                }
            }
            Err(err) => match err {
                LeveledHashMapError::KeyChainEmpty => Err(LeveledHashMapError::KeyChainEmpty),
                LeveledHashMapError::KeyTooMany => {
                    if self.pool.is_empty() {
                        let mut map = HashMap::new();

                        map.insert(Arc::clone(&key_chain[0]), (None, value));

                        self.pool.push(map);

                        let mut map = HashMap::new();

                        map.insert(Arc::clone(&key_chain[0]), HashSet::new());

                        self.sub.push(map);

                        Ok(None)
                    } else {
                        let mut map = HashMap::new();

                        map.insert(Arc::clone(&key_chain[key_chain_len_dec]), (Some(Arc::clone(&key_chain[key_chain_len_dec - 1])), value));

                        self.pool.push(map);

                        let mut map = HashMap::new();

                        map.insert(Arc::clone(&key_chain[key_chain_len_dec]), HashSet::new());

                        self.sub.push(map);

                        let map = self.sub[key_chain_len_dec - 1].get_mut(&key_chain[key_chain_len_dec - 1]).unwrap();

                        map.insert(Arc::clone(&key_chain[key_chain_len_dec]));

                        Ok(None)
                    }
                }
                LeveledHashMapError::KeyChainIncorrect { level, key, last_key } => Err(LeveledHashMapError::KeyChainIncorrect { level, key, last_key }),
                LeveledHashMapError::KeyNotExist { level, key } => {
                    self.sub[level].insert(Arc::clone(&key_chain[key_chain_len_dec]), HashSet::new());
                    if level > 0 {
                        self.pool[level].insert(key, (Some(Arc::clone(&key_chain[key_chain_len_dec - 1])), value));
                        self.sub[level - 1].get_mut(&key_chain[key_chain_len_dec - 1]).unwrap().insert(Arc::clone(&key_chain[key_chain_len_dec]));
                    } else {
                        self.pool[level].insert(key, (None, value));
                    }
                    Ok(None)
                }
            }
        }
    }

    pub fn insert_many(&mut self, key_chain: &[Arc<K>], value: HashMap<K, V>, start_level: usize) -> Result<HashMap<Arc<K>, V>, LeveledHashMapError<K>> {
        let key_chain_len = key_chain.len();

        if key_chain_len > self.pool.len() + 1 {
            return Err(LeveledHashMapError::KeyTooMany);
        }

        match self.get_low_level(key_chain, start_level) {
            Ok(_) => {
                let key_chain_len_dec = key_chain_len - 1;

                let mut previous = HashMap::new();

                let level = key_chain_len + start_level;

                if level >= self.pool.len() {
                    self.pool.push(HashMap::new());
                    self.sub.push(HashMap::new());
                }

                let last_key = &key_chain[key_chain_len_dec];

                let mut temp = HashMap::new();

                for (k, v) in value {
                    let k = Arc::new(k);

                    if let Some((pk, _)) = self.pool[level].get(&Arc::clone(&k)) {
                        if last_key.ne(&pk.as_ref().unwrap()) {
                            return Err(LeveledHashMapError::KeyChainIncorrect {
                                level,
                                key: Arc::clone(&k),
                                last_key: pk.as_ref().map(|v| Arc::clone(v)),
                            });
                        }
                    }

                    temp.insert(k, v);
                }

                for (k, v) in temp {
                    match self.pool[level].insert(Arc::clone(&k), (Some(Arc::clone(last_key)), v)) {
                        Some((_, v)) => {
                            previous.insert(k, v);
                        }
                        None => {
                            self.sub[level].insert(Arc::clone(&k), HashSet::new());
                            self.sub[level - 1].get_mut(last_key).unwrap().insert(Arc::clone(&k));
                        }
                    }
                }

                Ok(previous)
            }
            Err(err) => {
                match err {
                    LeveledHashMapError::KeyChainIncorrect { level, key, last_key } => Err(LeveledHashMapError::KeyChainIncorrect { level, key, last_key }),
                    LeveledHashMapError::KeyNotExist { level, key } => Err(LeveledHashMapError::KeyNotExist { level, key }),
                    LeveledHashMapError::KeyTooMany => Err(LeveledHashMapError::KeyTooMany),
                    LeveledHashMapError::KeyChainEmpty => {
                        if start_level > 0 {
                            return Err(LeveledHashMapError::KeyChainEmpty);
                        }

                        if self.pool.is_empty() {
                            self.pool.push(HashMap::new());
                            self.sub.push(HashMap::new());
                        }

                        let mut previous = HashMap::new();

                        for (k, v) in value {
                            let k = Arc::new(k);
                            match self.pool[0].insert(Arc::clone(&k), (None, v)) {
                                Some((_, v)) => {
                                    previous.insert(k, v);
                                }
                                None => {
                                    self.sub[0].insert(Arc::clone(&k), HashSet::new());
                                }
                            }
                        }

                        Ok(previous)
                    }
                }
            }
        }
    }

    pub fn keys(&mut self, level: usize) -> Option<Keys<Arc<K>, HashSet<Arc<K>>>> {
        match self.sub.get(level) {
            Some(v) => Some(v.keys()),
            None => None
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_advanced() {
        let mut map: LeveledHashMap<&'static str, u8> = LeveledHashMap::new();

        map.insert(&[Arc::new("food")], 10).unwrap();

        assert_eq!(&10, map.get_advanced(&[Arc::new("food")], 0).unwrap());

        map.insert(&[Arc::new("animal")], 11).unwrap();

        assert_eq!(&11, map.get_advanced(&[Arc::new("animal")], 0).unwrap());

        map.insert(&[Arc::new("plant")], 12).unwrap();

        assert_eq!(&12, map.get_advanced(&[Arc::new("plant")], 0).unwrap());

        map.insert(&[Arc::new("plant")], 13).unwrap();

        assert_eq!(&13, map.get_advanced(&[Arc::new("plant")], 0).unwrap());

        map.insert(&[Arc::new("food"), Arc::new("dessert")], 20).unwrap();

        assert_eq!(&20, map.get_advanced(&[Arc::new("food"), Arc::new("dessert")], 0).unwrap());

        map.insert(&[Arc::new("food"), Arc::new("dessert")], 21).unwrap();

        assert_eq!(&21, map.get_advanced(&[Arc::new("food"), Arc::new("dessert")], 0).unwrap());

        assert_eq!(&21, map.get_advanced(&[Arc::new("dessert")], 1).unwrap());

        map.insert(&[Arc::new("food"), Arc::new("dessert"), Arc::new("cake")], 30).unwrap();

        assert_eq!(&30, map.get_advanced(&[Arc::new("food"), Arc::new("dessert"), Arc::new("cake")], 0).unwrap());

        map.insert(&[Arc::new("food"), Arc::new("dessert"), Arc::new("pudding")], 31).unwrap();

        assert_eq!(&31, map.get_advanced(&[Arc::new("food"), Arc::new("dessert"), Arc::new("pudding")], 0).unwrap());

        assert_eq!(&31, map.get_advanced(&[Arc::new("dessert"), Arc::new("pudding")], 1).unwrap());

        assert_eq!(&31, map.get_advanced(&[Arc::new("pudding")], 2).unwrap());

        assert!(map.insert(&[Arc::new("animal"), Arc::new("dessert")], 205).is_err());

        map.insert(&[Arc::new("animal"), Arc::new("mammal")], 77).unwrap();

        assert_eq!(&77, map.get_advanced(&[Arc::new("mammal")], 1).unwrap());

        map.insert(&[Arc::new("food"), Arc::new("dessert"), Arc::new("cake")], 30).unwrap();

        map.insert(&[Arc::new("food"), Arc::new("dessert"), Arc::new("cake"), Arc::new("cheese cake")], 40).unwrap();

        assert_eq!(&40, map.get_advanced(&[Arc::new("food"), Arc::new("dessert"), Arc::new("cake"), Arc::new("cheese cake")], 0).unwrap());

        let remove_result = map.remove_advanced(&[Arc::new("food"), Arc::new("dessert")], 0).unwrap();

        assert_eq!(21, remove_result.0);
        assert_eq!(2, remove_result.1.len());
        assert_eq!(2, remove_result.1[0].len());

        let mut batch = HashMap::new();

        batch.insert("bacterium", 50);
        batch.insert("virus", 51);
        batch.insert("food", 55);

        let remove_result = map.insert_many(&[], batch, 0).unwrap();
        assert_eq!(&10, remove_result.get(&Arc::new("food")).unwrap());

        assert_eq!(&50, map.get_advanced(&[Arc::new("bacterium")], 0).unwrap());
        assert_eq!(&51, map.get_advanced(&[Arc::new("virus")], 0).unwrap());
        assert_eq!(&55, map.get_advanced(&[Arc::new("food")], 0).unwrap());

        let mut batch = HashMap::new();

        batch.insert("dessert", 100);
        batch.insert("meat", 101);
        batch.insert("soup", 102);

        let remove_result = map.insert_many(&[Arc::new("food")], batch, 0).unwrap();

        assert_eq!(&100, map.get_advanced(&[Arc::new("dessert")], 1).unwrap());
        assert_eq!(&101, map.get_advanced(&[Arc::new("meat")], 1).unwrap());
        assert_eq!(&102, map.get_advanced(&[Arc::new("food"), Arc::new("soup")], 0).unwrap());

        assert_eq!(0, remove_result.len());
    }
}
