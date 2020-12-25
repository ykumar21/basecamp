mod thread_pool;
mod timer;

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
use std::path::Path;
use futures::executor::block_on;
use threadpool::ThreadPool;
use crate::timer::Timer;

// Struct to model the behaviour of the CLI application
struct ConsoleCLI;

impl ConsoleCLI {
    fn print_line<T>(line: T)
    where T : std::fmt::Display {
        print!("{}", &line);
        io::stdout().flush()
            .expect("Error: Fallback (TODO)");
    }

    fn print_new_line() {
        println!();
    }

    fn delete_prev_line() {
        ConsoleCLI::print_line("\r");
    }

    fn load<T>(loading_text:  T) -> Sender<bool>
    where T : std::fmt::Display {
        let (tx, rx): (Sender<bool>, Receiver<bool>) = mpsc::channel();
        ConsoleCLI::print_line(&loading_text);
        thread::spawn(move || {
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

                i+=1;
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

    /// Method to connect (SSH) to a remote server using PEM encoded key asynchronously
    ///
    /// # Examples
    /// ```no_run
    /// let server = Server::new();
    /// let connected = server.connect().await?;
    ///
    /// if connected {
    ///     println!("Connected to server!");
    /// }
    /// ```
    async fn connect(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        // Connect to the remote SSH server
        let tcp_stream = TcpStream::connect("ec2-15-206-94-33.ap-south-1.compute.amazonaws.com:22").unwrap();
        let mut sess = Session::new().unwrap();
        sess.set_tcp_stream(tcp_stream);
        sess.handshake().unwrap();

        // Authenticate the user using PEM file
        sess.userauth_pubkey_file("ubuntu", Option::None, Path::new("C:/Users/Nityam/Downloads/test-new.pem"), Option::None).unwrap();

        self.session = Some(sess);
        return Ok(true);
    }

    /// Method to execute a SSH job on the remote server asynchronously
    ///
    /// # Examples
    /// ```no_run
    /// let server = Server::new();
    /// let _ = server.connect().await?;
    /// let res = server.execute("ls").await?;
    /// ```
    async fn execute(&self, job: &str) -> Result<bool, Box<dyn std::error::Error>> {
        let session = match &self.session {
            Some(sess) => sess,
            None => {
                panic!("Session not initialized!")
            }
        };

        // Create a new channel
        let mut channel = session.channel_session().unwrap();
        // Execute the job on the server
        channel.exec(job).unwrap();

        // Get the output of the job and display
        let mut s = String::new();
        channel.read_to_string(&mut s).unwrap();
        ConsoleCLI::print_new_line();
        println!("{}", s);

        // Close the channel
        channel.wait_close().unwrap();
        return Ok(true);
    }


}

/// Wrapper class to handle HTTP requests
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
        todo!()
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


    // Configuration for the thread pool
    const NUM_WORKERS: usize = 1;
    const NUM_JOBS: usize = 1;

    // Show the loading text
    let tx = ConsoleCLI::load(format!("Executing {} jobs", NUM_JOBS));

    // Create a thread pool to run the SSH jobs in parallel
    let pool = ThreadPool::new(NUM_WORKERS);

    // Start the timer to time the duration for all the jobs
    // to be completed
    let timer = Timer::new();

    // Execute the jobs using worker threads
    for i in 0..NUM_JOBS {
        pool.execute( move || {
            // Connect to the server and execute the SSH job
            let mut server = Server::new();
            block_on(server.connect()).unwrap();
            let res = block_on(server.execute("touch new.txt; echo 'This is a file created by rust!' > new.txt; cat new.txt;")).unwrap();
            ConsoleCLI::print_line(format!("Completed job {}\n", i));
        });
    }

    // Make a blocking call to wait for all the jobs to be
    // completed
    pool.join();

    // Terminate the loading screen thread
    let _ = tx.send(true);
    ConsoleCLI::delete_prev_line();
    ConsoleCLI::print_line(format!("Finished Jobs in {}s", timer.ellapsed().as_secs()));
    drop(timer);

    return Ok(());
}