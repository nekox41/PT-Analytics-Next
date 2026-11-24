// SPDX-License-Identifier: GPL-3.0-or-later
//! Native clipboard fallback, keeping PNG alpha and providing a Windows bitmap format.
#[tauri::command]
pub fn copy_report_text(text: String) -> Result<(), String> {
    if text.len()>1024*1024 { return Err("文本过长".into()); }
    #[cfg(windows)]
    { clipboard_win::set_clipboard(clipboard_win::formats::Unicode, text.as_str()).map_err(|e|e.to_string()) }
    #[cfg(not(windows))]
    { let _=text; Err("请使用系统 WebView 剪贴板".into()) }
}
#[cfg(windows)]
fn dibv5(img: &image::RgbaImage) -> Vec<u8> {
    let (w,h)=img.dimensions();
    let mut out=vec![0u8;124];
    fn put(b:&mut [u8],i:usize,v:u32){b[i..i+4].copy_from_slice(&v.to_le_bytes());}
    put(&mut out,0,124);put(&mut out,4,w);put(&mut out,8,h);
    out[12..14].copy_from_slice(&1u16.to_le_bytes());out[14..16].copy_from_slice(&32u16.to_le_bytes());
    put(&mut out,16,3);put(&mut out,20,w*h*4);
    put(&mut out,40,0x00ff0000);put(&mut out,44,0x0000ff00);put(&mut out,48,0x000000ff);put(&mut out,52,0xff000000);
    put(&mut out,56,0x73524742);put(&mut out,108,4);
    out.reserve((w*h*4) as usize);
    for row in img.as_raw().chunks_exact(w as usize*4).rev() {
        for p in row.chunks_exact(4){out.extend_from_slice(&[p[2],p[1],p[0],p[3]]);}
    }
    out
}
#[tauri::command]
pub fn copy_report_image(png: String) -> Result<(), String> {
    #[cfg(windows)]
    {
        use base64::Engine;
        if png.len()>16*1024*1024 {return Err("图片过大".into());}
        let bytes=base64::engine::general_purpose::STANDARD.decode(png).map_err(|e|e.to_string())?;
        let mut reader=image::ImageReader::with_format(std::io::Cursor::new(&bytes),image::ImageFormat::Png);
        let mut limits=image::Limits::default();limits.max_image_width=Some(4096);limits.max_image_height=Some(8192);limits.max_alloc=Some(128*1024*1024);
        reader.limits(limits);
        let img=reader.decode().map_err(|e|e.to_string())?.into_rgba8();
        if img.width()==0||img.height()==0{return Err("无效图片".into());}
        let dib=dibv5(&img);
        let format=clipboard_win::register_format("PNG").ok_or("无法注册 PNG 剪贴板格式")?.get();
        let _guard=clipboard_win::Clipboard::new_attempts(10).map_err(|e|e.to_string())?;
        clipboard_win::raw::set(format,&bytes).map_err(|e|e.to_string())?;
        // PNG is authoritative; DIBV5 enables paste into bitmap-only applications.
        let _=clipboard_win::raw::set_without_clear(clipboard_win::formats::CF_DIBV5,&dib);
        Ok(())
    }
    #[cfg(not(windows))]
    { let _=png; Err("请使用系统 WebView 剪贴板".into()) }
}
#[cfg(all(test,windows))]
mod tests {
    use super::*;
    #[test]
    fn dib_keeps_alpha_and_channel_order(){
        let img=image::RgbaImage::from_raw(1,2,vec![1,2,3,0,4,5,6,255]).unwrap();
        let b=dibv5(&img);
        assert_eq!(&b[52..56],&0xff000000u32.to_le_bytes());
        assert_eq!(&b[124..],&[6,5,4,255,3,2,1,0]);
    }
}
