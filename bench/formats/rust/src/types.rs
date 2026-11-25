use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct Page {
    pub items: Vec<Record>,
    pub pagination: Pagination,
}
#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct Pagination {
    pub next_cursor: String,
}
#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct Record {
    pub id: String,
    pub sku: String,
    pub name: String,
    pub description: Option<String>,
    pub price: Price,
    pub quantity: u32,
    pub rating: f64,
    pub tags: Vec<String>,
    pub status: Status,
    pub supplier: Supplier,
    pub dimensions: Dimensions,
    pub created_at: String,
    pub updated_at: Option<String>,
}
#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct Price {
    pub amount: String,
    pub currency: String,
}
#[derive(Serialize, Deserialize, PartialEq, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Status {
    Active,
    Discontinued { since: String },
    Backordered { eta_days: u32 },
}
#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct Supplier {
    pub id: String,
    pub name: String,
    pub country: String,
}
#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct Dimensions {
    pub width_mm: u32,
    pub height_mm: u32,
    pub depth_mm: u32,
    pub weight_g: u32,
}
#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct Points {
    pub items: Vec<Point>,
}
#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct Point {
    pub t: u64,
    pub count: u64,
    pub values: Vec<f64>,
}
#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct Docs {
    pub items: Vec<Doc>,
}
#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct Doc {
    pub id: String,
    pub title: String,
    pub body: String,
    pub created_at: String,
}

