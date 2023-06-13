use crate::{
    key::{Key, Keys},
    utility::FullIterator,
};
use rayon::prelude::*;

pub struct Read<'a, T>(&'a Keys, &'a [(Key, T)]);

impl<'a, T> Read<'a, T> {
    pub(crate) const fn new(keys: &'a Keys, reads: &'a [(Key, T)]) -> Self {
        Self(keys, reads)
    }

    pub fn get(&self, key: Key) -> Option<&T> {
        let row = self.0.row(key)?;
        let (_, value) = self.1.get(row as usize)?;
        Some(value)
    }

    pub fn iter(&self) -> impl FullIterator<Item = (Key, &T)> {
        self.1.iter().map(|(key, value)| (*key, value))
    }
}

impl<T: Sync> Read<'_, T> {
    pub fn par_iter(&self) -> impl IndexedParallelIterator<Item = (Key, &T)> {
        self.1.par_iter().map(|(key, value)| (*key, value))
    }
}

impl<T> Clone for Read<'_, T> {
    fn clone(&self) -> Self {
        Self(self.0, self.1)
    }
}

impl<T> Copy for Read<'_, T> {}
