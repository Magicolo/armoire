use std::iter::FusedIterator;

pub trait FullIterator: Iterator + DoubleEndedIterator + ExactSizeIterator + FusedIterator {}
impl<I: Iterator + DoubleEndedIterator + ExactSizeIterator + FusedIterator> FullIterator for I {}

pub trait IteratorExtensions: Iterator {
    #[inline]
    fn complete<C: FnMut(Self::Item)>(self, complete: C) -> Complete<Self, C>
    where
        Self: Sized,
    {
        Complete(self, complete)
    }
}
impl<I: Iterator> IteratorExtensions for I {}

pub struct Complete<I: Iterator, C: FnMut(I::Item)>(I, C);

impl<I: Iterator, C: FnMut(I::Item)> Drop for Complete<I, C> {
    #[inline]
    fn drop(&mut self) {
        for item in self.0.by_ref() {
            self.1(item);
        }
    }
}

impl<I: Iterator, C: FnMut(I::Item)> Iterator for Complete<I, C> {
    type Item = I::Item;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl<I: DoubleEndedIterator, C: FnMut(I::Item)> DoubleEndedIterator for Complete<I, C> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }
}

impl<I: ExactSizeIterator, C: FnMut(I::Item)> ExactSizeIterator for Complete<I, C> {
    #[inline]
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl<I: FusedIterator, C: FnMut(I::Item)> FusedIterator for Complete<I, C> {}
