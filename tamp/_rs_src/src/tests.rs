use std::path::Path;
use std::fs;
use crate::compressor;
use crate::decompressor;
/// Helper to get all (.txt, .tamp) file pairs from the test_files directory
fn get_test_file_pairs() -> Vec<(String, String, u8, u8)> {
    let dir = Path::new("test_files");
    let mut txt_files = vec![];
    let mut tamp_files = vec![];
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                match ext {
                    "txt" => txt_files.push(path.clone()),
                    "tamp" => tamp_files.push(path.clone()),
                    _ => {}
                }
            }
        }
    }
    let mut pairs = vec![];
    for txt in &txt_files {
        if let Some(stem) = txt.file_stem().and_then(|s| s.to_str()) {
            let tamp = tamp_files.iter().find(|f| f.file_stem().and_then(|s| s.to_str()) == Some(stem));
            if let Some(tamp) = tamp {
                // Try to extract window and literal from the stem, e.g. name_10_8
                let mut window = 10u8;
                let mut literal = 8u8;
                let parts: Vec<&str> = stem.split('_').collect();
                if parts.len() >= 3 {
                    if let (Ok(w), Ok(l)) = (parts[parts.len()-2].parse::<u8>(), parts[parts.len()-1].parse::<u8>()) {
                        window = w;
                        literal = l;
                    }
                }
                pairs.push((
                    txt.to_string_lossy().to_string(),
                    tamp.to_string_lossy().to_string(),
                    window,
                    literal,
                ));
            }
        }
    }
    pairs
}

/// Helper to print the first difference between two files byte-by-byte
fn print_first_diff(left: &[u8], right: &[u8]) {
    let min_len = left.len().min(right.len());
    for i in 0..min_len {
        if left[i] != right[i] {
            println!("Difference at byte {}: Rust = 0x{:02X}, Python = 0x{:02X}", i, left[i], right[i]);
            println!("Context (Rust):   {:?} length: {}", &left, &left.len());
            println!("Context (Python): {:?} length: {}", &right, &right.len());
            return;
        }
    }
    if left.len() != right.len() {
        println!("Files differ in length: Rust = {}, Python = {}", left.len(), right.len());
    } else {
        println!("No differences found.");
    }
}

#[test]
fn test_cyclic_small(){
    // Compress sample data, decompress it, and check if the result matches the original data
    let data: Vec<u8> = b"Hello, world!".to_vec();
    let compressed = compressor::compress(compressor::CompressInput::Bytes(&data), 10, 8, None).unwrap();
    let decompressed = decompressor::decompress(&compressed, None).unwrap();
    assert_eq!(data, decompressed);
}

#[test]
fn test_cyclic_large(){
    // Compress sample data, decompress it, and check if the result matches the original data
    let data: Vec<u8> = b"A".repeat(1000).into();
    let compressed = compressor::compress(compressor::CompressInput::Bytes(&data), 10, 8, None).unwrap();
    let decompressed = decompressor::decompress(&compressed, None).unwrap();
    assert_eq!(data, decompressed);
}

#[test]
fn test_interoperability_compression() {
    // Test that the compressor and decompressor produce the same output as the python implementation
    // For each pair of files in "test_files", compress the text, compare to the decompressed text
    let test_files = get_test_file_pairs();

    for (input_file, output_file, window, literal) in test_files {
        let uncompressed = std::fs::read(&input_file).expect("Failed to read input file");
        let expected_compressed = std::fs::read(&output_file).expect("Failed to read output file");

        let compressed = compressor::compress(compressor::CompressInput::Bytes(&uncompressed), window, literal, None).expect("Compression failed");
        if compressed != expected_compressed {
            println!("Compressed data does not match expected output for file: {}", input_file);
            print_first_diff(&compressed, &expected_compressed);
            // write the compressed data to a file for manual inspection
            std::fs::write("debug_compressed_output.tamp", &compressed).expect("Failed to write compressed output for inspection");
            assert_eq!(compressed, expected_compressed, "Compressed data does not match expected output for file: {}", input_file);
        }

    }
}

#[test]
fn test_interoperability_decompression() {
    // Test that the compressor and decompressor produce the same output as the python implementation
    // For each pair of files in "test_files", compress the text, compare to the decompressed text
    let test_files = get_test_file_pairs();

    for (input_file, output_file, _window, _literal) in test_files {
        let uncompressed = std::fs::read(&input_file).expect("Failed to read input file");
        let expected_compressed = std::fs::read(output_file).expect("Failed to read output file");

        let decompressed = decompressor::decompress(&expected_compressed, None).expect("Decompression failed");
        assert_eq!(uncompressed, decompressed, "Decompressed data does not match original input for file: {}", input_file);
    }
}