//! # Support Vector Classifier.
//!
//! Example
//!
//! ```
//! use smartcore::linalg::naive::dense_matrix::*;
//! use smartcore::linear::linear_regression::*;
//! use smartcore::svm::LinearKernel;
//! use smartcore::svm::svc::{SVC, SVCParameters};
//!
//! // Iris dataset
//! let x = DenseMatrix::from_2d_array(&[
//!            &[5.1, 3.5, 1.4, 0.2],
//!            &[4.9, 3.0, 1.4, 0.2],
//!            &[4.7, 3.2, 1.3, 0.2],
//!            &[4.6, 3.1, 1.5, 0.2],
//!            &[5.0, 3.6, 1.4, 0.2],
//!            &[5.4, 3.9, 1.7, 0.4],
//!            &[4.6, 3.4, 1.4, 0.3],
//!            &[5.0, 3.4, 1.5, 0.2],
//!            &[4.4, 2.9, 1.4, 0.2],
//!            &[4.9, 3.1, 1.5, 0.1],
//!            &[7.0, 3.2, 4.7, 1.4],
//!            &[6.4, 3.2, 4.5, 1.5],
//!            &[6.9, 3.1, 4.9, 1.5],
//!            &[5.5, 2.3, 4.0, 1.3],
//!            &[6.5, 2.8, 4.6, 1.5],
//!            &[5.7, 2.8, 4.5, 1.3],
//!            &[6.3, 3.3, 4.7, 1.6],
//!            &[4.9, 2.4, 3.3, 1.0],
//!            &[6.6, 2.9, 4.6, 1.3],
//!            &[5.2, 2.7, 3.9, 1.4],
//!         ]);
//! let y = vec![ -1., -1., -1., -1., -1., -1., -1., -1.,
//!            1., 1., 1., 1., 1., 1., 1., 1., 1., 1., 1., 1.];
//!
//! let svr = SVC::fit(&x, &y,
//!             LinearKernel {},
//!             SVCParameters {
//!                 epoch: 2,
//!                 c: 200.0,
//!                 tol: 1e-3,
//!             }).unwrap();
//!
//! let y_hat = svr.predict(&x).unwrap();
//! ```
//!
//! ## References:
//!
//! * ["Support Vector Machines" Kowalczyk A., 2017](https://www.svm-tutorial.com/2017/10/support-vector-machines-succinctly-released/)
//! * ["Fast Kernel Classifiers with Online and Active Learning", Bordes A., Ertekin S., Weston J., Bottou L., 2005](https://www.jmlr.org/papers/volume6/bordes05a/bordes05a.pdf)

use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::marker::PhantomData;

use rand::seq::SliceRandom;

use serde::{Deserialize, Serialize};

use crate::error::Failed;
use crate::linalg::BaseVector;
use crate::linalg::Matrix;
use crate::math::num::RealNumber;
use crate::svm::Kernel;

#[derive(Serialize, Deserialize, Debug)]

/// SVC Parameters
pub struct SVCParameters<T: RealNumber> {
    /// Number of epochs
    pub epoch: usize,
    /// Regularization parameter.
    pub c: T,
    /// Tolerance for stopping criterion
    pub tol: T,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(bound(
    serialize = "M::RowVector: Serialize, K: Serialize, T: Serialize",
    deserialize = "M::RowVector: Deserialize<'de>, K: Deserialize<'de>, T: Deserialize<'de>",
))]
/// Support Vector Classifier
pub struct SVC<T: RealNumber, M: Matrix<T>, K: Kernel<T, M::RowVector>> {
    kernel: K,
    instances: Vec<M::RowVector>,
    w: Vec<T>,
    b: T,
}

#[derive(Serialize, Deserialize, Debug)]
struct SupportVector<T: RealNumber, V: BaseVector<T>> {
    index: usize,
    x: V,
    alpha: T,
    grad: T,
    cmin: T,
    cmax: T,
    k: T,
}

struct Cache<'a, T: RealNumber, M: Matrix<T>, K: Kernel<T, M::RowVector>> {
    kernel: &'a K,
    data: HashMap<(usize, usize), T>,
    phantom: PhantomData<M>,
}

struct Optimizer<'a, T: RealNumber, M: Matrix<T>, K: Kernel<T, M::RowVector>> {
    x: &'a M,
    y: &'a M::RowVector,
    parameters: &'a SVCParameters<T>,
    svmin: usize,
    svmax: usize,
    gmin: T,
    gmax: T,
    tau: T,
    sv: Vec<SupportVector<T, M::RowVector>>,
    kernel: &'a K,
    recalculate_minmax_grad: bool,
}

