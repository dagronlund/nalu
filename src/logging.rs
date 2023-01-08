use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct FrameTimestamps {
    start: Instant,
    sections: Vec<(String, Duration)>,
}

impl FrameTimestamps {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
            sections: Vec::new(),
        }
    }

    pub fn timestamp(&mut self, name: String) {
        let duration = self.start.elapsed();
        self.sections.push((name, duration));
        self.start = Instant::now();
    }

    pub fn total(&self) -> Duration {
        let mut total = Duration::new(0, 0);
        for (_, d) in &self.sections {
            total += *d;
        }
        total
    }

    pub fn get_sections(&self) -> &Vec<(String, Duration)> {
        &self.sections
    }
}

impl Default for FrameTimestamps {
    fn default() -> Self {
        Self::new()
    }
}
