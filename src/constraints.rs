use crate::config::KeyMap;
use rand::Rng;

pub struct Constraints {
    pub immutable_keys: Vec<String>,
}

impl Constraints {
    pub fn new(immutable_keys: Vec<String>) -> Constraints {
        Constraints { immutable_keys }
    }

    pub fn constrained_random_move(&self, map: &KeyMap) -> KeyMap {
        let mut next = map.clone();
        let mut rng = rand::thread_rng();
        let mutable_keys: Vec<&String> = map.keys().collect();
        let mutatable_length = mutable_keys.len();
        let index1 = rng.gen_range(0..mutatable_length);
        let index2 = rng.gen_range(0..mutatable_length);
        let key1 = mutable_keys[index1];
        let key2 = mutable_keys[index2];
        let letter1 = map.get(key1);
        let letter2 = map.get(key2);
        if let (Some(l1), Some(l2)) = (letter1, letter2) {
            next.insert(key1.to_string(), *l2);
            next.insert(key2.to_string(), *l1);
        }
        next
    }
}
