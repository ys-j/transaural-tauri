use crate::{filter::{Processable, PrimaryFilter, BiquadFilter}, ring_buffer::Fixed};

pub struct CtcEngine {
    filter_a_l: Vec<PrimaryFilter>,
    filter_a_r: Vec<PrimaryFilter>,
    filter_b_l: Vec<PrimaryFilter>,
    filter_b_r: Vec<PrimaryFilter>,
    rb_l_90: Fixed<Vec<f32>>,
    rb_r_90: Fixed<Vec<f32>>,
    rb_l_idx: usize,
    rb_r_idx: usize,
    low_pass_l: BiquadFilter,
    low_pass_r: BiquadFilter,
    high_pass_l: BiquadFilter,
    high_pass_r: BiquadFilter,
    low_shelf_l: BiquadFilter,
    low_shelf_r: BiquadFilter,
    delay_l: usize,
    delay_r: usize,
}

impl CtcEngine {
    pub fn new(sample_rate: f32, delay: [usize; 2], lp_cutoffs: [f32; 2], hp_cutoff: f32, ls_cutoff: f32, ls_gain: f32) -> Self {
        let (coeffs_a, coeffs_b) = calc_allpass_coeffs(sample_rate);
        Self {
            filter_a_l: coeffs_a.iter().map(|&a| PrimaryFilter::all_pass(a)).collect(),
            filter_a_r: coeffs_a.iter().map(|&a| PrimaryFilter::all_pass(a)).collect(),
            filter_b_l: coeffs_b.iter().map(|&b| PrimaryFilter::all_pass(b)).collect(),
            filter_b_r: coeffs_b.iter().map(|&b| PrimaryFilter::all_pass(b)).collect(),
            rb_l_90: Fixed::from(vec![0.0; delay[0]]),
            rb_r_90: Fixed::from(vec![0.0; delay[1]]),
            rb_l_idx: 0,
            rb_r_idx: 0,
            low_pass_l: BiquadFilter::low_pass(sample_rate, lp_cutoffs[0]),
            low_pass_r: BiquadFilter::low_pass(sample_rate, lp_cutoffs[1]),
            high_pass_l: BiquadFilter::high_pass(sample_rate, hp_cutoff),
            high_pass_r: BiquadFilter::high_pass(sample_rate, hp_cutoff),
            low_shelf_l: BiquadFilter::low_shelf(sample_rate, ls_cutoff, ls_gain),
            low_shelf_r: BiquadFilter::low_shelf(sample_rate, ls_cutoff, ls_gain),
            delay_l: delay[0],
            delay_r: delay[1],
        }
    }
    
    pub fn process(&mut self, [l, r]: [f32; 2], attenuation: f32, amp_factors: &[f32; 4]) -> [f32; 2] {
        let fold_fn = |acc: f32, f: &mut PrimaryFilter| f.process(acc);

        let l_0 = self.filter_a_l.iter_mut().fold(l, fold_fn);
        let r_0 = self.filter_a_r.iter_mut().fold(r, fold_fn);

        let ct_l_90 = self.low_pass_l.process(self.rb_l_90[self.rb_l_idx]);
        let ct_r_90 = self.low_pass_r.process(self.rb_r_90[self.rb_r_idx]);

        let res_l = l_0 * amp_factors[0] - ct_r_90 * attenuation * amp_factors[2];
        let res_r = r_0 * amp_factors[3] - ct_l_90 * attenuation * amp_factors[1];

        let out_l = self.low_shelf_l.process(res_l).clamp(-1.0, 1.0);
        let out_r = self.low_shelf_r.process(res_r).clamp(-1.0, 1.0);

        let fb_l_90 = self.filter_b_l.iter_mut().fold(l, fold_fn); // res_lは再帰型
        let fb_r_90 = self.filter_b_r.iter_mut().fold(r, fold_fn); // res_rは再帰型
        
        self.rb_l_90[self.rb_l_idx] = self.high_pass_l.process(fb_l_90);
        self.rb_r_90[self.rb_r_idx] = self.high_pass_r.process(fb_r_90);

        self.rb_l_idx = (self.rb_l_idx + 1) % self.delay_l;
        self.rb_r_idx = (self.rb_r_idx + 1) % self.delay_r;

        [ out_l, out_r ]
    }
}

fn calc_allpass_coeffs(sample_rate: f32) -> (Vec<f32>, Vec<f32>) {
    let poles_a = [ 1.252477174013740, 5.567151121010343, 22.33405370220630, 121.1823101311035 ];
    let poles_b = [ 0.470942544153024, 2.511195608677685, 9.736028549641775, 52.32115162453549 ];

    let calc = |p: f64| {
        let omega = 2.0 * std::f64::consts::PI * (p as f64) * 150.0 / sample_rate as f64;
        (1.0 - omega) / (1.0 + omega)
    };

    (
        poles_a.iter().map(|&p| calc(p) as f32).collect(),
        poles_b.iter().map(|&p| calc(p) as f32).collect()
    )
}