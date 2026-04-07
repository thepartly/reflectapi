pub trait Stream {
    type Item;
}

impl<S: ?Sized + Stream + Unpin> Stream for &mut S {
    type Item = S::Item;
}

impl<S: ?Sized + Stream + Unpin> Stream for Box<S> {
    type Item = S::Item;
}

impl<P> Stream for core::pin::Pin<P>
where
    P: core::ops::DerefMut + Unpin,
    P::Target: Stream,
{
    type Item = <P::Target as Stream>::Item;
}

pub mod stream {
    pub fn unfold<T, Item, F, Fut>(_init: T, _f: F) -> Unfold<Item>
    where
        F: FnMut(T) -> Fut,
    {
        unimplemented!()
    }

    pub struct Unfold<Item> {
        _marker: core::marker::PhantomData<Item>,
    }

    impl<Item> super::Stream for Unfold<Item> {
        type Item = Item;
    }

    impl<Item> Unpin for Unfold<Item> {}
}

pub struct Map<Item> {
    _marker: core::marker::PhantomData<Item>,
}

impl<Item> Stream for Map<Item> {
    type Item = Item;
}

impl<Item> Unpin for Map<Item> {}

pub struct Next<'a, St: ?Sized> {
    _marker: core::marker::PhantomData<&'a St>,
}

impl<St: Stream + Unpin + ?Sized> core::future::Future for Next<'_, St> {
    type Output = Option<St::Item>;
    fn poll(
        self: core::pin::Pin<&mut Self>,
        _cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        unimplemented!()
    }
}

pub trait StreamExt: Stream {
    fn next(&mut self) -> Next<'_, Self>
    where
        Self: Unpin,
    {
        unimplemented!()
    }

    fn map<T, F>(self, _f: F) -> Map<T>
    where
        Self: Sized,
        F: FnMut(Self::Item) -> T,
    {
        unimplemented!()
    }
}

impl<S: Stream + ?Sized> StreamExt for S {}

#[macro_export]
macro_rules! pin_mut {
    ($($x:ident),*) => {
        $(
            let mut $x = $x;
            #[allow(unused_mut)]
            let mut $x = unsafe { core::pin::Pin::new_unchecked(&mut $x) };
        )*
    }
}

