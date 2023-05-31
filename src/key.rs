use std::{
    mem::replace,
    sync::atomic::{AtomicI64, AtomicU32, Ordering},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Key {
    generation: u32,
    index: u32,
}

pub(crate) struct Keys {
    slots: (Vec<Slot>, AtomicU32),
    pub free: (Vec<Key>, AtomicI64),
}

#[derive(Default)]
pub(crate) struct Slot {
    generation: u32,
}

impl Key {
    pub(crate) const ZERO: Key = Key::new(0, 0);

    #[inline]
    pub(crate) const fn new(generation: u32, index: u32) -> Self {
        Self { generation, index }
    }

    #[inline]
    pub(crate) const fn index(&self) -> usize {
        self.index as _
    }

    #[inline]
    pub(crate) const fn generation(&self) -> u32 {
        self.generation
    }

    #[inline]
    pub(crate) fn increment(self) -> Option<Key> {
        Some(Key::new(self.generation.checked_add(1)?, self.index))
    }
}

impl Slot {
    #[inline]
    pub(crate) const fn key(&self, index: u32) -> Key {
        Key::new(self.generation, index)
    }

    #[inline]
    pub(crate) const fn is(&self, generation: u32) -> bool {
        self.generation == generation
    }

    #[inline]
    pub(crate) fn set(&mut self, generation: u32) -> u32 {
        replace(&mut self.generation, generation)
    }
}

impl Keys {
    #[inline]
    pub const fn new() -> Self {
        Self {
            slots: (Vec::new(), AtomicU32::new(0)),
            free: (Vec::new(), AtomicI64::new(0)),
        }
    }

    #[inline]
    pub fn key(&self, index: usize) -> Option<Key> {
        Some(self.slots.0.get(index)?.key(index as _))
    }

    #[inline]
    pub fn valid(&self, key: Key) -> bool {
        matches!(self.get(key), Some(slot) if slot.is(key.generation()))
    }

    #[inline]
    pub fn set(&mut self, key: Key) -> Option<u32> {
        Some(self.get_mut(key)?.set(key.generation()))
    }

    #[inline]
    pub fn get(&self, key: Key) -> Option<&Slot> {
        self.slots.0.get(key.index())
    }

    #[inline]
    pub fn get_mut(&mut self, key: Key) -> Option<&mut Slot> {
        self.slots.0.get_mut(key.index())
    }

    pub fn reserve(&self, keys: &mut [Key]) {
        let tail = self.free.1.fetch_sub(keys.len() as _, Ordering::Relaxed);
        let keys = if tail > 0 {
            let tail = tail as usize;
            let free = tail.min(keys.len());
            keys[..free].copy_from_slice(&self.free.0[tail - free..tail]);
            &mut keys[free..]
        } else {
            keys
        };

        if !keys.is_empty() {
            let index = self.slots.1.fetch_add(keys.len() as _, Ordering::Relaxed);
            for (i, key) in keys.iter_mut().enumerate() {
                *key = Key::new(0, index.wrapping_add(i as _));
            }
        }
    }

    pub fn reserve_n<const N: usize>(&self) -> [Key; N] {
        let mut keys = [Key::ZERO; N];
        self.reserve(&mut keys);
        keys
    }

    pub fn reserve_mut(&mut self, keys: &mut [Key]) {
        let mut tail = *self.free.1.get_mut();
        let keys = if tail > 0 {
            tail -= keys.len() as i64;
            let tail = tail as usize;
            let free = tail.min(keys.len());
            keys[..free].copy_from_slice(&self.free.0[tail - free..free]);
            &mut keys[free..]
        } else {
            keys
        };

        if !keys.is_empty() {
            let capacity = self.slots.1.get_mut();
            let index = *capacity;
            *capacity = capacity.wrapping_add(keys.len() as _);
            for (i, key) in keys.iter_mut().enumerate() {
                *key = Key::new(0, index.wrapping_add(i as _));
            }
        }
    }

    pub fn reserve_n_mut<const N: usize>(&mut self) -> [Key; N] {
        let mut keys = [Key::ZERO; N];
        self.reserve_mut(&mut keys);
        keys
    }

    pub fn ensure(&mut self) -> usize {
        let capacity = *self.slots.1.get_mut() as usize;
        self.slots.0.resize_with(capacity, Slot::default);
        capacity
    }
}
