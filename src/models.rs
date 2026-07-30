// use axum::http::StatusCode;
// use std::io::{Read,Write};
// use axum::{};
use ::serde::{Serialize,Deserialize};

pub enum ServerCommands
{
    GetAgent(String),
    GetAgents,
    GetTask(String),
    SwitchAgent(String),
    Payload(BeaconArgs)
}

#[derive(Clone,Serialize,Deserialize,Debug)]
#[serde(tag="type",content="arguments",rename_all="snake_case")]
pub enum BeaconArgs
{
    Config{sleep:f32, jitter:f32}, 
    SetSleep(f32),
    SetJitter(f32),
    ExecCommand(String, Option<Vec<String>>),
    FileDownload(String),
    Pwd,
    ChangeDir,
    Persist,
    End
    //is strongly typed enum better here, or should i handle numeric argument parsing at the boundary?
}

impl TryFrom<&str> for ServerCommands {
    //Parses &str to corresponding ServerCommand
    type Error = &'static str;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let tokens:Vec<&str>=value.split_whitespace().collect();
        match tokens.as_slice()
        {
            ["config",sleep,jitter]=>{
                let s:f32=sleep.parse().map_err(|_|"Invalid sleep")?;
                let j:f32=jitter.parse().map_err(|_|"Invalid jitter")?;
                Ok(ServerCommands::Payload(BeaconArgs::Config{sleep:s,jitter:j}))
            }
            ["setsleep",sleep]=>{
                let s=sleep.parse().map_err(|_|"Invalid sleep")?;
                Ok(ServerCommands::Payload(BeaconArgs::SetSleep(s)))
            }
            ["setjitter",jitter]=>{
                let j=jitter.parse().map_err(|_|"Invalid jitter")?;
                Ok(ServerCommands::Payload(BeaconArgs::SetJitter(j)))
            }
            ["GetAgent",uid]=>{
                // let uuid=uid.parse.map_err(|_|"Invalid UUID");
                Ok(ServerCommands::GetAgent(uid.to_string()))
            }
            ["GetAgents"]=>{Ok(ServerCommands::GetAgents)}
            ["GetTask",taskid]=>{
                // let tid=taskid.parse.map_err(|_|"Invalid UUID");
                Ok(ServerCommands::GetTask(taskid.to_string()))
            }
            ["SwitchAgent",uid]=>{
                // let uuid=uid.parse.map_err(|_|"Invalid UUID");
                Ok(ServerCommands::SwitchAgent(uid.to_string()))
            }
            ["ExecCommand",cmd, arguments@..]=>{
                let args={
                    match arguments{
                        []=>{None}
                        _=>{Some(arguments.iter().map(|&arg|arg.to_string()).collect())}
                    }
                };
                Ok(ServerCommands::Payload(BeaconArgs::ExecCommand(cmd.to_string(), args)))
            }
            ["FileDownload",argument]=>{
                Ok(ServerCommands::Payload(BeaconArgs::FileDownload(argument.to_string())))
            }
            ["pwd"]=>{
                Ok(ServerCommands::Payload(BeaconArgs::Pwd))
            }
            ["cd"]=>{
                Ok(ServerCommands::Payload(BeaconArgs::ChangeDir))
            }
            ["persist"]=>{
                Ok(ServerCommands::Payload(BeaconArgs::Persist))
            }
            ["end"]=>{
                Ok(ServerCommands::Payload(BeaconArgs::End))
            }
            _=>{return Err("Command invalid!")}
        }
    }
}   