use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use clap::{Arg, Command};
use colored::*;
use crossterm::{
    terminal::{Clear, ClearType},
    ExecutableCommand,
};
use figlet_rs::FIGfont;
use rustyline::Editor;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::{Command as StdCommand, Stdio};
use std::thread;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;
use url::Url;

// Configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ModemConfig {
    baud_rate: u32,
    connection_type: String,
    sound_enabled: bool,
    log_level: String,
}

impl Default for ModemConfig {
    fn default() -> Self {
        Self {
            baud_rate: 1200,
            connection_type: "hayes".to_string(),
            sound_enabled: true,
            log_level: "info".to_string(),
        }
    }
}

// Connection log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ConnectionLog {
    timestamp: DateTime<Utc>,
    connection_type: String,
    target: String,
    status: String,
    duration_ms: u64,
}

// Main VModem structure
struct VModem {
    config: ModemConfig,
    config_path: PathBuf,
    log_path: PathBuf,
    connection_history: Vec<ConnectionLog>,
}

impl VModem {
    fn new() -> Result<Self> {
        let config_dir = dirs::home_dir()
            .ok_or_else(|| anyhow!("Could not find home directory"))?;
        
        let config_path = config_dir.join(".vmodem99a.json");
        let log_path = config_dir.join(".vmodem99a.log");
        
        let config = if config_path.exists() {
            let config_str = fs::read_to_string(&config_path)?;
            serde_json::from_str(&config_str).unwrap_or_default()
        } else {
            ModemConfig::default()
        };
        
        let connection_history = if log_path.exists() {
            let log_str = fs::read_to_string(&log_path)?;
            serde_json::from_str(&log_str).unwrap_or_default()
        } else {
            Vec::new()
        };
        
        Ok(Self {
            config,
            config_path,
            log_path,
            connection_history,
        })
    }
    
    fn save_config(&self) -> Result<()> {
        let config_str = serde_json::to_string_pretty(&self.config)?;
        fs::write(&self.config_path, config_str)?;
        Ok(())
    }
    
    fn save_log(&self) -> Result<()> {
        let log_str = serde_json::to_string_pretty(&self.connection_history)?;
        fs::write(&self.log_path, log_str)?;
        Ok(())
    }
    
    fn log_connection(&mut self, conn_type: &str, target: &str, status: &str, duration: Duration) {
        let entry = ConnectionLog {
            timestamp: Utc::now(),
            connection_type: conn_type.to_string(),
            target: target.to_string(),
            status: status.to_string(),
            duration_ms: duration.as_millis() as u64,
        };
        
        self.connection_history.push(entry);
        
        // Keep only last 100 entries
        if self.connection_history.len() > 100 {
            self.connection_history.remove(0);
        }
        
        let _ = self.save_log();
    }
    
    fn show_banner(&self) {
        let _ = io::stdout().execute(Clear(ClearType::All));
        
        // Try to use figlet, fallback to simple text
        if let Ok(font) = FIGfont::standard() {
            if let Some(figure) = font.convert("VModem 99/A") {
                println!("{}", figure.to_string().cyan().bold());
            } else {
                println!("{}", "VModem Model 99/A".cyan().bold());
            }
        } else {
            println!("{}", "VModem Model 99/A".cyan().bold());
        }
        
        println!("{}", "═".repeat(60).dimmed());
        println!("{}", "Virtual Modem Terminal v1.0 - Hayes Compatible".magenta());
        println!("{} {} | {} {}", 
            "Baud Rate:".dimmed(),
            self.config.baud_rate.to_string().yellow(),
            "Protocol:".dimmed(),
            self.config.connection_type.yellow()
        );
        println!("{}", "═".repeat(60).dimmed());
        println!();
    }
    
    fn show_status(&self, message: &str) {
        println!("{} {}", "[STATUS]".blue().bold(), message);
    }
    
    fn show_error(&self, message: &str) {
        println!("{} {}", "[ERROR]".red().bold(), message);
    }
    
    fn show_success(&self, message: &str) {
        println!("{} {}", "[OK]".green().bold(), message);
    }
    
    // Sound effects using system commands
    fn play_dial_tone(&self) {
        if !self.config.sound_enabled {
            return;
        }
        
        println!("{}", "♪ Dialing...".cyan());
        thread::spawn(|| {
            let _ = StdCommand::new("sh")
                .arg("-c")
                .arg("echo 'ATDT' | minimodem --tx -a 1200")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        });
        thread::sleep(Duration::from_millis(800));
    }
    
    fn play_handshake(&self) {
        if !self.config.sound_enabled {
            return;
        }
        
        println!("{}", "♪ Handshaking...".yellow());
        thread::spawn(move || {
            let _ = StdCommand::new("sh")
                .arg("-c")
                .arg("echo 'CONNECT 1200' | minimodem --tx -a 1200")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        });
        thread::sleep(Duration::from_millis(500));
    }
    
