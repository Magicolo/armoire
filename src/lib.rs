mod fork;
mod utility;

use fork::{Fork, Item};
use parking_lot::Mutex;
use rayon::prelude::*;
use std::{
    collections::HashSet,
    mem::replace,
    sync::atomic::{AtomicI64, AtomicU32, Ordering},
};
use utility::FullIterator;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Key {
    generation: u32,
    index: u32,
}

#[derive(Clone, Copy)]
struct Slot {
    generation: u32,
    index: u32,
}

type Pair<T> = (Key, T);

pub struct Armoire<T> {
    last: AtomicU32,
    cursor: AtomicI64,
    slots: Vec<Slot>,
    free: Vec<Key>,
    pairs: Vec<Pair<T>>,
    inserts: Mutex<Vec<Pair<T>>>,
    removes: Mutex<HashSet<Key>>,
}

pub struct Pairs<'a, T> {
    slots: &'a mut Vec<Slot>,
    pairs: &'a mut Vec<Pair<T>>,
}

pub struct Defer<'a, T> {
    last: &'a AtomicU32,
    cursor: &'a AtomicI64,
    free: &'a Vec<Key>,
    inserts: &'a Mutex<Vec<Pair<T>>>,
    removes: &'a Mutex<HashSet<Key>>,
}

impl Slot {
    pub const ZERO: Slot = Slot::new(0, u32::MAX);

    #[inline]
    pub const fn new(generation: u32, index: u32) -> Self {
        Self { generation, index }
    }

    #[inline]
    pub fn initialize(&mut self, generation: u32, index: u32) -> bool {
        debug_assert!(generation < u32::MAX);
        debug_assert!(index < u32::MAX);
        if self.generation == generation && self.index == u32::MAX {
            self.index = index;
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn release(&mut self, generation: u32) -> Option<u32> {
        debug_assert!(generation < u32::MAX);
        if self.generation == generation && self.index < u32::MAX {
            self.generation = self.generation.saturating_add(1);
            Some(replace(&mut self.index, u32::MAX))
        } else {
            None
        }
    }

    #[inline]
    pub fn update(&mut self, index: u32) -> bool {
        debug_assert!(index < u32::MAX);
        if self.generation < u32::MAX || self.index < u32::MAX {
            self.index = index;
            true
        } else {
            false
        }
    }
}

impl Key {
    pub const NULL: Key = Key::new(u32::MAX, u32::MAX);

    #[inline]
    pub(crate) const fn new(generation: u32, index: u32) -> Self {
        Self { generation, index }
    }

    #[inline]
    pub(crate) fn increment(self) -> Option<Key> {
        Some(Key::new(self.generation.checked_add(1)?, self.index))
    }
}

impl<'a, T> Defer<'a, T> {
    #[inline]
    pub fn insert(&self, value: T) -> Key {
        let [key] = self.insert_n([value]);
        key
    }

    #[inline]
    pub fn insert_n<const N: usize>(&self, values: [T; N]) -> [Key; N] {
        let mut keys = [Key::NULL; N];
        reserve(&mut keys, self.cursor, self.free, self.last);
        self.inserts.lock().extend(keys.iter().copied().zip(values));
        keys
    }

    #[inline]
    pub fn try_insert<P: IntoIterator<Item = Pair<T>>>(&self, pairs: P) {
        self.inserts.lock().extend(pairs)
    }

    #[inline]
    pub fn remove<K: IntoIterator<Item = Key>>(&self, keys: K) {
        self.removes.lock().extend(keys);
    }
}

impl<T> Pairs<'_, T> {
    #[inline]
    pub fn len(&self) -> usize {
        self.pairs.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub fn has(&self, key: Key) -> bool {
        index(key, self.slots).is_some()
    }

    #[inline]
    pub fn get(&self, key: Key) -> Option<&T> {
        let index = index(key, self.slots)?;
        Some(&self.pairs[index].1)
    }

    #[inline]
    pub fn fork<'a, L: Item, R: Item>(
        &'a mut self,
        fork: impl Fn(Key, &'a mut T) -> (L, R) + Copy,
    ) -> (
        Fork<'a, Pair<T>, impl Fn(&'a mut Pair<T>) -> L>,
        Fork<'a, Pair<T>, impl Fn(&'a mut Pair<T>) -> R>,
    ) {
        fork::fork(self.pairs, move |pair| fork(pair.0, &mut pair.1))
    }

