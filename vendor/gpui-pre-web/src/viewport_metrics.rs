//! Resolve browser pixel boxes against CSS geometry and the reported scale.
pub(crate) fn physical_and_logical(
    css_width: f32,
    css_height: f32,
    dpr: f64,
    device_box: Option<(f64, f64)>,
) -> (u32, u32, f32, f32) {
    let expected_width = f64::from(css_width) * dpr;
    let expected_height = f64::from(css_height) * dpr;
    if let Some((width, height)) = device_box.filter(|(width, height)| {
        width.is_finite()
            && height.is_finite()
            && (*width - expected_width).abs() <= 1.
            && (*height - expected_height).abs() <= 1.
    }) {
        (
            width as u32,
            height as u32,
            (width / dpr) as f32,
            (height / dpr) as f32,
        )
    } else {
        // Emulated viewports may report the host display's pixel box even when
        // devicePixelRatio is overridden. CSS bounds remain the hit-test space.
        (
            expected_width.round() as u32,
            expected_height.round() as u32,
            css_width,
            css_height,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::physical_and_logical;
    #[test]
    fn safari_fallback_keeps_css_coordinates_on_retina() {
        assert_eq!(
            physical_and_logical(320., 900., 2., None),
            (640, 1800, 320., 900.)
        );
    }
    #[test]
    fn consistent_device_boxes_keep_the_existing_pixel_alignment() {
        assert_eq!(
            physical_and_logical(320., 900., 2., Some((640., 1800.))),
            (640, 1800, 320., 900.)
        );
    }
    #[test]
    fn emulation_must_not_double_layout_and_offset_pointer_targets() {
        assert_eq!(
            physical_and_logical(320., 900., 1., Some((640., 1800.))),
            (320, 900, 320., 900.)
        );
    }
    #[test]
    fn hidden_surfaces_remain_zero_sized() {
        assert_eq!(
            physical_and_logical(0., 0., 2., Some((0., 0.))),
            (0, 0, 0., 0.)
        );
    }
    #[test]
    fn invalid_device_boxes_fall_back_to_css_geometry() {
        assert_eq!(
            physical_and_logical(320., 900., 1., Some((f64::NAN, 1800.))),
            (320, 900, 320., 900.)
        );
    }
}
