const STEP: f32 = 1.25;
const MIN_SCALE: f32 = 0.05;
const MAX_SCALE: f32 = 32.0;

/// How content is scaled to the viewport.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Zoom {
    Fit,
    Scale(f32),
}

impl Zoom {
    pub fn scale(self, fit: f32) -> f32 {
        match self {
            Self::Fit => fit,
            Self::Scale(scale) => scale,
        }
    }

    pub fn zoomed_in(self, fit: f32) -> Self {
        self.multiplied(STEP, fit)
    }

    pub fn zoomed_out(self, fit: f32) -> Self {
        self.multiplied(STEP.recip(), fit)
    }

    pub fn multiplied(self, factor: f32, fit: f32) -> Self {
        Self::Scale((self.scale(fit) * factor).clamp(MIN_SCALE, MAX_SCALE))
    }

    pub fn percent(self, fit: f32) -> String {
        format!("{:.0}%", self.scale(fit) * 100.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zooming_starts_from_the_fitted_scale_and_stays_in_range() {
        assert_eq!(Zoom::Fit.zoomed_in(0.5), Zoom::Scale(0.625));
        assert_eq!(
            Zoom::Scale(MAX_SCALE).zoomed_in(1.0),
            Zoom::Scale(MAX_SCALE)
        );
        assert_eq!(Zoom::Fit.percent(0.5), "50%");
    }
}
