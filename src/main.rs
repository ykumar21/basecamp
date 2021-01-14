#![allow(warnings)]
mod timer;
mod job;

use std::sync::{Arc,Mutex};
use std::io::{self, Write, stdout};
use std::collections::HashMap;
use std::thread;
use std::sync::mpsc::{self, TryRecvError, Sender, Receiver};
use std::net::TcpStream;
use ssh2::Session;
use std::path::Path;
use futures::executor::block_on;
use threadpool::ThreadPool;
use cli_table::{format::Justify, print_stdout, Style as TableStyle, Cell, Table, CellStruct};
use tui::Terminal;
use tui::backend::CrosstermBackend;
use tui::widgets::{Widget, Block, Borders, ListItem, Wrap, Paragraph, ListState};
use tui::layout::{Layout, Constraint, Direction, Alignment};
use tui::style::{Style, Color, Modifier};
use crossterm::{execute, style::{SetBackgroundColor}, ExecutableCommand};
use crossterm::event::{poll, read, Event, KeyEvent, KeyCode, KeyModifiers};

use crate::timer::Timer;
use crate::job::Job;
use tui::text::{Span, Spans};
use tokio::time::Duration;

struct ConsoleCLI {
    active_listener_index: usize,
    terminal: Terminal<CrosstermBackend<std::io::Stdout>>
}


impl ConsoleCLI {

    /// The index of different listeners
    const SERVER_INDEX : usize = 0;
    const TASK_INDEX : usize = 1;
    const FOOTER_INDEX : usize = 2;
    const OUTPUT_INDEX : usize = 3;

    /// The styling options
    const BACKGROUND_COLOR_HEX : (u8, u8, u8) = (3, 36, 45);
    const BACKGROUND_COLOR: Color = Color::Rgb(3, 36, 45);

    /// Method to construct a new cli with
    /// the crossterm backend
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let stdout = io::stdout();
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Change the background color to blue
        io::stdout()
            .execute(SetBackgroundColor( crossterm::style::Color::from( ConsoleCLI::BACKGROUND_COLOR_HEX ) ) );

        let mut cli = ConsoleCLI {
            active_listener_index: 0,
            terminal
        };

