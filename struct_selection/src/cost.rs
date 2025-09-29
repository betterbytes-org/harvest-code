/// Simplified big O complexity cost
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Cost {
    pub time: u64,     // exponent on N
    pub has_log: bool, // whether log(N) is in the complexity
}

impl Cost {
    /// Create a new cost with given time exponent and log factor
    pub fn new(time: u64, has_log: bool) -> Self {
        Self { time, has_log }
    }
}

impl Ord for Cost {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.time
            .cmp(&other.time)
            .then(self.has_log.cmp(&other.has_log))
    }
}

impl PartialOrd for Cost {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cmp::Ordering;

    #[test]
    fn test_cost_ordering_by_time_exponent() {
        // O(1) < O(N) < O(N^2) < O(N^3)
        let constant = Cost::new(0, false);
        let linear = Cost::new(1, false);
        let quadratic = Cost::new(2, false);

        assert!(constant < linear);
        assert!(linear < quadratic);
    }

    #[test]
    fn test_cost_ordering_time_dominates_log() {
        // Time exponent is more important than log factor
        // O(log N) < O(N) even though log has has_log=true
        let log = Cost::new(0, true);
        let linear = Cost::new(1, false);

        assert!(log < linear);
        assert_eq!(log.cmp(&linear), Ordering::Less);

        // O(N log N) < O(N^2)
        let linear_log = Cost::new(1, true);
        let quadratic = Cost::new(2, false);

        assert!(linear_log < quadratic);
        assert_eq!(linear_log.cmp(&quadratic), Ordering::Less);
    }
}
