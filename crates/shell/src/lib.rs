//! FileFlip Explorer extension.
//!
//! A classic `IShellExtInit` + `IContextMenu` handler registered for all files
//! (`HKCU\Software\Classes\*\shellex\ContextMenuHandlers\FileFlip`). It looks at
//! the selected files, builds the nested "Convert to" menu from
//! [`fileflip_core::menu`] and, when an item is clicked, starts `fileflip.exe`
//! (which lives next to this DLL) once for the whole selection.
//!
//! Everything that crosses the COM boundary is wrapped in `catch_unwind`, since a
//! panic escaping into Explorer would take the shell down.

#![allow(non_snake_case)]

mod context_menu;
mod factory;
mod launch;
mod popup;

pub use launch::launch;

use factory::Factory;
use std::ffi::c_void;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;
use std::sync::atomic::{AtomicIsize, AtomicUsize, Ordering};
use windows::core::{Interface, Result, GUID, HRESULT};
use windows::Win32::Foundation::*;
use windows::Win32::System::Com::IClassFactory;
use windows::Win32::System::LibraryLoader::{DisableThreadLibraryCalls, GetModuleFileNameW};
use windows::Win32::System::SystemServices::DLL_PROCESS_ATTACH;

/// {4A8660F0-8A4E-45E6-86B1-77D553F9148A}
pub const CLSID_FILEFLIP: GUID = GUID::from_u128(0x4a8660f0_8a4e_45e6_86b1_77d553f9148a);

static DLL_INSTANCE: AtomicIsize = AtomicIsize::new(0);
static LIVE_OBJECTS: AtomicUsize = AtomicUsize::new(0);

fn guard<T>(f: impl FnOnce() -> Result<T>) -> Result<T> {
    catch_unwind(AssertUnwindSafe(f)).unwrap_or_else(|_| Err(E_UNEXPECTED.into()))
}

#[no_mangle]
extern "system" fn DllMain(hinst: HINSTANCE, reason: u32, _reserved: *mut c_void) -> BOOL {
    if reason == DLL_PROCESS_ATTACH {
        DLL_INSTANCE.store(hinst.0 as isize, Ordering::SeqCst);
        unsafe {
            let _ = DisableThreadLibraryCalls(HMODULE(hinst.0));
        }
    }
    TRUE
}

#[no_mangle]
extern "system" fn DllCanUnloadNow() -> HRESULT {
    if LIVE_OBJECTS.load(Ordering::SeqCst) == 0 {
        S_OK
    } else {
        S_FALSE
    }
}

#[no_mangle]
unsafe extern "system" fn DllGetClassObject(rclsid: *const GUID, riid: *const GUID, ppv: *mut *mut c_void) -> HRESULT {
    if ppv.is_null() || rclsid.is_null() || riid.is_null() {
        return E_POINTER;
    }
    *ppv = std::ptr::null_mut();
    if *rclsid != CLSID_FILEFLIP {
        return CLASS_E_CLASSNOTAVAILABLE;
    }
    let r = catch_unwind(AssertUnwindSafe(|| {
        let factory: IClassFactory = Factory::new().into();
        factory.query(riid, ppv)
    }));
    r.unwrap_or(E_UNEXPECTED)
}

fn module_dir() -> PathBuf {
    let mut buf = [0u16; 1024];
    let n = unsafe { GetModuleFileNameW(HMODULE(DLL_INSTANCE.load(Ordering::SeqCst) as *mut c_void), &mut buf) } as usize;
    let p = PathBuf::from(String::from_utf16_lossy(&buf[..n]));
    p.parent().map(|d| d.to_path_buf()).unwrap_or_default()
}
