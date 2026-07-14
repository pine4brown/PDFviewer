use pdfium_render::prelude::*;
use image::ImageFormat;
use std::io::Cursor;

#[test]
fn test_render_dummy_pdf() -> Result<(), Box<dyn std::error::Error>> {
    let lib_path = "/Users/brown/develop/PDFviewer/WaffleMatrix/src-tauri/target/release/bundle/macos/WaffleMatrix.app/Contents/Resources/resources/libpdfium.dylib";
    
    let bindings = Pdfium::bind_to_library(lib_path)?;
    let pdfium = Pdfium::new(bindings);
    
    let document = pdfium.load_pdf_from_file("/tmp/dummy.pdf", None)?;
    let page = document.pages().get(0)?;
    
    let config = PdfRenderConfig::new()
        .set_target_width(800)
        .set_clear_color(PdfColor::WHITE)
        .render_form_data(true)
        .render_annotations(true);
        
    let bitmap = page.render_with_config(&config)?;
    let dynamic_image = bitmap.as_image();
    
    // Save it to disk so we can see if it's white or has content.
    dynamic_image.save_with_format("/tmp/rendered_test.png", ImageFormat::Png)?;
    
    // Check if the image is entirely white
    let mut all_white = true;
    for pixel in dynamic_image.as_rgba8().unwrap().pixels() {
        if pixel[0] != 255 || pixel[1] != 255 || pixel[2] != 255 {
            all_white = false;
            break;
        }
    }
    
    assert!(!all_white, "Image is entirely white!");
    
    Ok(())
}
