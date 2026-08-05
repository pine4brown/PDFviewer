use pdfium_render::prelude::*;

mod common;

#[test]
fn test_render_bytes() -> Result<(), Box<dyn std::error::Error>> {
    let lib_path = common::pdfium_lib_path();

    let dummy = std::env::temp_dir().join("dummy.pdf");
    common::write_pdf_with_lines(dummy.to_str().unwrap(), &[("Render bytes".to_string(), 780.0)])?;

    let bindings = Pdfium::bind_to_library(lib_path)?;
    let pdfium = Pdfium::new(bindings);

    let bytes = std::fs::read(dummy)?;

    // Exactly as in page.rs
    let document = pdfium.load_pdf_from_byte_slice(&bytes, None)?;
    let page = document.pages().get(0)?;
    let config = PdfRenderConfig::new().set_target_width(800).set_clear_color(PdfColor::WHITE);
    let bitmap = page.render_with_config(&config)?;
    let dynamic_image = bitmap.as_image();
    let out = std::env::temp_dir().join("rendered_bytes.png");
    dynamic_image.save_with_format(&out, image::ImageFormat::Png)?;

    let mut all_white = true;
    for pixel in dynamic_image.as_rgba8().unwrap().pixels() {
        if pixel[0] != 255 || pixel[1] != 255 || pixel[2] != 255 {
            all_white = false;
            break;
        }
    }
    assert!(!all_white, "Rendered bytes image is entirely white!");

    Ok(())
}
