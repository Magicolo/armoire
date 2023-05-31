use crate::key::{Key, Keys};
use parking_lot::Mutex;
use rayon::prelude::*;
use std::{borrow::BorrowMut, collections::HashSet};

pub struct Write<'a, T> {
    keys: &'a Keys,
    writes: &'a mut Vec<Option<T>>,
    reads: &'a [Option<T>],
    indices: &'a mut HashSet<usize>,
}

impl<'a, T> Write<'a, T> {
    pub(crate) fn new(
        keys: &'a Keys,
        writes: &'a mut Vec<Option<T>>,
        reads: &'a [Option<T>],
        indices: &'a mut HashSet<usize>,
    ) -> Self {
        Self {
            keys,
            writes,
            reads,
            indices,
        }
    }
}

impl<T: Clone> Write<'_, T> {
    pub fn get_mut(&mut self, key: Key) -> Option<&mut T> {
        if self.keys.valid(key) {
            if self.indices.insert(key.index()) {
                self.writes.resize_with(self.reads.len(), || None);
                let read = self.reads.get(key.index())?.as_ref()?;
                let write = self.writes.get_mut(key.index())?;
                Some(write.insert(read.clone()))
            } else {
                self.writes.get_mut(key.index())?.as_mut()
            }
        } else {
            None
        }
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Key, &mut T)> {
        self.writes.resize_with(self.reads.len(), || None);
        self.writes.iter_mut().enumerate().filter_map(|(i, write)| {
            if self.indices.insert(i) {
                let read = self.reads.get(i)?.as_ref()?;
                Some((self.keys.key(i)?, write.insert(read.clone())))
            } else {
                Some((self.keys.key(i)?, write.as_mut()?))
            }
        })
    }
}

impl<T: Clone + Send + Sync> Write<'_, T> {
    pub fn par_iter_mut(&mut self) -> impl ParallelIterator<Item = (Key, &mut T)> {
        let indices = Mutex::new(self.indices.borrow_mut());
        let reads = self.reads;
        let keys = self.keys;
        self.writes.resize_with(self.reads.len(), || None);
        self.writes
            .par_iter_mut()
            .enumerate()
            .filter_map(move |(i, write)| {
                if indices.lock().insert(i) {
                    let read = reads.get(i)?.as_ref()?;
                    Some((keys.key(i)?, write.insert(read.clone())))
                } else {
                    Some((keys.key(i)?, write.as_mut()?))
                }
            })
    }
}
