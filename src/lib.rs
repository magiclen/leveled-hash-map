use std::{
    collections::{HashMap, HashSet},
    error::Error,
    fmt::{self, Debug, Display, Formatter},
    hash::Hash,
    sync::Arc,
};

/// A single stored entry: the optional parent key at the superior level paired with the value.
type Entry<K, V> = (Option<Arc<K>>, V);

/// One level of the map, mapping each key to its entry.
type Level<K, V> = HashMap<Arc<K>, Entry<K, V>>;

/// The children of every key at one level, mapping each key to the set of its child keys at the inferior level.
type ChildKeys<K> = HashMap<Arc<K>, HashSet<Arc<K>>>;

/// The outcome of removing a node: its parent key, its value, and the levels of its removed subtree (`[i]` holds the descendants one level plus `i` below the removed node).
type Removed<K, V> = (Option<Arc<K>>, V, Vec<Level<K, V>>);

/// A structure to separate values into different levels with keys. Every key-value entry which is not at the top level has a parent key at the superior level. Keys at the same level are unique, no matter what parent keys they have.
#[derive(Debug)]
pub struct LeveledHashMap<K: Eq + Hash, V> {
    pool: Vec<Level<K, V>>,
    sub:  Vec<ChildKeys<K>>,
}

/// Possible errors come from `LeveledHashMap`.
pub enum LeveledHashMapError<K> {
    /// The length of a key chain is over the max level of a `LeveledHashMap`.
    /// ```
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
    /// map.insert(&[Arc::new("food"), Arc::new("dessert"), Arc::new("cake")], 100)
    ///     .unwrap();
    ///
    /// // try to get "food/dessert/chocolate"
    ///
    /// match map.get_professional(
    ///     &[Arc::new("food"), Arc::new("dessert"), Arc::new("chocolate")],
    ///     0,
    /// ) {
    ///     Ok(_) => unreachable!(),
    ///     Err(err) => match err {
    ///         LeveledHashMapError::KeyNotExist {
    ///             level,
    ///             key,
    ///         } => {
    ///             assert_eq!(2, level);
    ///             assert_eq!(Arc::new("chocolate"), key);
    ///         },
    ///         _ => unreachable!(),
    ///     },
    /// }
    /// ```
    KeyNotExist { level: usize, key: Arc<K> },
    /// The key chain is empty.
    /// ```
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
    ///     Err(err) => match err {
    ///         LeveledHashMapError::KeyChainEmpty => (),
    ///         _ => unreachable!(),
    ///     },
    /// }
    /// ```
    KeyChainEmpty,
    /// The key chain is incorrect.
    /// ```
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
    /// map.insert(&[Arc::new("food"), Arc::new("dessert"), Arc::new("cake")], 100)
    ///     .unwrap();
    ///
    /// // try to get "food/meat/cake", here "food/meat" exists
    ///
    /// match map.get_professional(
    ///     &[Arc::new("food"), Arc::new("meat"), Arc::new("cake")],
    ///     0,
    /// ) {
    ///     Ok(_) => unreachable!(),
    ///     Err(err) => match err {
    ///         LeveledHashMapError::KeyChainIncorrect {
    ///             level,
    ///             key,
    ///             last_key,
    ///         } => {
    ///             assert_eq!(2, level);
    ///             assert_eq!(Arc::new("cake"), key);
    ///             assert_eq!(Some(Arc::new("dessert")), last_key);
    ///         },
    ///         _ => unreachable!(),
    ///     },
    /// }
    /// ```
    KeyChainIncorrect { level: usize, key: Arc<K>, last_key: Option<Arc<K>> },
}

impl<K> Debug for LeveledHashMapError<K> {
    #[inline]
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        match self {
            LeveledHashMapError::KeyTooMany => f.write_str("KeyTooMany"),
            LeveledHashMapError::KeyNotExist {
                level, ..
            } => {
                let mut s = f.debug_struct("KeyNotExist");
                s.field("Level", level);
                s.finish()
            },
            LeveledHashMapError::KeyChainEmpty => f.write_str("KeyChainEmpty"),
            LeveledHashMapError::KeyChainIncorrect {
                level, ..
            } => {
                let mut s = f.debug_struct("KeyChainIncorrect");
                s.field("Level", level);
                s.finish()
            },
        }
    }
}

