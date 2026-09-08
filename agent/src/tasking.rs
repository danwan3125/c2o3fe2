use std::{
    collections::VecDeque, 
    sync::{Arc,atomic::AtomicBool, mpsc},
    error::Error
};
use ::serde::{Serialize,Deserialize};

use crate::agent::{self, BeaconArgs,AgentTask};

//Currently only needed for upload function; included for scalability to additional features
pub struct BackgroundTask{
    pub command:String,
    pub parameters:String,
    pub jobid:u32,
    pub running:Arc<AtomicBool>,
    pub killable:bool,
    pub uuid:String,
    tx:mpsc::Sender<serde_json::Value>,
    rx:mpsc::Receiver<serde_json::Value>
}
pub struct TaskingEngine{
    pub running_tasks:Vec<BackgroundTask>,
    completed_tasks:Vec<serde_json::Value>,
    new_job_id:u32,
    used_job_id:VecDeque<u32>
}

impl TaskingEngine{
    pub fn new()->Self{
        Self{
            running_tasks:Vec::new(),
            completed_tasks:Vec::new(),
            new_job_id:0,
            used_job_id:VecDeque::new()
        }
     }

    pub fn parse_tasks(
        &mut self,
        tasks:Option<&Vec<agent::AgentTask>>,
        config:&mut agent::SharedConfig
    )->Result<(),Box<dyn Error>>{
        if let Some(tasks)=tasks{
            for task in tasks.iter(){
                self.completed_tasks.push(
                    match task.instruction{
                        //Tasks that modify shared config will be processed without entering a separate function 
                        BeaconArgs::Config{sleep:sleep_val, jitter:jitter_val} =>{
                            return Ok(());
                        },
                        BeaconArgs::SetSleep(sleep_val) =>{
                            return Ok(());
                        },
                        BeaconArgs::SetJitter(jitter_val) =>{
                            return Ok(());
                        },
                        BeaconArgs::End =>{
                            return Ok(());
                        },
                        
                        _ =>process_task(task),
                    }
                );
            }
        }
        Ok(())
    }

    pub fn get_completed_tasks(&mut self)->Result<Vec<serde_json::Value>,Box<dyn Error>>{
        let mut completed_tasks:Vec<serde_json::Value>=Vec::new();
        completed_tasks.append(&mut self.completed_tasks);
        //TODO: add completed background tasks to result

        Ok(completed_tasks)
    }
    
}
fn process_task(task:&agent::AgentTask)->serde_json::Value{
    match task.instruction{
        BeaconArgs::ListDirectory=>{().into()}
        _=>{().into()}
    }
}