use crate::{filter::PrimaryFilter, ring_buffer::Fixed};

pub struct CtcEngine {
    filter_a_l: Vec<PrimaryFilter>,
    filter_a_r: Vec<PrimaryFilter>,
    filter_b_l: Vec<PrimaryFilter>,
    filter_b_r: Vec<PrimaryFilter>,
    rb_l_90: Fixed<Vec<f32>>,
    rb_r_90: Fixed<Vec<f32>>,
    rb_l_idx: usize,
    rb_r_idx: usize,
    highpass_l: PrimaryFilter,
    highpass_r: PrimaryFilter,
    shadow_l: PrimaryFilter,
    shadow_r: PrimaryFilter,
    delay_l: usize,
    delay_r: usize,
}

impl CtcEngine {
    pub fn new(sample_rate: f32, delay: [usize; 2], cutoff: [f32; 2], hp_cutoff: f32) -> Self {
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
            highpass_l: PrimaryFilter::high_pass(hp_cutoff, sample_rate),
            highpass_r: PrimaryFilter::high_pass(hp_cutoff, sample_rate),
            shadow_l: PrimaryFilter::low_pass(cutoff[0], sample_rate),
            shadow_r: PrimaryFilter::low_pass(cutoff[1], sample_rate),
            delay_l: delay[0],
            delay_r: delay[1],
        }
    }
    
    pub fn process<'a>(&mut self, [l, r]: [f32; 2], attenuation: f32, amp_factors: &[f32; 4]) -> [f32; 2] {
        let fold_fn = |acc: f32, f: &mut PrimaryFilter| f.process(acc);

        let l_0 = self.filter_a_l.iter_mut().fold(l, fold_fn);
        let r_0 = self.filter_a_r.iter_mut().fold(r, fold_fn);

        let ct_l_shadow = self.shadow_l.process(self.rb_l_90[self.rb_l_idx]);
        let ct_r_shadow = self.shadow_r.process(self.rb_r_90[self.rb_r_idx]);

        let res_l = l_0 * amp_factors[0] - ct_r_shadow * attenuation * amp_factors[2];
        let res_r = r_0 * amp_factors[3] - ct_l_shadow * attenuation * amp_factors[1];

        let out_l = soft_saturate(res_l);
        let out_r = soft_saturate(res_r);

        let fb_l_90 = self.filter_b_l.iter_mut().fold(out_l, fold_fn);
        let fb_r_90 = self.filter_b_r.iter_mut().fold(out_r, fold_fn);
        
        self.rb_l_90[self.rb_l_idx] = self.highpass_l.process(fb_l_90);
        self.rb_r_90[self.rb_r_idx] = self.highpass_r.process(fb_r_90);

        self.rb_l_idx = (self.rb_l_idx + 1) % self.delay_l;
        self.rb_r_idx = (self.rb_r_idx + 1) % self.delay_r;

        [out_l, out_r]
    }
}

fn calc_allpass_coeffs(sample_rate: f32) -> (Vec<f32>, Vec<f32>) {
    let poles_a: [f32; 4] = [ 1.2524, 5.5671, 22.334, 121.18 ];
    let poles_b: [f32; 4] = [ 0.4709, 2.5112, 9.7360, 52.321 ];

    let calc = |p: f32| {
        let omega = 2.0 * std::f32::consts::PI * p * 150.0 / sample_rate;
        (1.0 - omega) / (1.0 + omega)
    };

    (
        poles_a.iter().map(|&p| calc(p)).collect(),
        poles_b.iter().map(|&p| calc(p)).collect()
    )
}

fn soft_saturate(x: f32) -> f32 {
    if x > 1.0 { 1.0 }
    else if x < -1.0 { -1.0 }
    else { x - (x.powi(3) / 3.0) }
}

