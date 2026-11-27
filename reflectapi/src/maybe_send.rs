/// `Send`, except on wasm32.
///
/// A wasm32 program runs on one thread, and the values it gets from its host,
/// such as the body of a `fetch` response, aren't `Send`. So on wasm32
/// reflectapi asks nothing of them: neither the client runtime of what a
/// transport gives it, nor the server of a route's handler and the stream it
/// returns. Everywhere else they need `Send`. A route on wasm32 can therefore
/// await a call through a generated client.
#[cfg(not(target_arch = "wasm32"))]
pub trait MaybeSend: Send {}
#[cfg(not(target_arch = "wasm32"))]
impl<T: Send + ?Sized> MaybeSend for T {}

/// `Send`, except on wasm32.
///
/// A wasm32 program runs on one thread, and the values it gets from its host,
/// such as the body of a `fetch` response, aren't `Send`. So on wasm32
/// reflectapi asks nothing of them: neither the client runtime of what a
/// transport gives it, nor the server of a route's handler and the stream it
/// returns. Everywhere else they need `Send`. A route on wasm32 can therefore
/// await a call through a generated client.
#[cfg(target_arch = "wasm32")]
pub trait MaybeSend {}
#[cfg(target_arch = "wasm32")]
impl<T: ?Sized> MaybeSend for T {}
