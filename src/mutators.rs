use rand::Rng;
use crate::config::KeyMap;  

pub fn random_mutate(map: &KeyMap, mutatable_keys: &Vec<String>) -> KeyMap {
    let mut next = map.clone();
    let mut rng = rand::thread_rng();
    let mutatable_length = mutatable_keys.len();
    let index1 = rng.gen_range(0..mutatable_length);
    let index2 = rng.gen_range(0..mutatable_length);
    let key1 = &mutatable_keys[index1];
    let key2 = &mutatable_keys[index2];
    let letter1 = map.get(key1);
    let letter2 = map.get(key2);
    if let (Some(l1), Some(l2)) = (letter1, letter2) {
        next.insert(key1.to_string(), *l2);
        next.insert(key2.to_string(), *l1);
    }
    next
}
