use std::marker::{PhantomData, Unsize};
use std::{iter, ops, mem};
use std::boxed::into_raw;
use std::cell::Cell;

use raw::Raw;

struct Node<T: ?Sized, U:?Sized=T> {
    link: Raw<Node<U>>,
    data: T
}

impl<T: ?Sized> Node<T> {
    fn new<U: Unsize<T>>(val: U) -> Box<Node<T>> {
        let node : Box<Node<U, T>> = box Node {
            link: Raw::null(),
            data: val
        };

        return node;
    }
}

/**
 * An XOR list capable of holding dynamically-sized elements.
 *
 * An XOR list is doubly-linked list that reduces overhead by compressing the `previous` and `next`
 * pointers into a single field. As such, each node in an XOR list has only one pointer's worth of
 * overhead.
 *
 * This list is capable of holding dynamically-sized types. Each node is a seperate allocation
 * containing the data and a field. Due to XOR compression, each node has only a single pointer of
 * overhead, for a dynamically-sized type, this is two-words.
 */
pub struct XorList<T: ?Sized> {
    head: Raw<Node<T>>,
    tail: Raw<Node<T>>
}

impl<T: ?Sized> XorList<T> {
    /**
     * Constructs a new empty list
     */
    pub fn new() -> XorList<T> {
        XorList {
            head: Raw::null(),
            tail: Raw::null()
        }
    }

    /**
     * Pushes a new element to the end of the list. The element must coerce to the type of the
     * list. In general, this means that if `T` is a trait, `U` must implement that trait.
     */
    pub fn push_back<U: Unsize<T>>(&mut self, val: U) {
        let mut node = Node::new(val);

        if self.head.is_null() {
            let node_ptr = Raw::new(into_raw(node));
            self.head = node_ptr;
        } else if self.tail.is_null() {
            node.link = self.head;
            let node_ptr = Raw::new(into_raw(node));
            self.tail = node_ptr;
            let head = self.head.as_mut().expect("There should be a head!");
            head.link = self.tail;
        } else {
            node.link = self.tail;

            let node_ptr = Raw::new(into_raw(node));

            {
                let tail = self.tail.as_mut().expect("There should be a tail!");
                tail.link = tail.link.xor(&node_ptr);
            }
            self.tail = node_ptr;
        }
    }

    /**
     * Pushes a new element to the beginning of the list.
     */
    pub fn push_front<U: Unsize<T>>(&mut self, val: U) {
        let mut node = Node::new(val);
        if self.head.is_null() {
            let node_ptr = Raw::new(into_raw(node));
            self.head = node_ptr;
        } else if self.tail.is_null() {
            let mut old_head = self.head;
            self.tail = old_head;
            node.link = self.tail;
            let node_ptr = Raw::new(into_raw(node));
            self.head = node_ptr;
            let old_head = old_head.as_mut().unwrap();
            old_head.link = self.head;
        } else {
            node.link = self.head;
            let node_ptr = Raw::new(into_raw(node));

            {
                let head = self.head.as_mut().unwrap();
                head.link = head.link.xor(&node_ptr);
            }

            self.head = node_ptr;
        }
    }

    /**
     * Removes and returns the element at the end of the list.
     */
    pub fn pop_back(&mut self) -> Option<Elem<T>> {
        if self.head.is_null() {
            None
        } else if self.tail.is_null() {
            self.head.take().map(|n| Elem { __node: n })
        } else {
            let head_link = self.head.as_ref().unwrap().link;
            let tail_link = self.tail.as_ref().unwrap().link;

            if head_link == self.tail && tail_link == self.head {
                let node = self.tail.take();

                let head = self.head.as_mut().unwrap();
                head.link = Raw::null();

                node.map(|n| Elem { __node: n })
            } else {
                let mut node = self.tail;
                self.tail = node.as_ref().unwrap().link;

                let tail = self.tail.as_mut().unwrap();
                tail.link = tail.link.xor(&node);

                node.take().map(|n| Elem { __node: n })
            }
        }

    }

    /**
     * Removes and returns the element at the end of the list.
     */
    pub fn pop_front(&mut self) -> Option<Elem<T>> {
        if self.head.is_null() {
            None
        } else if self.tail.is_null() {
            self.head.take().map(|n| Elem { __node: n })
        } else {
            let head_link = self.head.as_ref().unwrap().link;
            let tail_link = self.tail.as_ref().unwrap().link;

            if head_link == self.tail && tail_link == self.head {
                let mut node = self.head;
                self.head = self.tail;
                self.tail = Raw::null();

                let head = self.head.as_mut().unwrap();
                head.link = Raw::null();

                node.take().map(|n| Elem { __node: n })
            } else {
                let mut node = self.head;
                self.head = node.as_ref().unwrap().link;

                let head = self.head.as_mut().unwrap();
                head.link = head.link.xor(&node);

                node.take().map(|n| Elem { __node: n })
            }
        }
    }

