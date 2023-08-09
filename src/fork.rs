use crate::{utility::FullIterator, Key};
use rayon::prelude::{IndexedParallelIterator, IntoParallelRefMutIterator, ParallelIterator};
use std::{marker::PhantomData, slice::from_raw_parts_mut};

pub struct Fork<'a, S, F>(*mut S, usize, F, PhantomData<&'a mut [S]>);

pub trait Item {
    type Read;
    type Write;
    fn read(self) -> Self::Read;
    fn write(self) -> Self::Write;
}

impl<'a, T> Item for &'a T {
    type Read = &'a T;
    type Write = &'a T;

    fn read(self) -> Self::Read {
        self
    }

    fn write(self) -> Self::Write {
        self
    }
}

impl<'a, T> Item for &'a mut T {
    type Read = &'a T;
    type Write = &'a mut T;

    fn read(self) -> Self::Read {
        self
    }

    fn write(self) -> Self::Write {
        self
    }
}

impl<I1: Item, I2: Item> Item for (I1, I2) {
    type Read = (I1::Read, I2::Read);
    type Write = (I1::Write, I2::Write);

    fn read(self) -> Self::Read {
        (self.0.read(), self.1.read())
    }

    fn write(self) -> Self::Write {
        (self.0.write(), self.1.write())
    }
}

impl<I1: Item, I2: Item, I3: Item> Item for (I1, I2, I3) {
    type Read = (I1::Read, I2::Read, I3::Read);
    type Write = (I1::Write, I2::Write, I3::Write);

    fn read(self) -> Self::Read {
        (self.0.read(), self.1.read(), self.2.read())
    }

    fn write(self) -> Self::Write {
        (self.0.write(), self.1.write(), self.2.write())
    }
}

impl<I: Item, const N: usize> Item for [I; N] {
    type Read = [I::Read; N];
    type Write = [I::Write; N];

    fn read(self) -> Self::Read {
        self.map(I::read)
    }

    fn write(self) -> Self::Write {
        self.map(I::write)
    }
}

impl<I: Item> Item for Option<I> {
    type Read = Option<I::Read>;
    type Write = Option<I::Write>;

    fn read(self) -> Self::Read {
        self.map(I::read)
    }

    fn write(self) -> Self::Write {
        self.map(I::write)
    }
}

impl Item for Key {
    type Read = Self;
    type Write = Self;

    fn read(self) -> Self::Read {
        self
    }
    fn write(self) -> Self::Write {
        self
    }
}

#[inline]
pub fn fork<'a, T, L: Item, R: Item>(
    slice: &'a mut [T],
    fork: impl Fn(&'a mut T) -> (L, R) + Copy,
) -> (
    Fork<'a, T, impl Fn(&'a mut T) -> L>,
    Fork<'a, T, impl Fn(&'a mut T) -> R>,
) {
    let data = slice.as_mut_ptr();
    let count = slice.len();
    let left = Fork(data, count, move |item| fork(item).0, PhantomData);
    let right = Fork(data, count, move |item| fork(item).1, PhantomData);
    (left, right)
}

impl<'a, S: 'static, T: Item, F: Fn(&'a mut S) -> T> Fork<'a, S, F> {
    pub fn iter_mut(&mut self) -> impl FullIterator<Item = T::Write> + '_ {
        unsafe { from_raw_parts_mut(self.0, self.1) }
            .iter_mut()
            .map(|item| self.2(item).write())
    }

    pub fn iter(&self) -> impl FullIterator<Item = T::Read> + '_ {
        unsafe { from_raw_parts_mut(self.0, self.1) }
            .iter_mut()
            .map(|item| self.2(item).read())
    }
}

unsafe impl<S: Sync, F: Sync> Sync for Fork<'_, S, F> {}

impl<'a, S: Send + Sync + 'static, T: Item, F: Fn(&'a mut S) -> T + Sync> Fork<'a, S, F>
where
    T::Read: Send,
    T::Write: Send,
{
    pub fn par_iter_mut(&mut self) -> impl IndexedParallelIterator<Item = T::Write> + '_ {
        unsafe { from_raw_parts_mut(self.0, self.1) }
            .par_iter_mut()
            .map(|item| self.2(item).write())
    }

    pub fn par_iter(&self) -> impl IndexedParallelIterator<Item = T::Read> + '_ {
        unsafe { from_raw_parts_mut(self.0, self.1) }
            .par_iter_mut()
            .map(|item| self.2(item).read())
    }
}
