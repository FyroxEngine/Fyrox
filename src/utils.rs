pub fn is_slice_equal_permutation<T: PartialEq>(a: &[T], b: &[T]) -> bool {
    if a.is_empty() && !b.is_empty() {
        false
    } else {
        // TODO: Find a way to do this faster.
        for source in a.iter() {
            let mut found = false;
            for other in b.iter() {
                if other == source {
                    found = true;
                    break;
                }
            }
            if !found {
                return false;
            }
        }
        true
    }
}
