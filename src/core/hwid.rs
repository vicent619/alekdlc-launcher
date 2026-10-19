use sha2::{Digest, Sha256};
use windows_sys::Win32::Storage::FileSystem::GetVolumeInformationW;
use windows_sys::Win32::System::Registry::{RegGetValueW, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ};
use windows_sys::Win32::System::SystemInformation::GetSystemFirmwareTable;

pub fn compute() -> String {
    let mut hasher = Sha256::new();
    hasher.update(crate::secret!("alekdlc-hwid-v1").as_bytes());
    for part in [firmware_uuid(), machine_guid(), system_volume_serial(), processor()] {
        if let Some(value) = part {
            hasher.update([0]);
            hasher.update(value.as_bytes());
        }
    }
    hasher.finalize().iter().map(|byte| format!("{byte:02x}")).collect()
}

fn machine_guid() -> Option<String> {
    registry_string(
        crate::secret!("SOFTWARE\\Microsoft\\Cryptography"),
        crate::secret!("MachineGuid"),
    )
}

fn registry_string(subkey: &str, value: &str) -> Option<String> {
    let subkey = wide(subkey);
    let value = wide(value);
    let mut size: u32 = 0;
    unsafe {
        if RegGetValueW(HKEY_LOCAL_MACHINE, subkey.as_ptr(), value.as_ptr(), RRF_RT_REG_SZ, std::ptr::null_mut(), std::ptr::null_mut(), &mut size) != 0 || size == 0 {
            return None;
        }
        let mut buffer = vec![0u16; size as usize / 2 + 1];
        if RegGetValueW(HKEY_LOCAL_MACHINE, subkey.as_ptr(), value.as_ptr(), RRF_RT_REG_SZ, std::ptr::null_mut(), buffer.as_mut_ptr().cast(), &mut size) != 0 {
            return None;
        }
        let used = (size as usize / 2).saturating_sub(1).min(buffer.len());
        let text = String::from_utf16_lossy(&buffer[..used]);
        let text = text.trim().to_owned();
        if text.is_empty() { None } else { Some(text) }
    }
}

fn system_volume_serial() -> Option<String> {
    let drive = std::env::var(crate::secret!("SystemDrive")).unwrap_or_else(|_| crate::secret!("C:").to_owned());
    let root = wide(&format!("{drive}\\"));
    let mut serial: u32 = 0;
    unsafe {
        if GetVolumeInformationW(root.as_ptr(), std::ptr::null_mut(), 0, &mut serial, std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut(), 0) == 0 || serial == 0 {
            return None;
        }
    }
    Some(format!("{serial:08x}"))
}

fn firmware_uuid() -> Option<String> {
    let provider = u32::from_be_bytes(*crate::secret_bytes!(b"RSMB"));
    unsafe {
        let size = GetSystemFirmwareTable(provider, 0, std::ptr::null_mut(), 0);
        if size == 0 {
            return None;
        }
        let mut buffer = vec![0u8; size as usize];
        let written = GetSystemFirmwareTable(provider, 0, buffer.as_mut_ptr().cast(), size);
        if written == 0 || written as usize > buffer.len() {
            return None;
        }
        buffer.truncate(written as usize);
        parse_firmware_uuid(&buffer)
    }
}

fn parse_firmware_uuid(data: &[u8]) -> Option<String> {
    if data.len() < 8 {
        return None;
    }
    let mut offset = 8;
    while offset + 4 <= data.len() {
        let kind = data[offset];
        let length = data[offset + 1] as usize;
        if length < 4 || offset + length > data.len() {
            break;
        }
        if kind == 1 && length >= 0x19 {
            let uuid = &data[offset + 8..offset + 24];
            if uuid.iter().all(|&byte| byte == 0) || uuid.iter().all(|&byte| byte == 0xff) {
                return None;
            }
            return Some(uuid.iter().map(|byte| format!("{byte:02x}")).collect());
        }
        let mut end = offset + length;
        while end + 1 < data.len() && !(data[end] == 0 && data[end + 1] == 0) {
            end += 1;
        }
        offset = end + 2;
        if kind == 127 {
            break;
        }
    }
    None
}

fn processor() -> Option<String> {
    let mut parts = Vec::new();
    for key in [
        crate::secret!("PROCESSOR_IDENTIFIER"),
        crate::secret!("PROCESSOR_ARCHITECTURE"),
        crate::secret!("NUMBER_OF_PROCESSORS"),
    ] {
        if let Some(value) = std::env::var_os(key) {
            parts.push(value.to_string_lossy().into_owned());
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(crate::secret!("|")))
    }
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}
