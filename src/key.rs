use std::sync::atomic::{AtomicI64, AtomicU32, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Key {
    generation: u32,
    index: u32,
}

pub(crate) struct Keys {
    pub slots: (Vec<Slot>, AtomicU32),
    pub free: (Vec<Key>, AtomicI64),
}

#[derive(Default)]
pub(crate) struct Slot {
    pub generation: u32,
}

impl Key {
    pub(crate) const ZERO: Key = Key::new(0, 0);

    pub(crate) const fn new(generation: u32, index: u32) -> Self {
        Self { generation, index }
    }

    pub(crate) const fn index(&self) -> usize {
        self.index as _
    }

    pub(crate) const fn generation(&self) -> u32 {
        self.generation
    }

    #[inline]
    pub(crate) fn increment(self) -> Option<Key> {
        Some(Key::new(self.generation.checked_add(1)?, self.index))
    }
}

impl Keys {
    pub const fn new() -> Self {
        Self {
            slots: (Vec::new(), AtomicU32::new(0)),
            free: (Vec::new(), AtomicI64::new(0)),
        }
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
}
