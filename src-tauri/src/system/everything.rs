//! Search via voidtools Everything (must be running). Uses Everything's documented IPC
//! (WM_COPYDATA with EVERYTHING_IPC_QUERY2 to the "EVERYTHING_TASKBAR_NOTIFICATION" window; the
//! reply comes back as WM_COPYDATA to a message-only window of ours). No SDK DLL needed.

use serde::Serialize;
use std::sync::Mutex;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::DataExchange::COPYDATASTRUCT;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::*;

const QUERY2W: usize = 18; // EVERYTHING_IPC_COPYDATA_QUERY2W
const REPLY_ID: u32 = 0x5354_5241; // our own reply tag
const REQ_FULL_PATH: u32 = 0x4; // EVERYTHING_REQUEST_FULL_PATH_AND_FILE_NAME
const IS_FOLDER: u32 = 0x1;
// Sorts (Everything 1.4 IPC): 1 = name ascending, 14 = date modified descending, 20 = run count descending.

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Hit {
    pub path: String,
    pub name: String,
    pub folder: bool,
}

static REPLY: Mutex<Option<Vec<Hit>>> = Mutex::new(None);

unsafe extern "system" fn wndproc(h: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if msg == WM_COPYDATA {
        let cds = &*(lp.0 as *const COPYDATASTRUCT);
        if cds.dwData == REPLY_ID as usize && !cds.lpData.is_null() {
            *REPLY.lock().unwrap() = Some(parse(cds.lpData as *const u8, cds.cbData as usize));
            return LRESULT(1);
        }
    }
    DefWindowProcW(h, msg, wp, lp)
}

/// EVERYTHING_IPC_LIST2: totitems, numitems, offset, request_flags, sort_type, then numitems × {flags, data_offset};
/// with only FULL_PATH requested, each item's data is DWORD length + UTF-16 text (+ NUL).
unsafe fn parse(base: *const u8, len: usize) -> Vec<Hit> {
    let rd = |off: usize| -> Option<u32> { (off + 4 <= len).then(|| std::ptr::read_unaligned(base.add(off) as *const u32)) };
    let mut out = vec![];
    let Some(n) = rd(4) else { return out };
    for i in 0..n as usize {
        let item = 20 + i * 8;
        let (Some(flags), Some(off)) = (rd(item), rd(item + 4)) else { break };
        let off = off as usize;
        let Some(chars) = rd(off) else { break };
        let start = off + 4;
        if start + chars as usize * 2 > len { break }
        let s = std::slice::from_raw_parts(base.add(start) as *const u16, chars as usize);
        let path = String::from_utf16_lossy(s);
        let name = path.rsplit('\\').next().unwrap_or(&path).to_string();
        out.push(Hit { folder: flags & IS_FOLDER != 0, name, path });
    }
    out
}

pub fn running() -> bool {
    unsafe { FindWindowW(w!("EVERYTHING_TASKBAR_NOTIFICATION"), PCWSTR::null()).is_ok() }
}

/// Runs one query on a dedicated thread (message-only window + short message pump).
pub fn search(query: String, max: u32, sort: u32) -> Result<Vec<Hit>, String> {
    std::thread::spawn(move || unsafe {
        let target = FindWindowW(w!("EVERYTHING_TASKBAR_NOTIFICATION"), PCWSTR::null())
            .map_err(|_| "Everything isn't running".to_string())?;
        let hinst = GetModuleHandleW(None).map_err(|e| e.to_string())?;
        let class = w!("StrataEverythingReply");
        let wc = WNDCLASSW { lpfnWndProc: Some(wndproc), hInstance: hinst.into(), lpszClassName: class, ..Default::default() };
        RegisterClassW(&wc); // fails harmlessly when already registered
        let me = CreateWindowExW(WINDOW_EX_STYLE(0), class, w!(""), WINDOW_STYLE(0), 0, 0, 0, 0, Some(HWND_MESSAGE), None, Some(hinst.into()), None)
            .map_err(|e| e.to_string())?;
        *REPLY.lock().unwrap() = None;

        // EVERYTHING_IPC_QUERY2 header (7 DWORDs) + NUL-terminated UTF-16 search.
        let mut buf: Vec<u8> = vec![];
        for v in [me.0 as usize as u32, REPLY_ID, 0, 0, max, REQ_FULL_PATH, sort] { buf.extend_from_slice(&v.to_le_bytes()); }
        for c in query.encode_utf16().chain(std::iter::once(0)) { buf.extend_from_slice(&c.to_le_bytes()); }
        let cds = COPYDATASTRUCT { dwData: QUERY2W, cbData: buf.len() as u32, lpData: buf.as_mut_ptr() as _ };
        let ok = SendMessageTimeoutW(target, WM_COPYDATA, WPARAM(me.0 as usize), LPARAM(&cds as *const _ as isize), SMTO_ABORTIFHUNG, 1500, None);
        let mut res = Err("Everything didn't answer".to_string());
        if ok.0 != 0 {
            let until = std::time::Instant::now() + std::time::Duration::from_millis(2000);
            let mut msg = MSG::default();
            while std::time::Instant::now() < until {
                while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() { DispatchMessageW(&msg); }
                if let Some(v) = REPLY.lock().unwrap().take() { res = Ok(v); break }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        }
        let _ = DestroyWindow(me);
        res
    })
    .join()
    .map_err(|_| "search thread failed".to_string())?
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore] // needs Everything running
    fn live_query() {
        let r = super::search("strata.exe".into(), 5, 20).unwrap();
        println!("{r:#?}");
        assert!(!r.is_empty());
    }
}
