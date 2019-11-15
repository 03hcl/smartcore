use std::default::Default;
use crate::linalg::Vector;
use crate::optimization::{F, DF};
use crate::optimization::line_search::LineSearchMethod;
use crate::optimization::first_order::{FirstOrderOptimizer, OptimizerResult};

pub struct LBFGS {
    pub max_iter: usize,
    pub g_rtol: f64,
    pub g_atol: f64,
    pub x_atol: f64,  
    pub x_rtol: f64,
    pub f_abstol: f64,
    pub f_reltol: f64,
    pub successive_f_tol: usize,
    pub m: usize
}

impl Default for LBFGS {
    fn default() -> Self { 
        LBFGS {
            max_iter: 1000,
            g_rtol: 1e-8,
            g_atol: 1e-8,
            x_atol: 0.,
            x_rtol: 0.,
            f_abstol: 0.,
            f_reltol: 0.,
            successive_f_tol: 1,
            m: 10
        }
     }
}

impl LBFGS {

    fn two_loops<X: Vector>(&self, state: &mut LBFGSState<X>) {        

        let lower = state.iteration.max(self.m) - self.m;
        let upper = state.iteration;

        state.twoloop_q.copy_from(&state.dx);         

        for index in (lower..upper).rev() {                 
            let i = index.rem_euclid(self.m);                
            let dgi = &state.dg_history[i];
            let dxi = &state.dx_history[i];            
            state.twoloop_alpha[i] = state.rho[i] * dxi.dot(&state.twoloop_q);
            state.twoloop_q.sub_mut(&dgi.mul_scalar(state.twoloop_alpha[i]));            
        }

        if state.iteration > 0 {                            
            let i = (upper - 1).rem_euclid(self.m);            
            let dxi = &state.dx_history[i];
            let dgi = &state.dg_history[i];
            let scaling = dxi.dot(dgi) / dgi.abs().pow_mut(2.).sum();            
            state.s.copy_from(&state.twoloop_q.mul_scalar(scaling));
        } else {
            state.s.copy_from(&state.twoloop_q);
        }            

        for index in lower..upper {
            let i = index.rem_euclid(self.m);
            let dgi = &state.dg_history[i];
            let dxi = &state.dx_history[i];                 
            let beta = state.rho[i] * dgi.dot(&state.s);              
            state.s.add_mut(&dxi.mul_scalar(state.twoloop_alpha[i] - beta));                             
        }        

        state.s.mul_scalar_mut(-1.);          
         
    }

    fn init_state<X: Vector>(&self, x: &X) -> LBFGSState<X> {
        LBFGSState {
            x: x.clone(),
            x_prev: x.clone(),
            fx: std::f64::NAN,
            g_prev: x.clone(),
            rho: vec![0.; self.m],
            dx_history: vec![x.clone(); self.m],
            dg_history: vec![x.clone(); self.m],
            dx: x.clone(),
            dg: x.clone(),            
            fx_prev: std::f64::NAN,
            twoloop_q: x.clone(),
            twoloop_alpha: vec![0.; self.m],
            iteration: 0,
            counter_f_tol: 0,
            s: x.clone(),
            alpha: 1.0
        }
    }

