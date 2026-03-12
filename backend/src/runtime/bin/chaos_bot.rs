use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    chaos_bot_backend::runtime::cli::run(std::env::args_os()).await
}
