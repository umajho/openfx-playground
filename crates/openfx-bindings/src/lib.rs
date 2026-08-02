pub mod bindings;
pub mod handle;

// SAFETY: The host promises not to mess with the raw string pointers in this struct
unsafe impl Send for bindings::OfxPlugin {}
unsafe impl Sync for bindings::OfxPlugin {}
