#[derive(
    Debug, Default, Clone, serde::Deserialize, serde::Serialize, reflect::Input, reflect::Output,
)]
pub struct Empty {}

impl Empty {
    pub fn new() -> Self {
        Self {}
    }
}

impl From<()> for Empty {
    fn from(_: ()) -> Self {
        Self::new()
    }
}
