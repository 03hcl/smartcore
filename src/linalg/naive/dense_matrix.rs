use std::ops::Range;
use crate::linalg::Matrix;
use crate::math;
use rand::prelude::*;

#[derive(Debug, Clone)]
pub struct DenseMatrix {

    ncols: usize,
    nrows: usize,
    values: Vec<f64> 

}

impl DenseMatrix {

    pub fn from_2d_array(values: &[&[f64]]) -> DenseMatrix {
        DenseMatrix::from_2d_vec(&values.into_iter().map(|row| Vec::from(*row)).collect())
    }

    pub fn from_2d_vec(values: &Vec<Vec<f64>>) -> DenseMatrix {
        let nrows = values.len();
        let ncols = values.first().unwrap_or_else(|| panic!("Cannot create 2d matrix from an empty vector")).len();
        let mut m = DenseMatrix {
            ncols: ncols,
            nrows: nrows,
            values: vec![0f64; ncols*nrows]
        };
        for row in 0..nrows {
            for col in 0..ncols {
                m.set(row, col, values[row][col]);
            }
        }
        m
    }

    pub fn from_array(nrows: usize, ncols: usize, values: &[f64]) -> DenseMatrix {
       DenseMatrix::from_vec(nrows, ncols, Vec::from(values)) 
    }

    pub fn from_vec(nrows: usize, ncols: usize, values: Vec<f64>) -> DenseMatrix {
        DenseMatrix {
            ncols: ncols,
            nrows: nrows,
            values: values
        }
    }

    pub fn div_mut(&mut self, b: DenseMatrix) -> () {
        if self.nrows != b.nrows || self.ncols != b.ncols {
            panic!("Can't divide matrices of different sizes.");
        }

        for i in 0..self.values.len() {
            self.values[i] /= b.values[i];
        }
    }

    fn set(&mut self, row: usize, col: usize, x: f64) {
        self.values[col*self.nrows + row] = x;        
    }

    fn div_element_mut(&mut self, row: usize, col: usize, x: f64) {
        self.values[col*self.nrows + row] /= x;
    }

    fn mul_element_mut(&mut self, row: usize, col: usize, x: f64) {
        self.values[col*self.nrows + row] *= x;
    }

    fn add_element_mut(&mut self, row: usize, col: usize, x: f64) {
        self.values[col*self.nrows + row] += x
    }

    fn sub_element_mut(&mut self, row: usize, col: usize, x: f64) {
        self.values[col*self.nrows + row] -= x;
    }
    
}

impl PartialEq for DenseMatrix {
    fn eq(&self, other: &Self) -> bool {
        if self.ncols != other.ncols || self.nrows != other.nrows {
            return false
        }

        let len = self.values.len();
        let other_len = other.values.len();

        if len != other_len {
            return false;
        }

        for i in 0..len {
            if (self.values[i] - other.values[i]).abs() > math::EPSILON {
                return false;
            }
        }

        true
    }
}

impl Into<Vec<f64>> for DenseMatrix {
    fn into(self) -> Vec<f64> {
        self.values
    }
}

impl Matrix for DenseMatrix {

    fn get(&self, row: usize, col: usize) -> f64 {
        self.values[col*self.nrows + row]
    }

    fn zeros(nrows: usize, ncols: usize) -> DenseMatrix {
        DenseMatrix::fill(nrows, ncols, 0f64)
    }

    fn ones(nrows: usize, ncols: usize) -> DenseMatrix {
        DenseMatrix::fill(nrows, ncols, 1f64)
    }

    fn shape(&self) -> (usize, usize) {
        (self.nrows, self.ncols)
    }

    fn v_stack(&self, other: &Self) -> Self {
        if self.ncols != other.ncols {
            panic!("Number of columns in both matrices should be equal");
        }
        let mut result = DenseMatrix::zeros(self.nrows + other.nrows, self.ncols);
        for c in 0..self.ncols {
            for r in 0..self.nrows+other.nrows {                 
                if r <  self.nrows {              
                    result.set(r, c, self.get(r, c));
                } else {
                    result.set(r, c, other.get(r - self.nrows, c));
                }
            }
        }    
        result    
    }

