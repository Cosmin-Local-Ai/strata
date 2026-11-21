//! Native Open / Save file dialogs (IFileOpenDialog / IFileSaveDialog), shown on their own STA
//! thread and owned by the card that asked, so they stay in front of it.

use windows::core::{HSTRING, PCWSTR};
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED};
use windows::Win32::UI::Shell::Common::COMDLG_FILTERSPEC;
use windows::Win32::UI::Shell::*;

/// `filters`: (label, "*.csv;*.txt") pairs. Returns the chosen path, or None when cancelled.
pub fn pick(owner: isize, save: bool, title: &str, filters: Vec<(String, String)>, default_name: Option<String>) -> Option<String> {
    pick_inner(owner, save, false, title, filters, default_name)
}

/// Choose a folder.
pub fn pick_folder(owner: isize, title: &str) -> Option<String> {
    pick_inner(owner, false, true, title, vec![], None)
}

fn pick_inner(owner: isize, save: bool, folder: bool, title: &str, filters: Vec<(String, String)>, default_name: Option<String>) -> Option<String> {
    let title = title.to_string();
    std::thread::spawn(move || unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let labels: Vec<HSTRING> = filters.iter().map(|f| HSTRING::from(f.0.as_str())).collect();
        let specs: Vec<HSTRING> = filters.iter().map(|f| HSTRING::from(f.1.as_str())).collect();
        let spec: Vec<COMDLG_FILTERSPEC> = labels.iter().zip(&specs).map(|(l, s)| COMDLG_FILTERSPEC { pszName: PCWSTR(l.as_ptr()), pszSpec: PCWSTR(s.as_ptr()) }).collect();
        let dlg: IFileDialog = if save {
            CoCreateInstance::<_, IFileSaveDialog>(&FileSaveDialog, None, CLSCTX_INPROC_SERVER).ok()?.into()
        } else {
            CoCreateInstance::<_, IFileOpenDialog>(&FileOpenDialog, None, CLSCTX_INPROC_SERVER).ok()?.into()
        };
        let _ = dlg.SetTitle(&HSTRING::from(title.as_str()));
        if !spec.is_empty() {
            let _ = dlg.SetFileTypes(&spec);
        }
        if let Some(n) = &default_name {
            let _ = dlg.SetFileName(&HSTRING::from(n.as_str()));
            if let Some(ext) = n.rsplit_once('.').map(|x| x.1) {
                let _ = dlg.SetDefaultExtension(&HSTRING::from(ext));
            }
        }
        let opts = dlg.GetOptions().unwrap_or_default();
        let _ = dlg.SetOptions(opts | FOS_FORCEFILESYSTEM | if folder { FOS_PICKFOLDERS } else if save { FOS_OVERWRITEPROMPT } else { FOS_FILEMUSTEXIST });
        let owner = if owner != 0 { Some(HWND(owner as _)) } else { None };
        dlg.Show(owner).ok()?;
        let item = dlg.GetResult().ok()?;
        let p = item.GetDisplayName(SIGDN_FILESYSPATH).ok()?;
        let s = p.to_string().ok();
        CoTaskMemFree(Some(p.0 as _));
        s
    })
    .join()
    .ok()
    .flatten()
}
