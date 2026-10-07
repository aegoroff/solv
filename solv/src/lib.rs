#![warn(unused_extern_crates)]
#![allow(clippy::missing_errors_doc)]
pub mod error;
pub mod info;
pub mod json;
pub mod nuget;
pub mod ux;
pub mod validate;

use solp::Consume;

#[must_use]
pub fn calculate_percent(value: i32, total: i32) -> f64 {
    if total == 0 {
        0_f64
    } else {
        (f64::from(value) / f64::from(total)) * 100_f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    #[test_case(1, 100, 1.0)]
    #[test_case(0, 100, 0.0)]
    #[test_case(100, 100, 100.0)]
    #[test_case(50, 100, 50.0)]
    #[test_case(20, 100, 20.0)]
    fn calculate_percent_tests(value: i32, total: i32, expected: f64) {
        // Arrange

        // Act
        let actual = calculate_percent(value, total);

        // Assert
        assert_eq!(actual, expected);
    }
}