    fn h_stack(&self, other: &Self) -> Self{
        if self.nrows != other.nrows {
            panic!("Number of rows in both matrices should be equal");
        }
        let mut result = DenseMatrix::zeros(self.nrows, self.ncols + other.ncols);
        for r in 0..self.nrows {
            for c in 0..self.ncols+other.ncols {                             
                if c <  self.ncols {              
                    result.set(r, c, self.get(r, c));
                } else {
                    result.set(r, c, other.get(r, c - self.ncols));
                }
            }
        }
        result
    }

    fn dot(&self, other: &Self) -> Self {
        if self.ncols != other.nrows {
            panic!("Number of rows of A should equal number of columns of B");
        }
        let inner_d = self.ncols;
        let mut result = DenseMatrix::zeros(self.nrows, other.ncols);

        for r in 0..self.nrows {
            for c in 0..other.ncols {
                let mut s = 0f64;
                for i in 0..inner_d {
                    s += self.get(r, i) * other.get(i, c);
                }
                result.set(r, c, s);
            }
        }

        result
    }

    fn slice(&self, rows: Range<usize>, cols: Range<usize>) -> DenseMatrix {

        let ncols = cols.len();
        let nrows = rows.len();

        let mut m = DenseMatrix::from_vec(nrows, ncols, vec![0f64; nrows * ncols]);

        for r in rows.start..rows.end {
            for c in cols.start..cols.end {
                m.set(r-rows.start, c-cols.start, self.get(r, c));
            }
        }

        m
    }

    fn qr_solve_mut(&mut self, mut b: DenseMatrix) -> DenseMatrix {
        let m = self.nrows;
        let n = self.ncols;
        let nrhs = b.ncols;

        if self.nrows != b.nrows {
            panic!("Dimensions do not agree. Self.nrows should equal b.nrows but is {}, {}", self.nrows, b.nrows);
        }

        let mut r_diagonal: Vec<f64> = vec![0f64; n];

        for k in 0..n {
            let mut nrm = 0f64;
            for i in k..m {
                nrm = nrm.hypot(self.get(i, k));
            }

            if nrm.abs() > math::EPSILON {

                if self.get(k, k) < 0f64 {
                    nrm = -nrm;
                }
                for i in k..m {
                    self.div_element_mut(i, k, nrm);
                }
                self.add_element_mut(k, k, 1f64);

                for j in k+1..n {
                    let mut s = 0f64;
                    for i in k..m {
                        s += self.get(i, k) * self.get(i, j);
                    }
                    s = -s / self.get(k, k);
                    for i in k..m {
                        self.add_element_mut(i, j, s * self.get(i, k));
                    }
                }
            }            
            r_diagonal[k] = -nrm;
        }        
        
        for j in 0..r_diagonal.len() {
            if r_diagonal[j].abs() < math::EPSILON  {
                panic!("Matrix is rank deficient.");                
            }
        }

        for k in 0..n {
            for j in 0..nrhs {
                let mut s = 0f64;
                for i in k..m {
                    s += self.get(i, k) * b.get(i, j);
                }
                s = -s / self.get(k, k);
                for i in k..m {
                    b.add_element_mut(i, j, s * self.get(i, k));
                }
            }
        }         

        for k in (0..n).rev() {
            for j in 0..nrhs {
                b.set(k, j, b.get(k, j) / r_diagonal[k]);
            }
            
            for i in 0..k {
                for j in 0..nrhs {
                    b.sub_element_mut(i, j, b.get(k, j) * self.get(i, k));
                }
            }
        }

        b

    }

