use std::sync::atomic::{AtomicI64, AtomicU32, Ordering};

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
    row: u32,
}

impl Key {
    pub const ZERO: Key = Key::new(0, 0);

    #[inline]
    pub const fn new(generation: u32, index: u32) -> Self {
        Self { generation, index }
    }

    #[inline]
    pub const fn index(&self) -> u32 {
        self.index
    }

    #[inline]
    pub const fn generation(&self) -> u32 {
        self.generation
    }

    #[inline]
    pub(crate) fn increment(self) -> Option<Key> {
        Some(Key::new(self.generation.checked_add(1)?, self.index))
    }
}

impl Slot {
    #[inline]
    pub const fn is(&self, generation: u32) -> bool {
        self.generation == generation
    }

    #[inline]
    pub const fn row(&self) -> u32 {
        self.row
    }

    #[inline]
    pub fn initialize(&mut self, generation: u32, row: u32) {
        self.generation = generation;
        self.update(row);
    }

    #[inline]
    pub fn update(&mut self, row: u32) {
        self.row = row;
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
    pub fn row(&self, key: Key) -> Option<u32> {
        let slot = self.get(key)?;
        if slot.is(key.generation()) {
            Some(slot.row())
        } else {
            None
        }
    }

    #[inline]
    pub fn initialize(&mut self, key: Key, row: u32) -> bool {
        if let Some(slot) = self.get_mut(key) {
            slot.initialize(key.generation(), row);
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn get(&self, key: Key) -> Option<&Slot> {
        self.slots.0.get(key.index() as usize)
    }

    #[inline]
    pub fn get_mut(&mut self, key: Key) -> Option<&mut Slot> {
        self.slots.0.get_mut(key.index() as usize)
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
                *key = Key::new(1, index.wrapping_add(i as _));
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
                *key = Key::new(1, index.wrapping_add(i as _));
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