impl<K> Display for LeveledHashMapError<K> {
    #[inline]
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        match self {
            LeveledHashMapError::KeyTooMany => f.write_str(
                "The length of a key chain is over the max level of a `LeveledHashMap`.",
            ),
            LeveledHashMapError::KeyNotExist {
                level, ..
            } => write!(
                f,
                "The key chain is correct, but the last key at level {level} in the key chain \
                 does not exist."
            ),
            LeveledHashMapError::KeyChainEmpty => f.write_str("The key chain is empty."),
            LeveledHashMapError::KeyChainIncorrect {
                level, ..
            } => write!(f, "The key chain is incorrect at level {level}."),
        }
    }
}

impl<K> Error for LeveledHashMapError<K> {}

impl<K> Clone for LeveledHashMapError<K> {
    /// Clone the error. The `Arc<K>` keys are shared, so no `K: Clone` bound is required.
    #[inline]
    fn clone(&self) -> Self {
        match self {
            LeveledHashMapError::KeyTooMany => LeveledHashMapError::KeyTooMany,
            LeveledHashMapError::KeyNotExist {
                level,
                key,
            } => LeveledHashMapError::KeyNotExist {
                level: *level, key: Arc::clone(key)
            },
            LeveledHashMapError::KeyChainEmpty => LeveledHashMapError::KeyChainEmpty,
            LeveledHashMapError::KeyChainIncorrect {
                level,
                key,
                last_key,
            } => LeveledHashMapError::KeyChainIncorrect {
                level:    *level,
                key:      Arc::clone(key),
                last_key: last_key.as_ref().map(Arc::clone),
            },
        }
    }
}

impl<K: Eq + Hash, V> LeveledHashMap<K, V> {
    /// Create a new `LeveledHashMap` instance. The key needs to be implemented `Eq` and `Hash` traits.
    /// ```
    /// use leveled_hash_map::LeveledHashMap;
    ///
    /// let _map: LeveledHashMap<&'static str, String> = LeveledHashMap::new();
    /// ```
    #[inline]
    pub const fn new() -> LeveledHashMap<K, V> {
        LeveledHashMap {
            pool: Vec::new(), sub: Vec::new()
        }
    }

    /// Walk the whole key chain from `start_level`, validating every intermediate link, and return the level index of the final key on success. Unlike `get_professional`, this does not clone the parent key.
    fn locate(
        &self,
        key_chain: &[Arc<K>],
        start_level: usize,
    ) -> Result<usize, LeveledHashMapError<K>> {
        let key_chain_len = key_chain.len();

        if key_chain_len == 0 {
            return Err(LeveledHashMapError::KeyChainEmpty);
        } else if key_chain_len + start_level > self.pool.len() {
            return Err(LeveledHashMapError::KeyTooMany);
        }

        let mut last_key: Option<&Arc<K>> = None;

        for (i, ck) in key_chain.iter().enumerate() {
            let ii = i + start_level;

            match self.pool[ii].get(ck) {
                Some((pk, _)) => {
                    // The first key sits at `start_level` and has no parent to check against within this chain.
                    if ii > start_level && last_key != pk.as_ref() {
                        return Err(LeveledHashMapError::KeyChainIncorrect {
                            level:    ii,
                            key:      Arc::clone(ck),
                            last_key: pk.as_ref().map(Arc::clone),
                        });
                    }
                    last_key = Some(ck);
                },
                None => {
                    return Err(LeveledHashMapError::KeyNotExist {
                        level: ii,
                        key:   Arc::clone(ck),
                    });
                },
            }
        }

        Ok(key_chain_len - 1 + start_level)
    }

    /// Get a value by a key chain. The key chain starts at Level 0.
    /// ```
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
        let level = self.locate(key_chain, start_level).ok()?;