        return Ok(cli);
    }

    /// Method to render the UI
    fn render(&mut self) -> Result<(), Box<dyn std::error::Error>> {

        let mut task_listener = Listener::new(vec![
            String::from("+ Add New Task"),
            String::from("List out files"),
            String::from("Create new file"),
            String::from("Open TCP Port")
        ]);

        let mut server_listener = Listener::new(vec![
            String::from("+ Add New Server"),
            String::from("Server 1 - xyz"),
            String::from("Server 2 - abc"),
            String::from("Server 3 - def")
        ]);


        loop {
            if poll(Duration::from_millis(500))? {
                match read()? {
                    Event::Key(event) => {
                        let key_code = event.code;
                        let key_modifier = event.modifiers;

                        match key_code {
                            KeyCode::Char(' ') => {
                                // Move to the next section
                                self.active_listener_index = (self.active_listener_index+1)%2;
                            },
                            KeyCode::Up => {
                                match self.active_listener_index {
                                    0 => {
                                        task_listener.unselect();
                                        server_listener.previous();
                                    },
                                    1 => {
                                        server_listener.unselect();
                                        task_listener.previous();
                                    },
                                    _ => unimplemented!()
                                }
                            },
                            KeyCode::Down => {
                                match self.active_listener_index {
                                    0 => {
                                        task_listener.unselect();
                                        server_listener.next();
                                    },
                                    1 => {
                                        server_listener.unselect();
                                        task_listener.next();
                                    },
                                    _ => unimplemented!()
                                }
                            }
                            _ => println!("do nothing"),
                        }
                    },
                    Event::Mouse(event) => println!("{:?}", event),
                    Event::Resize(..) => println!("Resized!")
                }
            }

            self.terminal.draw(|f| {
                // Create a copy of the listeners
                let main_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(
                        [
                            Constraint::Percentage(50),
                            Constraint::Percentage(50)
                        ].as_ref()
                    )
                    .split(f.size());

                // Render the output terminal
                let text = vec![
                    Spans::from(vec![
                        Span::raw("Lorem ipsum dolor sit amet, consectetur adipiscing elit. Vestibulum feugiat dui eu nunc finibus, eget iaculis lorem malesuada. Mauris ipsum dui, rutrum nec purus quis, rhoncus eleifend sapien"),
                    ]),
                ];
                let output_term = Paragraph::new(text)
                    .block(Block::default().title(" OUTPUT ").borders(Borders::ALL))
                    .style(Style::default().fg(Color::White).bg(ConsoleCLI::BACKGROUND_COLOR))
                    .alignment(Alignment::Left)
                    .wrap(tui::widgets::Wrap { trim: true });
                f.render_widget(output_term, main_chunks[1]);

                let mini_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(
                        [
                            Constraint::Percentage(40),
                            Constraint::Percentage(40),
                            Constraint::Percentage(20)
                        ].as_ref()
                    )
                    .split(main_chunks[0]);

                let task_items : Vec<ListItem> = task_listener.items.iter().map(|i| ListItem::new(i.as_ref())).collect();
                let task_list = tui::widgets::List::new(task_items)
                    .block(Block::default().title(" TASKS ").borders(Borders::ALL))
                    .style(Style::default().fg(Color::White).bg(ConsoleCLI::BACKGROUND_COLOR))
                    .highlight_style(Style::default().add_modifier(Modifier::BOLD))
                    .highlight_symbol(">>");

                f.render_stateful_widget(task_list, mini_chunks[1], &mut task_listener.state);

                // Render the server list
                let server_items : Vec<ListItem> = server_listener.items.iter().map(|v| ListItem::new(v.as_ref())).collect();
                let server_list = tui::widgets::List::new(server_items)
                    .block(Block::default().title(" SERVERS ").borders(Borders::ALL))
                    .style(Style::default().fg(Color::White).bg(ConsoleCLI::BACKGROUND_COLOR))
                    .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
                    .highlight_symbol(">>");
                f.render_stateful_widget(server_list, mini_chunks[0], &mut server_listener.state);


                // Render the footer
                let text = vec![
                    Spans::from(vec![
                        Span::raw("Lorem ipsum dolor sit amet, consectetur adipiscing elit. Vestibulum feugiat dui eu nunc finibus, eget iaculis lorem malesuada. Mauris ipsum dui, rutrum nec purus quis, rhoncus eleifend sapien"),
                    ]),
                ];

                let footer = tui::widgets::Paragraph::new(text)
                    .block(Block::default().title(" INFORMATION ").borders(Borders::ALL))
                    .style(Style::default().fg(Color::White).bg(ConsoleCLI::BACKGROUND_COLOR))
                    .alignment(tui::layout::Alignment::Left)
                    .wrap(tui::widgets::Wrap { trim: true });

                f.render_widget(footer, mini_chunks[2]);
            });
        }


    }

    /// Method to print some text to the Console
    ///
    /// # Examples
    /// ```no_run
    /// ConsoleCLI::print_line("Hey there");
    /// ConsoleCLI::print_line( String::from( "You can use strings as well!" ) );
    /// ```
    fn print_line<T>(line: T)
    where T : std::fmt::Display {
        print!("{}", &line);
        io::stdout().flush()
            .expect("Error: Fallback (TODO)");
    }

    /// Method to clear the terminal and places
    /// the cursor at the top-left of the terminal
    fn clear() {
        print!("\x1B[2J\x1B[1;1H");
    }

    fn print_new_line() {
        println!();
    }

    fn delete_prev_line() {
        ConsoleCLI::print_line("\r");
    }

    fn load<T>(loading_text: T) -> Sender<bool>
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

    fn display_table<T>(data: &Vec<T>)
    where T : std::fmt::Display {
        let num_rows = data.len();
        let mut table: Vec<Vec<CellStruct>>= Vec::with_capacity(num_rows);
        for (i, v) in data.iter().enumerate() {
            let row  = vec![i.cell().justify(Justify::Right), v.cell().justify(Justify::Right)];
            table.push(row);
        }

        let tableStruct = table.table()
            .title(vec![
                "Job #".cell().bold(true),
                "Result".cell().bold(true)
            ])
            .bold(true);

        print_stdout(tableStruct);
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
        sess.userauth_pubkey_file("ubuntu", Option::None, Path::new("C:/Users/Yash/Downloads/test-new.pem"), Option::None).unwrap();

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
    async fn execute(&self, job: &Job) -> Result<String, Box<dyn std::error::Error>> {
        let session = match &self.session {
            Some(sess) => sess,
            None => {
                panic!("Session not initialized!")
            }
        };
        // Create a new channel
        let mut channel = session.channel_session().unwrap();
        // Execute the job on the server and get the output

        let output = job.execute(&mut channel).await?;

        // Close the channel
        channel.wait_close().unwrap();
        return Ok(output);
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

#[derive(Clone, Debug)]
struct Listener {
    items: Vec<String>,
    state: ListState
}

impl Listener {
    fn new(items: Vec<String>) -> Self {
        assert!(items.len() > 0);
        let mut listener = Listener {
            items,
            state: ListState::default()
        };

        return listener;
    }

    pub fn set_items(&mut self, items: Vec<String>) {
        // Reset the items and state
        self.items = items;
        self.state = ListState::default();
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len()-1 {
                    0
                } else {
                    i+1
                }
            },
            None => 0
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            },
            None => 0
        };
        self.state.select(Some(i));
    }

    pub fn unselect(&mut self) {
        self.state.select(None);
    }
}



