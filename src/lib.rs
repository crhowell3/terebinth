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
}