    fn svd_solve_mut(&mut self, mut b: DenseMatrix) -> DenseMatrix {

        if self.nrows != b.nrows {
            panic!("Dimensions do not agree. Self.nrows should equal b.nrows but is {}, {}", self.nrows, b.nrows);
        }

        let m = self.nrows;
        let n = self.ncols;
        
        let (mut l, mut nm) = (0usize, 0usize);
        let (mut anorm, mut g, mut scale) = (0f64, 0f64, 0f64);        
        
        let mut v = DenseMatrix::zeros(n, n);
        let mut w = vec![0f64; n];
        let mut rv1 = vec![0f64; n];

        for i in 0..n {
            l = i + 2;
            rv1[i] = scale * g;
            g = 0f64;
            let mut s = 0f64;
            scale = 0f64;

            if i < m {
                for k in i..m {
                    scale += self.get(k, i).abs();
                }

                if scale.abs() > math::EPSILON {

                    for k in i..m {
                        self.div_element_mut(k, i, scale);
                        s += self.get(k, i) * self.get(k, i);
                    }

                    let mut f = self.get(i, i);
                    g = -s.sqrt().copysign(f);                    
                    let h = f * g - s;
                    self.set(i, i, f - g);
                    for j in l - 1..n {
                        s = 0f64;
                        for k in i..m {
                            s += self.get(k, i) * self.get(k, j);
                        }
                        f = s / h;
                        for k in i..m {
                            self.add_element_mut(k, j, f * self.get(k, i));
                        }
                    }
                    for k in i..m {
                        self.mul_element_mut(k, i, scale);
                    }
                }
            }

            w[i] = scale * g;
            g = 0f64;
            let mut s = 0f64;
            scale = 0f64;

            if i + 1 <= m && i + 1 != n {
                for k in l - 1..n {
                    scale += self.get(i, k).abs();
                }

                if scale.abs() > math::EPSILON  {
                    for k in l - 1..n {
                        self.div_element_mut(i, k, scale);
                        s += self.get(i, k) * self.get(i, k);
                    }

                    let f = self.get(i, l - 1);
                    g = -s.sqrt().copysign(f);                    
                    let h = f * g - s;
                    self.set(i, l - 1, f - g);

                    for k in l - 1..n {
                        rv1[k] = self.get(i, k) / h;
                    }

                    for j in l - 1..m {
                        s = 0f64;
                        for k in l - 1..n {
                            s += self.get(j, k) * self.get(i, k);
                        }

                        for k in l - 1..n {
                            self.add_element_mut(j, k, s * rv1[k]);
                        }
                    }

                    for k in l - 1..n {
                        self.mul_element_mut(i, k, scale);
                    }
                }
            }

            
            anorm = f64::max(anorm, w[i].abs() + rv1[i].abs());
        }

        for i in (0..n).rev() {
            if i < n - 1 {
                if g != 0.0 {
                    for j in l..n {
                        v.set(j, i, (self.get(i, j) / self.get(i, l)) / g);
                    }
                    for j in l..n {
                        let mut s = 0f64;
                        for k in l..n {
                            s += self.get(i, k) * v.get(k, j);
                        }
                        for k in l..n {
                            v.add_element_mut(k, j, s * v.get(k, i));
                        }
                    }
                }
                for j in l..n {
                    v.set(i, j, 0f64);
                    v.set(j, i, 0f64);
                }
            }
            v.set(i, i, 1.0);
            g = rv1[i];
            l = i;
        }

        for i in (0..usize::min(m, n)).rev() {
            l = i + 1;
            g = w[i];
            for j in l..n {
                self.set(i, j, 0f64);
            }

            if g.abs() > math::EPSILON {
                g = 1f64 / g;
                for j in l..n {
                    let mut s = 0f64;
                    for k in l..m {
                        s += self.get(k, i) * self.get(k, j);
                    }
                    let f = (s / self.get(i, i)) * g;
                    for k in i..m {
                        self.add_element_mut(k, j, f * self.get(k, i));
                    }
                }
                for j in i..m {
                    self.mul_element_mut(j, i, g);
                }
            } else {
                for j in i..m {
                    self.set(j, i, 0f64);
                }
            }

            self.add_element_mut(i, i, 1f64);
        }

        for k in (0..n).rev() {
            for iteration in 0..30 {
                let mut flag = true;                
                l = k;
                while l != 0 {                               
                    if l == 0 || rv1[l].abs() <= math::EPSILON * anorm {
                        flag = false;   
                        break;
                    }
                    nm = l - 1;
                    if w[nm].abs() <= math::EPSILON * anorm {
                        break;
                    }
                    l -= 1;
                }

                if flag {
                    let mut c = 0.0;
                    let mut s = 1.0;
                    for i in l..k+1 {
                        let f = s * rv1[i];
                        rv1[i] = c * rv1[i];
                        if f.abs() <= math::EPSILON * anorm {
                            break;
                        }
                        g = w[i];
                        let mut h = f.hypot(g);
                        w[i] = h;
                        h = 1.0 / h;
                        c = g * h;
                        s = -f * h;
                        for j in 0..m {
                            let y = self.get(j, nm);
                            let z = self.get(j, i);
                            self.set(j, nm, y * c + z * s);
                            self.set(j,  i, z * c - y * s);
                        }
                    }
                }

                let z = w[k];
                if l == k {
                    if z < 0f64 {
                        w[k] = -z;
                        for j in 0..n {
                            v.set(j, k, -v.get(j, k));
                        }
                    }
                    break;
                }

                if iteration == 29 {
                    panic!("no convergence in 30 iterations");
                }

                let mut x = w[l];
                nm = k - 1;
                let mut y = w[nm];
                g = rv1[nm];
                let mut h = rv1[k];
                let mut f = ((y - z) * (y + z) + (g - h) * (g + h)) / (2.0 * h * y);
                g = f.hypot(1.0);
                f = ((x - z) * (x + z) + h * ((y / (f + g.copysign(f))) - h)) / x;
                let mut c = 1f64;
                let mut s = 1f64;

                for j in l..=nm {
                    let i = j + 1;
                    g = rv1[i];
                    y = w[i];
                    h = s * g;
                    g = c * g;
                    let mut z = f.hypot(h);
                    rv1[j] = z;
                    c = f / z;
                    s = h / z;
                    f = x * c + g * s;
                    g = g * c - x * s;
                    h = y * s;
                    y *= c;

                    for jj in 0..n {
                        x = v.get(jj, j);
                        z = v.get(jj, i);
                        v.set(jj, j, x * c + z * s);
                        v.set(jj, i, z * c - x * s);
                    }

                    z = f.hypot(h);
                    w[j] = z;
                    if z.abs() > math::EPSILON {
                        z = 1.0 / z;
                        c = f * z;
                        s = h * z;
                    }

                    f = c * g + s * y;
                    x = c * y - s * g;
                    for jj in 0..m {
                        y = self.get(jj, j);
                        z = self.get(jj, i);
                        self.set(jj, j, y * c + z * s);
                        self.set(jj, i, z * c - y * s);
                    }
                }

                rv1[l] = 0.0;
                rv1[k] = f;
                w[k] = x;
            }
        }
        
        let mut inc = 1usize;        
        let mut su = vec![0f64; m];
        let mut sv = vec![0f64; n];
        
        loop {
            inc *= 3;
            inc += 1;
            if inc > n {
                break;
            }
        }

        loop {
            inc /= 3;
            for  i in inc..n {                
                let sw = w[i];
                for k in 0..m {
                    su[k] = self.get(k, i);
                }
                for k in 0..n {
                    sv[k] = v.get(k, i);
                }
                let mut j = i;
                while w[j - inc] < sw {
                    w[j] = w[j - inc];
                    for k in 0..m {
                        self.set(k, j, self.get(k, j - inc));
                    }
                    for k in 0..n {
                        v.set(k, j, v.get(k, j - inc));
                    }
                    j -= inc;
                    if j < inc {
                        break;
                    }
                }
                w[j] = sw;
                for k in 0..m {
                    self.set(k, j, su[k]);
                }
                for k in 0..n {
                    v.set(k, j, sv[k]);
                }

            }
            if inc <= 1 {
                break;
            }
        }

        for k in 0..n {
            let mut s = 0.;
            for i in 0..m {
                if self.get(i, k) < 0. {
                    s += 1.;
                }
            }
            for j in 0..n {
                if v.get(j, k) < 0. {
                    s += 1.;
                }
            }
            if s > (m + n) as f64 / 2. {
                for i in 0..m {
                    self.set(i, k, -self.get(i, k));
                }
                for j in 0..n {
                    v.set(j, k, -v.get(j, k));
                }
            }
        }

        let tol = 0.5 * ((m + n) as f64 + 1.).sqrt() * w[0] * math::EPSILON;

        let p = b.ncols;

        for k in 0..p {
            let mut tmp = vec![0f64; v.nrows];
            for j in 0..n {
                let mut r = 0f64;
                if w[j] > tol {
                    for i in 0..m {
                        r += self.get(i, j) * b.get(i, k);
                    }
                    r /= w[j];
                }
                tmp[j] = r;
            }

            for j in 0..n {
                let mut r = 0.0;
                for jj in 0..n {
                    r += v.get(j, jj) * tmp[jj];
                }
                b.set(j, k, r);
            }
        }

        b

    }