#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    let mut cli = Arc::new(Mutex::new(ConsoleCLI::new().unwrap()));
    let cli_thread = thread::spawn( move || {
        let clone = Arc::clone(&cli);
        let mut cli = &mut *clone.lock().unwrap();
        cli.render();
    });

    cli_thread.join().unwrap();

    // let mut user = User::new();
    //
    // // Authenticate the user
    // let authenticated = user.authenticate().await?;
    // if authenticated {
    //     println!("You have been logged in!");
    // } else {
    //     println!("Error logging in!");
    // }
    // // Configuration for the thread pool
    // const NUM_WORKERS: usize = 5;
    // // Create the jobs
    // const NUM_JOBS: usize = 5;
    //
    // // Holds the tasks entered by the user
    // let mut task_list: Vec<String> = Vec::with_capacity(NUM_JOBS);
    // // Holds the results of the jobs
    // let job_results = Arc::new(Mutex::new( Vec::with_capacity(NUM_JOBS) ) );
    //
    // // Receive the tasks from the user
    // for _ in 0..NUM_JOBS {
    //     let mut job_task = String::new();
    //     io::stdin().read_line(&mut job_task)
    //         .expect("Failed to read job");
    //     job_task = job_task.parse().expect("Failed to parse job!");
    //     task_list.push(job_task);
    // }
    //
    // // Show the loading text
    // let tx = ConsoleCLI::load(format!("Executing {} jobs", NUM_JOBS));
    // // Create a thread pool to run the SSH jobs in parallel
    // let pool = ThreadPool::new(NUM_WORKERS);
    //
    // // Start the timer to time the duration for all the jobs
    // // to be completed
    // let timer = Timer::new();
    //
    // // Execute the jobs using worker threads
    // for i in 0..NUM_JOBS {
    //     // Get the task for the current job
    //     let job_task= task_list[i].clone();
    //     // Make a clone of the results
    //     let clone = Arc::clone(&job_results);
    //     pool.execute( move || {
    //         // Get the vector storing the results
    //         let mut result_vec = clone.lock().unwrap();
    //         // Connect to the server and execute the SSH job
    //         let mut server = Server::new();
    //         block_on(server.connect()).unwrap();
    //
    //         // Create the new job for the worker thread
    //         // and execute the job
    //         let job = Job::new(job_task);
    //         let res = block_on(server.execute(&job)).unwrap();
    //         result_vec.push(String::from(res));
    //     });
    // }
    //
    // // Make a blocking call to wait for all the jobs to be
    // // completed
    // pool.join();
    //
    // // Terminate the loading screen thread
    // let _ = tx.send(true);
    // ConsoleCLI::delete_prev_line();
    // ConsoleCLI::print_line(format!("Finished Jobs in {}s\n", timer.ellapsed().as_secs()));
    // drop(timer);
    //
    // // Display the results in the table
    // ConsoleCLI::display_table(&*Arc::clone(&job_results).lock().unwrap());
    // drop(job_results);
    return Ok(());
}