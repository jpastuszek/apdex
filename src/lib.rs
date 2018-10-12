pub extern crate yansi;
use yansi::Color;

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

    pub fn is_low_sample_size(&self) -> bool {
        let total = self.total();
        total > 0 && total < 100
    }

    pub fn score(&self) -> Option<f64> {
        let total = self.total();
        if total == 0 {
            None
        } else {
            Some((self.satisfied as f64 + (self.tolerating as f64 / 2.0)) / total as f64)
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
            if self.is_low_sample_size() {
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
}

use std::fmt;
impl fmt::Display for Apdex {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(score) = self.score() {
            let low_sample_indicator = if self.is_low_sample_size() {
                "*"
            } else {
                ""
            };

            write!(f, "{:.2}{} [{}]", score, low_sample_indicator, self.threshold)
        } else {
            write!(f, "NS [{}]", self.threshold)
        }
    }
}

impl<'i> fmt::Display for ApdexRating<'i> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let low_sample_indicator = if self.0.is_low_sample_size() {
            "*"
        } else {
            ""
        };

        write!(f, "{}{} [{}]", self.0.rating_word(), low_sample_indicator, self.0.threshold)
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
}
