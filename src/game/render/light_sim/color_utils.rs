use std::i32;

/// Converts a color from a computational [i32; 3] representation (RGB)
/// to a standard [u8; 4] RGBA representation.
///
/// Each i32 color component (0 to i32::MAX) is mapped to u8 (0 to 255)
/// using a logarithmic scale. Negative i32 values are clamped to 0.
/// The alpha component is always set to 255 (fully opaque).
///
/// # Arguments
///
/// * `comp_color` - An array of three i32 values representing Red, Green, and Blue.
///                  Expected range for each component is 0 to i32::MAX.
///
/// # Returns
///
/// An array of four u8 values representing Red, Green, Blue, and Alpha.
///
/// # Examples
///
/// ```
/// // Assuming `convert_color` is in scope
/// use std::i32;
/// # fn convert_color(comp_color: [i32; 3]) -> [u8; 4] {
/// #     let i32_max_f64 = i32::MAX as f64;
/// #     let mut rgba_color = [0; 4];
/// #
/// #     for i in 0..3 {
/// #         let value_f64 = comp_color[i] as f64;
/// #
/// #         if value_f64 <= 0.0 {
/// #             rgba_color[i] = 0;
/// #         } else {
/// #             let normalized_val = value_f64 / i32_max_f64;
/// #             let log_scaled_0_to_1 = (normalized_val * 9.0 + 1.0).log10();
/// #             let final_scaled_u8 = (log_scaled_0_to_1 * 255.0).round() as u8;
/// #             rgba_color[i] = final_scaled_u8;
/// #         }
/// #     }
/// #     rgba_color[3] = 255;
/// #     rgba_color
/// # }
///
/// let computational_color_min = [0, 0, 0];
/// let rgba_color_min = convert_color(computational_color_min);
/// assert_eq!(rgba_color_min, [0, 0, 0, 255]);
///
/// let computational_color_max = [i32::MAX, i32::MAX, i32::MAX];
/// let rgba_color_max = convert_color(computational_color_max);
/// assert_eq!(rgba_color_max, [255, 255, 255, 255]);
///
/// let computational_color_mid = [i32::MAX / 2, i32::MAX / 4, -100];
/// let rgba_color_mid = convert_color(computational_color_mid);
/// // For i32::MAX / 2 (approx 1.07e9), the log-scaled u8 value is 189.
/// // For i32::MAX / 4 (approx 5.36e8), the log-scaled u8 value is 131.
/// assert_eq!(rgba_color_mid[0], 189);
/// assert_eq!(rgba_color_mid[1], 131);
/// assert_eq!(rgba_color_mid[2], 0); // Negative input is clamped to 0
/// assert_eq!(rgba_color_mid[3], 255); // Alpha is always 255
/// ```
pub fn convert_color(comp_color: [i32; 3]) -> [u8; 4] {
    // Cache i32::MAX as an f64 to avoid repeated casting inside the loop.
    // This provides a constant for normalization.
    let i32_max_f64 = i32::MAX as f64;
    // Initialize the output RGBA color array with default values.
    let mut rgba_color = [0; 4];

    // Iterate over the R, G, B components.
    for i in 0..3 {
        // Cast the current i32 component to f64 for floating-point calculations.
        let value_f64 = comp_color[i] as f64;

        // Handle values less than or equal to 0: clamp them to 0.
        if value_f64 <= 0.0 {
            rgba_color[i] = 0;
        } else {
            // Normalize the i32 value to a [0, 1] range.
            // This is done by dividing by the maximum possible i32 value.
            let normalized_val = value_f64 / i32_max_f64;

            // Apply logarithmic scaling.
            // The formula `(normalized_val * 9.0 + 1.0).log10()` creates a logarithmic curve.
            // - When `normalized_val` is 0, `log10(1.0)` is 0.
            // - When `normalized_val` is 1, `log10(9.0 + 1.0)` which is `log10(10.0)` is 1.
            // This effectively maps the [0, 1] normalized range logarithmically to [0, 1].
            let log_scaled_0_to_1 = (normalized_val * 9.0 + 1.0).log10();

            // Scale the logarithmically transformed value to the [0, 255] u8 range.
            // `round()` is used to get the nearest integer before casting to u8,
            // which helps in minimizing precision loss.
            let final_scaled_u8 = (log_scaled_0_to_1 * 255.0).round() as u8;
            rgba_color[i] = final_scaled_u8;
        }
    }

    // Set the alpha component to 255, indicating full opacity.
    rgba_color[3] = 255;

    rgba_color
}