    fn approximate_eq(&self, other: &Self, error: f64) -> bool {
        if self.ncols != other.ncols || self.nrows != other.nrows {
            return false
        }

        for c in 0..self.ncols {
            for r in 0..self.nrows {
                if (self.get(r, c) - other.get(r, c)).abs() > error {
                    return false
                }
            }
        }

        true
    }

    fn fill(nrows: usize, ncols: usize, value: f64) -> Self {
        DenseMatrix::from_vec(nrows, ncols, vec![value; ncols * nrows])
    }

    fn add_mut(&mut self, other: &Self) {
        if self.ncols != other.ncols || self.nrows != other.nrows {
            panic!("A and B should have the same shape");
        }        
        for c in 0..self.ncols {
            for r in 0..self.nrows {
                self.add_element_mut(r, c, other.get(r, c));
            }
        }
    }

    fn generate_positive_definite(nrows: usize, ncols: usize) -> Self {
        let m = DenseMatrix::rand(nrows, ncols);
        m.dot(&m.transpose())
    }

    fn transpose(&self) -> Self {
        let mut m = DenseMatrix {
            ncols: self.nrows,
            nrows: self.ncols,
            values: vec![0f64; self.ncols * self.nrows]
        };
        for c in 0..self.ncols {
            for r in 0..self.nrows {
                m.set(c, r, self.get(r, c));
            }
        }
        m

    }

