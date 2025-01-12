use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader, Write};
use std::fs::OpenOptions;
use std::collections::VecDeque;
use std::env;
use chrono::Local;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <command> <args>", args[0]);
        std::process::exit(1);
    }

    let command = &args[1];
    let command_args = &args[2..];

    println!("Executing command: {} {:?}", command, command_args);
    let timestamp = Local::now().format("%Y%m%d%H%M%S").to_string();
    let log_file_path = if command == "sudo" {
        format!("{}_{}.log", command_args[0], timestamp)
    } else {
        format!("{}_{}.log", command, timestamp)
    };
    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file_path)
        .expect("无法打开日志文件");

    let mut writer = std::io::BufWriter::new(log_file);
    let mut output_lines = VecDeque::new();

    let process = Command::new(command)
        .args(command_args)
        .stdout(Stdio::piped())
        .spawn()
        .expect("无法启动命令，检查命令和参数是否正确");

    let stdout = process.stdout.expect("无法获取标准输出");
    let reader = BufReader::new(stdout);

    for line in reader.lines() {
        let line = line.expect("无法读取行");
        
        output_lines.push_back(line.clone());

        if output_lines.len() > 300 {
            output_lines.pop_front();
        }

        print!("\x1B[2J\x1B[1;1H");

        for output_line in &output_lines {
            println!("{}", output_line);
        }

        writeln!(writer, "{}", line).expect("无法写入日志文件");
    }

    writer.flush().expect("无法刷新日志文件");
}
