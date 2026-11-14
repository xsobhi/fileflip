//! The `IShellExtInit` + `IContextMenu` object that Explorer talks to.

use crate::launch::launch;
use crate::popup::{build_popup, insert_separator, menu_icon, wide};
use crate::{guard, LIVE_OBJECTS};
use fileflip_core::menu::{self, Action, Node, Selection};
use fileflip_core::settings::Settings;
use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use windows::core::{implement, Error, Result, HRESULT, PWSTR};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::Com::*;
use windows::Win32::System::Ole::{ReleaseStgMedium, CF_HDROP};
use windows::Win32::System::Registry::HKEY;
use windows::Win32::UI::Shell::Common::ITEMIDLIST;
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::WindowsAndMessaging::*;

#[implement(IShellExtInit, IContextMenu)]
pub(crate) struct ContextMenu {
    files: RefCell<Vec<PathBuf>>,
    /// Files that are actually convertible (filled in QueryContextMenu).
    selection: RefCell<Vec<PathBuf>>,
    actions: RefCell<Vec<Action>>,
    bitmaps: RefCell<Vec<HBITMAP>>,
}

impl ContextMenu {
    pub(crate) fn new() -> Self {
        LIVE_OBJECTS.fetch_add(1, Ordering::SeqCst);
        ContextMenu {
            files: RefCell::default(),
            selection: RefCell::default(),
            actions: RefCell::default(),
            bitmaps: RefCell::default(),
        }
    }
}

impl Drop for ContextMenu {
    fn drop(&mut self) {
        for b in self.bitmaps.borrow_mut().drain(..) {
            unsafe {
                let _ = DeleteObject(b);
            }
        }
        LIVE_OBJECTS.fetch_sub(1, Ordering::SeqCst);
    }
}

impl IShellExtInit_Impl for ContextMenu_Impl {
    fn Initialize(&self, _folder: *const ITEMIDLIST, data: Option<&IDataObject>, _progid: HKEY) -> Result<()> {
        guard(|| unsafe {
            let data = data.ok_or_else(|| Error::from(E_INVALIDARG))?;
            let fmt = FORMATETC {
                cfFormat: CF_HDROP.0,
                ptd: std::ptr::null_mut(),
                dwAspect: DVASPECT_CONTENT.0,
                lindex: -1,
                tymed: TYMED_HGLOBAL.0 as u32,
            };
            let mut medium = data.GetData(&fmt)?;
            let hdrop = HDROP(medium.u.hGlobal.0);
            let count = DragQueryFileW(hdrop, u32::MAX, None);
            let mut files = Vec::with_capacity(count as usize);
            for i in 0..count {
                let len = DragQueryFileW(hdrop, i, None) as usize;
                let mut buf = vec![0u16; len + 1];
                let got = DragQueryFileW(hdrop, i, Some(&mut buf)) as usize;
                files.push(PathBuf::from(String::from_utf16_lossy(&buf[..got])));
            }
            ReleaseStgMedium(&mut medium);
            *self.files.borrow_mut() = files;
            Ok(())
        })
    }
}

impl IContextMenu_Impl for ContextMenu_Impl {
    fn QueryContextMenu(&self, hmenu: HMENU, index: u32, id_first: u32, id_last: u32, flags: u32) -> Result<()> {
        guard(|| unsafe {
            if flags & CMF_DEFAULTONLY != 0 {
                return Ok(());
            }
            let settings = Settings::load();
            let sel = Selection::new(self.files.borrow().iter().cloned(), &settings);
            let Some(root) = menu::build(&sel) else { return Ok(()) };
            let mut actions = Vec::new();
            menu::leaves(&root, &mut actions);
            if id_first as usize + actions.len() > id_last as usize + 1 {
                return Ok(()); // Explorer did not give us enough command ids
            }

            let Node::Sub { label, children } = &root else { return Ok(()) };
            let mut next_id = id_first;
            let popup = build_popup(children, &mut next_id)?;
            let mut text = wide(label);
            let icon = menu_icon();
            if let Some(b) = icon {
                self.bitmaps.borrow_mut().push(b);
            }
            let mut mask = MIIM_STRING | MIIM_SUBMENU;
            if icon.is_some() {
                mask |= MIIM_BITMAP;
            }
            let info = MENUITEMINFOW {
                cbSize: std::mem::size_of::<MENUITEMINFOW>() as u32,
                fMask: mask,
                hSubMenu: popup,
                dwTypeData: PWSTR(text.as_mut_ptr()),
                hbmpItem: icon.unwrap_or_default(),
                ..Default::default()
            };
            insert_separator(hmenu, index)?;
            InsertMenuItemW(hmenu, index + 1, TRUE, &info)?;
            insert_separator(hmenu, index + 2)?;

            *self.selection.borrow_mut() = sel.files;
            *self.actions.borrow_mut() = actions;
            let used = (next_id - id_first) as i32;
            // Success HRESULT whose code is the number of command ids we used.
            Err(Error::from_hresult(HRESULT(used)))
        })
    }

    fn InvokeCommand(&self, pici: *const CMINVOKECOMMANDINFO) -> Result<()> {
        guard(|| unsafe {
            let info = pici.as_ref().ok_or_else(|| Error::from(E_INVALIDARG))?;
            let verb = info.lpVerb.0 as usize;
            if verb >> 16 != 0 {
                return Err(E_FAIL.into()); // string verbs are not supported
            }
            let action = self.actions.borrow().get(verb).cloned().ok_or_else(|| Error::from(E_INVALIDARG))?;
            launch(&action, &self.selection.borrow()).map_err(|_| Error::from(E_FAIL))
        })
    }

    fn GetCommandString(&self, _id: usize, _type: u32, _res: *const u32, _name: windows::core::PSTR, _cch: u32) -> Result<()> {
        Err(E_NOTIMPL.into())
    }
}
