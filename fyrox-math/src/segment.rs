use nalgebra::{
    allocator::Allocator, ArrayStorage, ClosedAdd, ClosedMul, ClosedSub, DefaultAllocator, Dim,
    OVector, RealField, Scalar, Storage, Vector, Vector2, U2, U3,
};
use num_traits::{One, Signed, Zero};
use rectutils::{Number, Rect};

/// Line segment in two dimensions
pub type LineSegment2<T> = LineSegment<T, U2>;
/// Line segment in three dimensions
pub type LineSegment3<T> = LineSegment<T, U3>;

/// Line segment in any number of dimensions
#[derive(Clone, Debug)]
pub struct LineSegment<T, D>
where
    DefaultAllocator: Allocator<T, D>,
    D: Dim,
{
    /// One end of the line segment, the point returned when interpolating at t = 0.0
    pub start: OVector<T, D>,
    /// One end of the line segment, the point returned when interpolating at t = 1.0
    pub end: OVector<T, D>,
}

impl<T, D> LineSegment<T, D>
where
    T: Zero + One + Scalar + ClosedAdd + ClosedSub + ClosedMul + RealField,
    D: Dim,
    DefaultAllocator: Allocator<T, D>,
{
    /// Create a new line segment with the given points.
    pub fn new<S1, S2>(start: &Vector<T, D, S1>, end: &Vector<T, D, S2>) -> Self
    where
        S1: Storage<T, D>,
        S2: Storage<T, D>,
    {
        Self {
            start: start.clone_owned(),
            end: end.clone_owned(),
        }
    }
    /// Creates a reversed line segment by swapping `start` and `end`.
    pub fn swapped(&self) -> Self {
        Self::new(&self.end, &self.start)
    }
    /// The two end-points of the line segment are equal.
    pub fn is_degenerate(&self) -> bool {
        self.start == self.end
    }
    /// Create a point somewhere between `start` and `end`.
    /// When t = 0.0, `start` is returned.
    /// When t = 1.0, `end` is returned.
    /// The result is `(1.0 - t) * start + t * end`, which may produce points off the line segment,
    /// if t < 0.0 or t > 1.0.
    pub fn interpolate(&self, t: T) -> OVector<T, D> {
        self.start.lerp(&self.end, t)
    }
    /// Create a point somewhere between `start` and `end`.
    /// This is just like [LineSegment::interpolate] except that t is clamped to between 0.0 and 1.0,
    /// so points off the line segment can never be returned.
    pub fn interpolate_clamped(&self, t: T) -> OVector<T, D> {
        self.interpolate(t.clamp(<T as Zero>::zero(), <T as One>::one()))
    }
    /// The vector from `start` to `end`
    pub fn vector(&self) -> OVector<T, D> {
        self.end.clone() - self.start.clone()
    }
    /// The distance between `start` and `end`
    pub fn length(&self) -> T {
        self.vector().norm()
    }
    /// The square of the distance between `start` and `end`
    pub fn length_squared(&self) -> T {
        self.vector().norm_squared()
    }
    /// The interpolation parameter of the point on this segment that is closest to the given point.
    ///
    /// [Stack Exchange question: Find a point on a line segment which is the closest to other point not on the line segment](https://math.stackexchange.com/questions/2193720/find-a-point-on-a-line-segment-which-is-the-closest-to-other-point-not-on-the-li)
    pub fn nearest_t<S>(&self, point: &Vector<T, D, S>) -> T
    where
        S: Storage<T, D>,
    {
        let v = self.vector();
        let u = self.start.clone() - point;
        let n2 = v.norm_squared();
        if n2.is_zero() {
            return T::zero();
        }
        -v.dot(&u) / n2
    }
    /// The point on this segment that is closest to the given point.
    pub fn nearest_point<S>(&self, point: &Vector<T, D, S>) -> OVector<T, D>
    where
        S: Storage<T, D>,
    {
        self.interpolate_clamped(self.nearest_t(point))
    }
    /// The squared distance between the given point and the nearest point on this line segment.
    pub fn distance_squared<S>(&self, point: &Vector<T, D, S>) -> T
    where
        S: Storage<T, D>,
    {
        (point - self.nearest_point(point)).norm_squared()
    }
    /// The distance between the given point and the nearest point on this line segment.
    pub fn distance<S>(&self, point: &Vector<T, D, S>) -> T
    where
        S: Storage<T, D>,
    {
        (point - self.nearest_point(point)).norm()
    }
}

