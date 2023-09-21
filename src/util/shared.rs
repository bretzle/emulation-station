use std::mem::MaybeUninit;
use std::ops::{Deref, DerefMut};
use std::ptr;
use std::ptr::NonNull;

struct SharedBox<T> {
    inner: T,
    count: usize,
}

pub struct Shared<T> {
    ptr: NonNull<SharedBox<T>>,
}

impl<T> Shared<T> {
    pub fn new(val: T) -> Self {
        Self::new_cyclic(|_| val)
    }

    pub fn new_cyclic<F: FnOnce(&Self) -> T>(f: F) -> Self {
        let uninit_ptr: NonNull<_> = Box::leak(Box::new(SharedBox {
            inner: MaybeUninit::<T>::uninit(),
            count: 1,
        }))
        .into();

        let init_ptr: NonNull<_> = uninit_ptr.cast();
        let shared = Shared { ptr: init_ptr };

        let data = f(&shared);

        unsafe {
            let ptr = init_ptr.as_ptr();
            ptr::write(ptr::addr_of_mut!((*ptr).inner), data);
        }

        shared
    }

    fn inc_count(&self) {
        unsafe {
            (*self.ptr.as_ptr()).count += 1;
        }
    }

    fn dec_count(&self) -> usize {
        unsafe {
            let sb = &mut *self.ptr.as_ptr();
            sb.count -= 1;
            sb.count
        }
    }
}

impl<T> Clone for Shared<T> {
    fn clone(&self) -> Self {
        self.inc_count();
        Self { ptr: self.ptr }
    }
}

impl<T> Deref for Shared<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &(*self.ptr.as_ptr()).inner }
    }
}

impl<T> DerefMut for Shared<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut (*self.ptr.as_ptr()).inner }
    }
}

impl<T> Drop for Shared<T> {
    fn drop(&mut self) {
        if self.dec_count() == 0 {
            unsafe { ptr::drop_in_place(self.ptr.as_ptr()) }
        }
    }
}

impl<T: Default> Default for Shared<T> {
    fn default() -> Self {
        Shared::new(T::default())
    }
}
