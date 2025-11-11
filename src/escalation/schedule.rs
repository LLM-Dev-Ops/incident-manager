use crate::error::{AppError, Result};
use crate::models::policy::{OnCallSchedule, RotationStrategy, ScheduleLayer, TimeRestrictions};
use chrono::{DateTime, Datelike, Duration, TimeZone, Timelike, Utc, Weekday};
use chrono_tz::Tz;

/// Resolves who is currently on-call based on schedule configuration
pub struct ScheduleResolver {
    /// Reference time for schedule calculations (defaults to now, but can be overridden for testing)
    reference_time: Option<DateTime<Utc>>,
}

impl ScheduleResolver {
    pub fn new() -> Self {
        Self {
            reference_time: None,
        }
    }

    /// Create resolver with a specific reference time (useful for testing)
    pub fn with_reference_time(reference_time: DateTime<Utc>) -> Self {
        Self {
            reference_time: Some(reference_time),
        }
    }

    /// Get the current time to use for calculations
    fn now(&self) -> DateTime<Utc> {
        self.reference_time.unwrap_or_else(Utc::now)
    }

    /// Resolve who is currently on-call for a schedule
    pub fn resolve_oncall(&self, schedule: &OnCallSchedule) -> Result<Vec<OnCallUser>> {
        let now = self.now();

        // Parse timezone
        let tz: Tz = schedule
            .timezone
            .parse()
            .map_err(|_| AppError::Validation(format!("Invalid timezone: {}", schedule.timezone)))?;

        // Convert current time to schedule's timezone
        let local_time = now.with_timezone(&tz);

        let mut oncall_users = Vec::new();

        // Process each layer
        for layer in &schedule.layers {
            if let Some(user) = self.resolve_layer_oncall(layer, &local_time, &tz)? {
                oncall_users.push(OnCallUser {
                    email: user,
                    layer_name: layer.name.clone(),
                    schedule_id: schedule.id,
                    schedule_name: schedule.name.clone(),
                });
            }
        }

        Ok(oncall_users)
    }

    /// Resolve who is on-call for a specific layer
    fn resolve_layer_oncall<Tz: TimeZone>(
        &self,
        layer: &ScheduleLayer,
        local_time: &DateTime<Tz>,
        tz: &Tz,
    ) -> Result<Option<String>> {
        // Check if this layer is currently active based on time restrictions
        if let Some(ref restrictions) = layer.restrictions {
            if !self.is_within_restrictions(local_time, restrictions) {
                return Ok(None);
            }
        }

        // If no users, no one is on-call
        if layer.users.is_empty() {
            return Ok(None);
        }

        // Calculate which user is on-call based on rotation strategy
        let user_index = self.calculate_rotation_index(layer, local_time, tz)?;
        let user = layer.users.get(user_index).cloned();

        Ok(user)
    }

    /// Check if current time is within the layer's time restrictions
    fn is_within_restrictions<Tz: TimeZone>(
        &self,
        local_time: &DateTime<Tz>,
        restrictions: &TimeRestrictions,
    ) -> bool {
        // Check day of week
        let current_day = local_time.weekday().num_days_from_sunday() as u8;
        if !restrictions.days_of_week.contains(&current_day) {
            return false;
        }

        // Check hour range
        let current_hour = local_time.hour();
        if restrictions.start_hour <= restrictions.end_hour {
            // Normal range (e.g., 9am to 5pm)
            if current_hour < restrictions.start_hour || current_hour >= restrictions.end_hour {
                return false;
            }
        } else {
            // Wraps around midnight (e.g., 10pm to 6am)
            if current_hour < restrictions.start_hour && current_hour >= restrictions.end_hour {
                return false;
            }
        }

        true
    }

    /// Calculate which user index is currently on-call based on rotation
    fn calculate_rotation_index<Tz: TimeZone>(
        &self,
        layer: &ScheduleLayer,
        local_time: &DateTime<Tz>,
        tz: &Tz,
    ) -> Result<usize> {
        let user_count = layer.users.len();
        if user_count == 0 {
            return Err(AppError::Validation("No users in schedule layer".to_string()));
        }

        match &layer.rotation {
            RotationStrategy::Daily { handoff_hour } => {
                self.calculate_daily_rotation(local_time, *handoff_hour, user_count, tz)
            }
            RotationStrategy::Weekly {
                handoff_day,
                handoff_hour,
            } => self.calculate_weekly_rotation(
                local_time,
                handoff_day,
                *handoff_hour,
                user_count,
                tz,
            ),
            RotationStrategy::Custom { duration_hours } => {
                self.calculate_custom_rotation(local_time, *duration_hours, user_count, tz)
            }
        }
    }