    #[inline]
    pub fn get_mut(&mut self, key: Key) -> Option<&mut T> {
        let index = index(key, self.slots)?;
        Some(&mut self.pairs[index].1)
    }

    #[inline]
    pub fn iter(&self) -> impl FullIterator<Item = (Key, &T)> {
        self.pairs.iter().map(|(key, value)| (*key, value))
    }

    #[inline]
    pub fn iter_mut(&mut self) -> impl FullIterator<Item = (Key, &mut T)> {
        self.pairs.iter_mut().map(|(key, value)| (*key, value))
    }
}

impl<T: Send + Sync> Pairs<'_, T> {
    #[inline]
    pub fn par_iter(&self) -> impl IndexedParallelIterator<Item = (Key, &T)> {
        self.pairs.par_iter().map(|(key, value)| (*key, value))
    }

    #[inline]
    pub fn par_iter_mut(&mut self) -> impl IndexedParallelIterator<Item = (Key, &mut T)> {
        self.pairs.par_iter_mut().map(|(key, value)| (*key, value))
    }
}

impl<T> Clone for Defer<'_, T> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            last: self.last,
            cursor: self.cursor,
            free: self.free,
            inserts: self.inserts,
            removes: self.removes,
        }
    }
}

impl<T> Armoire<T> {
    #[inline]
    pub fn new() -> Self {
        Self {
            last: AtomicU32::new(0),
            cursor: AtomicI64::new(0),
            slots: Vec::new(),
            free: Vec::new(),
            pairs: Vec::new(),
            inserts: Mutex::new(Vec::new()),
            removes: Mutex::new(HashSet::new()),
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.pairs.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub fn has(&self, key: Key) -> bool {
        index(key, &self.slots).is_some()
    }

    #[inline]
    pub fn get(&self, key: Key) -> Option<&T> {
        let index = index(key, &self.slots)?;
        Some(&self.pairs[index].1)
    }

    #[inline]
    pub fn get_mut(&mut self, key: Key) -> Option<&mut T> {
        let index = index(key, &self.slots)?;
        Some(&mut self.pairs[index].1)
    }

    #[inline]
    pub fn iter(&self) -> impl FullIterator<Item = (Key, &T)> {
        self.pairs.iter().map(|(key, value)| (*key, value))
    }

    #[inline]
    pub fn iter_mut(&mut self) -> impl FullIterator<Item = (Key, &mut T)> {
        self.pairs.iter_mut().map(|(key, value)| (*key, value))
    }

    #[inline]
    pub fn insert(&mut self, value: T) -> Key {
        let [key] = self.insert_n([value]);
        key
    }

    pub fn insert_n<const N: usize>(&mut self, values: [T; N]) -> [Key; N] {
        let keys = self.reserve_n_mut();
        ensure(&mut self.last, &mut self.slots);
        for (key, value) in keys.iter().copied().zip(values) {
            self.slots[key.index as usize].initialize(key.generation, self.pairs.len() as _);
            self.pairs.push((key, value));
        }
        keys
    }

    #[inline]
    pub fn try_insert(&mut self, key: Key, value: T) -> Result<(), T> {
        let [result] = self.try_insert_n([(key, value)]);
        result
    }

    #[inline]
    pub fn try_insert_n<const N: usize>(&mut self, pairs: [Pair<T>; N]) -> [Result<(), T>; N] {
        insert(pairs, &mut self.pairs, &mut self.last, &mut self.slots)
    }

    #[inline]
    pub fn remove(&mut self, key: Key) -> Option<T> {
        let [value] = self.remove_n([key]);
        value
    }

    #[inline]
    pub fn remove_n<const N: usize>(&mut self, keys: [Key; N]) -> [Option<T>; N] {
        remove(
            keys,
            &mut self.pairs,
            &mut self.slots,
            &mut self.free,
            &mut self.cursor,
        )
    }

    #[inline]
    pub fn reserve(&self, keys: &mut [Key]) {
        reserve(keys, &self.cursor, &self.free, &self.last)
    }

    #[inline]
    pub fn reserve_mut(&mut self, keys: &mut [Key]) {
        reserve_mut(keys, &mut self.cursor, &self.free, &mut self.last)
    }

    #[inline]
    pub fn reserve_n<const N: usize>(&self) -> [Key; N] {
        let mut keys = [Key::NULL; N];
        self.reserve(&mut keys);
        keys
    }

    #[inline]
    pub fn reserve_n_mut<const N: usize>(&mut self) -> [Key; N] {
        let mut keys = [Key::NULL; N];
        self.reserve_mut(&mut keys);
        keys
    }

    /// Releases reserved keys. Use only with keys that are valid (i.e. acquired through [`Self::reserve`]) and that have
    /// not been inserted, otherwise there may be key collisions on later [`Self::reserve`] or [`Self::insert`] calls.
    pub fn release(&mut self, keys: impl IntoIterator<Item = Key>) {
        let cursor = self.cursor.get_mut();
        self.free.truncate((*cursor).max(0) as usize);
        self.free
            .extend(keys.into_iter().filter_map(Key::increment));
        *cursor = self.free.len() as _;
    }

    #[inline]
    pub fn scope<U, S: FnOnce(Pairs<T>, Defer<T>) -> U>(&mut self, scope: S) -> U {
        let (pairs, defer) = self.defer();
        let value = scope(pairs, defer);
        self.resolve();
        value
    }

    #[inline]
    pub fn fork<'a, L: Item, R: Item>(
        &'a mut self,
        fork: impl Fn(Key, &'a mut T) -> (L, R) + Copy,
    ) -> (
        Fork<'a, Pair<T>, impl Fn(&'a mut Pair<T>) -> L>,
        Fork<'a, Pair<T>, impl Fn(&'a mut Pair<T>) -> R>,
    ) {
        fork::fork(&mut self.pairs, move |pair| fork(pair.0, &mut pair.1))
    }

