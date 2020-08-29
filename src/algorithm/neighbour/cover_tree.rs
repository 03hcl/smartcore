//! # Cover Tree
//!
//! The Cover Tree data structure is specifically designed to facilitate the speed-up of a nearest neighbor search, see [KNN algorithms](../index.html).
//!
//! ```
//! use smartcore::algorithm::neighbour::cover_tree::*;
//! use smartcore::math::distance::Distance;
//!
//! struct SimpleDistance {} // Our distance function
//!
//! impl Distance<i32, f64> for SimpleDistance {
//!   fn distance(&self, a: &i32, b: &i32) -> f64 { // simple simmetrical scalar distance
//!     (a - b).abs() as f64
//!   }
//! }
//!
//! let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9]; // data points
//!
//! let mut tree = CoverTree::new(data, SimpleDistance {});
//!
//! tree.find(&5, 3); // find 3 knn points from 5
//!
//! ```
use core::hash::{Hash, Hasher};
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::iter::FromIterator;

use serde::{Deserialize, Serialize};

use crate::algorithm::sort::heap_select::HeapSelect;
use crate::math::distance::Distance;
use crate::math::num::FloatExt;

/// Implements Cover Tree algorithm
#[derive(Serialize, Deserialize, Debug)]
pub struct CoverTree<T, F: FloatExt, D: Distance<T, F>> {
    base: F,
    max_level: i8,
    min_level: i8,
    distance: D,
    nodes: Vec<Node<T>>,
}

impl<T: Debug, F: FloatExt, D: Distance<T, F>> CoverTree<T, F, D> {
    /// Construct a cover tree.
    /// * `data` - vector of data points to search for.
    /// * `distance` - distance metric to use for searching. This function should extend [`Distance`](../algorithm/neighbour/index.html) interface.
    pub fn new(mut data: Vec<T>, distance: D) -> CoverTree<T, F, D> {
        let mut tree = CoverTree {
            base: F::two(),
            max_level: 100,
            min_level: 100,
            distance: distance,
            nodes: Vec::new(),
        };

        let p = tree.new_node(None, data.remove(0));
        tree.construct(p, data, Vec::new(), 10);

        tree
    }

    /// Insert new data point into the cover tree.
    /// * `p` - new data points.    
    pub fn insert(&mut self, p: T) {
        if self.nodes.is_empty() {
            self.new_node(None, p);
        } else {
            let mut parent: Option<NodeId> = Option::None;
            let mut p_i = 0;
            let mut qi_p_ds = vec![(self.root(), self.distance.distance(&p, &self.root().data))];
            let mut i = self.max_level;
            loop {
                let i_d = self.base.powf(F::from(i).unwrap());
                let q_p_ds = self.get_children_dist(&p, &qi_p_ds, i);
                let d_p_q = self.min_by_distance(&q_p_ds);
                if d_p_q < F::epsilon() {
                    return;
                } else if d_p_q > i_d {
                    break;
                }
                if self.min_by_distance(&qi_p_ds) <= self.base.powf(F::from(i).unwrap()) {
                    parent = q_p_ds.iter().find(|(_, d)| d <= &i_d).map(|(n, _)| n.index);
                    p_i = i;
                }

                qi_p_ds = q_p_ds.into_iter().filter(|(_, d)| d <= &i_d).collect();
                i -= 1;
            }

            let new_node = self.new_node(parent, p);
            self.add_child(parent.unwrap(), new_node, p_i);
            self.min_level = i8::min(self.min_level, p_i - 1);
        }
    }

    fn new_node(&mut self, parent: Option<NodeId>, data: T) -> NodeId {
        let next_index = self.nodes.len();
        let node_id = NodeId { index: next_index };
        self.nodes.push(Node {
            index: node_id,
            data: data,
            parent: parent,
            children: HashMap::new(),
        });
        node_id
    }

    /// Find k nearest neighbors of `p`
    /// * `p` - look for k nearest points to `p`
    /// * `k` - the number of nearest neighbors to return
    pub fn find(&self, p: &T, k: usize) -> Vec<(usize, F)> {
        let mut qi_p_ds = vec![(self.root(), self.distance.distance(&p, &self.root().data))];
        for i in (self.min_level..self.max_level + 1).rev() {
            let i_d = self.base.powf(F::from(i).unwrap());
            let mut q_p_ds = self.get_children_dist(&p, &qi_p_ds, i);
            let d_p_q = self.min_k_by_distance(&mut q_p_ds, k);
            qi_p_ds = q_p_ds
                .into_iter()
                .filter(|(_, d)| d <= &(d_p_q + i_d))
                .collect();
        }
        qi_p_ds.sort_by(|(_, d1), (_, d2)| d1.partial_cmp(d2).unwrap());
        qi_p_ds[..usize::min(qi_p_ds.len(), k)]
            .iter()
            .map(|(n, d)| (n.index.index, *d))
            .collect()
    }

