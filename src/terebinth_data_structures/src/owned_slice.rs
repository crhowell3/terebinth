use std::borrow::Borrow;
use std::ops::Deref;
use std::sync::Arc;

#[derive(Clone)]
pub struct OwnedSlice {
    bytes: *const [u8],
    #[expect(dead_code)]
    owner: Arc<dyn Send + Sync>,
}

pub fn slice_owned<O, F>(owner: O, slicer: F) -> OwnedSlice
where
    O: Send + Sync + 'static,
    F: FnOnce(&O) -> &[u8],
{
    try_slice_owned(owner, |x| Ok::<_, !>(slicer(x))).into_ok()
}

pub fn try_slice_owned<O, F, E>(owner: O, slicer: F) -> Result<OwnedSlice, E>
where
    O: Send + Sync + 'static,
    F: FnOnce(&O) -> Result<&[u8], E>,
{
    let owner = Arc::new(owner);
    let bytes = slicer(&*owner)?;

    Ok(OwnedSlice { bytes, owner })
}

impl OwnedSlice {
    pub fn slice(self, slicer: impl FnOnce(&[u8]) -> &[u8]) -> OwnedSlice {
        let bytes = slicer(&self);
        OwnedSlice { bytes, ..self }
    }
}

impl Deref for OwnedSlice {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &[u8] {
        unsafe { &*self.bytes }
    }
}

impl Borrow<[u8]> for OwnedSlice {
    #[inline]
    fn borrow(&self) -> &[u8] {
        self
    }
}

unsafe impl Send for OwnedSlice {}

unsafe impl Sync for OwnedSlice {}
