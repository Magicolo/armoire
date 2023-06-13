use crate::key::{Key, Keys};
use parking_lot::Mutex;
use rayon::prelude::*;
use std::{borrow::BorrowMut, collections::HashSet};

pub struct Write<'a, T> {
    keys: &'a Keys,
    writes: &'a mut Vec<Option<T>>,
    reads: &'a [(Key, T)],
    rows: &'a mut HashSet<u32>,
}

impl<'a, T> Write<'a, T> {
    pub(crate) fn new(
        keys: &'a Keys,
        writes: &'a mut Vec<Option<T>>,
        reads: &'a [(Key, T)],
        rows: &'a mut HashSet<u32>,
    ) -> Self {
        Self {
            keys,
            writes,
            reads,
            rows,
        }
    }
}

impl<T: Clone> Write<'_, T> {
    pub fn get_mut(&mut self, key: Key) -> Option<&mut T> {
        let row = self.keys.row(key)?;
        if self.rows.insert(row) {
            self.writes.resize_with(self.reads.len(), || None);
            let read = self.reads.get(row as usize)?;
            let write = self.writes.get_mut(row as usize)?;
            Some(write.insert(read.1.clone()))
        } else {
            self.writes.get_mut(row as usize)?.as_mut()
        }
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Key, &mut T)> {
        self.writes.resize_with(self.reads.len(), || None);
        self.writes.iter_mut().enumerate().filter_map(|(i, write)| {
            let read = self.reads.get(i)?;
            let write = if self.rows.insert(i as _) {
                write.insert(read.1.clone())
            } else {
                write.as_mut()?
            };
            Some((read.0, write))
        })
    }
}

impl<T: Clone + Send + Sync> Write<'_, T> {
    pub fn par_iter_mut(&mut self) -> impl ParallelIterator<Item = (Key, &mut T)> {
        let rows = Mutex::new(self.rows.borrow_mut());
        let reads = self.reads;
        self.writes.resize_with(self.reads.len(), || None);
        self.writes
            .par_iter_mut()
            .enumerate()
            .filter_map(move |(i, write)| {
                let read = reads.get(i)?;
                let write = if rows.lock().insert(i as _) {
                    write.insert(read.1.clone())
                } else {
                    write.as_mut()?
                };
                Some((read.0, write))
            })
    }
}