    fn split(
        &self,
        p_id: NodeId,
        r: F,
        s1: &mut Vec<T>,
        s2: Option<&mut Vec<T>>,
    ) -> (Vec<T>, Vec<T>) {
        let mut my_near = (Vec::new(), Vec::new());

        my_near = self.split_remove_s(p_id, r, s1, my_near);

        for s in s2 {
            my_near = self.split_remove_s(p_id, r, s, my_near);
        }

        return my_near;
    }

    fn split_remove_s(
        &self,
        p_id: NodeId,
        r: F,
        s: &mut Vec<T>,
        mut my_near: (Vec<T>, Vec<T>),
    ) -> (Vec<T>, Vec<T>) {
        if s.len() > 0 {
            let p = &self.nodes.get(p_id.index).unwrap().data;
            let mut i = 0;
            while i != s.len() {
                let d = self.distance.distance(p, &s[i]);
                if d <= r {
                    my_near.0.push(s.remove(i));
                } else if d > r && d <= F::two() * r {
                    my_near.1.push(s.remove(i));
                } else {
                    i += 1;
                }
            }
        }

        return my_near;
    }

    fn construct<'b>(
        &mut self,
        p: NodeId,
        mut near: Vec<T>,
        mut far: Vec<T>,
        i: i8,
    ) -> (NodeId, Vec<T>) {
        if near.len() < 1 {
            self.min_level = std::cmp::min(self.min_level, i);
            return (p, far);
        } else {
            let (my, n) = self.split(p, self.base.powf(F::from(i - 1).unwrap()), &mut near, None);
            let (pi, mut near) = self.construct(p, my, n, i - 1);
            while near.len() > 0 {
                let q_data = near.remove(0);
                let nn = self.new_node(Some(p), q_data);
                let (my, n) = self.split(
                    nn,
                    self.base.powf(F::from(i - 1).unwrap()),
                    &mut near,
                    Some(&mut far),
                );
                let (child, mut unused) = self.construct(nn, my, n, i - 1);
                self.add_child(pi, child, i);
                let new_near_far =
                    self.split(p, self.base.powf(F::from(i).unwrap()), &mut unused, None);
                near.extend(new_near_far.0);
                far.extend(new_near_far.1);
            }
            self.min_level = std::cmp::min(self.min_level, i);
            return (pi, far);
        }
    }

    fn add_child(&mut self, parent: NodeId, node: NodeId, i: i8) {
        self.nodes
            .get_mut(parent.index)
            .unwrap()
            .children
            .insert(i, node);
    }

    fn root(&self) -> &Node<T> {
        self.nodes.first().unwrap()
    }

    fn get_children_dist<'b>(
        &'b self,
        p: &T,
        qi_p_ds: &Vec<(&'b Node<T>, F)>,
        i: i8,
    ) -> Vec<(&'b Node<T>, F)> {
        let mut children = Vec::<(&'b Node<T>, F)>::new();

        children.extend(qi_p_ds.iter().cloned());

        let q: Vec<&Node<T>> = qi_p_ds
            .iter()
            .flat_map(|(n, _)| self.get_child(n, i))
            .collect();

        children.extend(
            q.into_iter()
                .map(|n| (n, self.distance.distance(&n.data, &p))),
        );

        children
    }

    fn min_k_by_distance(&self, q_p_ds: &mut Vec<(&Node<T>, F)>, k: usize) -> F {
        let mut heap = HeapSelect::with_capacity(k);
        for (_, d) in q_p_ds {
            heap.add(d);
        }
        heap.sort();
        *heap.get().pop().unwrap()
    }

    fn min_by_distance(&self, q_p_ds: &Vec<(&Node<T>, F)>) -> F {
        q_p_ds
            .into_iter()
            .min_by(|(_, d1), (_, d2)| d1.partial_cmp(d2).unwrap())
            .unwrap()
            .1
    }

    fn get_child(&self, node: &Node<T>, i: i8) -> Option<&Node<T>> {
        node.children
            .get(&i)
            .and_then(|n_id| self.nodes.get(n_id.index))
    }

    #[allow(dead_code)]
    fn check_invariant(
        &self,
        invariant: fn(&CoverTree<T, F, D>, &Vec<&Node<T>>, &Vec<&Node<T>>, i8) -> (),
    ) {
        let mut current_nodes: Vec<&Node<T>> = Vec::new();
        current_nodes.push(self.root());
        for i in (self.min_level..self.max_level + 1).rev() {
            let mut next_nodes: Vec<&Node<T>> = Vec::new();
            next_nodes.extend(current_nodes.iter());
            next_nodes.extend(current_nodes.iter().flat_map(|n| self.get_child(n, i)));
            invariant(self, &current_nodes, &next_nodes, i);
            current_nodes = next_nodes
        }
    }

    #[allow(dead_code)]
    fn nesting_invariant(
        _: &CoverTree<T, F, D>,
        nodes: &Vec<&Node<T>>,
        next_nodes: &Vec<&Node<T>>,
        _: i8,
    ) {
        let nodes_set: HashSet<&Node<T>> = HashSet::from_iter(nodes.into_iter().map(|n| *n));
        let next_nodes_set: HashSet<&Node<T>> =
            HashSet::from_iter(next_nodes.into_iter().map(|n| *n));
        for n in nodes_set.iter() {
            assert!(next_nodes_set.contains(n), "Nesting invariant of the cover tree is not satisfied. Set of nodes [{:?}] is not a subset of [{:?}]", nodes_set, next_nodes_set);
        }
    }

    #[allow(dead_code)]
    fn covering_tree(
        tree: &CoverTree<T, F, D>,
        nodes: &Vec<&Node<T>>,
        next_nodes: &Vec<&Node<T>>,
        i: i8,
    ) {
        let mut p_selected: Vec<&Node<T>> = Vec::new();
        for p in next_nodes {
            for q in nodes {
                if tree.distance.distance(&p.data, &q.data) <= tree.base.powf(F::from(i).unwrap()) {
                    p_selected.push(*p);
                }
            }
            let c = p_selected
                .iter()
                .filter(|q| p.parent.map(|p| q.index == p).unwrap_or(false))
                .count();
            assert!(c <= 1);
        }
    }

    #[allow(dead_code)]
    fn separation(tree: &CoverTree<T, F, D>, nodes: &Vec<&Node<T>>, _: &Vec<&Node<T>>, i: i8) {
        for p in nodes {
            for q in nodes {
                if p != q {
                    assert!(
                        tree.distance.distance(&p.data, &q.data)
                            > tree.base.powf(F::from(i).unwrap())
                    );
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
struct NodeId {
    index: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct Node<T> {
    index: NodeId,
    data: T,
    children: HashMap<i8, NodeId>,
    parent: Option<NodeId>,
}

impl<T> PartialEq for Node<T> {
    fn eq(&self, other: &Self) -> bool {
        self.index.index == other.index.index
    }
}

impl<T> Eq for Node<T> {}

impl<T> Hash for Node<T> {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        state.write_usize(self.index.index);
        state.finish();
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    struct SimpleDistance {}

    impl Distance<i32, f64> for SimpleDistance {
        fn distance(&self, a: &i32, b: &i32) -> f64 {
            (a - b).abs() as f64
        }
    }

    #[test]
    fn cover_tree_test() {
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];

        let mut tree = CoverTree::new(data, SimpleDistance {});
        for d in vec![10, 11, 12, 13, 14, 15, 16, 17, 18, 19] {
            tree.insert(d);
        }

        let mut nearest_3_to_5 = tree.find(&5, 3);
        nearest_3_to_5.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        let nearest_3_to_5_indexes: Vec<usize> = nearest_3_to_5.iter().map(|v| v.0).collect();
        assert_eq!(vec!(4, 5, 3), nearest_3_to_5_indexes);

        let mut nearest_3_to_15 = tree.find(&15, 3);
        nearest_3_to_15.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        let nearest_3_to_15_indexes: Vec<usize> = nearest_3_to_15.iter().map(|v| v.0).collect();
        assert_eq!(vec!(14, 13, 15), nearest_3_to_15_indexes);

        assert_eq!(-1, tree.min_level);
        assert_eq!(100, tree.max_level);
    }

    #[test]
    fn test_invariants() {
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];

        let tree = CoverTree::new(data, SimpleDistance {});
        tree.check_invariant(CoverTree::nesting_invariant);
        tree.check_invariant(CoverTree::covering_tree);
        tree.check_invariant(CoverTree::separation);
    }
}
