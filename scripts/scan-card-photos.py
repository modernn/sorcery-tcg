#!/usr/bin/env python3
"""Deterministic, offline card-photo detection and reference matching helper."""

from __future__ import annotations

import json
import math
import os
import platform
import sys
from pathlib import Path
from typing import Any

os.environ["OPENCV_IO_MAX_IMAGE_PIXELS"] = "50000000"

try:
    import cv2
    import numpy as np
except ImportError:
    sys.stderr.write("scanner unavailable\n")
    raise SystemExit(1)


MAX_PIXELS = 50_000_000
NORMALIZED_SIZE = (300, 420)


def exact_keys(value: Any, keys: set[str]) -> bool:
    return isinstance(value, dict) and set(value) == keys


def finite_number(value: Any) -> bool:
    return isinstance(value, (int, float)) and not isinstance(value, bool) and math.isfinite(value)


def parse_request() -> dict[str, Any]:
    raw = sys.stdin.read(8_000_001)
    if len(raw) > 8_000_000:
        raise ValueError

    def reject_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
        result: dict[str, Any] = {}
        for key, value in pairs:
            if key in result:
                raise ValueError
            result[key] = value
        return result

    request = json.loads(raw, object_pairs_hook=reject_duplicate_keys)
    if not exact_keys(request, {"schemaVersion", "photos", "references", "options"}):
        raise ValueError
    if request["schemaVersion"] != 1 or isinstance(request["schemaVersion"], bool):
        raise ValueError

    photos = request["photos"]
    references = request["references"]
    if not isinstance(photos, list) or len(photos) > 100:
        raise ValueError
    if not isinstance(references, list) or len(references) > 5000:
        raise ValueError

    photo_ids: set[str] = set()
    for photo in photos:
        if not exact_keys(photo, {"photoId", "path"}):
            raise ValueError
        if (
            not isinstance(photo["photoId"], str)
            or not 1 <= len(photo["photoId"]) <= 200
            or photo["photoId"] in photo_ids
        ):
            raise ValueError
        if not isinstance(photo["path"], str) or not photo["path"] or "\0" in photo["path"]:
            raise ValueError
        photo_ids.add(photo["photoId"])

    for reference in references:
        if not exact_keys(reference, {"labelId", "path"}):
            raise ValueError
        if not isinstance(reference["labelId"], str) or not 1 <= len(reference["labelId"]) <= 100:
            raise ValueError
        if not isinstance(reference["path"], str) or not reference["path"] or "\0" in reference["path"]:
            raise ValueError

    options = request["options"]
    option_keys = {
        "confidenceThreshold",
        "marginThreshold",
        "minimumCardAreaRatio",
        "maximumCardAreaRatio",
        "grid",
    }
    if not exact_keys(options, option_keys):
        raise ValueError
    for key in option_keys - {"grid"}:
        if not finite_number(options[key]):
            raise ValueError
    if not 0 <= options["confidenceThreshold"] <= 1 or not 0 <= options["marginThreshold"] <= 1:
        raise ValueError
    minimum = options["minimumCardAreaRatio"]
    maximum = options["maximumCardAreaRatio"]
    if not 0 < minimum < maximum <= 1:
        raise ValueError

    grid = options["grid"]
    if grid is not None:
        if not exact_keys(grid, {"columns", "rows"}):
            raise ValueError
        columns, rows = grid["columns"], grid["rows"]
        if any(not isinstance(value, int) or isinstance(value, bool) or value < 1 for value in (columns, rows)):
            raise ValueError
        if columns * rows > 5000:
            raise ValueError
    return request


def load_image(path: str) -> np.ndarray:
    encoded = np.frombuffer(Path(path).read_bytes(), dtype=np.uint8)
    image = cv2.imdecode(encoded, cv2.IMREAD_COLOR)
    if image is None or image.size == 0 or image.shape[0] * image.shape[1] > MAX_PIXELS:
        raise ValueError
    return image


def order_quad(points: np.ndarray) -> np.ndarray:
    points = np.asarray(points, dtype=np.float32).reshape(4, 2)
    center = points.mean(axis=0)
    ordered = points[np.argsort(np.arctan2(points[:, 1] - center[1], points[:, 0] - center[0]))]
    start = min(
        range(4),
        key=lambda index: (ordered[index, 0] + ordered[index, 1], ordered[index, 1], ordered[index, 0]),
    )
    return np.roll(ordered, -start, axis=0)


