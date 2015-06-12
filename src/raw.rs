use std::{mem, cmp};

pub struct Raw<T: ?Sized> {
    pub ptr: *mut T
}
impl<T: ?Sized> Copy for Raw<T> { }
impl<T: ?Sized> Clone for Raw<T> { fn clone(&self) -> Raw<T> { *self } }

impl<T: ?Sized> Raw<T> {
    pub fn new(ptr: *mut T) -> Raw<T> {
        Raw {
            ptr: ptr
        }
    }

    pub fn null() -> Raw<T> {
        unsafe {
            mem::zeroed()
        }
    }

    pub fn as_ref<'a>(&'a self) -> Option<&'a T> {
        if self.is_null() {
            None
        } else {
            unsafe {
                Some(mem::transmute(self.ptr))
            }
        }
    }

    pub fn as_mut<'a>(&'a mut self) -> Option<&'a mut T> {
        if self.is_null() {
            None
        } else {
            unsafe {
                Some(mem::transmute(self.ptr))
            }
        }
    }

    pub fn take(&mut self) -> Option<Box<T>> {
        if self.is_null() {
            None
        } else {
            unsafe {
                let p = self.ptr;
                self.ptr = mem::zeroed();
                Some(Box::from_raw(p))
            }
        }
    }

    pub fn is_null(&self) -> bool {
        let p = self.ptr as *const ();
        p.is_null()
    }

    pub fn xor(&self, other: &Raw<T>) -> Raw<T> {
        unsafe {
            if is_sized::<T>() {
                let a = self.ptr as *const () as usize;
                let b = other.ptr as *const () as usize;

                let res = a ^ b;
                let res : *const *mut T = &res as *const _ as *const *mut T;

                Raw::new(*res)
            } else {
                let a : *const (usize, usize) = self as *const _ as *const (usize, usize);
                let b : *const (usize, usize) = other as *const _ as *const (usize, usize);

                let res = ((*a).0 ^ (*b).0, (*a).1 ^ (*b).1);
                let res : *const *mut T = &res as *const _ as *const *mut T;

                Raw::new(*res)
            }
        }
    }
}

pub fn is_sized<T: ?Sized>() -> bool {
    mem::size_of::<*const T>() == mem::size_of::<*const ()>()
}

impl<T:?Sized> cmp::PartialEq for Raw<T> {

    fn eq(&self, other: &Raw<T>) -> bool {
        let p1 = self.ptr as *const ();
        let p2 = other.ptr as *const ();

        p1 == p2
    }
}
