/// Iterator that skips elements of its inner iterator so that it
/// returns every nth element, starting from a given offset from the
/// start of the inner iterator.
///
/// For example: [0, 1, 2, 3, 4, 5, 6, 7, 8]
/// Becomes: [1, 4, 7]
///
/// In this case, it is returning the second element within each three elements.
#[derive(Debug, Clone)]
pub struct SelectIterator<I> {
    source: I,
    offset: usize,
    window_size: usize,
    starting: bool,
}

/// Create an iterator that skips elements of the given iterator to produce a single element
/// within each group of window_size elements. The element returned is `offset` elements from the
/// start of each window.
///
/// For example: select_iterator(iter, 3, 5)
/// If iter produces: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
/// select_iterartor(iter, 3, 5]) procues: [3, 8]
pub fn select_iterator<I>(source: I, offset: usize, window_size: usize) -> SelectIterator<I> {
    SelectIterator {
        source,
        offset,
        window_size,
        starting: true,
    }
}

impl<I> Iterator for SelectIterator<I>
where
    I: Iterator,
{
    type Item = <I as Iterator>::Item;
    fn next(&mut self) -> Option<Self::Item> {
        if self.starting {
            self.starting = false;
            for _ in 0..self.offset {
                self.source.next()?;
            }
            self.source.next()
        } else {
            for _ in 0..self.window_size - 1 {
                self.source.next()?;
            }
            self.source.next()
        }
    }
}

pub fn iter_cubic_data<I, O>(
    inputs: I,
    outputs: O,
) -> impl Iterator<Item = (<I as Iterator>::Item, [<O as Iterator>::Item; 3])>
where
    I: Iterator,
    O: Iterator,
{
    CubicIter { inputs, outputs }
}

#[derive(Debug, Clone)]
pub struct CubicIter<I, O> {
    inputs: I,
    outputs: O,
}

impl<I, O> Iterator for CubicIter<I, O>
where
    I: Iterator,
    O: Iterator,
{
    type Item = (<I as Iterator>::Item, [<O as Iterator>::Item; 3]);
    fn next(&mut self) -> Option<Self::Item> {
        Some((
            self.inputs.next()?,
            [
                self.outputs.next()?,
                self.outputs.next()?,
                self.outputs.next()?,
            ],
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn select_example1() {
        let iter = [0, 1, 2, 3, 4, 5, 6, 7, 8].into_iter();
        let result: Vec<_> = select_iterator(iter, 1, 3).collect();
        assert!(result.clone().into_iter().eq([1, 4, 7]), "{:?}", result);
    }
    #[test]
    fn select_example2() {
        let iter = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10].into_iter();
        let result: Vec<_> = select_iterator(iter, 3, 5).collect();
        assert!(result.clone().into_iter().eq([3, 8]), "{:?}", result);
    }
    #[test]
    fn cubic1() {
        let input = [0, 1, 2, 3].into_iter();
        let output = [-1, 0, 1, 2, 3, 4, 5, 6].into_iter();
        let result: Vec<_> = iter_cubic_data(input, output).collect();
        assert!(
            result
                .clone()
                .into_iter()
                .eq([(0, [-1, 0, 1]), (1, [2, 3, 4])]),
            "{:?}",
            result
        );
    }
}
