use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Id {
    value: i64,
}

impl Id {
    #[must_use]
    pub const fn new(id: i64) -> Option<Self> {
        if id < 1 {
            None
        } else {
            Some(Self { value: id })
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
    fn test_only_positive_ids() {
        let id = Id::new(-1);
        assert!(id.is_none());

        let id = Id::new(0);
        assert!(id.is_none());

        let id = Id::new(1);
        assert!(id.is_some());
        assert!(id.unwrap().to_int() == 1);
    }
}
