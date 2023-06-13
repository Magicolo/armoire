use std::iter::FusedIterator;

use crate::key::{Key, Keys};

pub struct Inserts<'a, P: Iterator<Item = (Key, T)>, T> {
    inserts: P,
    keys: &'a mut Keys,
    reads: &'a mut Vec<(Key, T)>,
}

impl<'a, P: Iterator<Item = (Key, T)>, T> Inserts<'a, P, T> {
    pub(crate) fn new(inserts: P, keys: &'a mut Keys, reads: &'a mut Vec<(Key, T)>) -> Self {
        keys.ensure();
        Self {
            inserts,
            keys,
            reads,
        }
    }

    fn insert(&mut self, key: Key, value: T) -> Result<(), T> {
        if let Some(slot) = self.keys.get_mut(key) {
            slot.initialize(key.generation(), self.reads.len() as _);
            self.reads.push((key, value));
            Ok(())
        } else {
            Err(value)
        }
    }
}

impl<'a, P: Iterator<Item = (Key, T)>, T> Iterator for Inserts<'a, P, T> {
    type Item = Result<(), T>;

    fn next(&mut self) -> Option<Self::Item> {
        let (key, value) = self.inserts.next()?;
        Some(self.insert(key, value))
    }
}

impl<'a, P: DoubleEndedIterator<Item = (Key, T)>, T> DoubleEndedIterator for Inserts<'a, P, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let (key, value) = self.inserts.next_back()?;
        Some(self.insert(key, value))
    }
}

impl<'a, P: ExactSizeIterator<Item = (Key, T)>, T> ExactSizeIterator for Inserts<'a, P, T> {
    fn len(&self) -> usize {
        self.inserts.len()
    }
}

impl<'a, P: FusedIterator<Item = (Key, T)>, T> FusedIterator for Inserts<'a, P, T> {}

impl<'a, P: Iterator<Item = (Key, T)>, T> Drop for Inserts<'a, P, T> {
    fn drop(&mut self) {
        for _ in self.by_ref() {}
    }
}
