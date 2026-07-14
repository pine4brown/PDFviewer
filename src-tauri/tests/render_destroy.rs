use pdfium_render::prelude::*;
use image::ImageFormat;

#[test]
fn test_render_twice() -> Result<(), Box<dyn std::error::Error>> {
    let lib_path = "/Users/brown/develop/PDFviewer/WaffleMatrix/src-tauri/target/release/bundle/macos/WaffleMatrix.app/Contents/Resources/resources/libpdfium.dylib";
    
    // First time
    {
        let bindings = Pdfium::bind_to_library(lib_path)?;
        let pdfium = Pdfium::new(bindings);
        let document = pdfium.load_pdf_from_file("/tmp/dummy.pdf", None)?;
        let page = document.pages().get(0)?;
        let config = PdfRenderConfig::new().set_target_width(800).set_clear_color(PdfColor::WHITE);
        let bitmap = page.render_with_config(&config)?;
        bitmap.as_image().save_with_format("/tmp/rendered_1.png", ImageFormat::Png)?;
        println!("Render 1 done");
    } // pdfium is dropped, FPDF_DestroyLibrary is called
    
    // Second time
    {
        let bindings = Pdfium::bind_to_library(lib_path)?;
        let pdfium = Pdfium::new(bindings);
        let document = pdfium.load_pdf_from_file("/tmp/dummy.pdf", None)?;
        let page = document.pages().get(0)?;
        let config = PdfRenderConfig::new().set_target_width(800).set_clear_color(PdfColor::WHITE);
        let bitmap = page.render_with_config(&config)?;
        
        let dynamic_image = bitmap.as_image();
        dynamic_image.save_with_format("/tmp/rendered_2.png", ImageFormat::Png)?;
        
        // Check if white
        let mut all_white = true;
        for pixel in dynamic_image.as_rgba8().unwrap().pixels() {
            if pixel[0] != 255 || pixel[1] != 255 || pixel[2] != 255 {
                all_white = false;
                break;
            }
        }
        assert!(!all_white, "Render 2 is entirely white!");
    }
    
    Ok(())
}
