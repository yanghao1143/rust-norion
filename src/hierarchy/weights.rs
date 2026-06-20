#[derive(Debug, Clone, Copy)]
pub struct HierarchyWeights {
    pub global: f32,
    pub local: f32,
    pub convolution: f32,
}

impl HierarchyWeights {
    pub fn new(global: f32, local: f32, convolution: f32) -> Self {
        let mut weights = Self {
            global,
            local,
            convolution,
        };
        weights.normalize();
        weights
    }

    pub fn normalize(&mut self) {
        self.global = self.global.max(0.0);
        self.local = self.local.max(0.0);
        self.convolution = self.convolution.max(0.0);

        let sum = self.global + self.local + self.convolution;
        if sum <= f32::EPSILON {
            self.global = 0.34;
            self.local = 0.33;
            self.convolution = 0.33;
            return;
        }

        self.global /= sum;
        self.local /= sum;
        self.convolution /= sum;
    }

    pub fn blend(self, target: Self, rate: f32) -> Self {
        let rate = rate.clamp(0.0, 1.0);
        Self::new(
            self.global * (1.0 - rate) + target.global * rate,
            self.local * (1.0 - rate) + target.local * rate,
            self.convolution * (1.0 - rate) + target.convolution * rate,
        )
    }
}

impl Default for HierarchyWeights {
    fn default() -> Self {
        Self::new(0.36, 0.42, 0.22)
    }
}
