// AprilTag detection example - Pure Rust alternative to ArUco

fn main() {
    println!("AprilTag Detection Demo (Pure Rust)");
    println!("====================================");
    
    println!("\nTo use AprilTags in Rust, add these dependencies to Cargo.toml:");
    println!("\n[dependencies]");
    println!("apriltag = \"0.4\"");
    println!("apriltag-image = \"0.1\"");
    println!("image = \"0.25\"");
    
    println!("\nExample code:");
    println!("-------------");
    println!("use apriltag::{{Detector, Family, Image}};");
    println!();
    println!("fn detect_apriltags(image_path: &str) {{");
    println!("    let img = image::open(image_path).unwrap().to_luma8();");
    println!("    let (width, height) = img.dimensions();");
    println!("    let image = Image::from_buffer(width as usize, height as usize, &img);");
    println!();
    println!("    let mut detector = Detector::new();");
    println!("    detector.add_family(Family::tag_36h11());");
    println!();
    println!("    let detections = detector.detect(&image);");
    println!();
    println!("    for det in detections {{");
    println!("        println!(\"Detected ID: {{}}\", det.id());");
    println!("        let c = det.center();");
    println!("        println!(\"  Center: ({{:.1}}, {{:.1}})\", c.x(), c.y());");
    println!("    }}");
    println!("}}");

    println!("\nKey differences from ArUco:");
    println!("- AprilTags use different marker patterns (not compatible with ArUco)");
    println!("- Better Rust support (pure Rust implementation)");
    println!("- Used by NASA, more robust at long range");
    println!("- Slightly slower than ArUco but more accurate");
    
    println!("\nDownload markers from:");
    println!("https://github.com/AprilRobotics/apriltag-imgs");
}
