#![allow(clippy::type_complexity)]

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt::{self, Debug, Display, Formatter};
use std::hash::Hash;
use std::sync::Arc;

/// A structure to separate values into different levels with keys. Every key-value entry which is not at the top level has a parent key at the superior level. Keys at the same level are unique, no matter what parent keys they have.
#[derive(Debug)]
pub struct LeveledHashMap<K: Eq + Hash, V> {
    pool: Vec<HashMap<Arc<K>, (Option<Arc<K>>, V)>>,
    sub: Vec<HashMap<Arc<K>, HashSet<Arc<K>>>>,
}

/// Possible errors come from `LeveledHashMap`.
pub enum LeveledHashMapError<K> {
    /// The length of a key chain is over the max level of a `LeveledHashMap`.
    /// ```
    /// extern crate leveled_hash_map;
    ///
    /// use std::sync::Arc;
    ///
    /// use leveled_hash_map::{LeveledHashMap, LeveledHashMapError};
    ///
    /// let mut map = LeveledHashMap::new();
    ///
    /// map.insert(&[Arc::new("food")], 100).unwrap();
    ///
    /// // now the map has "Level 0", "Level 0" is available to be got, "Level 0" and "Level 1" are available to be inserted
    ///
    /// assert_eq!(&100, map.get_advanced(&[Arc::new("food")], 0).unwrap());
    ///
    /// // try to get value at "Level 1"
    ///
    /// match map.get_professional(&[Arc::new("food"), Arc::new("dessert")], 0) {
    ///     Ok(_) => unreachable!(),
    ///     Err(err) => match err {
    ///         LeveledHashMapError::KeyTooMany => (),
    ///         _ => unreachable!()
    ///     }
    /// }
    ///
    /// // try to insert value to "Level 2"
    ///
    /// match map.insert(&[Arc::new("food"), Arc::new("dessert"), Arc::new("cake")], 10) {
    ///     Ok(_) => unreachable!(),
    ///     Err(err) => match err {
    ///         LeveledHashMapError::KeyTooMany => (),
    ///         _ => unreachable!()
    ///     }
    /// }
    ///
    /// // try to insert value to "Level 1"
    ///
    /// match map.insert(&[Arc::new("food"), Arc::new("dessert")], 10) {
    ///     Ok(_) => (),
    ///     Err(err) => unreachable!()
    /// }
    /// ```
    KeyTooMany,
    /// The key chain is correct, but the last key in the key chain does not exist.
    /// ```
    /// extern crate leveled_hash_map;
    ///
    /// use std::sync::Arc;
    ///
    /// use leveled_hash_map::{LeveledHashMap, LeveledHashMapError};
    ///
    /// let mut map = LeveledHashMap::new();
    ///
    /// map.insert(&[Arc::new("food")], 100).unwrap();
    ///
    /// map.insert(&[Arc::new("food"), Arc::new("dessert")], 100).unwrap();
    ///
    /// map.insert(&[Arc::new("food"), Arc::new("dessert"), Arc::new("cake")], 100).unwrap();
    ///
    /// // try to get "food/dessert/chocolate"
    ///
    /// match map.get_professional(&[Arc::new("food"), Arc::new("dessert"), Arc::new("chocolate")], 0) {
    ///     Ok(_) => unreachable!(),
    ///     Err(err) => {
    ///         match err {
    ///             LeveledHashMapError::KeyNotExist {
    ///                 level,
    ///                 key,
    ///             } => {
    ///                 assert_eq!(2, level);
    ///                 assert_eq!(Arc::new("chocolate"), key);
    ///             }
    ///             _ => unreachable!(),
    ///         }
    ///     }
    /// }
    /// ```
    KeyNotExist {
        level: usize,
        key: Arc<K>,
    },
    /// The key chain is empty.
    /// ```
    /// extern crate leveled_hash_map;
    ///
    /// use std::sync::Arc;
    ///
    /// use leveled_hash_map::{LeveledHashMap, LeveledHashMapError};
    ///
    /// let mut map = LeveledHashMap::new();
    ///
    /// map.insert(&[Arc::new("food")], 100).unwrap();
    ///
    /// // try to get ""
    ///
    /// match map.get_professional(&[], 0) {
    ///     Ok(_) => unreachable!(),
    ///     Err(err) => {
    ///         match err {
    ///             LeveledHashMapError::KeyChainEmpty => (),
    ///             _ => unreachable!(),
    ///         }
    ///     }
    /// }
    /// ```
    KeyChainEmpty,
    /// The key chain is incorrect.
    /// ```
    /// extern crate leveled_hash_map;
    ///
    /// use std::sync::Arc;
    ///
    /// use leveled_hash_map::{LeveledHashMap, LeveledHashMapError};
    ///
    /// let mut map = LeveledHashMap::new();
    ///
    /// map.insert(&[Arc::new("food")], 100).unwrap();
    ///
    /// map.insert(&[Arc::new("food"), Arc::new("meat")], 200).unwrap();
    ///
    /// map.insert(&[Arc::new("food"), Arc::new("dessert")], 100).unwrap();
    ///
    /// map.insert(&[Arc::new("food"), Arc::new("dessert"), Arc::new("cake")], 100).unwrap();
    ///
    /// // try to get "food/meat/chocolate", here "food/meat" exists
    ///
    /// match map.get_professional(&[Arc::new("food"), Arc::new("meat"), Arc::new("cake")], 0) {
    ///     Ok(_) => unreachable!(),
    ///     Err(err) => {
    ///         match err {
    ///             LeveledHashMapError::KeyChainIncorrect {
    ///                 level,
    ///                 key,
    ///                 last_key,
    ///             } => {
    ///                 assert_eq!(2, level);
    ///                 assert_eq!(Arc::new("cake"), key);
    ///                 assert_eq!(Some(Arc::new("dessert")), last_key);
    ///             }
    ///             _ => unreachable!(),
    ///         }
    ///     }
    /// }
    /// ```
    KeyChainIncorrect {
        level: usize,
        key: Arc<K>,
        last_key: Option<Arc<K>>,
    },
}

