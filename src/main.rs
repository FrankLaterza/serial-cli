use crossterm::{ event::{ self, KeyCode, KeyEventKind }, terminal::{ self } };
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
    print!("**  Welcome to ETA SPACE's Binary Decoder!  **\n");
    print!("**                                          **\n");
    print!("**********************************************\n");

    // printing commands an operation
    print!("Exit the program with ESC\n");
    print!("\n");

    // print loading config
    print!("Loading Config:\n");
    print!("{}", config_data);

    print!("********************START*********************\n");

    print!("");
    println!();
    io::stdout().flush().unwrap();

    let mut port = serialport
        ::new(&port_path, config.baud)
        .open()
        .expect("Failed to open serial port");
    // Clone the port
    let mut clone = port.try_clone().expect("Failed to clone");

    print!(">");
    io::stdout().flush().unwrap();
    thread::spawn(move || {
        while !thread1_terminate_flag.load(Ordering::Relaxed) {
            terminal::enable_raw_mode().unwrap();

            if event::poll(Duration::from_millis(1)).unwrap() {
                if let crossterm::event::Event::Key(event) = event::read().unwrap() {
                    // if its not a press then don't care
                    if event.kind != KeyEventKind::Press {
                        continue;
                    }
                    match event.code {
                        KeyCode::Char(c) => {
                            // Move the cursor up one line
                            // print!("\x1b[1A");
                            // // Delete the current line
                            // print!("\x1b[K");
                            thread1.lock().unwrap().push(c);
                            print!("{}", c);
                            // drop(thread1);
                            io::stdout().flush().unwrap();
                        }
                        KeyCode::Backspace => {
                            // Handle backspace: Remove the last character from the string and update the output
                            let mut string_guard = thread1.lock().unwrap();
                            if !string_guard.is_empty() {
                                string_guard.pop();
                                // Move the cursor left and clear to the end of the line
                                print!("\x08 \x08");
                                io::stdout().flush().unwrap();
                            }
                        }
                        KeyCode::Enter => {
                            // Print a new line when Enter is pressed
                            // print!("\r\n");
                            print!("writing \r\n");
                            clone
                                .write_all(thread1.lock().unwrap().as_bytes())
                                .expect("Failed to write to serial port");
                            print!("written \r\n");
                            thread1.lock().unwrap().clear();
                            io::stdout().flush().unwrap();
                        }
                        KeyCode::Esc => {
                            // thread flag not read fast enough
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
    let mut byte_buffer: Vec<u8> = vec![0; 1];

    let mut state: State = State::Start;

    loop {
        if thread2_terminate_flag.load(Ordering::Relaxed) {
            break; // Exit the loop when the termination flag is set
        }
        match state {
            State::Start => {
                match port.read(&mut byte_buffer.as_mut_slice()) {
                    Ok(n) => {
                        if n >= 1 {
                            print!("\r\x1B[K");
                            print!("Start: {}\r\n", byte_buffer[0]);
                            print!(">{}", thread2.lock().unwrap());
                            io::stdout().flush().unwrap();
                            if byte_buffer[0] == (config.start as u8) {
                                state = State::Body;
                            }
                        }
                    }
                    Err(e) => {
                        eprint!("{}", e);
                        // thread flag not read fast enough (kill)
                        thread2_terminate_flag.store(true, Ordering::Relaxed);
                        break;
                    }
                }
            }
            State::Body => {
                for types in &config.body {
                    let mut byte_found_count = 0;
                    while byte_found_count < types.count {
                        match types.data_type.as_str() {
                            "float" => {
                                let mut float_byte_count = 0;
                                while float_byte_count < 4 {
                                    match port.read(&mut byte_buffer) {
                                        Ok(n) => {
                                            if n == 1 {
                                                // pack the float
                                                float_buffer[float_byte_count] = byte_buffer[0];
                                                float_byte_count += 1;
                                            }
                                        }
                                        Err(_) => {
                                            // thread flag not read fast enough (kill)
                                            thread2_terminate_flag.store(true, Ordering::Relaxed);
                                            break;
                                        }
                                    }
                                }
                                byte_found_count += 1;
                                // float_buffer.reverse();
                                let float_value = f32::from_ne_bytes(float_buffer);
                                print!("\r\x1B[K");
                                print!("Float: {}\r\n", float_value);
                                print!(">{}", thread2.lock().unwrap());
                                io::stdout().flush().unwrap();
                            }
                            "int" => {
                                let mut int_byte_count = 0;
                                while int_byte_count < 4 {
                                    match port.read(&mut byte_buffer) {
                                        Ok(n) => {
                                            if n == 1 {
                                                // pack the int
                                                int_buffer[int_byte_count] = byte_buffer[0];
                                                int_byte_count += 1;
                                            }
                                        }
                                        Err(_) => {
                                            // thread flag not read fast enough (kill)
                                            thread2_terminate_flag.store(true, Ordering::Relaxed);
                                            break;
                                        }
                                    }
                                }
                                byte_found_count += 1;
                                int_buffer.reverse();
                                let int_value = i32::from_ne_bytes(int_buffer);
                                print!("\r\x1B[K");
                                print!("Int: {}\n\r", int_value);
                                print!(">{}", thread2.lock().unwrap());
                                io::stdout().flush().unwrap();
                                // Convert Float into
                            }
                            "byte" => {
                                match port.read(&mut byte_buffer.as_mut_slice()) {
                                    Ok(n) => {
                                        if n == 1 {
                                            byte_found_count += 1;
                                            print!("\r\x1B[K");
                                            // Move the cursor up one line
                                            print!("Byte: {}\n\r", byte_buffer[0]);
                                            print!(">{}", thread2.lock().unwrap());
                                            io::stdout().flush().unwrap();
                                        }
                                    }
                                    Err(_) => {
                                        // thread flag not read fast enough (kill)
                                        thread2_terminate_flag.store(true, Ordering::Relaxed);
                                        break;
                                    }
                                }
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
                            if byte_buffer[0] == (config.end as u8) {
                                print!("\r\x1B[K");
                                print!("End: {}\r\n", byte_buffer[0]);
                                print!(">{}", thread2.lock().unwrap());
                                state = State::Start;
                            } else {
                                print!("\r\x1B[K");
                                print!("End not found: {}\r\n", byte_buffer[0]);
                                print!(">{}", thread2.lock().unwrap());
                            }
                            io::stdout().flush().unwrap();
                        }
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::TimedOut => {
                        // thread flag not read fast enough (kill)
                        thread2_terminate_flag.store(true, Ordering::Relaxed);
                        break;
                    }
                    Err(e) => {
                        eprintln!("{:?}", e);
                        // thread flag not read fast enough (kill)
                        thread2_terminate_flag.store(true, Ordering::Relaxed);
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
