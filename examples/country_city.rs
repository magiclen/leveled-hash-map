extern crate leveled_hash_map;

use std::collections::HashMap;
use std::sync::Arc;

use leveled_hash_map::LeveledHashMap;

#[derive(Debug)]
struct MultiName {
    us: &'static str,
    tw: &'static str,
    cn: &'static str,
}

fn main() {
    let earth_map: LeveledHashMap<&'static str, MultiName> = {
        let mut map = LeveledHashMap::new();

        map.insert(&[Arc::new("US")], MultiName {
            us: "United States of America",
            tw: "美國",
            cn: "美国",
        })
        .unwrap();

        let mut us_states = HashMap::new();

        us_states.insert("New York", MultiName {
            us: "New York",
            tw: "紐約州",
            cn: "纽约州",
        });
        us_states.insert("Utah", MultiName {
            us: "Utah",
            tw: "猶他州",
            cn: "犹他州",
        });

        map.insert_many(&[Arc::new("US")], us_states, 0).unwrap();

        map.insert(&[Arc::new("CN")], MultiName {
            us: "China",
            tw: "中國",
            cn: "中国",
        })
        .unwrap();

        let mut cn_provinces = HashMap::new();

        cn_provinces.insert("Guangdong", MultiName {
            us: "Guangdong",
            tw: "廣東省",
            cn: "广东省",
        });
        cn_provinces.insert("Fujian", MultiName {
            us: "Fujian",
            tw: "福建省",
            cn: "福建省",
        });

        map.insert_many(&[Arc::new("CN")], cn_provinces, 0).unwrap();

        map.insert(&[Arc::new("TW")], MultiName {
            us: "Taiwan",
            tw: "臺灣",
            cn: "臺湾",
        })
        .unwrap();

        let mut tw_counties = HashMap::new();

        tw_counties.insert("Taipei", MultiName {
            us: "Taipei",
            tw: "台北",
            cn: "台北",
        });

        tw_counties.insert("Taichung", MultiName {
            us: "Taichung",
            tw: "台中",
            cn: "台中",
        });

        map.insert_many(&[Arc::new("TW")], tw_counties, 0).unwrap();

        map
    };

    println!("{:?}", earth_map);

    let countries = earth_map.keys(0).unwrap();

    println!("{:?}", countries); // {"CN": {"Fujian", "Guangdong"}, "US": {"Utah", "New York"}, "TW": {"Taichung", "Taipei"}}

    let new_york = earth_map.get(&[Arc::new("US"), Arc::new("New York")]).unwrap();

    println!("{:?}", new_york); // MultiName { us: "New York", tw: "紐約州", cn: "纽约州" }

    let new_york = earth_map.get_advanced(&[Arc::new("New York")], 1).unwrap();

    println!("{:?}", new_york); // MultiName { us: "New York", tw: "紐約州", cn: "纽约州" }

    let new_york_suspicion = earth_map.get(&[Arc::new("TW"), Arc::new("New York")]);

    println!("{:?}", new_york_suspicion); // None
}
