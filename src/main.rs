use clap::Parser;
mod app;

fn main() -> anyhow::Result<()> {
    app::run(rex_linux::connection::Options::parse())
}
