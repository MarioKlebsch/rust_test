/*
    This is a test project to check out async communication to a child process.. 

    Copyright (C) 2026  Mario Klebsch, mario@klebsch.de

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use tokio::io::{self, AsyncWriteExt, AsyncReadExt, AsyncBufReadExt, BufReader};
use tokio::process::Command;
use std::process::Stdio;


async fn command_handler() -> io::Result<()> {
    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin);

    loop {
        let mut buffer = String::new();
        let x = reader.read_line(&mut buffer).await;
        match x {
            Ok(0) => { println!("EOF"); break }, // EOF
            Ok(_) => println!("You typed: >{}<", buffer),
            Err(e) => {
                eprintln!("Error reading line: {}", e);
                break;
            }
        }
    }

    Ok(())
}


async fn command_handler2() -> io::Result<()> {
    let stdin = io::stdin();
    let reader = BufReader::new(stdin);

    let mut lines = reader.lines();

    loop {
        let line = match lines.next_line().await {
            Ok(Some(line)) => line,
            Ok(None) => { println!("EOF"); break }, // EOF
            Err(e) => {
                eprintln!("Error reading line: {}", e);
                return Err(e);
            }
        };
        println!("You typed: >{}<", line);
    }

    Ok(())
}

async fn child_handler() -> io::Result<()> {
    let mut child = match Command::new("tr")
        .arg("-u")
        .arg("a-z")
        .arg("A-Z")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn() {
            Ok(child) => child,
            Err(e) => {
                eprintln!("Failed to spawn child process: {}", e);
                return Err(e);
            }
        };
    let child_stdin = match child.stdin.take() {
        Some(stdin) => stdin,
        None => {
            eprintln!("Failed to capture child process stdin");
            return Err(io::Error::new(io::ErrorKind::Other, "Failed to capture stdin"));
        }
    };
    let script_output = match child.stdout.take() {
        Some(stdout) => stdout,
        None => {
            eprintln!("Failed to capture child process stdout");
            return Err(io::Error::new(io::ErrorKind::Other, "Failed to capture stdout"));
        }
    };

    let input_task = tokio::spawn(async move {
        let mut stdin = io::stdin();
        let mut child_stdin = child_stdin;
        let mut buffer = [0; 256];

        loop {
            let len = match stdin.read(&mut buffer).await {
                Ok(0) => { println!("EOF"); break }, // EOF
                Ok(size) => { size },
                Err(e) => {
                    eprintln!("Error reading line: {}", e);
                    break;
                }
            };
            println!("read {} bytes", len);
            match child_stdin.write_all(&buffer[..len]).await {
                Ok(()) => {}
                Err(e) => {
                    eprintln!("Failed to write to child stdin: {}", e);
                    break;
                }
            }
            println!("Sent to child: >{}<", String::from_utf8_lossy(&buffer[..len]));

        }
    });
    let mut script_output_reader = BufReader::new(script_output).lines();
    let output_task = tokio::spawn(async move {
        while let Ok(Some(line)) = script_output_reader.next_line().await {
            println!("Child output: {}", line);
        }
    });

    let status = match child.wait().await {
        Ok(status) => status,
        Err(e) => {
            eprintln!("Failed to wait for child process: {}", e);
            return Err(e);
        }
    };
    println!("Child process exited with status: {}", status);
    match input_task.await {
        Ok(()) => println!("Input task finished successfully."),
        Err(e) => eprintln!("Input task error: {}", e),
    };
    match output_task.await {
        Ok(()) => println!("Output task finished successfully."),
        Err(e) => eprintln!("Output task error: {}", e),
    };
    Ok(())
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Hello, world!");
    let task = tokio::spawn(child_handler());
    let x = task.await;
    match x {
        Ok(Ok(())) => println!("Command handler finished successfully."),
        Ok(Err(e)) => eprintln!("Command handler error: {}", e),
        Err(e) => eprintln!("Task join error: {}", e),
    }
    Ok(())
}
