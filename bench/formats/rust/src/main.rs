//! JSON, MessagePack and CBOR in Rust with serde, decoding into typed structs as a
//! reflectapi server does, plus the CPU cost of gzip, brotli and zstd.
//!   cargo run --release
use std::hint::black_box;
use std::io::{Read, Write};
use std::time::Instant;

use serde::{de::DeserializeOwned, Serialize};

#[path = "types.rs"]
mod types;
use types::*;

/// Median time per call in ms, over 15 batches after a warm-up.
fn time<R>(mut f: impl FnMut() -> R) -> f64 {
    let t0 = Instant::now();
    let mut n = 0u64;
    while t0.elapsed().as_millis() < 300 {
        black_box(f());
        n += 1;
    }
    let per = (n / 3).max(1);
    let mut samples: Vec<f64> = (0..15)
        .map(|_| {
            let s = Instant::now();
            for _ in 0..per {
                black_box(f());
            }
            s.elapsed().as_secs_f64() * 1e3 / per as f64
        })
        .collect();
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
    samples[7]
}

fn fmt(ms: f64) -> String {
    if ms < 0.1 {
        format!("{:.1} µs", ms * 1e3)
    } else {
        format!("{ms:.2} ms")
    }
}

fn formats<T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug>(name: &str, json: &[u8]) {
    let value: T = serde_json::from_slice(json).unwrap();
    let json = serde_json::to_vec(&value).unwrap();
    let mpk = rmp_serde::to_vec_named(&value).unwrap();
    let mut cbor = Vec::new();
    ciborium::into_writer(&value, &mut cbor).unwrap();
    let mcbor = minicbor_serde::to_vec(&value).unwrap();

    // Every decoder gives back the same value.
    assert_eq!(sonic_rs::from_slice::<T>(&json).unwrap(), value);
    assert_eq!(simd_json::serde::from_slice::<T>(&mut json.clone()).unwrap(), value);
    assert_eq!(rmp_serde::from_slice::<T>(&mpk).unwrap(), value);
    assert_eq!(ciborium::from_reader::<T, _>(&cbor[..]).unwrap(), value);
    assert_eq!(minicbor_serde::from_slice::<T>(&mcbor).unwrap(), value);

    println!("## {name}");
    println!("| Format | Raw KB | Decode | Encode |");
    println!("|---|---:|---:|---:|");
    let kb = |b: &[u8]| format!("{:.1}", b.len() as f64 / 1024.0);
    let rows: Vec<(&str, &[u8], f64, f64)> = vec![
        (
            "JSON, serde_json",
            &json,
            time(|| serde_json::from_slice::<T>(&json).unwrap()),
            time(|| serde_json::to_vec(&value).unwrap()),
        ),
        (
            "JSON, sonic-rs",
            &json,
            time(|| sonic_rs::from_slice::<T>(&json).unwrap()),
            time(|| sonic_rs::to_vec(&value).unwrap()),
        ),
        (
            "JSON, simd-json (incl. buffer copy)",
            &json,
            time(|| simd_json::serde::from_slice::<T>(&mut json.clone()).unwrap()),
            time(|| simd_json::serde::to_vec(&value).unwrap()),
        ),
        (
            "MessagePack, rmp-serde",
            &mpk,
            time(|| rmp_serde::from_slice::<T>(&mpk).unwrap()),
            time(|| rmp_serde::to_vec_named(&value).unwrap()),
        ),
        (
            "CBOR, ciborium",
            &cbor,
            time(|| ciborium::from_reader::<T, _>(&cbor[..]).unwrap()),
            time(|| {
                let mut out = Vec::new();
                ciborium::into_writer(&value, &mut out).unwrap();
                out
            }),
        ),
        (
            "CBOR, minicbor-serde",
            &mcbor,
            time(|| minicbor_serde::from_slice::<T>(&mcbor).unwrap()),
            time(|| minicbor_serde::to_vec(&value).unwrap()),
        ),
    ];
    for (label, bytes, de, en) in rows {
        println!("| {label} | {} | {} | {} |", kb(bytes), fmt(de), fmt(en));
    }
    println!();
}

fn compression(name: &str, bytes: &[u8]) {
    println!("## Compressing {name} ({:.1} KB)", bytes.len() as f64 / 1024.0);
    println!("| Method | Compressed KB | Compress | Decompress |");
    println!("|---|---:|---:|---:|");
    let row = |label: &str, c: &dyn Fn() -> Vec<u8>, d: &dyn Fn(&[u8]) -> Vec<u8>| {
        let out = c();
        assert_eq!(d(&out), bytes);
        println!(
            "| {label} | {:.1} | {} | {} |",
            out.len() as f64 / 1024.0,
            fmt(time(c)),
            fmt(time(|| d(&out)))
        );
    };
    for level in [1, 6] {
        row(
            &format!("gzip {level} (flate2)"),
            &|| {
                let mut e = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::new(level));
                e.write_all(bytes).unwrap();
                e.finish().unwrap()
            },
            &|c| {
                let mut out = Vec::new();
                flate2::read::GzDecoder::new(c).read_to_end(&mut out).unwrap();
                out
            },
        );
    }
    for quality in [1, 4, 5] {
        row(
            &format!("brotli {quality}"),
            &|| {
                let mut out = Vec::new();
                let params = brotli::enc::BrotliEncoderParams { quality, ..Default::default() };
                brotli::BrotliCompress(&mut &bytes[..], &mut out, &params).unwrap();
                out
            },
            &|c| {
                let mut out = Vec::new();
                brotli::BrotliDecompress(&mut &c[..], &mut out).unwrap();
                out
            },
        );
    }
    #[cfg(not(target_arch = "wasm32"))]
    for level in [1, 3] {
        row(
            &format!("zstd {level}"),
            &|| zstd::encode_all(bytes, level).unwrap(),
            &|c| zstd::decode_all(c).unwrap(),
        );
    }
    println!();
}

fn main() {
    let read = |n: &str| std::fs::read(format!("data/{n}.json")).unwrap();
    formats::<Page>("list (1,000 records)", &read("list"));
    formats::<Record>("one record", &read("one"));
    formats::<Points>("numeric (5,000 points)", &read("numeric"));
    formats::<Docs>("text (200 docs, ~2 KB each)", &read("text"));

    let list: Page = serde_json::from_slice(&read("list")).unwrap();
    compression("list as JSON", &serde_json::to_vec(&list).unwrap());
    compression("list as MessagePack", &rmp_serde::to_vec_named(&list).unwrap());
    let points: Points = serde_json::from_slice(&read("numeric")).unwrap();
    compression("numeric as JSON", &serde_json::to_vec(&points).unwrap());
}
