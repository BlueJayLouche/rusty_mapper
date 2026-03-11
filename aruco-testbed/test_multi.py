#!/usr/bin/env python3
import cv2
import numpy as np

# Create a blank canvas
canvas = np.ones((600, 800, 3), dtype=np.uint8) * 240  # Light gray background

# Get dictionary
aruco_dict = cv2.aruco.getPredefinedDictionary(cv2.aruco.DICT_6X6_250)

# Place markers at different positions
positions = [(100, 100), (400, 100), (250, 350)]
marker_ids = [0, 1, 3]

for (x, y), marker_id in zip(positions, marker_ids):
    # Generate marker (150x150)
    marker = cv2.aruco.generateImageMarker(aruco_dict, marker_id, 150)
    # Convert to BGR
    marker_bgr = cv2.cvtColor(marker, cv2.COLOR_GRAY2BGR)
    # Place on canvas
    canvas[y:y+150, x:x+150] = marker_bgr

# Save test image
cv2.imwrite("test_multi.png", canvas)
print("Created test_multi.png with 3 markers")

# Detect markers
detector = cv2.aruco.ArucoDetector(aruco_dict, cv2.aruco.DetectorParameters())
corners, ids, rejected = detector.detectMarkers(canvas)

print(f"\nDetected {len(ids) if ids is not None else 0} markers:")
if ids is not None:
    for marker_id in ids:
        print(f"  - ID: {marker_id[0]}")

# Draw detection results
output = canvas.copy()
if ids is not None:
    cv2.aruco.drawDetectedMarkers(output, corners, ids)
    
    for i, marker_id in enumerate(ids):
        corner = corners[i][0]
        top_left = tuple(corner[0].astype(int))
        cv2.putText(output, f"ID: {marker_id[0]}", 
                   (top_left[0], top_left[1] - 10),
                   cv2.FONT_HERSHEY_SIMPLEX, 0.7, (0, 0, 255), 2)

cv2.imwrite("test_multi_result.png", output)
print("\nSaved detection result to: test_multi_result.png")
