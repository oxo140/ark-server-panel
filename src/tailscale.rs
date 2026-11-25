pub const DOWNLOAD_URL: &str = "https://tailscale.com/download";
pub fn download_qr() -> &'static str {
    static SVG: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    SVG.get_or_init(||{
        let code=qrcodegen::QrCode::encode_text(DOWNLOAD_URL,qrcodegen::QrCodeEcc::Medium).expect("Fixed short download URL fits in QR");
        let size=code.size()+8;
        let mut path=String::new();
        use std::fmt::Write;
        for y in 0..code.size(){for x in 0..code.size(){if code.get_module(x,y){let _=write!(path,"M{},{}h1v1h-1z",x+4,y+4);}}}
        format!(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {size} {size}" width="240" height="240" shape-rendering="crispEdges" role="img"><title>Installer Tailscale : https://tailscale.com/download</title><rect width="100%" height="100%" fill="white"/><path d="{path}" fill="black"/></svg>"#)
    })
}
