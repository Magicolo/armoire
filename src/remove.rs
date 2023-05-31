use crate::key::{Key, Keys};
use std::iter::FusedIterator;

pub struct Removes<'a, K: Iterator<Item = Key>, T> {
    removes: K,
    keys: &'a mut Keys,
    reads: &'a mut [Option<T>],
    writes: &'a mut [Option<T>],
}

impl<'a, K: Iterator<Item = Key>, T> Removes<'a, K, T> {
    pub(crate) fn new(
        removes: K,
        keys: &'a mut Keys,
        reads: &'a mut [Option<T>],
        writes: &'a mut [Option<T>],
    ) -> Self {
        let count = *keys.free.1.get_mut();
        keys.free.0.truncate(count.max(0) as _);
        Self {
            removes,
            keys,
            reads,
            writes,
        }
    }

    fn remove(&mut self, key: Key) -> Result<(Key, T), Key> {
        if let Some(slot) = self.keys.get_mut(key) {
            if slot.is(key.generation()) {
                if let Some(read) = self.reads.get_mut(key.index()) {
                    if let Some(value) = read.take() {
                        if let Some(write @ Some(_)) = self.writes.get_mut(key.index()) {
                            *write = None;
                        }
                        if let Some(key) = key.increment() {
                            self.keys.free.0.push(key);
                        }
                        return Ok((key, value));
                    }
                }
            }
        }
        Err(key)
    }
}

impl<'a, K: Iterator<Item = Key>, T> Iterator for Removes<'a, K, T> {
    type Item = Result<(Key, T), Key>;

    fn next(&mut self) -> Option<Self::Item> {
        let key = self.removes.next()?;
        Some(self.remove(key))
    }
}

impl<'a, K: DoubleEndedIterator<Item = Key>, T> DoubleEndedIterator for Removes<'a, K, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let key = self.removes.next_back()?;
        Some(self.remove(key))
    }
}

impl<'a, K: ExactSizeIterator<Item = Key>, T> ExactSizeIterator for Removes<'a, K, T> {
    fn len(&self) -> usize {
        self.removes.len()
    }
}

impl<'a, K: FusedIterator<Item = Key>, T> FusedIterator for Removes<'a, K, T> {}

impl<'a, K: Iterator<Item = Key>, T> Drop for Removes<'a, K, T> {
    fn drop(&mut self) {
        for _ in self.by_ref() {}
        *self.keys.free.1.get_mut() = self.keys.free.0.len() as _;
    }
}