        self.pool[level].get(&key_chain[key_chain.len() - 1]).map(|(_, v)| v)
    }

    /// Get a value by a key chain and a level which the key chain starts with.
    /// ```
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
        let level = self.locate(key_chain, start_level).ok()?;

        self.pool[level].get_mut(&key_chain[key_chain.len() - 1]).map(|(_, v)| v)
    }

    /// Get a value and its parent key by a key chain and a level which the key chain starts with. It returns a `Err(LeveledHashMapError)` instance to describe the reason of the getting failure.
    /// ```
    /// use std::sync::Arc;
    ///
    /// use leveled_hash_map::LeveledHashMap;
    ///
    /// let mut map: LeveledHashMap<&'static str, String> = LeveledHashMap::new();
    ///
    /// map.insert(&[Arc::new("food")], "食物".to_string()).unwrap();
    ///
    /// map.insert(&[Arc::new("food"), Arc::new("dessert")], "甜點".to_string())
    ///     .unwrap();
    ///
    /// let result_1 = map.get_professional(&[Arc::new("food")], 0).unwrap();
    ///
    /// let result_2 = map
    ///     .get_professional(&[Arc::new("food"), Arc::new("dessert")], 0)
    ///     .unwrap();
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
        let level = self.locate(key_chain, start_level)?;

        // `locate` already confirmed the final key exists at this level.
        let (pk, v) = self.pool[level].get(&key_chain[key_chain.len() - 1]).unwrap();

        Ok((pk.as_ref().map(Arc::clone), v))
    }

    /// Get a value and its parent key by a key chain and a level which the key chain starts with. It returns a `Err(LeveledHashMapError)` instance to describe the reason of the getting failure.
    /// ```
    /// use std::sync::Arc;
    ///
    /// use leveled_hash_map::LeveledHashMap;
    ///
    /// let mut map: LeveledHashMap<&'static str, String> = LeveledHashMap::new();
    ///
    /// map.insert(&[Arc::new("food")], "食物".to_string()).unwrap();
    ///
    /// map.insert(&[Arc::new("food"), Arc::new("dessert")], "甜點".to_string())
    ///     .unwrap();
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
        let level = self.locate(key_chain, start_level)?;

        // `locate` already confirmed the final key exists at this level.
        let (pk, v) = self.pool[level].get_mut(&key_chain[key_chain.len() - 1]).unwrap();

        Ok((pk.as_ref().map(Arc::clone), v))
    }

    /// Remove a value by a key chain. The key chain starts at Level 0.
    /// ```
    /// use std::sync::Arc;
    ///
    /// use leveled_hash_map::LeveledHashMap;
    ///
    /// let mut map: LeveledHashMap<&'static str, String> = LeveledHashMap::new();
    ///
    /// map.insert(&[Arc::new("food")], "食物".to_string()).unwrap();
    ///
    /// map.insert(&[Arc::new("food"), Arc::new("dessert")], "甜點".to_string())
    ///     .unwrap();
    ///
    /// map.insert(&[Arc::new("food"), Arc::new("meat")], "肉類".to_string())
    ///     .unwrap();
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
    pub fn remove(&mut self, key_chain: &[Arc<K>]) -> Option<(V, Vec<Level<K, V>>)> {
        self.remove_advanced(key_chain, 0)
    }

    /// Remove a value by a key chain and a level which the key chain starts with.
    /// ```
    /// use std::sync::Arc;
    ///
    /// use leveled_hash_map::LeveledHashMap;
    ///
    /// let mut map: LeveledHashMap<&'static str, String> = LeveledHashMap::new();
    ///
    /// map.insert(&[Arc::new("food")], "食物".to_string()).unwrap();
    ///
    /// map.insert(&[Arc::new("food"), Arc::new("dessert")], "甜點".to_string())
    ///     .unwrap();
    ///
    /// map.insert(&[Arc::new("food"), Arc::new("meat")], "肉類".to_string())
    ///     .unwrap();
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
    ) -> Option<(V, Vec<Level<K, V>>)> {
        self.remove_professional(key_chain, start_level).ok().map(|v| (v.1, v.2))
    }

    /// Remove a value by a key chain and a level which the key chain starts with. It returns a `Err(LeveledHashMapError)` instance to describe the reason of the getting failure.
    /// ```
    /// use std::sync::Arc;
    ///
    /// use leveled_hash_map::LeveledHashMap;
    ///
    /// let mut map: LeveledHashMap<&'static str, String> = LeveledHashMap::new();
    ///
    /// map.insert(&[Arc::new("food")], "食物".to_string()).unwrap();
    ///
    /// map.insert(&[Arc::new("food"), Arc::new("dessert")], "甜點".to_string())
    ///     .unwrap();
    ///
    /// map.insert(&[Arc::new("food"), Arc::new("meat")], "肉類".to_string())
    ///     .unwrap();
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
    ) -> Result<Removed<K, V>, LeveledHashMapError<K>> {
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

        // `levels[i]` collects every removed descendant living at `level + 1 + i`.
        let mut levels: Vec<Level<K, V>> = Vec::new();
        let mut direct_children = HashMap::new();

        for s in sub {
            let (parent, value, descendants) =
                self.remove_professional(&[Arc::clone(&s)], level + 1).unwrap();

            // Merge each depth of this child's subtree into the shared depth-aligned accumulator.
            for (i, descendant_level) in descendants.into_iter().enumerate() {
                match levels.get_mut(i) {
                    Some(existing) => existing.extend(descendant_level),
                    None => levels.push(descendant_level),
                }
            }

            direct_children.insert(s, (parent, value));
        }

        levels.insert(0, direct_children);

        Ok((pk, v, levels))
    }

    /// Insert a value by a key chain. It returns a `Err(LeveledHashMapError)` instance to describe the reason of the getting failure.
    /// ```
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

        let last = key_chain_len - 1;

        // Validate the parent chain first so we never build a dangling link. Any error (a missing intermediate key, an incorrect link, or too many keys) is reported to the caller instead of corrupting the map.
        if last > 0 {
            self.locate(&key_chain[..last], 0)?;
        }

        let final_key = &key_chain[last];

        match self.pool.get(last).and_then(|level| level.get(final_key)) {
            Some((pk, _)) => {
                // The key already exists, so make sure it hangs under the expected parent before overwriting it.
                let expected_parent = if last > 0 { Some(&key_chain[last - 1]) } else { None };

                if pk.as_ref() != expected_parent {
                    return Err(LeveledHashMapError::KeyChainIncorrect {
                        level:    last,
                        key:      Arc::clone(final_key),
                        last_key: pk.as_ref().map(Arc::clone),
                    });
                }

                let parent = expected_parent.map(Arc::clone);

                Ok(self.pool[last].insert(Arc::clone(final_key), (parent, value)).map(|v| v.1))
            },
            None => {
                // A brand-new key. Append a fresh deepest level when this key introduces one.
                if last == self.pool.len() {
                    self.pool.push(HashMap::new());
                    self.sub.push(HashMap::new());
                }

                let parent = if last > 0 { Some(Arc::clone(&key_chain[last - 1])) } else { None };

                self.pool[last].insert(Arc::clone(final_key), (parent, value));
                self.sub[last].insert(Arc::clone(final_key), HashSet::new());

                if last > 0 {
                    self.sub[last - 1]
                        .get_mut(&key_chain[last - 1])
                        .unwrap()
                        .insert(Arc::clone(final_key));
                }

                Ok(None)
            },
        }
    }

    /// Insert values by a key chain and a `HashMap` instance and a level which the key chain starts with. It returns a `Err(LeveledHashMapError)` instance to describe the reason of the getting failure.
    /// ```
    /// use std::{collections::HashMap, sync::Arc};
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

                    if let Some((pk, _)) = self.pool[level].get(&k) {
                        if last_key.ne(pk.as_ref().unwrap()) {
                            return Err(LeveledHashMapError::KeyChainIncorrect {
                                level,
                                key: Arc::clone(&k),
                                last_key: pk.as_ref().map(Arc::clone),
                            });
                        }
                    }

                    temp.insert(k, v);
                }

                for (k, v) in temp {
                    match self.pool[level].insert(Arc::clone(&k), (Some(Arc::clone(last_key)), v)) {
                        Some((_, v)) => {
                            previous.insert(k, v);
                        },
                        None => {
                            self.sub[level].insert(Arc::clone(&k), HashSet::new());
                            self.sub[level - 1].get_mut(last_key).unwrap().insert(Arc::clone(&k));
                        },
                    }
                }

                Ok(previous)
            },
            Err(err) => match err {
                LeveledHashMapError::KeyChainIncorrect {
                    level,
                    key,
                    last_key,
                } => Err(LeveledHashMapError::KeyChainIncorrect {
                    level,
                    key,
                    last_key,
                }),
                LeveledHashMapError::KeyNotExist {
                    level,
                    key,
                } => Err(LeveledHashMapError::KeyNotExist {
                    level,
                    key,
                }),
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
                            },
                            None => {
                                self.sub[0].insert(Arc::clone(&k), HashSet::new());
                            },
                        }
                    }

                    Ok(previous)
                },
            },
        }
    }

    /// Get the keys at a specific level.
    /// ```
    /// use std::{collections::HashMap, sync::Arc};
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
    pub fn keys(&self, level: usize) -> Option<&ChildKeys<K>> {
        self.sub.get(level)
    }

    /// Get the number of levels currently allocated in this `LeveledHashMap`.
    /// ```
    /// use std::sync::Arc;
    ///
    /// use leveled_hash_map::LeveledHashMap;
    ///
    /// let mut map: LeveledHashMap<&'static str, u8> = LeveledHashMap::new();
    ///
    /// assert_eq!(0, map.levels());
    ///
    /// map.insert(&[Arc::new("food")], 100).unwrap();
    /// map.insert(&[Arc::new("food"), Arc::new("dessert")], 10).unwrap();
    ///
    /// assert_eq!(2, map.levels());
    /// ```
    #[inline]
    pub const fn levels(&self) -> usize {
        self.pool.len()
    }

    /// Get the total number of entries stored across all levels.
    /// ```
    /// use std::sync::Arc;
    ///
    /// use leveled_hash_map::LeveledHashMap;
    ///
    /// let mut map: LeveledHashMap<&'static str, u8> = LeveledHashMap::new();
    ///
    /// map.insert(&[Arc::new("food")], 100).unwrap();
    /// map.insert(&[Arc::new("food"), Arc::new("dessert")], 10).unwrap();
    ///
    /// assert_eq!(2, map.len());
    /// ```
    #[inline]
    pub fn len(&self) -> usize {
        self.pool.iter().map(HashMap::len).sum()
    }

    /// Return whether this `LeveledHashMap` holds no entry at all. An empty top level means every level is empty, because a deeper key always needs a parent above it.
    /// ```
    /// use std::sync::Arc;
    ///
    /// use leveled_hash_map::LeveledHashMap;
    ///
    /// let mut map: LeveledHashMap<&'static str, u8> = LeveledHashMap::new();
    ///
    /// assert!(map.is_empty());
    ///
    /// map.insert(&[Arc::new("food")], 100).unwrap();
    ///
    /// assert!(!map.is_empty());
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        match self.pool.first() {
            Some(level) => level.is_empty(),
            None => true,
        }
    }

    /// Return whether a value exists for the given key chain, which starts at Level 0.
    /// ```
    /// use std::sync::Arc;
    ///
    /// use leveled_hash_map::LeveledHashMap;
    ///
    /// let mut map: LeveledHashMap<&'static str, u8> = LeveledHashMap::new();
    ///
    /// map.insert(&[Arc::new("food")], 100).unwrap();
    ///
    /// assert!(map.contains_key(&[Arc::new("food")]));
    /// assert!(!map.contains_key(&[Arc::new("drink")]));
    /// ```
    #[inline]
    pub fn contains_key(&self, key_chain: &[Arc<K>]) -> bool {
        self.get(key_chain).is_some()
    }

    /// Remove every entry and level, resetting this `LeveledHashMap` to its initial state.
    /// ```
    /// use std::sync::Arc;
    ///
    /// use leveled_hash_map::LeveledHashMap;
    ///
    /// let mut map: LeveledHashMap<&'static str, u8> = LeveledHashMap::new();
    ///
    /// map.insert(&[Arc::new("food")], 100).unwrap();
    ///
    /// map.clear();
    ///
    /// assert!(map.is_empty());
    /// assert_eq!(0, map.levels());
    /// ```
    #[inline]
    pub fn clear(&mut self) {
        self.pool.clear();
        self.sub.clear();
    }
}

impl<K: Eq + Hash, V> Default for LeveledHashMap<K, V> {
    #[inline]
    fn default() -> Self {
        LeveledHashMap::new()
    }
}
