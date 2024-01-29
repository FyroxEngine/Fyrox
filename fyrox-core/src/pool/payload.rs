use std::cell::UnsafeCell;

pub trait PayloadContainer: Sized {
    type Element: Sized;

    fn new_empty() -> Self;

    fn new(element: Self::Element) -> Self;

    fn is_some(&self) -> bool;

    fn as_ref(&self) -> Option<&Self::Element>;

    fn as_mut(&mut self) -> Option<&mut Self::Element>;

    fn replace(&mut self, element: Self::Element) -> Option<Self::Element>;

    fn take(&mut self) -> Option<Self::Element>;
}

impl<T> PayloadContainer for Option<T> {
    type Element = T;

    #[inline]
    fn new_empty() -> Self {
        Self::None
    }

    #[inline]
    fn new(element: Self::Element) -> Self {
        Self::Some(element)
    }

    #[inline]
    fn is_some(&self) -> bool {
        Option::is_some(self)
    }

    #[inline]
    fn as_ref(&self) -> Option<&Self::Element> {
        Option::as_ref(self)
    }

    #[inline]
    fn as_mut(&mut self) -> Option<&mut Self::Element> {
        Option::as_mut(self)
    }

    #[inline]
    fn replace(&mut self, element: Self::Element) -> Option<Self::Element> {
        Option::replace(self, element)
    }

    #[inline]
    fn take(&mut self) -> Option<Self::Element> {
        Option::take(self)
    }
}

#[derive(Debug)]
pub struct Payload<P>(pub UnsafeCell<P>);

impl<T, P> Clone for Payload<P>
where
    T: Sized,
    P: PayloadContainer<Element = T> + Clone,
{
    fn clone(&self) -> Self {
        Self(UnsafeCell::new(self.get().clone()))
    }
}

impl<T, P> Payload<P>
where
    T: Sized,
    P: PayloadContainer<Element = T>,
{
    pub fn new(data: T) -> Self {
        Self(UnsafeCell::new(P::new(data)))
    }

    pub fn new_empty() -> Self {
        Self(UnsafeCell::new(P::new_empty()))
    }

    pub fn get(&self) -> &P {
        unsafe { &*self.0.get() }
    }

    pub fn get_mut(&mut self) -> &mut P {
        self.0.get_mut()
    }

    pub fn is_some(&self) -> bool {
        self.get().is_some()
    }

    pub fn as_ref(&self) -> Option<&T> {
        self.get().as_ref()
    }

    pub fn as_mut(&mut self) -> Option<&mut T> {
        self.get_mut().as_mut()
    }

    pub fn replace(&mut self, element: T) -> Option<T> {
        self.get_mut().replace(element)
    }

    pub fn take(&mut self) -> Option<T> {
        self.get_mut().take()
    }
}

// SAFETY: This is safe, because Payload is never directly exposed to the call site. It is always
// accessed using a sort of read-write lock that forces borrowing rules at runtime.
unsafe impl<T, P> Sync for Payload<P>
where
    T: Sized,
    P: PayloadContainer<Element = T>,
{
}

// SAFETY: This is safe, because Payload is never directly exposed to the call site. It is always
// accessed using a sort of read-write lock that forces borrowing rules at runtime.
unsafe impl<T, P> Send for Payload<P>
where
    T: Sized,
    P: PayloadContainer<Element = T>,
{
}