impl<T> LineSegment2<T>
where
    T: Zero + One + Scalar + ClosedAdd + ClosedSub + ClosedMul + RealField,
    DefaultAllocator: Allocator<T, U2, Buffer = ArrayStorage<T, 2, 1>>,
{
    /// AABB for a 2D line segment
    pub fn bounds(&self) -> Rect<T>
    where
        T: Number,
    {
        Rect::from_points(self.start, self.end)
    }
    /// Test whether a point is collinear with this segment.
    /// * 0.0 means collinear. Near to 0.0 means near to collinear.
    /// * Negative means that the point is to the counter-clockwise of `end` as viewed from `start`.
    /// * Positive means that the point is to the clockwise of `end` as viewed from `start`.
    pub fn collinearity(&self, point: &Vector2<T>) -> T {
        let v = self.vector();
        let u = self.start.clone() - point;
        v.x.clone() * u.y.clone() - u.x.clone() * v.y.clone()
    }
    /// True if this segment intersects the given segment based on collinearity.
    pub fn intersects(&self, other: &LineSegment2<T>) -> bool {
        fn pos<T>(t: &T) -> bool
        where
            T: Zero + Signed,
        {
            t.is_positive() && !t.is_zero()
        }
        fn neg<T>(t: &T) -> bool
        where
            T: Zero + Signed,
        {
            t.is_negative() && !t.is_zero()
        }
        let o1 = self.collinearity(&other.start);
        let o2 = self.collinearity(&other.end);
        let s1 = other.collinearity(&self.start);
        let s2 = other.collinearity(&self.end);
        // If both points of self are left of `other` or both points are right of `other`...
        if neg(&s1) && neg(&s2) || pos(&s1) && pos(&s2) {
            return false;
        }
        // If both points of `other` are left of self or both points are right of self...
        if neg(&o1) && neg(&o2) || pos(&o1) && pos(&o2) {
            return false;
        }
        true
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use nalgebra::Vector2;
    #[test]
    fn nearest_at_start() {
        let segment = LineSegment2::new(&Vector2::new(0.0, 0.0), &Vector2::new(1.0, 2.0));
        assert_eq!(segment.nearest_t(&Vector2::new(-1.0, -1.0)).max(0.0), 0.0);
        assert_eq!(
            segment.nearest_point(&Vector2::new(-1.0, -1.0)),
            Vector2::new(0.0, 0.0)
        );
        assert_eq!(segment.distance_squared(&Vector2::new(-1.0, -1.0)), 2.0);
        assert_eq!(segment.distance(&Vector2::new(-1.0, 0.0)), 1.0);
    }
    #[test]
    fn nearest_at_end() {
        let segment = LineSegment2::new(&Vector2::new(0.0, 0.0), &Vector2::new(1.0, 2.0));
        assert_eq!(segment.nearest_t(&Vector2::new(2.0, 2.0)).min(1.0), 1.0);
        assert_eq!(
            segment.nearest_point(&Vector2::new(2.0, 2.0)),
            Vector2::new(1.0, 2.0)
        );
        assert_eq!(segment.distance_squared(&Vector2::new(3.0, 2.0)), 4.0);
        assert_eq!(segment.distance(&Vector2::new(3.0, 2.0)), 2.0);
    }
    #[test]
    fn nearest_in_middle() {
        let segment = LineSegment2::new(&Vector2::new(0.0, 0.0), &Vector2::new(1.0, 2.0));
        assert_eq!(segment.nearest_t(&Vector2::new(2.5, 0.0)), 0.5);
        assert_eq!(
            segment.nearest_point(&Vector2::new(2.5, 0.0)),
            Vector2::new(0.5, 1.0)
        );
        assert_eq!(segment.distance_squared(&Vector2::new(2.5, 0.0)), 5.0);
    }
    #[test]
    fn length() {
        let segment = LineSegment2::new(&Vector2::new(0.0, 0.0), &Vector2::new(4.0, 3.0));
        assert_eq!(segment.length_squared(), 25.0);
        assert_eq!(segment.length(), 5.0);
    }
    #[test]
    fn degenerate() {
        let segment = LineSegment2::new(&Vector2::new(1.0, 2.0), &Vector2::new(1.0, 2.0));
        assert!(segment.is_degenerate());
        assert_eq!(segment.length_squared(), 0.0);
        assert_eq!(segment.length(), 0.0);
    }
    #[test]
    fn collinear() {
        let segment = LineSegment2::new(&Vector2::new(0.0, 0.0), &Vector2::new(1.0, 2.0));
        assert_eq!(segment.collinearity(&Vector2::new(2.0, 4.0)), 0.0);
        assert_eq!(segment.collinearity(&Vector2::new(0.0, 0.0)), 0.0);
        assert_eq!(segment.collinearity(&Vector2::new(1.0, 2.0)), 0.0);
        assert!(
            segment.collinearity(&Vector2::new(1.0, 5.0)) < 0.0,
            "{} >= 0.0",
            segment.collinearity(&Vector2::new(1.0, 5.0))
        );
        assert!(
            segment.collinearity(&Vector2::new(1.0, 3.0)) < 0.0,
            "{} >= 0.0",
            segment.collinearity(&Vector2::new(1.0, 3.0))
        );
        assert!(
            segment.collinearity(&Vector2::new(1.0, 1.0)) > 0.0,
            "{} <= 0.0",
            segment.collinearity(&Vector2::new(1.0, 1.0))
        );
        assert!(
            segment.collinearity(&Vector2::new(-1.0, -5.0)) > 0.0,
            "{} <= 0.0",
            segment.collinearity(&Vector2::new(-1.0, -5.0))
        );
    }
    #[test]
    fn intersects() {
        let a = LineSegment::new(&Vector2::new(1.0, 2.0), &Vector2::new(3.0, 1.0));
        let b = LineSegment::new(&Vector2::new(2.0, 0.0), &Vector2::new(2.5, 3.0));
        let c = LineSegment::new(&Vector2::new(1.0, 2.0), &Vector2::new(-3.0, 1.0));
        assert!(a.intersects(&b));
        assert!(a.intersects(&c));
        assert!(b.intersects(&a));
        assert!(c.intersects(&a));
        assert!(a.swapped().intersects(&b));
        assert!(a.swapped().intersects(&c));
    }
    #[test]
    fn not_intersects() {
        let a = LineSegment::new(&Vector2::new(1.0, 2.0), &Vector2::new(3.0, 1.0));
        let b = LineSegment::new(&Vector2::new(0.0, 0.0), &Vector2::new(-1.0, 6.0));
        let c = LineSegment::new(&Vector2::new(2.0, 0.0), &Vector2::new(2.0, -1.0));
        assert!(!a.intersects(&b));
        assert!(!b.intersects(&c));
        assert!(!c.intersects(&a));
        assert!(!b.intersects(&a));
        assert!(!c.intersects(&b));
        assert!(!a.intersects(&c));
        assert!(!a.swapped().intersects(&b));
        assert!(!b.swapped().intersects(&c));
        assert!(!c.swapped().intersects(&a));
    }
}
