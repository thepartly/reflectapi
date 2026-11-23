//! Reading a few fields without decoding the whole value: JSON and MessagePack.
//!   cargo run --release --bin peek
use std::hint::black_box;
use std::time::Instant;

use rmp::Marker;
use serde::Deserialize;

#[path = "../types.rs"]
mod types;
use types::*;

fn time<R>(mut f: impl FnMut() -> R) -> f64 {
    let t0 = Instant::now();
    let mut n = 0u64;
    while t0.elapsed().as_millis() < 300 {
        black_box(f());
        n += 1;
    }
    let per = (n / 3).max(1);
    let mut s: Vec<f64> = (0..15)
        .map(|_| {
            let t = Instant::now();
            for _ in 0..per {
                black_box(f());
            }
            t.elapsed().as_secs_f64() * 1e3 / per as f64
        })
        .collect();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap());
    s[7]
}

fn fmt(ms: f64) -> String {
    if ms < 0.1 {
        format!("{:.2} µs", ms * 1e3)
    } else {
        format!("{ms:.3} ms")
    }
}

// Views that serde fills, skipping every other field.
#[derive(Deserialize)]
struct CursorView<'a> {
    #[serde(borrow)]
    pagination: CursorInner<'a>,
}
#[derive(Deserialize)]
struct CursorInner<'a> {
    #[serde(borrow)]
    next_cursor: &'a str,
}
#[derive(Deserialize)]
struct IdsView<'a> {
    #[serde(borrow)]
    items: Vec<IdOnly<'a>>,
}
#[derive(Deserialize)]
struct IdOnly<'a> {
    #[serde(borrow)]
    id: &'a str,
}
#[derive(Deserialize)]
struct RecordView<'a> {
    #[serde(borrow)]
    id: &'a str,
    #[serde(borrow)]
    supplier: SupplierView<'a>,
}
#[derive(Deserialize)]
struct SupplierView<'a> {
    #[serde(borrow)]
    country: &'a str,
}

// MessagePack: skip a value by its lengths, without decoding it.
fn skip(buf: &mut &[u8]) {
    let marker = Marker::from_u8(buf[0]);
    *buf = &buf[1..];
    let take = |buf: &mut &[u8], n: usize| *buf = &buf[n..];
    let be = |buf: &[u8], n: usize| buf[..n].iter().fold(0usize, |a, b| (a << 8) | *b as usize);
    match marker {
        Marker::FixPos(_) | Marker::FixNeg(_) | Marker::Null | Marker::True | Marker::False => {}
        Marker::U8 | Marker::I8 => take(buf, 1),
        Marker::U16 | Marker::I16 => take(buf, 2),
        Marker::U32 | Marker::I32 | Marker::F32 => take(buf, 4),
        Marker::U64 | Marker::I64 | Marker::F64 => take(buf, 8),
        Marker::FixStr(n) => take(buf, n as usize),
        Marker::Str8 | Marker::Bin8 => { let n = be(buf, 1); take(buf, 1 + n) }
        Marker::Str16 | Marker::Bin16 => { let n = be(buf, 2); take(buf, 2 + n) }
        Marker::Str32 | Marker::Bin32 => { let n = be(buf, 4); take(buf, 4 + n) }
        Marker::FixArray(n) => (0..n).for_each(|_| skip(buf)),
        Marker::Array16 => { let n = be(buf, 2); take(buf, 2); (0..n).for_each(|_| skip(buf)) }
        Marker::Array32 => { let n = be(buf, 4); take(buf, 4); (0..n).for_each(|_| skip(buf)) }
        Marker::FixMap(n) => (0..2 * n).for_each(|_| skip(buf)),
        Marker::Map16 => { let n = be(buf, 2); take(buf, 2); (0..2 * n).for_each(|_| skip(buf)) }
        Marker::Map32 => { let n = be(buf, 4); take(buf, 4); (0..2 * n).for_each(|_| skip(buf)) }
        m => unimplemented!("{m:?}"),
    }
}

fn str_at<'a>(buf: &mut &'a [u8]) -> &'a str {
    let len = rmp::decode::read_str_len(buf).unwrap() as usize;
    let (s, rest) = buf.split_at(len);
    *buf = rest;
    std::str::from_utf8(s).unwrap()
}

/// The value under `key` in the map at the start of `buf`.
fn get<'a>(mut buf: &'a [u8], key: &str) -> &'a [u8] {
    let n = rmp::decode::read_map_len(&mut buf).unwrap();
    for _ in 0..n {
        if str_at(&mut buf) == key {
            return buf;
        }
        skip(&mut buf);
    }
    panic!("no {key}")
}

/// The element at `index` of the array at the start of `buf`.
fn index(mut buf: &[u8], index: usize) -> &[u8] {
    let n = rmp::decode::read_array_len(&mut buf).unwrap() as usize;
    assert!(index < n);
    for _ in 0..index {
        skip(&mut buf);
    }
    buf
}

