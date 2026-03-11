#!/usr/bin/env python3
import cv2
import sys
import os

# Get image path from args or use default
image_path = sys.argv[1] if len(sys.argv) > 1 else "markers/marker_0.png"

# Load image
image = cv2.imread(image_path)
if image is None:
    print(f"Error: Could not load image: {image_path}")
    sys.exit(1)

print(f"Loaded image: {image_path} ({image.shape[1]}x{image.shape[0]})")

# Create ArUco detector
dictionary = cv2.aruco.getPredefinedDictionary(cv2.aruco.DICT_6X6_250)
parameters = cv2.aruco.DetectorParameters()
detector = cv2.aruco.ArucoDetector(dictionary, parameters)

# Detect markers
corners, ids, rejected = detector.detectMarkers(image)

print(f"Detected {len(ids) if ids is not None else 0} markers")

# Draw results
output = image.copy()

if ids is not None:
    for i, marker_id in enumerate(ids):
        print(f"  Marker ID: {marker_id[0]}")
        
        # Draw bounding box
        corner = corners[i][0]
        for j in range(4):
            p1 = tuple(corner[j].astype(int))
            p2 = tuple(corner[(j + 1) % 4].astype(int))
            cv2.line(output, p1, p2, (0, 255, 0), 2)
        
        # Draw marker ID
        top_left = tuple(corner[0].astype(int))
        cv2.putText(output, f"ID: {marker_id[0]}", 
                   (top_left[0], top_left[1] - 10),
                   cv2.FONT_HERSHEY_SIMPLEX, 0.8, (0, 0, 255), 2)

if ids is None or len(ids) == 0:
    print("No markers detected!")

# Save output image
output_path = "detection_result.png"
cv2.imwrite(output_path, output)
print(f"\nSaved result to: {output_path}")

# Try to display if GUI is available
if os.environ.get('DISPLAY') or sys.platform == 'darwin':
    try:
        cv2.imshow("ArUco Detection", output)
        print("Press any key to exit...")
        cv2.waitKey(0)
        cv2.destroyAllWindows()
    except cv2.error:
        pass  # GUI not available