impl<T: RealNumber> Default for SVCParameters<T> {
    fn default() -> Self {
        SVCParameters {
            epoch: 2,
            c: T::one(),
            tol: T::from_f64(1e-3).unwrap(),
        }
    }
}

impl<T: RealNumber, M: Matrix<T>, K: Kernel<T, M::RowVector>> SVC<T, M, K> {
    /// Fits SVC to your data.
    /// * `x` - _NxM_ matrix with _N_ observations and _M_ features in each observation.
    /// * `y` - class labels
    /// * `kernel` - the kernel function
    /// * `parameters` - optional parameters, use `Default::default()` to set parameters to default values.
    pub fn fit(
        x: &M,
        y: &M::RowVector,
        kernel: K,
        parameters: SVCParameters<T>,
    ) -> Result<SVC<T, M, K>, Failed> {
        let (n, _) = x.shape();

        if n != y.len() {
            return Err(Failed::fit(&format!(
                "Number of rows of X doesn't match number of rows of Y"
            )));
        }

        let optimizer = Optimizer::new(x, y, &kernel, &parameters);

        let (support_vectors, weight, b) = optimizer.optimize();

        Ok(SVC {
            kernel: kernel,
            instances: support_vectors,
            w: weight,
            b: b,
        })
    }

    /// Predicts estimated class labels from `x`
    /// * `x` - _KxM_ data where _K_ is number of observations and _M_ is number of features.
    pub fn predict(&self, x: &M) -> Result<M::RowVector, Failed> {
        let (n, _) = x.shape();

        let mut y_hat = M::RowVector::zeros(n);

        for i in 0..n {
            y_hat.set(i, self.predict_for_row(x.get_row(i)));
        }

        Ok(y_hat)
    }

    fn predict_for_row(&self, x: M::RowVector) -> T {
        let mut f = self.b;

        for i in 0..self.instances.len() {
            f += self.w[i] * self.kernel.apply(&x, &self.instances[i]);
        }

        if f > T::zero() {
            T::one()
        } else {
            -T::one()
        }
    }
}

impl<T: RealNumber, M: Matrix<T>, K: Kernel<T, M::RowVector>> PartialEq for SVC<T, M, K> {
    fn eq(&self, other: &Self) -> bool {
        if self.b != other.b
            || self.w.len() != other.w.len()
            || self.instances.len() != other.instances.len()
        {
            return false;
        } else {
            for i in 0..self.w.len() {
                if (self.w[i] - other.w[i]).abs() > T::epsilon() {
                    return false;
                }
            }
            for i in 0..self.instances.len() {
                if !self.instances[i].approximate_eq(&other.instances[i], T::epsilon()) {
                    return false;
                }
            }
            return true;
        }
    }
}

impl<T: RealNumber, V: BaseVector<T>> SupportVector<T, V> {
    fn new<K: Kernel<T, V>>(i: usize, x: V, y: T, g: T, c: T, k: &K) -> SupportVector<T, V> {
        let k_v = k.apply(&x, &x);
        let (cmin, cmax) = if y > T::zero() {
            (T::zero(), c)
        } else {
            (-c, T::zero())
        };
        SupportVector {
            index: i,
            x: x,
            grad: g,
            k: k_v,
            alpha: T::zero(),
            cmin: cmin,
            cmax: cmax,
        }
    }
}

impl<'a, T: RealNumber, M: Matrix<T>, K: Kernel<T, M::RowVector>> Cache<'a, T, M, K> {
    fn new(kernel: &'a K) -> Cache<'a, T, M, K> {
        Cache {
            kernel: kernel,
            data: HashMap::new(),
            phantom: PhantomData,
        }
    }

    fn get(&mut self, i: &SupportVector<T, M::RowVector>, j: &SupportVector<T, M::RowVector>) -> T {
        let idx_i = i.index;
        let idx_j = j.index;
        if !self.data.contains_key(&(idx_i, idx_j)) {
            let v = self.kernel.apply(&i.x, &j.x);
            self.data.insert((idx_i, idx_j), v);
        }
        *self.data.get(&(idx_i, idx_j)).unwrap()
    }

    fn insert(&mut self, key: (usize, usize), value: T) {
        self.data.insert(key, value);
    }

    fn drop(&mut self, idxs_to_drop: HashSet<usize>) {
        self.data.retain(|k, _| !idxs_to_drop.contains(&k.0));
    }
}

impl<'a, T: RealNumber, M: Matrix<T>, K: Kernel<T, M::RowVector>> Optimizer<'a, T, M, K> {
    fn new(
        x: &'a M,
        y: &'a M::RowVector,
        kernel: &'a K,
        parameters: &'a SVCParameters<T>,
    ) -> Optimizer<'a, T, M, K> {
        let (n, _) = x.shape();