impl<K> Debug for LeveledHashMapError<K> {
    #[inline]
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        match self {
            LeveledHashMapError::KeyTooMany => f.write_str("KeyTooMany"),
            LeveledHashMapError::KeyNotExist {
                level,
                ..
            } => {
                let mut s = f.debug_struct("KeyNotExist");
                s.field("Level", level);
                s.finish()
            }
            LeveledHashMapError::KeyChainEmpty => f.write_str("KeyChainEmpty"),
            LeveledHashMapError::KeyChainIncorrect {
                level,
                ..
            } => {
                let mut s = f.debug_struct("KeyChainIncorrect");
                s.field("Level", level);
                s.finish()
            }
        }
    }
}

impl<K> Display for LeveledHashMapError<K> {
    #[inline]
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        match self {
            LeveledHashMapError::KeyTooMany => {
                f.write_str("The length of a key chain is over the max level of a `LeveledHashMap`.")
            }
            LeveledHashMapError::KeyNotExist {
                level,
                ..
            } => {
                f.write_fmt(format_args!("The key chain is correct, but the last key at level {} in the key chain does not exist.", level))
            }
            LeveledHashMapError::KeyChainEmpty => {
                f.write_str("The key chain is empty.")
            }
            LeveledHashMapError::KeyChainIncorrect {
                level,
                ..
            } => {
                f.write_fmt(format_args!("The key chain is incorrect at level {}.", level))
            }
        }
    }
}

impl<K> Error for LeveledHashMapError<K> {}

impl<K: Eq + Hash, V> LeveledHashMap<K, V> {
    /// Create a new `LeveledHashMap` instance. The key needs to be implemented `Eq` and `Hash` traits.
    /// ```
    /// extern crate leveled_hash_map;
    ///
    /// use leveled_hash_map::LeveledHashMap;
    ///
    /// let _map: LeveledHashMap<&'static str, String> = LeveledHashMap::new();
    /// ```
    #[inline]
    pub fn new() -> LeveledHashMap<K, V> {
        LeveledHashMap {
            pool: Vec::new(),
            sub: Vec::new(),
        }
    }

    /// Get a value by a key chain. The key chain starts at Level 0.
    /// ```
    /// extern crate leveled_hash_map;
    ///
    /// use std::sync::Arc;
    ///
    /// use leveled_hash_map::LeveledHashMap;
    ///
    /// let map: LeveledHashMap<&'static str, String> = LeveledHashMap::new();
    ///
    /// let _result = map.get(&[Arc::new("first_key")]);
    /// ```
    #[inline]
    pub fn get(&self, key_chain: &[Arc<K>]) -> Option<&V> {
        self.get_advanced(key_chain, 0)
    }

