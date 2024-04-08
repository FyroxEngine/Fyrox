use std::fmt::Debug;

pub trait CurvePoint {
    fn x(&self) -> f32;
    fn y(&self) -> f32;
}

pub fn simplify<P: CurvePoint + Clone + Debug>(
    points: &[P],
    epsilon: f32,
    max_step: f32,
) -> Vec<P> {
    find_important_points(points, epsilon, max_step)
        .into_iter()
        .map(|i| points[i].clone())
        .collect()
}

pub fn find_important_points<P: CurvePoint + Debug>(
    points: &[P],
    epsilon: f32,
    max_step: f32,
) -> Vec<usize> {
    if points.is_empty() {
        return Vec::new();
    }
    let mut keep_flags: Vec<bool> = Vec::new();
    keep_flags.resize(points.len(), false);
    let end = keep_flags.len() - 1;
    keep_flags[0] = true;
    keep_flags[end] = true;
    find_points_in_span(points, keep_flags.as_mut_slice(), 0, end, epsilon);
    if max_step.is_finite() {
        limit_step_size(points, keep_flags.as_mut_slice(), max_step);
    }
    let mut result: Vec<usize> = Vec::new();
    for (i, k) in keep_flags.into_iter().enumerate() {
        if k {
            result.push(i)
        }
    }
    if result.len() == 2 && f32::abs(points[result[0]].y() - points[result[1]].y()) < epsilon {
        result.pop();
    }
    result
}

fn limit_step_size<P: CurvePoint>(points: &[P], keep_flags: &mut [bool], max_step: f32) {
    let end = points.len() - 1;
    let mut i: usize = 1;
    while i < end {
        if keep_flags[i] {
            i += 1;
        } else {
            let next = find_step(i - 1, points, keep_flags, max_step);
            keep_flags[next] = true;
            i = usize::max(next + 1, i + 1);
        }
    }
}

fn find_step<P: CurvePoint>(
    start: usize,
    points: &[P],
    keep_flags: &mut [bool],
    max_step: f32,
) -> usize {
    let start_y = points[start].y();
    for i in start + 1..points.len() {
        let step = f32::abs(points[i].y() - start_y);
        if step > max_step {
            return usize::max(i - 1, start + 1);
        } else if keep_flags[i] {
            return i;
        }
    }
    points.len() - 1
}

#[allow(clippy::needless_range_loop)]
fn find_points_in_span<P: CurvePoint + Debug>(
    points: &[P],
    keep_flags: &mut [bool],
    start: usize,
    end: usize,
    epsilon: f32,
) {
    if end <= start + 1 {
        return;
    }
    let x0 = points[start].x();
    let y0 = points[start].y();
    let slope = (points[end].y() - y0) / (points[end].x() - x0);
    let mut far_point_index: usize = 0;
    let mut far_point_dist: f32 = 0.0;
    for i in start + 1..end {
        let (x, y) = (points[i].x(), points[i].y());
        let y_line: f32 = y0 + slope * (x - x0);
        let dist: f32 = (y - y_line).abs();
        if far_point_dist < dist {
            far_point_dist = dist;
            far_point_index = i;
        }
    }
    if far_point_index == 0 || far_point_dist < epsilon {
        return;
    }
    keep_flags[far_point_index] = true;
    find_points_in_span(points, keep_flags, start, far_point_index, epsilon);
    find_points_in_span(points, keep_flags, far_point_index, end, epsilon);
}

