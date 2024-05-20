use sevenz_rust::{decompress_file, decompress_file_with_password, Password};
use std::env;
use std::path::Path;

fn main() {
    // Get command line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <path_to_7z_file> [password]", args[0]);
        return;
    }

    let file_path = &args[1];
    let password = if args.len() > 2 { Some(args[2].as_str()) } else { None };

    // Check if the file exists
    if !Path::new(file_path).exists() {
        eprintln!("Error: File '{}' not found.", file_path);
        return;
    }

    // Define the output directory
    let output_dir = "."; // Specify the desired output directory

    // Decompress the 7z file with or without a password
    let result = match password {
        Some(pass) => decompress_file_with_password(file_path, output_dir, Password::from(pass)),
        None => decompress_file(file_path, output_dir),
    };

    // Handle the result
    match result {
        Ok(_) => println!("Success!"),
        Err(e) => {
            match e {
                // sevenz_rust::Error::BadSignature(_) => todo!(),
                // sevenz_rust::Error::UnsupportedVersion { major, minor } => todo!(),
                // sevenz_rust::Error::ChecksumVerificationFailed => todo!(),
                // sevenz_rust::Error::NextHeaderCrcMismatch => todo!(),
                // sevenz_rust::Error::Io(_, _) => todo!(),
                // sevenz_rust::Error::FileOpen(_, _) => todo!(),
                // sevenz_rust::Error::Other(_) => todo!(),
                // sevenz_rust::Error::BadTerminatedStreamsInfo(_) => todo!(),
                // sevenz_rust::Error::BadTerminatedUnpackInfo => todo!(),
                // sevenz_rust::Error::BadTerminatedPackInfo(_) => todo!(),
                // sevenz_rust::Error::BadTerminatedSubStreamsInfo => todo!(),
                // sevenz_rust::Error::BadTerminatedheader(_) => todo!(),
                // sevenz_rust::Error::ExternalUnsupported => todo!(),
                // sevenz_rust::Error::UnsupportedCompressionMethod(_) => todo!(),
                // sevenz_rust::Error::MaxMemLimited { max_kb, actaul_kb } => todo!(),
                sevenz_rust::Error::PasswordRequired => eprintln!("Password Required"),
                // sevenz_rust::Error::Unsupported(_) => todo!(),
                sevenz_rust::Error::MaybeBadPassword(_) => eprintln!("Wrong Password"),
                _ => eprintln!("Extraction failed with error: {}", e),
            }
        },
    }
}
