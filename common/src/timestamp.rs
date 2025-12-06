use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Timestamp {
    value: i64,
}

impl Timestamp {
    #[must_use]
    pub const fn new(timestamp: i64) -> Option<Self> {
        if timestamp < 1 {
            None
        } else {
            Some(Self { value: timestamp })
        }
    }

    #[allow(clippy::cast_sign_loss)]
    #[must_use]
    pub const fn to_int(&self) -> u64 {
        self.value as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_only_positive_timestamps() {
        let timestamp = Timestamp::new(-1);
        assert!(timestamp.is_none());

        let timestamp = Timestamp::new(0);
        assert!(timestamp.is_none());

        let timestamp = Timestamp::new(1);
        assert!(timestamp.is_some());
        assert!(timestamp.unwrap().to_int() == 1);
    }
}
