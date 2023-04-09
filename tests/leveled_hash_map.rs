use std::{collections::HashMap, sync::Arc};

use leveled_hash_map::LeveledHashMap;

#[test]
fn advanced() {
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

    assert_eq!(
        &30,
        map.get_advanced(&[Arc::new("food"), Arc::new("dessert"), Arc::new("cake")], 0).unwrap()
    );

    map.insert(&[Arc::new("food"), Arc::new("dessert"), Arc::new("pudding")], 31).unwrap();

    assert_eq!(
        &31,
        map.get_advanced(&[Arc::new("food"), Arc::new("dessert"), Arc::new("pudding")], 0).unwrap()
    );

    assert_eq!(&31, map.get_advanced(&[Arc::new("dessert"), Arc::new("pudding")], 1).unwrap());

    assert_eq!(&31, map.get_advanced(&[Arc::new("pudding")], 2).unwrap());

    assert!(map.insert(&[Arc::new("animal"), Arc::new("dessert")], 205).is_err());

    map.insert(&[Arc::new("animal"), Arc::new("mammal")], 77).unwrap();

    assert_eq!(&77, map.get_advanced(&[Arc::new("mammal")], 1).unwrap());

    map.insert(&[Arc::new("food"), Arc::new("dessert"), Arc::new("cake")], 30).unwrap();

    map.insert(
        &[Arc::new("food"), Arc::new("dessert"), Arc::new("cake"), Arc::new("cheese cake")],
        40,
    )
    .unwrap();

    assert_eq!(
        &40,
        map.get_advanced(
            &[Arc::new("food"), Arc::new("dessert"), Arc::new("cake"), Arc::new("cheese cake")],
            0
        )
        .unwrap()
    );

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
