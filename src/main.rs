#![allow(dead_code)]
#![allow(unused_variables)]
mod timer;
mod job;

use std::sync::{Arc,Mutex};
use std::io::{self, Write};
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
use tui::widgets::{Block, Borders, ListItem, Paragraph, ListState};
use tui::layout::{Layout, Constraint, Direction, Alignment};
use tui::style::{Style, Color};
use crossterm::{style::{SetBackgroundColor}, ExecutableCommand};
use crossterm::event::{poll, read, Event, KeyCode};

use crate::timer::Timer;
use crate::job::Job;
use tui::text::{Span, Spans};
use tokio::time::Duration;

struct ConsoleCLI {
    active_listener_index: usize,
    terminal: Terminal<CrosstermBackend<std::io::Stdout>>,
    task_listener: Arc<Mutex<Listener<String>>>,
    server_listener: Arc<Mutex<Listener<String>>>,
    selected_servers: Vec<String>,
    selected_jobs: Vec<String>,
    console_text: String,
    render: bool
}


impl ConsoleCLI {

    /// The index of different listeners
    const SERVER_INDEX : usize = 0;
    const TASK_INDEX : usize = 1;
    const FOOTER_INDEX : usize = 2;
    const OUTPUT_INDEX : usize = 3;

    /// The styling options
    const BACKGROUND_COLOR_HEX : (u8, u8, u8) = (42, 3, 33);
    const BACKGROUND_COLOR: Color = Color::Rgb(42, 3, 33);

    /// Method to construct a new cli with
    /// the crossterm backend
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let stdout = io::stdout();
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        // Change the background color
        io::stdout()
            .execute(SetBackgroundColor( crossterm::style::Color::from( ConsoleCLI::BACKGROUND_COLOR_HEX ) ) ).unwrap();


        let task_listener = Listener::new(vec![
            String::from("touch hey.txt"),
            String::from("ls"),
            String::from("ls")
        ]);


        let server_listener = Listener::new(vec![
            String::from("DELMAIN01"),
            String::from("DELBACKUP01"),
            String::from("HRMAIN01")
        ]);