fn main() {
    let read = |n: &str| std::fs::read(format!("data/{n}.json")).unwrap();
    let page: Page = serde_json::from_slice(&read("list")).unwrap();
    let record: Record = serde_json::from_slice(&read("one")).unwrap();
    let (pj, pm) = (serde_json::to_vec(&page).unwrap(), rmp_serde::to_vec_named(&page).unwrap());
    let (rj, rm) = (serde_json::to_vec(&record).unwrap(), rmp_serde::to_vec_named(&record).unwrap());

    let cursor = page.pagination.next_cursor.as_str();
    let mid = page.items[500].id.as_str();

    // Every method reads the same values.
    assert_eq!(sonic_rs::get_from_slice(&pj, &["pagination", "next_cursor"]).unwrap().as_raw_str(), format!("\"{cursor}\""));
    assert_eq!(str_at(&mut get(get(&pm, "pagination"), "next_cursor")), cursor);
    assert_eq!(str_at(&mut get(index(get(&pm, "items"), 500), "id")), mid);
    assert_eq!(rmp_serde::from_slice::<CursorView>(&pm).unwrap().pagination.next_cursor, cursor);

    println!("## 1,000-record list: `pagination.next_cursor`, after all the items ({:.0} KB JSON, {:.0} KB MessagePack)\n", pj.len() as f64 / 1024.0, pm.len() as f64 / 1024.0);
    println!("| Method | Time |\n|---|---:|");
    println!("| JSON, full typed decode (serde_json) | {} |", fmt(time(|| serde_json::from_slice::<Page>(&pj).unwrap())));
    println!("| JSON, serde view (serde_json, what `Raw::peek` does) | {} |", fmt(time(|| serde_json::from_slice::<CursorView>(&pj).unwrap().pagination.next_cursor.len())));
    println!("| JSON, serde view (sonic-rs) | {} |", fmt(time(|| sonic_rs::from_slice::<CursorView>(&pj).unwrap().pagination.next_cursor.len())));
    println!("| JSON, sonic-rs `get_from_slice` (validates) | {} |", fmt(time(|| sonic_rs::get_from_slice(&pj, &["pagination", "next_cursor"]).unwrap().as_raw_str().len())));
    println!("| JSON, sonic-rs `get_from_slice_unchecked` | {} |", fmt(time(|| unsafe { sonic_rs::get_from_slice_unchecked(&pj, &["pagination", "next_cursor"]) }.unwrap().as_raw_str().len())));
    println!("| MessagePack, full typed decode (rmp-serde) | {} |", fmt(time(|| rmp_serde::from_slice::<Page>(&pm).unwrap())));
    println!("| MessagePack, serde view (rmp-serde) | {} |", fmt(time(|| rmp_serde::from_slice::<CursorView>(&pm).unwrap().pagination.next_cursor.len())));
    println!("| MessagePack, skip by lengths | {} |", fmt(time(|| str_at(&mut get(get(&pm, "pagination"), "next_cursor")).len())));

    println!("\n## 1,000-record list: `items[500].id`\n");
    println!("| Method | Time |\n|---|---:|");
    println!("| JSON, serde view of every item's id (serde_json) | {} |", fmt(time(|| serde_json::from_slice::<IdsView>(&pj).unwrap().items[500].id.len())));
    println!("| JSON, sonic-rs `get_from_slice` | {} |", fmt(time(|| sonic_rs::get_from_slice(&pj, sonic_rs::pointer!["items", 500, "id"]).unwrap().as_raw_str().len())));
    println!("| MessagePack, serde view of every item's id (rmp-serde) | {} |", fmt(time(|| rmp_serde::from_slice::<IdsView>(&pm).unwrap().items[500].id.len())));
    println!("| MessagePack, skip by lengths | {} |", fmt(time(|| str_at(&mut get(index(get(&pm, "items"), 500), "id")).len())));

    println!("\n## One record: `id` and `supplier.country` ({} B JSON, {} B MessagePack)\n", rj.len(), rm.len());
    println!("| Method | Time |\n|---|---:|");
    println!("| JSON, full typed decode (serde_json) | {} |", fmt(time(|| serde_json::from_slice::<Record>(&rj).unwrap())));
    println!("| JSON, serde view (serde_json) | {} |", fmt(time(|| serde_json::from_slice::<RecordView>(&rj).unwrap().supplier.country.len())));
    println!("| JSON, sonic-rs `get_from_slice`, twice | {} |", fmt(time(|| sonic_rs::get_from_slice(&rj, &["id"]).unwrap().as_raw_str().len() + sonic_rs::get_from_slice(&rj, &["supplier", "country"]).unwrap().as_raw_str().len())));
    println!("| MessagePack, full typed decode (rmp-serde) | {} |", fmt(time(|| rmp_serde::from_slice::<Record>(&rm).unwrap())));
    println!("| MessagePack, serde view (rmp-serde) | {} |", fmt(time(|| rmp_serde::from_slice::<RecordView>(&rm).unwrap().supplier.country.len())));
    println!("| MessagePack, skip by lengths | {} |", fmt(time(|| str_at(&mut get(&rm, "id")).len() + str_at(&mut get(get(&rm, "supplier"), "country")).len())));
}