    /// Get a value by a key chain. The key chain starts at Level 0.
    /// ```
    /// extern crate leveled_hash_map;
    ///
    /// use std::sync::Arc;
    ///
    /// use leveled_hash_map::LeveledHashMap;
    ///
    /// let mut map: LeveledHashMap<&'static str, String> = LeveledHashMap::new();
    ///
    /// let _result = map.get_mut(&[Arc::new("first_key")]);
    /// ```
    #[inline]
    pub fn get_mut(&mut self, key_chain: &[Arc<K>]) -> Option<&mut V> {
        self.get_advanced_mut(key_chain, 0)
    }

    /// Get a value by a key chain and a level which the key chain starts with.
    /// ```
    /// extern crate leveled_hash_map;
    ///
    /// use std::sync::Arc;
    ///
    /// use leveled_hash_map::LeveledHashMap;
    ///
    /// let map: LeveledHashMap<&'static str, String> = LeveledHashMap::new();
    ///
    /// let _result = map.get_advanced(&[Arc::new("second_key")], 1);
    /// ```
    #[inline]
    pub fn get_advanced(&self, key_chain: &[Arc<K>], start_level: usize) -> Option<&V> {
        self.get_professional(key_chain, start_level).ok().map(|v| v.1)
    }

    /// Get a value by a key chain and a level which the key chain starts with.
    /// ```
    /// extern crate leveled_hash_map;
    ///
    /// use std::sync::Arc;
    ///
    /// use leveled_hash_map::LeveledHashMap;
    ///
    /// let mut map: LeveledHashMap<&'static str, String> = LeveledHashMap::new();
    ///
    /// let _result = map.get_advanced_mut(&[Arc::new("second_key")], 1);
    /// ```
    #[inline]
    pub fn get_advanced_mut(&mut self, key_chain: &[Arc<K>], start_level: usize) -> Option<&mut V> {
        self.get_professional_mut(key_chain, start_level).ok().map(|v| v.1)
    }

