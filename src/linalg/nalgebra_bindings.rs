use std::ops::{Range, AddAssign, SubAssign, MulAssign, DivAssign};
use std::iter::Sum;

use nalgebra::{MatrixMN, DMatrix, Matrix, Scalar, Dynamic, U1, VecStorage};

use crate::math::num::FloatExt;
use crate::linalg::BaseMatrix;
use crate::linalg::Matrix as SmartCoreMatrix;
use crate::linalg::svd::SVDDecomposableMatrix;
use crate::linalg::evd::EVDDecomposableMatrix;
use crate::linalg::qr::QRDecomposableMatrix;
use crate::linalg::lu::LUDecomposableMatrix;

impl<T: FloatExt + Scalar + AddAssign + SubAssign + MulAssign + DivAssign + Sum + 'static> BaseMatrix<T> for Matrix<T, Dynamic, Dynamic, VecStorage<T, Dynamic, Dynamic>>
{
    type RowVector = MatrixMN<T, U1, Dynamic>;

    fn from_row_vector(vec: Self::RowVector) -> Self{
        Matrix::from_rows(&[vec])
    }

    fn to_row_vector(self) -> Self::RowVector{
        self.row(0).into_owned()
    }

    fn get(&self, row: usize, col: usize) -> T {
        *self.get((row, col)).unwrap()
    }

    fn get_row_as_vec(&self, row: usize) -> Vec<T> {
        self.row(row).iter().map(|v| *v).collect()
    }

    fn get_col_as_vec(&self, col: usize) -> Vec<T> {
        self.column(col).iter().map(|v| *v).collect()
    }

    fn set(&mut self, row: usize, col: usize, x: T) {
        *self.get_mut((row, col)).unwrap() = x;
    }   

    fn eye(size: usize) -> Self {
        DMatrix::identity(size, size)
    }

    fn zeros(nrows: usize, ncols: usize) -> Self {        
        DMatrix::zeros(nrows, ncols)
    }

    fn ones(nrows: usize, ncols: usize) -> Self {
        BaseMatrix::fill(nrows, ncols, T::one())
    }

    fn to_raw_vector(&self) -> Vec<T> {
        let (nrows, ncols) = self.shape();
        let mut result = vec![T::zero(); nrows * ncols];
        for (i, row) in self.row_iter().enumerate() {
            for (j, v) in row.iter().enumerate() {
                result[i * ncols + j] = *v;
            }            
        }
        result
    }

    fn fill(nrows: usize, ncols: usize, value: T) -> Self {
        let mut m = DMatrix::zeros(nrows, ncols);
        m.fill(value);
        m
    }

    fn shape(&self) -> (usize, usize) {
        self.shape()
    }    

    fn v_stack(&self, other: &Self) -> Self {        
        let mut columns = Vec::new();        
        for r in 0..self.ncols(){
            columns.push(self.column(r));
        }
        for r in 0..other.ncols(){
            columns.push(other.column(r));
        }
        Matrix::from_columns(&columns)
    }

    fn h_stack(&self, other: &Self) -> Self {
        let mut rows = Vec::new();        
        for r in 0..self.nrows(){
            rows.push(self.row(r));
        }
        for r in 0..other.nrows(){
            rows.push(other.row(r));
        }
        Matrix::from_rows(&rows)
    }

    fn dot(&self, other: &Self) -> Self {
        self * other
    }    

    fn vector_dot(&self, other: &Self) -> T {
        self.dot(other)
    }    

    fn slice(&self, rows: Range<usize>, cols: Range<usize>) -> Self {
        self.slice_range(rows, cols).into_owned()
    }

    fn approximate_eq(&self, other: &Self, error: T) -> bool {
        assert!(self.shape() == other.shape());
        self.iter()
            .zip(other.iter())
            .all(|(a, b)| (*a - *b).abs() <= error)
    }

    fn add_mut(&mut self, other: &Self) -> &Self {
        *self += other;
        self
    }

    fn sub_mut(&mut self, other: &Self) -> &Self {
        *self -= other;
        self
    }

    fn mul_mut(&mut self, other: &Self) -> &Self {
        self.component_mul_assign(other);
        self
    }

    fn div_mut(&mut self, other: &Self) -> &Self{
        self.component_div_assign(other);
        self
    }

    fn add_scalar_mut(&mut self, scalar: T) -> &Self{
        Matrix::add_scalar_mut(self, scalar);
        self
    }