    fn play_disconnect(&self) {
        if !self.config.sound_enabled {
            return;
        }
        
        println!("{}", "♪ Disconnecting...".red());
        thread::spawn(|| {
            let _ = StdCommand::new("sh")
                .arg("-c")
                .arg("echo '+++ATH' | minimodem --tx -a 1200")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        });
        thread::sleep(Duration::from_millis(500));
    }
    
    // HTTP connection using reqwest
    async fn connect_http(&mut self, url: &str, method: Option<&str>) -> Result<()> {
        let method = method.unwrap_or("GET");
        let start_time = std::time::Instant::now();
        
        self.show_status(&format!("Initializing HTTP connection to {}", url));
        self.play_dial_tone();
        
        println!("{}", "Connecting via HTTP...".yellow());
        
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;
        
        let result = match method.to_uppercase().as_str() {
            "GET" => {
                match client.get(url).send().await {
                    Ok(response) => {
                        self.play_handshake();
                        let status = response.status();
                        let headers = response.headers().clone();
                        let body = response.text().await?;
                        
                        println!("{}", format!("HTTP {} | Size: {} bytes | Time: {:.2}s", 
                            status, body.len(), start_time.elapsed().as_secs_f64()).green());
                        
                        // Show some headers
                        for (name, value) in headers.iter().take(5) {
                            println!("{}: {}", name.as_str().cyan(), 
                                value.to_str().unwrap_or("invalid").dimmed());
                        }
                        
                        // Show first 500 chars of body
                        if body.len() > 500 {
                            println!("\n{}\n...truncated", &body[..500].dimmed());
                        } else if !body.is_empty() {
                            println!("\n{}", body.dimmed());
                        }
                        
                        self.show_success("HTTP GET connection established");
                        Ok(())
                    }
                    Err(e) => {
                        self.show_error(&format!("HTTP connection failed: {}", e));
                        Err(anyhow!(e))
                    }
                }
            }
            "HEAD" => {
                match client.head(url).send().await {
                    Ok(response) => {
                        self.play_handshake();
                        let status = response.status();
                        let headers = response.headers();
                        
                        println!("{}", format!("HTTP {} HEAD", status).green());
                        for (name, value) in headers.iter().take(10) {
                            println!("{}: {}", name.as_str().cyan(), 
                                value.to_str().unwrap_or("invalid").dimmed());
                        }
                        
                        self.show_success("HTTP HEAD request completed");
                        Ok(())
                    }
                    Err(e) => {
                        self.show_error(&format!("HTTP HEAD request failed: {}", e));
                        Err(anyhow!(e))
                    }
                }
            }
            _ => {
                self.show_error("Unsupported HTTP method");
                Err(anyhow!("Unsupported HTTP method"))
            }
        };
        
        let duration = start_time.elapsed();
        let status = if result.is_ok() { "SUCCESS" } else { "FAILED" };
        self.log_connection("HTTP", url, status, duration);
        
        result
    }
    
    // Download file using external wget
    async fn download_file(&mut self, url: &str, output: Option<&str>) -> Result<()> {
        let start_time = std::time::Instant::now();
        let filename = output.unwrap_or_else(|| {
            Url::parse(url)
                .ok()
                .and_then(|u| u.path_segments())
                .and_then(|segments| segments.last())
                .unwrap_or("download")
        });
        
        self.show_status(&format!("Initiating file transfer from {}", url));
        self.play_dial_tone();
        
        println!("{}", "Downloading via WGET protocol...".cyan());
        
        let mut cmd = TokioCommand::new("wget");
        cmd.args(&["--progress=bar", "--timeout=30", "-O", filename, url])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        
        let mut child = cmd.spawn()?;
        
        // Read stderr for progress updates
        if let Some(stderr) = child.stderr.take() {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            
            tokio::spawn(async move {
                while let Ok(Some(line)) = lines.next_line().await {
                    if line.contains('%') || line.contains("saved") {
                        println!("{}", line.dimmed());
                    }
                }
            });
        }
        
        let status = child.wait().await?;
        let duration = start_time.elapsed();
        
        if status.success() {
            self.play_handshake();
            self.show_success(&format!("File downloaded successfully: {}", filename));
            self.log_connection("DOWNLOAD", url, "SUCCESS", duration);
            Ok(())
        } else {
            self.show_error("Download failed");
            self.log_connection("DOWNLOAD", url, "FAILED", duration);
            Err(anyhow!("Download failed"))
        }
    }
    
