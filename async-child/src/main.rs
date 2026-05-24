use tokio::io::{self, AsyncBufReadExt, BufReader};

async fn command_handler() -> io::Result<()> {
    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin);

    loop {
        let mut buffer = String::new();
        let x = reader.read_line(&mut buffer).await;
        match x {
            Ok(0) => { println!("EOF"); break }, // EOF
            Ok(_) => println!("You typed: {}", buffer.trim()),
            Err(e) => {
                eprintln!("Error reading line: {}", e);
                break;
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let task = tokio::spawn(command_handler());
    println!("Hello, world!");
    let x = task.await;
    match x {
        Ok(Ok(())) => println!("Command handler finished successfully."),
        Ok(Err(e)) => eprintln!("Command handler error: {}", e),
        Err(e) => eprintln!("Task join error: {}", e),
    }
    Ok(())
}
