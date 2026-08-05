# Project Specification: Hybrid PDF Diff Tool in Rust

## 1. Project Overview
This document outlines the development architecture for a robust, hybrid PDF comparison tool written in Rust. The tool must accurately identify differences between two PDF files, supporting two distinct modes of comparison to handle various types of engineering documents (e.g., text-heavy data sheets/specifications and visual-heavy schematic diagrams/layouts).

## 2. Target Development Environment
*   **Language:** Rust (Latest Stable)
*   **Recommended IDEs/Agents:** Designed for consumption by autonomous development agents (e.g., Claude Code, Jules) and agentic IDEs such as Antigravity (Google's AI IDE forked from VS Code).

## 3. Core Architecture & Tech Stack

The system should be modular, strictly separating PDF parsing, diff computation, and output generation.

### 3.1 Dependencies (Cargo.toml recommendations)
*   **PDF Engine:** `pdfium-render` (Preferred for high-fidelity rasterization and text extraction with bounding boxes) or `lopdf` (for low-level AST/dictionary inspection).
*   **Image Processing:** `image` (for memory buffering), `opencv` (Rust bindings, for feature-matching and alignment in schematics).
*   **Visual Diff Algorithm:** A Rust implementation of `pixelmatch` (or custom threshold-based pixel comparison).
*   **Text Diff Algorithm:** `similar` (for high-performance Myers diff implementation on extracted text streams).
*   **Serialization/Output:** `serde`, `serde_json`.

### 3.2 System Modules
1.  **`pdf_loader`:** Handles I/O, verifies PDF integrity, and normalizes page counts/sizes.
2.  **`text_analyzer`:** Extracts text streams alongside their physical X/Y bounding boxes. Handles reading order heuristics to mitigate table/column formatting issues.
3.  **`visual_analyzer`:** Rasterizes pages at a specified DPI (e.g., 300 DPI) into standard image buffers.
4.  **`diff_engine`:**
    *   *Sub-module A (Text):* Computes semantic differences (insertions/deletions) using the `similar` crate.
    *   *Sub-module B (Visual):* Applies alignment (AKAZE/ORB via OpenCV) to correct minor offsets, then computes threshold-based pixel differences.
5.  **`reporter`:** Generates the final output (e.g., a side-by-side HTML report, JSON diff map, or composite Diff Images with bounding box highlights).

## 4. Implementation Directives for the AI Agent

**Phase 1: CLI Foundation & Core Logic**
*   Implement the project initially as a CLI tool (`clap`). Do not implement a GUI (Tauri/egui) until the core parsing and diffing libraries have high test coverage.
*   **Agent Task:** Set up the Rust workspace. Create the CLI entry points taking `--old <FILE>` and `--new <FILE>` arguments, with a `--mode <text|visual|hybrid>` flag.

**Phase 2: Text Diff Engine (Data Sheets & Specs)**
*   **Challenge:** PDFs lack semantic HTML-like structure. Table extraction is prone to failure.
*   **Agent Task:** Utilize `pdfium-render` to extract text with spatial coordinates. Implement a sorting algorithm (top-to-bottom, left-to-right) before passing the sanitized strings to `similar`. Map the diff results back to their original coordinates to draw highlight bounding boxes.

**Phase 3: Visual Diff Engine (Schematics & Layouts)**
*   **Challenge:** Rendering anti-aliasing and minor sub-pixel shifts cause false positives in strict pixel comparisons.
*   **Agent Task:** Rasterize pages. Implement an alignment step before comparison. Use a pixel-matching function that incorporates color distance and neighboring pixel anti-aliasing checks (similar to the JS `pixelmatch` logic). Generate an output image where diff pixels are colored `#FF00FF` (Magenta).

## 5. Testing Requirements
*   The agent must generate synthetic test PDFs during the test phase.
*   Write unit tests for the sorting heuristics in `text_analyzer`.
*   Write integration tests comparing known-good baseline images against the output of the `visual_analyzer`.