    pub fn iter<'a>(&'a self) -> Iter<'a, T> {
        Iter {
            prev: Raw::null(),
            curr: self.head,
            phantom: PhantomData
        }
    }

    pub fn iter_mut<'a>(&'a mut self) -> IterMut<'a, T> {
        IterMut {
            prev: Raw::null(),
            curr: self.head,
            phantom: PhantomData
        }
    }

    /**
     * Returns a cursor for this list that starts at the beginning of the list.
     *
     * See the documentation for `Cursor` for more details.
     */
    pub fn cursor<'a>(&'a mut self) -> Cursor<'a, T> {
        Cursor {
            prev: Cell::new(Raw::null()),
            curr: Cell::new(self.head),
            list: self,
            phantom: PhantomData
        }
    }

    pub fn is_empty(&self) -> bool {
        self.head.is_null()
    }

    /**
     * Removes all the elements from the list.
     */
    pub fn clear(&mut self) {
        while let Some(_) = self.pop_back() { }
    }
}

impl<T: ?Sized> Drop for XorList<T> {
    fn drop(&mut self) {
        self.clear();
    }
}

pub struct Iter<'a, T: ?Sized + 'a> {
    prev: Raw<Node<T>>,
    curr: Raw<Node<T>>,
    phantom: PhantomData<&'a XorList<T>>
}

impl<'a, T:?Sized> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        let prev = self.prev;
        let curr = self.curr;
        self.prev = curr;

        if let Some(node) = curr.as_ref() {
            let next = prev.xor(&node.link);
            self.curr = next;
            unsafe {
                Some(mem::transmute(&node.data))
            }
        } else {
            None
        }
    }
}

pub struct IterMut<'a, T: ?Sized + 'a> {
    prev: Raw<Node<T>>,
    curr: Raw<Node<T>>,
    phantom: PhantomData<&'a mut XorList<T>>
}

impl<'a, T:?Sized> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<&'a mut T> {
        let prev = self.prev;
        let mut curr = self.curr;
        self.prev = curr;

        if let Some(node) = curr.as_mut() {
            let next = prev.xor(&node.link);
            self.curr = next;
            unsafe {
                Some(mem::transmute(&mut node.data))
            }
        } else {
            None
        }
    }
}

pub struct IntoIter<T: ?Sized> {
    list: XorList<T>
}

impl<T: ?Sized> Iterator for IntoIter<T> {
    type Item = Elem<T>;

    fn next(&mut self) -> Option<Elem<T>> {
        self.list.pop_front()
    }
}

impl<T: ?Sized> DoubleEndedIterator for IntoIter<T> {
    fn next_back(&mut self) -> Option<Elem<T>> {
        self.list.pop_back()
    }
}

/**
 * A "Cursor" into a list.
 *
 * A `Cursor` is a structure representing a position between two elements in the list. It acts as
 * if there are special sentinel values at either end of the list so it can be placed after the
 * tail of the list or before the head of the list.
 *
 * `Cursor` allows you to traverse the list, insert and remove elements at arbitrary positions in
 * the list, insert other XorLists and split the list at the cursor position.
 */
pub struct Cursor<'a, T: ?Sized + 'a> {
    prev: Cell<Raw<Node<T>>>,
    curr: Cell<Raw<Node<T>>>,
    list: *mut XorList<T>,
    phantom: PhantomData<&'a mut XorList<T>>
}

impl<'a, T: ?Sized> Cursor<'a, T> {

    #[inline]
    pub fn at_start(&self) -> bool {
        self.prev.get().is_null()
    }

    #[inline]
    pub fn at_end(&self) -> bool {
        self.curr.get().is_null()
    }

