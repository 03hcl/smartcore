use serde::{Serialize, Deserialize};

use crate::math::num::FloatExt;

use super::Distance;

#[derive(Serialize, Deserialize, Debug)]
pub struct Minkowski<T: FloatExt> {
    pub p: T
}

impl<T: FloatExt> Distance<Vec<T>, T> for Minkowski<T> {

    fn distance(&self, x: &Vec<T>, y: &Vec<T>) -> T {    
        if x.len() != y.len() {
            panic!("Input vector sizes are different");
        }
        if self.p < T::one() {
            panic!("p must be at least 1");
        }
    
        let mut dist = T::zero();
        for i in 0..x.len() {
            let d = (x[i] - y[i]).abs();
            dist = dist + d.powf(self.p);            
        }        
    
        dist.powf(T::one()/self.p)
    } 

}


#[cfg(test)]
mod tests {
    use super::*;    

    #[test]
    fn minkowski_distance() {
        let a = vec![1., 2., 3.];
        let b = vec![4., 5., 6.];             
        
        let l1: f64 = Minkowski{p: 1.0}.distance(&a, &b);
        let l2: f64 = Minkowski{p: 2.0}.distance(&a, &b);        
        let l3: f64 = Minkowski{p: 3.0}.distance(&a, &b);        

        assert!((l1 - 9.0).abs() < 1e-8);
        assert!((l2 - 5.19615242).abs() < 1e-8);                
        assert!((l3 - 4.32674871).abs() < 1e-8);
    }  
    
    #[test]
    #[should_panic(expected = "p must be at least 1")]
    fn minkowski_distance_negative_p() {
        let a = vec![1., 2., 3.];
        let b = vec![4., 5., 6.]; 
        
        let _: f64 = Minkowski{p: 0.0}.distance(&a, &b);
    }



}