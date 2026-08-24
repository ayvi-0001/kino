pub struct Data {}

// type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Error = anyhow::Error;

pub type Context<'a> = poise::Context<'a, Data, Error>;

impl Data {
    pub fn new() -> Self {
        Data {}
    }
}
