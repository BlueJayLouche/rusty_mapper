use opencv::{
    imgcodecs::{imwrite, IMWRITE_PNG_COMPRESSION},
    objdetect::{get_predefined_dictionary, DetectorParameters, Objdetect_ArucoDictionary, generate_image_marker},
    prelude::*,
};

fn main() {
    // Create output directory
    std::fs::create_dir_all("markers").expect("Failed to create markers directory");
    
    // Use the 6x6_250 dictionary (6x6 bits, 250 possible markers)
    let dictionary: Objdetect_ArucoDictionary = get_predefined_dictionary(
        Objdetect_ArucoDictionary::DICT_6X6_250
    ).expect("Failed to get dictionary");
    
    // Generate first 5 markers
    for marker_id in 0..5 {
        let mut marker_img = Mat::default();
        
        // Generate marker image (200x200 pixels with 1-bit border)
        generate_image_marker(
            &dictionary,
            marker_id,
            200,  // side pixels
            &mut marker_img,
            1     // border bits
        ).expect("Failed to generate marker");
        
        // Save as PNG
        let filename = format!("markers/marker_{}.png", marker_id);
        let params = Vector::<i32>::from_slice(&[IMWRITE_PNG_COMPRESSION as i32, 9]);
        
        imwrite(&filename, &marker_img, &params)
            .expect("Failed to save marker");
        
        println!("Generated: {}", filename);
    }
    
    println!("\nAll markers saved to 'markers/' directory");
}
