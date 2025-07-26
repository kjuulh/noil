mod commit;
mod encode_rand;
mod find_prefix;
mod format;
mod models;
mod output;
mod parse;

mod cli;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    cli::execute().await?;

    Ok(())
}
