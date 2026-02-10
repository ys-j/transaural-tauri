use std::f64::consts::PI;

pub enum PrimaryFilterType {
    AllPass,
    HighPass,
    LowPass,
}

pub trait Processable {
    fn process(&mut self, input: f64) -> f64;
}

pub struct PrimaryFilter {
    filter_type: PrimaryFilterType,
    alpha: f64,
    prev_in: f64,
    prev_out: f64,
}

impl PrimaryFilter {
    pub fn all_pass(alpha: f32) -> Self {
        Self {
            filter_type: PrimaryFilterType::AllPass,
            alpha: alpha as f64,
            prev_in: 0.0,
            prev_out: 0.0,
        }
    }

    pub fn high_pass(sample_rate: f32, cutoff: f32) -> Self {
        let rc = 1.0 / (2.0 * PI * cutoff as f64);
        let dt = 1.0 / (sample_rate as f64);
        let alpha = rc / (rc + dt);
        Self {
            filter_type: PrimaryFilterType::HighPass,
            alpha,
            prev_in: 0.0,
            prev_out: 0.0,
        }
    }

    pub fn low_pass(sample_rate: f32, cutoff: f32) -> Self {
        let rc = 1.0 / (2.0 * PI * cutoff as f64);
        let dt = 1.0 / (sample_rate as f64);
        let alpha = dt / (rc + dt);
        Self {
            filter_type: PrimaryFilterType::LowPass,
            alpha,
            prev_in: 0.0,
            prev_out: 0.0,
        }
    }
}

impl Processable for PrimaryFilter {
    fn process(&mut self, input: f64) -> f64 {
        let out64 = match self.filter_type {
            PrimaryFilterType::AllPass => {
                self.alpha * input + self.prev_in - self.alpha * self.prev_out
            }
            PrimaryFilterType::HighPass => {
                self.alpha * (self.prev_out + input - self.prev_in)
            }
            PrimaryFilterType::LowPass => {
                self.prev_out + self.alpha * (input - self.prev_out)
            }
        };
        self.prev_in = input;
        self.prev_out = out64;
        out64
    }
}

pub struct BiquadFilter {
    b0: f64,
    b1: f64, b2: f64,
    a1: f64, a2: f64,
    z1: f64, z2: f64,
}

impl BiquadFilter {
    fn new(b0: f64, b1: f64, b2: f64, a0: f64, a1: f64, a2: f64) -> Self {
        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
            z1: 0.0,
            z2: 0.0
        }
    }

    pub fn low_pass(sample_rate: f32, cutoff: f32) -> Self {
        let q = 0.70710678118;
        let omega = 2.0 * PI * cutoff as f64 / sample_rate as f64;
        let cos_w = omega.cos();
        let alpha = omega.sin() / (2.0 * q);

        Self::new(
            (1.0 - cos_w) / 2.0,
            1.0 - cos_w,
            (1.0 - cos_w) / 2.0,
            1.0 + alpha,
            -2.0 * cos_w,
            1.0 - alpha,
        )
    }

    pub fn high_pass(sample_rate: f32, cutoff: f32) -> Self {
        let q = 0.70710678118;
        let omega = 2.0 * PI * cutoff as f64 / sample_rate as f64;
        let cos_w = omega.cos();
        let alpha = omega.sin() / (2.0 * q);

        Self::new(
            (1.0 + cos_w) / 2.0,
            -(1.0 + cos_w),
            (1.0 + cos_w) / 2.0,
            1.0 + alpha,
            -2.0 * cos_w,
            1.0 - alpha,
        )
    }

    pub fn low_shelf(sample_rate: f32, cutoff: f32, gain_db: f32) -> Self {
        let q = 0.707;
        let a = 10.0f64.powf(gain_db as f64 / 40.0);
        let omega = 2.0 * PI * cutoff as f64 / sample_rate as f64;
        let cos_w = omega.cos();
        let beta = (a + 1.0 / a) * (1.0 / q - 1.0) + 2.0;
        let alpha = omega.sin() / 2.0 * beta.max(0.0).sqrt();

        Self::new(
            a * ((a + 1.0) - (a - 1.0) * cos_w + 2.0 * a.sqrt() * alpha),
            2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w),
            a * ((a + 1.0) - (a - 1.0) * cos_w - 2.0 * a.sqrt() * alpha),
            (a + 1.0) + (a - 1.0) * cos_w + 2.0 * a.sqrt() * alpha,
            -2.0 * ((a - 1.0) + (a + 1.0) * cos_w),
            (a + 1.0) + (a - 1.0) * cos_w - 2.0 * a.sqrt() * alpha,
        )
    }
}

impl Processable for BiquadFilter {
    fn process(&mut self, input: f64) -> f64 {
        let output = self.b0 * input + self.z1;

        if output.is_finite() {
            self.z1 = self.b1 * input - self.a1 * output + self.z2;
            self.z2 = self.b2 * input - self.a2 * output;
            if output.abs() < f64::EPSILON { 0.0 }
            else { output }
        } else {
            self.z1 = 0.0;
            self.z2 = 0.0;
            0.0
        }
    }
}