    /// Calculate rotation index for daily rotation
    fn calculate_daily_rotation<Tz: TimeZone>(
        &self,
        local_time: &DateTime<Tz>,
        handoff_hour: u32,
        user_count: usize,
        tz: &Tz,
    ) -> Result<usize> {
        // Calculate days since epoch
        let epoch = tz
            .with_ymd_and_hms(2020, 1, 1, handoff_hour, 0, 0)
            .single()
            .ok_or_else(|| AppError::Internal("Failed to create epoch time".to_string()))?;

        let duration = local_time.signed_duration_since(epoch);
        let days = duration.num_days();

        // Adjust if we haven't reached handoff hour today
        let adjusted_days = if local_time.hour() < handoff_hour {
            days - 1
        } else {
            days
        };

        Ok((adjusted_days as usize) % user_count)
    }

    /// Calculate rotation index for weekly rotation
    fn calculate_weekly_rotation<Tz: TimeZone>(
        &self,
        local_time: &DateTime<Tz>,
        handoff_day: &str,
        handoff_hour: u32,
        user_count: usize,
        tz: &Tz,
    ) -> Result<usize> {
        // Parse handoff day
        let target_weekday = parse_weekday(handoff_day)?;

        // Find the most recent handoff time
        let current_weekday = local_time.weekday();
        let days_since_handoff = if current_weekday == target_weekday {
            if local_time.hour() >= handoff_hour {
                0 // Handoff happened today
            } else {
                7 // Handoff is later today, so use last week's
            }
        } else {
            let current_day_num = current_weekday.num_days_from_monday();
            let target_day_num = target_weekday.num_days_from_monday();
            if current_day_num > target_day_num {
                current_day_num - target_day_num
            } else {
                7 - (target_day_num - current_day_num)
            }
        };

        // Calculate weeks since epoch
        let epoch = tz
            .with_ymd_and_hms(2020, 1, 6, handoff_hour, 0, 0) // 2020-01-06 was a Monday
            .single()
            .ok_or_else(|| AppError::Internal("Failed to create epoch time".to_string()))?;

        let duration = local_time.signed_duration_since(epoch);
        let total_days = duration.num_days() - days_since_handoff as i64;
        let weeks = total_days / 7;

        Ok((weeks as usize) % user_count)
    }

    /// Calculate rotation index for custom duration rotation
    fn calculate_custom_rotation<Tz: TimeZone>(
        &self,
        local_time: &DateTime<Tz>,
        duration_hours: u32,
        user_count: usize,
        tz: &Tz,
    ) -> Result<usize> {
        // Calculate rotations since epoch
        let epoch = tz
            .with_ymd_and_hms(2020, 1, 1, 0, 0, 0)
            .single()
            .ok_or_else(|| AppError::Internal("Failed to create epoch time".to_string()))?;

        let duration = local_time.signed_duration_since(epoch);
        let hours = duration.num_hours();
        let rotations = hours / duration_hours as i64;

        Ok((rotations as usize) % user_count)
    }

    /// Resolve all on-call users for a list of schedule IDs
    pub fn resolve_multiple(
        &self,
        schedules: &[OnCallSchedule],
    ) -> Result<Vec<OnCallUser>> {
        let mut all_users = Vec::new();

        for schedule in schedules {
            let users = self.resolve_oncall(schedule)?;
            all_users.extend(users);
        }

        Ok(all_users)
    }