    fn rand(nrows: usize, ncols: usize) -> Self {
        let mut rng = rand::thread_rng();
        let values: Vec<f64> = (0..nrows*ncols).map(|_| {
            rng.gen()
        }).collect();
        DenseMatrix {
            ncols: ncols,
            nrows: nrows,
            values: values
        }
    }    

    fn norm2(&self) -> f64 {
        let mut norm = 0f64;

        for xi in self.values.iter() {
            norm += xi * xi;
        }

        norm.sqrt()
    }

    fn add_scalar_mut(&mut self, scalar: f64) {
        for i in 0..self.values.len() {
            self.values[i] += scalar;
        }
    }

    fn sub_scalar_mut(&mut self, scalar: f64) {
        for i in 0..self.values.len() {
            self.values[i] -= scalar;
        }
    }

    fn mul_scalar_mut(&mut self, scalar: f64) {
        for i in 0..self.values.len() {
            self.values[i] *= scalar;
        }
    }

    fn div_scalar_mut(&mut self, scalar: f64) {
        for i in 0..self.values.len() {
            self.values[i] /= scalar;
        }
    }

    fn negative_mut(&mut self) {
        for i in 0..self.values.len() {
            self.values[i] = -self.values[i];
        }
    }

}

#[cfg(test)]
mod tests {    
    use super::*; 

    #[test]
    fn qr_solve_mut() { 

            let mut a = DenseMatrix::from_2d_array(&[&[0.9, 0.4, 0.7], &[0.4, 0.5, 0.3], &[0.7, 0.3, 0.8]]);
            let b = DenseMatrix::from_2d_array(&[&[0.5, 0.2],&[0.5, 0.8], &[0.5, 0.3]]);
            let expected_w = DenseMatrix::from_array(3, 2, &[-0.20, 0.87, 0.47, -1.28, 2.22, 0.66]);
            let w = a.qr_solve_mut(b);   
            assert!(w.approximate_eq(&expected_w, 1e-2));
    }

