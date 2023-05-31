use crate::{
    key::{Key, Keys},
    Clones,
};
use rayon::prelude::*;
use std::mem::replace;

pub struct Write<'a, T>(
    &'a Keys,
    &'a mut Vec<Option<T>>,
    &'a [Option<T>],
    &'a mut Clones,
);

impl<'a, T> Write<'a, T> {
    pub(crate) fn new(
        keys: &'a Keys,
        writes: &'a mut Vec<Option<T>>,
        reads: &'a [Option<T>],
        clones: &'a mut Clones,
    ) -> Self {
        Self(keys, writes, reads, clones)
    }

    fn partial(&mut self) {
        *self.3 = match replace(self.3, Clones::None) {
            Clones::None => {
                // TODO: Do I need to resize?
                self.1.resize_with(self.2.len(), || None);
                Clones::Partial
            }
            Clones::Partial => Clones::Partial,
            Clones::Full => Clones::Full,
        };
    }

    fn full(&mut self) {
        *self.3 = match replace(self.3, Clones::None) {
            Clones::None => {
                self.1.resize_with(self.2.len(), || None);
                Clones::Full
            }
            Clones::Partial => Clones::Full,
            Clones::Full => Clones::Full,
        };
    }
}

impl<T: Clone> Write<'_, T> {
    pub fn get_mut(&mut self, key: Key) -> Option<&mut T> {
        if self.0.valid(key) {
            self.partial();
            let read = self.2.get(key.index())?.as_ref()?;
            let write = self.1.get_mut(key.index())?;
            Some(write.insert(read.clone()))
        } else {
            None
        }
    }

    pub fn iter_mut(&mut self) -> impl DoubleEndedIterator<Item = (Key, &mut T)> {
        self.full(); // ???
        todo!();
        [].into_iter()
    }
}

impl<T: Clone + Send + Sync> Write<'_, T> {
    pub fn par_iter_mut(&mut self) -> impl ParallelIterator<Item = (Key, &mut T)> {
        self.full(); // ???
        todo!();
        [].into_par_iter()
    }
}
