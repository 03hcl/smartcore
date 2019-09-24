pub mod cover_tree;
pub mod linear_search;

pub enum KNNAlgorithmName {
    CoverTree,
    LinearSearch,
}

pub trait KNNAlgorithm<T>{
    fn find(&self, from: &T, k: usize) -> Vec<usize>;
}