    fn sub_scalar_mut(&mut self, scalar: T) -> &Self{
        Matrix::add_scalar_mut(self, -scalar);
        self
    }

    fn mul_scalar_mut(&mut self, scalar: T) -> &Self{
        *self *= scalar;
        self
    }

    fn div_scalar_mut(&mut self, scalar: T) -> &Self{
        *self /= scalar;
        self
    }

    fn transpose(&self) -> Self{
        self.transpose()
    }

    fn rand(nrows: usize, ncols: usize) -> Self{        
        DMatrix::from_iterator(nrows, ncols, (0..nrows*ncols).map(|_| {
            T::rand()
        }))
    }

    fn norm2(&self) -> T{
        self.iter().map(|x| *x * *x).sum::<T>().sqrt()
    }

    fn norm(&self, p:T) -> T {
        if p.is_infinite() && p.is_sign_positive() {
            self.iter().fold(T::neg_infinity(), |f, &val| {
                let v = val.abs();
                if f > v {
                    f
                } else {
                    v
                }
            })            
        } else if p.is_infinite() && p.is_sign_negative() {
            self.iter().fold(T::infinity(), |f, &val| {
                let v = val.abs();
                if f < v {
                    f
                } else {
                    v
                }
            })
        } else {

            let mut norm = T::zero();

            for xi in self.iter() {
                norm = norm + xi.abs().powf(p);
            }

            norm.powf(T::one()/p)
        }
    }

    fn column_mean(&self) -> Vec<T> {

        let mut res = Vec::new();

        for column in self.column_iter() {
            let mut sum = T::zero();
            let mut count = 0;
            for v in column.iter() {
                sum += *v;
                count += 1;
            }    
            res.push(sum / T::from(count).unwrap());
        }
        
        res
    }

    fn div_element_mut(&mut self, row: usize, col: usize, x: T){
        *self.get_mut((row, col)).unwrap() = *self.get((row, col)).unwrap() / x;
    }

    fn mul_element_mut(&mut self, row: usize, col: usize, x: T){
        *self.get_mut((row, col)).unwrap() = *self.get((row, col)).unwrap() * x;
    }

    fn add_element_mut(&mut self, row: usize, col: usize, x: T){
        *self.get_mut((row, col)).unwrap() = *self.get((row, col)).unwrap() + x;
    }

    fn sub_element_mut(&mut self, row: usize, col: usize, x: T){
        *self.get_mut((row, col)).unwrap() = *self.get((row, col)).unwrap() - x;
    }

    fn negative_mut(&mut self){
        *self *= -T::one();
    }

    fn reshape(&self, nrows: usize, ncols: usize) -> Self{        
        DMatrix::from_row_slice(nrows, ncols, &self.to_raw_vector())
    }

    fn copy_from(&mut self, other: &Self){
        Matrix::copy_from(self, other);
    }

    fn abs_mut(&mut self) -> &Self{
        for v in self.iter_mut(){
            *v = v.abs()
        }        
        self
    }

    fn sum(&self) -> T{
        let mut sum = T::zero();
        for v in self.iter(){
            sum += *v;
        } 
        sum
    }

    fn max_diff(&self, other: &Self) -> T{
        let mut max_diff = T::zero();
        for r in 0..self.nrows() {
            for c in 0..self.ncols() {
                max_diff = max_diff.max((self[(r, c)] - other[(r, c)]).abs());
            }
        }
        max_diff        
    }
    
    fn softmax_mut(&mut self){
        let max = self.iter().map(|x| x.abs()).fold(T::neg_infinity(), |a, b| a.max(b));
        let mut z = T::zero();
        for r in 0..self.nrows() {
            for c in 0..self.ncols() {
                let p = (self[(r, c)] - max).exp();
                self.set(r, c, p);
                z = z + p;
            }
        }
        for r in 0..self.nrows() {
            for c in 0..self.ncols() {
                self.set(r, c, self[(r, c)] / z);
            }
        }
    }

    fn pow_mut(&mut self, p: T) -> &Self{
        for v in self.iter_mut(){
            *v = v.powf(p)
        }
        self
    }

    fn argmax(&self) -> Vec<usize>{
        let mut res = vec![0usize; self.nrows()];

        for r in 0..self.nrows() {
            let mut max = T::neg_infinity();
            let mut max_pos = 0usize;
            for c in 0..self.ncols() {
                let v = self[(r, c)];
                if max < v {
                    max = v;
                    max_pos = c; 
                }
            }
            res[r] = max_pos;
        }

        res

    }
    
