use crate::key::{Key, Keys};
use rayon::prelude::*;

pub struct Read<'a, T>(&'a Keys, &'a [Option<T>]);

impl<'a, T> Read<'a, T> {
    pub(crate) const fn new(keys: &'a Keys, reads: &'a [Option<T>]) -> Self {
        Self(keys, reads)
    }

    pub fn get(&self, key: Key) -> Option<&T> {
        let index = key.index();
        let slot = self.0.slots.0.get(index)?;
        if slot.generation == key.generation() {
            self.1.get(index)?.as_ref()
        } else {
            None
        }
    }

    pub fn iter(&self) -> impl DoubleEndedIterator<Item = (Key, &T)> {
        self.1.iter().enumerate().filter_map(|(i, read)| {
            let value = read.as_ref()?;
            let key = Key::new(self.0.slots.0.get(i)?.generation, i as _);
            Some((key, value))
        })
    }
}

impl<T: Sync> Read<'_, T> {
    pub fn par_iter(&self) -> impl ParallelIterator<Item = (Key, &T)> {
        self.1.par_iter().enumerate().filter_map(|(i, read)| {
            let value = read.as_ref()?;
            let key = Key::new(self.0.slots.0.get(i)?.generation, i as _);
            Some((key, value))
        })
    }
}

impl<T> Clone for Read<'_, T> {
    fn clone(&self) -> Self {
        Self(self.0, self.1)
    }
}

impl<T> Copy for Read<'_, T> {}
