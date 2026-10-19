fn main() {
    #[cfg(windows)]
    {
        let mut resource = winres::WindowsResource::new();
        resource.set_icon("assets/icon.ico");
        let _ = resource.compile();
    }
}
