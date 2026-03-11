#!/usr/bin/env python3
import cv2
import os
import numpy as np

# Create output directory
os.makedirs("markers", exist_ok=True)

# Get the 6x6_250 dictionary
aruco_dict = cv2.aruco.getPredefinedDictionary(cv2.aruco.DICT_6X6_250)

# Generate first 5 markers with padding for detection
for marker_id in range(5):
    # Generate marker image (200x200 pixels)
    marker_img = cv2.aruco.generateImageMarker(aruco_dict, marker_id, 200)
    
    # Add white padding around the marker (50px on each side)
    padded = cv2.copyMakeBorder(marker_img, 50, 50, 50, 50, 
                                 cv2.BORDER_CONSTANT, value=255)
    
    # Save as PNG
    filename = f"markers/marker_{marker_id}.png"
    cv2.imwrite(filename, padded)
    print(f"Generated: {filename}")

print("\nAll markers saved to 'markers/' directory")
