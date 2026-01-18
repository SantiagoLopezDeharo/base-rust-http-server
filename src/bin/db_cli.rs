use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use base_rust_web_api::db;

fn main() -> io::Result<()> {
    dotenv::dotenv().ok();

    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        print_usage();
        std::process::exit(1);
    }

    let command = args.remove(0);
    match command.as_str() {
        "migration:new" => create_sql_file("migrations"),
        "seed:new" => create_sql_file("seeders"),
        "migrate" => run_pending("migrations"),
        "seed" => run_pending("seeders"),
        "migrate:undo" => undo_last("migrations"),
        "seed:undo" => undo_last("seeders"),
        _ => {
            print_usage();
            Ok(())
        }
    }
}

fn print_usage() {
    eprintln!(
        "Usage:\n  \
  cargo run --bin db_cli -- migration:new\n  \
  cargo run --bin db_cli -- seed:new\n  \
  cargo run --bin db_cli -- migrate\n  \
  cargo run --bin db_cli -- seed\n  \
  cargo run --bin db_cli -- migrate:undo\n  \
  cargo run --bin db_cli -- seed:undo\n"
    );
}

fn prompt_name() -> io::Result<String> {
    print!("Enter name: ");
    io::stdout().flush()?;
    let mut name = String::new();
    io::stdin().read_line(&mut name)?;
    Ok(name.trim().replace(' ', "_"))
}

fn timestamp_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn create_sql_file(kind: &str) -> io::Result<()> {
    let name = prompt_name()?;
    if name.is_empty() {
        eprintln!("Name cannot be empty.");
        return Ok(());
    }

    let ts = timestamp_ms();
    let base = format!("{}_{}", ts, name);
    let dir = PathBuf::from("src/db").join(kind);
    fs::create_dir_all(&dir)?;

    let up_file = dir.join(format!("{}_up.sql", base));
    let down_file = dir.join(format!("{}_down.sql", base));

    write_file_if_missing(&up_file, "-- write your SQL here\n")?;
    write_file_if_missing(&down_file, "-- write your SQL here\n")?;

    println!(
        "Created:\n  {}\n  {}",
        up_file.display(),
        down_file.display()
    );
    Ok(())
}

fn write_file_if_missing(path: &Path, content: &str) -> io::Result<()> {
    if path.exists() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = fs::File::create(path)?;
    file.write_all(content.as_bytes())
}

fn read_sql(path: &Path) -> io::Result<String> {
    let mut file = fs::File::open(path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    Ok(content)
}

fn list_sql_files(kind: &str, suffix: &str) -> io::Result<Vec<PathBuf>> {
    let dir = PathBuf::from("src/db").join(kind);
    let mut files = Vec::new();
    if dir.exists() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.ends_with(suffix) {
                    files.push(path);
                }
            }
        }
    }
    files.sort();
    Ok(files)
}

fn parse_id_name_from_file(path: &Path) -> Option<(String, String)> {
    let filename = path.file_name()?.to_string_lossy();
    let parts: Vec<&str> = filename.split('_').collect();
    if parts.len() < 2 {
        return None;
    }
    let id = parts[0].to_string();
    let name = parts[1..]
        .join("_")
        .replace("_up.sql", "")
        .replace("_down.sql", "");
    Some((id, name))
}

fn run_pending(kind: &str) -> io::Result<()> {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async move {
        db::init_pool().await.map_err(to_io_err)?;
        db::ensure_migrations_tables().await.map_err(to_io_err)?;

        let applied = if kind == "migrations" {
            db::applied_migration_ids().await.map_err(to_io_err)?
        } else {
            db::applied_seed_ids().await.map_err(to_io_err)?
        };

        let files = list_sql_files(kind, "_up.sql")?;
        for file in files {
            let (id, name) = match parse_id_name_from_file(&file) {
                Some(v) => v,
                None => continue,
            };
            if applied.contains(&id) {
                continue;
            }
            let sql = read_sql(&file)?;
            db::execute_sql(&sql).await.map_err(to_io_err)?;
            if kind == "migrations" {
                db::mark_migration_applied(&id, &name)
                    .await
                    .map_err(to_io_err)?;
            } else {
                db::mark_seed_applied(&id, &name).await.map_err(to_io_err)?;
            }
            println!("Applied {}: {}", kind.trim_end_matches('s'), file.display());
        }
        Ok(())
    })
}

fn undo_last(kind: &str) -> io::Result<()> {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async move {
        db::init_pool().await.map_err(to_io_err)?;
        db::ensure_migrations_tables().await.map_err(to_io_err)?;

        let applied = if kind == "migrations" {
            db::applied_migration_ids().await.map_err(to_io_err)?
        } else {
            db::applied_seed_ids().await.map_err(to_io_err)?
        };

        let mut files = list_sql_files(kind, "_down.sql")?;
        files.sort();
        files.reverse();

        for file in files {
            let (id, _) = match parse_id_name_from_file(&file) {
                Some(v) => v,
                None => continue,
            };
            if !applied.contains(&id) {
                continue;
            }
            let sql = read_sql(&file)?;
            db::execute_sql(&sql).await.map_err(to_io_err)?;
            if kind == "migrations" {
                db::unmark_migration_applied(&id).await.map_err(to_io_err)?;
            } else {
                db::unmark_seed_applied(&id).await.map_err(to_io_err)?;
            }
            println!(
                "Reverted {}: {}",
                kind.trim_end_matches('s'),
                file.display()
            );
            break;
        }
        Ok(())
    })
}

fn to_io_err(err: sqlx::Error) -> io::Error {
    io::Error::new(io::ErrorKind::Other, err.to_string())
}
