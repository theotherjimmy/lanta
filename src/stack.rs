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

    /// Adds an element to the stack (at the end) and focuses it.
    pub fn push(&mut self, value: T) {
        self.windows.push(value);
        self.focused = self.windows.len() - 1;
    }

    /// Returns an iterator over the elements in order, ignoring focus.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.windows.iter()
    }

    /// Returns an iterator mutably over the element in order, ignoring focus.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.windows.iter_mut()
    }

    /// Returns a reference to the focused element.
    pub fn focused(&self) -> Option<&T> {
        self.windows.get(self.focused)
    }

    /// Returns a mutable reference to the focued element.
    pub fn focused_mut(&mut self) -> Option<&mut T> {
        self.windows.get_mut(self.focused)
    }

    // If there is no element focused, try to focus the last element.
    fn ensure_after_not_empty(&mut self) {
        if self.focused >= self.windows.len() {
            self.focused = self.windows.len() - 1;
        }
    }

    /// Removes and returns the first element matching the predicate.
    ///
    /// If the removed element is currently focused, focus shifts to the
    /// next element.
    ///
    /// # Panics
    ///
    /// Panics if no element matches the predicate.
    pub fn remove<P>(&mut self, mut p: P) -> T
    where
        P: FnMut(&T) -> bool,
    {
        if let Some(position) = self.windows.iter().position(&mut p) {
            if self.focused >= position && self.focused > 0 {
                self.focused -= 1;
            }
            self.windows.remove(position)
        } else {
            panic!("element not found")
        }
    }

    /// Removes and returns the focused element (if any), shifting focus to the
    /// next element.
    pub fn remove_focused(&mut self) -> Option<T> {
        if self.windows.is_empty() {
            None
        } else {
            let removed = Some(self.windows.remove(self.focused));
            self.ensure_after_not_empty();
            removed
        }
    }

    /// Focuses the first element in the stack that matches the predicate.
    ///
    /// # Panics
    ///
    /// Panics if no element matches the predicate.
    pub fn focus<P>(&mut self, mut p: P)
    where
        P: FnMut(&T) -> bool,
    {
        if let Some(position) = self.windows.iter().position(&mut p) {
            self.focused = position;
        }
    }

    /// Shifts focus to the next element.
    pub fn focus_next(&mut self) {
        if self.focused < self.len() - 1 {
            self.focused += 1;
        }
    }

    /// Shifts focus to the next element.
    pub fn focus_next_wrap(&mut self) {
        if self.focused < self.len() - 1 {
            self.focused += 1;
        } else {
            self.focused = 0;
        }
    }

    /// Shifts focus to the previous element.
    pub fn focus_previous(&mut self) {
        if self.focused > 0 {
            self.focused -= 1;
        }
    }

    /// Inserts the currently focused element after the next element.
    pub fn shuffle_next(&mut self) {
        if self.focused <= self.len() - 2 {
            self.windows.swap(self.focused, self.focused + 1);
            self.focused += 1;
        } 
    }

    /// Inserts the currently focused element before the previous element.
    pub fn shuffle_previous(&mut self) {
        if self.focused >= 1 {
            self.windows.swap(self.focused, self.focused - 1);
            self.focused -= 1;
        }
    }

    pub fn slice(&self, range: Range<usize>) -> &[T] {
        &self.windows[range] 
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
    use std::cmp::PartialEq;
    use super::Stack;

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
    fn test_push() {
        let mut stack = Stack::<u8>::new();
        stack.push(2);
        assert_eq!(stack, vec![2]);
        assert_eq!(stack.focused(), Some(&2));
        stack.push(3);
        assert_eq!(stack.focused(), Some(&3));
        // Resulting order is also important:
        assert_eq!(stack, vec![2, 3]);
    }

    #[test]
    fn test_focused() {
        let stack = stack_from_pieces(vec![], vec![2]);
        assert_eq!(stack.focused(), Some(&2));
        let stack: Stack<u8> = stack_from_pieces(vec![], vec![]);
        assert_eq!(stack.focused(), None);
    }

    #[test]
    fn test_remove() {
        let mut stack = Stack::<u8>::new();
        stack.push(2);
        stack.push(3);
        stack.push(4);
        stack.remove(|v| v == &3);
        assert_eq!(stack, vec![2, 4]);
    }

    #[test]
    fn test_remove_focused() {
        let mut stack = Stack::<u8>::new();
        stack.push(2);
        stack.push(3);
        assert_eq!(stack.focused(), Some(&3));

        let element = stack.remove_focused();
        assert_eq!(element, Some(3));
        assert_eq!(stack.focused(), Some(&2));
        assert_eq!(stack, vec![2]);
    }

    #[test]
    fn test_remove_focused_when_no_focus() {
        let mut stack = Stack::<u8>::new();
        assert_eq!(stack.focused(), None);
        let element = stack.remove_focused();
        assert_eq!(element, None);
        assert_eq!(stack.focused(), None);
    }

    #[test]
    fn test_iter() {
        let mut stack = Stack::<u8>::new();
        stack.push(2);
        stack.push(3);
        stack.push(4);
        let mut iter = stack.iter();
        assert_eq!(iter.next(), Some(&2));
        assert_eq!(iter.next(), Some(&3));
        assert_eq!(iter.next(), Some(&4));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_set_get_focus() {
        let mut stack = Stack::<u8>::new();
        assert_eq!(stack.focused(), None);
        stack.push(2);
        stack.push(3);
        assert_eq!(stack.focused(), Some(&3));
        assert_eq!(stack, vec![2, 3]);
        stack.focus(|v| v == &2);
        assert_eq!(stack.focused(), Some(&2));
        assert_eq!(stack, vec![2, 3]);
        stack.focus(|v| v == &3);
        assert_eq!(stack.focused(), Some(&3));
        assert_eq!(stack, vec![2, 3]);
    }

    #[test]
    fn test_focus_next() {
        let vec = vec![1, 2, 3];
        let mut stack = Stack::from(vec.clone());

        // We constantly assert the order of the stack, as it's very easy
        // to write a non-order-preserving implementation!

        assert_eq!(stack.focused(), Some(&1));
        assert_eq!(stack, vec);

        stack.focus_next();
        assert_eq!(stack.focused(), Some(&2));
        assert_eq!(stack, vec);

        stack.focus_next();
        assert_eq!(stack.focused(), Some(&3));
        assert_eq!(stack, vec);

        stack.focus_next();
        assert_eq!(stack.focused(), Some(&3));
        assert_eq!(stack, vec);
    }

    #[test]
    fn test_focus_previous() {
        let vec = vec![1, 2, 3];
        let mut stack = Stack::from(vec.clone());

        // We constantly assert the order of the stack, as it's very easy
        // to write a non-order-preserving implementation!

        assert_eq!(stack.focused(), Some(&1));
        assert_eq!(stack, vec);

        stack.focus_next();
        stack.focus_next();
        stack.focus_next();

        assert_eq!(stack.focused(), Some(&3));
        assert_eq!(stack, vec);

        stack.focus_previous();
        assert_eq!(stack.focused(), Some(&2));
        assert_eq!(stack, vec);

        stack.focus_previous();
        assert_eq!(stack.focused(), Some(&1));
        assert_eq!(stack, vec);

        stack.focus_previous();
        assert_eq!(stack.focused(), Some(&1));
        assert_eq!(stack, vec);
    }

    #[test]
    fn test_shuffle_next() {
        let mut stack = Stack::<u8>::new();
        stack.push(2);
        stack.push(3);
        stack.push(4);
        assert_eq!(stack.focused(), Some(&4));

        assert_eq!(stack, vec![2, 3, 4]);
        stack.shuffle_next();
        assert_eq!(stack, vec![2, 3, 4]);
        stack.focus_previous();
        stack.focus_previous();
        stack.shuffle_next();
        assert_eq!(stack, vec![3, 2, 4]);
        stack.shuffle_next();
        assert_eq!(stack, vec![3, 4, 2]);
    }

    #[test]
    fn test_shuffle_previous() {
        let mut stack = Stack::<u8>::new();
        stack.push(2);
        stack.push(3);
        stack.push(4);
        assert_eq!(stack.focused(), Some(&4));

        assert_eq!(stack, vec![2, 3, 4]);
        stack.shuffle_previous();
        assert_eq!(stack, vec![2, 4, 3]);
        stack.shuffle_previous();
        assert_eq!(stack, vec![4, 2, 3]);
        stack.shuffle_previous();
        assert_eq!(stack, vec![4, 2, 3]);
    }
}
