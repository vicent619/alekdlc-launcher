use crate::secret;

pub struct Config;

impl Config {
    pub const VERSION: &'static str = "1.0";
    pub const MEMORY_MIN: u32 = 2;
    pub const MEMORY_MAX: u32 = 64;

    pub fn system_memory_gb() -> u32 {
        use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
        unsafe {
            let mut status: MEMORYSTATUSEX = std::mem::zeroed();
            status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
            if GlobalMemoryStatusEx(&mut status) != 0 {
                let gb = (status.ullTotalPhys as f64 / 1_073_741_824.0).round() as u32;
                return gb.clamp(Self::MEMORY_MIN, Self::MEMORY_MAX);
            }
        }
        Self::MEMORY_MIN.max(8)
    }

    #[cfg(not(debug_assertions))]
    pub fn api_url() -> String {
        secret!("https://alekdlc.tech").to_string()
    }

    #[cfg(not(debug_assertions))]
    pub fn site_url() -> String {
        secret!("https://alekdlc.tech").to_string()
    }

    #[cfg(not(debug_assertions))]
    pub fn server_key() -> String {
        secret!("hMeq2VO9kjpOJWeiYFGyfbmVC1sQY4hRa4H80dX0YMI=").to_string()
    }

    #[cfg(debug_assertions)]
    pub fn api_url() -> String {
        secret!("http://localhost:8090").to_string()
    }

    #[cfg(debug_assertions)]
    pub fn site_url() -> String {
        secret!("http://localhost:8090").to_string()
    }

    #[cfg(debug_assertions)]
    pub fn server_key() -> String {
        secret!("EPlBaNA+kb3u52m617IgIvCn1A/Lb5MqtbWgI0r66Vs=").to_string()
    }
}
