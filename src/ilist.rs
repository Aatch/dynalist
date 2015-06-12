use std::marker::{self, Unsize};
use std::boxed::into_raw;
use std::cell::Cell;
use std::mem;

use std::intrinsics::drop_in_place;
use std::rt::heap::{allocate, deallocate};

use core::nonzero::NonZero;

use raw::{self, Raw};

/**
 * A reference-counted node for use in an `IList`. An `INode` can only be in one IList at a time.
 */
#[unsafe_no_drop_flag]
pub struct INode<T: ?Sized> {
    __ptr: NonZero<*mut Node<T>>
}

impl<T: ?Sized> !marker::Send for INode<T> {}
impl<T: ?Sized> !marker::Sync for INode<T> {}

struct Node<T: ?Sized, U: ?Sized=T> {
    count: Cell<usize>,
    next: Cell<Raw<Node<U>>>,
    prev: Cell<Raw<Node<U>>>,
    data: T
}

impl<T: ?Sized> INode<T> {
    pub fn new<U: Unsize<T>>(value: U) -> INode<T> {
        unsafe {
            let node : Box<Node<U, T>> = box Node {
                count: Cell::new(1),
                next: Cell::new(Raw::null()),
                prev: Cell::new(Raw::null()),
                data: value
            };

            let node : Box<Node<T, T>> = node;
            let ptr = into_raw(node);

            INode {
                __ptr: NonZero::new(ptr)
            }
        }
    }

    pub fn as_ref<'a>(&'a self) -> &'a T {
        unsafe {
            let node = &**self.__ptr;
            return &node.data;
        }
    }

    /**
     * Removes this `INode` from the list it is in, if it is a list.
     */
    pub fn remove_from_list(&self) {
        self.node().remove_from_list();
    }

    /**
     * Inserts the given node after this one.
     *
     * Panics if this node isn't in a list.
     */
    pub fn insert_after(&self, val: INode<T>) {
        assert!(self.in_list());
        val.remove_from_list();
        let raw_self = Raw::new(*self.__ptr);

        let next = self.node().next.get();

        val.node().prev.set(raw_self);
        val.node().next.set(next);

        let raw_val = val.into_raw();
        self.node().next.set(raw_val);

        if let Some(next) = next.as_ref() {
            next.prev.set(raw_val);
        }
    }

    /**
     * Inserts the given node before this one.
     *
     * Panics if this node isn't in a list.
     */
    pub fn insert_before(&self, val: INode<T>) {
        assert!(self.in_list());
        val.remove_from_list();
        let raw_self = Raw::new(*self.__ptr);

        let prev = self.node().prev.get();

        val.node().next.set(raw_self);
        val.node().prev.set(prev);

        let raw_val = val.into_raw();
        self.node().prev.set(raw_val);

        if let Some(prev) = prev.as_ref() {
            prev.next.set(raw_val);
        }
    }

    /**
     * Returns the next node in the list, or None if there is no next node.
     */
    pub fn next(&self) -> Option<INode<T>> {
        let raw_next = self.node().next.get();

        if let Some(next) = raw_next.as_ref() {
            if !next.is_sentinel() {
                unsafe {
                    let next = INode { __ptr: NonZero::new(raw_next.ptr) };
                    next.inc_count();
                    return Some(next);
                }
            }
        }

        None
    }

    /**
     * Returns the previous node in the list, or None if there is no previous node.
     */
    pub fn prev(&self) -> Option<INode<T>> {
        let raw_prev = self.node().prev.get();

        if let Some(prev) = raw_prev.as_ref() {
            if !prev.is_sentinel() {
                unsafe {
                    let prev = INode { __ptr: NonZero::new(raw_prev.ptr) };
                    prev.inc_count();
                    return Some(prev);
                }
            }
        }

        None
    }

    /**
     * Returns whether or not this node is in a list.
     */
    pub fn in_list(&self) -> bool {
        !self.node().next().is_null()
    }

    fn count(&self) -> usize {
        self.node().count.get()
    }

    fn node(&self) -> &Node<T> {
        unsafe {
            &**self.__ptr
        }
    }

    fn inc_count(&self) {
        self.node().inc_count();
    }

    fn dec_count(&self) {
        self.node().dec_count();
    }

    fn into_raw(self) -> Raw<Node<T>> {
        let raw = Raw::new(*self.__ptr);
        mem::forget(self);
        raw
    }

    fn to_raw(&self) -> Raw<Node<T>> {
        Raw::new(*self.__ptr)
    }

    fn from_raw(raw: Raw<Node<T>>) -> INode<T> {
        unsafe {
            let node = INode { __ptr: NonZero::new(raw.ptr) };
            node.inc_count();
            node
        }
    }
}

