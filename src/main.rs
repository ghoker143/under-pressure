use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader, Write};
use std::fs::OpenOptions;
use std::collections::VecDeque;
use std::env;
use chrono::{Local, DateTime};
use serialport::SerialPort;
use std::path::Path;
use ctrlc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <mode> [<command/port> <args>]", args[0]);
        eprintln!("Modes: command, serial");
        std::process::exit(1);
    }

    let mode = &args[1];

    match mode.as_str() {
        "command" => run_command_mode(&args[2..]),
        "serial" => run_serial_mode(&args[2..]),
        _ => {
            eprintln!("Invalid mode. Use 'command' or 'serial'.");
            std::process::exit(1);
        }
    }
}

fn run_command_mode(args: &[String]) {
    if args.is_empty() {
        eprintln!("Usage: command <command> <args>");
        std::process::exit(1);
    }

    let command = &args[0];
    let command_args = &args[1..];

    println!("Executing command: {} {:?}", command, command_args);
    let timestamp = Local::now().format("%Y%m%d%H%M%S").to_string();
    let log_file_path = if command == "sudo" {
        format!("{}_{}.log", command_args[0], timestamp)
    } else {
        format!("{}_{}.log", command, timestamp)
    };

    let process = Command::new(command)
        .args(command_args)
        .stdout(Stdio::piped())
        .spawn()
        .expect("Cannot start command");

    let stdout = process.stdout.expect("Cannot get stdout");
    let reader = BufReader::new(stdout);

    process_output(reader, &log_file_path);
}

fn sanitize_filename(filename: &str) -> String {
    let file_name = Path::new(filename).file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(filename);

    file_name.chars()
        .filter(|&c| c.is_alphanumeric() || c == '_' || c == '-')
        .collect()
}

fn run_serial_mode(args: &[String]) {
    if args.len() < 2 {
        eprintln!("Usage: serial <port> <baud_rate>");
        std::process::exit(1);
    }

    let port_name = &args[0];
    let baud_rate: u32 = args[1].parse().expect("Invalid baud rate");

    println!("Opening serial port: {} at {} baud", port_name, baud_rate);
    let timestamp = Local::now().format("%Y%m%d%H%M%S").to_string();
    let sanitized_port_name = sanitize_filename(port_name);
    let log_file_path = format!("serial_{}_{}.log", sanitized_port_name, timestamp);

    let port = serialport::new(port_name, baud_rate)
        .timeout(std::time::Duration::from_millis(10))
        .open()
        .expect("Failed to open serial port");

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");

    process_serial_output(port, &log_file_path, running);
}

fn process_output<R: BufRead>(reader: R, log_file_path: &str) {
    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file_path)
        .expect("Cannot open log file");

    let mut writer = std::io::BufWriter::new(log_file);
    let mut output_lines = VecDeque::new();

    for line in reader.lines() {
        let line = line.expect("Cannot read line");
        
        output_lines.push_back(line.clone());

        if output_lines.len() > 300 {
            output_lines.pop_front();
        }

        print!("\x1B[2J\x1B[1;1H");

        for output_line in &output_lines {
            println!("{}", output_line);
        }

        writeln!(writer, "{}", line).expect("Cannot write to log file");
    }

    writer.flush().expect("Cannot flush log file");
}

fn process_serial_output(mut port: Box<dyn SerialPort>, log_file_path: &str, running: Arc<AtomicBool>) {
    let log_file = OpenOptions::new()
    .create(true)
    .append(true)
    .open(log_file_path)
    .expect("Cannot open log file");

    let mut writer = std::io::BufWriter::new(log_file);
    let mut output_lines = VecDeque::new();
    let mut serial_buf: Vec<u8> = vec![0; 1000];

    while running.load(Ordering::SeqCst) {
        match port.read(serial_buf.as_mut_slice()) {
            Ok(t) => {
                let now: DateTime<Local> = Local::now();
                let timestamp = now.format("%Y-%m-%d %H:%M:%S").to_string();
                let line = String::from_utf8_lossy(&serial_buf[..t]).to_string();
                let timestamped_line = format!("\n[{}]\n{}", timestamp, line);

                output_lines.push_back(timestamped_line.clone());

                if output_lines.len() > 300 {
                    output_lines.pop_front();
                }

                print!("\x1B[2J\x1B[1;1H");

                for output_line in &output_lines {
                    print!("{}", output_line);
                }

                write!(writer, "{}", timestamped_line).expect("Cannot write to log file");
                writer.flush().expect("Cannot flush log file");
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => (),
            Err(e) => eprintln!("{:?}", e),
        }
    }

    println!("\nClosing serial port and exiting...");
}
