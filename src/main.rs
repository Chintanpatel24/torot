use tracing_subscriber::EnvFilter;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("torot=warn".parse().unwrap()))
        .with_target(false)
        .init();

    match torot_lib::app::cli::run() {
        Ok(true) => {}
        Ok(false) => {
            if let Err(e) = torot_lib::app::tui::run() {
                eprintln!("torot tui error: {e}");
                std::process::exit(1);
            }
        }
        Err(err) => {
            eprintln!("torot error: {err}");
            std::process::exit(1);
        }
    }
}
