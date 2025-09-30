use std::f64::consts::PI;

/// Calculates the clockwise rotation angle in degrees from vector OA to vector OB,
/// where O is the origin point, A is the first point, and B is the second point.
///
/// # Arguments
/// * `a_x`, `a_y` - Coordinates of point A
/// * `o_x`, `o_y` - Coordinates of the origin point O
/// * `b_x`, `b_y` - Coordinates of point B
///
/// # Returns
/// The clockwise angle in degrees from vector OA to vector OB, in the range [0, 360).
/// Returns 0.0 if either vector is zero-length (undefined).
///
/// # Notes
/// - Assumes valid f64 inputs (non-NaN, non-infinite).
/// - Zero-length vectors (A=O or B=O) result in a 0.0 angle.
pub fn calculate_rotation_degrees(
    a_x: f64,
    a_y: f64,
    o_x: f64,
    o_y: f64,
    b_x: f64,
    b_y: f64,
) -> f64 {
    // Calculate vector components relative to origin
    let vec_a_x = a_x - o_x;
    let vec_a_y = a_y - o_y;
    let vec_b_x = b_x - o_x;
    let vec_b_y = b_y - o_y;

    // Handle zero-length vectors
    if (vec_a_x == 0.0 && vec_a_y == 0.0) || (vec_b_x == 0.0 && vec_b_y == 0.0) {
        return 0.0;
    }

    // Calculate the angles of both vectors relative to positive x-axis
    let angle_a = vec_a_y.atan2(vec_a_x);
    let angle_b = vec_b_y.atan2(vec_b_x);

    // Calculate the difference and normalize to [0, 2π)
    let mut angle_diff_radians = angle_b - angle_a;
    if angle_diff_radians < 0.0 {
        angle_diff_radians += 2.0 * PI;
    }

    // Convert radians to degrees
    angle_diff_radians * 180.0 / PI
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_right_angle_rotation() {
        let result = calculate_rotation_degrees(1.0, 0.0, 0.0, 0.0, 0.0, -1.0);
        assert!(
            (result - 90.0).abs() < 1e-10,
            "Expected 90.0, got {}",
            result
        );
    }

    #[test]
    fn test_180_degree_rotation() {
        let result = calculate_rotation_degrees(1.0, 0.0, 0.0, 0.0, -1.0, 0.0);
        assert!(
            (result - 180.0).abs() < 1e-10,
            "Expected 180.0, got {}",
            result
        );
    }

    #[test]
    fn test_full_rotation() {
        let result = calculate_rotation_degrees(1.0, 0.0, 0.0, 0.0, 1.0, 0.0);
        assert!((result - 0.0).abs() < 1e-10, "Expected 0.0, got {}", result);
    }

    #[test]
    fn test_arbitrary_rotation() {
        let result = calculate_rotation_degrees(1.0, 1.0, 0.0, 0.0, -1.0, 1.0);
        assert!(
            (result - 90.0).abs() < 1e-10,
            "Expected 90.0, got {}",
            result
        );
    }

    #[test]
    fn test_non_zero_origin() {
        let result = calculate_rotation_degrees(2.0, 1.0, 1.0, 1.0, 1.0, 0.0);
        assert!(
            (result - 90.0).abs() < 1e-10,
            "Expected 90.0, got {}",
            result
        );
    }

    #[test]
    fn test_zero_length_vector() {
        let result = calculate_rotation_degrees(0.0, 0.0, 0.0, 0.0, 1.0, 1.0);
        assert!(
            (result - 0.0).abs() < 1e-10,
            "Expected 0.0 for zero-length vector, got {}",
            result
        );
    }
}
