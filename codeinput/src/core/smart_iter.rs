#![allow(dead_code)]

use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

pub(crate) trait SmartIter<T: Send + Sync> {
    fn smart_iter(&self, n: usize) -> SmartIterator<'_, T>;
}

impl<T: Send + Sync> SmartIter<T> for [T] {
    fn smart_iter(&self, n: usize) -> SmartIterator<'_, T> {
        if self.len() <= n {
            SmartIterator::Sequential(self.iter())
        } else {
            SmartIterator::Parallel(self.par_iter())
        }
    }
}

pub(crate) enum SmartIterator<'a, T: Send + Sync> {
    Sequential(std::slice::Iter<'a, T>),
    Parallel(rayon::slice::Iter<'a, T>),
}

pub(crate) enum SmartFilterMap<'a, T: Send + Sync, F> {
    Parallel(rayon::iter::FilterMap<rayon::slice::Iter<'a, T>, F>),
    Sequential(std::iter::FilterMap<std::slice::Iter<'a, T>, F>),
}

impl<'a, T: Send + Sync> SmartIterator<'a, T> {
    pub(crate) fn filter_map<B: Send + Sync, F>(self, f: F) -> SmartFilterMap<'a, T, F>
    where
        F: Fn(&'a T) -> Option<B> + Send + Sync,
    {
        match self {
            SmartIterator::Parallel(iter) => SmartFilterMap::Parallel(iter.filter_map(f)),
            SmartIterator::Sequential(iter) => SmartFilterMap::Sequential(iter.filter_map(f)),
        }
    }
}

impl<'a, T: Send + Sync, B: Send + Sync, F> SmartFilterMap<'a, T, F>
where
    F: Fn(&'a T) -> Option<B> + Send + Sync,
{
    pub(crate) fn collect(self) -> Vec<B> {
        match self {
            SmartFilterMap::Parallel(iter) => iter.collect(),
            SmartFilterMap::Sequential(iter) => iter.collect(),
        }
    }
}