    #[test]
    fn svd_solve_mut() { 

            let mut a = DenseMatrix::from_2d_array(&[&[0.9, 0.4, 0.7], &[0.4, 0.5, 0.3], &[0.7, 0.3, 0.8]]);
            let b = DenseMatrix::from_2d_array(&[&[0.5, 0.2],&[0.5, 0.8], &[0.5, 0.3]]);
            let expected_w = DenseMatrix::from_array(3, 2, &[-0.20, 0.87, 0.47, -1.28, 2.22, 0.66]);
            let w = a.svd_solve_mut(b);  
            assert!(w.approximate_eq(&expected_w, 1e-2));            
    }

    #[test]
    fn v_stack() { 

            let a = DenseMatrix::from_2d_array(
                &[
                    &[1., 2., 3.],
                    &[4., 5., 6.],
                    &[7., 8., 9.]]);
            let b = DenseMatrix::from_2d_array(
                &[
                    &[1., 2., 3.],
                    &[4., 5., 6.]]);
            let expected = DenseMatrix::from_2d_array(
                &[
                    &[1., 2., 3.], 
                    &[4., 5., 6.], 
                    &[7., 8., 9.], 
                    &[1., 2., 3.], 
                    &[4., 5., 6.]]);
            let result = a.v_stack(&b);               
            assert_eq!(result, expected);
    }

    #[test]
    fn h_stack() { 

            let a = DenseMatrix::from_2d_array(
                &[
                    &[1., 2., 3.],
                    &[4., 5., 6.],
                    &[7., 8., 9.]]);
            let b = DenseMatrix::from_2d_array(
                &[
                    &[1., 2.],
                    &[3., 4.],
                    &[5., 6.]]);
            let expected = DenseMatrix::from_2d_array(
                &[
                    &[1., 2., 3., 1., 2.], 
                    &[4., 5., 6., 3., 4.], 
                    &[7., 8., 9., 5., 6.]]);
            let result = a.h_stack(&b);               
            assert_eq!(result, expected);
    }

    #[test]
    fn dot() { 

            let a = DenseMatrix::from_2d_array(
                &[
                    &[1., 2., 3.],
                    &[4., 5., 6.]]);
            let b = DenseMatrix::from_2d_array(
                &[
                    &[1., 2.],
                    &[3., 4.],
                    &[5., 6.]]);
            let expected = DenseMatrix::from_2d_array(
                &[
                    &[22., 28.], 
                    &[49., 64.]]);
            let result = a.dot(&b);               
            assert_eq!(result, expected);
    }

    #[test]
    fn slice() { 

            let m = DenseMatrix::from_2d_array(
                &[
                    &[1., 2., 3., 1., 2.], 
                    &[4., 5., 6., 3., 4.], 
                    &[7., 8., 9., 5., 6.]]);
            let expected = DenseMatrix::from_2d_array(
                &[
                    &[2., 3.], 
                    &[5., 6.]]);
            let result = m.slice(0..2, 1..3);
            assert_eq!(result, expected);
    }
    

    #[test]
    fn approximate_eq() {             
            let m = DenseMatrix::from_2d_array(
                &[
                    &[2., 3.], 
                    &[5., 6.]]);
            let m_eq = DenseMatrix::from_2d_array(
                &[
                    &[2.5, 3.0], 
                    &[5., 5.5]]);
            let m_neq = DenseMatrix::from_2d_array(
                &[
                    &[3.0, 3.0], 
                    &[5., 6.5]]);            
            assert!(m.approximate_eq(&m_eq, 0.5));
            assert!(!m.approximate_eq(&m_neq, 0.5));
    }

    #[test]
    fn rand() {
        let m = DenseMatrix::rand(3, 3);
        for c in 0..3 {
            for r in 0..3 {
                assert!(m.get(r, c) != 0f64);
            }
        }
    }

    #[test]
    fn transpose() {
        let m = DenseMatrix::from_2d_array(&[&[1.0, 3.0], &[2.0, 4.0]]);
        let expected = DenseMatrix::from_2d_array(&[&[1.0, 2.0], &[3.0, 4.0]]);
        let m_transposed = m.transpose();
        for c in 0..2 {
            for r in 0..2 {
                assert!(m_transposed.get(r, c) == expected.get(r, c));
            }
        }
    }

    #[test]
    fn generate_positive_definite() {
        let m = DenseMatrix::generate_positive_definite(3, 3);        
    }

}

