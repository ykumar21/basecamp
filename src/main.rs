#[allow(dead_code)]
#[allow(unused_variables)]
#[allow(non_snake_case)]

use std::io;
use std::collections::HashMap;
use std::io::{Write, Read};
use std::thread;
use std::thread::JoinHandle;
use std::sync::mpsc::{self, TryRecvError, Sender, Receiver};
use std::net::TcpStream;
use std::ptr::null;
use std::error::Error;
use ssh2::Session;
use pem::parse;
use std::path::Path;

// Struct to model the behaviour of the CLI application
struct ConsoleCLI;

impl ConsoleCLI {
    fn print_line(line: &str) {
        print!("{}", line);
        io::stdout().flush()
            .expect("Error: Fallback (TODO)");;
    }

    fn print_new_line() {
        println!();
    }

    fn delete_prev_line() {
        ConsoleCLI::print_line("\r");
    }

    fn load(loading_text:  &'static str) -> Sender<bool> {
         let (tx, rx): (Sender<bool>, Receiver<bool>) = mpsc::channel();
         thread::spawn(move || {
            ConsoleCLI::print_line(loading_text);
            let mut i : u8 = 1;
            loop {
                ConsoleCLI::print_line(".");
                if i % 4 == 0 {
                    ConsoleCLI::print_line("\x08\x08\x08\x08");
                    ConsoleCLI::print_line("    ");
                    ConsoleCLI::print_line("\x08\x08\x08\x08");
                }

                thread::sleep(std::time::Duration::from_millis(280));

                match rx.try_recv() {
                    Ok(_) | Err(TryRecvError::Disconnected) => {
                        break;
                    },
                    Err(TryRecvError::Empty) => {}
                }
            }
        });
        return tx;
    }
}

struct Server {
    session: Option<Session>,
}

impl Server {
    fn new() -> Self {
        return Server {
            session: Option::None
        };
    }

    async fn connect(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        let tx = ConsoleCLI::load("Connecting to remote SSH server");

        // Connect to the remote SSH server
        let tcp_stream = TcpStream::connect("REMOTE_SERVER").unwrap();
        let mut sess = Session::new().unwrap();
        sess.set_tcp_stream(tcp_stream);
        sess.handshake().unwrap();

        // Authenticate the user using PEM file
        sess.userauth_pubkey_file("ubuntu", Option::None, Path::new("PRIVATE_KEY"), Option::None).unwrap();

        // Terminate the loading screen thread
        let _ = tx.send(true);
        ConsoleCLI::delete_prev_line();
        ConsoleCLI::print_line("Connected to remote server!");


        let mut channel = sess.channel_session().unwrap();
        channel.exec("ls").unwrap();
        let mut s = String::new();
        channel.read_to_string(&mut s).unwrap();
        println!("{}", s);

        channel.wait_close().unwrap();
        println!("{}", channel.exit_status().unwrap());111
        return Ok(true);
    }
}

// Wrapper class to handle HTTP requests
struct HttpClient {
    hostname: String
}

impl HttpClient {
    pub fn new(hostname: String) -> Self {
            return HttpClient {
                hostname
        }
    }

    pub async fn get(&self, url: String) -> Result<String, Box<dyn std::error::Error>> {
        todo!();
    }
}

// Struct to model the behaviour of the user
struct User {
    username: String,
    password: String,
    token: String
}

impl User {
    // Creates a new user with empty username
    // and password
    fn new() -> Self {
        return User {
            username: String::new(),
            password: String::new(),
            token: String::new()
        }
    }

    // Validate the credentials of the user
    pub async fn validate(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        let mut request_params : HashMap<String, String> = HashMap::new();

        request_params.insert(String::from("email"), self.username.clone());
        request_params.insert(String::from("password"), self.password.clone());

        let client = reqwest::Client::new();
        let response = client.post("https://reqres.in/api/login")
            .json(&request_params)
            .send()
            .await?
            .json::<HashMap<String, String>>()
            .await?;

        if response.contains_key("token") {
            match response.get("token") {
                Some(v) => {
                    // Set the token of the user
                    self.token = v.clone();
                    return Ok(true);
                },
                None => return Ok(false)
            }
        }
        return Ok(false);
    }

    pub async fn authenticate(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        // Read the username from the user
        ConsoleCLI::print_line("Enter your username: ");

        io::stdin().read_line(&mut self.username)
            .expect("Error reading username");
        self.username = self.username.trim().parse().unwrap();

        // Read the password from the user
        ConsoleCLI::print_line("Enter your password: ");

        io::stdin().read_line(&mut self.password)
            .expect("Error reading password");
        self.password = self.password.trim().parse().unwrap();

        let tx = ConsoleCLI::load("Authenticating");

        let authenticated = self.validate().await?;
        // Terminate the loading thread
        let _ = tx.send(true);
        ConsoleCLI::delete_prev_line();

        // Validate the credentials`
        return if authenticated {
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut user = User::new();

    // Authenticate the user
    let authenticated = user.authenticate().await?;
    if authenticated {
        println!("You have been logged in!");
    } else {
        println!("Error logging in!");
    }

    let mut server = Server::new();
    let connection = server.connect().await?;
    if connection {
        ConsoleCLI::print_line("Connected to server!");
    }

    return Ok(());
}