impl<T: ?Sized> Drop for INode<T> {
    fn drop(&mut self) {
        unsafe {
            let ptr = *self.__ptr;

            let vp = ptr as *const ();

            if !vp.is_null() && vp as usize != mem::POST_DROP_USIZE {
                self.dec_count();
                if self.count() == 0 {
                    drop_in_place(&mut (*ptr).data);
                    deallocate(ptr as *mut u8,
                               mem::size_of_val(&*ptr),
                               mem::min_align_of_val(&*ptr));
                }
            }
        }
    }
}

impl<T: ?Sized> Clone for INode<T> {
    fn clone(&self) -> INode<T> {
        self.inc_count();
        INode { __ptr: self.__ptr }
    }
}

impl<T: ?Sized> Node<T> {
    fn is_sentinel(&self) -> bool {
        self.count.get() == !0
    }

    fn inc_count(&self) {
        let count = self.count.get();
        self.count.set(count + 1);
    }

    fn dec_count(&self) {
        let count = self.count.get();
        self.count.set(count - 1);
    }

    fn remove_from_list(&self) {
        let prev = self.prev.get();
        let next = self.next.get();

        self.prev.set(Raw::null());
        self.next.set(Raw::null());

        if let Some(prev) = prev.as_ref() {
            // The next pointers for each node are the ones that keep the refcount
            // up
            self.dec_count();
            prev.next.set(next);
        }

        if let Some(next) = next.as_ref() {
            next.prev.set(prev);
        }
    }

}

fn make_sentinel<T: ?Sized>() -> INode<T> {
    unsafe {
        let align = mem::min_align_of::<Node<(), T>>();
        let size  = mem::size_of::<Node<(), T>>();

        let mut ptr = allocate(size, align);

        let ptr = if raw::is_sized::<T>() {
            let mut ptr : (*mut _, usize) = (ptr, 0);

            let ptr : *mut *mut Node<T> = &mut ptr as *mut _ as *mut *mut Node<T>;

            *ptr
        } else {
            let ptr : *mut *mut Node<T> = &mut ptr as *mut _ as *mut *mut Node<T>;
            *ptr
        };

        (*ptr).next.set(Raw::null());
        (*ptr).prev.set(Raw::null());
        (*ptr).count.set(!0);

        INode { __ptr: NonZero::new(ptr) }
    }
}

pub struct IList<T: ?Sized> {
    sentinel: INode<T>
}

impl<T: ?Sized> IList<T> {
    pub fn new() -> IList<T> {
        let sentinel = make_sentinel::<T>();
        IList { sentinel: sentinel }
    }

    pub fn is_empty(&self) -> bool {
        self.sentinel.node().next.get().is_null()
    }

    /**
     * Pushes the given node to the front of the list.
     */
    pub fn push_front(&self, val: INode<T>) {
        if self.is_empty() {
            val.remove_from_list();
            let raw_s = self.sentinel.to_raw();
            val.node().next.set(raw_s);
            val.node().prev.set(raw_s);

            let raw_val = val.into_raw();

            self.sentinel.node().next.set(raw_val);
            self.sentinel.node().prev.set(raw_val);
        } else {
            self.sentinel.insert_after(val);
        }
    }