    /// Get a value and its parent key by a key chain and a level which the key chain starts with. It returns a `Err(LeveledHashMapError)` instance to describe the reason of the getting failure.
    /// ```
    /// extern crate leveled_hash_map;
    ///
    /// use std::sync::Arc;
    ///
    /// use leveled_hash_map::LeveledHashMap;
    ///
    /// let mut map: LeveledHashMap<&'static str, String> = LeveledHashMap::new();
    ///
    /// map.insert(&[Arc::new("food")], "食物".to_string()).unwrap();
    ///
    /// map.insert(&[Arc::new("food"), Arc::new("dessert")], "甜點".to_string()).unwrap();
    ///
    /// let result_1 = map.get_professional(&[Arc::new("food")], 0).unwrap();
    ///
    /// let result_2 = map.get_professional(&[Arc::new("food"), Arc::new("dessert")], 0).unwrap();
    ///
    /// assert_eq!(None, result_1.0);
    /// assert_eq!("食物", result_1.1);
    ///
    /// assert_eq!(Some(Arc::new("food")), result_2.0);
    /// assert_eq!("甜點", result_2.1);
    /// ```
    pub fn get_professional(
        &self,
        key_chain: &[Arc<K>],
        start_level: usize,
    ) -> Result<(Option<Arc<K>>, &V), LeveledHashMapError<K>> {
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
                None => {
                    return Err(LeveledHashMapError::KeyNotExist {
                        level: ii,
                        key: Arc::clone(ck),
                    })
                }
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
            None => {
                Err(LeveledHashMapError::KeyNotExist {
                    level: ii,
                    key: Arc::clone(ck),
                })
            }
        }
    }

    /// Get a value and its parent key by a key chain and a level which the key chain starts with. It returns a `Err(LeveledHashMapError)` instance to describe the reason of the getting failure.
    /// ```
    /// extern crate leveled_hash_map;
    ///
    /// use std::sync::Arc;
    ///
    /// use leveled_hash_map::LeveledHashMap;
    ///
    /// let mut map: LeveledHashMap<&'static str, String> = LeveledHashMap::new();
    ///
    /// map.insert(&[Arc::new("food")], "食物".to_string()).unwrap();
    ///
    /// map.insert(&[Arc::new("food"), Arc::new("dessert")], "甜點".to_string()).unwrap();
    ///
    /// let result = map.get_professional_mut(&[Arc::new("food")], 0).unwrap();
    ///
    /// result.1.push_str("/食品");
    ///
    /// assert_eq!(None, result.0);
    /// assert_eq!("食物/食品", result.1);
    /// ```
    pub fn get_professional_mut(
        &mut self,
        key_chain: &[Arc<K>],
        start_level: usize,
    ) -> Result<(Option<Arc<K>>, &mut V), LeveledHashMapError<K>> {
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
                None => {
                    return Err(LeveledHashMapError::KeyNotExist {
                        level: ii,
                        key: Arc::clone(ck),
                    })
                }
            }

            i += 1;
        }

        let ck = &key_chain[key_chain_len_dec];

        let ii = key_chain_len_dec + start_level;

        match self.pool[ii].get_mut(ck) {
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
            None => {
                Err(LeveledHashMapError::KeyNotExist {
                    level: ii,
                    key: Arc::clone(ck),
                })
            }
        }
    }

    /// Remove a value by a key chain. The key chain starts at Level 0.
    /// ```
    /// extern crate leveled_hash_map;
    ///
    /// use std::sync::Arc;
    ///
    /// use leveled_hash_map::LeveledHashMap;
    ///
    /// let mut map: LeveledHashMap<&'static str, String> = LeveledHashMap::new();
    ///
    /// map.insert(&[Arc::new("food")], "食物".to_string()).unwrap();
    ///
    /// map.insert(&[Arc::new("food"), Arc::new("dessert")], "甜點".to_string()).unwrap();
    ///
    /// map.insert(&[Arc::new("food"), Arc::new("meat")], "肉類".to_string()).unwrap();
    ///
    /// let result = map.remove(&[Arc::new("food"), Arc::new("dessert")]).unwrap();
    ///
    /// assert_eq!("甜點", result.0);
    /// assert_eq!(0, result.1.len());
    ///
    /// let result = map.remove(&[Arc::new("food")]).unwrap();
    ///
    /// assert_eq!("食物", result.0);
    /// assert_eq!(1, result.1.len());
    /// assert_eq!(
    ///     &(Some(Arc::new("food")), "肉類".to_string()),
    ///     result.1[0].get(&Arc::new("meat")).unwrap()
    /// );
    /// ```
    #[inline]
    pub fn remove(
        &mut self,
        key_chain: &[Arc<K>],
    ) -> Option<(V, Vec<HashMap<Arc<K>, (Option<Arc<K>>, V)>>)> {
        self.remove_advanced(key_chain, 0)
    }

    /// Remove a value by a key chain and a level which the key chain starts with.
    /// ```
    /// extern crate leveled_hash_map;
    ///
    /// use std::sync::Arc;
    ///
    /// use leveled_hash_map::LeveledHashMap;
    ///
    /// let mut map: LeveledHashMap<&'static str, String> = LeveledHashMap::new();
    ///
    /// map.insert(&[Arc::new("food")], "食物".to_string()).unwrap();
    ///
    /// map.insert(&[Arc::new("food"), Arc::new("dessert")], "甜點".to_string()).unwrap();
    ///
    /// map.insert(&[Arc::new("food"), Arc::new("meat")], "肉類".to_string()).unwrap();
    ///
    /// let result = map.remove_advanced(&[Arc::new("dessert")], 1).unwrap();
    ///
    /// assert_eq!("甜點", result.0);
    /// assert_eq!(0, result.1.len());
    ///
    /// let result = map.remove_advanced(&[Arc::new("food")], 0).unwrap();
    ///
    /// assert_eq!("食物", result.0);
    /// assert_eq!(1, result.1.len());
    /// assert_eq!(
    ///     &(Some(Arc::new("food")), "肉類".to_string()),
    ///     result.1[0].get(&Arc::new("meat")).unwrap()
    /// );
    /// ```
    #[inline]
    pub fn remove_advanced(
        &mut self,
        key_chain: &[Arc<K>],
        start_level: usize,
    ) -> Option<(V, Vec<HashMap<Arc<K>, (Option<Arc<K>>, V)>>)> {
        self.remove_professional(key_chain, start_level).ok().map(|v| (v.1, v.2))
    }

    /// Remove a value by a key chain and a level which the key chain starts with. It returns a `Err(LeveledHashMapError)` instance to describe the reason of the getting failure.
    /// ```
    /// extern crate leveled_hash_map;
    ///
    /// use std::sync::Arc;
    ///
    /// use leveled_hash_map::LeveledHashMap;
    ///
    /// let mut map: LeveledHashMap<&'static str, String> = LeveledHashMap::new();
    ///
    /// map.insert(&[Arc::new("food")], "食物".to_string()).unwrap();
    ///
    /// map.insert(&[Arc::new("food"), Arc::new("dessert")], "甜點".to_string()).unwrap();
    ///
    /// map.insert(&[Arc::new("food"), Arc::new("meat")], "肉類".to_string()).unwrap();
    ///
    /// let result = map.remove_professional(&[Arc::new("dessert")], 1).unwrap();
    ///
    /// assert_eq!(Some(Arc::new("food")), result.0);
    /// assert_eq!("甜點", result.1);
    /// assert_eq!(0, result.2.len());
    ///
    /// let result = map.remove_professional(&[Arc::new("food")], 0).unwrap();
    ///
    /// assert_eq!(None, result.0);
    /// assert_eq!("食物", result.1);
    /// assert_eq!(1, result.2.len());
    /// assert_eq!(
    ///     &(Some(Arc::new("food")), "肉類".to_string()),
    ///     result.2[0].get(&Arc::new("meat")).unwrap()
    /// );
    /// ```
    pub fn remove_professional(
        &mut self,
        key_chain: &[Arc<K>],
        start_level: usize,
    ) -> Result<
        (Option<Arc<K>>, V, Vec<HashMap<Arc<K>, (Option<Arc<K>>, V)>>),
        LeveledHashMapError<K>,
    > {
        let last_key = self.get_professional(key_chain, start_level)?.0;

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
            let (a, b, mut c) = self.remove_professional(&[Arc::clone(&s)], level + 1).unwrap();

            let len = c.len();

            sub_values.reserve(len);

            c.reverse();

            for i in (0..len).rev() {
                if let Some(h) = sub_values.get_mut(i) {
                    for (k, v) in c.remove(i) {
                        h.insert(k, v);
                    }
                    continue;
                }
                sub_values.push(c.remove(i));
            }

            my_sub_values.insert(s, (a, b));
        }

        sub_values.insert(0, my_sub_values);

        Ok((pk, v, sub_values))
    }

    /// Insert a value by a key chain. It returns a `Err(LeveledHashMapError)` instance to describe the reason of the getting failure.
    /// ```
    /// extern crate leveled_hash_map;
    ///
    /// use std::sync::Arc;
    ///
    /// use leveled_hash_map::LeveledHashMap;
    ///
    /// let mut map: LeveledHashMap<&'static str, String> = LeveledHashMap::new();
    ///
    /// map.insert(&[Arc::new("food")], "食物".to_string()).unwrap();
    ///
    /// {
    ///     let result = map.get(&[Arc::new("food")]).unwrap();
    ///
    ///     assert_eq!("食物", result);
    /// }
    ///
    /// let result = map.insert(&[Arc::new("food")], "食品".to_string()).unwrap();
    ///
    /// assert_eq!(Some("食物".to_string()), result);
    ///
    /// let result = map.get(&[Arc::new("food")]).unwrap();
    ///
    /// assert_eq!("食品", result);
    /// ```
    pub fn insert(
        &mut self,
        key_chain: &[Arc<K>],
        value: V,
    ) -> Result<Option<V>, LeveledHashMapError<K>> {
        let key_chain_len = key_chain.len();

        if key_chain_len == 0 {
            return Err(LeveledHashMapError::KeyChainEmpty);
        }

        let key_chain_len_dec = key_chain_len - 1;

        if key_chain_len_dec > self.pool.len() {
            return Err(LeveledHashMapError::KeyTooMany);
        }

        match self.get_professional(key_chain, 0) {
            Ok(_) => {
                if key_chain_len_dec > 0 {
                    Ok(self.pool[key_chain_len_dec]
                        .insert(
                            Arc::clone(&key_chain[key_chain_len_dec]),
                            (Some(Arc::clone(&key_chain[key_chain_len_dec - 1])), value),
                        )
                        .map(|v| v.1))
                } else {
                    Ok(self.pool[0].insert(Arc::clone(&key_chain[0]), (None, value)).map(|v| v.1))
                }
            }
            Err(err) => {
                match err {
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

                            map.insert(
                                Arc::clone(&key_chain[key_chain_len_dec]),
                                (Some(Arc::clone(&key_chain[key_chain_len_dec - 1])), value),
                            );

                            self.pool.push(map);

                            let mut map = HashMap::new();

                            map.insert(Arc::clone(&key_chain[key_chain_len_dec]), HashSet::new());

                            self.sub.push(map);

                            let map = self.sub[key_chain_len_dec - 1]
                                .get_mut(&key_chain[key_chain_len_dec - 1])
                                .unwrap();

                            map.insert(Arc::clone(&key_chain[key_chain_len_dec]));

                            Ok(None)
                        }
                    }
                    LeveledHashMapError::KeyChainIncorrect {
                        level,
                        key,
                        last_key,
                    } => {
                        Err(LeveledHashMapError::KeyChainIncorrect {
                            level,
                            key,
                            last_key,
                        })
                    }
                    LeveledHashMapError::KeyNotExist {
                        level,
                        key,
                    } => {
                        self.sub[level]
                            .insert(Arc::clone(&key_chain[key_chain_len_dec]), HashSet::new());
                        if level > 0 {
                            self.pool[level].insert(
                                key,
                                (Some(Arc::clone(&key_chain[key_chain_len_dec - 1])), value),
                            );
                            self.sub[level - 1]
                                .get_mut(&key_chain[key_chain_len_dec - 1])
                                .unwrap()
                                .insert(Arc::clone(&key_chain[key_chain_len_dec]));
                        } else {
                            self.pool[level].insert(key, (None, value));
                        }
                        Ok(None)
                    }
                }
            }
        }
    }

    /// Insert values by a key chain and a `HashMap` instance and a level which the key chain starts with. It returns a `Err(LeveledHashMapError)` instance to describe the reason of the getting failure.
    /// ```
    /// extern crate leveled_hash_map;
    ///
    /// use std::collections::HashMap;
    /// use std::sync::Arc;
    ///
    /// use leveled_hash_map::LeveledHashMap;
    ///
    /// let mut map: LeveledHashMap<&'static str, String> = LeveledHashMap::new();
    ///
    /// map.insert(&[Arc::new("food")], "食物".to_string()).unwrap();
    ///
    /// let mut insert_map = HashMap::new();
    ///
    /// insert_map.insert("dessert", "甜點".to_string());
    /// insert_map.insert("meat", "肉類".to_string());
    ///
    /// map.insert_many(&[Arc::new("food")], insert_map, 0).unwrap();
    ///
    /// let result = map.get(&[Arc::new("food"), Arc::new("dessert")]).unwrap();
    ///
    /// assert_eq!("甜點", result);
    ///
    /// let result = map.get(&[Arc::new("food"), Arc::new("meat")]).unwrap();
    ///
    /// assert_eq!("肉類", result);
    /// ```
    pub fn insert_many(
        &mut self,
        key_chain: &[Arc<K>],
        value: HashMap<K, V>,
        start_level: usize,
    ) -> Result<HashMap<Arc<K>, V>, LeveledHashMapError<K>> {
        let key_chain_len = key_chain.len();

        if key_chain_len > self.pool.len() + 1 {
            return Err(LeveledHashMapError::KeyTooMany);
        }

        match self.get_professional(key_chain, start_level) {
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
                    LeveledHashMapError::KeyChainIncorrect {
                        level,
                        key,
                        last_key,
                    } => {
                        Err(LeveledHashMapError::KeyChainIncorrect {
                            level,
                            key,
                            last_key,
                        })
                    }
                    LeveledHashMapError::KeyNotExist {
                        level,
                        key,
                    } => {
                        Err(LeveledHashMapError::KeyNotExist {
                            level,
                            key,
                        })
                    }
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

    /// Get the keys at a specific level.
    /// ```
    /// extern crate leveled_hash_map;
    ///
    /// use std::collections::HashMap;
    /// use std::sync::Arc;
    ///
    /// use leveled_hash_map::LeveledHashMap;
    ///
    /// let mut map: LeveledHashMap<&'static str, String> = LeveledHashMap::new();
    ///
    /// map.insert(&[Arc::new("food")], "食物".to_string()).unwrap();
    ///
    /// let mut insert_map = HashMap::new();
    ///
    /// insert_map.insert("dessert", "甜點".to_string());
    /// insert_map.insert("meat", "肉類".to_string());
    ///
    /// map.insert_many(&[Arc::new("food")], insert_map, 0).unwrap();
    ///
    /// let result = map.keys(0).unwrap();
    ///
    /// assert_eq!(1, result.len());
    ///
    /// let result = map.keys(1).unwrap();
    ///
    /// assert_eq!(2, result.len());
    /// ```
    #[inline]
    pub fn keys(&self, level: usize) -> Option<&HashMap<Arc<K>, HashSet<Arc<K>>>> {
        self.sub.get(level)
    }
}

impl<K: Eq + Hash, V> Default for LeveledHashMap<K, V> {
    #[inline]
    fn default() -> Self {
        LeveledHashMap::new()
    }
}