    fn unique(&self) -> Vec<T> {
        let mut result: Vec<T> = self.iter().map(|v| *v).collect();
        result.sort_by(|a, b| a.partial_cmp(b).unwrap());
        result.dedup();
        result
    }

    fn cov(&self) -> Self {
        panic!("Not implemented");
    }

}

impl<T: FloatExt + Scalar + AddAssign + SubAssign + MulAssign + DivAssign + Sum + 'static> SVDDecomposableMatrix<T> for Matrix<T, Dynamic, Dynamic, VecStorage<T, Dynamic, Dynamic>> {}

impl<T: FloatExt + Scalar + AddAssign + SubAssign + MulAssign + DivAssign + Sum + 'static> EVDDecomposableMatrix<T> for Matrix<T, Dynamic, Dynamic, VecStorage<T, Dynamic, Dynamic>> {}

impl<T: FloatExt + Scalar + AddAssign + SubAssign + MulAssign + DivAssign + Sum + 'static> QRDecomposableMatrix<T> for Matrix<T, Dynamic, Dynamic, VecStorage<T, Dynamic, Dynamic>> {}

impl<T: FloatExt + Scalar + AddAssign + SubAssign + MulAssign + DivAssign + Sum + 'static> LUDecomposableMatrix<T> for Matrix<T, Dynamic, Dynamic, VecStorage<T, Dynamic, Dynamic>> {}

impl<T: FloatExt + Scalar + AddAssign + SubAssign + MulAssign + DivAssign + Sum + 'static> SmartCoreMatrix<T> for Matrix<T, Dynamic, Dynamic, VecStorage<T, Dynamic, Dynamic>> {}

#[cfg(test)]
mod tests {    
    use super::*; 
    use nalgebra::{Matrix2x3, DMatrix, RowDVector};

    #[test]
    fn get_set_dynamic() {
        let mut m = DMatrix::from_row_slice(
            2,
            3,
            &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
        );

        let expected = Matrix2x3::new(1., 2., 3., 4.,
            10., 6.);

        m.set(1, 1, 10.);

        assert_eq!(m, expected);
        assert_eq!(10., BaseMatrix::get(&m, 1, 1));     
    }

    #[test]
    fn zeros() {
        let expected = DMatrix::from_row_slice(
            2,
            2,
            &[0., 0., 0., 0.],
        );

        let m:DMatrix<f64> = BaseMatrix::zeros(2, 2);  

        assert_eq!(m, expected);
    }

    #[test]
    fn ones() {
        let expected = DMatrix::from_row_slice(
            2,
            2,
            &[1., 1., 1., 1.],
        );

        let m:DMatrix<f64> = BaseMatrix::ones(2, 2);          

        assert_eq!(m, expected);
    }

    #[test]
    fn eye(){
        let expected = DMatrix::from_row_slice(3, 3, &[1., 0., 0., 0., 1., 0., 0., 0., 1.]);  
        let m: DMatrix<f64> = BaseMatrix::eye(3);        
        assert_eq!(m, expected);
    }

    #[test]
    fn shape() {
        let m:DMatrix<f64> = BaseMatrix::zeros(5, 10);  
        let (nrows, ncols) = m.shape();

        assert_eq!(nrows, 5);
        assert_eq!(ncols, 10);
    }

    #[test]
    fn scalar_add_sub_mul_div(){        
        let mut m = DMatrix::from_row_slice(
            2,
            3,
            &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
        );        

        let expected = DMatrix::from_row_slice(
            2,
            3,
            &[0.6, 0.8, 1., 1.2, 1.4, 1.6],
        );

        m.add_scalar_mut(3.0);
        m.sub_scalar_mut(1.0);
        m.mul_scalar_mut(2.0);
        m.div_scalar_mut(10.0);
        assert_eq!(m, expected);
    }

    #[test]
    fn add_sub_mul_div(){        
        let mut m = DMatrix::from_row_slice(
            2,
            2,
            &[1.0, 2.0, 3.0, 4.0],
        );
        
        let a = DMatrix::from_row_slice(
            2,
            2,
            &[1.0, 2.0, 3.0, 4.0],
        );

        let b: DMatrix<f64> = BaseMatrix::fill(2, 2, 10.);

        let expected = DMatrix::from_row_slice(
            2,
            2,
            &[0.1, 0.6, 1.5, 2.8],
        );        

        m.add_mut(&a);
        m.mul_mut(&a);
        m.sub_mut(&a);
        m.div_mut(&b);
        
        assert_eq!(m, expected);
    }

