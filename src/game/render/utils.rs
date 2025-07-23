use bevy::image::Image;

// Function to draw a rectangle on an Image
pub fn draw_rect_on_image(
    image: &mut Image,
    rect_x: usize,
    rect_y: usize,
    rect_width: usize,
    rect_height: usize,
    color: [u8; 4],
) {
    let image_width = image.width() as usize;
    let image_height = image.height() as usize;

    // info!("Drawing RECKT: {rect_x} {rect_y} {rect_width} {rect_height} {color:?}");
    
    // Ensure the image format is RGBA8UnormSrgb for easy pixel manipulation
    // If your image uses a different format, you'll need to adjust how you write pixel data.
    // assert_eq!(image.texture_format, TextureFormat::Rgba8UnormSrgb, "This function only supports RGBA8UnormSrgb images for simplicity.");

    // Iterate over the pixels within the rectangle bounds
    for y in rect_y..(rect_y + rect_height).min(image_height) {
        for x in rect_x..(rect_x + rect_width).min(image_width) {
            let index = ((y * image_width + x) * 4) as usize; // Each pixel is 4 bytes (R, G, B, A)

            if let Some(pixel_slice) = image.data.as_deref_mut().unwrap().get_mut(index..index + 4) {
                pixel_slice.copy_from_slice(&color);
            }
        }
    }
}