    fn update_state<'a, X: Vector, LS: LineSearchMethod>(&self, f: &'a F<X>, df: &'a DF<X>, ls: &'a LS, state: &mut LBFGSState<X>) {
        df(&mut state.dx, &state.x); 

        self.two_loops(state);

        df(&mut state.g_prev, &state.x);

        let df0 = state.dx.dot(&state.s);

        state.fx_prev = f(&state.x);
        state.x_prev.copy_from(&state.x);

        let f_alpha = |alpha: f64| -> f64 {
            let mut dx = state.s.clone();
            dx.mul_scalar_mut(alpha);
            f(&dx.add_mut(&state.x)) // f(x) = f(x .+ gvec .* alpha)
        };

        let df_alpha = |alpha: f64| -> f64 {                
            let mut dx = state.s.clone();
            let mut dg = state.dx.clone();
            dx.mul_scalar_mut(alpha);
            df(&mut dg, &dx.add_mut(&state.x)); //df(x) = df(x .+ gvec .* alpha)
            state.dx.dot(&dg)
        };                    

        let ls_r = ls.search(&f_alpha, &df_alpha, 1.0, state.fx_prev, df0);
        state.alpha = ls_r.alpha;                    

        state.dx.copy_from(state.s.mul_scalar_mut(state.alpha));
        state.x.add_mut(&state.dx); 
    }

    fn assess_convergence<X: Vector>(&self, state: &mut LBFGSState<X>) -> bool {
        let (mut x_converged, mut g_converged) = (false, false);

        if state.x.max_diff(&state.x_prev) <= self.x_atol {
            x_converged = true;
        }

        if state.x.max_diff(&state.x_prev) <= self.x_rtol * state.x.norm(std::f64::INFINITY) {
            x_converged = true;
        }            

        if (state.fx - state.fx_prev).abs() <= self.f_abstol {                
            state.counter_f_tol += 1;
        }

        if (state.fx - state.fx_prev).abs() <= self.f_reltol * state.fx.abs() {
            state.counter_f_tol += 1;
        }

        if state.dx.norm(std::f64::INFINITY) <= self.g_atol {
            g_converged = true;
        }            

        g_converged || x_converged || state.counter_f_tol > self.successive_f_tol
    }

    fn update_hessian<'a, X: Vector>(&self, df: &'a DF<X>, state: &mut LBFGSState<X>) {
        let mut dx = state.dx.clone();
        df(&mut dx, &state.x);                
        state.dg = dx.sub(&state.g_prev);
        let rho_iteration = 1. / state.dx.dot(&state.dg);
        if !rho_iteration.is_infinite() {
            let idx = state.iteration.rem_euclid(self.m);                                               
            state.dx_history[idx].copy_from(&state.dx);
            state.dg_history[idx].copy_from(&state.dg);
            state.rho[idx] = rho_iteration;
        }  
    }
}

struct LBFGSState<X: Vector> {
    x: X,
    x_prev: X,
    fx: f64,
    g_prev: X,
    rho: Vec<f64>,
    dx_history: Vec<X>,
    dg_history: Vec<X>,
    dx: X,
    dg: X,    
    fx_prev: f64,
    twoloop_q: X,
    twoloop_alpha: Vec<f64>,
    iteration: usize,
    counter_f_tol: usize,
    s: X,
    alpha: f64
}

impl FirstOrderOptimizer for LBFGS {

    fn optimize<'a, X: Vector, LS: LineSearchMethod>(&self, f: &'a F<X>, df: &'a DF<X>, x0: &X, ls: &'a LS) -> OptimizerResult<X> {     
        
        let mut state = self.init_state(x0);

        df(&mut state.dx, &x0);                 

        let g_converged = state.dx.norm(std::f64::INFINITY) < self.g_atol;
        let mut converged = g_converged;
        let stopped = false;        

        while !converged && !stopped && state.iteration < self.max_iter {    
            
            self.update_state(f, df, ls, &mut state);

            state.fx = f(&state.x);                      

            converged = self.assess_convergence(&mut state);

            if !converged { 
                self.update_hessian(df, &mut state);
            }            

            state.iteration += 1;

        }        

        OptimizerResult{
            x: state.x,
            f_x: state.fx,
            iterations: state.iteration
        }
        
    }

}

#[cfg(test)]
mod tests {    
    use super::*; 
    use crate::linalg::naive::dense_vector::DenseVector;
    use crate::optimization::line_search::Backtracking;
    use crate::optimization::FunctionOrder;
    use crate::math::EPSILON;

    #[test]
    fn lbfgs() { 
        let x0 = DenseVector::from_array(&[0., 0.]);
        let f = |x: &DenseVector| {                
            (1.0 - x.get(0)).powf(2.) + 100.0 * (x.get(1) - x.get(0).powf(2.)).powf(2.)
        };

        let df = |g: &mut DenseVector, x: &DenseVector| {                                         
            g.set(0, -2. * (1. - x.get(0)) - 400. * (x.get(1) - x.get(0).powf(2.)) * x.get(0));
            g.set(1, 200. * (x.get(1) - x.get(0).powf(2.)));                
        };
        let mut ls: Backtracking = Default::default();
        ls.order = FunctionOrder::THIRD;
        let optimizer: LBFGS = Default::default();
        
        let result = optimizer.optimize(&f, &df, &x0, &ls);
        
        assert!((result.f_x - 0.0).abs() < EPSILON);        
        assert!((result.x.get(0) - 1.0).abs() < 1e-8);
        assert!((result.x.get(1) - 1.0).abs() < 1e-8);
        assert!(result.iterations <= 24);
    }
}