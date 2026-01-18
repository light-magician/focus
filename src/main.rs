use clap::{Parser, Subcommand};
use std::fs;
use std::io::{self, BufRead};
use std::path::PathBuf;
use std::process::Command;

const HOSTS_FILE: &str = "/etc/hosts";
const BLOCK_MARKER_START: &str = "# FOCUS-MODE-BLOCK START";
const BLOCK_MARKER_END: &str = "# FOCUS-MODE-BLOCK END";

#[derive(Parser)]
#[command(name = "focus")]
#[command(about = "Block distracting websites while you work", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Enable focus mode - block all configured domains
    On,
    /// Disable focus mode - unblock all domains
    Off,
    /// Edit the list of blocked domains
    Edit,
    /// Show current status and blocked domains
    Status,
}

fn get_focus_dir() -> PathBuf {
    let home = dirs::home_dir().expect("Could not find home directory");
    home.join(".focus")
}

fn get_domains_file() -> PathBuf {
    get_focus_dir().join("domains.txt")
}

fn ensure_focus_dir() -> io::Result<()> {
    let focus_dir = get_focus_dir();
    if !focus_dir.exists() {
        fs::create_dir_all(&focus_dir)?;
    }

    let domains_file = get_domains_file();
    if !domains_file.exists() {
        // Create default domains file
        let default_domains = "# Add one domain per line\n\
                              # Lines starting with # are comments\n\
                              # Example:\n\
                              # instagram.com\n\
                              # twitter.com\n\
                              instagram.com\n\
                              www.instagram.com\n\
                              x.com\n\
                              www.x.com\n\
                              twitter.com\n\
                              www.twitter.com\n";
        fs::write(&domains_file, default_domains)?;
    }
    Ok(())
}

fn read_domains() -> io::Result<Vec<String>> {
    let domains_file = get_domains_file();
    let file = fs::File::open(&domains_file)?;
    let reader = io::BufReader::new(file);

    let domains: Vec<String> = reader
        .lines()
        .filter_map(|line| line.ok())
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();

    Ok(domains)
}

fn is_focus_active() -> bool {
    if let Ok(content) = fs::read_to_string(HOSTS_FILE) {
        content.contains(BLOCK_MARKER_START)
    } else {
        false
    }
}

fn focus_on() -> io::Result<()> {
    if is_focus_active() {
        println!("Focus mode is already active.");
        return Ok(());
    }

    let domains = read_domains()?;
    if domains.is_empty() {
        println!("No domains configured. Run 'focus edit' to add domains.");
        return Ok(());
    }

    // Build the block to add
    let mut block = String::new();
    block.push('\n');
    block.push_str(BLOCK_MARKER_START);
    block.push('\n');
    for domain in &domains {
        block.push_str(&format!("127.0.0.1 {}\n", domain));
    }
    block.push_str(BLOCK_MARKER_END);
    block.push('\n');

    // Read current hosts file and append
    let mut hosts_content = fs::read_to_string(HOSTS_FILE)?;
    hosts_content.push_str(&block);

    // Write back (requires sudo)
    fs::write(HOSTS_FILE, hosts_content)?;

    // Flush DNS cache
    flush_dns_cache();

    println!("Focus mode activated. Blocked {} domains:", domains.len());
    for domain in &domains {
        println!("  - {}", domain);
    }

    Ok(())
}

fn focus_off() -> io::Result<()> {
    if !is_focus_active() {
        println!("Focus mode is not active.");
        return Ok(());
    }

    let hosts_content = fs::read_to_string(HOSTS_FILE)?;

    // Remove the focus block
    let mut new_content = String::new();
    let mut in_block = false;

    for line in hosts_content.lines() {
        if line.contains(BLOCK_MARKER_START) {
            in_block = true;
            continue;
        }
        if line.contains(BLOCK_MARKER_END) {
            in_block = false;
            continue;
        }
        if !in_block {
            new_content.push_str(line);
            new_content.push('\n');
        }
    }

    // Remove trailing newlines that we might have added
    let new_content = new_content.trim_end().to_string() + "\n";

    fs::write(HOSTS_FILE, new_content)?;

    // Flush DNS cache
    flush_dns_cache();

    println!("Focus mode deactivated. All sites unblocked.");

    Ok(())
}

fn focus_edit() -> io::Result<()> {
    let domains_file = get_domains_file();

    // Get editor from EDITOR env var, fall back to vim
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());

    let status = Command::new(&editor)
        .arg(&domains_file)
        .status()?;

    if status.success() {
        println!("Domains file saved. Changes will apply next time you run 'focus on'.");
        if is_focus_active() {
            println!("Tip: Run 'focus off && focus on' to apply changes immediately.");
        }
    }

    Ok(())
}

fn focus_status() -> io::Result<()> {
    if is_focus_active() {
        println!("Focus mode: ACTIVE");
    } else {
        println!("Focus mode: INACTIVE");
    }

    println!("\nConfigured domains ({}):", get_domains_file().display());
    let domains = read_domains()?;
    if domains.is_empty() {
        println!("  (none configured)");
    } else {
        for domain in &domains {
            println!("  - {}", domain);
        }
    }

    Ok(())
}

fn flush_dns_cache() {
    // macOS DNS cache flush
    let _ = Command::new("dscacheutil").arg("-flushcache").status();
    let _ = Command::new("killall")
        .args(["-HUP", "mDNSResponder"])
        .status();
}

fn main() {
    let cli = Cli::parse();

    // Ensure .focus directory and default domains file exist
    if let Err(e) = ensure_focus_dir() {
        eprintln!("Error creating focus directory: {}", e);
        std::process::exit(1);
    }

    let result = match cli.command {
        Commands::On => focus_on(),
        Commands::Off => focus_off(),
        Commands::Edit => focus_edit(),
        Commands::Status => focus_status(),
    };

    if let Err(e) = result {
        if e.kind() == io::ErrorKind::PermissionDenied {
            eprintln!("Permission denied. Try running with sudo:");
            eprintln!("  sudo focus on");
            eprintln!("  sudo focus off");
        } else {
            eprintln!("Error: {}", e);
        }
        std::process::exit(1);
    }
}
