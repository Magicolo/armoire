mod defer;
mod insert;
mod key;
mod read;
mod remove;
mod write;

pub use defer::Defer;
pub use key::Key;
pub use read::Read;
pub use write::Write;

use crate::{insert::Inserts, key::Keys, remove::Removes};
use parking_lot::Mutex;
use rayon::prelude::*;
use std::{collections::HashSet, mem::swap};

pub struct Armoire<T> {
    keys: Keys,
    reads: Vec<Option<T>>,
    writes: Vec<Option<T>>,
    inserts: Mutex<Vec<(Key, T)>>,
    removes: Mutex<HashSet<Key>>,
}

enum Clones {
    None,
    Partial,
    Full,
}

impl<T> Armoire<T> {
    pub fn new() -> Self {
        Self {
            keys: Keys::new(),
            reads: Vec::new(),
            writes: Vec::new(),
            inserts: Mutex::new(Vec::new()),
            removes: Mutex::new(HashSet::new()),
        }
    }
}

impl<T> Armoire<T> {
    pub fn len(&self) -> usize {
        self.reads.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(&self, key: Key) -> Option<&T> {
        if self.keys.valid(key) {
            self.reads[key.index()].as_ref()
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, key: Key) -> Option<&mut T> {
        if self.keys.valid(key) {
            self.reads[key.index()].as_mut()
        } else {
            None
        }
    }

    pub fn insert(&mut self, value: T) -> (Key, &mut T) {
        let [key] = self.keys.reserve_n_mut::<1>();
        Self::ensure(&mut self.keys, &mut self.reads);
        self.keys.set(key);
        (key, self.reads[key.index()].insert(value))
    }

    pub fn insert_n<const N: usize>(&mut self, values: [T; N]) -> [Key; N] {
        let keys = self.keys.reserve_n_mut::<N>();
        Self::ensure(&mut self.keys, &mut self.reads);
        for (key, value) in keys.iter().copied().zip(values) {
            self.keys.set(key);
            self.reads[key.index()] = Some(value);
        }
        keys
    }

    pub fn try_insert<P: IntoIterator<Item = (Key, T)>>(
        &mut self,
        pairs: P,
    ) -> Inserts<P::IntoIter, T> {
        Inserts::new(pairs.into_iter(), &mut self.keys, &mut self.reads)
    }

    pub fn remove<K: IntoIterator<Item = Key>>(&mut self, keys: K) -> Removes<K::IntoIter, T> {
        Removes::new(
            keys.into_iter(),
            &mut self.keys,
            &mut self.reads,
            &mut self.writes,
        )
    }

    pub fn iter(&self) -> impl DoubleEndedIterator<Item = (Key, &T)> {
        self.reads
            .iter()
            .enumerate()
            .filter_map(|(i, read)| Some((self.keys.key(i)?, read.as_ref()?)))
    }

    pub fn iter_mut(&mut self) -> impl DoubleEndedIterator<Item = (Key, &mut T)> {
        self.reads
            .iter_mut()
            .enumerate()
            .filter_map(|(i, read)| Some((self.keys.key(i)?, read.as_mut()?)))
    }

    pub fn scope<U>(&mut self, scope: impl FnOnce(Write<T>, Read<T>, Defer<T>) -> U) -> U {
        let mut clones = Clones::None;
        let writes = Write::new(&self.keys, &mut self.writes, &self.reads, &mut clones);
        let reads = Read::new(&self.keys, &self.reads);
        let defer = Defer::new(&self.keys, &self.inserts, &self.removes);
        let value = scope(writes, reads, defer);
        match clones {
            Clones::None => {}
            Clones::Partial => todo!(),
            Clones::Full => swap(&mut self.reads, &mut self.writes),
        }
        Self::resolve_defer(
            &mut self.keys,
            &mut self.reads,
            &mut self.writes,
            &mut self.inserts,
            &mut self.removes,
        );
        value
    }

    pub(crate) fn ensure(keys: &mut Keys, reads: &mut Vec<Option<T>>) {
        reads.resize_with(keys.ensure(), || None);
    }

    fn resolve_defer(
        keys: &mut Keys,
        reads: &mut Vec<Option<T>>,
        writes: &mut [Option<T>],
        inserts: &mut Mutex<Vec<(Key, T)>>,
        removes: &mut Mutex<HashSet<Key>>,
    ) {
        drop(Inserts::new(inserts.get_mut().drain(..), keys, reads));
        drop(Removes::new(removes.get_mut().drain(), keys, reads, writes));
    }
}

impl<T: Send + Sync> Armoire<T> {
    pub fn par_iter(&self) -> impl ParallelIterator<Item = (Key, &T)> {
        self.reads
            .par_iter()
            .enumerate()
            .filter_map(|(i, read)| Some((self.keys.key(i)?, read.as_ref()?)))
    }

    pub fn par_iter_mut(&mut self) -> impl ParallelIterator<Item = (Key, &mut T)> {
        self.reads
            .par_iter_mut()
            .enumerate()
            .filter_map(|(i, read)| Some((self.keys.key(i)?, read.as_mut()?)))
    }
}

impl<T> Default for Armoire<T> {
    fn default() -> Self {
        Self::new()
    }
}