    // SSH connection using external ssh client
    async fn connect_ssh(&mut self, target: &str) -> Result<()> {
        let start_time = std::time::Instant::now();
        
        self.show_status(&format!("Establishing SSH connection to {}", target));
        self.play_dial_tone();
        
        println!("{}", "Connecting via SSH protocol...".green());
        
        let status = StdCommand::new("ssh")
            .arg(target)
            .status();
        
        let duration = start_time.elapsed();
        
        match status {
            Ok(exit_status) => {
                if exit_status.success() {
                    self.play_handshake();
                    self.show_success("SSH connection completed");
                    self.log_connection("SSH", target, "SUCCESS", duration);
                } else {
                    self.show_error("SSH connection failed");
                    self.log_connection("SSH", target, "FAILED", duration);
                }
                self.play_disconnect();
                Ok(())
            }
            Err(e) => {
                self.show_error(&format!("SSH client error: {}", e));
                self.log_connection("SSH", target, "ERROR", duration);
                Err(anyhow!(e))
            }
        }
    }
    
    // Telnet connection
    async fn connect_telnet(&mut self, host: &str, port: Option<&str>) -> Result<()> {
        let port = port.unwrap_or("23");
        let target = format!("{}:{}", host, port);
        let start_time = std::time::Instant::now();
        
        self.show_status(&format!("Establishing Telnet connection to {}", target));
        self.play_dial_tone();
        
        println!("{}", "Connecting via TELNET protocol...".magenta());
        
        let status = StdCommand::new("telnet")
            .args(&[host, port])
            .status();
        
        let duration = start_time.elapsed();
        
        match status {
            Ok(exit_status) => {
                if exit_status.success() {
                    self.play_handshake();
                    self.show_success("Telnet connection completed");
                    self.log_connection("TELNET", &target, "SUCCESS", duration);
                } else {
                    self.show_error("Telnet connection failed");
                    self.log_connection("TELNET", &target, "FAILED", duration);
                }
                self.play_disconnect();
                Ok(())
            }
            Err(e) => {
                self.show_error(&format!("Telnet client error: {}", e));
                self.log_connection("TELNET", &target, "ERROR", duration);
                Err(anyhow!(e))
            }
        }
    }
    
    // Show configuration menu
    fn configure_modem(&mut self) -> Result<()> {
        println!("{}", "Modem Configuration".yellow().bold());
        println!("{}", "────────────────────".dimmed());
        println!("1) Baud Rate (current: {})", self.config.baud_rate);
        println!("2) Connection Type (current: {})", self.config.connection_type);
        println!("3) Sound Enabled (current: {})", self.config.sound_enabled);
        println!("4) Reset to defaults");
        println!("5) Back to main menu");
        
        print!("\nSelect option: ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        match input.trim() {
            "1" => {
                println!("Available baud rates: 300, 1200, 2400, 9600, 14400, 28800, 56000");
                print!("Enter baud rate: ");
                io::stdout().flush()?;
                
                let mut rate_input = String::new();
                io::stdin().read_line(&mut rate_input)?;
                
                if let Ok(rate) = rate_input.trim().parse::<u32>() {
                    self.config.baud_rate = rate;
                    self.save_config()?;
                    self.show_success(&format!("Baud rate set to {}", rate));
                } else {
                    self.show_error("Invalid baud rate");
                }
            }
            "2" => {
                println!("Available types: hayes, bell, v90, v92");
                print!("Enter connection type: ");
                io::stdout().flush()?;
                
                let mut type_input = String::new();
                io::stdin().read_line(&mut type_input)?;
                
                self.config.connection_type = type_input.trim().to_string();
                self.save_config()?;
                self.show_success(&format!("Connection type set to {}", self.config.connection_type));
            }
            "3" => {
                self.config.sound_enabled = !self.config.sound_enabled;
                self.save_config()?;
                self.show_success(&format!("Sound {}", 
                    if self.config.sound_enabled { "enabled" } else { "disabled" }));
            }
            "4" => {
                self.config = ModemConfig::default();
                self.save_config()?;
                self.show_success("Configuration reset to defaults");
            }
            _ => {}
        }
        
        Ok(())
    }
    
    // Show phonebook/connection history
    fn show_phonebook(&self) {
        println!("{}", "VModem Phone Book".cyan().bold());
        println!("{}", "─────────────────".dimmed());
        println!("Recent connections:");
        
        if self.connection_history.is_empty() {
            println!("  No recent connections");
        } else {
            for entry in self.connection_history.iter().rev().take(10) {
                let status_color = match entry.status.as_str() {
                    "SUCCESS" => "green",
                    "FAILED" => "red",
                    _ => "yellow",
                };
                
                println!("  {} {} {} {} ({}ms)", 
                    entry.timestamp.format("%m-%d %H:%M").to_string().dimmed(),
                    entry.connection_type.blue(),
                    entry.target.white(),
                    entry.status.color(status_color),
                    entry.duration_ms.to_string().dimmed()
                );
            }
        }
        println!();
    }
    