        Optimizer {
            x: x,
            y: y,
            parameters: &parameters,
            svmin: 0,
            svmax: 0,
            gmin: T::max_value(),
            gmax: T::min_value(),
            tau: T::from_f64(1e-12).unwrap(),
            sv: Vec::with_capacity(n),
            kernel: kernel,
            recalculate_minmax_grad: true,
        }
    }

    fn optimize(mut self) -> (Vec<M::RowVector>, Vec<T>, T) {
        let (n, _) = self.x.shape();

        let mut cache = Cache::new(self.kernel);

        self.initialize(&mut cache);

        let tol = self.parameters.tol;
        let good_enough = T::from_i32(1000).unwrap();

        for _ in 0..self.parameters.epoch {
            for i in Self::permutate(n) {
                self.process(i, self.x.get_row(i), self.y.get(i), &mut cache);
                loop {
                    self.reprocess(tol, &mut cache);
                    self.find_min_max_gradient();
                    if self.gmax - self.gmin < good_enough {
                        break;
                    }
                }
            }
        }

        self.finish(&mut cache);

        let mut support_vectors: Vec<M::RowVector> = Vec::new();
        let mut w: Vec<T> = Vec::new();

        let b = (self.gmax + self.gmin) / T::two();

        for v in self.sv {
            support_vectors.push(v.x);
            w.push(v.alpha);
        }

        (support_vectors, w, b)
    }

    fn initialize(&mut self, cache: &mut Cache<T, M, K>) {
        let (n, _) = self.x.shape();
        let few = 5;
        let mut cp = 0;
        let mut cn = 0;

        for i in Self::permutate(n) {
            if self.y.get(i) == T::one() && cp < few {
                if self.process(i, self.x.get_row(i), self.y.get(i), cache) {
                    cp += 1;
                }
            } else if self.y.get(i) == -T::one() && cn < few {
                if self.process(i, self.x.get_row(i), self.y.get(i), cache) {
                    cn += 1;
                }
            }

            if cp >= few && cn >= few {
                break;
            }
        }
    }

    fn process(&mut self, i: usize, x: M::RowVector, y: T, cache: &mut Cache<T, M, K>) -> bool {
        for j in 0..self.sv.len() {
            if self.sv[j].index == i {
                return true;
            }
        }

        let mut g = y;

        let mut cache_values: Vec<((usize, usize), T)> = Vec::new();

        for v in self.sv.iter() {
            let k = self.kernel.apply(&v.x, &x);
            cache_values.push(((i, v.index), k));
            g -= v.alpha * k;
        }

        self.find_min_max_gradient();

        if self.gmin < self.gmax {
            if (y > T::zero() && g < self.gmin) || (y < T::zero() && g > self.gmax) {
                return false;
            }
        }

        for v in cache_values {
            cache.insert(v.0, v.1);
        }

        self.sv.insert(
            0,
            SupportVector::new(i, x, y, g, self.parameters.c, self.kernel),
        );

        if y > T::zero() {
            self.smo(None, Some(0), T::zero(), cache);
        } else {
            self.smo(Some(0), None, T::zero(), cache);
        }

        true
    }

    fn reprocess(&mut self, tol: T, cache: &mut Cache<T, M, K>) -> bool {
        let status = self.smo(None, None, tol, cache);
        self.clean(cache);
        status
    }

    fn finish(&mut self, cache: &mut Cache<T, M, K>) {
        let mut max_iter = self.sv.len();

        while self.smo(None, None, self.parameters.tol, cache) && max_iter > 0 {
            max_iter -= 1;
        }

        self.clean(cache);
    }

    fn find_min_max_gradient(&mut self) {
        if !self.recalculate_minmax_grad {
            return;
        }

        self.gmin = T::max_value();
        self.gmax = T::min_value();

        for i in 0..self.sv.len() {
            let v = &self.sv[i];
            let g = v.grad;
            let a = v.alpha;
            if g < self.gmin && a > v.cmin {
                self.gmin = g;
                self.svmin = i;
            }
            if g > self.gmax && a < v.cmax {
                self.gmax = g;
                self.svmax = i;
            }
        }

        self.recalculate_minmax_grad = false
    }

