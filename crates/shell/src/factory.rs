//! The COM class factory that Explorer asks for a [`ContextMenu`] instance.

use crate::context_menu::ContextMenu;
use crate::{guard, LIVE_OBJECTS};
use std::ffi::c_void;
use std::sync::atomic::Ordering;
use windows::core::{implement, Interface, IUnknown, Result, GUID};
use windows::Win32::Foundation::*;
use windows::Win32::System::Com::*;

#[implement(IClassFactory)]
pub(crate) struct Factory;

impl Factory {
    pub(crate) fn new() -> Self {
        LIVE_OBJECTS.fetch_add(1, Ordering::SeqCst);
        Factory
    }
}

impl Drop for Factory {
    fn drop(&mut self) {
        LIVE_OBJECTS.fetch_sub(1, Ordering::SeqCst);
    }
}

impl IClassFactory_Impl for Factory_Impl {
    fn CreateInstance(&self, outer: Option<&IUnknown>, riid: *const GUID, ppv: *mut *mut c_void) -> Result<()> {
        guard(|| unsafe {
            if ppv.is_null() {
                return Err(E_POINTER.into());
            }
            *ppv = std::ptr::null_mut();
            if outer.is_some() {
                return Err(CLASS_E_NOAGGREGATION.into());
            }
            let unk: IUnknown = ContextMenu::new().into();
            unk.query(riid, ppv).ok()
        })
    }

    fn LockServer(&self, lock: BOOL) -> Result<()> {
        if lock.as_bool() {
            LIVE_OBJECTS.fetch_add(1, Ordering::SeqCst);
        } else {
            LIVE_OBJECTS.fetch_sub(1, Ordering::SeqCst);
        }
        Ok(())
    }
}
