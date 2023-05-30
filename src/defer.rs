use crate::key::{Key, Keys};
use parking_lot::Mutex;
use std::collections::HashSet;

pub struct Defer<'a, T>(&'a Keys, &'a Mutex<Vec<(Key, T)>>, &'a Mutex<HashSet<Key>>);

impl<'a, T> Defer<'a, T> {
    pub(crate) const fn new(
        keys: &'a Keys,
        inserts: &'a Mutex<Vec<(Key, T)>>,
        removes: &'a Mutex<HashSet<Key>>,
    ) -> Self {
        Self(keys, inserts, removes)
    }

    pub fn insert(&self, value: T) -> Key {
        let [key] = self.0.reserve_n::<1>();
        self.1.lock().push((key, value));
        key
    }

    pub fn insert_n<const N: usize>(&self, values: [T; N]) -> [Key; N] {
        let keys = self.0.reserve_n::<N>();
        self.1.lock().extend(keys.iter().copied().zip(values));
        keys
    }

    pub fn try_insert<P: IntoIterator<Item = (Key, T)>>(&self, pairs: P) {
        self.1.lock().extend(pairs)
    }

    pub fn remove<K: IntoIterator<Item = Key>>(&self, keys: K) {
        self.2.lock().extend(keys);
    }
}

impl<T> Clone for Defer<'_, T> {
    fn clone(&self) -> Self {
        Self(self.0, self.1, self.2)
    }
}

impl<T> Copy for Defer<'_, T> {}