#[cfg(test)]
mod tests {
    use super::*;
    type Point = (f32, f32);
    impl CurvePoint for Point {
        fn x(&self) -> f32 {
            self.0
        }
        fn y(&self) -> f32 {
            self.1
        }
    }
    #[test]
    fn empty() {
        let points: Vec<Point> = Vec::new();
        let result = find_important_points(points.as_slice(), 0.001, f32::INFINITY);
        assert_eq!(result.len(), 0);
    }
    #[test]
    fn size_1() {
        let points: Vec<Point> = vec![(0.0, 0.0)];
        let result = find_important_points(points.as_slice(), 0.001, f32::INFINITY);
        assert_eq!(result, vec![0]);
    }
    #[test]
    fn size_2() {
        let points: Vec<Point> = vec![(0.0, 0.0), (1.0, 1.0)];
        let result = find_important_points(points.as_slice(), 0.001, f32::INFINITY);
        assert_eq!(result, vec![0, 1]);
    }
    #[test]
    fn size_3() {
        let points: Vec<Point> = vec![(0.0, 0.0), (1.0, 1.0), (2.0, 0.0)];
        let result = find_important_points(points.as_slice(), 0.001, f32::INFINITY);
        assert_eq!(result, vec![0, 1, 2]);
    }
    #[test]
    fn size_4() {
        let points: Vec<Point> = vec![(0.0, 0.0), (1.0, 1.0), (2.0, -1.0), (3.0, 0.0)];
        let result = find_important_points(points.as_slice(), 0.001, f32::INFINITY);
        assert_eq!(result, vec![0, 1, 2, 3]);
    }
    #[test]
    fn size_5() {
        let points: Vec<Point> = vec![(0.0, 0.0), (1.0, 1.0), (2.0, -1.0), (3.0, 0.0), (4.0, 0.0)];
        let result = find_important_points(points.as_slice(), 0.001, f32::INFINITY);
        assert_eq!(result, vec![0, 1, 2, 3, 4]);
    }
    #[test]
    fn irregular_x() {
        let points: Vec<Point> = vec![(0.0, 0.0), (1.0, 1.0), (4.0, -1.0), (6.0, 0.0), (10.0, 0.0)];
        let result = find_important_points(points.as_slice(), 0.001, f32::INFINITY);
        assert_eq!(result, vec![0, 1, 2, 3, 4]);
    }
    #[test]
    fn irregular_x_remove_2() {
        let points: Vec<Point> = vec![(0.0, 0.0), (1.0, 1.0), (4.0, 4.0), (6.0, 2.0), (10.0, -2.0)];
        let result = find_important_points(points.as_slice(), 0.001, f32::INFINITY);
        assert_eq!(result, vec![0, 2, 4]);
    }
    #[test]
    fn remove_middle() {
        let points: Vec<Point> = vec![(0.0, 0.0), (1.0, 1.0), (2.0, 2.0)];
        let result = find_important_points(points.as_slice(), 0.001, f32::INFINITY);
        assert_eq!(result, vec![0, 2]);
    }
    #[test]
    fn remove_all_but_one() {
        let points: Vec<Point> = vec![(0.0, 0.0), (1.0, 0.00001), (2.0, 0.0)];
        let result = find_important_points(points.as_slice(), 0.001, f32::INFINITY);
        assert_eq!(result, vec![0]);
    }
    #[test]
    fn remove_2() {
        let points: Vec<Point> = vec![(0.0, 0.0), (1.0, 1.0), (2.0, 2.0), (3.0, 1.0), (4.0, 0.0)];
        let result = find_important_points(points.as_slice(), 0.001, f32::INFINITY);
        assert_eq!(result, vec![0, 2, 4]);
    }
    #[test]
    fn small_step_size() {
        let points: Vec<Point> = vec![(0.0, 0.0), (1.0, 1.0), (2.0, 2.0), (3.0, 1.0), (4.0, 0.0)];
        let result = find_important_points(points.as_slice(), 0.001, 0.5);
        assert_eq!(result, vec![0, 1, 2, 3, 4]);
    }
    #[test]
    fn large_step_size() {
        let points: Vec<Point> = vec![(0.0, 0.0), (1.0, 1.0), (2.0, 2.0), (3.0, 1.0), (4.0, 0.0)];
        let result = find_important_points(points.as_slice(), 0.001, 2.0);
        assert_eq!(result, vec![0, 2, 4]);
    }
    #[test]
    fn mid_step_size() {
        let points: Vec<Point> = vec![(0.0, 0.0), (1.0, 1.0), (2.0, 2.0), (3.0, 1.5), (4.0, 1.0)];
        let result = find_important_points(points.as_slice(), 0.001, 1.0);
        assert_eq!(result, vec![0, 1, 2, 4]);
    }
}