    fn clean(&mut self, cache: &mut Cache<T, M, K>) {
        self.find_min_max_gradient();

        let gmax = self.gmax;
        let gmin = self.gmin;

        let mut idxs_to_drop: HashSet<usize> = HashSet::new();

        self.sv.retain(|v| {
            if v.alpha == T::zero() {
                if (v.grad >= gmax && T::zero() >= v.cmax)
                    || (v.grad <= gmin && T::zero() <= v.cmin)
                {
                    idxs_to_drop.insert(v.index);
                    return false;
                }
            };
            true
        });

        cache.drop(idxs_to_drop);
        self.recalculate_minmax_grad = true;
    }

    fn permutate(n: usize) -> Vec<usize> {
        let mut rng = rand::thread_rng();
        let mut range: Vec<usize> = (0..n).collect();
        range.shuffle(&mut rng);
        range
    }

    fn smo(
        &mut self,
        idx_1: Option<usize>,
        idx_2: Option<usize>,
        tol: T,
        cache: &mut Cache<T, M, K>,
    ) -> bool {
        let mut idx_1 = idx_1;
        let mut idx_2 = idx_2;

        let mut k_v_12: Option<T> = None;

        if idx_1.is_none() && idx_2.is_none() {
            self.find_min_max_gradient();
            if self.gmax > -self.gmin {
                idx_2 = Some(self.svmax);
            } else {
                idx_1 = Some(self.svmin);
            }
        }

        if idx_2.is_none() {
            let idx_1 = &self.sv[idx_1.unwrap()];
            let km = idx_1.k;
            let gm = idx_1.grad;
            let mut best = T::zero();
            for i in 0..self.sv.len() {
                let v = &self.sv[i];
                let z = v.grad - gm;
                let k = cache.get(idx_1, &v);
                let mut curv = km + v.k - T::two() * k;
                if curv <= T::zero() {
                    curv = self.tau;
                }
                let mu = z / curv;
                if (mu > T::zero() && v.alpha < v.cmax) || (mu < T::zero() && v.alpha > v.cmin) {
                    let gain = z * mu;
                    if gain > best {
                        best = gain;
                        idx_2 = Some(i);
                        k_v_12 = Some(k);
                    }
                }
            }
        }

        if idx_1.is_none() {
            let idx_2 = &self.sv[idx_2.unwrap()];
            let km = idx_2.k;
            let gm = idx_2.grad;
            let mut best = T::zero();
            for i in 0..self.sv.len() {
                let v = &self.sv[i];
                let z = gm - v.grad;
                let k = cache.get(idx_2, v);
                let mut curv = km + v.k - T::two() * k;
                if curv <= T::zero() {
                    curv = self.tau;
                }

                let mu = z / curv;
                if (mu > T::zero() && v.alpha > v.cmin) || (mu < T::zero() && v.alpha < v.cmax) {
                    let gain = z * mu;
                    if gain > best {
                        best = gain;
                        idx_1 = Some(i);
                        k_v_12 = Some(k);
                    }
                }
            }
        }

        if idx_1.is_none() || idx_2.is_none() {
            return false;
        }

        let idx_1 = idx_1.unwrap();
        let idx_2 = idx_2.unwrap();

        if k_v_12.is_none() {
            k_v_12 = Some(self.kernel.apply(&self.sv[idx_1].x, &self.sv[idx_2].x));
        }

        let k_v_12 = k_v_12.unwrap();

        let mut curv = self.sv[idx_1].k + self.sv[idx_2].k - T::two() * k_v_12;
        if curv <= T::zero() {
            curv = self.tau;
        }

        let mut step = (self.sv[idx_2].grad - self.sv[idx_1].grad) / curv;

        if step >= T::zero() {
            let mut ostep = self.sv[idx_1].alpha - self.sv[idx_1].cmin;
            if ostep < step {
                step = ostep;
            }
            ostep = self.sv[idx_2].cmax - self.sv[idx_2].alpha;
            if ostep < step {
                step = ostep;
            }
        } else {
            let mut ostep = self.sv[idx_2].cmin - self.sv[idx_2].alpha;
            if ostep > step {
                step = ostep;
            }
            ostep = self.sv[idx_1].alpha - self.sv[idx_1].cmax;
            if ostep > step {
                step = ostep;
            }
        }

        self.update(idx_1, idx_2, step, cache);

        return self.gmax - self.gmin > tol;
    }

