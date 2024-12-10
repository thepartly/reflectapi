#[derive(Debug, PartialEq, Eq)]
pub struct IndexMap<K, V> {
    _k: std::marker::PhantomData<K>,
    _v: std::marker::PhantomData<V>,
}

pub type IndexSet<T> = IndexMap<T, ()>;
