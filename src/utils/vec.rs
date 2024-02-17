pub trait UniqueVec<T> {
    fn unique_by_key<K, F>(&mut self, f: F)
    where
        F: FnMut(&T) -> K,
        K: PartialEq + Ord;
}

impl<T> UniqueVec<T> for Vec<T> {
    fn unique_by_key<K, F>(&mut self, mut f: F)
    where
        F: FnMut(&T) -> K,
        K: PartialEq + Ord,
    {
        self.sort_by_key(|x| f(x));
        self.dedup_by_key(|x| f(x));
    }
}
