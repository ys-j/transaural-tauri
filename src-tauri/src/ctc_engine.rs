use crate::filter::{Processable, PrimaryFilter, BiquadFilter};

pub struct CtcEngine {
    filter_a_l: Vec<PrimaryFilter>,
    filter_a_r: Vec<PrimaryFilter>,
    filter_b_l: Vec<PrimaryFilter>,
    filter_b_r: Vec<PrimaryFilter>,
    rb_l_0: [f64; 512],
    rb_r_0: [f64; 512],
    rb_idx: usize,
    main_delay_l: f64,
    main_delay_r: f64,
    rb_l_90: [f64; 512],
    rb_r_90: [f64; 512],
    low_pass_l: BiquadFilter,
    low_pass_r: BiquadFilter,
    high_pass_l: BiquadFilter,
    high_pass_r: BiquadFilter,
    low_shelf_l: BiquadFilter,
    low_shelf_r: BiquadFilter,
    ct_delay_l: f64,
    ct_delay_r: f64,
}

impl CtcEngine {
    pub fn new(
        sample_rate: f32,
        ct_delays: [f64; 2],
        main_delays: [f64; 2],
        lp_cutoffs: [f32; 2],
        hp_cutoff: f32,
        ls_cutoff: f32,
        ls_gain: f32
    ) -> Self {
        let (coeffs_a, coeffs_b) = calc_allpass_coeffs(sample_rate);
        println!("{:?}", main_delays);
        Self {
            filter_a_l: coeffs_a.iter().map(|&a| PrimaryFilter::all_pass(a)).collect(),
            filter_a_r: coeffs_a.iter().map(|&a| PrimaryFilter::all_pass(a)).collect(),
            filter_b_l: coeffs_b.iter().map(|&b| PrimaryFilter::all_pass(b)).collect(),
            filter_b_r: coeffs_b.iter().map(|&b| PrimaryFilter::all_pass(b)).collect(),
            rb_l_0: [0.0; 512],
            rb_r_0: [0.0; 512],
            rb_idx: 0,
            main_delay_l: main_delays[0],
            main_delay_r: main_delays[1],
            rb_l_90: [0.0; 512],
            rb_r_90: [0.0; 512],
            low_pass_l: BiquadFilter::low_pass(sample_rate, lp_cutoffs[0]),
            low_pass_r: BiquadFilter::low_pass(sample_rate, lp_cutoffs[1]),
            high_pass_l: BiquadFilter::high_pass(sample_rate, hp_cutoff),
            high_pass_r: BiquadFilter::high_pass(sample_rate, hp_cutoff),
            low_shelf_l: BiquadFilter::low_shelf(sample_rate, ls_cutoff, ls_gain),
            low_shelf_r: BiquadFilter::low_shelf(sample_rate, ls_cutoff, ls_gain),
            ct_delay_l: ct_delays[0],
            ct_delay_r: ct_delays[1],
        }
    }

    #[inline(always)]
    fn get_interpolated(&self, buffer: &[f64], current_idx: usize, delay: f64) -> f64 {
        let read_pos = current_idx as f64 - delay;
        
        let pos_floor = read_pos.floor();
        let idx_a = (pos_floor as i64).rem_euclid(512i64) as usize;
        let idx_b = (idx_a + 1) & 511;

        let frac = read_pos- pos_floor;

        unsafe {
            let val_a = *buffer.get_unchecked(idx_a);
            let val_b = *buffer.get_unchecked(idx_b);
            val_a + frac * (val_b - val_a)
        }
    }
    
    pub fn process(&mut self, [l, r]: [f32; 2], attenuation: f64, amp_factors: &[f64; 4]) -> [f32; 2] {
        let fold_fn = |acc: f64, f: &mut PrimaryFilter| f.process(acc);

        let l_in = l as f64;
        let r_in = r as f64;

        let l_0 = self.filter_a_l.iter_mut().fold(l_in, fold_fn);
        let r_0 = self.filter_a_r.iter_mut().fold(r_in, fold_fn);

        let ct_l_90_delayed = self.get_interpolated(&self.rb_l_90, self.rb_idx, self.ct_delay_l);
        let ct_r_90_delayed = self.get_interpolated(&self.rb_r_90, self.rb_idx, self.ct_delay_r);

        let ct_l_90 = self.low_pass_l.process(ct_l_90_delayed);
        let ct_r_90 = self.low_pass_r.process(ct_r_90_delayed);

        let res_l = l_0 * amp_factors[0] - ct_r_90 * attenuation * amp_factors[2];
        let res_r = r_0 * amp_factors[3] - ct_l_90 * attenuation * amp_factors[1];

        self.rb_l_0[self.rb_idx] = self.low_shelf_l.process(res_l);
        self.rb_r_0[self.rb_idx] = self.low_shelf_r.process(res_r);

        let out_l = self.get_interpolated(&self.rb_l_0, self.rb_idx, self.main_delay_l);
        let out_r = self.get_interpolated(&self.rb_r_0, self.rb_idx, self.main_delay_r);

        let fb_l_90 = self.filter_b_l.iter_mut().fold(l_in, fold_fn); // res_lは再帰型
        let fb_r_90 = self.filter_b_r.iter_mut().fold(r_in, fold_fn); // res_rは再帰型
        self.rb_l_90[self.rb_idx] = self.high_pass_l.process(fb_l_90);
        self.rb_r_90[self.rb_idx] = self.high_pass_r.process(fb_r_90);

        self.rb_idx = (self.rb_idx + 1) % 512;

        [ out_l.clamp(-1.0, 1.0) as f32, out_r.clamp(-1.0, 1.0) as f32 ]
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