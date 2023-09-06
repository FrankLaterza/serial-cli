use crossterm::event::KeyEventState;
use crossterm::ExecutableCommand;
use crossterm::{
    event::{self, KeyCode, KeyEvent, KeyEventKind},
    terminal::{self, ClearType},
};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{io, thread};

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

enum State {
    Start,
    Body,
    End,
}

fn main() {
    let mut config_file = File::open("config.json").expect("Unable to read file");

    let mut config_data = String::new();
    config_file.read_to_string(&mut config_data).unwrap();

    let config: Config = serde_json::from_str(&config_data).expect("Error loading config.json");

    println!("Config loaded");

    // kill thread
    // Create an Arc<AtomicBool> to signal termination
    let terminate_flag = Arc::new(AtomicBool::new(false));

    // Clone the Arc for the threads
    let thread1_terminate_flag = Arc::clone(&terminate_flag);
    let thread2_terminate_flag = Arc::clone(&terminate_flag);

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
    thread::spawn(move || {
        while !thread1_terminate_flag.load(Ordering::Relaxed) {
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
                            clone
                                .write_all(thread1.lock().unwrap().as_bytes())
                                .expect("Failed to write to serial port");
                            thread1.lock().unwrap().clear();
                            io::stdout().flush().unwrap();
                        }
                        KeyCode::Esc => {
                            thread1_terminate_flag.store(true, Ordering::Relaxed);
                            break;
                        }

                        _ => {}
                    }
                }
            }
        }
    });

    // Read the four bytes back from the cloned port
    let mut float_buffer = [0; 4];
    let mut int_buffer = [0; 4];
    let mut byte_buffer = [0; 1];

    let mut state: State = State::Start;

    loop {
        if thread2_terminate_flag.load(Ordering::Relaxed) {
            print!("thread killed\r\n");
            break; // Exit the loop when the termination flag is set
        }
        match state {
            State::Start => {
                match port.read(&mut byte_buffer) {
                    Ok(bytes) => {
                        if bytes == 1 {
                            // Move the cursor up one line
                            print!("\x1b[1A");
                            // Delete the current line
                            print!("\x1b[K");
                            print!("Start: {}\n\r", byte_buffer[0]);
                            print!("{}\r\n", thread2.lock().unwrap());
                            io::stdout().flush().unwrap();
                            if byte_buffer[0] == config.start as u8 {
                                state = State::Body;
                            }
                        }
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
                    Err(e) => {
                        eprintln!("{:?}", e);
                        break; // Exit the loop on error
                    }
                }
            }
            State::Body => {
                for types in &config.body {
                    for count in 1..=types.count {
                        // Chill
                        thread::sleep(Duration::from_millis(10));

                        match types.data_type.as_str() {
                            "float" => {
                                port.read_exact(&mut float_buffer).unwrap();
                                float_buffer.reverse();
                                // Move the cursor up one line
                                print!("\x1b[1A");
                                // Delete the current lineA
                                print!("\x1b[K");
                                let float_value = f32::from_ne_bytes(float_buffer);
                                print!("Float: {}\n\r", float_value);
                                print!("{}\r\n", thread2.lock().unwrap());
                                io::stdout().flush().unwrap();
                            }
                            "int" => {
                                port.read_exact(&mut int_buffer).unwrap();
                                int_buffer.reverse();
                                // Move the cursor up one line
                                print!("\x1b[1A");
                                // Delete the current lineA
                                print!("\x1b[K");
                                let int_value = i32::from_ne_bytes(int_buffer);
                                print!("Int: {}\r\n", int_value);
                                print!("{}\r\n", thread2.lock().unwrap());
                                io::stdout().flush().unwrap();
                            }
                            "byte" => {
                                port.read(&mut byte_buffer).unwrap();
                                // Move the cursor up one line
                                print!("\x1b[1A");
                                // Delete the current lineA
                                print!("\x1b[K");
                                print!("Byte: {}\n\r", byte_buffer[0]);
                                print!("{}\r\n", thread2.lock().unwrap());
                                io::stdout().flush().unwrap();
                            }
                            _ => {}
                        }
                    }
                }
                // loop finshed check end
                state = State::End;
            }

            State::End => {
                match port.read(&mut byte_buffer) {
                    Ok(bytes) => {
                        if bytes == 1 {
                            // Move the cursor up one line
                            print!("\x1b[1A");
                            // Delete the current line
                            print!("\x1b[K");
                            print!("End: {}\n\r", byte_buffer[0]);
                            print!("{}\r\n", thread2.lock().unwrap());
                            io::stdout().flush().unwrap();
                            if byte_buffer[0] == config.end as u8 {
                                state = State::Start;
                            }
                            else {
                                print!("End not found!\r\n");
                            }
                        }
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
                    Err(e) => {
                        eprintln!("{:?}", e);
                        break; // Exit the loop on error
                    }
                }
            }
        }
        // Chill out
        thread::sleep(Duration::from_millis(10));
    }

    // Put terminal back
    terminal::disable_raw_mode().unwrap();
    println!("Exiting Program");
    return;
}
