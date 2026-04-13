use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::{id::Id, item::Item, language::Language};

#[derive(Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct Calendar {
    #[serde(default)]
    #[cfg_attr(feature = "utoipa", schema(value_type = i64))]
    pub id: Id,
    pub items: Vec<Item>,
    pub language: Language,
    pub name: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl Calendar {
    /// Validates that the calendar name meets character limits
    /// # Errors
    /// Returns Ok(()) if valid, Err with error message if invalid
    /// Valid if between 1 and 100 characters
    pub fn validate_name(&self) -> Result<(), String> {
        // Set character limit for calendar names
        const MAX_NAME_LENGTH: usize = 100;

        if self.name.is_empty() {
            return Err("Calendar name cannot be empty".to_string());
        }

        if self.name.chars().count() > MAX_NAME_LENGTH {
            return Err(format!(
                "Calendar name must be {MAX_NAME_LENGTH} characters or less"
            ));
        }

        Ok(())
    }

    /// Validates the entire calendar structure
    /// # Errors
    /// Returns Ok(()) if valid, Err with error message if invalid
    pub fn validate(&self) -> Result<(), String> {
        self.validate_name()?;
        Ok(())
    }
}
// TODO probably doesn't need to be a separate crate