    fn update(&mut self, v1: usize, v2: usize, step: T, cache: &mut Cache<T, M, K>) {
        self.sv[v1].alpha -= step;
        self.sv[v2].alpha += step;

        for i in 0..self.sv.len() {
            let k2 = cache.get(&self.sv[v2], &self.sv[i]);
            let k1 = cache.get(&self.sv[v1], &self.sv[i]);
            self.sv[i].grad -= step * (k2 - k1);
        }

        self.recalculate_minmax_grad = true;
        self.find_min_max_gradient();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linalg::naive::dense_matrix::*;
    use crate::metrics::accuracy;
    use crate::svm::*;

    #[test]
    fn svc_fit_predict() {
        let x = DenseMatrix::from_2d_array(&[
            &[5.1, 3.5, 1.4, 0.2],
            &[4.9, 3.0, 1.4, 0.2],
            &[4.7, 3.2, 1.3, 0.2],
            &[4.6, 3.1, 1.5, 0.2],
            &[5.0, 3.6, 1.4, 0.2],
            &[5.4, 3.9, 1.7, 0.4],
            &[4.6, 3.4, 1.4, 0.3],
            &[5.0, 3.4, 1.5, 0.2],
            &[4.4, 2.9, 1.4, 0.2],
            &[4.9, 3.1, 1.5, 0.1],
            &[7.0, 3.2, 4.7, 1.4],
            &[6.4, 3.2, 4.5, 1.5],
            &[6.9, 3.1, 4.9, 1.5],
            &[5.5, 2.3, 4.0, 1.3],
            &[6.5, 2.8, 4.6, 1.5],
            &[5.7, 2.8, 4.5, 1.3],
            &[6.3, 3.3, 4.7, 1.6],
            &[4.9, 2.4, 3.3, 1.0],
            &[6.6, 2.9, 4.6, 1.3],
            &[5.2, 2.7, 3.9, 1.4],
        ]);

        let y: Vec<f64> = vec![
            -1., -1., -1., -1., -1., -1., -1., -1., 1., 1., 1., 1., 1., 1., 1., 1., 1., 1., 1., 1.,
        ];

        let y_hat = SVC::fit(
            &x,
            &y,
            LinearKernel {},
            SVCParameters {
                epoch: 2,
                c: 200.0,
                tol: 1e-3,
            },
        )
        .and_then(|lr| lr.predict(&x))
        .unwrap();

        assert!(accuracy(&y_hat, &y) >= 0.9);
    }

    #[test]
    fn svc_serde() {
        let x = DenseMatrix::from_2d_array(&[
            &[5.1, 3.5, 1.4, 0.2],
            &[4.9, 3.0, 1.4, 0.2],
            &[4.7, 3.2, 1.3, 0.2],
            &[4.6, 3.1, 1.5, 0.2],
            &[5.0, 3.6, 1.4, 0.2],
            &[5.4, 3.9, 1.7, 0.4],
            &[4.6, 3.4, 1.4, 0.3],
            &[5.0, 3.4, 1.5, 0.2],
            &[4.4, 2.9, 1.4, 0.2],
            &[4.9, 3.1, 1.5, 0.1],
            &[7.0, 3.2, 4.7, 1.4],
            &[6.4, 3.2, 4.5, 1.5],
            &[6.9, 3.1, 4.9, 1.5],
            &[5.5, 2.3, 4.0, 1.3],
            &[6.5, 2.8, 4.6, 1.5],
            &[5.7, 2.8, 4.5, 1.3],
            &[6.3, 3.3, 4.7, 1.6],
            &[4.9, 2.4, 3.3, 1.0],
            &[6.6, 2.9, 4.6, 1.3],
            &[5.2, 2.7, 3.9, 1.4],
        ]);

        let y: Vec<f64> = vec![
            -1., -1., -1., -1., -1., -1., -1., -1., 1., 1., 1., 1., 1., 1., 1., 1., 1., 1., 1., 1.,
        ];

        let svr = SVC::fit(&x, &y, LinearKernel {}, Default::default()).unwrap();

        let deserialized_svr: SVC<f64, DenseMatrix<f64>, LinearKernel> =
            serde_json::from_str(&serde_json::to_string(&svr).unwrap()).unwrap();

        assert_eq!(svr, deserialized_svr);
    }
}
