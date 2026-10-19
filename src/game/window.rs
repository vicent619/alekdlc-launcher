use std::collections::HashSet;

use windows_sys::Win32::Foundation::{HWND, LPARAM};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindow, GetWindowThreadProcessId, IsWindowVisible, GW_OWNER,
};

struct Search {
    pids: HashSet<u32>,
    found: bool,
}

pub fn has_visible_window(root_pid: u32) -> bool {
    let pids = process_tree(root_pid);
    let mut search = Search { pids, found: false };
    unsafe {
        EnumWindows(Some(callback), &mut search as *mut Search as LPARAM);
    }
    search.found
}

unsafe extern "system" fn callback(hwnd: HWND, lparam: LPARAM) -> i32 {
    let search = unsafe { &mut *(lparam as *mut Search) };
    let mut pid: u32 = 0;
    unsafe { GetWindowThreadProcessId(hwnd, &mut pid) };
    if search.pids.contains(&pid)
        && unsafe { IsWindowVisible(hwnd) } != 0
        && unsafe { GetWindow(hwnd, GW_OWNER) }.is_null()
    {
        search.found = true;
        return 0;
    }
    1
}

fn process_tree(root: u32) -> HashSet<u32> {
    let mut tree = HashSet::new();
    tree.insert(root);
    let pairs = snapshot();
    loop {
        let mut added = false;
        for (pid, parent) in &pairs {
            if tree.contains(parent) && tree.insert(*pid) {
                added = true;
            }
        }
        if !added {
            break;
        }
    }
    tree
}

fn snapshot() -> Vec<(u32, u32)> {
    let mut pairs = Vec::new();
    unsafe {
        let handle = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if handle.is_null() {
            return pairs;
        }
        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        if Process32FirstW(handle, &mut entry) != 0 {
            loop {
                pairs.push((entry.th32ProcessID, entry.th32ParentProcessID));
                if Process32NextW(handle, &mut entry) == 0 {
                    break;
                }
            }
        }
        windows_sys::Win32::Foundation::CloseHandle(handle);
    }
    pairs
}
