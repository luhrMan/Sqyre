//! Migrate a legacy `db.yaml` to the current Sqyre schema.
//!
//! Usage: `cargo run -p sqyre-persist --bin migrate_db_yaml -- input.yaml [output.yaml]`

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(input) = args.next() else {
        eprintln!("usage: migrate_db_yaml <input.yaml> [output.yaml]");
        return ExitCode::from(2);
    };
    let output = args.next().map(PathBuf::from);

    let text = match fs::read_to_string(&input) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{input}: {e}");
            return ExitCode::from(1);
        }
    };

    match sqyre_persist::migrate_db_yaml(&text) {
        Ok(migrated) => {
            if let Some(path) = output {
                if let Err(e) = fs::write(&path, &migrated) {
                    eprintln!("{}: {e}", path.display());
                    return ExitCode::from(1);
                }
                eprintln!("wrote {}", path.display());
            } else {
                print!("{migrated}");
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("migration failed: {e}");
            ExitCode::from(1)
        }
    }
}
