#[allow(dead_code)]
#[allow(unused_variables)]
#[allow(non_snake_case)]

use std::io;
use std::ptr::null;
use std::collections::HashMap;
use std::io::Write;
use colored::*;
use std::thread;
use std::thread::JoinHandle;

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

    fn load(loading_text:  &'static str) -> JoinHandle<()> {
        return thread::spawn(move || {
            ConsoleCLI::print_line(loading_text);
            for i in 1..13 {
                ConsoleCLI::print_line(".");
                if i % 4 == 0 {
                    ConsoleCLI::print_line("\x08\x08\x08\x08");
                    ConsoleCLI::print_line("    ");
                    ConsoleCLI::print_line("\x08\x08\x08\x08");
                }

                // Model some delay
                for _t in 1..10000000 {
                    // Do nothing :P
                }
            }
            ConsoleCLI::delete_prev_line();
        });
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

        // Validate the credentials
        if self.validate().await? {
            return Ok(true);
        } else {
            return Ok(false);
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let load_thread = ConsoleCLI::load("Initializing");
    load_thread.join();

    let mut user = User::new();

    let authenticated = user.authenticate().await?;

    if authenticated {
        println!("You have been logged in!");
    } else {
        println!("Error logging in!");
    }

    return Ok(());
}