    /**
     * Move to the cursor forwards one position and return a reference to the element that was
     * skipped over.
     */
    pub fn next<'b>(&'b self) -> Option<&'b T> {
        let prev = self.prev.get();
        let curr = self.curr.get();
        self.prev.set(curr);

        if let Some(node) = curr.as_ref() {
            let next = prev.xor(&node.link);
            self.curr.set(next);
            unsafe {
                Some(mem::transmute(&node.data))
            }
        } else {
            None
        }
    }

    /**
     * Move to the cursor backwards one position and return a reference to the element that was
     * skipped over.
     */
    pub fn prev<'b>(&'b self) -> Option<&'b T> {
        let prev = self.prev.get();
        let curr = self.curr.get();
        self.curr.set(prev);

        if let Some(node) = prev.as_ref() {
            let prev = curr.xor(&node.link);
            self.prev.set(prev);
            unsafe {
                Some(mem::transmute(&node.data))
            }
        } else {
            None
        }
    }

    /**
     * Skip forward `n` positions, or until the end of the list, whichever
     * is sooner.
     */
    pub fn skip_forwards(&self, n: usize) {
        if n == 0 { return; }
        let mut i = 0;
        while let Some(_) = self.next() {
            i += 1;
            if i >= n { break; }
        }
    }


    /**
     * Skip backward `n` positions, or until the start of the list, whichever
     * is sooner.
     */
    pub fn skip_backwards(&self, n: usize) {
        if n == 0 { return; }
        let mut i = 0;
        while let Some(_) = self.next() {
            i += 1;
            if i >= n { break; }
        }
    }

    /**
     * Move the cursor to the beginning of the list.
     */
    pub fn seek_to_start(&self) {
        unsafe {
            self.prev.set(Raw::null());
            self.curr.set((*self.list).head);
        }
    }

    /**
     * Move the cursor to the end of the list.
     */
    pub fn seek_to_end(&self) {
        unsafe {
            self.prev.set((*self.list).tail);
            self.curr.set(Raw::null());
        }
    }

    /**
     * Returns an immutable reference to element after the cursor.
     */
    pub fn peek<'b>(&'b self) -> Option<&'b T> {
        self.curr.get().as_ref().map(|node| {
            unsafe {
                mem::transmute(&node.data)
            }
        })
    }

    /**
     * Returns a mutable reference to the element after the cursor.
     */
    pub fn peek_mut<'b>(&'b mut self) -> Option<&'b mut T> {
        self.curr.get().as_mut().map(|node| {
            unsafe {
                mem::transmute(&mut node.data)
            }
        })
    }

    /**
     * Removes the element after the cursor and returns it.
     */
    pub fn remove(&mut self) -> Option<Elem<T>> {
        unsafe {
            if (*self.list).head == self.curr.get() {
                let elem = (*self.list).pop_front();
                self.curr.set((*self.list).head);
                return elem;
            } else if (*self.list).tail == self.curr.get() {
                self.curr.set(Raw::null());
                return (*self.list).pop_back();
            }
        }

        let mut prev = self.prev.get();
        let curr_ptr = self.curr.get();
        let curr = self.curr.get().take();
        self.curr.set(Raw::null());


        curr.map(|node| {
            let mut next = prev.xor(&node.link);

            // Calculate the new link values, based on this:
            //
            //         |
            //         v
            // A   B   C   D   E
            //
            // Where we're removing C

            if let Some(prev_node) = prev.as_mut() {
                // Link for B need to be A ^ D
                // A ^ D = ((A ^ C) ^ C) ^ D
                let new_link = prev_node.link.xor(&curr_ptr).xor(&next);
                prev_node.link = new_link;
            }
            if let Some(next_node) = next.as_mut() {
                // Link for D need to be B ^ E
                // B ^ E = ((C ^ E) ^ C) ^ B
                let new_link = next_node.link.xor(&curr_ptr).xor(&prev);
                next_node.link = new_link;
            }

            self.curr.set(next);

            Elem { __node: node }
        })
    }

    /**
     * Inserts the given value at the cursor position, leaving the cursor after the inserted value.
     */
    pub fn insert_before<U: Unsize<T>>(&self, val: U) {
        unsafe {
            if (*self.list).head == self.curr.get() {
                // We're at the head of the list, push to the front
                (*self.list).push_front(val);
                self.prev.set((*self.list).head);
            } else if self.curr.get().is_null() {
                // We're at the tail of the list, push to the back
                (*self.list).push_back(val);
                self.prev.set((*self.list).tail);
            } else {
                // We're somewhere in the middle

                debug_assert!(!self.curr.get().is_null());
                debug_assert!(!self.prev.get().is_null());

                let node = Node::new(val);

                let prev = self.prev.get();
                let curr = self.curr.get();

                self.prev.set(self.insert_between(prev, curr, node));
            }
        }
    }


    /**
     * Inserts the given value at the cursor position, leaving the cursor before the inserted value.
     */
    pub fn insert_after<U: Unsize<T>>(&self, val: U) {
        unsafe {
            if (*self.list).head == self.curr.get() {
                // We're at the head of the list, push to the front
                (*self.list).push_front(val);
                self.curr.set((*self.list).head);
            } else if self.curr.get().is_null() {
                // We're at the tail of the list, push to the back
                (*self.list).push_back(val);
                self.curr.set((*self.list).tail);
            } else {
                // We're somewhere in the middle

                debug_assert!(!self.curr.get().is_null());
                debug_assert!(!self.prev.get().is_null());

                let node = Node::new(val);

                let prev = self.prev.get();
                let curr = self.curr.get();

                self.curr.set(self.insert_between(prev, curr, node));
            }
        }
    }

    fn insert_between(&self, mut prev: Raw<Node<T>>, mut next: Raw<Node<T>>,
                      mut node: Box<Node<T>>) -> Raw<Node<T>> {
        node.link = prev.xor(&next);
        let node = Raw::new(into_raw(node));

        if let Some(prev_node) = prev.as_mut() {
            let new_link = prev_node.link.xor(&next).xor(&node);
            prev_node.link = new_link;
        }

        if let Some(next_node) = next.as_mut() {
            let new_link = next_node.link.xor(&prev).xor(&node);
            next_node.link = new_link;
        }

        return node;
    }

    /**
     * Inserts the given list at the cursor location. The cursor will be placed before the first
     * inserted element
     */
    pub fn splice(&mut self, mut list: XorList<T>) {
        unsafe {
            // Given list is empty
            if list.head.is_null() { return; }

            // Only a single node in the given list
            if list.tail.is_null() {
                let node = list.head.take().unwrap();

                let prev = self.prev.get();
                let curr = self.curr.get();

                let node = self.insert_between(prev, curr, node);
                self.curr.set(node);

                // Fix-up the head/tail references in the list
                if prev.is_null() {
                    (*self.list).head = node;
                } else if curr.is_null() {
                    (*self.list).tail = node;
                }

                return;
            }

            // This list we have is actually empty, just move the
            // head/tail pointers over
            if (*self.list).is_empty() {
                (*self.list).head = list.head;
                (*self.list).tail = list.tail;
                list.head = Raw::null();
                list.tail = Raw::null();

                self.prev.set(Raw::null());
                self.curr.set((*self.list).head);
            }

            let mut list_head = list.head.take().unwrap();
            let mut list_tail = list.tail.take().unwrap();

            let mut prev = self.prev.get();
            let mut curr = self.curr.get();

            list_head.link = list_head.link.xor(&prev);
            list_tail.link = list_tail.link.xor(&curr);

            let head = Raw::new(into_raw(list_head));
            let tail = Raw::new(into_raw(list_tail));

            if let Some(prev_node) = prev.as_mut() {
                prev_node.link = prev_node.link.xor(&curr).xor(&head);
            } else {
                (*self.list).head = head;
            }

            if let Some(curr_node) = curr.as_mut() {
                curr_node.link = curr_node.link.xor(&prev).xor(&tail);
            } else {
                (*self.list).tail = tail;
            }

            self.curr.set(head);
        }
    }

    /**
     * Splits the list at the cursor returning the remaining elements in a new list
     */
    pub fn split(&mut self) -> XorList<T> {
        unsafe {
            let mut new_list = XorList::new();

            // We're at the end of the list, so return the empty list
            if self.curr.get().is_null() {
                return new_list;
            }

            // We're at start end of the list, so move the current list
            // over to the new one
            if self.prev.get().is_null() {
                new_list.head = (*self.list).head;
                new_list.tail = (*self.list).tail;

                (*self.list).head = Raw::null();
                (*self.list).tail = Raw::null();

                self.curr.set(Raw::null());

                return new_list;
            }

            // We're somewhere in the middle
            let curr = self.curr.get();
            self.curr.set(Raw::null());

            new_list.head = curr;
            new_list.tail = (*self.list).tail;

            (*self.list).tail = self.prev.get();

            return new_list;
        }
    }
}