        return Ok(ConsoleCLI {
            terminal,
            active_listener_index: 0,
            task_listener: Arc::new(Mutex::new(task_listener)),
            server_listener: Arc::new(Mutex::new(server_listener)),
            selected_jobs: Vec::new(),
            selected_servers: Vec::new(),
            console_text: String::new(),
            render: true
        });
    }

    /// Method to clear the output to the
    /// terminal
    ///
    /// # Examples:
    /// ```no_run
    /// let mut cli = ConsoleCLI::new();
    /// cli.print("Hey!");
    /// cli.clear();
    /// ```
    fn clear(&mut self) {
        self.console_text = String::new();
    }

    /// Method to print text to the terminal
    /// console
    ///
    /// # Examples:
    /// ```no_run
    ///  let mut cli = ConsoleCLI::new();
    ///  cli.print("Hey!");
    /// ```
    fn print(&mut self, mut text: String) {
        text.push_str("\n");
        self.console_text.push_str(&*text);
    }

    /// Method to render the UI
    fn render(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            // Stop rendering the cli
            if self.render == false {
                break Ok(());
            }

            let task_listener_clone = Arc::clone(&self.task_listener);
            let mut task_listener = &mut *(task_listener_clone).lock().unwrap();

            let server_listener_clone = Arc::clone(&self.server_listener);
            let mut server_listener = &mut *(server_listener_clone).lock().unwrap();

            // Get a mutable reference to the current active listener
            let active_listener= match self.active_listener_index {
                0 => &mut server_listener,
                1 => &mut task_listener,
                _ => unimplemented!()
            };

            let terminal_text = self.console_text.clone();

            if poll(Duration::from_millis(200))? {
                match read()? {
                    Event::Key(event) => {
                        let key_code = event.code;

                        match key_code {
                            KeyCode::Char(' ') => {

                                if self.selected_servers.len() == 0 {
                                    self.print(
                                        format!(
                                            "Please select atleast 1 server! Selected {}",
                                            self.selected_servers.len()
                                        )
                                    );
                                } else if self.selected_jobs.len() == 0 {
                                    self.print(
                                        format!(
                                            "Please select atleast 1 server! Selected {}",
                                            self.selected_servers.len()
                                        )
                                    );
                                } else {
                                    self.print(
                                        format!(
                                            "Executing {} jobs on {} servers!",
                                            self.selected_jobs.len(),
                                            self.selected_servers.len()
                                        )
                                    );
                                    // Stop rendering the ui and
                                    // clear the terminal
                                    self.render = false;
                                    ConsoleCLI::clear_screen();
                                }
                            },

                            KeyCode::Tab => {
                                active_listener.unselect();
                                self.active_listener_index = (self.active_listener_index+1)%2;

                            }
                            KeyCode::Up => {
                                active_listener.previous();
                            },
                            KeyCode::Down => {
                                active_listener.next();
                            },
                            KeyCode::Enter => {
                                let selected_item = match active_listener.get_selected() {
                                    Some(item) => item,
                                    None => {  panic!("Could not fetch element"); }
                                };

                                match self.active_listener_index {
                                    0 => {
                                        self.print(format!("Selected server: {}", selected_item));
                                        self.selected_servers.push(selected_item.to_string());
                                    },
                                    1 => {
                                        self.print(format!("Selected job: {}", selected_item));
                                        self.selected_jobs.push(selected_item.to_string());
                                    },
                                    _ => panic!("Ye kaise hogaya?")
                                };

                            }
                            _ => println!("do nothing"),
                        }
                    },
                    Event::Mouse(event) => {
                        println!("{:?}", event)
                    },
                    Event::Resize(..) => {
                        println!("Resized!")
                    }
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
                let text = tui::text::Text::from(terminal_text);

                let output_term = Paragraph::new(text)
                    .block(Block::default().title(" OUTPUT ").borders(Borders::ALL))
                    .style(Style::default().fg(Color::White).bg(ConsoleCLI::BACKGROUND_COLOR))
                    .alignment(Alignment::Left)
                    .wrap(tui::widgets::Wrap { trim: true })
                    .scroll((0,1));

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
                    .highlight_style(
                        Style::default()
                            .fg(Color::Yellow)
                    )
                    .highlight_symbol(">> ");

                f.render_stateful_widget(task_list, mini_chunks[1], &mut task_listener.state);

                // Render the server list
                let server_items : Vec<ListItem> = server_listener.items.iter().map(|v| ListItem::new(v.as_ref())).collect();
                let server_list = tui::widgets::List::new(server_items)
                    .block(Block::default().title(" SERVERS ").borders(Borders::ALL))
                    .style(Style::default().fg(Color::White).bg(ConsoleCLI::BACKGROUND_COLOR))
                    .highlight_style(
                        Style::default()
                            .fg(Color::Yellow)
                    )
                    .highlight_symbol(">> ");
                f.render_stateful_widget(server_list, mini_chunks[0], &mut server_listener.state);


                // Render the footer
                let text = vec![
                    Spans::from(vec![
                        Span::raw("Basecamp is an application which allows you to execute shell jobs on multiple servers directly from your local machine. Update the server.yaml file to configure the servers and job.yaml file to configure the jobs!"),
                    ]),
                ];

                let footer = tui::widgets::Paragraph::new(text)
                    .block(Block::default().title(" INFORMATION ").borders(Borders::ALL))
                    .style(Style::default().fg(Color::White).bg(ConsoleCLI::BACKGROUND_COLOR))
                    .alignment(tui::layout::Alignment::Left)
                    .wrap(tui::widgets::Wrap { trim: true });

                f.render_widget(footer, mini_chunks[2]);
            }).unwrap();
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
    fn clear_screen() {
        print!("\x1B[2J\x1B[1;1H");
    }

    fn print_new_line() {
        println!();
    }

    /// Method to execute the selected jobs on the
    /// selected servers asynchrouslly
    pub async fn execute_jobs(&mut self) -> Result<(), Box<dyn std::error::Error>> {

        let mut user = User::new();

        // Authenticate the user
        let authenticated = user.authenticate().await?;
        if authenticated {
            println!("You have been logged in!");
        } else {
            println!("User not found (Running on TEST user)");
        }
        // Configuration for the thread pool
        const NUM_WORKERS: usize = 5;

        // Create the jobs
        let num_jobs: usize = self.selected_jobs.len();

        // Holds the results of the jobs
        let job_results = Arc::new(Mutex::new( Vec::with_capacity(num_jobs) ) );

        // Receive the tasks from the user
        // for _ in 0..NUM_JOBS {
        //     let mut job_task = String::new();
        //     io::stdin().read_line(&mut job_task)
        //         .expect("Failed to read job");
        //     job_task = job_task.parse().expect("Failed to parse job!");
        //     task_list.push(job_task);
        // }

        // Show the loading text
        let tx = ConsoleCLI::load(format!("Executing {} jobs", num_jobs));

        // Create a thread pool to run the SSH jobs in parallel
        let pool = ThreadPool::new(NUM_WORKERS);

        // Start the timer to time the duration for all the jobs
        // to be completed
        let timer = Timer::new();

        // Execute the jobs using worker threads
        for i in 0..num_jobs {
            // Get the task for the current job
            let job_task = self.selected_jobs[i].clone();
            // Make a clone of the results
            let clone = Arc::clone(&job_results);
            pool.execute( move || {
                // Get the vector storing the results
                let mut result_vec = clone.lock().unwrap();
                // Connect to the server and execute the SSH job
                let mut server = Server::new();
                block_on(server.connect()).unwrap();

                // Create the new job for the worker thread
                // and execute the job
                let job = Job::new(job_task);
                let res = block_on(server.execute(&job)).unwrap();
                result_vec.push(String::from(res));
            });
        }

        // Make a blocking call to wait for all the jobs to be
        // completed
        pool.join();

        // Terminate the loading screen thread
        let _ = tx.send(true);
        ConsoleCLI::delete_prev_line();
        ConsoleCLI::print_line(format!("Finished Jobs in {}s\n", timer.ellapsed().as_secs()));
        drop(timer);

        // Display the results in the table
        ConsoleCLI::display_table(&*Arc::clone(&job_results).lock().unwrap());
        drop(job_results);

        return Ok(());
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

        let table_struct = table.table()
            .title(vec![
                "Job #".cell().bold(true),
                "Result".cell().bold(true)
            ])
            .bold(true);

        print_stdout(table_struct).unwrap();
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
struct Listener<T> {
    items: Vec<T>,
    state: ListState
}

impl<T> Listener<T>
where
    T : std::fmt::Display
{

    fn new(items: Vec<T>) -> Self {
        assert!(items.len() > 0);
        let listener = Listener {
            items,
            state: ListState::default()
        };

        return listener;
    }

    pub fn set_items(&mut self, items: Vec<T>) {
        // Reset the items and state
        self.items = items;
        self.state = ListState::default();
    }

    pub fn get_selected(&self) -> Option<&T> {
        let elem = match self.state.selected() {
            Some(i) => Some(&self.items[i]),
            None => None
        };
        return elem;
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

    pub fn get_item(&self, index: usize) -> &T {
        assert!(index < self.items.len());
        return &self.items[index];
    }
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    let cli = Arc::new(Mutex::new(ConsoleCLI::new().unwrap()));
    let clone = Arc::clone(&cli);

    let render_handle = thread::spawn(move || {
        let cli_clone = &mut *(clone).lock().unwrap();
        cli_clone.render().unwrap();
    });

    render_handle.join().unwrap();

    // Execute the jobs
    (*Arc::clone(&cli)).lock().unwrap().execute_jobs().await?;

    return Ok(());
}