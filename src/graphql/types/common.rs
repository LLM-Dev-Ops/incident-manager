//! Common GraphQL types and scalars

use async_graphql::*;
use chrono::{DateTime, Utc};

/// Custom DateTime scalar that uses chrono::DateTime<Utc>
#[derive(Clone, Copy, Debug)]
pub struct DateTimeScalar(pub DateTime<Utc>);

#[Scalar]
impl ScalarType for DateTimeScalar {
    fn parse(value: Value) -> InputValueResult<Self> {
        if let Value::String(s) = value {
            Ok(DateTimeScalar(
                DateTime::parse_from_rfc3339(&s)
                    .map_err(|e| InputValueError::custom(format!("Invalid datetime: {}", e)))?
                    .with_timezone(&Utc),
            ))
        } else {
            Err(InputValueError::expected_type(value))
        }
    }

    fn to_value(&self) -> Value {
        Value::String(self.0.to_rfc3339())
    }
}

impl From<DateTime<Utc>> for DateTimeScalar {
    fn from(dt: DateTime<Utc>) -> Self {
        DateTimeScalar(dt)
    }
}

impl From<DateTimeScalar> for DateTime<Utc> {
    fn from(scalar: DateTimeScalar) -> Self {
        scalar.0
    }
}

/// Pagination input for queries
#[derive(InputObject, Debug, Clone)]
pub struct PaginationInput {
    /// Page number (0-indexed)
    #[graphql(default = 0)]
    pub page: u32,

    /// Number of items per page (max 100)
    #[graphql(default = 20, validator(maximum = 100, minimum = 1))]
    pub page_size: u32,
}

impl Default for PaginationInput {
    fn default() -> Self {
        Self {
            page: 0,
            page_size: 20,
        }
    }
}

/// Pagination information for responses
#[derive(SimpleObject, Debug, Clone)]
pub struct PageInfo {
    /// Current page number
    pub page: u32,

    /// Items per page
    pub page_size: u32,

    /// Total number of items
    pub total_count: u64,

    /// Total number of pages
    pub total_pages: u32,

    /// Whether there is a next page
    pub has_next_page: bool,

    /// Whether there is a previous page
    pub has_previous_page: bool,
}

impl PageInfo {
    pub fn new(page: u32, page_size: u32, total_count: u64) -> Self {
        let total_pages = ((total_count as f64) / (page_size as f64)).ceil() as u32;
        let has_next_page = page + 1 < total_pages;
        let has_previous_page = page > 0;

        Self {
            page,
            page_size,
            total_count,
            total_pages,
            has_next_page,
            has_previous_page,
        }
    }
}

/// Sorting order
#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum SortOrder {
    /// Ascending order
    Asc,
    /// Descending order
    Desc,
}

impl Default for SortOrder {
    fn default() -> Self {
        SortOrder::Desc
    }
}
