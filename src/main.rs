use rayon::prelude::*;
use sevenz_rust::{decompress_file_with_password, Password};
use std::env;
use std::fs::{self, File, Metadata};
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use progress_bar::ProgressBar;

fn create_temp_base_dir(base: &str) -> io::Result<PathBuf> {
    let temp_dir = PathBuf::from(base).join("temp_decompression");

    if !temp_dir.exists() {
        fs::create_dir(&temp_dir)?;
    }
    Ok(temp_dir)
}

fn create_unique_dir(base: &Path) -> io::Result<PathBuf> {
    let mut counter = 0;
    loop {
        let unique_dir = base.join(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_nanos()
                .to_string(),
        );
        let unique_dir = if counter > 0 {
            unique_dir.with_extension(counter.to_string())
        } else {
            unique_dir
        };
        match fs::create_dir(&unique_dir) {
            Ok(_) => return Ok(unique_dir),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                counter += 1;
                continue;
            }
            Err(e) => return Err(e),
        }
    }
}

fn verify_decompression(dir: &Path) -> bool {
    fs::read_dir(dir)
        .map(|entries| entries.filter_map(|entry| entry.ok()).any(|entry| {
            entry.metadata().map(|metadata| metadata.len() > 0).unwrap_or(false)
        }))
        .unwrap_or(false)
}

fn main() -> io::Result<()> {
    // Get command line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <path_to_7z_file> <path_to_password_file>", args[0]);
        return Ok(());
    }

    let file_path = &args[1];
    let password_file_path = &args[2];

    // Check if the file exists
    if !Path::new(file_path).exists() {
        eprintln!("Error: File '{}' not found.", file_path);
        return Ok(());
    }

    // Define the output directory as the current directory
    let output_dir = ".";
    let temp_dir_base = ".";

    // Create the base temporary directory
    let temp_base_dir = create_temp_base_dir(temp_dir_base)?;

    // Open the password file and count the total lines (passwords)
    let file = File::open(password_file_path)?;
    let reader = BufReader::new(file);
    let total_passwords = reader.lines().count();

    // Re-open the password file for reading
    let file = File::open(password_file_path)?;
    let reader = BufReader::new(file);

    // Create and start the progress bar
    let mut progress_bar = ProgressBar::new(total_passwords)?;
    progress_bar.update(0)?; // Initial display

    // Create an atomic counter to track progress
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = Arc::clone(&counter);

    // Create an atomic flag to indicate success
    let success_flag = Arc::new(AtomicBool::new(false));
    let success_flag_clone = Arc::clone(&success_flag);

    // Mutex to store the successful password
    let password_found = Arc::new(Mutex::new(None));

    // Path to the successfully decompressed directory
    let successful_temp_dir = Arc::new(Mutex::new(None));

    // Start a new thread for updating the progress bar
    let handle = thread::spawn(move || {
        loop {
            let processed = counter_clone.load(Ordering::Relaxed);
            if let Err(e) = progress_bar.update(processed) {
                eprintln!("Error updating progress bar: {}", e);
                return;
            }

            if processed >= total_passwords || success_flag_clone.load(Ordering::Relaxed) {
                if let Err(e) = progress_bar.finish() {
                    eprintln!("Error finishing progress bar: {}", e);
                }
                break;
            }
            thread::sleep(Duration::from_millis(100)); // Increase update frequency
        }
    });

    // Create a parallel iterator over lines
    reader.lines().par_bridge().for_each(|line| {
        if success_flag.load(Ordering::Relaxed) {
            return;
        }

        let password = match line {
            Ok(pass) => Password::from(pass.as_str()),
            Err(_) => return,
        };

        let unique_temp_dir = create_unique_dir(&temp_base_dir).expect("Failed to create unique temp directory");

        if decompress_file_with_password(file_path, unique_temp_dir.to_str().unwrap(), password.clone()).is_ok() {
            if verify_decompression(&unique_temp_dir) {
                *password_found.lock().unwrap() = Some(password.clone());
                *successful_temp_dir.lock().unwrap() = Some(unique_temp_dir.clone());
                success_flag.store(true, Ordering::Relaxed);
            } else {
                let _ = fs::remove_dir_all(&unique_temp_dir);
            }
        } else {
            counter.fetch_add(1, Ordering::Relaxed);
            let _ = fs::remove_dir_all(&unique_temp_dir);
        }
    });

    // Wait for the progress bar thread to finish
    handle.join().unwrap();

    // Handle the result
    if !success_flag.load(Ordering::Relaxed) {
        eprintln!("Failed to decompress the file with any of the provided passwords.");
    } else {
        // Move the successfully decompressed files to the output directory
        if let Some(temp_dir) = successful_temp_dir.lock().unwrap().take() {
            // Attempt to remove existing files in the output directory
            if let Err(e) = fs::remove_dir_all(output_dir) {
                if e.kind() != std::io::ErrorKind::NotFound {
                    eprintln!("Error removing output directory: {}", e);
                }
            }
            // Attempt to move the successfully decompressed files
            if let Err(e) = fs::rename(&temp_dir, output_dir) {
                eprintln!("Error moving files to output directory: {}", e);
            }
        }

        // Print the successful password
        if let Some(password) = &*password_found.lock().unwrap() {
            println!("Successfully decompressed the file with the correct password: {:?}", password);
        }
    }

    Ok(())
}
