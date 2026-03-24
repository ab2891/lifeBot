use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PolicyConfig {
    pub minor_max_age: u8,
    pub minor_allowed_end_time: String,
    pub max_daily_hours: u8,
    pub max_weekly_hours: u8,
    pub min_gap_hours: u8,
}

#[derive(Debug, Clone)]
pub struct GuardContext {
    pub guard_id: String,
    pub name: String,
    pub date_of_birth: NaiveDate,
    pub certifications: Vec<(String, NaiveDate)>,
}

#[derive(Debug, Clone)]
pub struct ShiftContext {
    pub shift_id: String,
    pub site_id: String,
    pub role_id: String,
    pub shift_date: NaiveDate,
    pub start_time: String,
    pub end_time: String,
    pub required_certifications: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ExistingAssignment {
    pub shift_id: String,
    pub shift_date: NaiveDate,
    pub start_time: String,
    pub end_time: String,
}

#[derive(Debug, Clone)]
pub struct PolicyInput {
    pub guard: GuardContext,
    pub shift: ShiftContext,
    pub existing_assignments: Vec<ExistingAssignment>,
    pub policies: PolicyConfig,
}

#[derive(Debug, Error)]
#[error("{reason}")]
pub struct EligibilityError {
    pub reason: String,
}

pub fn evaluate_candidate(input: &PolicyInput) -> Result<(), EligibilityError> {
    check_certifications(input)?;
    check_minor_time_window(input)?;
    check_overlaps(input)?;
    check_daily_hours(input)?;
    check_weekly_hours(input)?;
    check_min_gap(input)?;
    Ok(())
}

fn check_certifications(input: &PolicyInput) -> Result<(), EligibilityError> {
    let today = Utc::now().date_naive();
    for required in &input.shift.required_certifications {
        let cert_name = cert_id_to_name(required);
        let has_valid = input
            .guard
            .certifications
            .iter()
            .any(|(name, expires_on)| name == cert_name && *expires_on >= today);
        if !has_valid {
            return Err(EligibilityError {
                reason: format!("Skipped because the required certification `{cert_name}` is missing or expired."),
            });
        }
    }
    Ok(())
}

fn check_minor_time_window(input: &PolicyInput) -> Result<(), EligibilityError> {
    let age = age_on(input.guard.date_of_birth, input.shift.shift_date);
    if age <= input.policies.minor_max_age as i64 {
        let allowed_end = parse_time(&input.policies.minor_allowed_end_time)?;
        let shift_end = parse_time(&input.shift.end_time)?;
        if shift_end > allowed_end {
            return Err(EligibilityError {
                reason: format!(
                    "Skipped because {} is {} and the shift ends after the allowed minor cutoff of {}.",
                    input.guard.name, age, input.policies.minor_allowed_end_time
                ),
            });
        }
    }
    Ok(())
}

fn check_overlaps(input: &PolicyInput) -> Result<(), EligibilityError> {
    let start = parse_time(&input.shift.start_time)?;
    let end = parse_time(&input.shift.end_time)?;
    for assignment in &input.existing_assignments {
        if assignment.shift_date == input.shift.shift_date {
            let other_start = parse_time(&assignment.start_time)?;
            let other_end = parse_time(&assignment.end_time)?;
            if start < other_end && other_start < end {
                return Err(EligibilityError {
                    reason: "Skipped because this shift overlaps an existing assignment.".into(),
                });
            }
        }
    }
    Ok(())
}

fn check_daily_hours(input: &PolicyInput) -> Result<(), EligibilityError> {
    let requested = duration_hours(&input.shift.start_time, &input.shift.end_time)?;
    let existing_same_day: f32 = input
        .existing_assignments
        .iter()
        .filter(|assignment| assignment.shift_date == input.shift.shift_date)
        .map(|assignment| duration_hours(&assignment.start_time, &assignment.end_time).unwrap_or(0.0))
        .sum();
    if existing_same_day + requested > input.policies.max_daily_hours as f32 {
        return Err(EligibilityError {
            reason: format!(
                "Skipped because it would exceed the daily limit of {} hours.",
                input.policies.max_daily_hours
            ),
        });
    }
    Ok(())
}

fn check_weekly_hours(input: &PolicyInput) -> Result<(), EligibilityError> {
    let requested = duration_hours(&input.shift.start_time, &input.shift.end_time)?;
    let existing_total: f32 = input
        .existing_assignments
        .iter()
        .map(|assignment| duration_hours(&assignment.start_time, &assignment.end_time).unwrap_or(0.0))
        .sum();
    if existing_total + requested > input.policies.max_weekly_hours as f32 {
        return Err(EligibilityError {
            reason: format!(
                "Skipped because it would exceed the cycle limit of {} hours.",
                input.policies.max_weekly_hours
            ),
        });
    }
    Ok(())
}

fn check_min_gap(input: &PolicyInput) -> Result<(), EligibilityError> {
    let shift_start = NaiveDateTime::new(input.shift.shift_date, parse_time(&input.shift.start_time)?);
    let shift_end = NaiveDateTime::new(input.shift.shift_date, parse_time(&input.shift.end_time)?);
    for assignment in &input.existing_assignments {
        let other_start = NaiveDateTime::new(assignment.shift_date, parse_time(&assignment.start_time)?);
        let other_end = NaiveDateTime::new(assignment.shift_date, parse_time(&assignment.end_time)?);
        let gap_before = (shift_start - other_end).num_hours().abs();
        let gap_after = (other_start - shift_end).num_hours().abs();
        if gap_before < input.policies.min_gap_hours as i64 || gap_after < input.policies.min_gap_hours as i64 {
            if assignment.shift_date != input.shift.shift_date {
                return Err(EligibilityError {
                    reason: format!(
                        "Skipped because the required {} hour gap between shifts is not met.",
                        input.policies.min_gap_hours
                    ),
                });
            }
        }
    }
    Ok(())
}

fn duration_hours(start: &str, end: &str) -> Result<f32, EligibilityError> {
    let start = parse_time(start)?;
    let end = parse_time(end)?;
    Ok((end - start).num_minutes() as f32 / 60.0)
}

fn parse_time(value: &str) -> Result<NaiveTime, EligibilityError> {
    NaiveTime::parse_from_str(value, "%H:%M").map_err(|_| EligibilityError {
        reason: format!("Invalid time value `{value}`"),
    })
}

fn cert_id_to_name(id: &str) -> &str {
    match id {
        "cert-lifeguard" => "Lifeguard",
        "cert-cpr" => "CPR/AED",
        "cert-waterfront" => "Waterfront",
        "cert-instructor" => "Swim Instruction",
        _ => id,
    }
}

fn age_on(dob: NaiveDate, on: NaiveDate) -> i64 {
    let mut years = on.year() - dob.year();
    if (on.month(), on.day()) < (dob.month(), dob.day()) {
        years -= 1;
    }
    years as i64
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;

    fn base_input() -> PolicyInput {
        PolicyInput {
            guard: GuardContext {
                guard_id: "guard-1".into(),
                name: "Jada".into(),
                date_of_birth: NaiveDate::from_ymd_opt(2009, 7, 11).unwrap(),
                certifications: vec![
                    ("Lifeguard".into(), NaiveDate::from_ymd_opt(2026, 10, 1).unwrap()),
                    ("CPR/AED".into(), NaiveDate::from_ymd_opt(2026, 10, 1).unwrap()),
                ],
            },
            shift: ShiftContext {
                shift_id: "shift-1".into(),
                site_id: "site-main".into(),
                role_id: "role-lifeguard".into(),
                shift_date: NaiveDate::from_ymd_opt(2026, 3, 24).unwrap(),
                start_time: "18:00".into(),
                end_time: "22:00".into(),
                required_certifications: vec!["cert-lifeguard".into(), "cert-cpr".into()],
            },
            existing_assignments: vec![],
            policies: PolicyConfig {
                minor_max_age: 17,
                minor_allowed_end_time: "20:00".into(),
                max_daily_hours: 8,
                max_weekly_hours: 24,
                min_gap_hours: 10,
            },
        }
    }

    #[test]
    fn blocks_minor_late_shift() {
        let input = base_input();
        let error = evaluate_candidate(&input).unwrap_err();
        assert!(error.reason.contains("minor cutoff"));
    }

    #[test]
    fn blocks_expired_certification() {
        let mut input = base_input();
        input.guard.date_of_birth = NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
        input.guard.certifications = vec![
            ("Lifeguard".into(), NaiveDate::from_ymd_opt(2026, 10, 1).unwrap()),
            ("CPR/AED".into(), NaiveDate::from_ymd_opt(2020, 10, 1).unwrap()),
        ];
        let error = evaluate_candidate(&input).unwrap_err();
        assert!(error.reason.contains("missing or expired"));
    }

    #[test]
    fn blocks_overlap() {
        let mut input = base_input();
        input.guard.date_of_birth = NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
        input.shift.end_time = "18:30".into();
        input.existing_assignments = vec![ExistingAssignment {
            shift_id: "other".into(),
            shift_date: NaiveDate::from_ymd_opt(2026, 3, 24).unwrap(),
            start_time: "17:00".into(),
            end_time: "19:00".into(),
        }];
        let error = evaluate_candidate(&input).unwrap_err();
        assert!(error.reason.contains("overlaps"));
    }
}