    #[test]
    fn to_from_row_vector(){        
        let v = RowDVector::from_vec(vec!(1., 2., 3., 4.));
        let expected = v.clone();
        let m: DMatrix<f64> = BaseMatrix::from_row_vector(v);
        assert_eq!(m.to_row_vector(), expected);                
    }

    #[test]
    fn get_row_col_as_vec(){        
        let m = DMatrix::from_row_slice(
            3,
            3,
            &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0],
        );  

        assert_eq!(m.get_row_as_vec(1), vec!(4., 5., 6.));
        assert_eq!(m.get_col_as_vec(1), vec!(2., 5., 8.));
    }

    #[test]
    fn to_raw_vector(){
        let m = DMatrix::from_row_slice(
            2,
            3,
            &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
        );  

        assert_eq!(m.to_raw_vector(), vec!(1., 2., 3., 4., 5., 6.));
    }

    #[test]
    fn element_add_sub_mul_div(){        
        let mut m = DMatrix::from_row_slice(
            2,
            2,
            &[1.0, 2.0, 3.0, 4.0],
        );        

        let expected = DMatrix::from_row_slice(
            2,
            2,
            &[4., 1., 6., 0.4],
        );

        m.add_element_mut(0, 0, 3.0);
        m.sub_element_mut(0, 1, 1.0);
        m.mul_element_mut(1, 0, 2.0);
        m.div_element_mut(1, 1, 10.0);
        assert_eq!(m, expected);
    }

    #[test]
    fn vstack_hstack() { 

        let m1 = DMatrix::from_row_slice(2, 3, &[1.,  2.,  3., 4., 5., 6.]);
        let m2 = DMatrix::from_row_slice(2, 1, &[ 7., 8.]);

        let m3 = DMatrix::from_row_slice(1, 4, &[9., 10., 11., 12.]);        

        let expected = DMatrix::from_row_slice(3, 4, &[1., 2., 3., 7., 4., 5., 6., 8., 9., 10., 11., 12.]);        

        let result = m1.v_stack(&m2).h_stack(&m3);        
        
        assert_eq!(result, expected);      

    }

    #[test]
    fn dot() { 

            let a = DMatrix::from_row_slice(2, 3, &[1., 2., 3., 4., 5., 6.]);
            let b = DMatrix::from_row_slice(3, 2, &[1., 2., 3., 4., 5., 6.]);
            let expected = DMatrix::from_row_slice(2, 2, &[22., 28., 49., 64.]);
            let result = BaseMatrix::dot(&a, &b);
            assert_eq!(result, expected);
    }  

    #[test]
    fn vector_dot() { 
            let a = DMatrix::from_row_slice(1, 3, &[1., 2., 3.]);
            let b = DMatrix::from_row_slice(1, 3, &[1., 2., 3.]);            
            assert_eq!(14., a.vector_dot(&b));
    }

    #[test]
    fn slice() { 

            let a = DMatrix::from_row_slice(3, 5, &[1., 2., 3., 1., 2., 4., 5., 6., 3., 4., 7., 8., 9., 5., 6.]);
            let expected = DMatrix::from_row_slice(2, 2, &[2., 3., 5., 6.]);
            let result = BaseMatrix::slice(&a, 0..2, 1..3);
            assert_eq!(result, expected);
    }

    #[test]
    fn approximate_eq() {
        let a = DMatrix::from_row_slice(3, 3, &[1., 2., 3., 4., 5., 6., 7., 8., 9.]);
        let noise = DMatrix::from_row_slice(3, 3, &[1e-5, 2e-5, 3e-5, 4e-5, 5e-5, 6e-5, 7e-5, 8e-5, 9e-5]);        
        assert!(a.approximate_eq(&(&noise + &a), 1e-4));
        assert!(!a.approximate_eq(&(&noise + &a), 1e-5));
    }

    #[test]
    fn negative_mut() { 
        let mut v = DMatrix::from_row_slice(1, 3, &[3., -2., 6.]);       
        v.negative_mut();     
        assert_eq!(v, DMatrix::from_row_slice(1, 3, &[-3., 2., -6.]));        
    }

    #[test]
    fn transpose() {
        let m = DMatrix::from_row_slice(2, 2, &[1.0, 3.0, 2.0, 4.0]);
        let expected = DMatrix::from_row_slice(2, 2, &[1.0, 2.0, 3.0, 4.0]);
        let m_transposed = m.transpose();
        assert_eq!(m_transposed, expected);       
    }

    #[test]
    fn rand() {
        let m: DMatrix<f64> = BaseMatrix::rand(3, 3);
        for c in 0..3 {
            for r in 0..3 {
                assert!(*m.get((r, c)).unwrap() != 0f64);
            }
        }
    }

    #[test]
    fn norm() { 
        let v = DMatrix::from_row_slice(1, 3, &[3., -2., 6.]);            
        assert_eq!(BaseMatrix::norm(&v, 1.), 11.);
        assert_eq!(BaseMatrix::norm(&v, 2.), 7.);
        assert_eq!(BaseMatrix::norm(&v, std::f64::INFINITY), 6.);
        assert_eq!(BaseMatrix::norm(&v, std::f64::NEG_INFINITY), 2.);
    }

    #[test]
    fn col_mean(){
        let a = DMatrix::from_row_slice(3, 3, &[1., 2., 3., 4., 5., 6., 7., 8., 9.]);
        let res = BaseMatrix::column_mean(&a);        
        assert_eq!(res, vec![4., 5., 6.]);        
    }

    #[test]
    fn reshape() {
        let m_orig = DMatrix::from_row_slice(1, 6, &[1., 2., 3., 4., 5., 6.]);        
        let m_2_by_3 = m_orig.reshape(2, 3);        
        let m_result = m_2_by_3.reshape(1, 6);        
        assert_eq!(BaseMatrix::shape(&m_2_by_3), (2, 3));
        assert_eq!(BaseMatrix::get(&m_2_by_3, 1, 1), 5.);
        assert_eq!(BaseMatrix::get(&m_result, 0, 1), 2.);
        assert_eq!(BaseMatrix::get(&m_result, 0, 3), 4.);
    }

    #[test]
    fn copy_from() {
        let mut src = DMatrix::from_row_slice(1, 3, &[1., 2., 3.]);
        let dst = BaseMatrix::zeros(1, 3);
        src.copy_from(&dst);
        assert_eq!(src, dst);
    }

    #[test]
    fn abs_mut() {
        let mut a = DMatrix::from_row_slice(2, 2, &[1., -2., 3., -4.]);
        let expected = DMatrix::from_row_slice(2, 2, &[1., 2., 3., 4.]);
        a.abs_mut();
        assert_eq!(a, expected);
    }

    #[test]
    fn sum() {
        let a = DMatrix::from_row_slice(1, 3, &[1., 2., 3.]);        
        assert_eq!(a.sum(), 6.);
    }

    #[test]
    fn max_diff() {
        let a1 = DMatrix::from_row_slice(2, 3, &[1., 2., 3., 4., -5., 6.]);
        let a2 = DMatrix::from_row_slice(2, 3, &[2., 3., 4., 1., 0., -12.]);        
        assert_eq!(a1.max_diff(&a2), 18.);
        assert_eq!(a2.max_diff(&a2), 0.);
    }

    #[test]
    fn softmax_mut(){
        let mut prob: DMatrix<f64> = DMatrix::from_row_slice(1, 3, &[1., 2., 3.]);  
        prob.softmax_mut();            
        assert!((BaseMatrix::get(&prob, 0, 0) - 0.09).abs() < 0.01);     
        assert!((BaseMatrix::get(&prob, 0, 1) - 0.24).abs() < 0.01);     
        assert!((BaseMatrix::get(&prob, 0, 2) - 0.66).abs() < 0.01); 
    }

    #[test]
    fn pow_mut(){
        let mut a = DMatrix::from_row_slice(1, 3, &[1., 2., 3.]);  
        a.pow_mut(3.);
        assert_eq!(a, DMatrix::from_row_slice(1, 3, &[1., 8., 27.]));
    }

    #[test]
    fn argmax(){
        let a = DMatrix::from_row_slice(3, 3, &[1., 2., 3., -5., -6., -7., 0.1, 0.2, 0.1]);
        let res = a.argmax();
        assert_eq!(res, vec![2, 0, 1]);
    }

    #[test]
    fn unique(){
        let a = DMatrix::from_row_slice(3, 3, &[1., 2., 2., -2., -6., -7., 2., 3., 4.]);
        let res = a.unique();
        assert_eq!(res.len(), 7);
        assert_eq!(res, vec![-7., -6., -2., 1., 2., 3., 4.]);
    }
    
}