    /// Get the next handoff time for a schedule layer
    pub fn next_handoff_time<Tz: TimeZone>(
        &self,
        layer: &ScheduleLayer,
        local_time: &DateTime<Tz>,
        tz: &Tz,
    ) -> Result<DateTime<Utc>> {
        match &layer.rotation {
            RotationStrategy::Daily { handoff_hour } => {
                let mut next = local_time
                    .date_naive()
                    .and_hms_opt(*handoff_hour, 0, 0)
                    .ok_or_else(|| AppError::Internal("Invalid handoff hour".to_string()))?;

                // If handoff already happened today, move to tomorrow
                if local_time.time() >= next.time() {
                    next += Duration::days(1);
                }

                Ok(tz
                    .from_local_datetime(&next)
                    .single()
                    .ok_or_else(|| AppError::Internal("Failed to convert to UTC".to_string()))?
                    .with_timezone(&Utc))
            }
            RotationStrategy::Weekly {
                handoff_day,
                handoff_hour,
            } => {
                let target_weekday = parse_weekday(handoff_day)?;
                let current_weekday = local_time.weekday();

                let days_until_handoff = if current_weekday == target_weekday {
                    if local_time.hour() >= *handoff_hour {
                        7 // Next week
                    } else {
                        0 // Today
                    }
                } else {
                    let current_day_num = current_weekday.num_days_from_monday();
                    let target_day_num = target_weekday.num_days_from_monday();
                    if target_day_num > current_day_num {
                        target_day_num - current_day_num
                    } else {
                        7 - (current_day_num - target_day_num)
                    }
                };

                let next = local_time.date_naive() + Duration::days(days_until_handoff as i64);
                let next = next
                    .and_hms_opt(*handoff_hour, 0, 0)
                    .ok_or_else(|| AppError::Internal("Invalid handoff hour".to_string()))?;

                Ok(tz
                    .from_local_datetime(&next)
                    .single()
                    .ok_or_else(|| AppError::Internal("Failed to convert to UTC".to_string()))?
                    .with_timezone(&Utc))
            }
            RotationStrategy::Custom { duration_hours } => {
                let next = *local_time + Duration::hours(*duration_hours as i64);
                Ok(next.with_timezone(&Utc))
            }
        }
    }
}

