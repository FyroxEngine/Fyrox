//! Example: 12. Reading MLA Archive
//!
//! Difficult: Medium
//!
//! This example is based on the examples on the MLA github README here: https://github.com/ANSSI-FR/MLA/blob/master/README.md
//! This tutorial goes through writing the archive and reading the archive and extracting the
//! files from the archive.

//! Writing an archive
//!
//! File location: root/build.rs

use curve25519_parser::parse_openssl_25519_pubkey;
use mla::config::ArchiveWriterConfig;
use mla::ArchiveWriter;

const PUB_KEY: &[u8] = include_bytes!("data/mla/test_x25519_pub.pem");

fn main() {
    // Load the needed public key
    let public_key = parse_openssl_25519_pubkey(PUB_KEY).unwrap();
    // Create an MLA Archive - Output only needs the Write trait
    let mut buf = Vec::new();
    // Default is Compression + Encryption, to avoid mistakes
    let mut config = ArchiveWriterConfig::default();
    // The use of multiple public keys is supported
    config.add_public_keys(&vec![public_key]);
    // Create the Writer
    let mut mla = ArchiveWriter::from_config(&mut buf, config).unwrap();
    // Add a file
    mla.add_file("data/mutant/idle.fbx", 4, &[0, 1, 2, 3][..]).unwrap();
    // Complete the archive
    mla.finalize().unwrap();

    // A file is tracked by an id, and follows this API's call order:
    // 1. id = start_file(filename);
    // 2. append_file_content(id, content length, content (impl Read))
    // 2-bis. repeat 2.
    // 3. end_file(id)

    // Start a file and add content
    let file1 = mla.start_file("data/mutant/mutant.FBX").unwrap();
    mla.append_file_content(file1, file1_part1.len() as u64,
    file1_part1.as_slice()).unwrap();
    // Start a second file and add content
    let file2 = mla.start_file("data/mutant/Mutant_normal.png").unwrap();
    mla.append_file_content(file2, file2_part1.len() as u64,
    file2_part1.as_slice()).unwrap();
    // Add a file as a whole
    mla.add_file("data/mutant/walk.fbx", file3.len() as u64, file3.as_slice()).unwrap();
    // Mark still opened files as finished
    mla.end_file(file1).unwrap();
    mla.end_file(file2).unwrap();
}

//! Extracting files from an archive
//!
//! File location: Anywhere but its recommended to keep it in root/src/main.rs to extract it all on
//! first run.

use curve25519_parser::parse_openssl_25519_privkey;
use mla::config::ArchiveReaderConfig;
use mla::ArchiveReader;
use std::io;

const PRIV_KEY: &[u8] = include_bytes!("data/mla/test_x25519_archive_v1.pem");
const DATA: &[u8] = include_bytes!("data/mla/archive_v1.mla");

fn main() {
    // Get the private key
    let private_key = parse_openssl_25519_privkey(PRIV_KEY).unwrap();

    // Specify the key for the Reader
    let mut config = ArchiveReaderConfig::new();
    config.add_private_keys(&[private_key]);
    
    // Read from buf, which needs Read + Seek
    let buf = io::Cursor::new(DATA);
    let mut mla_read = ArchiveReader::from_config(buf, config).unwrap();

    // Get a file
    let mut file = mla_read
        .get_file("simple".to_string())
        .unwrap() // An error can be raised (I/O, decryption, etc.)
        .unwrap(); // Option(file), as the file might not exist in the archive

    // Get back its filename, size, and data
    println!("{} ({} bytes)", file.filename, file.size);
    let mut output = Vec::new();
    std::io::copy(&mut file.data, &mut output).unwrap();
    
    // Get back the list of files in the archive:
    for fname in mla_read.list_files().unwrap() {
        println!("{}", fname);
    }
}
