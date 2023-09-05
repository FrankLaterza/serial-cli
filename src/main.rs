use crossterm::ExecutableCommand;
use crossterm::{
    event::{self, KeyCode, KeyEvent, KeyEventKind},
    terminal::{self, ClearType},
};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{io, thread};
// use std::sync::{Arc, AtomicBool};

#[derive(Serialize, Deserialize)]
struct Config {
    start: char,
    body: Vec<Types>,
    end: char,
}

#[derive(Serialize, Deserialize)]
struct Types {
    data_type: String,
    count: u32,
}

fn main() {
    let mut config_file = File::open("config.json").expect("Unable to read file");

    let mut config_data = String::new();
    config_file.read_to_string(&mut config_data).unwrap();

    let config: Config = serde_json::from_str(&config_data).expect("Error loading config.json");

    println!("Config loaded");

    // kill thread

    let mut input = String::new();

    // Create mutble input buf
    let shared_input = Arc::new(Mutex::new(String::new()));
    let thread1 = Arc::clone(&shared_input);
    let thread2 = Arc::clone(&shared_input);

    // Open the first serialport available.
    let port_name = &serialport::available_ports().expect("No serial port")[0].port_name;
    let mut port = serialport::new(port_name, 9600)
        .open()
        .expect("Failed to open serial port");

    // Clone the port
    let mut clone = port.try_clone().expect("Failed to clone");

    print!("\r\n");
    thread::spawn(move || loop {
        terminal::enable_raw_mode().unwrap();

        if event::poll(Duration::from_millis(1)).unwrap() {
            if let crossterm::event::Event::Key(event) = event::read().unwrap() {
                match event.code {
                    KeyCode::Char(c) => {
                        // Move the cursor up one line
                        print!("\x1b[1A");
                        // Delete the current line
                        print!("\x1b[K");
                        thread1.lock().unwrap().push(c);
                        print!("{}\r\n", thread1.lock().unwrap());
                        // drop(thread1);
                        io::stdout().flush().unwrap();
                    }
                    KeyCode::Backspace => {
                        // Print a new line when Enter is pressed
                        // print!("\r\n");
                        print!("\x1b[1A");
                        // Delete the current line
                        print!("\x1b[K");
                        let mut string_gaurd = thread1.lock().unwrap();
                        string_gaurd.pop();
                        print!("{}\r\n", string_gaurd);
                        io::stdout().flush().unwrap();
                        
                    }
                    KeyCode::Enter => {
                        // Print a new line when Enter is pressed
                        // print!("\r\n");
                        clone.write_all(thread1.lock().unwrap().as_bytes())
                        .expect("Failed to write to serial port");
                        thread1.lock().unwrap().clear();
                        io::stdout().flush().unwrap();
                        
                    }
                    KeyCode::Esc => {
                        // Exit the loop if the Esc key is pressed
                        break;
                    }
                    
                    _ => {}
                }
            }
        }
    });

    // Read the four bytes back from the cloned port
    let mut buffer: [u8; 1] = [0; 1];
    loop {
        match port.read(&mut buffer) {
            Ok(bytes) => {
                if bytes == 1 {
                    // Move the cursor up one line
                    print!("\x1b[1A");
                    // Delete the current line
                    print!("\x1b[K");
                    print!("Received: {:?}\n\r", buffer);
                    print!("{}\r\n", thread2.lock().unwrap());
                    io::stdout().flush().unwrap();

                }
            }
            Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
            Err(e) => eprintln!("{:?}", e),
        }
    }
}
