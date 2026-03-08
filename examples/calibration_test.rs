//! # Calibration Controller Test Example
//!
//! This example demonstrates the calibration controller state machine
//! without requiring a camera or actual displays. It simulates the
//! calibration workflow and prints the progress.
//!
//! ## Usage
//!
//! ```bash
//! cargo run --example calibration_test --no-default-features
//! ```

use rusty_mapper::videowall::{
    CalibrationController, CalibrationPhase, CalibrationTiming, GridSize,
};
use std::thread;
use std::time::Duration;

fn main() -> anyhow::Result<()> {
    env_logger::init();
    
    println!("Calibration Controller Test Example");
    println!("====================================\n");
    
    // Create controller with fast timing for demo
    let timing = CalibrationTiming {
        countdown_seconds: 2,  // 2 second countdown
        frames_per_display: 1,
        ms_between_displays: 500, // 0.5 seconds per display
        capture_timeout_ms: 1000,
    };
    
    let mut controller = CalibrationController::new()
        .with_timing(timing)
        .with_auto_advance(true);
    
    // Start calibration for a 2x2 grid
    let grid_size = GridSize::two_by_two();
    let camera_resolution = (1920, 1080);
    let output_resolution = (1920, 1080);
    
    controller.start_realtime(grid_size, camera_resolution, output_resolution)?;
    
    println!("Starting calibration for {:?} grid", grid_size);
    println!("Total displays: {}", grid_size.total_displays());
    println!("Press Ctrl+C to cancel\n");
    
    // Main loop - simulate calibration
    loop {
        // Print current status
        print_status(&controller);
        
        // Update controller
        match controller.update() {
            rusty_mapper::videowall::CalibrationStatus::InProgress => {
                // Continue
            }
            rusty_mapper::videowall::CalibrationStatus::Complete(config) => {
                println!("\n✓ Calibration complete!");
                println!("  Displays configured: {}", config.displays.len());
                println!("  Grid size: {:?}", config.grid_size);
                println!("  Duration: {:.1} seconds", config.calibration_info.calibration_duration_secs);
                break;
            }
            rusty_mapper::videowall::CalibrationStatus::Error(e) => {
                println!("\n✗ Calibration failed: {}", e);
                break;
            }
        }
        
        // Simulate frame capture during flashing
        if let CalibrationPhase::Flashing { current_display, .. } = controller.phase() {
            // Simulate capturing a frame
            let frame_data = vec![0u8; 1920 * 1080 * 4]; // Dummy RGBA frame
            controller.submit_frame(frame_data, 1920, 1080);
        }
        
        // Small delay to not spam the console
        thread::sleep(Duration::from_millis(100));
    }
    
    Ok(())
}

fn print_status(controller: &CalibrationController) {
    let phase = controller.phase();
    let progress = controller.progress();
    
    // Clear line and print status
    print!("\r");
    
    match phase {
        CalibrationPhase::Idle => {
            print!("Status: Idle                    ");
        }
        CalibrationPhase::Countdown { seconds_remaining } => {
            print!(
                "Status: Countdown {}s... [{:>3.0}%]          ",
                seconds_remaining,
                progress * 100.0
            );
        }
        CalibrationPhase::Flashing { current_display, total_displays, .. } => {
            if let Some(pattern) = controller.current_pattern() {
                print!(
                    "Status: Flashing display {}/{} ({}x{}) [{:>3.0}%]    ",
                    current_display + 1,
                    total_displays,
                    pattern.width(),
                    pattern.height(),
                    progress * 100.0
                );
            }
        }
        CalibrationPhase::Processing { current, total } => {
            print!(
                "Status: Processing frame {}/{} [{:>3.0}%]          ",
                current,
                total,
                progress * 100.0
            );
        }
        CalibrationPhase::BuildingMap => {
            print!("Status: Building quad map [{:>3.0}%]              ", progress * 100.0);
        }
        CalibrationPhase::Complete => {
            print!("Status: Complete [{:>3.0}%]                        ", progress * 100.0);
        }
        CalibrationPhase::Error(ref e) => {
            print!("Status: Error - {}              ", e);
        }
        _ => {
            print!("Status: {:?} [{:>3.0}%]          ", phase, progress * 100.0);
        }
    }
    
    // Flush stdout
    use std::io::Write;
    let _ = std::io::stdout().flush();
}
