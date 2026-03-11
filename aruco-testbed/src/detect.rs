use opencv::{
    core::{Scalar, Vector},
    highgui::{imshow, wait_key, destroy_all_windows},
    imgcodecs::imread,
    imgproc::{line, put_text, FONT_HERSHEY_SIMPLEX, LINE_8},
    objdetect::{get_predefined_dictionary, ArucoDetector, DetectorParameters, Objdetect_ArucoDictionary},
    prelude::*,
};

fn main() {
    // Get image path from args or use default
    let image_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "markers/marker_0.png".to_string());
    
    // Load image
    let image = imread(&image_path, opencv::imgcodecs::IMREAD_COLOR)
        .expect("Failed to load image. Usage: cargo run --bin detect -- <image_path>");
    
    if image.empty() {
        panic!("Could not open image: {}", image_path);
    }
    
    println!("Loaded image: {} ({}x{})", image_path, image.cols(), image.rows());
    
    // Create ArUco detector
    let dictionary = get_predefined_dictionary(
        Objdetect_ArucoDictionary::DICT_6X6_250
    ).expect("Failed to get dictionary");
    
    let detector_params = DetectorParameters::default()
        .expect("Failed to create detector params");
    
    let detector = ArucoDetector::new(
        &dictionary,
        &detector_params,
        &opencv::objdetect::RefineParameters::default().unwrap()
    ).expect("Failed to create detector");
    
    // Detect markers
    let mut marker_corners = Vector::<Vector<Point2f>>::new();
    let mut marker_ids = Vector::<i32>::new();
    let mut rejected_candidates = Vector::<Vector<Point2f>>::new();
    
    detector.detect_markers(
        &image,
        &mut marker_corners,
        &mut marker_ids,
        &mut rejected_candidates
    ).expect("Detection failed");
    
    println!("Detected {} markers", marker_ids.len());
    
    // Draw results on a copy of the image
    let mut output = image.clone();
    
    for i in 0..marker_ids.len() {
        let id = marker_ids.get(i).unwrap();
        let corners = marker_corners.get(i).unwrap();
        
        println!("  Marker ID: {} at corners: {:?}", id, corners);
        
        // Draw bounding box
        let points: Vec<Point> = corners.iter()
            .map(|p| Point::new(p.x as i32, p.y as i32))
            .collect();
        
        // Draw lines connecting corners
        let color = Scalar::new(0.0, 255.0, 0.0, 0.0); // Green
        for j in 0..4 {
            let p1 = points[j];
            let p2 = points[(j + 1) % 4];
            line(&mut output, p1, p2, color, 2, LINE_8, 0)
                .expect("Failed to draw line");
        }
        
        // Draw marker ID near first corner
        let text_pos = Point::new(points[0].x, points[0].y - 10);
        put_text(
            &mut output,
            &format!("ID: {}", id),
            text_pos,
            FONT_HERSHEY_SIMPLEX,
            0.8,
            Scalar::new(0.0, 0.0, 255.0, 0.0), // Red
            2,
            LINE_8,
            false
        ).expect("Failed to put text");
    }
    
    if marker_ids.is_empty() {
        println!("No markers detected!");
    }
    
    // Display result
    imshow("ArUco Detection", &output)
        .expect("Failed to show image");
    
    println!("\nPress any key to exit...");
    wait_key(0).expect("wait_key failed");
    destroy_all_windows().expect("destroy_all_windows failed");
}
