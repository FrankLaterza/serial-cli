use crossterm::{ event::{ self, Event, KeyCode, KeyEvent, KeyEventKind }, terminal::{ self } };
use serde::{ Deserialize, Serialize };
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::sync::atomic::{ AtomicBool, Ordering };
use std::sync::{ Arc, Mutex };
use std::time::Duration;
use std::{ io, thread };

#[derive(Serialize, Deserialize)]
struct Config {
    baud: u32,
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
    let terminate_flag = Arc::new(AtomicBool::new(false));

    // Clone the Arc for the threads
    let thread1_terminate_flag = Arc::clone(&terminate_flag);
    let thread2_terminate_flag = Arc::clone(&terminate_flag);

    // Create mutble input buf
    let shared_input = Arc::new(Mutex::new(String::new()));
    let thread1 = Arc::clone(&shared_input);
    let thread2 = Arc::clone(&shared_input);

    // Get available ports
    let ports = serialport::available_ports().expect("No ports found!");

    // Create a vector of port names
    let port_list: Vec<String> = ports
        .iter()
        .map(|p| p.port_name.clone())
        .collect();

    // Print the list of ports with numbers
    for (port_count, port) in port_list.iter().enumerate() {
        println!("{}) {}", port_count + 1, port);
    }

    print!("Select port: ");
    io::stdout().flush().unwrap();

    let mut user_input = String::new();
    io::stdin().read_line(&mut user_input).expect("Failed to read line");

    // Parse the user's input into a usize (index)
    let selected_port: usize = user_input.trim().parse().expect("Not a valid option");
    let port_path: String;

    // Check if the selected_port is a valid index
    if selected_port > port_list.len() {
        panic!("Invalid port selection.");
    } else {
        // Get the selected port's path
        port_path = String::from(&port_list[selected_port - 1]);
        println!("Selected port: {}", port_path);
    }

    println!();
    print!("**********************************************\n");
    print!("**                                          **\n");
    print!("**    Welcome to Frank's Serial Monitor!    **\n");
    print!("**                                          **\n");
    print!("**********************************************\n");
    println!();
    io::stdout().flush().unwrap();

    let mut port = serialport
        ::new(&port_path, config.baud)
        .open()
        .expect("Failed to open serial port");
    // Clone the port
    let mut clone = port.try_clone().expect("Failed to clone");

    print!("\r\n");
    thread::spawn(move || {
        while !thread1_terminate_flag.load(Ordering::Relaxed) {
            terminal::enable_raw_mode().unwrap();
            // Don't ask me about this syntax...
            if event::poll(Duration::from_millis(20)).unwrap() {
                if
                    let Event::Key(KeyEvent { code, kind: KeyEventKind::Press, .. }) = event
                        ::read()
                        .unwrap()
                {
                    match code {
                        KeyCode::Char(c) => {
                            // Move the cursor up one line
                            print!("\x1b[1A");
                            // Delete the current line
                            print!("\x1b[K");
                            thread1.lock().unwrap().push(c);
                            print!("{}\r\n", thread1.lock().unwrap());
                            io::stdout().flush().unwrap();
                            thread::sleep(Duration::from_millis(10));
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
                            println!("sending serial to clone");
                            let buf: u8 = 100;
                            clone
                                .write(&[buf])
                                .expect("Failed to write to serial port");
                            println!("sent serial to clone");
                            // thread1.lock().unwrap().clear();
                            println!("clearing locked thread");
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
        println!("Thread 1 terminated");
    });

    // Read the four bytes back from the cloned port
    let mut float_buffer = [0; 4];
    let mut int_buffer = [0; 4];
    let mut byte_buffer = [0; 1];

    let mut state: State = State::Start;

    loop {
        if thread2_terminate_flag.load(Ordering::Relaxed) {
            println!("Terminating thread 2");
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
                            if byte_buffer[0] == (config.start as u8) {
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
                    for _count in 1..=types.count {
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
                            if byte_buffer[0] == (config.end as u8) {
                                print!("End: {}\n\r", byte_buffer[0]);
                                print!("{}\r\n", thread2.lock().unwrap());
                                state = State::Start;
                            } else {
                                print!("End not found!\r\n\r\n");
                            }
                            io::stdout().flush().unwrap();
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
        thread::sleep(Duration::from_millis(100));
    }

    // Put terminal back
    terminal::disable_raw_mode().unwrap();
    println!("Exiting Program");
    return;
}