impl Default for ScheduleResolver {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents a user who is currently on-call
#[derive(Debug, Clone, PartialEq)]
pub struct OnCallUser {
    pub email: String,
    pub layer_name: String,
    pub schedule_id: uuid::Uuid,
    pub schedule_name: String,
}

/// Parse weekday from string
fn parse_weekday(day: &str) -> Result<Weekday> {
    match day.to_lowercase().as_str() {
        "monday" | "mon" => Ok(Weekday::Mon),
        "tuesday" | "tue" => Ok(Weekday::Tue),
        "wednesday" | "wed" => Ok(Weekday::Wed),
        "thursday" | "thu" => Ok(Weekday::Thu),
        "friday" | "fri" => Ok(Weekday::Fri),
        "saturday" | "sat" => Ok(Weekday::Sat),
        "sunday" | "sun" => Ok(Weekday::Sun),
        _ => Err(AppError::Validation(format!("Invalid weekday: {}", day))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::policy::ScheduleLayer;
    use chrono::TimeZone;
    use uuid::Uuid;

    fn create_test_schedule(layers: Vec<ScheduleLayer>) -> OnCallSchedule {
        OnCallSchedule {
            id: Uuid::new_v4(),
            name: "Test Schedule".to_string(),
            timezone: "America/New_York".to_string(),
            layers,
        }
    }

    #[test]
    fn test_daily_rotation() {
        let layer = ScheduleLayer {
            name: "Primary".to_string(),
            users: vec![
                "user1@example.com".to_string(),
                "user2@example.com".to_string(),
                "user3@example.com".to_string(),
            ],
            rotation: RotationStrategy::Daily { handoff_hour: 9 },
            restrictions: None,
        };

        let schedule = create_test_schedule(vec![layer]);

        // Test at 10am on a specific day
        let test_time = Utc.with_ymd_and_hms(2024, 1, 15, 15, 0, 0).unwrap(); // 10am EST
        let resolver = ScheduleResolver::with_reference_time(test_time);

        let result = resolver.resolve_oncall(&schedule).unwrap();
        assert_eq!(result.len(), 1);
        assert!(result[0].email.starts_with("user"));
    }

    #[test]
    fn test_weekly_rotation() {
        let layer = ScheduleLayer {
            name: "Primary".to_string(),
            users: vec![
                "user1@example.com".to_string(),
                "user2@example.com".to_string(),
            ],
            rotation: RotationStrategy::Weekly {
                handoff_day: "Monday".to_string(),
                handoff_hour: 9,
            },
            restrictions: None,
        };

        let schedule = create_test_schedule(vec![layer]);

        // Test on a Monday at 10am
        let test_time = Utc.with_ymd_and_hms(2024, 1, 15, 15, 0, 0).unwrap(); // Monday
        let resolver = ScheduleResolver::with_reference_time(test_time);

        let result = resolver.resolve_oncall(&schedule).unwrap();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_time_restrictions() {
        let layer = ScheduleLayer {
            name: "Business Hours".to_string(),
            users: vec!["user1@example.com".to_string()],
            rotation: RotationStrategy::Daily { handoff_hour: 9 },
            restrictions: Some(TimeRestrictions {
                days_of_week: vec![1, 2, 3, 4, 5], // Monday to Friday
                start_hour: 9,
                end_hour: 17,
            }),
        };

        let schedule = create_test_schedule(vec![layer]);

        // Test during business hours (should have someone on-call)
        let test_time = Utc.with_ymd_and_hms(2024, 1, 15, 15, 0, 0).unwrap(); // Monday 10am EST
        let resolver = ScheduleResolver::with_reference_time(test_time);
        let result = resolver.resolve_oncall(&schedule).unwrap();
        assert_eq!(result.len(), 1);

        // Test outside business hours (should have no one on-call)
        let test_time = Utc.with_ymd_and_hms(2024, 1, 15, 1, 0, 0).unwrap(); // Monday 8pm EST
        let resolver = ScheduleResolver::with_reference_time(test_time);
        let result = resolver.resolve_oncall(&schedule).unwrap();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_multiple_layers() {
        let primary = ScheduleLayer {
            name: "Primary".to_string(),
            users: vec!["primary@example.com".to_string()],
            rotation: RotationStrategy::Daily { handoff_hour: 9 },
            restrictions: None,
        };

        let secondary = ScheduleLayer {
            name: "Secondary".to_string(),
            users: vec!["secondary@example.com".to_string()],
            rotation: RotationStrategy::Daily { handoff_hour: 9 },
            restrictions: None,
        };

        let schedule = create_test_schedule(vec![primary, secondary]);

        let test_time = Utc.with_ymd_and_hms(2024, 1, 15, 15, 0, 0).unwrap();
        let resolver = ScheduleResolver::with_reference_time(test_time);

        let result = resolver.resolve_oncall(&schedule).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].layer_name, "Primary");
        assert_eq!(result[1].layer_name, "Secondary");
    }

    #[test]
    fn test_custom_rotation() {
        let layer = ScheduleLayer {
            name: "Primary".to_string(),
            users: vec![
                "user1@example.com".to_string(),
                "user2@example.com".to_string(),
            ],
            rotation: RotationStrategy::Custom { duration_hours: 12 },
            restrictions: None,
        };

        let schedule = create_test_schedule(vec![layer]);

        let test_time = Utc.with_ymd_and_hms(2024, 1, 15, 12, 0, 0).unwrap();
        let resolver = ScheduleResolver::with_reference_time(test_time);

        let result = resolver.resolve_oncall(&schedule).unwrap();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_parse_weekday() {
        assert_eq!(parse_weekday("Monday").unwrap(), Weekday::Mon);
        assert_eq!(parse_weekday("mon").unwrap(), Weekday::Mon);
        assert_eq!(parse_weekday("Friday").unwrap(), Weekday::Fri);
        assert_eq!(parse_weekday("fri").unwrap(), Weekday::Fri);
        assert!(parse_weekday("invalid").is_err());
    }

    #[test]
    fn test_weekend_restriction() {
        let layer = ScheduleLayer {
            name: "Weekend".to_string(),
            users: vec!["weekend@example.com".to_string()],
            rotation: RotationStrategy::Daily { handoff_hour: 0 },
            restrictions: Some(TimeRestrictions {
                days_of_week: vec![0, 6], // Sunday and Saturday
                start_hour: 0,
                end_hour: 24,
            }),
        };

        let schedule = create_test_schedule(vec![layer]);

        // Test on Saturday (should have someone on-call)
        let test_time = Utc.with_ymd_and_hms(2024, 1, 13, 15, 0, 0).unwrap(); // Saturday
        let resolver = ScheduleResolver::with_reference_time(test_time);
        let result = resolver.resolve_oncall(&schedule).unwrap();
        assert_eq!(result.len(), 1);

        // Test on Monday (should have no one on-call)
        let test_time = Utc.with_ymd_and_hms(2024, 1, 15, 15, 0, 0).unwrap(); // Monday
        let resolver = ScheduleResolver::with_reference_time(test_time);
        let result = resolver.resolve_oncall(&schedule).unwrap();
        assert_eq!(result.len(), 0);
    }
}
