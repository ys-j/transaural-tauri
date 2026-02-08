use std::f32::consts::PI;

pub enum FilterType {
    AllPass,
    HighPass,
    LowPass,
}

pub struct PrimaryFilter {
    filter_type: FilterType,
    alpha: f32,
    prev_in: f32,
    prev_out: f32,
}

impl PrimaryFilter {
    pub fn all_pass(alpha: f32) -> Self {
        Self {
            filter_type: FilterType::AllPass,
            alpha,
            prev_in: 0.0,
            prev_out: 0.0,
        }
    }

    pub fn high_pass(cutoff: f32, sample_rate: f32) -> Self {
        let rc = 1.0 / (2.0 * PI * cutoff);
        let dt = 1.0 / sample_rate;
        let alpha = rc / (rc + dt);
        Self {
            filter_type: FilterType::HighPass,
            alpha,
            prev_in: 0.0,
            prev_out: 0.0,
        }
    }

    pub fn low_pass(cutoff: f32, sample_rate: f32) -> Self {
        let rc = 1.0 / (2.0 * PI * cutoff);
        let dt = 1.0 / sample_rate;
        let alpha = dt / (rc + dt);
        Self {
            filter_type: FilterType::LowPass,
            alpha,
            prev_in: 0.0,
            prev_out: 0.0,
        }
    }

    pub fn process(&mut self, input: f32) -> f32 {
        let output = match self.filter_type {
            FilterType::AllPass => {
                self.alpha * input + self.prev_in - self.alpha * self.prev_out
            }
            FilterType::HighPass => {
                self.alpha * (self.prev_out + input - self.prev_in)
            }
            FilterType::LowPass => {
                self.prev_out + self.alpha * (input - self.prev_out)
            }
        };
        self.prev_in = input;
        self.prev_out = output * 0.9999999;
        output
    }
}