def rectify(image: np.ndarray, quad: np.ndarray) -> np.ndarray:
    quad = order_quad(quad)
    width = max(np.linalg.norm(quad[1] - quad[0]), np.linalg.norm(quad[2] - quad[3]))
    height = max(np.linalg.norm(quad[3] - quad[0]), np.linalg.norm(quad[2] - quad[1]))
    if width < 2 or height < 2:
        raise ValueError
    target_width, target_height = max(2, round(width)), max(2, round(height))
    destination = np.array(
        [[0, 0], [target_width - 1, 0], [target_width - 1, target_height - 1], [0, target_height - 1]],
        dtype=np.float32,
    )
    warped = cv2.warpPerspective(image, cv2.getPerspectiveTransform(quad, destination), (target_width, target_height))
    if target_width > target_height:
        warped = cv2.rotate(warped, cv2.ROTATE_90_CLOCKWISE)
    return cv2.resize(warped, NORMALIZED_SIZE, interpolation=cv2.INTER_AREA)


def grid_quads(image: np.ndarray, grid: dict[str, int]) -> list[np.ndarray]:
    height, width = image.shape[:2]
    cell_width, cell_height = width / grid["columns"], height / grid["rows"]
    inset_x, inset_y = cell_width * 0.02, cell_height * 0.02
    quads: list[np.ndarray] = []
    for row in range(grid["rows"]):
        for column in range(grid["columns"]):
            left = column * cell_width + inset_x
            top = row * cell_height + inset_y
            right = (column + 1) * cell_width - inset_x - 1
            bottom = (row + 1) * cell_height - inset_y - 1
            if right - left < 2 or bottom - top < 2:
                raise ValueError
            quads.append(np.array([[left, top], [right, top], [right, bottom], [left, bottom]], dtype=np.float32))
    return quads


def bounds(quad: np.ndarray) -> tuple[float, float, float, float]:
    return (
        float(np.min(quad[:, 0])),
        float(np.min(quad[:, 1])),
        float(np.max(quad[:, 0])),
        float(np.max(quad[:, 1])),
    )


def overlap(left: np.ndarray, right: np.ndarray) -> float:
    ax1, ay1, ax2, ay2 = bounds(left)
    bx1, by1, bx2, by2 = bounds(right)
    intersection = max(0.0, min(ax2, bx2) - max(ax1, bx1)) * max(0.0, min(ay2, by2) - max(ay1, by1))
    union = (ax2 - ax1) * (ay2 - ay1) + (bx2 - bx1) * (by2 - by1) - intersection
    return intersection / union if union > 0 else 0.0


def reading_order(quads: list[np.ndarray]) -> list[np.ndarray]:
    if not quads:
        return []
    median_height = float(np.median([bounds(quad)[3] - bounds(quad)[1] for quad in quads]))
    rows: list[list[np.ndarray]] = []
    for quad in sorted(quads, key=lambda item: (float(np.mean(item[:, 1])), float(np.mean(item[:, 0])))):
        center_y = float(np.mean(quad[:, 1]))
        if not rows or abs(center_y - np.mean([np.mean(item[:, 1]) for item in rows[-1]])) > median_height * 0.4:
            rows.append([quad])
        else:
            rows[-1].append(quad)
    return [quad for row in rows for quad in sorted(row, key=lambda item: float(np.mean(item[:, 0])))]


def contour_quads(image: np.ndarray, minimum_ratio: float, maximum_ratio: float) -> list[np.ndarray]:
    gray = cv2.cvtColor(image, cv2.COLOR_BGR2GRAY)
    blurred = cv2.GaussianBlur(gray, (5, 5), 0)
    edges = cv2.Canny(blurred, 50, 150)
    edges = cv2.morphologyEx(edges, cv2.MORPH_CLOSE, np.ones((5, 5), dtype=np.uint8))
    contours, _ = cv2.findContours(edges, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_SIMPLE)
    image_area = image.shape[0] * image.shape[1]
    candidates: list[tuple[float, np.ndarray]] = []
    for contour in contours:
        area = abs(cv2.contourArea(contour))
        if not minimum_ratio * image_area <= area <= maximum_ratio * image_area:
            continue
        perimeter = cv2.arcLength(contour, True)
        polygon = cv2.approxPolyDP(contour, perimeter * 0.02, True)
        if len(polygon) == 4 and cv2.isContourConvex(polygon):
            quad = polygon.reshape(4, 2).astype(np.float32)
        else:
            rectangle = cv2.minAreaRect(contour)
            short, long = sorted(rectangle[1])
            if short < 2 or long < 2 or not 0.55 <= short / long <= 0.85:
                continue
            quad = cv2.boxPoints(rectangle).astype(np.float32)
        ordered = order_quad(quad)
        side_lengths = [np.linalg.norm(ordered[(index + 1) % 4] - ordered[index]) for index in range(4)]
        short, long = sorted((sum(side_lengths[::2]) / 2, sum(side_lengths[1::2]) / 2))
        if long < 2 or not 0.55 <= short / long <= 0.85:
            continue
        candidates.append((area, ordered))

    selected: list[np.ndarray] = []
    for _, quad in sorted(candidates, key=lambda item: (-item[0], bounds(item[1]))):
        if all(overlap(quad, existing) < 0.5 for existing in selected):
            selected.append(quad)
    return reading_order(selected)


