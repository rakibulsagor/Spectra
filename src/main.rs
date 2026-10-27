use directories::ProjectDirs;
use spectra::app::App;
use std::path::PathBuf;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,spectra=debug")),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting Spectra — Native Privacy-Focused Telegram Client v0.1.0");

    let args: Vec<String> = std::env::args().collect();
    let mut bot_token = std::env::var("SPECTRA_BOT_TOKEN").ok();
    let mut session_path = std::env::var("SPECTRA_SESSION_PATH").ok();
    let mut db_path_opt = std::env::var("SPECTRA_DB").map(PathBuf::from).ok();
    let mut api_id: i32 = std::env::var("SPECTRA_API_ID")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(2040);
    let mut api_hash: String = std::env::var("SPECTRA_API_HASH")
        .unwrap_or_else(|_| "b18441a1ff607e10a989891a5462e627".to_string());

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--token" if i + 1 < args.len() => {
                bot_token = Some(args[i + 1].clone());
                i += 2;
            }
            "--session" if i + 1 < args.len() => {
                session_path = Some(args[i + 1].clone());
                i += 2;
            }
            "--db" if i + 1 < args.len() => {
                db_path_opt = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--api-id" if i + 1 < args.len() => {
                if let Ok(val) = args[i + 1].parse() {
                    api_id = val;
                }
                i += 2;
            }
            "--api-hash" if i + 1 < args.len() => {
                api_hash = args[i + 1].clone();
                i += 2;
            }
            "--help" | "-h" => {
                println!(
                    "Spectra — Native Privacy-Focused Telegram Client\n\n\
                     Usage: spectra [OPTIONS]\n\n\
                     Options:\n\
                       --token <TOKEN>         Telegram bot token for MTProto connection\n\
                       --session <PATH>        Path to session sqlite database\n\
                       --db <PATH>             Path to message history sqlite database\n\
                       --api-id <ID>           Custom Telegram API ID\n\
                       --api-hash <HASH>       Custom Telegram API Hash\n\
                       -h, --help              Print this help menu"
                );
                return Ok(());
            }
            _ => {
                i += 1;
            }
        }
    }

    let db_path = if let Some(p) = db_path_opt {
        p
    } else if let Some(proj_dirs) = ProjectDirs::from("org", "spectra", "Spectra") {
        let dir = proj_dirs.data_dir();
        std::fs::create_dir_all(dir)?;
        dir.join("spectra.db")
    } else {
        PathBuf::from("spectra.db")
    };

    info!("Using Spectra local database at {:?}", db_path);

    let app = App::new(db_path, api_id, api_hash)?;
    let exit_code = app.run(bot_token, session_path);

    std::process::exit(exit_code.value());
}