    #[inline]
    pub fn defer(&mut self) -> (Pairs<T>, Defer<T>) {
        let pairs = Pairs {
            slots: &mut self.slots,
            pairs: &mut self.pairs,
        };
        let defer = Defer {
            cursor: &self.cursor,
            free: &self.free,
            last: &self.last,
            inserts: &self.inserts,
            removes: &self.removes,
        };
        (pairs, defer)
    }

    pub fn resolve(&mut self) {
        for pair in self.inserts.get_mut().drain(..) {
            // TODO: Batch?
            let _ = insert([pair], &mut self.pairs, &mut self.last, &mut self.slots);
        }
        for key in self.removes.get_mut().drain() {
            // TODO: Batch?
            let _ = remove(
                [key],
                &mut self.pairs,
                &mut self.slots,
                &mut self.free,
                &mut self.cursor,
            );
        }
    }
}

impl<T: Send + Sync> Armoire<T> {
    #[inline]
    pub fn par_iter(&self) -> impl IndexedParallelIterator<Item = (Key, &T)> {
        self.pairs.par_iter().map(|(key, value)| (*key, value))
    }

    #[inline]
    pub fn par_iter_mut(&mut self) -> impl IndexedParallelIterator<Item = (Key, &mut T)> {
        self.pairs.par_iter_mut().map(|(key, value)| (*key, value))
    }
}

