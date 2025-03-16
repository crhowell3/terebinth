//     terebinth - lightweight programming language
//     Copyright (C) 2024 Cameron Howell
//
//     Licensed under the MIT License

#[macro_export]
macro_rules! idx {
    ($name:ident) => {
        #[derive(Debug, Clone, Eq, PartialEq, Hash, Copy)]
        pub struct $name {
            idx: usize,
        }

        impl Idx for $name {
            fn as_idx(&self) -> usize {
                self.idx
            }

            fn new(idx: usize) -> Self {
                Self { idx }
            }
        }
    };
}

pub trait Idx {
    fn as_idx(&self) -> usize;
    fn new(idx: usize) -> Self;
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct IdxVec<Index, T>
where
    Index: Idx,
{
    vec: Vec<T>,
    _marker: std::marker::PhantomData<Index>,
}

impl<Index, T> IdxVec<Index, T>
where
    Index: Idx,
{
    #[must_use]
    pub fn new() -> Self {
        Self {
            vec: vec![],
            _marker: std::marker::PhantomData,
        }
    }

    pub fn push(&mut self, value: T) -> Index {
        let next_idx = self.vec.len();
        self.vec.push(value);
        Index::new(next_idx)
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.vec.iter()
    }

    pub fn indexed_iter(&self) -> impl Iterator<Item = (Index, &T)> {
        self.vec
            .iter()
            .enumerate()
            .map(|(index, value)| (Index::new(index), value))
    }

    #[must_use]
    pub fn cloned_indices(&self) -> Vec<Index> {
        self.vec
            .iter()
            .enumerate()
            .map(|(index, _)| Index::new(index))
            .collect()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.vec.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.vec.is_empty()
    }

    pub fn get(&self, index: Index) -> &T {
        &self[index]
    }
}

impl<Index, T> Default for IdxVec<Index, T>
where
    Index: Idx,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<Index, T> std::ops::Index<Index> for IdxVec<Index, T>
where
    Index: Idx,
{
    type Output = T;

    fn index(&self, index: Index) -> &T {
        &self.vec[index.as_idx()]
    }
}

impl<Index, T> std::ops::IndexMut<Index> for IdxVec<Index, T>
where
    Index: Idx,
{
    fn index_mut(&mut self, index: Index) -> &mut T {
        &mut self.vec[index.as_idx()]
    }
}