    /**
     * Pushes the given node to the back of the list.
     */
    pub fn push_back(&self, val: INode<T>) {
        if self.is_empty() {
            val.remove_from_list();
            let raw_s = self.sentinel.to_raw();
            val.node().next.set(raw_s);
            val.node().prev.set(raw_s);

            let raw_val = val.into_raw();

            self.sentinel.node().next.set(raw_val);
            self.sentinel.node().prev.set(raw_val);
        } else {
            self.sentinel.insert_before(val);
        }
    }

    /**
     * Returns the head of the list, if there is one
     */
    pub fn head(&self) -> Option<INode<T>> {
        if self.is_empty() {
            None
        } else {
            let head = self.sentinel.node().next.get();
            let head = INode::from_raw(head);
            Some(head)
        }
    }

    /**
     * Returns the tail of the list, if there is one
     */
    pub fn tail(&self) -> Option<INode<T>> {
        if self.is_empty() {
            None
        } else {
            let tail = self.sentinel.node().prev.get();
            let tail = INode::from_raw(tail);
            Some(tail)
        }
    }

    pub fn iter(&self) -> Iter<T> {
        Iter {
            current: self.head()
        }
    }
}

impl<T:?Sized> Drop for IList<T> {
    fn drop(&mut self) {
        unsafe {
            let mut node = self.sentinel.node().next.get();

            while !node.is_null() {

                let inode = INode::from_raw(node);
                let next = inode.node().next.get();

                inode.remove_from_list();

                if let Some(n) = next.as_ref() {
                    if n.is_sentinel() { break; }
                }

                node = next;
            }

            let sentinel = self.sentinel.__ptr;
            self.sentinel.__ptr = NonZero::new(Raw::null().ptr);

            let sentinel = *sentinel as *mut u8;

            let align = mem::min_align_of::<Node<(), T>>();
            let size  = mem::size_of::<Node<(), T>>();

            deallocate(sentinel, size, align);
        }
    }
}

pub struct Iter<T: ?Sized> {
    current: Option<INode<T>>
}

impl<T: ?Sized> Iterator for Iter<T> {
    type Item = INode<T>;

    fn next(&mut self) -> Option<INode<T>> {
        let node = self.current.take();

        if let Some(ref n) = node {
            self.current = n.next();
        }

        node
    }
}

#[cfg(test)]
mod test {
    use std::fmt::Display;
    use super::*;

    #[test]
    fn smoketest() {
        let list : IList<Display> = IList::new();

        let node1 = INode::new(1);
        list.push_back(node1);

        let node2 = INode::new(2);
        list.push_back(node2.clone());

        let node3 = INode::new(3);

        node2.insert_after(node3);

        let node4 = INode::new("I'm a string");
        node2.insert_before(node4);

        let mut node = list.head().unwrap();
        assert_eq!(node.as_ref().to_string(), "1");

        node = node.next().unwrap();
        assert_eq!(node.as_ref().to_string(), "I'm a string");

        node = node.next().unwrap();
        assert_eq!(node.as_ref().to_string(), "2");

        node = node.next().unwrap();
        assert_eq!(node.as_ref().to_string(), "3");

        assert!(node.next().is_none());
    }

    #[test]
    fn move_lists() {
        let list1 : IList<Display> = IList::new();

        let node1 = INode::new(1);
        list1.push_back(node1);

        let node2 = INode::new(2);
        list1.push_back(node2.clone());

        let list2 : IList<Display> = IList::new();

        let node3 = INode::new(3);
        list2.push_back(node3);
        list2.push_back(node2);

        let node = list1.head().unwrap();
        assert_eq!(node.as_ref().to_string(), "1");

        let mut node = list2.head().unwrap();
        assert_eq!(node.as_ref().to_string(), "3");

        node = node.next().unwrap();
        assert_eq!(node.as_ref().to_string(), "2");

    }
}
