use arboard::Clipboard;
use base64::Engine;
use image::ImageBuffer;

struct ClipboardImage<'a>(arboard::ImageData<'a>);
impl ClipboardImage<'_> {
    fn to_img(&self) -> ImageBuffer<image::Rgba<u8>, Vec<u8>> {
        let cimg = &self.0;
        let (w, h) = (cimg.width as u32, cimg.height as u32);
        let mut img = ImageBuffer::new(w, h);
        for (x, y, pixel) in img.enumerate_pixels_mut() {
            let index = (y * w + x) as usize * 4;
            *pixel = image::Rgba([
                cimg.bytes[index],
                cimg.bytes[index + 1],
                cimg.bytes[index + 2],
                cimg.bytes[index + 3],
            ]);
        }
        img
    }
    fn to_jpeg(&self) -> JpegInMemory {
        let img = self.to_img();
        let mut buf = Vec::new();
        let mut encoder = image::codecs::jpeg::JpegEncoder::new(&mut buf);
        encoder.encode_image(&img).unwrap();
        JpegInMemory(buf)
    }
    fn to_jpeg_base64(&self) -> String {
        self.to_jpeg().base64()
    }
}

struct JpegInMemory(Vec<u8>);
impl JpegInMemory {
    fn base64(&self) -> String {
        base64::engine::general_purpose::STANDARD.encode(&self.0)
    }
}

pub fn get_img_base64_from_clipboard() -> String {
    let mut clip = Clipboard::new().unwrap();
    let img = clip.get_image().unwrap();
    let img = ClipboardImage(img);
    img.to_jpeg_base64()
}
pub fn set_clipboard(text: &str) {
    let mut clip = Clipboard::new().unwrap();
    clip.set_text(text).unwrap();
}