    // Show help
    fn show_help(&self) {
        println!("{}", "VModem Model 99/A Help".green().bold());
        println!("{}", "═".repeat(25).dimmed());
        println!();
        println!("{}", "Available Commands:".bold());
        println!("  {} - Connect via HTTP (GET/HEAD)", "http <url> [method]".cyan());
        println!("  {} - Download file via wget", "download <url> [file]".cyan());
        println!("  {} - Connect via SSH", "ssh <host>".cyan());
        println!("  {} - Connect via Telnet", "telnet <host> [port]".cyan());
        println!("  {} - Configure modem settings", "config".cyan());
        println!("  {} - View connection history", "phonebook".cyan());
        println!("  {} - Clear screen", "clear".cyan());
        println!("  {} - Show this help", "help".cyan());
        println!("  {} - Exit VModem", "quit".cyan());
        println!();
        println!("{}", "Examples:".bold());
        println!("  {}", "http https://httpbin.org/ip".dimmed());
        println!("  {}", "download https://example.com/file.txt".dimmed());
        println!("  {}", "ssh user@example.com".dimmed());
        println!("  {}", "telnet towel.blinkenlights.nl".dimmed());
        println!();
    }
    
    // Handle individual commands
    async fn handle_command(&mut self, command: &str, args: Vec<&str>) -> Result<bool> {
        match command {
            "http" => {
                if args.is_empty() {
                    self.show_error("URL required");
                    return Ok(false);
                }
                let method = args.get(1).copied();
                let _ = self.connect_http(args[0], method).await;
            }
            "download" | "dl" => {
                if args.is_empty() {
                    self.show_error("URL required");
                    return Ok(false);
                }
                let output = args.get(1).copied();
                let _ = self.download_file(args[0], output).await;
            }
            "ssh" => {
                if args.is_empty() {
                    self.show_error("Host required");
                    return Ok(false);
                }
                let _ = self.connect_ssh(args[0]).await;
            }
            "telnet" => {
                if args.is_empty() {
                    self.show_error("Host required");
                    return Ok(false);
                }
                let port = args.get(1).copied();
                let _ = self.connect_telnet(args[0], port).await;
            }
            "config" | "configure" => {
                let _ = self.configure_modem();
            }
            "phonebook" | "pb" => {
                self.show_phonebook();
            }
            "help" | "?" => {
                self.show_help();
            }
            "clear" | "cls" => {
                self.show_banner();
            }
            "quit" | "exit" | "bye" => {
                println!("{}", "Hanging up modem...".yellow());
                self.play_disconnect();
                println!("{}", "73! Thanks for using VModem 99/A".green());
                return Ok(true);
            }
            "" => {
                // Empty command, do nothing
            }
            _ => {
                self.show_error(&format!("Unknown command: {} (type 'help' for commands)", command));
            }
        }
        Ok(false)
    }
    
    // Interactive mode
    async fn interactive_mode(&mut self) -> Result<()> {
        self.show_banner();
        println!("{}", "Ready! Type 'help' for commands or 'quit' to exit.".green());
        println!();
        
        let mut rl = Editor::<()>::new()?;
        
        loop {
            match rl.readline(&format!("{}VModem>{} ", "".cyan().bold(), "".normal())) {
                Ok(line) => {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }
                    
                    rl.add_history_entry(line);
                    
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.is_empty() {
                        continue;
                    }
                    
                    let command = parts[0];
                    let args = parts[1..].to_vec();
                    
                    if self.handle_command(command, args).await? {
                        break;
                    }
                    
                    println!();
                }
                Err(rustyline::error::ReadlineError::Interrupted) |
                Err(rustyline::error::ReadlineError::Eof) => {
                    println!("{}", "\nHanging up modem...".yellow());
                    self.play_disconnect();
                    println!("{}", "73! Thanks for using VModem 99/A".green());
                    break;
                }
                Err(err) => {
                    self.show_error(&format!("Input error: {}", err));
                }
            }
        }
        
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let matches = Command::new("vmodem99a")
        .about("VModem Model 99/A - Virtual Modem Terminal")
        .version("1.0.0")
        .arg(Arg::new("command")
            .help("Command to execute")
            .index(1))
        .arg(Arg::new("args")
            .help("Command arguments")
            .multiple_values(true)
            .index(2))
        .get_matches();
    
    let mut vmodem = VModem::new()?;
    
    if let Some(command) = matches.value_of("command") {
        vmodem.show_banner();
        let args: Vec<&str> = matches.values_of("args").unwrap_or_default().collect();
        vmodem.handle_command(command, args).await?;
    } else {
        vmodem.interactive_mode().await?;
    }
    
    Ok(())
}
