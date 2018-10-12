pub extern crate yansi;
use yansi::Color;
use std::fmt;

/// https://docs.newrelic.com/docs/apm/new-relic-apm/apdex/apdex-measure-user-satisfaction
/// http://apdex.org/documents/ApdexTechnicalSpecificationV11_000.pdf
#[derive(Debug)]
pub struct Apdex {
    pub threshold: f64,
    pub satisfied: u64,
    pub tolerating: u64,
    pub frustrated: u64,
}

pub struct ApdexRating<'i>(&'i Apdex);

impl Default for Apdex {
    fn default() -> Apdex {
        Apdex::new(4.0)
    }
}

impl Apdex {
    pub fn new(threshold: f64) -> Apdex {
        Apdex {
            threshold,
            satisfied: 0,
            tolerating: 0,
            frustrated: 0,
        }
    }

    pub fn with_respnse_times(threshold: f64, response_times: impl IntoIterator<Item = Result<f64, ()>>) -> Apdex {
        response_times.into_iter().fold(Self::new(threshold), |mut apdex, response_time| {
            apdex.insert(response_time); 
        apdex})
    }

    pub fn with_hit_rate(threshold: f64, assumed_hit_rate: f64, response_times: impl IntoIterator<Item = Result<f64, ()>>) -> Apdex {
        let mut apdex = Self::with_respnse_times(threshold, response_times);

        let misses = apdex.total();
        let hits = (misses as f64 / (1.0 - assumed_hit_rate) - misses as f64).ceil() as u64;
        // Assuming hits will satisfy
        apdex.satisfied += hits;
        apdex
    }

    pub fn insert(&mut self, response_time: Result<f64, ()>) {
        if let Ok(response_time) = response_time {
            if response_time <= self.threshold {
                self.satisfied += 1;
            } else if response_time <= self.threshold * 4.0 {
                self.tolerating += 1;
            } else {
                self.frustrated += 1;
            }
        } else {
            // "If the tool can detect Task errors, then these application errors (e.g. Web page 404 replies) are counted as frustrated samples."
            self.frustrated += 1;
        }
    }

    pub fn total(&self) -> u64 {
        self.satisfied + self.tolerating + self.frustrated
    }

    pub fn no_samples(&self) -> bool {
        self.total() == 0
    }

    pub fn small_group(&self) -> bool {
        let total = self.total();
        total > 0 && total < 100
    }

    pub fn score(&self) -> Option<f64> {
        if self.no_samples() {
            None
        } else {
            Some((self.satisfied as f64 + (self.tolerating as f64 / 2.0)) / self.total() as f64)
        }
    }

    pub fn score_rating(&self) -> ApdexRating {
        ApdexRating(&self)
    }

    pub fn rating_word(&self) -> &'static str {
        if let Some(score) = self.score() {
            if score >= 0.94 {
                "Excellent"
            } else if score >= 0.85 {
                "Good"
            } else if score >= 0.70 {
                "Fair"
            } else if score >= 0.50 {
                "Poor"
            } else {
                "Unacceptable"
            }
        } else {
            "NoSample"
        }
    }

    pub fn color(&self) -> Color {
        if let Some(score) = self.score() {
            if self.small_group() {
                return Color::Unset
            }

            if score >= 0.94 {
                Color::Cyan
            } else if score >= 0.85 {
                Color::Green
            } else if score >= 0.70 {
                Color::Purple
            } else {
                Color::Red
            }
        } else {
            return Color::Unset
        }
    }

    fn write_threshold(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let low_sample_indicator = if self.small_group() {
            "*"
        } else {
            ""
        };

        if self.threshold < 10.0 {
            write!(f, " [{:.1}]{}", self.threshold, low_sample_indicator)
        } else {
            write!(f, " [{:.0}]{}", self.threshold, low_sample_indicator)
        }
    }
}

impl fmt::Display for Apdex {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(score) = self.score() {
            write!(f, "{:.2}", score)
        } else {
            write!(f, "NS")
        }?;
        self.write_threshold(f)
    }
}

impl<'i> fmt::Display for ApdexRating<'i> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0.rating_word())?;
        self.0.write_threshold(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn score() {
        let apdex = Apdex::with_respnse_times(1.0, [0.0, 0.1, 0.2, 0.5, 1.0, 4.0, 3.0, 2.0, 5.0].iter().cloned().map(Ok));
        assert!(apdex.score().unwrap() > 0.71);
        assert!(apdex.score().unwrap() < 0.73);
    }

    #[test]
    fn score_errors() {
        let apdex = Apdex::with_respnse_times(1.0, [Ok(0.0), Ok(0.1), Ok(0.2), Ok(0.5), Ok(1.0), Ok(4.0), Ok(3.0), Ok(2.0), Err(())].iter().cloned());
        assert!(apdex.score().unwrap() > 0.71);
        assert!(apdex.score().unwrap() < 0.73);
    }

    #[test]
    fn no_score() {
        let apdex = Apdex::default();
        assert!(apdex.score().is_none());
    }

    #[test]
    fn uniform_output_no_samples() {
        let apdex = Apdex::default();
        assert_eq!(format!("{}", apdex), "NS [4.0]");
    }

    #[test]
    fn uniform_output_no_samples_high_t() {
        let apdex = Apdex::new(10.0);
        assert_eq!(format!("{}", apdex), "NS [10]");
    }

    #[test]
    fn uniform_output_one_small_group() {
        let mut apdex = Apdex::default();
        apdex.insert(Ok(0.1));
        assert_eq!(format!("{}", apdex), "1.00 [4.0]*");
    }

    #[test]
    fn uniform_output_one() {
        let mut apdex = Apdex::default();
        for _i in 0..100 {
            apdex.insert(Ok(0.1));
        }
        assert_eq!(format!("{}", apdex), "1.00 [4.0]");
    }

    #[test]
    fn uniform_output() {
        let mut apdex = Apdex::default();
        for _i in 0..100 {
            apdex.insert(Ok(0.1));
        }
        for _i in 0..100 {
            apdex.insert(Ok(5.0));
        }
        assert_eq!(format!("{}", apdex), "0.75 [4.0]");
    }

    #[test]
    fn rating_output_no_samples_high_t() {
        let apdex = Apdex::new(10.0);
        assert_eq!(format!("{}", apdex.score_rating()), "NoSample [10]");
    }

    #[test]
    fn rating_output_no_samples() {
        let apdex = Apdex::default();
        assert_eq!(format!("{}", apdex.score_rating()), "NoSample [4.0]");
    }

    #[test]
    fn rating_output_small_group() {
        let mut apdex = Apdex::default();
        apdex.insert(Ok(0.1));
        assert_eq!(format!("{}", apdex.score_rating()), "Excellent [4.0]*");
    }

    #[test]
    fn rating_output() {
        let mut apdex = Apdex::default();
        for _i in 0..100 {
            apdex.insert(Ok(0.1));
        }
        for _i in 0..100 {
            apdex.insert(Ok(5.0));
        }
        assert_eq!(format!("{}", apdex.score_rating()), "Fair [4.0]");
    }
}
