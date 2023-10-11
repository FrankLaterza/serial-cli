# serial-cli
A command line interface built in rust designed to decode binary data from byte, int, and float types. Program runs with a configuration file called config.json.


## How to start
To start you can run the program in the main directory with ```cargo run```. This will read the config file in the main directory as well. Its recommended to run the program from the binary compiled with ```cargo build --release``` where the executable can be found in target/release. This comes with its own config file in the same directory.

## Installing rust
Installing rust is simple. You can navigate to [here](https://www.rust-lang.org/tools/installP) to download rustup. This is the install manager. Simply follow the instructions on the terminal and confirm your installation with ```rustc --version```.