def descriptors(orb: Any, image: np.ndarray) -> np.ndarray | None:
    gray = cv2.cvtColor(image, cv2.COLOR_BGR2GRAY)
    _, result = orb.detectAndCompute(gray, None)
    return result


def reference_descriptors(orb: Any, references: list[dict[str, str]]) -> list[tuple[str, np.ndarray | None]]:
    result: list[tuple[str, np.ndarray | None]] = []
    for reference in references:
        image = load_image(reference["path"])
        height, width = image.shape[:2]
        quad = np.array([[0, 0], [width - 1, 0], [width - 1, height - 1], [0, height - 1]], dtype=np.float32)
        result.append((reference["labelId"], descriptors(orb, rectify(image, quad))))
    return result


def candidates_for(
    orb: Any,
    matcher: Any,
    card: np.ndarray,
    references: list[tuple[str, np.ndarray | None]],
) -> list[dict[str, Any]]:
    query = descriptors(orb, card)
    scores: dict[str, float] = {}
    for label_id, reference in references:
        score = 0.0
        if query is not None and reference is not None and len(query) >= 2 and len(reference) >= 2:
            pairs = matcher.knnMatch(query, reference, k=2)
            good = [first for pair in pairs if len(pair) == 2 for first, second in [pair] if first.distance < 0.75 * second.distance]
            if good:
                quality = sum(1.0 - match.distance / 256.0 for match in good) / len(good)
                score = min(1.0, len(good) / 40.0) * quality
        scores[label_id] = max(scores.get(label_id, 0.0), score)
    return [
        {"labelId": label_id, "confidence": round(score, 6)}
        for label_id, score in sorted(scores.items(), key=lambda item: (-item[1], item[0]))[:3]
    ]


def public_quad(quad: np.ndarray) -> list[list[int]]:
    return [[int(round(float(x))), int(round(float(y)))] for x, y in order_quad(quad)]


def clip_quad(quad: np.ndarray, image: np.ndarray) -> np.ndarray:
    clipped = np.asarray(quad, dtype=np.float32).copy()
    clipped[:, 0] = np.clip(clipped[:, 0], 0, image.shape[1] - 1)
    clipped[:, 1] = np.clip(clipped[:, 1], 0, image.shape[0] - 1)
    return order_quad(clipped)


def run(request: dict[str, Any]) -> dict[str, Any]:
    cv2.setNumThreads(1)
    cv2.setRNGSeed(0)
    orb = cv2.ORB_create(nfeatures=750)
    matcher = cv2.BFMatcher(cv2.NORM_HAMMING, crossCheck=False)
    references = reference_descriptors(orb, request["references"])
    options = request["options"]
    output_photos: list[dict[str, Any]] = []
    for photo in request["photos"]:
        image = load_image(photo["path"])
        quads = (
            grid_quads(image, options["grid"])
            if options["grid"] is not None
            else contour_quads(image, options["minimumCardAreaRatio"], options["maximumCardAreaRatio"])
        )
        quads = [clip_quad(quad, image) for quad in quads]
        detections: list[dict[str, Any]] = []
        for index, quad in enumerate(quads):
            candidates = candidates_for(orb, matcher, rectify(image, quad), references)
            best = candidates[0]["confidence"] if candidates else 0.0
            second = candidates[1]["confidence"] if len(candidates) > 1 else 0.0
            status = (
                "confirmed"
                if candidates
                and best > 0
                and best > second
                and best >= options["confidenceThreshold"]
                and best - second >= options["marginThreshold"]
                else "review"
            )
            detections.append(
                {
                    "detectionId": f'{photo["photoId"]}:{index}',
                    "quad": public_quad(quad),
                    "candidates": candidates,
                    "status": status,
                }
            )
        output_photos.append({"photoId": photo["photoId"], "detections": detections})
    return {
        "schemaVersion": 1,
        "runtime": {"python": platform.python_version(), "opencv": cv2.__version__},
        "photos": output_photos,
    }


def main() -> int:
    try:
        response = run(parse_request())
        json.dump(response, sys.stdout, separators=(",", ":"))
        sys.stdout.write("\n")
        return 0
    except Exception:
        sys.stderr.write("scan failed\n")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
