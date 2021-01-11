use ssh2;
use ssh2::Channel;
use std::io::Read;

pub struct Job {
    task: String
}

impl Job {
    pub fn new(task: String) -> Self {
        return Job {
            task
        };
    }

    /// Method to assign a new task to
    /// the job
    pub fn assign_task(&mut self, task: String) {
        self.task = task;
    }

    /// Method to run the particular
    /// job over a specified channel
    pub async fn execute(&self, channel: &mut Channel) -> Result<String, Box<dyn std::error::Error>> {
        // Execute the job on the server
        channel.exec(&*self.task).unwrap();

        // Read the output from the server
        let mut output = String::new();
        channel.read_to_string(&mut output).unwrap();

        // Return the output of the command
        return Ok(output);
    }



}