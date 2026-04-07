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

pub fn map<St, F>(_stream: St, _f: F) -> impl Stream {
    Placeholder
}

pub mod stream {
    pub fn unfold<T, F, Fut, Item>(_init: T, _f: F) -> impl super::Stream
    where
        F: FnMut(T) -> Fut,
    {
        super::Placeholder
    }
}

struct Placeholder;
impl Stream for Placeholder {
    type Item = ();
}

pub struct StreamExt;

impl StreamExt {
    pub fn map<St, F>(_stream: St, _f: F) -> impl Stream {
        Placeholder
    }

    pub fn next<St>(_stream: &mut St) -> impl core::future::Future<Output = Option<()>> {
        async { None }
    }
}

pub struct TryStreamExt;

impl TryStreamExt {
    pub fn try_concat<St>(_stream: St) -> impl core::future::Future<Output = Result<bytes::Bytes, ()>> {
        async { Ok(bytes::Bytes::new()) }
    }
}