impl<T> Default for Armoire<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[inline]
fn index(key: Key, slots: &[Slot]) -> Option<usize> {
    let slot = slots.get(key.index as usize)?;
    if slot.generation == key.generation {
        Some(slot.index as usize)
    } else {
        None
    }
}

#[inline]
fn ensure(last: &mut AtomicU32, slots: &mut Vec<Slot>) {
    let last = *last.get_mut();
    slots.resize(last as _, Slot::ZERO);
}

fn reserve(keys: &mut [Key], cursor: &AtomicI64, free: &[Key], last: &AtomicU32) {
    if keys.is_empty() {
        return;
    }

    let cursor = cursor.fetch_sub(keys.len() as _, Ordering::Relaxed);
    let keys = if cursor > 0 {
        let end = cursor as usize;
        if end >= keys.len() {
            keys.copy_from_slice(&free[end - keys.len()..end]);
            return;
        } else {
            keys[..end].copy_from_slice(&free[..end]);
            &mut keys[end..]
        }
    } else {
        keys
    };

    let last = last.fetch_add(keys.len() as _, Ordering::Relaxed);
    assert!(last <= u32::MAX - keys.len() as u32);
    for (i, key) in keys.iter_mut().enumerate() {
        *key = Key::new(0, last.saturating_add(i as _));
    }
}

fn reserve_mut(keys: &mut [Key], cursor: &mut AtomicI64, free: &[Key], last: &mut AtomicU32) {
    if keys.is_empty() {
        return;
    }

    let cursor = sub(cursor.get_mut(), keys.len() as _);
    let keys = if cursor > 0 {
        let end = cursor as usize;
        if end >= keys.len() {
            keys.copy_from_slice(&free[end - keys.len()..end]);
            return;
        } else {
            keys[..end].copy_from_slice(&free[..end]);
            &mut keys[end..]
        }
    } else {
        keys
    };

    let last = add(last.get_mut(), keys.len() as _);
    assert!(last <= u32::MAX - keys.len() as u32);
    for (i, key) in keys.iter_mut().enumerate() {
        *key = Key::new(0, last.wrapping_add(i as _));
    }
}

fn insert<T, const N: usize>(
    inserts: [Pair<T>; N],
    pairs: &mut Vec<Pair<T>>,
    last: &mut AtomicU32,
    slots: &mut Vec<Slot>,
) -> [Result<(), T>; N] {
    ensure(last, slots);
    inserts.map(|(key, value)| {
        if let Some(slot) = slots.get_mut(key.index as usize) {
            if slot.initialize(key.generation, pairs.len() as _) {
                pairs.push((key, value));
                return Ok(());
            }
        }
        Err(value)
    })
}

fn remove<T, const N: usize>(
    removes: [Key; N],
    pairs: &mut Vec<Pair<T>>,
    slots: &mut [Slot],
    free: &mut Vec<Key>,
    cursor: &mut AtomicI64,
) -> [Option<T>; N] {
    let cursor = cursor.get_mut();
    free.truncate((*cursor).max(0) as usize);
    let values = removes.map(|key| {
        let slot = slots.get_mut(key.index as usize)?;
        if let Some(index) = slot.release(key.generation) {
            let pair = pairs.swap_remove(index as _);
            debug_assert_eq!(pair.0, key);

            if let Some((key, _)) = pairs.get(index as usize) {
                slots[key.index as usize].update(index);
            }

            if let Some(key) = key.increment() {
                free.push(key);
            }

            Some(pair.1)
        } else {
            None
        }
    });
    *cursor = free.len() as _;
    values
}

#[inline]
fn add(target: &mut u32, value: u32) -> u32 {
    let source = *target;
    *target = source.wrapping_add(value);
    source
}

#[inline]
fn sub(target: &mut i64, value: i64) -> i64 {
    let source = *target;
    *target = source.wrapping_sub(value);
    source
}