impl<U: ?Sized, T: Unsize<U>> iter::FromIterator<T> for XorList<U> {
    fn from_iter<I>(iter: I) -> XorList<U> where I: IntoIterator<Item=T> {
        let mut list = XorList::new();
        list.extend(iter);
        return list;
    }
}

impl<U: ?Sized, T: Unsize<U>> Extend<T> for XorList<U> {
    fn extend<I>(&mut self, iter: I) where I: IntoIterator<Item=T> {
        for el in iter {
            self.push_back(el);
        }
    }
}

/**
 * A simple wrapper type for removing elements by value.
 */
pub struct Elem<T: ?Sized> {
    __node: Box<Node<T>>
}

impl<T: ?Sized> ops::Deref for Elem<T> {
    type Target = T;

    fn deref<'a>(&'a self) -> &'a T {
        &self.__node.data
    }
}

impl<T: ?Sized> ops::DerefMut for Elem<T> {
    fn deref_mut<'a>(&'a mut self) -> &'a mut T {
        &mut self.__node.data
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::fmt::{Display, Debug};

    #[test]
    fn smoketest() {
        let mut list : XorList<Display> = XorList::new();

        list.push_back(1);
        list.push_back(2);
        list.push_back(3);
        list.push_back("None");
        list.push_back("Some(4)");

        let el = list.pop_front().unwrap();
        assert_eq!(&el.to_string()[..], "1");

        let el = list.pop_front().unwrap();
        assert_eq!(&el.to_string()[..], "2");

        let el = list.pop_front().unwrap();
        assert_eq!(&el.to_string()[..], "3");

        let el = list.pop_back().unwrap();
        assert_eq!(&el.to_string()[..], "Some(4)");

        list.push_back("Some(5)");

        let el = list.pop_front().unwrap();
        assert_eq!(&el.to_string()[..], "None");

        let el = list.pop_front().unwrap();
        assert_eq!(&el.to_string()[..], "Some(5)");

        assert!(list.pop_front().is_none());

        assert!(list.is_empty());
    }

    #[test]
    fn droptest() {

        #[derive(Debug)]
        struct DropTest;
        static mut DROP_TEST_COUNT : usize = 0;
        impl DropTest {
            fn new() -> DropTest {
                unsafe {
                    DROP_TEST_COUNT += 1;
                }
                DropTest
            }
        }
        impl Drop for DropTest {
            fn drop(&mut self) {
                unsafe {
                    DROP_TEST_COUNT -= 1;
                }
            }
        }

        {

            let mut list : XorList<Debug> = XorList::new();

            list.push_back(DropTest::new());
            list.push_back(DropTest::new());
            list.push_back(DropTest::new());

            unsafe {
                assert_eq!(DROP_TEST_COUNT, 3);
            }

        }

        unsafe {
            assert_eq!(DROP_TEST_COUNT, 0);
        }
    }

    #[test]
    fn iter() {
        let mut list : XorList<Display> = XorList::new();

        list.push_back(0);
        list.push_back(1);
        list.push_back(2);
        list.push_back(3);
        list.push_back(4);
        list.push_back(5);

        for (i, el) in list.iter().enumerate() {
            assert_eq!(el.to_string(), i.to_string());
        }
    }

    #[test]
    fn cursor_basic() {
        let mut list : XorList<Display> = XorList::new();

        list.push_back(0);
        list.push_back(1);
        list.push_back(2);
        list.push_back(3);
        list.push_back(4);
        list.push_back(5);

        {
            let mut cursor = list.cursor();
            cursor.remove();

            cursor.next();
            cursor.next();

            cursor.insert_before(6);

            cursor.remove();

            cursor.insert_after(7);
        }

        let el = list.pop_front().unwrap();
        assert_eq!(&el.to_string()[..], "1");

        let el = list.pop_front().unwrap();
        assert_eq!(&el.to_string()[..], "2");

        let el = list.pop_front().unwrap();
        assert_eq!(&el.to_string()[..], "6");

        let el = list.pop_front().unwrap();
        assert_eq!(&el.to_string()[..], "7");

        let el = list.pop_front().unwrap();
        assert_eq!(&el.to_string()[..], "4");

        let el = list.pop_front().unwrap();
        assert_eq!(&el.to_string()[..], "5");
    }

    #[test]
    fn cursor_splice() {
        let mut list : XorList<Display> = XorList::new();

        list.push_back(0);
        list.push_back(1);
        list.push_back(2);
        list.push_back(3);

        {
            let mut cursor = list.cursor();

            let mut list : XorList<Display> = XorList::new();
            list.push_back(4);
            list.push_back(5);
            list.push_back(6);
            list.push_back(7);


            cursor.next();
            cursor.next();

            cursor.splice(list);
        }

        let el = list.pop_front().unwrap();
        assert_eq!(&el.to_string()[..], "0");

        let el = list.pop_front().unwrap();
        assert_eq!(&el.to_string()[..], "1");

        let el = list.pop_front().unwrap();
        assert_eq!(&el.to_string()[..], "4");

        let el = list.pop_front().unwrap();
        assert_eq!(&el.to_string()[..], "5");

        let el = list.pop_front().unwrap();
        assert_eq!(&el.to_string()[..], "6");

        let el = list.pop_front().unwrap();
        assert_eq!(&el.to_string()[..], "7");

        let el = list.pop_front().unwrap();
        assert_eq!(&el.to_string()[..], "2");

        let el = list.pop_front().unwrap();
        assert_eq!(&el.to_string()[..], "3");

    }


}
