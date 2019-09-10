use std::ops::Range;
/// A stack that maintains a pointer to a focused element.
///
/// This primarily exists to keep track of the stack of windows in each
/// group and to remember which window within the stack is currently focused.
///
/// The order of the stack and the pointer to the focused element can be moved
/// independently:
///
/// - [`shuffle_next()`]/[`shuffle_previous()`] can be used to change
///   the order of the elements in the stack.
/// - [`focus_next()`]/[`focus_previous()`]
///   can be used to change the focused element, without affecting ordering.
///
/// [`shuffle_next()`]: #method.shuffle_next
/// [`shuffle_previous()`]: #method.shuffle_previous
/// [`focus_next()`]: #method.focus_next
/// [`focus_previous()`]: #method.focus_previous
#[derive(Clone, Debug, PartialEq)]
pub struct Stack<T> {
    windows: Vec<T>,
    focused: usize,
}

impl<T> Stack<T> {
    pub fn new() -> Stack<T> {
        Stack::default()
    }

    /// Returns the number of elements in the stack.
    pub fn len(&self) -> usize {
        self.windows.len()
    }

    /// Returns whether the stack is empty.
    pub fn is_empty(&self) -> bool {
        self.windows.is_empty()
    }

    /// Returns an iterator over the elements in order, ignoring focus.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.windows.iter()
    }

    /// Returns a reference to the focused element.
    pub fn focused(&self) -> Option<&T> {
        self.windows.get(self.focused)
    }

    pub fn focused_idx(&self) -> usize {
        self.focused
    }

    pub fn slice(&self, range: Range<usize>) -> &[T] {
        &self.windows[range]
    }

    pub fn from_parts(windows: Vec<T>, focused: usize) -> Stack<T> {
        Stack { windows, focused }
    }
}

impl<T> Default for Stack<T> {
    fn default() -> Self {
        Stack {
            windows: Vec::default(),
            focused: 0,
        }
    }
}

impl<T> From<Vec<T>> for Stack<T> {
    fn from(vec: Vec<T>) -> Self {
        Stack {
            windows: vec,
            focused: 0,
        }
    }
}

#[cfg(test)]
mod test {
    use super::Stack;
    use std::cmp::PartialEq;

    impl<T> PartialEq<Vec<T>> for Stack<T>
    where
        T: PartialEq + Clone,
    {
        fn eq(&self, other: &Vec<T>) -> bool {
            &self.windows == other
        }
    }

    fn stack_from_pieces<T>(before: Vec<T>, after: Vec<T>) -> Stack<T> {
        let focused = before.len();
        let mut windows = before;
        windows.extend(after);
        Stack { windows, focused }
    }

    #[test]
    fn test_from() {
        let vec = vec![1, 2, 3];
        let stack = Stack::from(vec.clone());
        assert_eq!(stack, vec);
        assert_eq!(stack.focused(), Some(&vec[0]));
        assert_eq!(stack.focused_idx(), 0)
    }

    #[test]
    fn test_len() {
        let stack = stack_from_pieces(vec![1, 2], vec![2, 3]);
        assert_eq!(stack.len(), 4);
    }

    #[test]
    fn test_is_empty() {
        let stack = Stack::<u8>::default();
        assert_eq!(stack.is_empty(), true);
        let stack = stack_from_pieces(vec![1, 2], vec![]);
        assert_eq!(stack.is_empty(), false);
        let stack = stack_from_pieces(vec![], vec![3, 4]);
        assert_eq!(stack.is_empty(), false);
    }

    #[test]
    fn test_focused() {
        let stack = stack_from_pieces(vec![], vec![2]);
        assert_eq!(stack.focused(), Some(&2));
        assert_eq!(stack.focused_idx(), 0);
        let stack: Stack<u8> = stack_from_pieces(vec![], vec![]);
        assert_eq!(stack.focused(), None);
    }

    #[test]
    fn test_iter() {
        let stack = Stack::<u8>::from(vec![2, 3, 4]);
        let mut iter = stack.iter();
        assert_eq!(iter.next(), Some(&2));
        assert_eq!(iter.next(), Some(&3));
        assert_eq!(iter.next(), Some(&4));
        assert_eq!(iter.next(), None);
    }

}
